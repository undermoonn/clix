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

#[derive(Clone, Debug, Default)]
pub struct AchievementDebugInfo {
    pub steam_id_found: bool,
    pub api_key_present: bool,
    pub schema_exists: bool,
    pub schema_name_count: usize,
    pub local_unlock_file_exists: bool,
    pub local_unlock_count: usize,
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

// ─── Binary VDF (Steam KeyValues binary format) parser ──────────────────────

enum BvdfVal {
    Nested(HashMap<String, BvdfVal>),
    Str,
    Int32(i32),
    Uint64(u64),
}

fn bvdf_cstr(data: &[u8], pos: &mut usize) -> Option<String> {
    let start = *pos;
    while *pos < data.len() && data[*pos] != 0 {
        *pos += 1;
    }
    if *pos >= data.len() {
        return None;
    }
    let s = String::from_utf8_lossy(&data[start..*pos]).into_owned();
    *pos += 1;
    Some(s)
}

fn bvdf_node(data: &[u8], pos: &mut usize) -> HashMap<String, BvdfVal> {
    let mut map = HashMap::new();
    loop {
        if *pos >= data.len() { break; }
        let ty = data[*pos];
        *pos += 1;
        if ty == 0x08 { break; }
        let Some(key) = bvdf_cstr(data, pos) else { break };
        match ty {
            0x00 => { map.insert(key, BvdfVal::Nested(bvdf_node(data, pos))); }
            0x01 => {
                let _ = bvdf_cstr(data, pos);
                map.insert(key, BvdfVal::Str);
            }
            0x02 => {
                if *pos + 4 > data.len() { break; }
                let v = i32::from_le_bytes([data[*pos],data[*pos+1],data[*pos+2],data[*pos+3]]);
                *pos += 4;
                map.insert(key, BvdfVal::Int32(v));
            }
            0x07 => {
                if *pos + 8 > data.len() { break; }
                let v = u64::from_le_bytes([data[*pos],data[*pos+1],data[*pos+2],data[*pos+3],
                                            data[*pos+4],data[*pos+5],data[*pos+6],data[*pos+7]]);
                *pos += 8;
                map.insert(key, BvdfVal::Uint64(v));
            }
            0x03 | 0x04 | 0x06 => {
                if *pos + 4 > data.len() { break; }
                *pos += 4;
            }
            0x05 => {
                while *pos + 1 < data.len() {
                    if data[*pos] == 0 && data[*pos+1] == 0 { *pos += 2; break; }
                    *pos += 2;
                }
            }
            _ => break,
        }
    }
    map
}

fn extract_ach_unlocks(m: &HashMap<String, BvdfVal>) -> HashMap<String, (bool, Option<u64>)> {
    let mut out = HashMap::new();
    for (name, val) in m {
        if let BvdfVal::Nested(fields) = val {
            let achieved = fields.iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("achieved"))
                .map(|(_, v)| matches!(v, BvdfVal::Int32(n) if *n != 0))
                .unwrap_or(false);
            let time = fields.iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("time"))
                .and_then(|(_, v)| match v {
                    BvdfVal::Int32(t) => Some(*t as u64),
                    BvdfVal::Uint64(t) => Some(*t),
                    _ => None,
                });
            out.insert(name.clone(), (achieved, time));
        }
    }
    out
}

fn ach_map_unlocks(root: &HashMap<String, BvdfVal>) -> Option<HashMap<String, (bool, Option<u64>)>> {
    // Direct: root["achievements"]
    if let Some(BvdfVal::Nested(m)) = root.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("achievements")).map(|(_, v)| v)
    {
        let u = extract_ach_unlocks(m);
        if !u.is_empty() { return Some(u); }
    }
    // One level deeper
    for (_, val) in root {
        if let BvdfVal::Nested(inner) = val {
            if let Some(BvdfVal::Nested(m)) = inner.iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("achievements")).map(|(_, v)| v)
            {
                let u = extract_ach_unlocks(m);
                if !u.is_empty() { return Some(u); }
            }
        }
    }
    None
}

fn load_local_unlock_status(app_id: u32, steam_paths: &[PathBuf]) -> HashMap<String, (bool, Option<u64>)> {
    let Some(steam_id) = find_most_recent_steam_id(steam_paths) else {
        return HashMap::new();
    };
    for steam_root in steam_paths {
        let base = steam_root.join("userdata").join(&steam_id).join(app_id.to_string());
        let candidates = [
            base.join("stats").join("UserGameStats.bin"),
            base.join("local").join("achievements.bin"),
        ];
        for path in &candidates {
            let Ok(data) = std::fs::read(path) else { continue };
            for &start in &[0usize, 2, 4, 8] {
                if start >= data.len() { break; }
                let mut pos = start;
                let root = bvdf_node(&data, &mut pos);
                if let Some(unlocks) = ach_map_unlocks(&root) {
                    return unlocks;
                }
            }
        }
    }
    HashMap::new()
}

