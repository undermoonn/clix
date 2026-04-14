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
            fonts.font_data.insert(
                "noto_sans_sc".to_owned(),
                egui::FontData::from_static(include_bytes!("fonts/NotoSansSC-Regular.otf")),
            );
            fonts.font_data.insert(
                "noto_sans_sc_bold".to_owned(),
                egui::FontData::from_static(include_bytes!("fonts/NotoSansSC-Bold.otf")),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "noto_sans_sc".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("noto_sans_sc".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Name("Bold".into()))
                .or_default()
                .push("noto_sans_sc_bold".to_owned());
            cc.egui_ctx.set_fonts(fonts);
            Box::new(app::LauncherApp::new())
        }),
    );
}
