use std::sync::Arc;

use eframe::egui;

pub(crate) fn corner_radius(value: f32) -> egui::CornerRadius {
    egui::CornerRadius::same(value.round().clamp(0.0, 255.0) as u8)
}

pub(crate) fn format_achievement_status(
    unlocked: Option<bool>,
    unlock_time: Option<u64>,
) -> Option<String> {
    match unlocked {
        Some(true) => unlock_time
            .and_then(|value| i64::try_from(value).ok())
            .and_then(|timestamp| chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0))
            .map(|datetime| {
                datetime
                    .with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
            }),
        _ => None,
    }
}

pub(crate) fn build_wrapped_galley(
    ui: &egui::Ui,
    text: String,
    font: egui::FontId,
    color: egui::Color32,
    max_width: f32,
) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = max_width;
    job.append(
        &text,
        0.0,
        egui::TextFormat {
            font_id: font,
            color,
            ..Default::default()
        },
    );
    ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
}

pub(crate) fn scale_alpha(alpha: u8, scale: f32) -> u8 {
    ((alpha as f32) * scale.clamp(0.0, 1.0))
        .round()
        .clamp(0.0, 255.0) as u8
}

pub(crate) fn color_with_scaled_alpha(color: egui::Color32, scale: f32) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        scale_alpha(color.a(), scale),
    )
}

pub(crate) fn main_clock_font() -> egui::FontId {
    egui::FontId::new(40.0, egui::FontFamily::Name("Bold".into()))
}

pub(crate) fn main_clock_color(wake_t: f32) -> egui::Color32 {
    color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(252, 253, 255, 228),
        wake_t,
    )
}

pub(crate) fn layout_main_clock(painter: &egui::Painter, wake_t: f32) -> Arc<egui::Galley> {
    painter.layout_no_wrap(
        chrono::Local::now().format("%H:%M").to_string(),
        main_clock_font(),
        main_clock_color(wake_t),
    )
}

pub(crate) fn draw_main_clock(painter: &egui::Painter, time_pos: egui::Pos2, wake_t: f32) {
    if wake_t <= 0.001 {
        return;
    }

    let time_galley = layout_main_clock(painter, wake_t);
    painter.galley(time_pos, time_galley, egui::Color32::WHITE);
}
