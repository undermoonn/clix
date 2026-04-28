use std::collections::HashSet;
use std::path::PathBuf;

use regex::Regex;

use crate::game::{sort_games_by_last_played, Game, GameSource};

const STEAM_STATE_FLAG_FULLY_INSTALLED: u32 = 0x4;

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

pub fn scan_games_with_paths(steam_paths: &[PathBuf]) -> Vec<Game> {
    let mut games: Vec<Game> = Vec::new();
    let mut seen_app_ids: HashSet<u32> = HashSet::new();
    let appinfo_bytes = crate::steam::library::load_appinfo_bytes(steam_paths);
    let library_folders = crate::steam::library::collect_library_folders(steam_paths);

    let (last_played_map, playtime_map) = crate::steam::library::parse_userdata(steam_paths);

    for lib in &library_folders {
        if let Ok(entries) = std::fs::read_dir(lib) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !fname.starts_with("appmanifest_") || !fname.ends_with(".acf") {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let vals = crate::steam::library::parse_acf_values(&content);
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
                        if !crate::steam::library::is_game_steam_app_id(
                            appinfo_bytes.as_deref(),
                            id,
                            &name,
                        ) {
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
                                || !crate::steam::library::is_game_steam_app_id(
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