fn local_unlock_file_exists(app_id: u32, steam_paths: &[PathBuf]) -> bool {
    let Some(steam_id) = find_most_recent_steam_id(steam_paths) else {
        return false;
    };

    for steam_root in steam_paths {
        let base = steam_root.join("userdata").join(&steam_id).join(app_id.to_string());
        let candidates = [
            base.join("stats").join("UserGameStats.bin"),
            base.join("local").join("achievements.bin"),
        ];

        if candidates.iter().any(|p| p.exists()) {
            return true;
        }
    }

    false
}

fn load_local_schema_achievement_names(app_id: u32, steam_paths: &[PathBuf]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let is_language_key = |s: &str| {
        matches!(
            s.to_ascii_lowercase().as_str(),
            "english"
                | "schinese"
                | "tchinese"
                | "german"
                | "french"
                | "italian"
                | "koreana"
                | "spanish"
                | "russian"
                | "japanese"
                | "portuguese"
                | "brazilian"
                | "latam"
                | "polish"
                | "danish"
                | "dutch"
                | "finnish"
                | "norwegian"
                | "swedish"
                | "turkish"
                | "thai"
                | "arabic"
        )
    };

    for steam_root in steam_paths {
        let schema_path = steam_root
            .join("appcache")
            .join("stats")
            .join(format!("UserGameStatsSchema_{}.bin", app_id));
        let Ok(bytes) = std::fs::read(&schema_path) else {
            continue;
        };

        // Schema file includes many null-terminated strings; harvest display names.
        // Binary VDF type markers (0x00, 0x01, 0x02, etc.) get prepended to key
        // tokens after splitting on '\0'. Strip leading/trailing control chars so
        // keyword comparisons like "english", "schinese" succeed.
        let text = String::from_utf8_lossy(&bytes);
        let tokens: Vec<String> = text
            .split('\0')
            .map(|t| {
                t.trim_matches(|c: char| c.is_ascii_control() || c.is_whitespace())
                    .to_string()
            })
            .collect();

        // Robust strategy: find token names like NEW_ACHIEVEMENT_*_NAME,
        // then inspect a small neighborhood around the token for localized names.
        for i in 0..tokens.len() {
            let tok = tokens[i].trim();
            if !tok.starts_with("NEW_ACHIEVEMENT_") || !tok.ends_with("_NAME") {
                continue;
            }

            let mut zh_name: Option<String> = None;
            let mut en_name: Option<String> = None;
            let mut internal_name: Option<String> = None;

            let start = i.saturating_sub(24);
            let end = (i + 24).min(tokens.len().saturating_sub(1));

            // Part A: inspect the title area before the token.
            for j in (start..i).rev() {
                let key = tokens[j].trim();
                if key.eq_ignore_ascii_case("display") {
                    break;
                }

                if key.eq_ignore_ascii_case("schinese") && j + 1 < i {
                    let v = tokens[j + 1].trim();
                    if !v.is_empty() {
                        zh_name = Some(v.to_string());
                    }
                } else if key.eq_ignore_ascii_case("english") && j + 1 < i {
                    let v = tokens[j + 1].trim();
                    if !v.is_empty() {
                        en_name = Some(v.to_string());
                    }
                } else if key.eq_ignore_ascii_case("name") && j + 1 < i {
                    let next = tokens[j + 1].trim();
                    if next.eq_ignore_ascii_case("schinese") && j + 2 < i {
                        let v = tokens[j + 2].trim();
                        if !v.is_empty() && !v.starts_with("NEW_ACHIEVEMENT_") {
                            zh_name = Some(v.to_string());
                        }
                    } else if next.eq_ignore_ascii_case("english") && j + 2 < i {
                        let v = tokens[j + 2].trim();
                        if !v.is_empty() && !v.starts_with("NEW_ACHIEVEMENT_") {
                            en_name = Some(v.to_string());
                        }
                    } else if !next.is_empty()
                        && !next.eq_ignore_ascii_case("display")
                        && !is_language_key(next)
                        && !next.starts_with("NEW_ACHIEVEMENT_")
                    {
                        internal_name = Some(next.to_string());
                    }
                }

            }

            // Part B: inspect the localized name list after the token, but stop at desc.
            for j in (i + 1)..=end {
                let key = tokens[j].trim();
                if key.eq_ignore_ascii_case("desc")
                    || key.eq_ignore_ascii_case("hidden")
                    || key.eq_ignore_ascii_case("icon")
                    || key.eq_ignore_ascii_case("icon_gray")
                    || key.eq_ignore_ascii_case("bit")
                {
                    break;
                }

                if key.eq_ignore_ascii_case("schinese") && j < end {
                    let v = tokens[j + 1].trim();
                    if !v.is_empty() {
                        zh_name = Some(v.to_string());
                    }
                } else if key.eq_ignore_ascii_case("english") && j < end {
                    let v = tokens[j + 1].trim();
                    if !v.is_empty() {
                        en_name = Some(v.to_string());
                    }
                }
            }

            if let Some(n) = zh_name.or(en_name).or(internal_name) {
                if !names.contains(&n) {
                    names.push(n);
                }
            }
        }

        if !names.is_empty() {
            return names;
        }
    }

    names
}

