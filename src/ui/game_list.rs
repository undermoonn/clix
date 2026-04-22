use std::collections::HashMap;

use eframe::egui;

use crate::i18n::AppLanguage;
use crate::steam::{AchievementSummary, Game};

use super::anim::{lerp_f32, smoothstep01};
use super::header::{
    build_selected_game_header, draw_selected_game_badge, draw_selected_game_summary,
    draw_selected_game_title, SelectedGameSummaryStyle,
};
use super::text::{color_with_scaled_alpha, corner_radius};

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

fn draw_running_status_dot(painter: &egui::Painter, icon_rect: egui::Rect) {
    let radius = (icon_rect.width().min(icon_rect.height()) * 0.055).clamp(4.0, 7.0);
    let inset = (radius * 0.9).clamp(6.0, 10.0);
    let center = egui::pos2(icon_rect.max.x - inset - radius, icon_rect.min.y + inset + radius);
    let halo_alpha = 84;
    let halo_radius = radius + 3.4;

    painter.circle_filled(
        center,
        halo_radius,
        egui::Color32::from_rgba_unmultiplied(12, 20, 14, halo_alpha),
    );
    painter.circle_filled(
        center,
        radius,
        egui::Color32::from_rgba_unmultiplied(78, 201, 108, 220),
    );
}

pub fn draw_game_list(
    ui: &mut egui::Ui,
    language: AppLanguage,
    games: &[Game],
    selected: usize,
    select_anim: f32,
    achievement_panel_anim: f32,
    scroll_offset: f32,
    game_icons: &HashMap<u32, egui::TextureHandle>,
    launch_feedback: Option<(usize, f32)>,
    running_indices: &[usize],
    _achievement_panel_active: bool,
    achievement_summary_for_selected: Option<&AchievementSummary>,
    achievement_summary_reveal_for_selected: f32,
    previous_achievement_summary: Option<&AchievementSummary>,
    previous_achievement_summary_reveal: f32,
    wake_anim: f32,
) {
    let base_icon_size: f32 = 152.0;
    let selected_icon_size: f32 = 224.0;
    let selected_icon_extra = selected_icon_size - base_icon_size;

    let panel_rect = ui.available_rect_before_wrap();
    let padding = 50.0;
    let padded_rect = panel_rect.shrink(padding);
    let page_scroll_t = smoothstep01(achievement_panel_anim);
    let page_offset_y = -panel_rect.height() * page_scroll_t;
    let wake_t = smoothstep01(wake_anim);
    let wake_offset_y = lerp_f32(42.0, 0.0, wake_t);

    let selected_size = 34.0;
    let base_size = 20.0;
    let column_spacing = 180.0;

    let hero_ratio = 1240.0 / 3840.0;
    let img_bottom = panel_rect.min.y + panel_rect.width() * hero_ratio;
    let content_top = img_bottom + 32.0 + page_offset_y + wake_offset_y;
    let anchor_x = padded_rect.min.x + 24.0;
    let painter = ui.painter().with_clip_rect(panel_rect);
    let item_left_for_offset = |offset_f: f32| {
        let dist = offset_f.abs();
        let sign = if offset_f >= 0.0 { 1.0 } else { -1.0 };
        let right_side_compensation = if offset_f > 0.0 {
            selected_icon_extra * smoothstep01(offset_f.clamp(0.0, 1.0))
        } else {
            0.0
        };

        anchor_x + sign * dist * column_spacing + right_side_compensation
    };

    if launch_feedback.is_some() {
        ui.ctx().request_repaint();
    }

    for (i, g) in games.iter().enumerate() {
        let offset_f = i as f32 - scroll_offset;
        let is_selected = i == selected;

        let dist = offset_f.abs();
        let icon_focus_t = smoothstep01((1.0 - dist).clamp(0.0, 1.0));
        let selection_t = if is_selected {
            smoothstep01(select_anim)
        } else {
            0.0
        };
        let x_pos = item_left_for_offset(offset_f);
        let meta_t = if is_selected {
            smoothstep01((select_anim - 0.18) / 0.82)
        } else {
            0.0
        };
        let launch_elapsed_seconds = launch_feedback
            .filter(|(launch_index, _)| *launch_index == i)
            .map(|(_, elapsed_seconds)| elapsed_seconds);
        let is_running = running_indices.contains(&i);
        let show_running_status = is_running
            || launch_feedback
                .map(|(launch_index, _)| launch_index == i)
                .unwrap_or(false);
        let font_size = if is_selected {
            base_size + (selected_size - base_size) * selection_t
        } else {
            base_size
        };

        let text_alpha = if is_selected { 255 } else { 220 };
        let text_color = if is_selected {
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255),
                wake_t,
            )
        } else {
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(200, 200, 210, text_alpha),
                wake_t,
            )
        };

        let icon_slot_size = base_icon_size + selected_icon_extra * icon_focus_t;
        let icon_scale = launch_elapsed_seconds.map(launch_icon_scale).unwrap_or(1.0);
        let icon_size = icon_slot_size * icon_scale;
        let icon_offset_y = launch_elapsed_seconds.map(launch_icon_offset_y).unwrap_or(0.0);
        let item_left = x_pos;

        let font_id = egui::FontId::proportional(font_size);
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(
                item_left + (icon_slot_size - icon_size) * 0.5,
                content_top + (icon_slot_size - icon_size) + icon_offset_y,
            ),
            egui::vec2(icon_size, icon_size),
        );

        if let Some(app_id) = g.app_id {
            if let Some(icon_tex) = game_icons.get(&app_id) {
                let icon_tint = color_with_scaled_alpha(egui::Color32::WHITE, wake_t);
                draw_game_icon(&painter, icon_tex, icon_rect, icon_tint);

                if show_running_status && wake_t > 0.12 {
                    draw_running_status_dot(&painter, icon_rect);
                }
            }
        }

        if is_selected {
            let title_x = if i + 1 < games.len() {
                item_left_for_offset((i + 1) as f32 - scroll_offset)
            } else {
                icon_rect.max.x + 18.0
            };
            let header_width = (icon_slot_size * 2.0 + 28.0).max(320.0);
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
                17.0,
                140.0 * meta_t,
                header_width,
            );
            let title_y = icon_rect.max.y - header.title_galley.size().y;
            let badge_pos = egui::pos2(title_x, title_y);
            let summary_pos = egui::pos2(icon_rect.min.x, icon_rect.max.y + 36.0);
            let playtime_width = icon_rect.width();
            let achievement_x = badge_pos.x;
            let achievement_width = ((padded_rect.max.x - achievement_x - 24.0).min(292.0)).max(220.0);
            let badge_offset =
                draw_selected_game_badge(&painter, badge_pos, header.title_galley.size(), wake_t);
            let title_pos = egui::pos2(title_x + badge_offset, title_y);

            draw_selected_game_title(&painter, &header, &g.name, title_pos, wake_t);
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
                &SelectedGameSummaryStyle::default(),
                wake_t,
            );
        }
    }
}
