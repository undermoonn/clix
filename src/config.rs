use std::path::PathBuf;

const CONFIG_FILE_NAME: &str = "settings.ini";
const UI_SECTION: &str = "ui";
const HINT_ICON_THEME_KEY: &str = "hint_icon_theme";
const CONTROLLER_VIBRATION_ENABLED_KEY: &str = "controller_vibration_enabled";

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
    controller_vibration_enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hint_icon_theme: PromptIconTheme::Xbox,
            controller_vibration_enabled: false,
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
    let dir = crate::assets::cache::app_root_dir().join("config");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn config_path() -> PathBuf {
    config_dir().join(CONFIG_FILE_NAME)
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

        if current_section.eq_ignore_ascii_case(UI_SECTION) {
            let key = key.trim();
            let value = value.trim();

            if key.eq_ignore_ascii_case(HINT_ICON_THEME_KEY) {
                config.hint_icon_theme = PromptIconTheme::from_config_value(value)
                    .unwrap_or(PromptIconTheme::Xbox);
            } else if key.eq_ignore_ascii_case(CONTROLLER_VIBRATION_ENABLED_KEY) {
                config.controller_vibration_enabled =
                    parse_bool_config_value(value).unwrap_or(false);
            }
        }
    }

    config
}

fn serialize_config(config: AppConfig) -> String {
    format!(
        "[{}]\n{}={}\n{}={}\n",
        UI_SECTION,
        HINT_ICON_THEME_KEY,
        config.hint_icon_theme.as_config_value(),
        CONTROLLER_VIBRATION_ENABLED_KEY,
        config.controller_vibration_enabled
    )
}

fn load_config() -> AppConfig {
    let Ok(contents) = std::fs::read_to_string(config_path()) else {
        return AppConfig::default();
    };

    parse_config(&contents)
}

fn store_config(config: AppConfig) {
    let _ = std::fs::write(config_path(), serialize_config(config));
}

pub fn load_hint_icon_theme() -> PromptIconTheme {
    load_config().hint_icon_theme
}

pub fn store_hint_icon_theme(theme: PromptIconTheme) {
    let mut config = load_config();
    config.hint_icon_theme = theme;
    store_config(config);
}

pub fn load_controller_vibration_enabled() -> bool {
    load_config().controller_vibration_enabled
}

pub fn store_controller_vibration_enabled(enabled: bool) {
    let mut config = load_config();
    config.controller_vibration_enabled = enabled;
    store_config(config);
}