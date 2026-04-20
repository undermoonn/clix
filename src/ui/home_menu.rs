use std::borrow::Cow;
use std::collections::HashMap;

use eframe::egui;

use crate::home_menu_structure::{HomeMenuEntry, HomeMenuLayout, HomeMenuOption};
use crate::i18n::AppLanguage;
use crate::system::external_apps::ExternalAppKind;

use super::anim::{lerp_f32, smoothstep01};
use super::hint_icons::HintIcons;
use super::text::{color_with_scaled_alpha, corner_radius};

fn draw_texture_icon(
    painter: &egui::Painter,
    texture: &egui::TextureHandle,
    icon_rect: egui::Rect,
    tint: egui::Color32,
) {
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    painter.add(egui::Shape::Rect(
        egui::epaint::RectShape::filled(icon_rect, corner_radius(8.0), tint)
            .with_texture(texture.id(), uv),
    ));
}

fn draw_external_app_fallback_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    kind: ExternalAppKind,
    selectedness: f32,
    opacity: f32,
) {
    let tile_fill = match kind {
        ExternalAppKind::DlssSwapper => egui::Color32::from_rgb(18, 28, 18),
        ExternalAppKind::NvidiaApp => egui::Color32::from_rgb(20, 44, 22),
    };
    let tile_stroke = match kind {
        ExternalAppKind::DlssSwapper => egui::Color32::from_rgb(120, 208, 120),
        ExternalAppKind::NvidiaApp => egui::Color32::from_rgb(118, 208, 78),
    };
    let highlight_fill = match kind {
        ExternalAppKind::DlssSwapper => egui::Color32::from_rgb(46, 82, 46),
        ExternalAppKind::NvidiaApp => egui::Color32::from_rgb(58, 112, 44),
    };
    let rounding = corner_radius(16.0);

    painter.rect_filled(
        rect,
        rounding,
        color_with_scaled_alpha(tile_fill, opacity),
    );
    painter.rect_stroke(
        rect,
        rounding,
        egui::Stroke::new(
            lerp_f32(1.2, 2.0, selectedness),
            color_with_scaled_alpha(tile_stroke, opacity),
        ),
        egui::StrokeKind::Middle,
    );

    if selectedness > 0.01 {
        let glow_rect = rect.shrink(3.0);
        painter.rect_stroke(
            glow_rect,
            corner_radius(13.0),
            egui::Stroke::new(
                2.0,
                color_with_scaled_alpha(highlight_fill, opacity * selectedness),
            ),
            egui::StrokeKind::Middle,
        );
    }

    match kind {
        ExternalAppKind::DlssSwapper => {
            let stroke = egui::Stroke::new(
                3.2,
                color_with_scaled_alpha(egui::Color32::from_rgb(146, 236, 126), opacity),
            );
            let left = rect.left() + rect.width() * 0.24;
            let right = rect.right() - rect.width() * 0.24;
            let top = rect.top() + rect.height() * 0.35;
            let bottom = rect.bottom() - rect.height() * 0.35;
            let arrow = rect.width() * 0.12;

            painter.line_segment([egui::pos2(left, top), egui::pos2(right - arrow, top)], stroke);
            painter.line_segment(
                [egui::pos2(right - arrow, top - arrow * 0.6), egui::pos2(right, top)],
                stroke,
            );
            painter.line_segment(
                [egui::pos2(right - arrow, top + arrow * 0.6), egui::pos2(right, top)],
                stroke,
            );

            painter.line_segment(
                [egui::pos2(right, bottom), egui::pos2(left + arrow, bottom)],
                stroke,
            );
            painter.line_segment(
                [egui::pos2(left + arrow, bottom - arrow * 0.6), egui::pos2(left, bottom)],
                stroke,
            );
            painter.line_segment(
                [egui::pos2(left + arrow, bottom + arrow * 0.6), egui::pos2(left, bottom)],
                stroke,
            );
        }
        ExternalAppKind::NvidiaApp => {
            let inner = rect.shrink(rect.width() * 0.19);
            let accent = color_with_scaled_alpha(egui::Color32::from_rgb(118, 208, 78), opacity);
            let white = color_with_scaled_alpha(egui::Color32::WHITE, opacity);

            painter.rect_filled(inner, corner_radius(12.0), accent);

            let left = inner.left() + inner.width() * 0.2;
            let right = inner.right() - inner.width() * 0.2;
            let top = inner.top() + inner.height() * 0.18;
            let bottom = inner.bottom() - inner.height() * 0.18;
            let middle = inner.center().x;
            let stroke = egui::Stroke::new(4.2, white);

            painter.line_segment([egui::pos2(left, bottom), egui::pos2(left, top)], stroke);
            painter.line_segment([egui::pos2(left, top), egui::pos2(right, bottom)], stroke);
            painter.line_segment([egui::pos2(right, bottom), egui::pos2(right, top)], stroke);
            painter.line_segment(
                [egui::pos2(middle - 1.0, inner.center().y), egui::pos2(right, top)],
                stroke,
            );
        }
    }
}