pub fn get_achievement_debug_info(app_id: u32, steam_paths: &[PathBuf]) -> AchievementDebugInfo {
    let schema_exists = steam_paths.iter().any(|steam_root| {
        steam_root
            .join("appcache")
            .join("stats")
            .join(format!("UserGameStatsSchema_{}.bin", app_id))
            .exists()
    });

    let schema_name_count = load_local_schema_achievement_names(app_id, steam_paths).len();
    let local_unlocks = load_local_unlock_status(app_id, steam_paths);

    AchievementDebugInfo {
        steam_id_found: find_most_recent_steam_id(steam_paths).is_some(),
        api_key_present: std::env::var("STEAM_WEB_API_KEY").ok().is_some(),
        schema_exists,
        schema_name_count,
        local_unlock_file_exists: local_unlock_file_exists(app_id, steam_paths),
        local_unlock_count: local_unlocks.len(),
    }
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
    // Phase 1 — local Binary VDF (instant, no network, no API key)
    let local_unlocks = load_local_unlock_status(app_id, steam_paths);

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(8))
        .timeout_read(std::time::Duration::from_secs(12))
        .build();

    // Phase 2 — global achievement names + percentages (no API key needed)
    let mut items_map: HashMap<String, AchievementItem> = HashMap::new();
    let global_url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2/?gameid={}",
        app_id
    );
    if let Ok(resp) = agent.get(&global_url).call() {
        if resp.status() == 200 {
            if let Ok(body) = resp.into_string() {
                if let Ok(parsed) = serde_json::from_str::<GlobalPercentResponse>(&body) {
                    if let Some(list) = parsed.achievementpercentages.and_then(|x| x.achievements) {
                        for item in list {
                            items_map.insert(item.name.clone(), AchievementItem {
                                api_name: item.name,
                                unlocked: None,
                                unlock_time: None,
                                global_percent: Some(item.percent),
                            });
                        }
                    }
                }
            }
        }
    }

    // Phase 3 — merge local unlock flags into items_map
    let mut unlocked_count: Option<u32> = None;
    if !local_unlocks.is_empty() {
        let mut count = 0u32;
        for (api_name, (achieved, time)) in &local_unlocks {
            let e = items_map.entry(api_name.clone()).or_insert_with(|| AchievementItem {
                api_name: api_name.clone(),
                unlocked: None,
                unlock_time: None,
                global_percent: None,
            });
            e.unlocked = Some(*achieved);
            e.unlock_time = *time;
            if *achieved { count += 1; }
        }
        unlocked_count = Some(count);
    }

    // Phase 4 — optional: GetPlayerAchievements with API key (overrides local)
    let steam_id = find_most_recent_steam_id(steam_paths);
    let api_key = std::env::var("STEAM_WEB_API_KEY").ok();
    if let (Some(steam_id), Some(api_key)) = (steam_id, api_key) {
        let player_url = format!(
            "https://api.steampowered.com/ISteamUserStats/GetPlayerAchievements/v1/?steamid={}&appid={}&key={}",
            steam_id, app_id, api_key
        );
        if let Ok(resp) = agent.get(&player_url).call() {
            if resp.status() == 200 {
                if let Ok(body) = resp.into_string() {
                    if let Ok(parsed) = serde_json::from_str::<PlayerAchievementsResponse>(&body) {
                        if let Some(player_items) = parsed.playerstats.and_then(|x| x.achievements) {
                            let mut api_count = 0u32;
                            for pa in player_items {
                                let achieved = pa.achieved != 0;
                                if achieved { api_count += 1; }
                                let e = items_map.entry(pa.apiname.clone()).or_insert_with(|| AchievementItem {
                                    api_name: pa.apiname.clone(),
                                    unlocked: None,
                                    unlock_time: None,
                                    global_percent: None,
                                });
                                e.unlocked = Some(achieved);
                                e.unlock_time = pa.unlocktime;
                            }
                            unlocked_count = Some(api_count);
                        }
                    }
                }
            }
        }
    }

    // Phase 5 — fallback to local schema file for offline achievement list.
    if items_map.is_empty() {
        let names = load_local_schema_achievement_names(app_id, steam_paths);
        for name in names {
            items_map.insert(
                name.clone(),
                AchievementItem {
                    api_name: name,
                    unlocked: None,
                    unlock_time: None,
                    global_percent: None,
                },
            );
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
