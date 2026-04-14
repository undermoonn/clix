mod app;
mod cover;
mod input;
mod steam;
mod ui;

use eframe::egui;

fn main() {
    let options = eframe::NativeOptions {
        fullscreen: true,
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Clix Launcher Prototype",
        options,
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();
            let font_path = std::path::Path::new("C:\\Windows\\Fonts\\msyh.ttc");
            if font_path.exists() {
                if let Ok(font_data) = std::fs::read(font_path) {
                    fonts.font_data.insert(
                        "msyh".to_owned(),
                        egui::FontData::from_owned(font_data),
                    );
                    fonts
                        .families
                        .entry(egui::FontFamily::Proportional)
                        .or_default()
                        .insert(0, "msyh".to_owned());
                    fonts
                        .families
                        .entry(egui::FontFamily::Monospace)
                        .or_default()
                        .push("msyh".to_owned());
                }
            }
            cc.egui_ctx.set_fonts(fonts);
            Box::new(app::LauncherApp::new())
        }),
    );
}
