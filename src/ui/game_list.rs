use std::collections::HashMap;

use eframe::egui;

use crate::game::{Game, GameIconKey, GameSource};
use crate::i18n::AppLanguage;
use crate::steam::AchievementSummary;

use super::anim::{lerp_f32, smoothstep01};
use super::header::{
    build_selected_game_header, draw_selected_game_badge, draw_selected_game_summary,
    draw_selected_game_title, SelectedGameSummaryStyle,
};
use super::text::{build_wrapped_galley, color_with_scaled_alpha, corner_radius};

const HOME_LAYOUT_BASELINE_PIXELS_PER_POINT: f32 = 2.0;

fn design_units(value: f32, pixels_per_point: f32) -> f32 {
    value * (HOME_LAYOUT_BASELINE_PIXELS_PER_POINT / pixels_per_point.max(0.01))
}

fn launch_press_t(elapsed_seconds: f32) -> f32 {
    let press_in_duration = 0.06;
    let release_duration = 0.1;

    if elapsed_seconds <= press_in_duration {
        smoothstep01(elapsed_seconds / press_in_duration)
    } else {
        let release_t = smoothstep01((elapsed_seconds - press_in_duration) / release_duration);
        1.0 - release_t
    }
}

fn launch_icon_scale(elapsed_seconds: f32) -> f32 {
    let press_t = launch_press_t(elapsed_seconds);
    lerp_f32(1.0, 0.94, press_t)
}

fn launch_icon_offset_y(elapsed_seconds: f32) -> f32 {
    let press_t = launch_press_t(elapsed_seconds);
    lerp_f32(0.0, 4.0, press_t)
}

fn scroll_compensation_for_offset(offset_f: f32, selected_icon_extra: f32) -> f32 {
    if offset_f <= 0.0 {
        0.0
    } else {
        selected_icon_extra * offset_f.clamp(0.0, 1.0)
    }
}

fn item_left_for_offset(
    anchor_x: f32,
    column_spacing: f32,
    selected_icon_extra: f32,
    offset_f: f32,
) -> f32 {
    anchor_x + offset_f * column_spacing
        + scroll_compensation_for_offset(offset_f, selected_icon_extra)
}

fn draw_game_icon(
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

fn draw_game_icon_focus_frame(
    painter: &egui::Painter,
    icon_rect: egui::Rect,
    focus_t: f32,
    wake_t: f32,
) {
    if focus_t <= 0.001 {
        return;
    }

    let focus_rect = icon_rect.expand(5.0);
    painter.rect_filled(
        focus_rect,
        corner_radius(12.0),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(248, 250, 255, 54),
            wake_t * focus_t,
        ),
    );
    painter.rect_stroke(
        focus_rect,
        corner_radius(12.0),
        egui::Stroke::new(
            lerp_f32(1.2, 3.0, focus_t),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 168),
                wake_t * focus_t,
            ),
        ),
        egui::StrokeKind::Outside,
    );
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GameStatusDot {
    Running,
    Launching,
}

fn draw_status_dot(painter: &egui::Painter, icon_rect: egui::Rect, status: GameStatusDot) {
    let radius = (icon_rect.width().min(icon_rect.height()) * 0.055).clamp(4.0, 7.0);
    let inset = (radius * 0.9).clamp(6.0, 10.0);
    let center = egui::pos2(icon_rect.max.x - inset - radius, icon_rect.min.y + inset + radius);
    let halo_alpha = 84;
    let halo_radius = radius + 3.4;
    let dot_color = match status {
        GameStatusDot::Running => egui::Color32::from_rgba_unmultiplied(78, 201, 108, 220),
        GameStatusDot::Launching => egui::Color32::from_rgba_unmultiplied(232, 188, 64, 228),
    };

    painter.circle_filled(
        center,
        halo_radius,
        egui::Color32::from_rgba_unmultiplied(12, 20, 14, halo_alpha),
    );
    painter.circle_filled(center, radius, dot_color);
}

