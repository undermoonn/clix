use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::game::GameScanOptions;
use crate::i18n::AppLanguageSetting;
use crate::system::display_mode::DisplayModeSetting;

const CONFIG_FILE_NAME: &str = "settings.ini";
const UI_SECTION: &str = "ui";
const GAMES_SECTION: &str = "games";
const DEBUG_SECTION: &str = "debug";
const HINT_ICON_THEME_KEY: &str = "hint_icon_theme";
const LANGUAGE_KEY: &str = "language";
const DISPLAY_MODE_KEY: &str = "display_mode";
const BACKGROUND_HOME_WAKE_ENABLED_KEY: &str = "background_home_wake_enabled";
const CONTROLLER_VIBRATION_ENABLED_KEY: &str = "controller_vibration_enabled";
const DETECT_STEAM_GAMES_KEY: &str = "detect_steam_games";
const DETECT_EPIC_GAMES_KEY: &str = "detect_epic_games";
const DETECT_XBOX_GAMES_KEY: &str = "detect_xbox_games";
const STEAM_CLIENT_STATE_LOGGING_ENABLED_KEY: &str = "steam_client_state_logging_enabled";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PromptIconTheme {
    #[default]
    Xbox,
    PlayStation,
}

impl PromptIconTheme {
    pub fn as_config_value(self) -> &'static str {
        match self {
            Self::Xbox => "xbox",
            Self::PlayStation => "playstation",
        }
    }

    pub fn from_config_value(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("xbox") {
            Some(Self::Xbox)
        } else if value.eq_ignore_ascii_case("playstation") {
            Some(Self::PlayStation)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
struct AppConfig {
    hint_icon_theme: PromptIconTheme,
    language: AppLanguageSetting,
    display_mode_setting: DisplayModeSetting,
    background_home_wake_enabled: bool,
    controller_vibration_enabled: bool,
    game_scan_options: GameScanOptions,
    steam_client_state_logging_enabled: bool,
}

static CONFIG_CACHE: OnceLock<Mutex<AppConfig>> = OnceLock::new();

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hint_icon_theme: PromptIconTheme::Xbox,
            language: AppLanguageSetting::Auto,
            display_mode_setting: DisplayModeSetting::Fullscreen,
            background_home_wake_enabled: true,
            controller_vibration_enabled: false,
            game_scan_options: GameScanOptions::default(),
            steam_client_state_logging_enabled: false,
        }
    }
}

fn parse_bool_config_value(value: &str) -> Option<bool> {
    if value.eq_ignore_ascii_case("true")
        || value.eq_ignore_ascii_case("yes")
        || value.eq_ignore_ascii_case("on")
        || value == "1"
    {
        Some(true)
    } else if value.eq_ignore_ascii_case("false")
        || value.eq_ignore_ascii_case("no")
        || value.eq_ignore_ascii_case("off")
        || value == "0"
    {
        Some(false)
    } else {
        None
    }
}

fn config_dir() -> PathBuf {
    crate::assets::cache::config_dir()
}

fn config_path() -> PathBuf {
    config_dir().join(CONFIG_FILE_NAME)
}

fn config_cache() -> &'static Mutex<AppConfig> {
    CONFIG_CACHE.get_or_init(|| Mutex::new(load_config_from_disk()))
}

fn lock_config_cache() -> std::sync::MutexGuard<'static, AppConfig> {
    config_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn parse_config(contents: &str) -> AppConfig {
    let mut config = AppConfig::default();
    let mut current_section = "";

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if let Some(section_name) = line.strip_prefix('[').and_then(|value| value.strip_suffix(']')) {
            current_section = section_name.trim();
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        let value = value.trim();

        if current_section.eq_ignore_ascii_case(UI_SECTION) {
            if key.eq_ignore_ascii_case(HINT_ICON_THEME_KEY) {
                config.hint_icon_theme = PromptIconTheme::from_config_value(value)
                    .unwrap_or(PromptIconTheme::Xbox);
            } else if key.eq_ignore_ascii_case(LANGUAGE_KEY) {
                config.language =
                    AppLanguageSetting::from_config_value(value).unwrap_or(AppLanguageSetting::Auto);
            } else if key.eq_ignore_ascii_case(DISPLAY_MODE_KEY) {
                config.display_mode_setting = DisplayModeSetting::from_config_value(value)
                    .unwrap_or(DisplayModeSetting::Fullscreen);
            } else if key.eq_ignore_ascii_case(BACKGROUND_HOME_WAKE_ENABLED_KEY) {
                config.background_home_wake_enabled =
                    parse_bool_config_value(value).unwrap_or(true);
            } else if key.eq_ignore_ascii_case(CONTROLLER_VIBRATION_ENABLED_KEY) {
                config.controller_vibration_enabled =
                    parse_bool_config_value(value).unwrap_or(false);
            }
        } else if current_section.eq_ignore_ascii_case(GAMES_SECTION) {
            if key.eq_ignore_ascii_case(DETECT_STEAM_GAMES_KEY) {
                config.game_scan_options.detect_steam_games =
                    parse_bool_config_value(value).unwrap_or(GameScanOptions::default().detect_steam_games);
            } else if key.eq_ignore_ascii_case(DETECT_EPIC_GAMES_KEY) {
                config.game_scan_options.detect_epic_games =
                    parse_bool_config_value(value).unwrap_or(GameScanOptions::default().detect_epic_games);
            } else if key.eq_ignore_ascii_case(DETECT_XBOX_GAMES_KEY) {
                config.game_scan_options.detect_xbox_games =
                    parse_bool_config_value(value).unwrap_or(GameScanOptions::default().detect_xbox_games);
            }
        } else if current_section.eq_ignore_ascii_case(DEBUG_SECTION) {
            if key.eq_ignore_ascii_case(STEAM_CLIENT_STATE_LOGGING_ENABLED_KEY) {
                config.steam_client_state_logging_enabled =
                    parse_bool_config_value(value).unwrap_or(false);
            }
        }
    }

    config
}

