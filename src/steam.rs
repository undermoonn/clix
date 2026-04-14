use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub struct Game {
    pub name: String,
    pub path: PathBuf,
    pub app_id: Option<u32>,
    pub last_played: u64,
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

pub fn scan_games_with_paths(steam_paths: &[PathBuf]) -> Vec<Game> {
    use regex::Regex;
    let mut games: Vec<Game> = Vec::new();
    let mut seen_app_ids: HashSet<u32> = HashSet::new();

    // Step 1: Collect all Steam library folders from libraryfolders.vdf
    let vdf_re = Regex::new(r#"[\"]([A-Za-z]:\\[^\"]+)[\"]"#).unwrap();
    let mut library_folders: Vec<PathBuf> = Vec::new();

    for steam_root in steam_paths.iter() {
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

    library_folders.retain(|p| p.exists());
    library_folders.sort();
    library_folders.dedup();

    // Step 2: Parse LastPlayed from userdata
    let last_played_map = parse_last_played_from_userdata(steam_paths);

    // Step 3: Parse appmanifest_*.acf files
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
                    let app_id = vals.get("appid").and_then(|v| v.parse::<u32>().ok());
                    let name = vals.get("name").cloned().unwrap_or_default();
                    let install_dir = vals.get("installdir").cloned().unwrap_or_default();
                    let state_flags = vals
                        .get("stateflags")
                        .and_then(|v| v.parse::<u32>().ok())
                        .unwrap_or(0);

                    if (state_flags & 4) == 0 || name.is_empty() {
                        continue;
                    }
                    if let Some(id) = app_id {
                        if !seen_app_ids.insert(id) {
                            continue;
                        }
                        let game_path = lib.join("common").join(&install_dir);
                        games.push(Game {
                            name,
                            path: game_path,
                            app_id: Some(id),
                            last_played: last_played_map.get(&id).copied().unwrap_or(0),
                        });
                    }
                }
            }
        }
    }

    // Step 4: Supplement from Windows Uninstall registry keys
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
                        let app_id: u32 =
                            match caps.get(1).and_then(|m| m.as_str().parse().ok()) {
                                Some(id) => id,
                                None => continue,
                            };
                        if seen_app_ids.contains(&app_id) {
                            continue;
                        }
                        if let Ok(subkey) = uninstall_key.open_subkey(&subkey_name) {
                            let display_name: String =
                                subkey.get_value("DisplayName").unwrap_or_default();
                            let install_location: String =
                                subkey.get_value("InstallLocation").unwrap_or_default();
                            if display_name.is_empty() {
                                continue;
                            }
                            seen_app_ids.insert(app_id);
                            games.push(Game {
                                name: display_name,
                                path: PathBuf::from(install_location),
                                app_id: Some(app_id),
                                last_played: last_played_map.get(&app_id).copied().unwrap_or(0),
                            });
                        }
                    }
                }
            }
        }
    }

    // Step 5: Sort by last played time descending, then by name
    games.sort_by(|a, b| {
        b.last_played
            .cmp(&a.last_played)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    games
}

fn parse_acf_values(content: &str) -> HashMap<String, String> {
    use regex::Regex;
    let re = Regex::new(r#""([^"]+)"\s+"([^"]*)""#).unwrap();
    let mut map = HashMap::new();
    for cap in re.captures_iter(content) {
        if let (Some(k), Some(v)) = (cap.get(1), cap.get(2)) {
            map.insert(k.as_str().to_lowercase(), v.as_str().to_string());
        }
    }
    map
}

fn parse_last_played_from_userdata(steam_paths: &[PathBuf]) -> HashMap<u32, u64> {
    use regex::Regex;
    let mut map: HashMap<u32, u64> = HashMap::new();

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

            let app_block_re = Regex::new(
                r#"(?m)^\s*"(\d+)"\s*\n\s*\{[^}]*?"LastPlayed"\s+"(\d+)"#,
            )
            .unwrap();

            if let Some(apps_pos) = content.find("\"apps\"") {
                let after_apps = &content[apps_pos..];
                if let Some(brace_pos) = after_apps.find('{') {
                    let apps_content = &after_apps[brace_pos..];
                    for cap in app_block_re.captures_iter(apps_content) {
                        if let (Some(id_m), Some(ts_m)) = (cap.get(1), cap.get(2)) {
                            if let (Ok(app_id), Ok(ts)) = (
                                id_m.as_str().parse::<u32>(),
                                ts_m.as_str().parse::<u64>(),
                            ) {
                                let entry = map.entry(app_id).or_insert(0);
                                if ts > *entry {
                                    *entry = ts;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    map
}
