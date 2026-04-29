use std::collections::HashMap;

use eframe::egui;

use crate::game::{Game, GameIconKey, GameSource};
use crate::i18n::AppLanguage;
use crate::steam::AchievementSummary;

use super::header::{
    build_selected_game_header, draw_selected_game_badge, draw_selected_game_summary,
    draw_selected_game_title, SelectedGameSummaryStyle,
};
use super::text::{build_wrapped_galley, color_with_scaled_alpha, corner_radius};
use super::{design_units, lerp_f32, smoothstep01, viewport_layout_scale};

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

pub(super) fn launch_icon_scale(elapsed_seconds: f32) -> f32 {
    let press_t = launch_press_t(elapsed_seconds);
    lerp_f32(1.0, 0.94, press_t)
}

pub(super) fn launch_icon_offset_y(elapsed_seconds: f32, layout_scale: f32) -> f32 {
    let press_t = launch_press_t(elapsed_seconds);
    lerp_f32(0.0, design_units(4.0, layout_scale), press_t)
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
    anchor_x
        + offset_f * column_spacing
        + scroll_compensation_for_offset(offset_f, selected_icon_extra)
}

fn draw_game_icon(
    painter: &egui::Painter,
    texture: &egui::TextureHandle,
    icon_rect: egui::Rect,
    layout_scale: f32,
    tint: egui::Color32,
) {
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    painter.add(egui::Shape::Rect(
        egui::epaint::RectShape::filled(
            icon_rect,
            corner_radius(design_units(8.0, layout_scale)),
            tint,
        )
        .with_texture(texture.id(), uv),
    ));
}

fn draw_shelf_entry(
    painter: &egui::Painter,
    texture: &egui::TextureHandle,
    icon_rect: egui::Rect,
    layout_scale: f32,
    icon_focus_t: f32,
    wake_t: f32,
) {
    painter.rect_filled(
        icon_rect,
        corner_radius(design_units(8.0, layout_scale)),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(34, 36, 40, 236),
            wake_t,
        ),
    );
    painter.rect_stroke(
        icon_rect,
        corner_radius(design_units(8.0, layout_scale)),
        egui::Stroke::new(
            design_units(1.2, layout_scale),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 76),
                wake_t,
            ),
        ),
        egui::StrokeKind::Middle,
    );

    let shelf_icon_size = lerp_f32(
        design_units(104.0, layout_scale),
        design_units(152.0, layout_scale),
        icon_focus_t,
    );
    let shelf_icon_rect = egui::Rect::from_center_size(
        icon_rect.center(),
        egui::vec2(shelf_icon_size, shelf_icon_size),
    );
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    painter.image(
        texture.id(),
        shelf_icon_rect,
        uv,
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(198, 204, 214, 210),
            wake_t,
        ),
    );
}

fn draw_game_icon_focus_frame(
    painter: &egui::Painter,
    icon_rect: egui::Rect,
    layout_scale: f32,
    focus_t: f32,
    wake_t: f32,
) {
    if focus_t <= 0.001 {
        return;
    }

    let focus_rect = icon_rect.expand(design_units(5.0, layout_scale));
    painter.rect_filled(
        focus_rect,
        corner_radius(design_units(12.0, layout_scale)),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(248, 250, 255, 54),
            wake_t * focus_t,
        ),
    );
    painter.rect_stroke(
        focus_rect,
        corner_radius(design_units(12.0, layout_scale)),
        egui::Stroke::new(
            lerp_f32(
                design_units(1.2, layout_scale),
                design_units(3.0, layout_scale),
                focus_t,
            ),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 168),
                wake_t * focus_t,
            ),
        ),
        egui::StrokeKind::Outside,
    );
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum GameStatusDot {
    Running,
    Launching,
}