fn draw_launch_notice_overlay(
    ui: &egui::Ui,
    painter: &egui::Painter,
    icon_rect: egui::Rect,
    notice_text: &str,
    action_icon: Option<&egui::TextureHandle>,
    use_action_icon: bool,
    overlay_t: f32,
    overlay_color: egui::Color32,
    wake_t: f32,
) {
    if overlay_t <= 0.001 {
        return;
    }

    let pixels_per_point = ui.ctx().pixels_per_point().max(0.01);
    let alpha_scale = wake_t.clamp(0.0, 1.0);
    if alpha_scale <= 0.001 {
        return;
    }

    let icon_size = design_units(24.0, pixels_per_point);
    let text_gap = design_units(8.0, pixels_per_point);
    let icon_and_text_padding = if use_action_icon && action_icon.is_some() {
        icon_size + text_gap
    } else {
        0.0
    };
    let max_text_width =
        (icon_rect.width() - design_units(18.0, pixels_per_point) - icon_and_text_padding)
            .max(design_units(48.0, pixels_per_point));
    let text_font = egui::FontId::new(
        design_units(17.0, pixels_per_point),
        egui::FontFamily::Name("Bold".into()),
    );
    let text_color = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(252, 253, 255, 248),
        alpha_scale,
    );
    let text_galley = build_wrapped_galley(ui, notice_text.to_string(), text_font, text_color, max_text_width);
    let target_height = icon_rect.height() * 0.25;
    let overlay_height = lerp_f32(0.0, target_height, overlay_t);
    let overlay_rect = egui::Rect::from_min_max(
        egui::pos2(icon_rect.min.x, icon_rect.max.y - overlay_height),
        icon_rect.max,
    );
    let overlay_painter = painter.with_clip_rect(icon_rect);

    overlay_painter.rect_filled(
        overlay_rect,
        egui::CornerRadius {
            nw: 0,
            ne: 0,
            sw: 8,
            se: 8,
        },
        color_with_scaled_alpha(overlay_color, alpha_scale),
    );

    let text_offset_y = lerp_f32(design_units(16.0, pixels_per_point), 0.0, overlay_t);
    let content_width = text_galley.size().x + icon_and_text_padding;
    let content_left = icon_rect.center().x - content_width * 0.5;
    if use_action_icon {
        if let Some(action_icon) = action_icon {
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            let icon_rect = egui::Rect::from_min_size(
                egui::pos2(content_left, overlay_rect.center().y - icon_size * 0.5 + text_offset_y),
                egui::vec2(icon_size, icon_size),
            );
            overlay_painter.image(
                action_icon.id(),
                icon_rect,
                uv,
                color_with_scaled_alpha(egui::Color32::WHITE, alpha_scale),
            );
        }
    }
    let text_pos = egui::pos2(
        content_left + icon_and_text_padding,
        overlay_rect.center().y - text_galley.size().y * 0.5 + text_offset_y,
    );
    overlay_painter.galley(text_pos, text_galley, egui::Color32::WHITE);
}

