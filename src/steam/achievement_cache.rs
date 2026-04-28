use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

use crate::assets::cache;
use crate::i18n::AppLanguage;

use super::types::AchievementSummary;

#[derive(Debug, Serialize, Deserialize)]
struct CachedAchievementSummary {
    summary: AchievementSummary,
}

#[derive(Debug, Deserialize)]
struct GlobalAchievementPercentagesResponse {
    achievementpercentages: GlobalAchievementPercentagesPayload,
}

#[derive(Debug, Deserialize)]
struct GlobalAchievementPercentagesPayload {
    achievements: Vec<GlobalAchievementPercentageEntry>,
}

#[derive(Debug, Deserialize)]
struct GlobalAchievementPercentageEntry {
    name: String,
    #[serde(deserialize_with = "deserialize_percent_value")]
    percent: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedGlobalAchievementPercentages {
    fetched_at_unix_secs: u64,
    percentages: HashMap<String, f32>,
}

const GLOBAL_ACHIEVEMENT_PERCENTAGES_CACHE_TTL_SECS: u64 = 60 * 60;

static GLOBAL_ACHIEVEMENT_PERCENTAGES_REFRESHES: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();
static GLOBAL_ACHIEVEMENT_PERCENTAGES_UPDATED: OnceLock<Mutex<Vec<u32>>> = OnceLock::new();

fn deserialize_percent_value<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum PercentValue {
        Number(f32),
        Text(String),
    }

    match PercentValue::deserialize(deserializer)? {
        PercentValue::Number(value) => Ok(value),
        PercentValue::Text(text) => text.trim().parse::<f32>().map_err(de::Error::custom),
    }
}

fn achievement_cache_dir(language: AppLanguage) -> PathBuf {
    let mut dir = cache::cache_subdir("achievement_cache");
    dir.push(language.steam_language_key());
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn achievement_cache_path(steam_app_id: u32, language: AppLanguage) -> PathBuf {
    achievement_cache_dir(language).join(format!("{}.json", steam_app_id))
}

fn achievement_diagnostics_log_path() -> PathBuf {
    cache::cache_subdir("achievement_cache").join("diagnostics.log")
}

fn global_achievement_percentages_cache_dir() -> PathBuf {
    let dir = cache::cache_subdir("achievement_cache").join("global_percentages");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn global_achievement_percentages_cache_path(steam_app_id: u32) -> PathBuf {
    global_achievement_percentages_cache_dir().join(format!("{}.json", steam_app_id))
}

fn current_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn global_achievement_percentages_cache_is_fresh(fetched_at_unix_secs: u64) -> bool {
    current_unix_secs().saturating_sub(fetched_at_unix_secs)
        < GLOBAL_ACHIEVEMENT_PERCENTAGES_CACHE_TTL_SECS
}

fn load_cached_global_achievement_percentages(
    steam_app_id: u32,
) -> Option<CachedGlobalAchievementPercentages> {
    let bytes = std::fs::read(global_achievement_percentages_cache_path(steam_app_id)).ok()?;
    serde_json::from_slice::<CachedGlobalAchievementPercentages>(&bytes).ok()
}

fn store_cached_global_achievement_percentages(
    steam_app_id: u32,
    percentages: &HashMap<String, f32>,
) {
    let payload = CachedGlobalAchievementPercentages {
        fetched_at_unix_secs: current_unix_secs(),
        percentages: percentages.clone(),
    };

    if let Ok(bytes) = serde_json::to_vec(&payload) {
        let _ = std::fs::write(
            global_achievement_percentages_cache_path(steam_app_id),
            bytes,
        );
    }
}

fn global_achievement_percentage_refreshes() -> &'static Mutex<HashSet<u32>> {
    GLOBAL_ACHIEVEMENT_PERCENTAGES_REFRESHES.get_or_init(|| Mutex::new(HashSet::new()))
}

fn global_achievement_percentage_updates() -> &'static Mutex<Vec<u32>> {
    GLOBAL_ACHIEVEMENT_PERCENTAGES_UPDATED.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn take_updated_global_achievement_percentages() -> Vec<u32> {
    let Ok(mut updated) = global_achievement_percentage_updates().lock() else {
        return Vec::new();
    };
    updated.drain(..).collect()
}

fn log_achievement_request_failure(steam_app_id: u32, url: &str, detail: &str) {
    let log_path = achievement_diagnostics_log_path();
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = writeln!(
            file,
            "[big-screen-launcher] app {}: global achievement percentage request failed: url={}, {}",
            steam_app_id,
            url,
            detail
        );
    }
}

fn should_retry_achievement_request(err: &ureq::Error) -> bool {
    matches!(err, ureq::Error::Timeout(_) | ureq::Error::Io(_))
}

pub fn load_cached_achievement_summary(
    steam_app_id: u32,
    language: AppLanguage,
) -> Option<AchievementSummary> {
    let bytes = std::fs::read(achievement_cache_path(steam_app_id, language)).ok()?;
    let cached = serde_json::from_slice::<CachedAchievementSummary>(&bytes).ok()?;
    if cached.summary.items.is_empty() {
        return None;
    }
    Some(cached.summary)
}

pub fn load_cached_achievement_overview(
    steam_app_id: u32,
    language: AppLanguage,
) -> Option<AchievementSummary> {
    load_cached_achievement_summary(steam_app_id, language).map(|summary| AchievementSummary {
        unlocked: summary.unlocked,
        total: summary.total,
        items: Vec::new(),
    })
}

pub fn store_cached_achievement_summary(
    steam_app_id: u32,
    summary: &AchievementSummary,
    language: AppLanguage,
) {
    if summary.items.is_empty() {
        return;
    }

    let cache_path = achievement_cache_path(steam_app_id, language);
    let payload = CachedAchievementSummary {
        summary: summary.clone(),
    };
    if let Ok(bytes) = serde_json::to_vec(&payload) {
        let _ = std::fs::write(cache_path, bytes);
    }
}

