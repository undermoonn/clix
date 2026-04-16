use std::path::{Path, PathBuf};

use crate::input::ControllerBrand;

const SETTINGS_FILE_NAME: &str = "clix.ini";

pub struct AppSettings {
    pub controller_brand: ControllerBrand,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            controller_brand: ControllerBrand::Xbox,
        }
    }
}

pub fn load_settings() -> AppSettings {
    let Ok(contents) = std::fs::read_to_string(settings_path()) else {
        return AppSettings::default();
    };

    parse_settings(&contents)
}

pub fn save_controller_brand(controller_brand: ControllerBrand) {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let contents = format!(
        "[theme]\ncontroller_brand={}\n",
        controller_brand.as_ini_value()
    );
    let _ = std::fs::write(path, contents);
}

fn settings_path() -> PathBuf {
    let mut dir = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    dir.push(SETTINGS_FILE_NAME);
    dir
}

fn parse_settings(contents: &str) -> AppSettings {
    let mut settings = AppSettings::default();
    let mut in_theme_section = false;

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            in_theme_section = line[1..line.len() - 1].trim().eq_ignore_ascii_case("theme");
            continue;
        }

        if !in_theme_section {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        if key.trim().eq_ignore_ascii_case("controller_brand") {
            if let Some(controller_brand) = ControllerBrand::from_ini_value(value.trim()) {
                settings.controller_brand = controller_brand;
            }
        }
    }

    settings
}