pub fn draw_game_list(
    ui: &mut egui::Ui,
    language: AppLanguage,
    games: &[Game],
    selected: usize,
    select_anim: f32,
    home_settings_focus_anim: f32,
    achievement_panel_anim: f32,
    scroll_offset: f32,
    game_icons: &HashMap<GameIconKey, egui::TextureHandle>,
    action_icon_a: Option<&egui::TextureHandle>,
    launch_feedback: Option<(usize, f32)>,
    launch_notice: Option<(usize, String, f32, egui::Color32, bool)>,
    launching_index: Option<usize>,
    running_indices: &[usize],
    summary_cards_visibility: f32,
    achievement_summary_for_selected: Option<&AchievementSummary>,
    achievement_summary_reveal_for_selected: f32,
    previous_achievement_summary: Option<&AchievementSummary>,
    previous_achievement_summary_reveal: f32,
    wake_anim: f32,
) {
    let pixels_per_point = ui.ctx().pixels_per_point().max(0.01);
    let home_layout_scale = HOME_LAYOUT_BASELINE_PIXELS_PER_POINT / pixels_per_point;
    let base_icon_size = design_units(152.0, pixels_per_point);
    let selected_icon_size = design_units(224.0, pixels_per_point);
    let selected_icon_extra = selected_icon_size - base_icon_size;

    let panel_rect = ui.available_rect_before_wrap();
    let padding = design_units(50.0, pixels_per_point);
    let padded_rect = panel_rect.shrink(padding);
    let page_scroll_t = smoothstep01(achievement_panel_anim);
    let page_offset_y = -panel_rect.height() * page_scroll_t;
    let wake_t = smoothstep01(wake_anim);
    let game_focus_visibility = 1.0 - smoothstep01(home_settings_focus_anim);
    let wake_offset_y = lerp_f32(design_units(42.0, pixels_per_point), 0.0, wake_t);

    let selected_size = design_units(34.0, pixels_per_point);
    let base_size = design_units(20.0, pixels_per_point);
    let column_spacing = design_units(180.0, pixels_per_point);

    let hero_ratio = 1240.0 / 3840.0;
    let img_bottom = panel_rect.min.y + panel_rect.width() * hero_ratio;
    let content_top =
        img_bottom + design_units(32.0, pixels_per_point) + page_offset_y + wake_offset_y;
    let anchor_x = padded_rect.min.x + design_units(24.0, pixels_per_point);
    let painter = ui.painter().with_clip_rect(panel_rect);
    let selected_focus_t = smoothstep01(select_anim);
    let selected_meta_t = smoothstep01((select_anim - 0.18) / 0.82);
    let selected_text_color = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255),
        wake_t,
    );
    let unselected_text_color = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(200, 200, 210, 220),
        wake_t,
    );

    if launch_feedback.is_some() {
        ui.ctx().request_repaint();
    }

    for (i, g) in games.iter().enumerate() {
        let offset_f = i as f32 - scroll_offset;
        let is_selected = i == selected;
        let x_pos = item_left_for_offset(anchor_x, column_spacing, selected_icon_extra, offset_f);

        // Cheap horizontal culling: skip items whose icon slot lies entirely
        // outside the panel. Selected always renders (header/title extend
        // beyond the icon and must be present for the meta animation).
        if !is_selected && (x_pos + selected_icon_size < panel_rect.min.x || x_pos > panel_rect.max.x)
        {
            continue;
        }

        let dist = offset_f.abs();
        let icon_focus_t = (1.0 - dist).clamp(0.0, 1.0);
        let selection_t = if is_selected { selected_focus_t } else { 0.0 };
        let meta_t = if is_selected { selected_meta_t } else { 0.0 };
        let launch_elapsed_seconds = launch_feedback
            .filter(|(launch_index, _)| *launch_index == i)
            .map(|(_, elapsed_seconds)| elapsed_seconds);
        let is_running = running_indices.contains(&i);
        let status_dot = if is_running {
            Some(GameStatusDot::Running)
        } else if launching_index == Some(i) {
            Some(GameStatusDot::Launching)
        } else if !matches!(g.source, GameSource::Steam)
            && launch_feedback
                .map(|(launch_index, _)| launch_index == i)
                .unwrap_or(false)
        {
            Some(GameStatusDot::Running)
        } else {
            None
        };
        let font_size = if is_selected {
            base_size + (selected_size - base_size) * selection_t
        } else {
            base_size
        };
        let text_color = if is_selected {
            selected_text_color
        } else {
            unselected_text_color
        };

        let icon_slot_size = base_icon_size + selected_icon_extra * icon_focus_t;
        let icon_scale = launch_elapsed_seconds.map(launch_icon_scale).unwrap_or(1.0);
        let icon_size = icon_slot_size * icon_scale;
        let icon_offset_y = launch_elapsed_seconds.map(launch_icon_offset_y).unwrap_or(0.0);

        let font_id = egui::FontId::proportional(font_size);
        let icon_slot_rect = egui::Rect::from_min_size(
            egui::pos2(x_pos, content_top),
            egui::vec2(icon_slot_size, icon_slot_size),
        );
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(
                icon_slot_rect.min.x + (icon_slot_rect.width() - icon_size) * 0.5,
                icon_slot_rect.min.y + (icon_slot_rect.height() - icon_size) + icon_offset_y,
            ),
            egui::vec2(icon_size, icon_size),
        );

        let border_focus_t = if is_selected {
            selection_t * game_focus_visibility
        } else {
            0.0
        };

        draw_game_icon_focus_frame(&painter, icon_rect, border_focus_t, wake_t);

        if let Some(icon_tex) = game_icons.get(&g.icon_key()) {
            let icon_tint = color_with_scaled_alpha(egui::Color32::WHITE, wake_t);
            draw_game_icon(&painter, icon_tex, icon_rect, icon_tint);

            if let Some(status_dot) = status_dot.filter(|_| wake_t > 0.12) {
                draw_status_dot(&painter, icon_rect, status_dot);
            }

            if let Some((notice_index, notice_text, notice_overlay_t, notice_color, use_action_icon)) = &launch_notice {
                if *notice_index == i {
                    draw_launch_notice_overlay(
                        ui,
                        &painter,
                        icon_rect,
                        notice_text,
                        action_icon_a,
                        *use_action_icon,
                        *notice_overlay_t,
                        *notice_color,
                        wake_t,
                    );
                }
            }
        }

        if is_selected {
            let title_x = if i + 1 < games.len() {
                item_left_for_offset(
                    anchor_x,
                    column_spacing,
                    selected_icon_extra,
                    (i + 1) as f32 - scroll_offset,
                )
            } else {
                icon_slot_rect.max.x + design_units(18.0, pixels_per_point)
            };
            let header_width =
                (icon_slot_size * 2.0 + design_units(28.0, pixels_per_point))
                    .max(design_units(320.0, pixels_per_point));
            let header = build_selected_game_header(
                ui,
                &painter,
                language,
                g,
                achievement_summary_for_selected,
                achievement_summary_reveal_for_selected,
                previous_achievement_summary,
                previous_achievement_summary_reveal,
                font_id,
                text_color,
                design_units(17.0, pixels_per_point),
                140.0 * meta_t,
                header_width,
            );
            let title_y = icon_slot_rect.max.y - header.title_galley.size().y;
            let badge_pos = egui::pos2(title_x, title_y);
            let summary_pos = egui::pos2(
                icon_slot_rect.min.x,
                icon_slot_rect.max.y + design_units(36.0, pixels_per_point),
            );
            let playtime_width = icon_slot_rect.width();
            let achievement_x = badge_pos.x;
            let achievement_width = ((padded_rect.max.x
                - achievement_x
                - design_units(24.0, pixels_per_point))
                .min(design_units(292.0, pixels_per_point)))
                .max(design_units(220.0, pixels_per_point));
            let summary_style = SelectedGameSummaryStyle {
                card_height: design_units(106.0, pixels_per_point),
                layout_scale: home_layout_scale,
                ..SelectedGameSummaryStyle::default()
            };
            let badge_offset = draw_selected_game_badge(
                &painter,
                g,
                badge_pos,
                header.title_galley.size(),
                wake_t,
            );
            let title_pos = egui::pos2(title_x + badge_offset, title_y);

            draw_selected_game_title(&painter, &header, &g.name, title_pos, wake_t);
            if summary_cards_visibility > 0.001 && matches!(g.source, GameSource::Steam) {
                draw_selected_game_summary(
                    &painter,
                    language,
                    g,
                    achievement_summary_for_selected,
                    achievement_summary_reveal_for_selected,
                    summary_pos,
                    playtime_width,
                    achievement_x,
                    achievement_width,
                    &summary_style,
                    summary_cards_visibility,
                    wake_t,
                );
            }
        }
    }
}