fn serialize_config(config: AppConfig) -> String {
    format!(
        "[{}]\n{}={}\n{}={}\n{}={}\n{}={}\n{}={}\n\n[{}]\n{}={}\n{}={}\n{}={}\n\n[{}]\n{}={}\n",
        UI_SECTION,
        HINT_ICON_THEME_KEY,
        config.hint_icon_theme.as_config_value(),
        LANGUAGE_KEY,
        config.language.as_config_value(),
        DISPLAY_MODE_KEY,
        config.display_mode_setting.as_config_value(),
        BACKGROUND_HOME_WAKE_ENABLED_KEY,
        config.background_home_wake_enabled,
        CONTROLLER_VIBRATION_ENABLED_KEY,
        config.controller_vibration_enabled,
        GAMES_SECTION,
        DETECT_STEAM_GAMES_KEY,
        config.game_scan_options.detect_steam_games,
        DETECT_EPIC_GAMES_KEY,
        config.game_scan_options.detect_epic_games,
        DETECT_XBOX_GAMES_KEY,
        config.game_scan_options.detect_xbox_games,
        DEBUG_SECTION,
        STEAM_CLIENT_STATE_LOGGING_ENABLED_KEY,
        config.steam_client_state_logging_enabled,
    )
}

fn load_config_from_disk() -> AppConfig {
    let Ok(contents) = std::fs::read_to_string(config_path()) else {
        return AppConfig::default();
    };

    parse_config(&contents)
}

fn load_config() -> AppConfig {
    *lock_config_cache()
}

fn store_config(config: AppConfig) {
    *lock_config_cache() = config;
    let _ = std::fs::write(config_path(), serialize_config(config));
}

pub fn initialize() {
    let _ = config_cache();
}

pub fn load_hint_icon_theme() -> PromptIconTheme {
    load_config().hint_icon_theme
}

pub fn load_app_language_setting() -> AppLanguageSetting {
    load_config().language
}

pub fn load_display_mode_setting() -> DisplayModeSetting {
    load_config().display_mode_setting
}

pub fn store_hint_icon_theme(theme: PromptIconTheme) {
    let mut config = load_config();
    config.hint_icon_theme = theme;
    store_config(config);
}

pub fn store_app_language_setting(language: AppLanguageSetting) {
    let mut config = load_config();
    config.language = language;
    store_config(config);
}

pub fn store_display_mode_setting(display_mode_setting: DisplayModeSetting) {
    let mut config = load_config();
    config.display_mode_setting = display_mode_setting;
    store_config(config);
}

pub fn load_controller_vibration_enabled() -> bool {
    load_config().controller_vibration_enabled
}

pub fn load_background_home_wake_enabled() -> bool {
    load_config().background_home_wake_enabled
}

pub fn store_background_home_wake_enabled(enabled: bool) {
    let mut config = load_config();
    config.background_home_wake_enabled = enabled;
    store_config(config);
}

pub fn store_controller_vibration_enabled(enabled: bool) {
    let mut config = load_config();
    config.controller_vibration_enabled = enabled;
    store_config(config);
}

pub fn load_game_scan_options() -> GameScanOptions {
    load_config().game_scan_options
}

pub fn store_game_scan_options(options: GameScanOptions) {
    let mut config = load_config();
    config.game_scan_options = options;
    store_config(config);
}

#[cfg(test)]
mod tests {
    use super::{parse_config, serialize_config, AppConfig};
    use crate::system::display_mode::DisplayModeSetting;

    #[test]
    fn parse_config_reads_display_mode_setting() {
        let config = parse_config("[ui]\ndisplay_mode=windowed\n");

        assert_eq!(config.display_mode_setting, DisplayModeSetting::Windowed);
    }

    #[test]
    fn serialize_config_writes_display_mode_setting() {
        let mut config = AppConfig::default();
        config.display_mode_setting = DisplayModeSetting::Windowed;

        let contents = serialize_config(config);

        assert!(contents.contains("display_mode=windowed"));
    }
}

pub fn load_steam_client_state_logging_enabled() -> bool {
    load_config().steam_client_state_logging_enabled
}