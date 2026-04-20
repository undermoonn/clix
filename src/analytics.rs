use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Utc;
use serde_json::{json, Map, Value};

use crate::assets::cache;

const ANALYTICS_CLIENT_ID_FILE: &str = "client_id.txt";
const ANALYTICS_REQUEST_LOG_FILE: &str = "requests.jsonl";
const GA4_ENDPOINT: &str = "https://www.google-analytics.com/mp/collect";

#[derive(Clone)]
pub struct Analytics {
    sender: Option<Sender<Value>>,
    client_id: Option<String>,
    session_id: u64,
    language: &'static str,
}

struct AnalyticsConfig {
    measurement_id: String,
    api_secret: String,
}

impl Analytics {
    pub fn new(language: &'static str) -> Self {
        let session_id = unix_timestamp_secs();
        let Some(config) = load_config() else {
            return Self {
                sender: None,
                client_id: None,
                session_id,
                language,
            };
        };

        let Some(sender) = spawn_worker(config) else {
            return Self {
                sender: None,
                client_id: None,
                session_id,
                language,
            };
        };

        Self {
            sender: Some(sender),
            client_id: load_or_create_client_id(),
            session_id,
            language,
        }
    }

    pub fn track_app_open(
        &self,
        game_count: usize,
        external_app_count: usize,
        launch_on_startup_enabled: bool,
    ) {
        self.track(
            "app_open",
            [
                ("game_count", json!(game_count as u64)),
                ("external_app_count", json!(external_app_count as u64)),
                (
                    "launch_on_startup_enabled",
                    json!(launch_on_startup_enabled),
                ),
            ],
        );
    }

    pub fn track_game_launch_request(&self, game_name: &str, app_id: Option<u32>, source: &str) {
        self.track_game_event(
            "game_launch_request",
            game_name,
            app_id,
            [("launch_source", json!(source))],
        );
    }

    pub fn track_game_launch_result(
        &self,
        game_name: &str,
        app_id: Option<u32>,
        result: &str,
        source: &str,
        elapsed_ms: u64,
    ) {
        self.track_game_event(
            "game_launch_result",
            game_name,
            app_id,
            [
                ("launch_result", json!(result)),
                ("launch_source", json!(source)),
                ("elapsed_ms", json!(elapsed_ms)),
            ],
        );
    }

    pub fn track_external_app_launch(&self, app_kind: &str, launched: bool) {
        self.track(
            "external_app_launch",
            [
                ("external_app", json!(app_kind)),
                ("launch_success", json!(launched)),
            ],
        );
    }

    pub fn track_achievement_panel_open(&self, game_name: &str, app_id: Option<u32>) {
        self.track_game_event("achievement_panel_open", game_name, app_id, []);
    }

    pub fn track_launch_on_startup_toggle(&self, enabled: bool) {
        self.track("launch_on_startup_toggle", [("enabled", json!(enabled))]);
    }

    pub fn track_display_resolution_change(&self, preset: &str) {
        self.track("display_resolution_change", [("preset", json!(preset))]);
    }

    fn track_game_event<const N: usize>(
        &self,
        event_name: &str,
        game_name: &str,
        app_id: Option<u32>,
        extra_params: [(&'static str, Value); N],
    ) {
        let mut params = Vec::with_capacity(extra_params.len() + 2);
        params.push(("game_name", json!(game_name)));
        if let Some(app_id) = app_id {
            params.push(("steam_app_id", json!(app_id)));
        }
        params.extend(extra_params);
        self.track(event_name, params);
    }

    fn track<I>(&self, event_name: &str, params: I)
    where
        I: IntoIterator<Item = (&'static str, Value)>,
    {
        let Some(sender) = &self.sender else {
            return;
        };
        let Some(client_id) = &self.client_id else {
            return;
        };

        let mut event_params = self.base_params();
        for (key, value) in params {
            event_params.insert(key.to_owned(), value);
        }

        let payload = json!({
            "client_id": client_id,
            "events": [
                {
                    "name": event_name,
                    "params": event_params,
                }
            ]
        });

        let _ = sender.send(payload);
    }

    fn base_params(&self) -> Map<String, Value> {
        let mut params = Map::new();
        params.insert("session_id".to_owned(), json!(self.session_id));
        params.insert("engagement_time_msec".to_owned(), json!(1_u64));
        params.insert("app_name".to_owned(), json!(env!("CARGO_PKG_NAME")));
        params.insert("app_version".to_owned(), json!(env!("CARGO_PKG_VERSION")));
        params.insert("app_language".to_owned(), json!(self.language));
        params.insert("platform".to_owned(), json!(std::env::consts::OS));
        params
    }
}

fn load_config() -> Option<AnalyticsConfig> {
    let enabled = option_env!("GA4_ENABLED")
        .and_then(parse_bool)
        .unwrap_or(true);
    if !enabled {
        return None;
    }

    let measurement_id = option_env!("GA4_MEASUREMENT_ID")?.to_owned();
    let api_secret = option_env!("GA4_API_SECRET")?.to_owned();

    Some(AnalyticsConfig {
        measurement_id,
        api_secret,
    })
}

fn spawn_worker(config: AnalyticsConfig) -> Option<Sender<Value>> {
    let (sender, receiver) = mpsc::channel::<Value>();
    let endpoint = format!(
        "{}?measurement_id={}&api_secret={}",
        GA4_ENDPOINT, config.measurement_id, config.api_secret
    );
    let proxy = resolve_proxy();
    let agent = Arc::new(build_agent(proxy.as_ref()));
    let proxy_uri = proxy.as_ref().map(|proxy| proxy.uri().to_string());

    thread::Builder::new()
        .name("analytics-worker".to_owned())
        .spawn(move || {
            while let Ok(payload) = receiver.recv() {
                let payload_text = payload.to_string();
                let result = agent
                    .post(&endpoint)
                    .content_type("application/json")
                    .send(&payload_text);
                log_request(
                    &payload,
                    result.as_ref().err().map(|err| err.to_string()),
                    proxy_uri.as_deref(),
                );
            }
        })
        .ok()?;

    Some(sender)
}

fn load_or_create_client_id() -> Option<String> {
    let dir = cache::cache_subdir("analytics");
    let path = dir.join(ANALYTICS_CLIENT_ID_FILE);

    if let Ok(existing) = fs::read_to_string(&path) {
        let trimmed = existing.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_owned());
        }
    }

    let generated = generate_client_id();
    if fs::write(&path, generated.as_bytes()).is_ok() {
        Some(generated)
    } else {
        None
    }
}

fn generate_client_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!(
        "{}.{}.{}",
        now.as_secs(),
        now.subsec_nanos(),
        std::process::id()
    )
}

fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn analytics_log_path() -> std::path::PathBuf {
    cache::cache_subdir("analytics").join(ANALYTICS_REQUEST_LOG_FILE)
}

fn log_request(payload: &Value, error: Option<String>, proxy: Option<&str>) {
    let log_entry = json!({
        "sent_at": Utc::now().to_rfc3339(),
        "proxy": proxy,
        "payload": payload,
        "result": if let Some(error) = error {
            json!({ "status": "error", "message": error })
        } else {
            json!({ "status": "ok" })
        }
    });

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(analytics_log_path())
    {
        let _ = writeln!(file, "{}", log_entry);
    }
}

fn build_agent(proxy: Option<&ureq::Proxy>) -> ureq::Agent {
    if let Some(proxy) = proxy {
        let config = ureq::config::Config::builder()
            .proxy(Some(proxy.clone()))
            .build();
        ureq::Agent::new_with_config(config)
    } else {
        ureq::Agent::new_with_defaults()
    }
}

fn resolve_proxy() -> Option<ureq::Proxy> {
    if let Some(proxy) = ureq::Proxy::try_from_env() {
        return Some(proxy);
    }

    #[cfg(target_os = "windows")]
    {
        resolve_windows_proxy()
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

#[cfg(target_os = "windows")]
fn resolve_windows_proxy() -> Option<ureq::Proxy> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

    let registry = RegKey::predef(HKEY_CURRENT_USER);
    let ie_settings = registry
        .open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            KEY_READ,
        )
        .ok()?;

    let enabled = ie_settings
        .get_value::<u32, _>("ProxyEnable")
        .ok()
        .is_some_and(|enable| enable == 1);
    if !enabled {
        return None;
    }

    let proxy_server = ie_settings.get_value::<String, _>("ProxyServer").ok()?;
    parse_windows_proxy_server(&proxy_server)
}

#[cfg(target_os = "windows")]
fn parse_windows_proxy_server(value: &str) -> Option<ureq::Proxy> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(proxy) = parse_named_proxy(trimmed, "https") {
        return ureq::Proxy::new(&proxy).ok();
    }
    if let Some(proxy) = parse_named_proxy(trimmed, "http") {
        return ureq::Proxy::new(&proxy).ok();
    }

    let direct = if has_proxy_scheme(trimmed) {
        trimmed.to_owned()
    } else {
        format!("http://{trimmed}")
    };
    ureq::Proxy::new(&direct).ok()
}

#[cfg(target_os = "windows")]
fn parse_named_proxy(value: &str, scheme: &str) -> Option<String> {
    for entry in value.split(';') {
        let (name, target) = entry.split_once('=')?;
        if !name.trim().eq_ignore_ascii_case(scheme) {
            continue;
        }

        let target = target.trim();
        if target.is_empty() {
            return None;
        }

        return Some(if has_proxy_scheme(target) {
            target.to_owned()
        } else {
            format!("http://{target}")
        });
    }

    None
}

fn has_proxy_scheme(value: &str) -> bool {
    value.starts_with("http://")
        || value.starts_with("https://")
        || value.starts_with("socks4://")
        || value.starts_with("socks4a://")
        || value.starts_with("socks5://")
        || value.starts_with("socks://")
}