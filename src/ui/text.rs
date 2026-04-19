use std::sync::Arc;

use eframe::egui;

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
    ui.ctx().fonts(|fonts| fonts.layout_job(job))
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

pub(crate) fn draw_main_clock(painter: &egui::Painter, time_pos: egui::Pos2, wake_t: f32) {
    if wake_t <= 0.001 {
        return;
    }

    let time_text = chrono::Local::now().format("%H:%M").to_string();
    let time_font = egui::FontId::new(40.0, egui::FontFamily::Name("Bold".into()));
    let time_galley = painter.layout_no_wrap(
        time_text.clone(),
        time_font.clone(),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(245, 247, 252, 168),
            wake_t,
        ),
    );
    let outline = painter.layout_no_wrap(
        time_text,
        time_font,
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 132),
            wake_t,
        ),
    );
    let offset = 0.9_f32;
    for delta in [
        egui::vec2(offset, 0.0),
        egui::vec2(-offset, 0.0),
        egui::vec2(0.0, offset),
        egui::vec2(0.0, -offset),
        egui::vec2(offset, offset),
        egui::vec2(-offset, offset),
        egui::vec2(offset, -offset),
        egui::vec2(-offset, -offset),
    ] {
        painter.galley(time_pos + delta, outline.clone());
    }
    painter.galley(time_pos, time_galley);
}
