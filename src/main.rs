#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod animation;
mod app;
mod assets;
mod config;
mod game;
mod game_platforms;
mod game_last_played;
mod power_menu_structure;
mod i18n;
mod input;
mod launch;
mod system;
mod steam;
mod ui;

use eframe::egui;
use std::sync::Arc;

#[cfg(target_os = "windows")]
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

fn load_app_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/app-icon-256.png"));
    let rgba = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = rgba.dimensions();

    Some(egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}

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
fn load_first_available_font(file_names: &[&str]) -> Option<egui::FontData> {
    file_names.iter().find_map(|file_name| load_windows_font(file_name))
}

#[cfg(target_os = "windows")]
fn configure_fonts(ctx: &egui::Context, language: i18n::AppLanguage) {
    let (regular_candidates, bold_candidates): (&[&str], &[&str]) = match language {
        i18n::AppLanguage::English => (
            &["segoeui.ttf"],
            &["segoeuib.ttf"],
        ),
        i18n::AppLanguage::SimplifiedChinese => (
            &["msyh.ttc"],
            &["msyhbd.ttc"],
        ),
    };

    let Some(regular_font) = load_first_available_font(regular_candidates)
        .or_else(|| load_first_available_font(&["segoeui.ttf"]))
    else {
        return;
    };
    let bold_font = load_first_available_font(bold_candidates)
        .or_else(|| load_first_available_font(&["segoeuib.ttf"]))
        .unwrap_or_else(|| regular_font.clone());

    let mut fonts = egui::FontDefinitions::default();

    fonts
        .font_data
        .insert("ui_regular".to_owned(), Arc::new(regular_font));
    fonts
        .font_data
        .insert("ui_bold".to_owned(), Arc::new(bold_font));
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
fn configure_fonts(_ctx: &egui::Context, _language: i18n::AppLanguage) {}

#[cfg(target_os = "windows")]
fn cache_root_window_handle(cc: &eframe::CreationContext<'_>) {
    let Ok(window_handle) = cc.window_handle() else {
        return;
    };

    let RawWindowHandle::Win32(handle) = window_handle.as_raw() else {
        return;
    };

    crate::launch::set_current_app_hwnd(handle.hwnd.get());
}

#[cfg(not(target_os = "windows"))]
fn cache_root_window_handle(_cc: &eframe::CreationContext<'_>) {}

fn main() {
    let language = i18n::AppLanguage::detect_system();
    let viewport = if let Some(icon) = load_app_icon() {
        egui::ViewportBuilder::default()
            .with_fullscreen(true)
            .with_icon(icon)
    } else {
        egui::ViewportBuilder::default().with_fullscreen(true)
    };
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    let _ = eframe::run_native(
        language.window_title(),
        options,
        Box::new(move |cc| {
            cache_root_window_handle(cc);
            configure_fonts(&cc.egui_ctx, language);
            Ok(Box::new(app::LauncherApp::new(language, &cc.egui_ctx)))
        }),
    );
}
