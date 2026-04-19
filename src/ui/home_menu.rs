use eframe::egui;

use crate::i18n::AppLanguage;

use super::assets::HintIcons;
use super::anim::{lerp_f32, smoothstep01};
use super::text::color_with_scaled_alpha;

pub fn draw_home_menu(
    ui: &mut egui::Ui,
    language: AppLanguage,
    icons: Option<&HintIcons>,
    current_mode_label: &str,
    half_refresh_label: &str,
    max_refresh_label: &str,
    show_power_options: bool,
    shutdown_hold_progress: f32,
    launch_on_startup_enabled: bool,
    show_launch_on_startup: bool,
    menu_anim: f32,
    selected_option_t: f32,
    wake_anim: f32,
) {
    let wake_t = smoothstep01(wake_anim);
    let menu_t = smoothstep01(menu_anim) * wake_t;
    if menu_t <= 0.001 {
        return;
    }

    let phase_t = |start: f32, end: f32| -> f32 {
        if end <= start {
            return 1.0;
        }
        smoothstep01(((menu_t - start) / (end - start)).clamp(0.0, 1.0))
    };

    let overlay_t = phase_t(0.0, 0.55);
    let sheet_t = phase_t(0.06, 0.68);
    let highlight_t = phase_t(0.22, 1.0);
    let draw_progress_ring = |painter: &egui::Painter,
                              center: egui::Pos2,
                              radius: f32,
                              progress: f32| {
        let clamped = progress.clamp(0.0, 1.0);
        let bg_stroke = egui::Stroke::new(
            2.0,
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 44),
                highlight_t,
            ),
        );
        painter.circle_stroke(center, radius, bg_stroke);

        if clamped <= 0.0 {
            return;
        }

        let fg_stroke = egui::Stroke::new(
            2.6,
            color_with_scaled_alpha(egui::Color32::from_rgb(255, 255, 255), highlight_t),
        );
        let start_angle = -std::f32::consts::FRAC_PI_2;
        let sweep = std::f32::consts::TAU * clamped;
        let segments = ((64.0 * clamped).ceil() as usize).max(8);
        let mut points = Vec::with_capacity(segments + 1);

        for index in 0..=segments {
            let t = index as f32 / segments as f32;
            let angle = start_angle + sweep * t;
            points.push(center + egui::vec2(angle.cos() * radius, angle.sin() * radius));
        }

        painter.add(egui::Shape::line(points, fg_stroke));
    };

    let panel_rect = ui.available_rect_before_wrap();
    let painter = ui.painter();
    painter.rect_filled(
        panel_rect,
        egui::Rounding::ZERO,
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(6, 8, 12, 178),
            overlay_t,
        ),
    );

    let option_height = 94.0;
    let option_gap = 22.0;
    let row_gap = 44.0;
    let section_gap = 16.0;
    let content_padding = 28.0;
    let section_font = egui::FontId::new(24.0, egui::FontFamily::Name("Bold".into()));
    let current_mode_font = egui::FontId::new(20.0, egui::FontFamily::Proportional);
    let power_section_text = show_power_options.then(|| {
        painter.layout_no_wrap(
            language.power_text().to_string(),
            section_font.clone(),
            color_with_scaled_alpha(egui::Color32::from_rgb(150, 158, 170), sheet_t),
        )
    });
    let resolution_section_text = painter.layout_no_wrap(
        language.set_display_resolution_text().to_string(),
        section_font.clone(),
        color_with_scaled_alpha(egui::Color32::from_rgb(150, 158, 170), sheet_t),
    );
    let startup_section_text = painter.layout_no_wrap(
        language.startup_settings_text().to_string(),
        section_font.clone(),
        color_with_scaled_alpha(egui::Color32::from_rgb(150, 158, 170), sheet_t),
    );
    let current_mode_summary = format!(
        "{} {}",
        language.current_display_mode_text(),
        current_mode_label
    );
    let current_mode_text = painter.layout_no_wrap(
        current_mode_summary,
        current_mode_font,
        color_with_scaled_alpha(egui::Color32::from_rgb(134, 142, 152), sheet_t),
    );
    let power_section_text_height = power_section_text
        .as_ref()
        .map(|text| text.size().y)
        .unwrap_or(0.0);
    let resolution_section_text_height = resolution_section_text.size().y;
    let startup_section_text_height = startup_section_text.size().y;
    let current_mode_text_height = current_mode_text.size().y;
    let current_mode_text_width = current_mode_text.size().x;
    let content_height = option_height
        + if show_power_options {
            row_gap + power_section_text_height + section_gap + option_height
        } else {
            0.0
        }
        + row_gap
        + resolution_section_text_height
        + section_gap
        + option_height
        + if show_launch_on_startup {
            row_gap + startup_section_text_height + section_gap + option_height
        } else {
            0.0
        };
    let min_sheet_height = content_height + content_padding * 2.0;
    let max_sheet_height = 540.0_f32.max(min_sheet_height);
    let sheet_height = (panel_rect.height() * 0.34).clamp(min_sheet_height, max_sheet_height);
    let sheet_rect = egui::Rect::from_center_size(
        panel_rect.center(),
        egui::vec2(panel_rect.width(), sheet_height),
    );
    painter.rect_filled(
        sheet_rect,
        egui::Rounding::ZERO,
        color_with_scaled_alpha(egui::Color32::from_rgb(18, 19, 22), sheet_t),
    );

    let primary_option_labels = [language.minimize_app_text(), language.close_app_text()];
    let power_option_labels = [language.sleep_text(), language.shutdown_text()];
    let resolution_option_labels = [half_refresh_label, max_refresh_label];
    let option_font = egui::FontId::new(22.0, egui::FontFamily::Name("Bold".into()));
    let option_detail_font = egui::FontId::new(18.0, egui::FontFamily::Proportional);
    let content_width = (sheet_rect.width() * 0.56).clamp(520.0, 860.0);
    let content_rect = egui::Rect::from_center_size(
        sheet_rect.center(),
        egui::vec2(content_width, content_height),
    );
    let option_width = (content_rect.width() - option_gap) * 0.5;
    let option_inner_padding = 20.0;
    let top_row_y = content_rect.min.y;
    let power_section_y = top_row_y + option_height + row_gap;
    let power_row_y = power_section_y + power_section_text_height + section_gap;
    let resolution_section_y = if show_power_options {
        power_row_y + option_height + row_gap
    } else {
        top_row_y + option_height + row_gap
    };
    let resolution_row_y = resolution_section_y + resolution_section_text_height + section_gap;
    let startup_section_y = resolution_row_y + option_height + row_gap;
    let startup_row_y = startup_section_y + startup_section_text_height + section_gap;
    let startup_index = show_launch_on_startup.then_some(if show_power_options { 6 } else { 4 });
    let shutdown_index = show_power_options.then_some(3usize);
    let option_phase_t = |index: usize| -> f32 {
        let (row_index, column_index) = if startup_index == Some(index) {
            (3, 0)
        } else {
            (index / 2, index % 2)
        };
        let start = 0.14 + row_index as f32 * 0.14 + column_index as f32 * 0.08;
        phase_t(start, (start + 0.60).min(1.0))
    };
    let top_row_rects: Vec<_> = primary_option_labels
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let option_t = option_phase_t(index);
            let option_offset = egui::vec2(0.0, lerp_f32(12.0, 0.0, option_t));
            egui::Rect::from_min_size(
                egui::pos2(
                    content_rect.min.x + index as f32 * (option_width + option_gap),
                    top_row_y,
                ),
                egui::vec2(option_width, option_height),
            )
            .translate(option_offset)
        })
        .collect();
    let power_row_rects: Vec<_> = power_option_labels
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let option_t = option_phase_t(index + 2);
            let option_offset = egui::vec2(0.0, lerp_f32(12.0, 0.0, option_t));
            egui::Rect::from_min_size(
                egui::pos2(
                    content_rect.min.x + index as f32 * (option_width + option_gap),
                    power_row_y,
                ),
                egui::vec2(option_width, option_height),
            )
            .translate(option_offset)
        })
        .collect();
    let resolution_row_rects: Vec<_> = resolution_option_labels
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let option_t = option_phase_t(index + if show_power_options { 4 } else { 2 });
            let option_offset = egui::vec2(0.0, lerp_f32(12.0, 0.0, option_t));
            egui::Rect::from_min_size(
                egui::pos2(
                    content_rect.min.x + index as f32 * (option_width + option_gap),
                    resolution_row_y,
                ),
                egui::vec2(option_width, option_height),
            )
            .translate(option_offset)
        })
        .collect();
    let startup_rect = show_launch_on_startup.then(|| {
        let option_t = option_phase_t(startup_index.unwrap_or_default());
        let option_offset = egui::vec2(0.0, lerp_f32(12.0, 0.0, option_t));
        egui::Rect::from_min_size(
            egui::pos2(content_rect.min.x, startup_row_y),
            egui::vec2(content_rect.width(), option_height),
        )
        .translate(option_offset)
    });
    let mut option_rects = Vec::new();
    option_rects.extend(top_row_rects.iter().copied());
    if show_power_options {
        option_rects.extend(power_row_rects.iter().copied());
    }
    option_rects.extend(resolution_row_rects.iter().copied());
    option_rects.extend(startup_rect.iter().copied());
    let selected_index = selected_option_t
        .round()
        .clamp(0.0, option_rects.len().saturating_sub(1) as f32) as usize;
    let highlight_offset = egui::vec2(0.0, lerp_f32(8.0, 0.0, highlight_t));
    let selected_rect = option_rects[selected_index].translate(highlight_offset);

    for (index, option_rect) in option_rects.iter().enumerate() {
        let option_t = option_phase_t(index);
        painter.rect_filled(
            *option_rect,
            egui::Rounding::same(12.0),
            color_with_scaled_alpha(egui::Color32::from_rgb(28, 30, 34), option_t),
        );
    }

    if let Some(power_section_text) = &power_section_text {
        painter.galley(egui::pos2(content_rect.min.x, power_section_y), power_section_text.clone());
    }
    painter.galley(egui::pos2(content_rect.min.x, resolution_section_y), resolution_section_text);
    painter.galley(
        egui::pos2(
            content_rect.max.x - current_mode_text_width,
            resolution_section_y + (resolution_section_text_height - current_mode_text_height) * 0.5,
        ),
        current_mode_text,
    );
    if show_launch_on_startup {
        painter.galley(egui::pos2(content_rect.min.x, startup_section_y), startup_section_text);
    }

    painter.rect_filled(
        selected_rect,
        egui::Rounding::same(14.0),
        color_with_scaled_alpha(egui::Color32::from_rgb(86, 90, 100), highlight_t),
    );

    let mut option_labels = vec![
        primary_option_labels[0].to_string(),
        primary_option_labels[1].to_string(),
    ];
    if show_power_options {
        option_labels.push(power_option_labels[0].to_string());
        option_labels.push(power_option_labels[1].to_string());
    }
    option_labels.push(resolution_option_labels[0].to_string());
    option_labels.push(resolution_option_labels[1].to_string());
    if show_launch_on_startup {
        option_labels.push(language.launch_on_startup_text().to_string());
    }
    for (index, option_rect) in option_rects.iter().enumerate() {
        let option_t = option_phase_t(index);
        let selectedness = if selected_index == index { 1.0 } else { 0.0 };
        let text_color = egui::Color32::from_rgb(
            lerp_f32(214.0, 248.0, selectedness).round() as u8,
            lerp_f32(218.0, 249.0, selectedness).round() as u8,
            lerp_f32(226.0, 252.0, selectedness).round() as u8,
        );
        let label = option_labels.get(index).map(String::as_str).unwrap_or_default();
        if startup_index == Some(index) {
            let option_text = painter.layout_no_wrap(
                label.to_string(),
                option_font.clone(),
                color_with_scaled_alpha(text_color, option_t),
            );
            let status_text = painter.layout_no_wrap(
                if launch_on_startup_enabled {
                    language.enabled_text().to_string()
                } else {
                    language.disabled_text().to_string()
                },
                option_detail_font.clone(),
                color_with_scaled_alpha(
                    if launch_on_startup_enabled {
                        egui::Color32::from_rgb(164, 214, 174)
                    } else {
                        egui::Color32::from_rgb(146, 154, 164)
                    },
                    option_t,
                ),
            );
            let total_height = option_text.size().y + 6.0 + status_text.size().y;
            let top_y = option_rect.center().y - total_height * 0.5;
            painter.galley(egui::pos2(option_rect.min.x + option_inner_padding, top_y), option_text);
            painter.galley(
                egui::pos2(
                    option_rect.min.x + option_inner_padding,
                    top_y + total_height - status_text.size().y,
                ),
                status_text,
            );
        } else {
            let option_text = painter.layout_no_wrap(
                label.to_string(),
                option_font.clone(),
                color_with_scaled_alpha(text_color, option_t),
            );
            painter.galley(
                egui::pos2(
                    option_rect.min.x + option_inner_padding,
                    option_rect.center().y - option_text.size().y * 0.5,
                ),
                option_text,
            );
        }
    }

    if let Some(icons) = icons {
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(
                selected_rect.max.x - option_inner_padding - 36.0,
                selected_rect.center().y - 18.0,
            ),
            egui::vec2(36.0, 36.0),
        );
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(
            icons.btn_a.id(),
            icon_rect,
            uv,
            color_with_scaled_alpha(egui::Color32::WHITE, highlight_t),
        );
        if shutdown_index == Some(selected_index) {
            draw_progress_ring(
                painter,
                icon_rect.center(),
                icon_rect.width() * 0.52,
                shutdown_hold_progress,
            );
        }
    }
}
