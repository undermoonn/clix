use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::game::GameScanOptions;
use crate::i18n::{AppLanguage, AppLanguageSetting};
use crate::system::display_mode::DisplayModeSetting;

const CONFIG_FILE_NAME: &str = "settings.ini";
const UI_SECTION: &str = "ui";
const GAMES_SECTION: &str = "games";
const DEBUG_SECTION: &str = "debug";
const HINT_ICON_THEME_KEY: &str = "hint_icon_theme";
const LANGUAGE_KEY: &str = "language";
const DISPLAY_MODE_KEY: &str = "display_mode";
const HOME_GAME_LIMIT_KEY: &str = "home_game_limit";
const IDLE_FRAME_RATE_REDUCTION_ENABLED_KEY: &str = "idle_frame_rate_reduction_enabled";
const BACKGROUND_HOME_WAKE_MODE_KEY: &str = "background_home_wake_enabled";
const CONTROLLER_VIBRATION_ENABLED_KEY: &str = "controller_vibration_enabled";
const DETECT_STEAM_GAMES_KEY: &str = "detect_steam_games";
const DETECT_EPIC_GAMES_KEY: &str = "detect_epic_games";
const DETECT_XBOX_GAMES_KEY: &str = "detect_xbox_games";
const STEAM_CLIENT_STATE_LOGGING_ENABLED_KEY: &str = "steam_client_state_logging_enabled";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum BackgroundHomeWakeMode {
    Off,
    #[default]
    ShortPress,
    LongPress,
}

impl BackgroundHomeWakeMode {
    const OFF_CONFIG_VALUE: &str = "off";
    const SHORT_PRESS_CONFIG_VALUE: &str = "short_press";
    const LONG_PRESS_CONFIG_VALUE: &str = "long_press";

