use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::i18n::AppLanguage;
use serde::{Deserialize, Serialize};

pub struct Game {
    pub name: String,
    pub path: PathBuf,
    pub app_id: Option<u32>,
    pub last_played: u64,
    pub playtime_minutes: u32,
    pub dlss_version: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AchievementItem {
    pub api_name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub unlocked: Option<bool>,
    pub unlock_time: Option<u64>,
    pub global_percent: Option<f32>,
    pub icon_url: Option<String>,
    pub icon_gray_url: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AchievementSummary {
    pub unlocked: Option<u32>,
    pub total: u32,
    pub items: Vec<AchievementItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedAchievementSummary {
    summary: AchievementSummary,
}

fn achievement_cache_dir(language: AppLanguage) -> PathBuf {
    let mut dir = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();
    dir.push("achievement_cache");
    dir.push(language.steam_language_key());
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn achievement_cache_path(app_id: u32, language: AppLanguage) -> PathBuf {
    achievement_cache_dir(language).join(format!("{}.json", app_id))
}

pub fn load_cached_achievement_summary(
    app_id: u32,
    language: AppLanguage,
) -> Option<AchievementSummary> {
    let bytes = std::fs::read(achievement_cache_path(app_id, language)).ok()?;
    let cached = serde_json::from_slice::<CachedAchievementSummary>(&bytes).ok()?;
    if cached.summary.items.is_empty() {
        return None;
    }
    Some(cached.summary)
}

pub fn store_cached_achievement_summary(
    app_id: u32,
    summary: &AchievementSummary,
    language: AppLanguage,
) {
    if summary.items.is_empty() {
        return;
    }

    let cache_path = achievement_cache_path(app_id, language);
    let payload = CachedAchievementSummary {
        summary: summary.clone(),
    };
    if let Ok(bytes) = serde_json::to_vec(&payload) {
        let _ = std::fs::write(cache_path, bytes);
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

fn is_game_app_id(appinfo_bytes: Option<&[u8]>, app_id: u32, expected_name: &str) -> bool {
    const WINDOW_SIZE: usize = 4096;

    let Some(appinfo_bytes) = appinfo_bytes else {
        return false;
    };

    let needle = app_id.to_le_bytes();
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
    use regex::Regex;
    let mut games: Vec<Game> = Vec::new();
    let mut seen_app_ids: HashSet<u32> = HashSet::new();
    let appinfo_bytes = load_appinfo_bytes(steam_paths);

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
                        if !is_game_app_id(appinfo_bytes.as_deref(), id, &name) {
                            continue;
                        }
                        if !seen_app_ids.insert(id) {
                            continue;
                        }
                        let game_path = lib.join("common").join(&install_dir);
                        let dlss_version = crate::dlss::detect_version(&game_path, Some(id));
                        games.push(Game {
                            name,
                            path: game_path,
                            app_id: Some(id),
                            last_played: last_played_map.get(&id).copied().unwrap_or(0),
                            playtime_minutes: playtime_map.get(&id).copied().unwrap_or(0),
                            dlss_version,
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
                            if display_name.is_empty()
                                || !is_game_app_id(appinfo_bytes.as_deref(), app_id, &display_name)
                            {
                                continue;
                            }
                            let install_path = PathBuf::from(install_location);
                            let dlss_version = crate::dlss::detect_version(&install_path, Some(app_id));
                            seen_app_ids.insert(app_id);
                            games.push(Game {
                                name: display_name,
                                path: install_path.clone(),
                                app_id: Some(app_id),
                                last_played: last_played_map.get(&app_id).copied().unwrap_or(0),
                                playtime_minutes: playtime_map.get(&app_id).copied().unwrap_or(0),
                                dlss_version,
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
    Str(String),
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
                let val = bvdf_cstr(data, pos).unwrap_or_default();
                map.insert(key, BvdfVal::Str(val));
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

fn load_local_unlock_status(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> HashMap<String, (bool, Option<u64>)> {
    let Some(account_id) = find_most_recent_steam_id(steam_paths)
        .as_deref()
        .and_then(steamid64_to_accountid) else {
        return HashMap::new();
    };

    // Try userdata files first (per-achievement achieved/time format)
    for steam_root in steam_paths {
        let base = steam_root.join("userdata").join(&account_id).join(app_id.to_string());
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

    // Fallback: try appcache/stats/UserGameStats_<accountid>_<appid>.bin (bitmask format)
    load_appcache_unlock_status(app_id, &account_id, steam_paths, language)
}

/// Parsed achievement info from the schema: api_name and optional display_name.
#[derive(Clone)]
struct SchemaAchInfo {
    api_name: String,
    display_name: Option<String>,
    description: Option<String>,
    icon_url: Option<String>,
    icon_gray_url: Option<String>,
}

/// Parse the schema file to extract achievement bit-index-to-info mappings.
/// Returns a map of section_id -> Vec<(bit_index, SchemaAchInfo)>.
fn load_schema_achievement_bits(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> HashMap<String, Vec<(u32, SchemaAchInfo)>> {
    for steam_root in steam_paths {
        let schema_path = steam_root
            .join("appcache")
            .join("stats")
            .join(format!("UserGameStatsSchema_{}.bin", app_id));
        let Ok(data) = std::fs::read(&schema_path) else { continue };

        for &start in &[0usize, 2, 4, 8] {
            if start >= data.len() { break; }
            let mut pos = start;
            let root = bvdf_node(&data, &mut pos);
            let mut result: HashMap<String, Vec<(u32, SchemaAchInfo)>> = HashMap::new();
            collect_achievement_bits(&root, &mut result, 0, language);
            if !result.is_empty() {
                return result;
            }
        }
    }
    HashMap::new()
}

fn load_schema_achievement_metadata(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> HashMap<String, SchemaAchInfo> {
    for steam_root in steam_paths {
        let schema_path = steam_root
            .join("appcache")
            .join("stats")
            .join(format!("UserGameStatsSchema_{}.bin", app_id));
        let Ok(data) = std::fs::read(&schema_path) else { continue };

        for &start in &[0usize, 2, 4, 8] {
            if start >= data.len() { break; }
            let mut pos = start;
            let root = bvdf_node(&data, &mut pos);
            let mut map: HashMap<String, SchemaAchInfo> = HashMap::new();
            collect_schema_achievement_metadata(&root, &mut map, 0, language);
            if !map.is_empty() {
                return map;
            }
        }
    }
    HashMap::new()
}

fn collect_schema_achievement_metadata(
    node: &HashMap<String, BvdfVal>,
    out: &mut HashMap<String, SchemaAchInfo>,
    depth: u32,
    language: AppLanguage,
) {
    if depth > 8 {
        return;
    }

    for (_, val) in node {
        if let BvdfVal::Nested(inner) = val {
            let api_name = inner
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("name"))
                .and_then(|(_, v)| if let BvdfVal::Str(s) = v { Some(s.clone()) } else { None });

            if let Some(api_name) = api_name {
                let display_name = extract_display_name(inner, language);
                let description = extract_description(inner, language);
                let (icon_url, icon_gray_url) = extract_icon_urls(inner);
                if display_name.is_some() || description.is_some() || icon_url.is_some() || icon_gray_url.is_some() {
                    let e = out.entry(api_name.clone()).or_insert_with(|| SchemaAchInfo {
                        api_name: api_name.clone(),
                        display_name: None,
                        description: None,
                        icon_url: None,
                        icon_gray_url: None,
                    });
                    if e.display_name.is_none() {
                        e.display_name = display_name;
                    }
                    if e.description.is_none() {
                        e.description = description;
                    }
                    if e.icon_url.is_none() {
                        e.icon_url = icon_url;
                    }
                    if e.icon_gray_url.is_none() {
                        e.icon_gray_url = icon_gray_url;
                    }
                }
            }

            collect_schema_achievement_metadata(inner, out, depth + 1, language);
        }
    }
}

fn collect_achievement_bits(
    node: &HashMap<String, BvdfVal>,
    result: &mut HashMap<String, Vec<(u32, SchemaAchInfo)>>,
    depth: u32,
    language: AppLanguage,
) {
    if depth > 6 { return; }
    for (key, val) in node {
        if let BvdfVal::Nested(inner) = val {
            // Check if this node has a "bits" child with achievement entries
            if let Some(BvdfVal::Nested(bits)) = inner.iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("bits"))
                .map(|(_, v)| v)
            {
                let mut entries: Vec<(u32, SchemaAchInfo)> = Vec::new();
                for (bit_key, bit_val) in bits {
                    if let (Ok(bit_idx), BvdfVal::Nested(fields)) = (bit_key.parse::<u32>(), bit_val) {
                        if let Some(BvdfVal::Str(api_name)) = fields.iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case("name"))
                            .map(|(_, v)| v)
                        {
                            if !api_name.is_empty() {
                                let display_name = extract_display_name(fields, language);
                                let description = extract_description(fields, language);
                                let (icon_url, icon_gray_url) = extract_icon_urls(fields);
                                entries.push((bit_idx, SchemaAchInfo {
                                    api_name: api_name.clone(),
                                    display_name,
                                    description,
                                    icon_url,
                                    icon_gray_url,
                                }));
                            }
                        }
                    }
                }
                if !entries.is_empty() {
                    entries.sort_by_key(|(idx, _)| *idx);
                    result.insert(key.clone(), entries);
                }
            }
            collect_achievement_bits(inner, result, depth + 1, language);
        }
    }
}

fn extract_localized_nested_string(
    node: &HashMap<String, BvdfVal>,
    language: AppLanguage,
) -> Option<String> {
    let preferred = language.steam_language_key();

    if let Some(BvdfVal::Str(value)) = node
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(preferred))
        .map(|(_, value)| value)
    {
        if !value.is_empty() {
            return Some(value.clone());
        }
    }

    if !preferred.eq_ignore_ascii_case("english") {
        if let Some(BvdfVal::Str(value)) = node
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case("english"))
            .map(|(_, value)| value)
        {
            if !value.is_empty() {
                return Some(value.clone());
            }
        }
    }

    None
}

/// Extract a localized display name from a bit entry's "display" -> "name" node.
fn extract_display_name(fields: &HashMap<String, BvdfVal>, language: AppLanguage) -> Option<String> {
    let display = fields.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("display"))
        .and_then(|(_, v)| if let BvdfVal::Nested(d) = v { Some(d) } else { None })?;
    let name_node = display.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("name"))
        .and_then(|(_, v)| if let BvdfVal::Nested(n) = v { Some(n) } else { None })?;

    extract_localized_nested_string(name_node, language)
}

fn extract_description(fields: &HashMap<String, BvdfVal>, language: AppLanguage) -> Option<String> {
    let display = fields.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("display"))
        .and_then(|(_, v)| if let BvdfVal::Nested(d) = v { Some(d) } else { None })?;

    let desc_node = display.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("desc") || k.eq_ignore_ascii_case("description"))
        .and_then(|(_, v)| if let BvdfVal::Nested(n) = v { Some(n) } else { None })?;

    extract_localized_nested_string(desc_node, language)
}

fn normalize_schema_icon_value(app_id: u32, raw: &str) -> Option<String> {
    let v = raw.trim();
    if v.is_empty() {
        return None;
    }
    if v.starts_with("http://") || v.starts_with("https://") {
        return Some(v.to_string());
    }
    // Local schema may store icon hash with or without extension.
    if v.ends_with(".jpg") || v.ends_with(".png") {
        return Some(format!(
            "https://cdn.cloudflare.steamstatic.com/steamcommunity/public/images/apps/{}/{}",
            app_id, v
        ));
    }
    Some(format!(
        "https://cdn.cloudflare.steamstatic.com/steamcommunity/public/images/apps/{}/{}.jpg",
        app_id, v
    ))
}

fn extract_icon_urls(fields: &HashMap<String, BvdfVal>) -> (Option<String>, Option<String>) {
    let mut icon = fields
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("icon"))
        .and_then(|(_, v)| if let BvdfVal::Str(s) = v { Some(s.clone()) } else { None });

    let mut icon_gray = fields
        .iter()
        .find(|(k, _)| {
            k.eq_ignore_ascii_case("icon_gray") || k.eq_ignore_ascii_case("icongray")
        })
        .and_then(|(_, v)| if let BvdfVal::Str(s) = v { Some(s.clone()) } else { None });

    // Some schema variants place icon keys under a nested "display" object.
    if icon.is_none() || icon_gray.is_none() {
        if let Some(BvdfVal::Nested(display)) = fields
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("display"))
            .map(|(_, v)| v)
        {
            if icon.is_none() {
                icon = display
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("icon"))
                    .and_then(|(_, v)| if let BvdfVal::Str(s) = v { Some(s.clone()) } else { None });
            }
            if icon_gray.is_none() {
                icon_gray = display
                    .iter()
                    .find(|(k, _)| {
                        k.eq_ignore_ascii_case("icon_gray") || k.eq_ignore_ascii_case("icongray")
                    })
                    .and_then(|(_, v)| if let BvdfVal::Str(s) = v { Some(s.clone()) } else { None });
            }
        }
    }

    (icon, icon_gray)
}

/// Build a map from api_name -> display_name from the schema.
fn load_schema_display_names(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> HashMap<String, String> {
    let bits = load_schema_achievement_bits(app_id, steam_paths, language);
    let metadata = load_schema_achievement_metadata(app_id, steam_paths, language);
    let mut map = HashMap::new();
    for (_, entries) in &bits {
        for (_, info) in entries {
            if let Some(dn) = &info.display_name {
                map.insert(info.api_name.clone(), dn.clone());
            }
        }
    }
    for (api_name, info) in metadata {
        if let Some(dn) = info.display_name {
            map.entry(api_name).or_insert(dn);
        }
    }
    map
}

fn load_schema_descriptions(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> HashMap<String, String> {
    let bits = load_schema_achievement_bits(app_id, steam_paths, language);
    let metadata = load_schema_achievement_metadata(app_id, steam_paths, language);
    let mut map = HashMap::new();
    for (_, entries) in &bits {
        for (_, info) in entries {
            if let Some(description) = &info.description {
                map.insert(info.api_name.clone(), description.clone());
            }
        }
    }
    for (api_name, info) in metadata {
        if let Some(description) = info.description {
            map.entry(api_name).or_insert(description);
        }
    }
    map
}

fn load_schema_icon_urls(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> HashMap<String, (String, String)> {
    let bits = load_schema_achievement_bits(app_id, steam_paths, language);
    let metadata = load_schema_achievement_metadata(app_id, steam_paths, language);
    let mut map = HashMap::new();
    for (_, entries) in &bits {
        for (_, info) in entries {
            let icon = info
                .icon_url
                .as_deref()
                .and_then(|v| normalize_schema_icon_value(app_id, v));
            let icon_gray = info
                .icon_gray_url
                .as_deref()
                .and_then(|v| normalize_schema_icon_value(app_id, v));
            match (icon, icon_gray) {
                (Some(i), Some(g)) => {
                    map.insert(info.api_name.clone(), (i, g));
                }
                (Some(i), None) => {
                    map.insert(info.api_name.clone(), (i.clone(), i));
                }
                (None, Some(g)) => {
                    map.insert(info.api_name.clone(), (g.clone(), g));
                }
                _ => {}
            }
        }
    }
    for (api_name, info) in metadata {
        let icon = info
            .icon_url
            .as_deref()
            .and_then(|v| normalize_schema_icon_value(app_id, v));
        let icon_gray = info
            .icon_gray_url
            .as_deref()
            .and_then(|v| normalize_schema_icon_value(app_id, v));
        match (icon, icon_gray) {
            (Some(i), Some(g)) => {
                map.entry(api_name).or_insert((i, g));
            }
            (Some(i), None) => {
                map.entry(api_name).or_insert((i.clone(), i));
            }
            (None, Some(g)) => {
                map.entry(api_name).or_insert((g.clone(), g));
            }
            _ => {}
        }
    }
    map
}

/// Load achievement unlock status from appcache/stats/UserGameStats_<accountid>_<appid>.bin.
/// This file uses a bitmask format where each stat section's "data" field is a u32
/// bitmask, and "AchievementTimes" contains per-bit timestamps.
fn load_appcache_unlock_status(
    app_id: u32,
    account_id: &str,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> HashMap<String, (bool, Option<u64>)> {
    let bit_mapping = load_schema_achievement_bits(app_id, steam_paths, language);
    if bit_mapping.is_empty() {
        return HashMap::new();
    }

    for steam_root in steam_paths {
        let stats_path = steam_root
            .join("appcache")
            .join("stats")
            .join(format!("UserGameStats_{}_{}.bin", account_id, app_id));
        let Ok(data) = std::fs::read(&stats_path) else { continue };

        for &start in &[0usize, 2, 4, 8] {
            if start >= data.len() { break; }
            let mut pos = start;
            let root = bvdf_node(&data, &mut pos);
            let mut result = HashMap::new();
            extract_bitmask_unlocks(&root, &bit_mapping, &mut result, 0);
            if !result.is_empty() {
                return result;
            }
        }
    }
    HashMap::new()
}

fn extract_bitmask_unlocks(
    node: &HashMap<String, BvdfVal>,
    bit_mapping: &HashMap<String, Vec<(u32, SchemaAchInfo)>>,
    result: &mut HashMap<String, (bool, Option<u64>)>,
    depth: u32,
) {
    if depth > 6 { return; }
    for (key, val) in node {
        if let BvdfVal::Nested(inner) = val {
            if let Some(entries) = bit_mapping.get(key) {
                if let Some(BvdfVal::Int32(data)) = inner.iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("data"))
                    .map(|(_, v)| v)
                {
                    let bitmask = *data as u32;
                    let timestamps: HashMap<u32, u64> = inner.iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case("AchievementTimes"))
                        .and_then(|(_, v)| if let BvdfVal::Nested(times) = v { Some(times) } else { None })
                        .map(|times| {
                            times.iter().filter_map(|(k, v)| {
                                let idx = k.parse::<u32>().ok()?;
                                let ts = match v {
                                    BvdfVal::Int32(t) => Some(*t as u64),
                                    BvdfVal::Uint64(t) => Some(*t),
                                    _ => None,
                                }?;
                                Some((idx, ts))
                            }).collect()
                        })
                        .unwrap_or_default();

                    for (bit_idx, info) in entries {
                        let achieved = (bitmask >> bit_idx) & 1 != 0;
                        let time = if achieved { timestamps.get(bit_idx).copied() } else { None };
                        result.insert(info.api_name.clone(), (achieved, time));
                    }
                }
            }
            extract_bitmask_unlocks(inner, bit_mapping, result, depth + 1);
        }
    }
}

fn load_local_schema_achievement_names(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> Vec<String> {
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

            let localized_name = match language {
                AppLanguage::SimplifiedChinese => zh_name.or(en_name),
                AppLanguage::English => en_name,
            };

            if let Some(n) = localized_name.or(internal_name) {
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

fn load_local_schema_items(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> HashMap<String, AchievementItem> {
    let bits = load_schema_achievement_bits(app_id, steam_paths, language);
    let metadata = load_schema_achievement_metadata(app_id, steam_paths, language);
    let mut items_map = HashMap::new();

    for entries in bits.values() {
        for (_, info) in entries {
            let e = items_map
                .entry(info.api_name.clone())
                .or_insert_with(|| AchievementItem {
                    api_name: info.api_name.clone(),
                    display_name: None,
                    description: None,
                    unlocked: None,
                    unlock_time: None,
                    global_percent: None,
                    icon_url: None,
                    icon_gray_url: None,
                });
            if e.display_name.is_none() {
                e.display_name = info.display_name.clone();
            }
            if e.description.is_none() {
                e.description = info.description.clone();
            }
            if e.icon_url.is_none() {
                e.icon_url = info
                    .icon_url
                    .as_deref()
                    .and_then(|v| normalize_schema_icon_value(app_id, v));
            }
            if e.icon_gray_url.is_none() {
                e.icon_gray_url = info
                    .icon_gray_url
                    .as_deref()
                    .and_then(|v| normalize_schema_icon_value(app_id, v));
            }
        }
    }

    for (api_name, info) in metadata {
        let e = items_map
            .entry(api_name.clone())
            .or_insert_with(|| AchievementItem {
                api_name,
                display_name: None,
                description: None,
                unlocked: None,
                unlock_time: None,
                global_percent: None,
                icon_url: None,
                icon_gray_url: None,
            });
        if e.display_name.is_none() {
            e.display_name = info.display_name;
        }
        if e.description.is_none() {
            e.description = info.description;
        }
        if e.icon_url.is_none() {
            e.icon_url = info
                .icon_url
                .as_deref()
                .and_then(|v| normalize_schema_icon_value(app_id, v));
        }
        if e.icon_gray_url.is_none() {
            e.icon_gray_url = info
                .icon_gray_url
                .as_deref()
                .and_then(|v| normalize_schema_icon_value(app_id, v));
        }
    }

    items_map
}

fn achievement_unlock_sort_rank(unlocked: Option<bool>) -> u8 {
    match unlocked {
        Some(false) => 0,
        None => 1,
        Some(true) => 2,
    }
}

fn achievement_percent_sort_value(global_percent: Option<f32>) -> f32 {
    match global_percent {
        Some(value) if value.is_finite() => value,
        _ => 101.0,
    }
}

pub fn load_achievement_summary(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> Option<AchievementSummary> {
    // Phase 1 — local Binary VDF unlock state
    let local_unlocks = load_local_unlock_status(app_id, steam_paths, language);

    // Phase 2 — local schema defines the full achievement list and metadata.
    let mut items_map = load_local_schema_items(app_id, steam_paths, language);

    // Phase 3 — merge local unlock flags into items_map
    let mut unlocked_count: Option<u32> = None;
    if !local_unlocks.is_empty() {
        let mut count = 0u32;
        for (api_name, (achieved, time)) in &local_unlocks {
            let e = items_map.entry(api_name.clone()).or_insert_with(|| AchievementItem {
                api_name: api_name.clone(),
                display_name: None,
                description: None,
                unlocked: None,
                unlock_time: None,
                global_percent: None,
                icon_url: None,
                icon_gray_url: None,
            });
            e.unlocked = Some(*achieved);
            e.unlock_time = *time;
            if *achieved { count += 1; }
        }
        unlocked_count = Some(count);
    }

    // Phase 4 — fallback token scan for schema variants that do not parse via Binary VDF.
    if items_map.is_empty() {
        let names = load_local_schema_achievement_names(app_id, steam_paths, language);
        for name in names {
            items_map.insert(
                name.clone(),
                AchievementItem {
                    api_name: name,
                    display_name: None,
                    description: None,
                    unlocked: None,
                    unlock_time: None,
                    global_percent: None,
                    icon_url: None,
                    icon_gray_url: None,
                },
            );
        }
    }

    if items_map.is_empty() {
        return None;
    }

    // Phase 5 — apply any remaining metadata from local schema helpers.
    let display_names = load_schema_display_names(app_id, steam_paths, language);
    let descriptions = load_schema_descriptions(app_id, steam_paths, language);
    let schema_icons = load_schema_icon_urls(app_id, steam_paths, language);
    for item in items_map.values_mut() {
        if item.display_name.is_none() {
            if let Some(dn) = display_names.get(&item.api_name) {
                item.display_name = Some(dn.clone());
            }
        }
        if item.description.is_none() {
            if let Some(description) = descriptions.get(&item.api_name) {
                item.description = Some(description.clone());
            }
        }
        if item.icon_url.is_none() || item.icon_gray_url.is_none() {
            if let Some((icon, gray)) = schema_icons.get(&item.api_name) {
                if item.icon_url.is_none() {
                    item.icon_url = Some(icon.clone());
                }
                if item.icon_gray_url.is_none() {
                    item.icon_gray_url = Some(gray.clone());
                }
            }
        }
    }

    let mut items: Vec<AchievementItem> = items_map.into_values().collect();
    items.sort_by(|a, b| {
        achievement_unlock_sort_rank(a.unlocked)
            .cmp(&achievement_unlock_sort_rank(b.unlocked))
            .then_with(|| {
                achievement_percent_sort_value(a.global_percent)
                    .total_cmp(&achievement_percent_sort_value(b.global_percent))
            })
            .then_with(|| a.api_name.cmp(&b.api_name))
    });

    Some(AchievementSummary {
        unlocked: unlocked_count,
        total: items.len() as u32,
        items,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        achievement_percent_sort_value, achievement_unlock_sort_rank, parse_app_type_token,
        window_contains_app_name, AchievementItem,
    };

    #[test]
    fn achievement_sort_handles_mixed_states_and_nan() {
        let mut items = vec![
            AchievementItem {
                api_name: "unknown".to_string(),
                unlocked: None,
                global_percent: Some(2.0),
                ..AchievementItem::default()
            },
            AchievementItem {
                api_name: "unlocked".to_string(),
                unlocked: Some(true),
                global_percent: Some(1.0),
                ..AchievementItem::default()
            },
            AchievementItem {
                api_name: "locked".to_string(),
                unlocked: Some(false),
                global_percent: Some(f32::NAN),
                ..AchievementItem::default()
            },
        ];

        items.sort_by(|a, b| {
            achievement_unlock_sort_rank(a.unlocked)
                .cmp(&achievement_unlock_sort_rank(b.unlocked))
                .then_with(|| {
                    achievement_percent_sort_value(a.global_percent)
                        .total_cmp(&achievement_percent_sort_value(b.global_percent))
                })
                .then_with(|| a.api_name.cmp(&b.api_name))
        });

        let names: Vec<_> = items.into_iter().map(|item| item.api_name).collect();
        assert_eq!(names, vec!["locked", "unknown", "unlocked"]);
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
        assert!(!window_contains_app_name(window, "Steamworks Common Redistributables"));
        assert!(window_contains_app_name(
            b"Steamworks Common Redistributables\0Tool\0windows\0",
            "Steamworks Common Redistributables"
        ));
    }
}

fn steamid64_to_accountid(steamid64: &str) -> Option<String> {
    let id: u64 = steamid64.parse().ok()?;
    let account_id = id & 0xFFFF_FFFF;
    if account_id == 0 { return None; }
    Some(account_id.to_string())
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
