use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::Deserialize;

pub struct Game {
    pub name: String,
    pub path: PathBuf,
    pub app_id: Option<u32>,
    pub last_played: u64,
    pub playtime_minutes: u32,
}

#[derive(Clone, Debug, Default)]
pub struct AchievementItem {
    pub api_name: String,
    pub unlocked: Option<bool>,
    pub unlock_time: Option<u64>,
    pub global_percent: Option<f32>,
}

#[derive(Clone, Debug, Default)]
pub struct AchievementSummary {
    pub unlocked: Option<u32>,
    pub total: u32,
    pub items: Vec<AchievementItem>,
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

    // Step 2: Parse LastPlayed and Playtime from userdata
    let (last_played_map, playtime_map) = parse_userdata(steam_paths);

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
                            playtime_minutes: playtime_map.get(&id).copied().unwrap_or(0),
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
                                playtime_minutes: playtime_map.get(&app_id).copied().unwrap_or(0),
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

fn parse_userdata(steam_paths: &[PathBuf]) -> (HashMap<u32, u64>, HashMap<u32, u32>) {
    use regex::Regex;
    let mut last_played: HashMap<u32, u64> = HashMap::new();
    let mut playtime: HashMap<u32, u32> = HashMap::new();
    let kv_re = Regex::new(r#""([^"]+)"\s+"([^"]*)""#).unwrap();

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

            // Depth-aware parsing: depth 0 = inside "apps", depth 1 = inside an app block
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
                        depth += 1; // nested block inside app
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
                        break; // closed "apps" block
                    }
                    continue;
                }
                if depth == 0 {
                    // At apps level, look for app ID
                    let t = trimmed.trim_matches('"');
                    if let Ok(id) = t.parse::<u32>() {
                        current_app_id = Some(id);
                        expect_block = true;
                    }
                } else if depth == 1 {
                    // Inside app block at top level, extract key-value pairs
                    if let (Some(app_id), Some(cap)) = (current_app_id, kv_re.captures(trimmed)) {
                        let key = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                        let val = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                        match key {
                            "LastPlayed" => {
                                if let Ok(ts) = val.parse::<u64>() {
                                    let e = last_played.entry(app_id).or_insert(0);
                                    if ts > *e {
                                        *e = ts;
                                    }
                                }
                            }
                            "Playtime" | "playtime_forever" => {
                                if let Ok(mins) = val.parse::<u32>() {
                                    let e = playtime.entry(app_id).or_insert(0);
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

#[derive(Deserialize)]
struct GlobalPercentResponse {
    achievementpercentages: Option<GlobalPercentContainer>,
}

#[derive(Deserialize)]
struct GlobalPercentContainer {
    achievements: Option<Vec<GlobalPercentItem>>,
}

#[derive(Deserialize)]
struct GlobalPercentItem {
    name: String,
    percent: f32,
}

#[derive(Deserialize)]
struct PlayerAchievementsResponse {
    playerstats: Option<PlayerStatsContainer>,
}

#[derive(Deserialize)]
struct PlayerStatsContainer {
    achievements: Option<Vec<PlayerAchievementItem>>,
}

#[derive(Deserialize)]
struct PlayerAchievementItem {
    apiname: String,
    achieved: u8,
    unlocktime: Option<u64>,
}

pub fn load_achievement_summary(app_id: u32, steam_paths: &[PathBuf]) -> Option<AchievementSummary> {
    let mut items_map: HashMap<String, AchievementItem> = HashMap::new();

    let global_url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2/?gameid={}",
        app_id
    );

    if let Ok(resp) = ureq::get(&global_url).call() {
        if resp.status() == 200 {
            if let Ok(body) = resp.into_string() {
                if let Ok(parsed) = serde_json::from_str::<GlobalPercentResponse>(&body) {
                    if let Some(list) = parsed
                        .achievementpercentages
                        .and_then(|x| x.achievements)
                    {
                        for item in list {
                            items_map.insert(
                                item.name.clone(),
                                AchievementItem {
                                    api_name: item.name,
                                    unlocked: None,
                                    unlock_time: None,
                                    global_percent: Some(item.percent),
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    let mut unlocked_count: Option<u32> = None;
    let steam_id = find_most_recent_steam_id(steam_paths);
    let api_key = std::env::var("STEAM_WEB_API_KEY").ok();

    if let (Some(steam_id), Some(api_key)) = (steam_id, api_key) {
        let player_url = format!(
            "https://api.steampowered.com/ISteamUserStats/GetPlayerAchievements/v1/?steamid={}&appid={}&key={}",
            steam_id, app_id, api_key
        );

        if let Ok(resp) = ureq::get(&player_url).call() {
            if resp.status() == 200 {
                if let Ok(body) = resp.into_string() {
                    if let Ok(parsed) = serde_json::from_str::<PlayerAchievementsResponse>(&body)
                    {
                        if let Some(player_items) = parsed.playerstats.and_then(|x| x.achievements)
                        {
                            let mut unlocked_local = 0_u32;
                            for pa in player_items {
                                let achieved = pa.achieved != 0;
                                if achieved {
                                    unlocked_local += 1;
                                }
                                let e = items_map
                                    .entry(pa.apiname.clone())
                                    .or_insert_with(|| AchievementItem {
                                        api_name: pa.apiname.clone(),
                                        unlocked: None,
                                        unlock_time: None,
                                        global_percent: None,
                                    });
                                e.unlocked = Some(achieved);
                                e.unlock_time = pa.unlocktime;
                            }
                            unlocked_count = Some(unlocked_local);
                        }
                    }
                }
            }
        }
    }

    if items_map.is_empty() {
        return None;
    }

    let mut items: Vec<AchievementItem> = items_map.into_values().collect();
    items.sort_by(|a, b| match (a.unlocked, b.unlocked) {
        (Some(false), Some(true)) => std::cmp::Ordering::Less,
        (Some(true), Some(false)) => std::cmp::Ordering::Greater,
        _ => {
            let ap = a.global_percent.unwrap_or(101.0);
            let bp = b.global_percent.unwrap_or(101.0);
            ap.partial_cmp(&bp)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.api_name.cmp(&b.api_name))
        }
    });

    Some(AchievementSummary {
        unlocked: unlocked_count,
        total: items.len() as u32,
        items,
    })
}

fn find_most_recent_steam_id(steam_paths: &[PathBuf]) -> Option<String> {
    let kv_re = regex::Regex::new(r#""([^"]+)"\s+"([^"]*)""#).ok()?;

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
