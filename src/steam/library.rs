use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::game::{sort_games_by_last_played, Game, GameSource};

const STEAM_STATE_FLAG_FULLY_INSTALLED: u32 = 0x4;
const STEAM_STATE_FLAG_UPDATE_REQUIRED: u32 = 0x2;
const STEAM_STATE_FLAG_UPDATING: u32 = 0x400;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SteamUpdateProgress {
    pub state_flags: u32,
    pub bytes_downloaded: u64,
    pub bytes_to_download: u64,
    pub bytes_staged: u64,
    pub bytes_to_stage: u64,
}

impl SteamUpdateProgress {
    fn has_pending_download(&self) -> bool {
        self.bytes_to_download > 0 && self.bytes_downloaded < self.bytes_to_download
    }

    fn has_pending_stage(&self) -> bool {
        self.bytes_to_stage > 0 && self.bytes_staged < self.bytes_to_stage
    }

    fn has_completed_work(&self) -> bool {
        (self.bytes_to_download > 0 && self.bytes_downloaded >= self.bytes_to_download)
            || (self.bytes_to_stage > 0 && self.bytes_staged >= self.bytes_to_stage)
    }

    pub fn is_complete(&self) -> bool {
        !self.has_pending_download()
            && !self.has_pending_stage()
            && self.has_completed_work()
            && (self.state_flags & STEAM_STATE_FLAG_UPDATING) == 0
    }

    pub fn needs_update(&self) -> bool {
        if self.is_complete() {
            return false;
        }

        (self.state_flags & STEAM_STATE_FLAG_UPDATE_REQUIRED) != 0
            || self.has_pending_download()
            || self.has_pending_stage()
    }

}

pub fn find_steam_paths() -> Vec<PathBuf> {
    let mut steam_paths: Vec<PathBuf> = Vec::new();

    #[cfg(target_os = "windows")]
    {
        if let Ok(hklm) = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
            .open_subkey("SOFTWARE\\WOW6432Node\\Valve\\Steam")
        {
            if let Ok(s) = hklm.get_value::<String, &str>("InstallPath") {
                steam_paths.push(PathBuf::from(s));
            }
        }
        if let Ok(hkcu) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .open_subkey("Software\\Valve\\Steam")
        {
            if let Ok(s) = hkcu.get_value::<String, &str>("SteamPath") {
                steam_paths.push(PathBuf::from(s));
            }
        }
    }

    if let Some(p) = std::env::var_os("ProgramFiles(x86)") {
        steam_paths.push(PathBuf::from(p).join("Steam"));
    }
    if let Some(p) = std::env::var_os("ProgramFiles") {
        steam_paths.push(PathBuf::from(p).join("Steam"));
    }

    steam_paths.retain(|p| p.exists());
    steam_paths.sort();
    steam_paths.dedup();
    steam_paths
}

fn load_appinfo_bytes(steam_paths: &[PathBuf]) -> Option<Vec<u8>> {
    for steam_root in steam_paths {
        let appinfo_path = steam_root.join("appcache").join("appinfo.vdf");
        let Ok(bytes) = std::fs::read(&appinfo_path) else {
            continue;
        };
        if !bytes.is_empty() {
            return Some(bytes);
        }
    }
    None
}

fn parse_app_type_token(window: &[u8]) -> Option<&'static str> {
    let mut start: Option<usize> = None;

    for (index, byte) in window.iter().copied().enumerate() {
        let is_printable = (0x20..=0x7e).contains(&byte);
        if is_printable {
            if start.is_none() {
                start = Some(index);
            }
            continue;
        }

        if let Some(token_start) = start.take() {
            let token = std::str::from_utf8(&window[token_start..index]).ok()?;
            if token.eq_ignore_ascii_case("game") {
                return Some("game");
            }
            if token.eq_ignore_ascii_case("application") {
                return Some("application");
            }
            if token.eq_ignore_ascii_case("tool") {
                return Some("tool");
            }
            if token.eq_ignore_ascii_case("demo") {
                return Some("demo");
            }
            if token.eq_ignore_ascii_case("dlc") {
                return Some("dlc");
            }
            if token.eq_ignore_ascii_case("video") {
                return Some("video");
            }
            if token.eq_ignore_ascii_case("music") {
                return Some("music");
            }
        }
    }

    if let Some(token_start) = start {
        let token = std::str::from_utf8(&window[token_start..]).ok()?;
        if token.eq_ignore_ascii_case("game") {
            return Some("game");
        }
        if token.eq_ignore_ascii_case("application") {
            return Some("application");
        }
        if token.eq_ignore_ascii_case("tool") {
            return Some("tool");
        }
        if token.eq_ignore_ascii_case("demo") {
            return Some("demo");
        }
        if token.eq_ignore_ascii_case("dlc") {
            return Some("dlc");
        }
        if token.eq_ignore_ascii_case("video") {
            return Some("video");
        }
        if token.eq_ignore_ascii_case("music") {
            return Some("music");
        }
    }

    None
}