pub fn draw_home_menu(
    ui: &mut egui::Ui,
    language: AppLanguage,
    layout: &HomeMenuLayout,
    external_app_icons: &HashMap<ExternalAppKind, egui::TextureHandle>,
    icons: Option<&HintIcons>,
    current_mode_label: &str,
    half_refresh_label: &str,
    max_refresh_label: &str,
    _show_power_options: bool,
    shutdown_hold_progress: f32,
    launch_on_startup_enabled: bool,
    _show_launch_on_startup: bool,
    menu_anim: f32,
    selected_option_t: f32,
    wake_anim: f32,
) {
    if layout.is_empty() {
        return;
    }

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
        egui::CornerRadius::ZERO,
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
    let external_entries: Vec<_> = layout
        .entries()
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, entry)| matches!(entry.option, HomeMenuOption::ExternalApp(_)))
        .collect();
    let primary_entries: Vec<_> = layout
        .entries()
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, entry)| {
            matches!(
                entry.option,
                HomeMenuOption::MinimizeApp | HomeMenuOption::CloseApp
            )
        })
        .collect();
    let power_entries: Vec<_> = layout
        .entries()
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, entry)| matches!(entry.option, HomeMenuOption::Sleep | HomeMenuOption::Shutdown))
        .collect();
    let resolution_entries: Vec<_> = layout
        .entries()
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, entry)| {
            matches!(
                entry.option,
                HomeMenuOption::HalfMaxRefresh | HomeMenuOption::MaxRefresh
            )
        })
        .collect();
    let startup_entry = layout
        .entries()
        .iter()
        .copied()
        .enumerate()
        .find(|(_, entry)| matches!(entry.option, HomeMenuOption::LaunchOnStartup));
    let show_power_options = !power_entries.is_empty();
    let has_external_apps = !external_entries.is_empty();
    let power_section_text = show_power_options.then(|| {
        painter.layout_no_wrap(
            language.power_text().to_string(),
            section_font.clone(),
            color_with_scaled_alpha(egui::Color32::from_rgb(150, 158, 170), sheet_t),
        )
    });
    let primary_section_text = painter.layout_no_wrap(
        "Big Screen Launcher".to_string(),
        section_font.clone(),
        color_with_scaled_alpha(egui::Color32::from_rgb(150, 158, 170), sheet_t),
    );
    let external_section_text = has_external_apps.then(|| {
        painter.layout_no_wrap(
            language.apps_text().to_string(),
            section_font.clone(),
            color_with_scaled_alpha(egui::Color32::from_rgb(150, 158, 170), sheet_t),
        )
    });
    let resolution_section_text = painter.layout_no_wrap(
        language.set_display_resolution_text().to_string(),
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
    let primary_section_text_height = primary_section_text.size().y;
    let external_section_text_height = external_section_text
        .as_ref()
        .map(|text| text.size().y)
        .unwrap_or(0.0);
    let resolution_section_text_height = resolution_section_text.size().y;
    let current_mode_text_height = current_mode_text.size().y;
    let current_mode_text_width = current_mode_text.size().x;
    let content_height = primary_section_text_height
        + section_gap
        + option_height
        + if show_power_options {
            row_gap + power_section_text_height + section_gap + option_height
        } else {
            0.0
        }
        + row_gap
        + resolution_section_text_height
        + section_gap
        + option_height
        + if has_external_apps {
            row_gap + external_section_text_height + section_gap + option_height
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
        egui::CornerRadius::ZERO,
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
    let startup_option_width = option_width * 0.62;
    let option_inner_padding = 20.0;
    let primary_section_y = content_rect.min.y;
    let primary_row_y = primary_section_y + primary_section_text_height + section_gap;
    let power_section_y = primary_row_y + option_height + row_gap;
    let power_row_y = power_section_y + power_section_text_height + section_gap;
    let resolution_section_y = if show_power_options {
        power_row_y + option_height + row_gap
    } else {
        primary_row_y + option_height + row_gap
    };
    let resolution_row_y = resolution_section_y + resolution_section_text_height + section_gap;
    let external_section_y = resolution_row_y + option_height + row_gap;
    let external_row_y = external_section_y + external_section_text_height + section_gap;
    let startup_row_y = if has_external_apps {
        external_row_y
    } else {
        resolution_row_y
    };
    let option_phase_t = |entry: HomeMenuEntry| -> f32 {
        let start = 0.14 + entry.row as f32 * 0.14 + entry.column as f32 * 0.08;
        phase_t(start, (start + 0.60).min(1.0))
    };

    let build_row_rects = |entries: &[(usize, HomeMenuEntry)], row_y: f32| -> Vec<_> {
        entries
            .iter()
            .map(|(index, entry)| {
                let option_t = option_phase_t(*entry);
                let option_offset = egui::vec2(0.0, lerp_f32(12.0, 0.0, option_t));
                let size = if entry.wide {
                    egui::vec2(content_rect.width(), option_height)
                } else {
                    egui::vec2(option_width, option_height)
                };
                let rect = egui::Rect::from_min_size(
                    egui::pos2(
                        content_rect.min.x + entry.column as f32 * (option_width + option_gap),
                        row_y,
                    ),
                    size,
                )
                .translate(option_offset);
                (*index, *entry, rect)
            })
            .collect()
    };

    let mut option_rects = Vec::new();
    option_rects.extend(build_row_rects(&primary_entries, primary_row_y));
    if show_power_options {
        option_rects.extend(build_row_rects(&power_entries, power_row_y));
    }
    option_rects.extend(build_row_rects(&resolution_entries, resolution_row_y));
    if has_external_apps {
        option_rects.extend(build_row_rects(&external_entries, external_row_y));
    }
    if let Some((index, entry)) = startup_entry {
        let option_t = option_phase_t(entry);
        let option_offset = egui::vec2(0.0, lerp_f32(12.0, 0.0, option_t));
        let startup_right = sheet_rect.max.x - 18.0;
        let startup_x = startup_right - startup_option_width;
        let startup_rect = egui::Rect::from_min_size(
            egui::pos2(startup_x, startup_row_y),
            egui::vec2(startup_option_width, option_height),
        )
        .translate(option_offset);
        option_rects.push((index, entry, startup_rect));
    }

    let selected_index = layout.clamp_selected(selected_option_t.round().max(0.0) as usize);
    let highlight_offset = egui::vec2(0.0, lerp_f32(8.0, 0.0, highlight_t));
    let Some((_, _, selected_rect)) = option_rects
        .iter()
        .find(|(index, _, _)| *index == selected_index)
        .copied()
    else {
        return;
    };
    let selected_rect = selected_rect.translate(highlight_offset);

    for (_, entry, option_rect) in &option_rects {
        let option_t = option_phase_t(*entry);
        painter.rect_filled(
            *option_rect,
            corner_radius(12.0),
            color_with_scaled_alpha(egui::Color32::from_rgb(28, 30, 34), option_t),
        );
    }

    painter.galley(
        egui::pos2(content_rect.min.x, primary_section_y),
        primary_section_text,
        egui::Color32::WHITE,
    );
    if let Some(power_section_text) = &power_section_text {
        painter.galley(
            egui::pos2(content_rect.min.x, power_section_y),
            power_section_text.clone(),
            egui::Color32::WHITE,
        );
    }
    painter.galley(
        egui::pos2(content_rect.min.x, resolution_section_y),
        resolution_section_text,
        egui::Color32::WHITE,
    );
    painter.galley(
        egui::pos2(
            content_rect.max.x - current_mode_text_width,
            resolution_section_y + (resolution_section_text_height - current_mode_text_height) * 0.5,
        ),
        current_mode_text,
        egui::Color32::WHITE,
    );
    if let Some(external_section_text) = &external_section_text {
        painter.galley(
            egui::pos2(content_rect.min.x, external_section_y),
            external_section_text.clone(),
            egui::Color32::WHITE,
        );
    }

    painter.rect_filled(
        selected_rect,
        corner_radius(14.0),
        color_with_scaled_alpha(egui::Color32::from_rgb(86, 90, 100), highlight_t),
    );

    for (index, entry, option_rect) in &option_rects {
        let option_t = option_phase_t(*entry);
        let selectedness = if selected_index == *index { 1.0 } else { 0.0 };
        let text_color = egui::Color32::from_rgb(
            lerp_f32(214.0, 248.0, selectedness).round() as u8,
            lerp_f32(218.0, 249.0, selectedness).round() as u8,
            lerp_f32(226.0, 252.0, selectedness).round() as u8,
        );
        if let HomeMenuOption::ExternalApp(kind) = entry.option {
            let icon_size = (option_rect.height() - option_inner_padding * 2.0).clamp(42.0, 54.0);
            let icon_rect = egui::Rect::from_min_size(
                egui::pos2(
                    option_rect.min.x + option_inner_padding,
                    option_rect.center().y - icon_size * 0.5,
                ),
                egui::vec2(icon_size, icon_size),
            );
            if let Some(texture) = external_app_icons.get(&kind) {
                draw_texture_icon(
                    &painter,
                    texture,
                    icon_rect,
                    color_with_scaled_alpha(egui::Color32::WHITE, option_t),
                );
            } else {
                draw_external_app_fallback_icon(&painter, icon_rect, kind, selectedness, option_t);
            }

            let label = match kind {
                ExternalAppKind::DlssSwapper => language.dlss_swapper_text(),
                ExternalAppKind::NvidiaApp => language.nvidia_app_text(),
            };
            let option_text = painter.layout_no_wrap(
                label.to_string(),
                option_font.clone(),
                color_with_scaled_alpha(text_color, option_t),
            );
            painter.galley(
                egui::pos2(
                    icon_rect.max.x + 16.0,
                    option_rect.center().y - option_text.size().y * 0.5,
                ),
                option_text,
                egui::Color32::WHITE,
            );
            continue;
        }

        let label = match entry.option {
            HomeMenuOption::MinimizeApp => Cow::Borrowed(primary_option_labels[0]),
            HomeMenuOption::CloseApp => Cow::Borrowed(primary_option_labels[1]),
            HomeMenuOption::Sleep => Cow::Borrowed(power_option_labels[0]),
            HomeMenuOption::Shutdown => Cow::Borrowed(power_option_labels[1]),
            HomeMenuOption::HalfMaxRefresh => Cow::Borrowed(resolution_option_labels[0]),
            HomeMenuOption::MaxRefresh => Cow::Borrowed(resolution_option_labels[1]),
            HomeMenuOption::LaunchOnStartup => Cow::Borrowed(language.launch_on_startup_text()),
            HomeMenuOption::ExternalApp(_) => continue,
        };

        if matches!(entry.option, HomeMenuOption::LaunchOnStartup) {
            let option_text = painter.layout_no_wrap(
                label.into_owned(),
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
            painter.galley(
                egui::pos2(option_rect.min.x + option_inner_padding, top_y),
                option_text,
                egui::Color32::WHITE,
            );
            painter.galley(
                egui::pos2(
                    option_rect.min.x + option_inner_padding,
                    top_y + total_height - status_text.size().y,
                ),
                status_text,
                egui::Color32::WHITE,
            );
        } else {
            let option_text = painter.layout_no_wrap(
                label.into_owned(),
                option_font.clone(),
                color_with_scaled_alpha(text_color, option_t),
            );
            painter.galley(
                egui::pos2(
                    option_rect.min.x + option_inner_padding,
                    option_rect.center().y - option_text.size().y * 0.5,
                ),
                option_text,
                egui::Color32::WHITE,
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
        if layout.is_shutdown_selected(selected_index) {
            draw_progress_ring(
                painter,
                icon_rect.center(),
                icon_rect.width() * 0.52,
                shutdown_hold_progress,
            );
        }
    }
}