pub(super) fn draw_status_dot(
    painter: &egui::Painter,
    icon_rect: egui::Rect,
    layout_scale: f32,
    status: GameStatusDot,
) {
    let radius = (icon_rect.width().min(icon_rect.height()) * 0.055).clamp(
        design_units(4.0, layout_scale),
        design_units(7.0, layout_scale),
    );
    let inset = (radius * 0.9).clamp(
        design_units(6.0, layout_scale),
        design_units(10.0, layout_scale),
    );
    let center = egui::pos2(
        icon_rect.max.x - inset - radius,
        icon_rect.min.y + inset + radius,
    );
    let halo_alpha = 84;
    let halo_radius = radius + design_units(3.4, layout_scale);
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

pub(super) fn draw_game_icon_overlay(
    ui: &egui::Ui,
    painter: &egui::Painter,
    icon_rect: egui::Rect,
    layout_scale: f32,
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

    let alpha_scale = wake_t.clamp(0.0, 1.0);
    if alpha_scale <= 0.001 {
        return;
    }

    let icon_size = design_units(24.0, layout_scale);
    let text_gap = design_units(8.0, layout_scale);
    let icon_and_text_padding = if use_action_icon && action_icon.is_some() {
        icon_size + text_gap
    } else {
        0.0
    };
    let max_text_width =
        (icon_rect.width() - design_units(18.0, layout_scale) - icon_and_text_padding)
            .max(design_units(48.0, layout_scale));
    let text_font = egui::FontId::new(
        design_units(17.0, layout_scale),
        egui::FontFamily::Name("Bold".into()),
    );
    let text_color = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(252, 253, 255, 248),
        alpha_scale,
    );
    let text_galley = build_wrapped_galley(
        ui,
        notice_text.to_string(),
        text_font,
        text_color,
        max_text_width,
    );
    let content_height = if use_action_icon && action_icon.is_some() {
        text_galley.size().y.max(icon_size)
    } else {
        text_galley.size().y
    };
    let target_height = (content_height + design_units(18.0, layout_scale))
        .max(icon_rect.height() * 0.25)
        .min(icon_rect.height() * 0.52);
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
            sw: design_units(8.0, layout_scale).round().clamp(0.0, 255.0) as u8,
            se: design_units(8.0, layout_scale).round().clamp(0.0, 255.0) as u8,
        },
        color_with_scaled_alpha(overlay_color, alpha_scale),
    );

    let text_offset_y = lerp_f32(design_units(16.0, layout_scale), 0.0, overlay_t);
    let content_width = text_galley.size().x + icon_and_text_padding;
    let content_left = icon_rect.center().x - content_width * 0.5;
    if use_action_icon {
        if let Some(action_icon) = action_icon {
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            let icon_rect = egui::Rect::from_min_size(
                egui::pos2(
                    content_left,
                    overlay_rect.center().y - icon_size * 0.5 + text_offset_y,
                ),
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
    home_game_indices: &[usize],
    selected: usize,
    select_anim: f32,
    home_settings_focus_anim: f32,
    achievement_panel_anim: f32,
    scroll_offset: f32,
    game_icons: &HashMap<GameIconKey, egui::TextureHandle>,
    shelf_icon: Option<&egui::TextureHandle>,
    action_icon_a: Option<&egui::TextureHandle>,
    launch_feedback: Option<(usize, f32)>,
    launch_notice: Option<(usize, String, f32, egui::Color32, bool)>,
    steam_update_notice: Option<(usize, String, egui::Color32)>,
    launching_index: Option<usize>,
    running_indices: &[usize],
    summary_cards_visibility: f32,
    achievement_summary_for_selected: Option<&AchievementSummary>,
    achievement_summary_reveal_for_selected: f32,
    previous_achievement_summary: Option<&AchievementSummary>,
    previous_achievement_summary_reveal: f32,
    wake_anim: f32,
) {
    let panel_rect = ui.available_rect_before_wrap();
    let home_layout_scale = viewport_layout_scale(panel_rect);
    let base_icon_size = design_units(152.0, home_layout_scale);
    let selected_icon_size = design_units(224.0, home_layout_scale);
    let selected_icon_extra = selected_icon_size - base_icon_size;

    let padding = design_units(50.0, home_layout_scale);
    let padded_rect = panel_rect.shrink(padding);
    let page_scroll_t = smoothstep01(achievement_panel_anim);
    let page_offset_y = -panel_rect.height() * page_scroll_t;
    let wake_t = smoothstep01(wake_anim);
    let game_focus_visibility = 1.0 - smoothstep01(home_settings_focus_anim);
    let wake_offset_y = lerp_f32(design_units(42.0, home_layout_scale), 0.0, wake_t);

    let selected_size = design_units(34.0, home_layout_scale);
    let base_size = design_units(20.0, home_layout_scale);
    let column_spacing = design_units(172.0, home_layout_scale);

    let hero_ratio = 1240.0 / 3840.0;
    let img_bottom = panel_rect.min.y + panel_rect.width() * hero_ratio;
    let content_top =
        img_bottom + design_units(32.0, home_layout_scale) + page_offset_y + wake_offset_y;
    let anchor_x = padded_rect.min.x + design_units(24.0, home_layout_scale);
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

    let item_count = home_game_indices.len() + 1;
    for i in 0..item_count {
        let game_index = home_game_indices.get(i).copied();
        let game = game_index.and_then(|index| games.get(index));
        let offset_f = i as f32 - scroll_offset;
        let is_selected = i == selected;
        let x_pos = item_left_for_offset(anchor_x, column_spacing, selected_icon_extra, offset_f);

        // Cheap horizontal culling: skip items whose icon slot lies entirely
        // outside the panel. Selected always renders (header/title extend
        // beyond the icon and must be present for the meta animation).
        if !is_selected
            && (x_pos + selected_icon_size < panel_rect.min.x || x_pos > panel_rect.max.x)
        {
            continue;
        }

        let dist = offset_f.abs();
        let icon_focus_t = (1.0 - dist).clamp(0.0, 1.0);
        let selection_t = if is_selected { selected_focus_t } else { 0.0 };
        let meta_t = if is_selected { selected_meta_t } else { 0.0 };
        let launch_elapsed_seconds = launch_feedback
            .filter(|(launch_index, _)| game_index == Some(*launch_index))
            .map(|(_, elapsed_seconds)| elapsed_seconds);
        let is_running = game_index.is_some_and(|index| running_indices.contains(&index));
        let status_dot = if is_running {
            Some(GameStatusDot::Running)
        } else if launching_index.is_some() && game_index == launching_index {
            Some(GameStatusDot::Launching)
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
        let icon_offset_y = launch_elapsed_seconds
            .map(|elapsed_seconds| launch_icon_offset_y(elapsed_seconds, home_layout_scale))
            .unwrap_or(0.0);

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

        draw_game_icon_focus_frame(
            &painter,
            icon_rect,
            home_layout_scale,
            border_focus_t,
            wake_t,
        );

        let entry_icon = game
            .and_then(|g| game_icons.get(&g.icon_key()))
            .or(shelf_icon.filter(|_| game.is_none()));

        if let Some(icon_tex) = entry_icon {
            if game.is_none() {
                draw_shelf_entry(
                    &painter,
                    icon_tex,
                    icon_rect,
                    home_layout_scale,
                    icon_focus_t,
                    wake_t,
                );
            } else {
                let icon_tint = color_with_scaled_alpha(egui::Color32::WHITE, wake_t);
                draw_game_icon(&painter, icon_tex, icon_rect, home_layout_scale, icon_tint);
            }

            if let Some(status_dot) = status_dot.filter(|_| wake_t > 0.12) {
                draw_status_dot(&painter, icon_rect, home_layout_scale, status_dot);
            }

            let mut overlay_drawn = false;
            if let Some((
                notice_index,
                notice_text,
                notice_overlay_t,
                notice_color,
                use_action_icon,
            )) = &launch_notice
            {
                if game_index == Some(*notice_index) {
                    draw_game_icon_overlay(
                        ui,
                        &painter,
                        icon_rect,
                        home_layout_scale,
                        notice_text,
                        action_icon_a,
                        *use_action_icon,
                        *notice_overlay_t,
                        *notice_color,
                        wake_t,
                    );
                    overlay_drawn = true;
                }
            }

            if !overlay_drawn {
                if let Some((notice_index, notice_text, notice_color)) = &steam_update_notice {
                    if game_index == Some(*notice_index) {
                        draw_game_icon_overlay(
                            ui,
                            &painter,
                            icon_rect,
                            home_layout_scale,
                            notice_text,
                            None,
                            false,
                            1.0,
                            *notice_color,
                            wake_t,
                        );
                    }
                }
            }
        }

        if is_selected {
            let title_x = if i + 1 < item_count {
                item_left_for_offset(
                    anchor_x,
                    column_spacing,
                    selected_icon_extra,
                    (i + 1) as f32 - scroll_offset,
                )
            } else {
                icon_slot_rect.max.x + design_units(18.0, home_layout_scale)
            };
            let title_text = game
                .map(|g| g.name.as_str())
                .unwrap_or_else(|| language.game_library_text());
            let header_width = (icon_slot_size * 2.0 + design_units(28.0, home_layout_scale))
                .max(design_units(320.0, home_layout_scale));
            let title_galley = painter.layout_no_wrap(
                title_text.to_owned(),
                font_id.clone(),
                color_with_scaled_alpha(text_color, wake_t),
            );
            let title_y = icon_slot_rect.max.y - title_galley.size().y;
            let badge_pos = egui::pos2(title_x, title_y);
            let summary_pos = egui::pos2(
                icon_slot_rect.min.x,
                icon_slot_rect.max.y + design_units(36.0, home_layout_scale),
            );
            let playtime_width = icon_slot_rect.width();
            let achievement_x = badge_pos.x;
            let achievement_width =
                ((padded_rect.max.x - achievement_x - design_units(24.0, home_layout_scale))
                    .min(design_units(292.0, home_layout_scale)))
                .max(design_units(220.0, home_layout_scale));
            let summary_style = SelectedGameSummaryStyle {
                card_height: design_units(106.0, home_layout_scale),
                layout_scale: home_layout_scale,
                ..SelectedGameSummaryStyle::default()
            };
            let badge_offset = game
                .map(|g| {
                    draw_selected_game_badge(
                        &painter,
                        g,
                        badge_pos,
                        title_galley.size(),
                        home_layout_scale,
                        wake_t,
                    )
                })
                .unwrap_or(0.0);
            let title_pos = egui::pos2(title_x + badge_offset, title_y);

            if let Some(g) = game {
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
                    design_units(17.0, home_layout_scale),
                    140.0 * meta_t,
                    header_width,
                );
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
            } else {
                painter.galley(title_pos, title_galley, egui::Color32::WHITE);
            }
        }
    }
}