fn window_contains_app_name(window: &[u8], expected_name: &str) -> bool {
    if expected_name.is_empty() {
        return false;
    }

    let haystack = String::from_utf8_lossy(window);
    haystack.contains(expected_name)
}

fn is_game_steam_app_id(
    appinfo_bytes: Option<&[u8]>,
    steam_app_id: u32,
    expected_name: &str,
) -> bool {
    const WINDOW_SIZE: usize = 4096;

    let Some(appinfo_bytes) = appinfo_bytes else {
        return false;
    };

    let needle = steam_app_id.to_le_bytes();
    let mut index = 0usize;

    while index + needle.len() <= appinfo_bytes.len() {
        if appinfo_bytes[index..index + needle.len()] == needle {
            let window_end = (index + WINDOW_SIZE).min(appinfo_bytes.len());
            let window = &appinfo_bytes[index..window_end];
            if !window_contains_app_name(window, expected_name) {
                index += 1;
                continue;
            }
            if let Some(app_type) = parse_app_type_token(window) {
                return app_type == "game";
            }
        }
        index += 1;
    }

    false
}

pub fn scan_games_with_paths(steam_paths: &[PathBuf]) -> Vec<Game> {
    let mut games: Vec<Game> = Vec::new();
    let mut seen_app_ids: HashSet<u32> = HashSet::new();
    let appinfo_bytes = load_appinfo_bytes(steam_paths);
    let library_folders = collect_library_folders(steam_paths);

    let (last_played_map, playtime_map) = parse_userdata(steam_paths);

    for lib in &library_folders {
        if let Ok(entries) = std::fs::read_dir(lib) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !fname.starts_with("appmanifest_") || !fname.ends_with(".acf") {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let vals = parse_acf_values(&content);
                    let steam_app_id = vals.get("appid").and_then(|v| v.parse::<u32>().ok());
                    let name = vals.get("name").cloned().unwrap_or_default();
                    let install_dir = vals.get("installdir").cloned().unwrap_or_default();
                    let state_flags = vals
                        .get("stateflags")
                        .and_then(|v| v.parse::<u32>().ok())
                        .unwrap_or(0);

                    if (state_flags & STEAM_STATE_FLAG_FULLY_INSTALLED) == 0 || name.is_empty() {
                        continue;
                    }
                    if let Some(id) = steam_app_id {
                        if !is_game_steam_app_id(appinfo_bytes.as_deref(), id, &name) {
                            continue;
                        }
                        if !seen_app_ids.insert(id) {
                            continue;
                        }
                        let game_path = lib.join("common").join(&install_dir);
                        games.push(Game {
                            source: GameSource::Steam,
                            name,
                            install_path: game_path,
                            launch_target: None,
                            steam_app_id: Some(id),
                            appx_id: None,
                            epic_app_name: None,
                            xbox_package_family_name: None,
                            last_played: last_played_map.get(&id).copied().unwrap_or(0),
                            playtime_minutes: playtime_map.get(&id).copied().unwrap_or(0),
                            installed_size_bytes: None,
                            dlss_version: None,
                        });
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let uninstall_paths = [
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ];
        let steam_app_re = Regex::new(r"^Steam App (\d+)$").unwrap();
        let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);

        for uninstall_path in &uninstall_paths {
            if let Ok(uninstall_key) = hklm.open_subkey(uninstall_path) {
                for subkey_name in uninstall_key.enum_keys().filter_map(|k| k.ok()) {
                    if let Some(caps) = steam_app_re.captures(&subkey_name) {
                        let steam_app_id: u32 =
                            match caps.get(1).and_then(|m| m.as_str().parse().ok()) {
                                Some(id) => id,
                                None => continue,
                            };
                        if seen_app_ids.contains(&steam_app_id) {
                            continue;
                        }
                        if let Ok(subkey) = uninstall_key.open_subkey(&subkey_name) {
                            let display_name: String =
                                subkey.get_value("DisplayName").unwrap_or_default();
                            let install_location: String =
                                subkey.get_value("InstallLocation").unwrap_or_default();
                            if display_name.is_empty()
                                || !is_game_steam_app_id(
                                    appinfo_bytes.as_deref(),
                                    steam_app_id,
                                    &display_name,
                                )
                            {
                                continue;
                            }
                            let install_path = PathBuf::from(install_location);
                            seen_app_ids.insert(steam_app_id);
                            games.push(Game {
                                source: GameSource::Steam,
                                name: display_name,
                                install_path,
                                launch_target: None,
                                steam_app_id: Some(steam_app_id),
                                appx_id: None,
                                epic_app_name: None,
                                xbox_package_family_name: None,
                                last_played: last_played_map
                                    .get(&steam_app_id)
                                    .copied()
                                    .unwrap_or(0),
                                playtime_minutes: playtime_map
                                    .get(&steam_app_id)
                                    .copied()
                                    .unwrap_or(0),
                                installed_size_bytes: None,
                                dlss_version: None,
                            });
                        }
                    }
                }
            }
        }
    }

    sort_games_by_last_played(&mut games);
    games
}

pub fn load_game_update_progress(
    steam_app_id: u32,
    steam_paths: &[PathBuf],
) -> Option<SteamUpdateProgress> {
    let manifest_path = find_appmanifest_path(steam_app_id, steam_paths)?;
    let content = std::fs::read_to_string(manifest_path).ok()?;
    let values = parse_acf_values(&content);

    let progress = SteamUpdateProgress {
        state_flags: values
            .get("stateflags")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(0),
        bytes_downloaded: parse_acf_u64(&values, "bytesdownloaded"),
        bytes_to_download: parse_acf_u64(&values, "bytestodownload"),
        bytes_staged: parse_acf_u64(&values, "bytesstaged"),
        bytes_to_stage: parse_acf_u64(&values, "bytestostage"),
    };

    progress.needs_update().then_some(progress)
}

fn collect_library_folders(steam_paths: &[PathBuf]) -> Vec<PathBuf> {
    let vdf_re = Regex::new(r#"[\"]([A-Za-z]:\\[^\"]+)[\"]"#).unwrap();
    let mut library_folders: Vec<PathBuf> = Vec::new();

    for steam_root in steam_paths {
        let libfile = steam_root.join("steamapps").join("libraryfolders.vdf");
        if libfile.exists() {
            if let Ok(s) = std::fs::read_to_string(&libfile) {
                for cap in vdf_re.captures_iter(&s) {
                    if let Some(m) = cap.get(1) {
                        let p = PathBuf::from(m.as_str());
                        if p.exists() {
                            library_folders.push(p.join("steamapps"));
                        }
                    }
                }
            }
        }
        library_folders.push(steam_root.join("steamapps"));
    }

    library_folders.retain(|path| path.exists());
    library_folders.sort();
    library_folders.dedup();
    library_folders
}

fn find_appmanifest_path(steam_app_id: u32, steam_paths: &[PathBuf]) -> Option<PathBuf> {
    let manifest_name = format!("appmanifest_{}.acf", steam_app_id);

    collect_library_folders(steam_paths)
        .into_iter()
        .map(|library| library.join(&manifest_name))
        .find(|path| path.is_file())
}

fn parse_acf_values(content: &str) -> HashMap<String, String> {
    let re = Regex::new(r#"\"([^\"]+)\"\s+\"([^\"]*)\""#).unwrap();
    let mut map = HashMap::new();
    for cap in re.captures_iter(content) {
        if let (Some(k), Some(v)) = (cap.get(1), cap.get(2)) {
            map.insert(k.as_str().to_lowercase(), v.as_str().to_string());
        }
    }
    map
}

fn parse_acf_u64(values: &HashMap<String, String>, key: &str) -> u64 {
    values
        .get(key)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
}

fn parse_userdata(steam_paths: &[PathBuf]) -> (HashMap<u32, u64>, HashMap<u32, u32>) {
    parse_userdata_filtered(steam_paths, None)
}

pub fn load_game_playtime_minutes(steam_app_id: u32, steam_paths: &[PathBuf]) -> Option<u32> {
    let (_, playtime) = parse_userdata_filtered(steam_paths, Some(steam_app_id));
    playtime.get(&steam_app_id).copied()
}

pub fn load_game_installed_size(path: &Path) -> Option<u64> {
    if !path.exists() {
        return None;
    }

    let mut total_size = 0u64;
    for entry in walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let Ok(metadata) = entry.metadata() else {
            continue;
        };

        total_size = total_size.saturating_add(metadata.len());
    }

    Some(total_size)
}

fn parse_userdata_filtered(
    steam_paths: &[PathBuf],
    target_app_id: Option<u32>,
) -> (HashMap<u32, u64>, HashMap<u32, u32>) {
    let mut last_played: HashMap<u32, u64> = HashMap::new();
    let mut playtime: HashMap<u32, u32> = HashMap::new();
    let kv_re = Regex::new(r#"\"([^\"]+)\"\s+\"([^\"]*)\""#).unwrap();

    for steam_root in steam_paths {
        let userdata_dir = steam_root.join("userdata");
        if !userdata_dir.exists() {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&userdata_dir) else {
            continue;
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let cfg = entry.path().join("config").join("localconfig.vdf");
            if !cfg.exists() {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&cfg) else {
                continue;
            };

            let Some(apps_pos) = content.find("\"apps\"") else {
                continue;
            };
            let after_apps = &content[apps_pos..];
            let Some(brace_pos) = after_apps.find('{') else {
                continue;
            };
            let apps_content = &after_apps[brace_pos + 1..];

            let mut depth: i32 = 0;
            let mut current_app_id: Option<u32> = None;
            let mut expect_block = false;

            for line in apps_content.lines() {
                let trimmed = line.trim();
                if trimmed == "{" {
                    if expect_block {
                        depth = 1;
                        expect_block = false;
                    } else if depth >= 1 {
                        depth += 1;
                    }
                    continue;
                }
                if trimmed == "}" {
                    if depth > 1 {
                        depth -= 1;
                    } else if depth == 1 {
                        depth = 0;
                        current_app_id = None;
                    } else {
                        break;
                    }
                    continue;
                }
                if depth == 0 {
                    let t = trimmed.trim_matches('"');
                    if let Ok(id) = t.parse::<u32>() {
                        current_app_id = Some(id);
                        expect_block = true;
                    }
                } else if depth == 1 {
                    if let (Some(steam_app_id), Some(cap)) =
                        (current_app_id, kv_re.captures(trimmed))
                    {
                        if target_app_id
                            .map(|target| target != steam_app_id)
                            .unwrap_or(false)
                        {
                            continue;
                        }

                        let key = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                        let val = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                        match key {
                            "LastPlayed" => {
                                if let Ok(ts) = val.parse::<u64>() {
                                    let e = last_played.entry(steam_app_id).or_insert(0);
                                    if ts > *e {
                                        *e = ts;
                                    }
                                }
                            }
                            "Playtime" | "playtime_forever" => {
                                if let Ok(mins) = val.parse::<u32>() {
                                    let e = playtime.entry(steam_app_id).or_insert(0);
                                    if mins > *e {
                                        *e = mins;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    (last_played, playtime)
}

pub(super) fn steamid64_to_accountid(steamid64: &str) -> Option<String> {
    let id: u64 = steamid64.parse().ok()?;
    let account_id = id & 0xFFFF_FFFF;
    if account_id == 0 {
        return None;
    }
    Some(account_id.to_string())
}

pub(super) fn find_most_recent_steam_id(steam_paths: &[PathBuf]) -> Option<String> {
    let kv_re = Regex::new(r#"\"([^\"]+)\"\s+\"([^\"]*)\""#).ok()?;

    for steam_root in steam_paths {
        let loginusers = steam_root.join("config").join("loginusers.vdf");
        let Ok(content) = std::fs::read_to_string(&loginusers) else {
            continue;
        };

        let mut depth: i32 = 0;
        let mut current_id: Option<String> = None;
        let mut expect_user_block = false;
        let mut first_id: Option<String> = None;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "{" {
                if expect_user_block {
                    depth = 1;
                    expect_user_block = false;
                } else if depth >= 1 {
                    depth += 1;
                }
                continue;
            }
            if trimmed == "}" {
                if depth > 1 {
                    depth -= 1;
                } else if depth == 1 {
                    depth = 0;
                    current_id = None;
                }
                continue;
            }

            if depth == 0 {
                let t = trimmed.trim_matches('"');
                if t.len() == 17 && t.chars().all(|c| c.is_ascii_digit()) {
                    if first_id.is_none() {
                        first_id = Some(t.to_string());
                    }
                    current_id = Some(t.to_string());
                    expect_user_block = true;
                }
            } else if depth == 1 {
                if let Some(cap) = kv_re.captures(trimmed) {
                    let key = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                    let val = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                    if key.eq_ignore_ascii_case("MostRecent") && val == "1" {
                        if let Some(id) = current_id.clone() {
                            return Some(id);
                        }
                    }
                }
            }
        }

        if first_id.is_some() {
            return first_id;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        is_game_steam_app_id, parse_app_type_token, parse_userdata_filtered,
        window_contains_app_name,
    };

    fn unique_temp_dir(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "big_screen_launcher_{}_{}_{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        std::env::temp_dir().join(unique)
    }

    fn write_localconfig(root: &Path, user_id: &str, content: &str) {
        let config_dir = root.join("userdata").join(user_id).join("config");
        std::fs::create_dir_all(&config_dir).expect("config dir should be created");
        std::fs::write(config_dir.join("localconfig.vdf"), content)
            .expect("localconfig should be written");
    }

    #[test]
    fn app_type_parser_accepts_exact_game_token() {
        let window = b"The Callisto Protocol\0Game\0windows\0released\0";
        assert_eq!(parse_app_type_token(window), Some("game"));
    }

    #[test]
    fn app_type_parser_rejects_partial_game_words() {
        let window = b"Steampipe Beta\0gameinstall\0windows\0";
        assert_eq!(parse_app_type_token(window), None);
    }

    #[test]
    fn app_name_match_requires_exact_entry_name_in_window() {
        let window = b"Steampipe Beta\0gameinstall\0windows\0";
        assert!(!window_contains_app_name(
            window,
            "Steamworks Common Redistributables"
        ));
        assert!(window_contains_app_name(
            b"Steamworks Common Redistributables\0Tool\0windows\0",
            "Steamworks Common Redistributables"
        ));
    }

    #[test]
    fn game_app_id_requires_matching_name_and_game_type() {
        let steam_app_id = 480u32;
        let mut appinfo = Vec::new();
        appinfo.extend_from_slice(b"prefix");
        appinfo.extend_from_slice(&steam_app_id.to_le_bytes());
        appinfo.extend_from_slice(b"Spacewar\0Game\0windows\0");

        assert!(is_game_steam_app_id(Some(&appinfo), steam_app_id, "Spacewar"));
        assert!(!is_game_steam_app_id(
            Some(&appinfo),
            steam_app_id,
            "Wrong Name"
        ));
    }

    #[test]
    fn game_app_id_rejects_non_game_types() {
        let steam_app_id = 228980u32;
        let mut appinfo = Vec::new();
        appinfo.extend_from_slice(&steam_app_id.to_le_bytes());
        appinfo.extend_from_slice(b"Steamworks Common Redistributables\0Tool\0windows\0");

        assert!(!is_game_steam_app_id(
            Some(&appinfo),
            steam_app_id,
            "Steamworks Common Redistributables"
        ));
    }

    #[test]
    fn parse_userdata_filtered_merges_max_last_played_and_playtime() {
        let steam_root = unique_temp_dir("userdata_merge");
        std::fs::create_dir_all(&steam_root).expect("steam root should be created");

        write_localconfig(
            &steam_root,
            "1001",
            r#"
"UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "480"
                    {
                        "LastPlayed"        "100"
                        "Playtime"          "15"
                    }
                    "570"
                    {
                        "playtime_forever"  "42"
                    }
                }
            }
        }
    }
}
"#,
        );
        write_localconfig(
            &steam_root,
            "1002",
            r#"
"UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "480"
                    {
                        "LastPlayed"        "250"
                        "Playtime"          "10"
                    }
                    "570"
                    {
                        "LastPlayed"        "300"
                        "Playtime"          "40"
                    }
                }
            }
        }
    }
}
"#,
        );

        let (last_played, playtime) = parse_userdata_filtered(&[steam_root.clone()], None);

        assert_eq!(last_played.get(&480), Some(&250));
        assert_eq!(playtime.get(&480), Some(&15));
        assert_eq!(last_played.get(&570), Some(&300));
        assert_eq!(playtime.get(&570), Some(&42));

        let _ = std::fs::remove_dir_all(&steam_root);
    }

    #[test]
    fn parse_userdata_filtered_honors_target_app_id() {
        let steam_root = unique_temp_dir("userdata_filter");
        std::fs::create_dir_all(&steam_root).expect("steam root should be created");

        write_localconfig(
            &steam_root,
            "1001",
            r#"
"apps"
{
    "480"
    {
        "LastPlayed"        "100"
        "Playtime"          "15"
    }
    "570"
    {
        "LastPlayed"        "200"
        "playtime_forever"  "45"
    }
}
"#,
        );

        let (last_played, playtime) = parse_userdata_filtered(&[steam_root.clone()], Some(570));

        assert_eq!(last_played.len(), 1);
        assert_eq!(playtime.len(), 1);
        assert_eq!(last_played.get(&570), Some(&200));
        assert_eq!(playtime.get(&570), Some(&45));
        assert!(!last_played.contains_key(&480));

        let _ = std::fs::remove_dir_all(&steam_root);
    }
}