    pub fn as_config_value(self) -> &'static str {
        match self {
            Self::Off => Self::OFF_CONFIG_VALUE,
            Self::ShortPress => Self::SHORT_PRESS_CONFIG_VALUE,
            Self::LongPress => Self::LONG_PRESS_CONFIG_VALUE,
        }
    }

    pub fn from_config_value(value: &str) -> Self {
        if value.eq_ignore_ascii_case(Self::SHORT_PRESS_CONFIG_VALUE)
            || value.eq_ignore_ascii_case("true")
        {
            Self::ShortPress
        } else if value.eq_ignore_ascii_case(Self::LONG_PRESS_CONFIG_VALUE) {
            Self::LongPress
        } else {
            Self::Off
        }
    }

    pub fn from_atomic_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Off),
            1 => Some(Self::ShortPress),
            2 => Some(Self::LongPress),
            _ => None,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::ShortPress,
            Self::ShortPress => Self::LongPress,
            Self::LongPress => Self::Off,
        }
    }

    pub fn display_text(self, language: AppLanguage) -> &'static str {
        match self {
            Self::Off => language.disabled_text(),
            Self::ShortPress => language.short_press_text(),
            Self::LongPress => language.long_press_text(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PromptIconTheme {
    #[default]
    Xbox,
    PlayStation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HomeGameLimit {
    Limited(usize),
    Unlimited,
}

impl Default for HomeGameLimit {
    fn default() -> Self {
        Self::Limited(9)
    }
}

impl HomeGameLimit {
    pub const MIN_LIMIT: usize = 5;
    pub const MAX_LIMIT: usize = 20;
    pub const OPTION_COUNT: usize = Self::MAX_LIMIT - Self::MIN_LIMIT + 2;

    pub fn from_option_index(index: usize) -> Self {
        if index < Self::MAX_LIMIT - Self::MIN_LIMIT + 1 {
            Self::Limited(Self::MIN_LIMIT + index)
        } else {
            Self::Unlimited
        }
    }

    pub fn option_index(self) -> usize {
        match self {
            Self::Limited(limit) => limit.clamp(Self::MIN_LIMIT, Self::MAX_LIMIT) - Self::MIN_LIMIT,
            Self::Unlimited => Self::MAX_LIMIT - Self::MIN_LIMIT + 1,
        }
    }

    pub fn as_config_value(self) -> String {
        match self {
            Self::Limited(limit) => limit.clamp(Self::MIN_LIMIT, Self::MAX_LIMIT).to_string(),
            Self::Unlimited => "unlimited".to_string(),
        }
    }

    pub fn from_config_value(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("unlimited")
            || value.eq_ignore_ascii_case("none")
            || value.eq_ignore_ascii_case("no_limit")
            || value == "0"
        {
            return Some(Self::Unlimited);
        }

        let limit = value.parse::<usize>().ok()?;
        (Self::MIN_LIMIT..=Self::MAX_LIMIT)
            .contains(&limit)
            .then_some(Self::Limited(limit))
    }

    pub fn display_text(self, language: AppLanguage) -> String {
        match self {
            Self::Limited(limit) => limit.clamp(Self::MIN_LIMIT, Self::MAX_LIMIT).to_string(),
            Self::Unlimited => language.unlimited_text().to_string(),
        }
    }
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
    home_game_limit: HomeGameLimit,
    idle_frame_rate_reduction_enabled: bool,
    background_home_wake_mode: BackgroundHomeWakeMode,
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
            home_game_limit: HomeGameLimit::default(),
            idle_frame_rate_reduction_enabled: true,
            background_home_wake_mode: BackgroundHomeWakeMode::ShortPress,
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

        if let Some(section_name) = line
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
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
                config.hint_icon_theme =
                    PromptIconTheme::from_config_value(value).unwrap_or(PromptIconTheme::Xbox);
            } else if key.eq_ignore_ascii_case(LANGUAGE_KEY) {
                config.language = AppLanguageSetting::from_config_value(value)
                    .unwrap_or(AppLanguageSetting::Auto);
            } else if key.eq_ignore_ascii_case(DISPLAY_MODE_KEY) {
                config.display_mode_setting = DisplayModeSetting::from_config_value(value)
                    .unwrap_or(DisplayModeSetting::Fullscreen);
            } else if key.eq_ignore_ascii_case(HOME_GAME_LIMIT_KEY) {
                config.home_game_limit =
                    HomeGameLimit::from_config_value(value).unwrap_or_default();
            } else if key.eq_ignore_ascii_case(IDLE_FRAME_RATE_REDUCTION_ENABLED_KEY) {
                config.idle_frame_rate_reduction_enabled =
                    parse_bool_config_value(value).unwrap_or(true);
            } else if key.eq_ignore_ascii_case(BACKGROUND_HOME_WAKE_MODE_KEY) {
                config.background_home_wake_mode = BackgroundHomeWakeMode::from_config_value(value);
            } else if key.eq_ignore_ascii_case(CONTROLLER_VIBRATION_ENABLED_KEY) {
                config.controller_vibration_enabled =
                    parse_bool_config_value(value).unwrap_or(false);
            }
        } else if current_section.eq_ignore_ascii_case(GAMES_SECTION) {
            if key.eq_ignore_ascii_case(DETECT_STEAM_GAMES_KEY) {
                config.game_scan_options.detect_steam_games = parse_bool_config_value(value)
                    .unwrap_or(GameScanOptions::default().detect_steam_games);
            } else if key.eq_ignore_ascii_case(DETECT_EPIC_GAMES_KEY) {
                config.game_scan_options.detect_epic_games = parse_bool_config_value(value)
                    .unwrap_or(GameScanOptions::default().detect_epic_games);
            } else if key.eq_ignore_ascii_case(DETECT_XBOX_GAMES_KEY) {
                config.game_scan_options.detect_xbox_games = parse_bool_config_value(value)
                    .unwrap_or(GameScanOptions::default().detect_xbox_games);
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
        "[{}]\n{}={}\n{}={}\n{}={}\n{}={}\n{}={}\n{}={}\n{}={}\n\n[{}]\n{}={}\n{}={}\n{}={}\n\n[{}]\n{}={}\n",
        UI_SECTION,
        HINT_ICON_THEME_KEY,
        config.hint_icon_theme.as_config_value(),
        LANGUAGE_KEY,
        config.language.as_config_value(),
        DISPLAY_MODE_KEY,
        config.display_mode_setting.as_config_value(),
        HOME_GAME_LIMIT_KEY,
        config.home_game_limit.as_config_value(),
        IDLE_FRAME_RATE_REDUCTION_ENABLED_KEY,
        config.idle_frame_rate_reduction_enabled,
        BACKGROUND_HOME_WAKE_MODE_KEY,
        config.background_home_wake_mode.as_config_value(),
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

pub fn load_home_game_limit() -> HomeGameLimit {
    load_config().home_game_limit
}

pub fn load_idle_frame_rate_reduction_enabled() -> bool {
    load_config().idle_frame_rate_reduction_enabled
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

pub fn store_home_game_limit(home_game_limit: HomeGameLimit) {
    let mut config = load_config();
    config.home_game_limit = home_game_limit;
    store_config(config);
}

pub fn store_idle_frame_rate_reduction_enabled(enabled: bool) {
    let mut config = load_config();
    config.idle_frame_rate_reduction_enabled = enabled;
    store_config(config);
}

pub fn load_controller_vibration_enabled() -> bool {
    load_config().controller_vibration_enabled
}

pub fn load_background_home_wake_mode() -> BackgroundHomeWakeMode {
    load_config().background_home_wake_mode
}

pub fn store_background_home_wake_mode(mode: BackgroundHomeWakeMode) {
    let mut config = load_config();
    config.background_home_wake_mode = mode;
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
    use super::{parse_config, serialize_config, AppConfig, BackgroundHomeWakeMode};
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

    #[test]
    fn parse_config_defaults_idle_frame_rate_reduction_to_enabled() {
        let config = parse_config("[ui]\ndisplay_mode=fullscreen\n");

        assert!(config.idle_frame_rate_reduction_enabled);
    }

    #[test]
    fn serialize_config_writes_idle_frame_rate_reduction_setting() {
        let mut config = AppConfig::default();
        config.idle_frame_rate_reduction_enabled = false;

        let contents = serialize_config(config);

        assert!(contents.contains("idle_frame_rate_reduction_enabled=false"));
    }

    #[test]
    fn parse_config_maps_legacy_true_home_wake_value_to_short_press() {
        let config = parse_config("[ui]\nbackground_home_wake_enabled=true\n");

        assert_eq!(
            config.background_home_wake_mode,
            BackgroundHomeWakeMode::ShortPress
        );
    }

    #[test]
    fn serialize_config_writes_home_wake_mode_value() {
        let mut config = AppConfig::default();
        config.background_home_wake_mode = BackgroundHomeWakeMode::LongPress;

        let contents = serialize_config(config);

        assert!(contents.contains("background_home_wake_enabled=long_press"));
    }
}

pub fn load_steam_client_state_logging_enabled() -> bool {
    load_config().steam_client_state_logging_enabled
}