fn fetch_global_achievement_percentages(steam_app_id: u32) -> Option<HashMap<String, f32>> {
    let request_timeout = Duration::from_secs(5);
    let max_attempts = 2;
    let url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2/?gameid={}",
        steam_app_id
    );
    let mut attempt = 1;
    let resp = loop {
        match ureq::get(&url)
            .config()
            .timeout_global(Some(request_timeout))
            .build()
            .call()
        {
            Ok(resp) => break resp,
            Err(ureq::Error::StatusCode(status)) => {
                log_achievement_request_failure(
                    steam_app_id,
                    &url,
                    &format!("status={}, attempt={}/{}", status, attempt, max_attempts),
                );
                return None;
            }
            Err(err) => {
                if should_retry_achievement_request(&err) && attempt < max_attempts {
                    attempt += 1;
                    continue;
                }

                log_achievement_request_failure(
                    steam_app_id,
                    &url,
                    &format!("error={}, attempt={}/{}", err, attempt, max_attempts),
                );
                return None;
            }
        }
    };

    let mut bytes = Vec::new();
    if resp
        .into_body()
        .into_reader()
        .take(256 * 1024)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return None;
    }

    let Ok(payload) = serde_json::from_slice::<GlobalAchievementPercentagesResponse>(&bytes) else {
        return None;
    };

    let percentages: HashMap<String, f32> = payload
        .achievementpercentages
        .achievements
        .into_iter()
        .filter(|entry| entry.percent.is_finite())
        .map(|entry| (entry.name, entry.percent))
        .collect();

    Some(percentages)
}

fn refresh_global_achievement_percentages_in_background(steam_app_id: u32) {
    let should_start = {
        let Ok(mut in_flight) = global_achievement_percentage_refreshes().lock() else {
            return;
        };
        in_flight.insert(steam_app_id)
    };

    if !should_start {
        return;
    }

    std::thread::spawn(move || {
        if let Some(percentages) = fetch_global_achievement_percentages(steam_app_id) {
            store_cached_global_achievement_percentages(steam_app_id, &percentages);
            if let Ok(mut updated) = global_achievement_percentage_updates().lock() {
                updated.push(steam_app_id);
            }
        }

        if let Ok(mut in_flight) = global_achievement_percentage_refreshes().lock() {
            in_flight.remove(&steam_app_id);
        }
    });
}

pub fn request_global_achievement_percentages_refresh(steam_app_id: u32) {
    if let Some(cached) = load_cached_global_achievement_percentages(steam_app_id) {
        if global_achievement_percentages_cache_is_fresh(cached.fetched_at_unix_secs) {
            return;
        }
    }

    refresh_global_achievement_percentages_in_background(steam_app_id);
}

pub(super) fn load_global_achievement_percentages(
    steam_app_id: u32,
    allow_network_refresh: bool,
) -> HashMap<String, f32> {
    if let Some(cached) = load_cached_global_achievement_percentages(steam_app_id) {
        if global_achievement_percentages_cache_is_fresh(cached.fetched_at_unix_secs) {
            return cached.percentages;
        }

        if !allow_network_refresh {
            return cached.percentages;
        }

        if !cached.percentages.is_empty() {
            refresh_global_achievement_percentages_in_background(steam_app_id);
            return cached.percentages;
        }
    }

    if !allow_network_refresh {
        return HashMap::new();
    }

    let Some(percentages) = fetch_global_achievement_percentages(steam_app_id) else {
        return HashMap::new();
    };

    store_cached_global_achievement_percentages(steam_app_id, &percentages);
    percentages
}

#[cfg(test)]
mod tests {
    use super::{
        global_achievement_percentages_cache_is_fresh, GlobalAchievementPercentagesResponse,
        GLOBAL_ACHIEVEMENT_PERCENTAGES_CACHE_TTL_SECS,
    };

    #[test]
    fn global_achievement_percentages_accept_string_percent_values() {
        let payload = r#"{
            "achievementpercentages": {
                "achievements": [
                    { "name": "ACH_ONE", "percent": "38.0" },
                    { "name": "ACH_TWO", "percent": 12.5 }
                ]
            }
        }"#;

        let parsed: GlobalAchievementPercentagesResponse =
            serde_json::from_str(payload).expect("payload should parse");

        assert_eq!(parsed.achievementpercentages.achievements.len(), 2);
        assert_eq!(parsed.achievementpercentages.achievements[0].percent, 38.0);
        assert_eq!(parsed.achievementpercentages.achievements[1].percent, 12.5);
    }

    #[test]
    fn global_achievement_percentages_trim_string_values() {
        let payload = r#"{
            "achievementpercentages": {
                "achievements": [
                    { "name": "ACH_ONE", "percent": "  7.5  " }
                ]
            }
        }"#;

        let parsed: GlobalAchievementPercentagesResponse =
            serde_json::from_str(payload).expect("payload should parse");

        assert_eq!(parsed.achievementpercentages.achievements[0].percent, 7.5);
    }

    #[test]
    fn global_achievement_cache_freshness_expires_at_ttl_boundary() {
        let now = super::current_unix_secs();

        assert!(global_achievement_percentages_cache_is_fresh(now));
        assert!(global_achievement_percentages_cache_is_fresh(
            now.saturating_sub(GLOBAL_ACHIEVEMENT_PERCENTAGES_CACHE_TTL_SECS - 1)
        ));
        assert!(!global_achievement_percentages_cache_is_fresh(
            now.saturating_sub(GLOBAL_ACHIEVEMENT_PERCENTAGES_CACHE_TTL_SECS)
        ));
    }
}
