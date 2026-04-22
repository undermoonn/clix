use crate::game::Game;

#[cfg(target_os = "windows")]
pub fn scan_games() -> Vec<Game> {
    imp::scan_games()
}

#[cfg(not(target_os = "windows"))]
pub fn scan_games() -> Vec<Game> {
    Vec::new()
}

#[cfg(target_os = "windows")]
mod imp {
    use std::collections::{HashMap, HashSet};
    use std::path::{Path, PathBuf};

    use chrono::{DateTime, Local, LocalResult, TimeZone};
    use serde_json::Value;

    use crate::game::{Game, GameSource};

    pub fn scan_games() -> Vec<Game> {
        let Some(manifest_dir) = manifest_dir() else {
            return Vec::new();
        };
        let Ok(entries) = std::fs::read_dir(manifest_dir) else {
            return Vec::new();
        };

        let last_played = load_last_played_map();
        let mut games = Vec::new();
        let mut seen_install_dirs = HashSet::new();

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() || !is_manifest_file(&path) {
                continue;
            }

            let Some(game) = parse_manifest(&path, &last_played) else {
                continue;
            };

            let install_dir_key = normalize_windows_path(&game.path);
            if !seen_install_dirs.insert(install_dir_key) {
                continue;
            }

            games.push(game);
        }

        games
    }

    fn manifest_dir() -> Option<PathBuf> {
        let base = std::env::var_os("ProgramData")
            .or_else(|| std::env::var_os("ALLUSERSPROFILE"))
            .map(PathBuf::from)
            .or_else(|| {
                let fallback = PathBuf::from(r"C:\ProgramData");
                fallback.exists().then_some(fallback)
            })?;

        let dir = base
            .join("Epic")
            .join("EpicGamesLauncher")
            .join("Data")
            .join("Manifests");

        dir.exists().then_some(dir)
    }

    fn is_manifest_file(path: &Path) -> bool {
        matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("item") | Some("manifest")
        )
    }

    fn parse_manifest(path: &Path, last_played: &HashMap<String, u64>) -> Option<Game> {
        let content = std::fs::read_to_string(path).ok()?;
        let manifest: Value = serde_json::from_str(&content).ok()?;

        let name = manifest
            .get("DisplayName")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())?
            .to_owned();

        let install_dir = manifest
            .get("InstallLocation")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .filter(|path| path.is_dir())?;

        let launch_executable = manifest
            .get("LaunchExecutable")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())?;
        let app_name = manifest
            .get("AppName")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);

        let launch_target = resolve_launch_target(&install_dir, launch_executable)
            .filter(|path| path.is_file())?;
        let last_played = manifest_last_played(&manifest, last_played).unwrap_or(0);

        let dlss_version = crate::assets::dlss::detect_version(&install_dir, None);

        Some(Game {
            source: GameSource::Epic,
            name,
            path: install_dir,
            launch_target: Some(launch_target),
            app_id: None,
            persistent_id: app_name,
            last_played,
            playtime_minutes: 0,
            installed_size_bytes: None,
            dlss_version,
        })
    }

    fn manifest_last_played(manifest: &Value, last_played: &HashMap<String, u64>) -> Option<u64> {
        let catalog_item_id = manifest
            .get("CatalogItemId")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())?;
        let app_name = manifest
            .get("AppName")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())?;

        last_played
            .get(&last_played_key(catalog_item_id, app_name))
            .copied()
    }

    fn load_last_played_map() -> HashMap<String, u64> {
        let Some(settings_path) = user_settings_path() else {
            return HashMap::new();
        };
        let Ok(content) = std::fs::read_to_string(settings_path) else {
            return HashMap::new();
        };

        let mut last_played = HashMap::new();

        for line in content.lines() {
            let Some(value) = line.strip_prefix("LastPlayedGame=") else {
                continue;
            };
            let Some((game_id, timestamp)) = value.rsplit_once(',') else {
                continue;
            };

            let mut parts = game_id.rsplit(':');
            let Some(app_name) = parts.next().map(str::trim).filter(|value| !value.is_empty()) else {
                continue;
            };
            let Some(catalog_item_id) = parts.next().map(str::trim).filter(|value| !value.is_empty()) else {
                continue;
            };
            let Some(timestamp) = parse_last_played_timestamp(timestamp) else {
                continue;
            };

            let key = last_played_key(catalog_item_id, app_name);
            let entry = last_played.entry(key).or_insert(0);
            if timestamp > *entry {
                *entry = timestamp;
            }
        }

        last_played
    }

    fn user_settings_path() -> Option<PathBuf> {
        let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from)?;
        let path = base
            .join("EpicGamesLauncher")
            .join("Saved")
            .join("Config")
            .join("WindowsEditor")
            .join("GameUserSettings.ini");
        path.exists().then_some(path)
    }

    fn parse_last_played_timestamp(value: &str) -> Option<u64> {
        let naive = DateTime::parse_from_rfc3339(value.trim())
            .ok()?
            .naive_local();
        let timestamp = match Local.from_local_datetime(&naive) {
            LocalResult::Single(datetime) => datetime.timestamp(),
            LocalResult::Ambiguous(datetime, _) => datetime.timestamp(),
            LocalResult::None => return None,
        };
        (timestamp >= 0).then_some(timestamp as u64)
    }

    fn last_played_key(catalog_item_id: &str, app_name: &str) -> String {
        format!("{}:{}", catalog_item_id, app_name)
    }

    fn resolve_launch_target(install_dir: &Path, launch_executable: &str) -> Option<PathBuf> {
        let launch_path = PathBuf::from(launch_executable);
        if launch_path.is_absolute() {
            return Some(launch_path);
        }

        Some(install_dir.join(launch_path))
    }

    fn normalize_windows_path(path: &Path) -> String {
        path.to_string_lossy().replace('/', "\\").to_ascii_lowercase()
    }
}