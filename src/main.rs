#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod achievements;
mod artwork;
mod cover;
mod dlss;
mod game_icons;
mod i18n;
mod input;
mod launch;
mod runtime_state;
mod page_state;
mod settings;
mod steam;
mod ui;

use eframe::egui;

#[cfg(target_os = "windows")]
use std::path::PathBuf;

#[cfg(target_os = "windows")]
fn load_font_from_path(path: PathBuf) -> Option<egui::FontData> {
    std::fs::read(path).ok().map(egui::FontData::from_owned)
}

#[cfg(target_os = "windows")]
fn load_windows_font(file_name: &str) -> Option<egui::FontData> {
    let windows_dir = std::env::var_os("WINDIR").or_else(|| std::env::var_os("SystemRoot"))?;
    load_font_from_path(PathBuf::from(windows_dir).join("Fonts").join(file_name))
}

#[cfg(target_os = "windows")]
fn configure_fonts(ctx: &egui::Context) {
    let Some(regular_font) = load_windows_font("msyh.ttc") else {
        return;
    };
    let bold_font = load_windows_font("msyhbd.ttc").unwrap_or_else(|| regular_font.clone());

    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert("ui_regular".to_owned(), regular_font);
    fonts.font_data.insert("ui_bold".to_owned(), bold_font);
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "ui_regular".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "ui_regular".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Name("Bold".into()))
        .or_default()
        .insert(0, "ui_bold".to_owned());

    ctx.set_fonts(fonts);
}

#[cfg(not(target_os = "windows"))]
fn configure_fonts(_ctx: &egui::Context) {}

fn main() {
    let language = i18n::AppLanguage::detect_system();
    let options = eframe::NativeOptions {
        fullscreen: true,
        ..Default::default()
    };
    let _ = eframe::run_native(
        language.window_title(),
        options,
        Box::new(move |cc| {
            configure_fonts(&cc.egui_ctx);
            Box::new(app::LauncherApp::new(language))
        }),
    );
}
