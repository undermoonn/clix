use std::collections::{HashMap, HashSet};

use eframe::egui;

use crate::game::{Game, GameIconKey};
use crate::i18n::AppLanguage;

use super::{
    color_with_scaled_alpha, corner_radius, design_units, lerp_f32, smoothstep01,
    viewport_layout_scale,
};

const GAME_LIBRARY_COLUMNS: usize = 7;
const GAME_LIBRARY_BACKDROP_PHASE: f32 = 0.18;

fn staged_entry_progress(anim: f32) -> (f32, f32) {
    let anim = anim.clamp(0.0, 1.0);
    let backdrop_t = smoothstep01((anim / GAME_LIBRARY_BACKDROP_PHASE).clamp(0.0, 1.0));
    let content_t = ((anim - GAME_LIBRARY_BACKDROP_PHASE) / (1.0 - GAME_LIBRARY_BACKDROP_PHASE))
        .clamp(0.0, 1.0);
    (backdrop_t, smoothstep01(content_t))
}

fn draw_cover_icon(
    painter: &egui::Painter,
    texture: &egui::TextureHandle,
    rect: egui::Rect,
    layout_scale: f32,
    tint: egui::Color32,
) {
    painter.add(egui::Shape::Rect(
        egui::epaint::RectShape::filled(rect, corner_radius(design_units(8.0, layout_scale)), tint)
            .with_texture(
                texture.id(),
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            ),
    ));
}

fn draw_focus_frame(
    painter: &egui::Painter,
    rect: egui::Rect,
    layout_scale: f32,
    focus_t: f32,
    layer_t: f32,
) {
    if focus_t <= 0.001 {
        return;
    }

    let focus_rect = rect.expand(design_units(5.0, layout_scale));
    painter.rect_filled(
        focus_rect,
        corner_radius(design_units(12.0, layout_scale)),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(248, 250, 255, 54),
            layer_t * focus_t,
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
                layer_t * focus_t,
            ),
        ),
        egui::StrokeKind::Outside,
    );
}

fn draw_hidden_home_badge(
    painter: &egui::Painter,
    icon: &egui::TextureHandle,
    rect: egui::Rect,
    layout_scale: f32,
    layer_t: f32,
) {
    if layer_t <= 0.001 {
        return;
    }

    let badge_size = design_units(44.0, layout_scale);
    let badge_padding = design_units(8.0, layout_scale);
    let badge_rect = egui::Rect::from_min_size(
        rect.min + egui::vec2(badge_padding, badge_padding),
        egui::vec2(badge_size, badge_size),
    );
    painter.rect_filled(
        badge_rect,
        corner_radius(design_units(8.0, layout_scale)),
        color_with_scaled_alpha(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 172), layer_t),
    );
    painter.rect_stroke(
        badge_rect,
        corner_radius(design_units(8.0, layout_scale)),
        egui::Stroke::new(
            design_units(1.0, layout_scale),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 44),
                layer_t,
            ),
        ),
        egui::StrokeKind::Middle,
    );

    let icon_size = design_units(27.0, layout_scale);
    let icon_rect =
        egui::Rect::from_center_size(badge_rect.center(), egui::vec2(icon_size, icon_size));
    painter.image(
        icon.id(),
        icon_rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(248, 250, 255, 255),
            layer_t,
        ),
    );
}

pub fn draw_game_library_page(
    ui: &mut egui::Ui,
    language: AppLanguage,
    games: &[Game],
    selected: usize,
    page_anim: f32,
    select_anim: f32,
    scroll_offset: f32,
    background: Option<&egui::TextureHandle>,
    game_icons: &HashMap<GameIconKey, egui::TextureHandle>,
    action_icon_a: Option<&egui::TextureHandle>,
    launch_feedback: Option<(usize, f32)>,
    launch_notice: Option<(usize, String, f32, egui::Color32, bool)>,
    steam_update_notice: Option<(usize, String, egui::Color32)>,
    launching_index: Option<usize>,
    running_indices: &[usize],
    hidden_home_game_keys: &HashSet<String>,
    hide_icon: Option<&egui::TextureHandle>,
    wake_anim: f32,
) {
    let (backdrop_t, content_t) = staged_entry_progress(page_anim);
    if backdrop_t <= 0.001 {
        return;
    }

    let panel_rect = ui.available_rect_before_wrap();
    let layout_scale = viewport_layout_scale(panel_rect);
    let su = |value: f32| design_units(value, layout_scale);
    let painter = ui.painter();
    let wake_t = smoothstep01(wake_anim);
    let layer_t = content_t * wake_t;

    painter.rect_filled(
        panel_rect,
        egui::CornerRadius::ZERO,
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(18, 18, 18, 255),
            backdrop_t,
        ),
    );
    if let Some(texture) = background {
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        let tex_size = texture.size_vec2();
        let scale = (panel_rect.width() / tex_size.x).max(panel_rect.height() / tex_size.y);
        let image_size = tex_size * scale;
        let image_rect = egui::Rect::from_center_size(panel_rect.center(), image_size);
        painter.image(
            texture.id(),
            image_rect,
            uv,
            color_with_scaled_alpha(egui::Color32::WHITE, backdrop_t),
        );
    }
    let page_rect = egui::Rect::from_min_max(
        panel_rect.min + egui::vec2(su(24.0), su(54.0)),
        panel_rect.max - egui::vec2(su(24.0), su(80.0)),
    )
    .translate(egui::vec2(0.0, lerp_f32(su(18.0), 0.0, content_t)));
    let icon_size = su(224.0);
    let focus_margin = su(8.0);
    let item_gap = su(30.0);
    let grid_top = page_rect.min.y + su(92.0);
    let viewport_rect =
        egui::Rect::from_min_max(egui::pos2(page_rect.min.x, grid_top), page_rect.max);
    let grid_rect = viewport_rect.shrink(focus_margin);
    let grid_width = GAME_LIBRARY_COLUMNS as f32 * icon_size
        + (GAME_LIBRARY_COLUMNS.saturating_sub(1)) as f32 * item_gap;
    let grid_left = grid_rect.min.x + ((grid_rect.width() - grid_width) * 0.5).max(0.0);
    let title_font = egui::FontId::new(su(42.0), egui::FontFamily::Name("Bold".into()));
    let title_galley = painter.layout_no_wrap(
        language.game_library_text().to_owned(),
        title_font,
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(248, 250, 255, 255),
            layer_t,
        ),
    );
    painter.galley(
        egui::pos2(grid_left, page_rect.min.y),
        title_galley,
        egui::Color32::WHITE,
    );
    let row_stride = icon_size + item_gap;
    let selected_row = selected / GAME_LIBRARY_COLUMNS;
    let max_visible_rows = ((grid_rect.height() / row_stride).floor() as usize).max(1);
    let max_scroll_row = games.len().saturating_sub(1) / GAME_LIBRARY_COLUMNS;
    let target_row = scroll_offset.clamp(
        0.0,
        max_scroll_row.saturating_sub(max_visible_rows.saturating_sub(1)) as f32,
    );
    let scroll_y = target_row * row_stride;
    let grid_painter = painter.with_clip_rect(viewport_rect);
    let focus_t = smoothstep01(select_anim);

    if launch_feedback.is_some() {
        ui.ctx().request_repaint();
    }

    for (index, game) in games.iter().enumerate() {
        let row = index / GAME_LIBRARY_COLUMNS;
        let col = index % GAME_LIBRARY_COLUMNS;
        let top = grid_rect.min.y + row as f32 * row_stride - scroll_y;
        let rect = egui::Rect::from_min_size(
            egui::pos2(grid_left + col as f32 * (icon_size + item_gap), top),
            egui::vec2(icon_size, icon_size),
        );
        if rect.max.y < viewport_rect.min.y || rect.min.y > viewport_rect.max.y {
            continue;
        }

        let is_selected = index == selected;
        let launch_elapsed_seconds = launch_feedback
            .filter(|(launch_index, _)| index == *launch_index)
            .map(|(_, elapsed_seconds)| elapsed_seconds);
        let icon_scale = launch_elapsed_seconds
            .map(super::game_list::launch_icon_scale)
            .unwrap_or(1.0);
        let scaled_icon_size = icon_size * icon_scale;
        let icon_offset_y = launch_elapsed_seconds
            .map(|elapsed_seconds| {
                super::game_list::launch_icon_offset_y(elapsed_seconds, layout_scale)
            })
            .unwrap_or(0.0);
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.min.x + (rect.width() - scaled_icon_size) * 0.5,
                rect.min.y + (rect.height() - scaled_icon_size) + icon_offset_y,
            ),
            egui::vec2(scaled_icon_size, scaled_icon_size),
        );
        let status_dot = if running_indices.contains(&index) {
            Some(super::game_list::GameStatusDot::Running)
        } else if launching_index == Some(index) {
            Some(super::game_list::GameStatusDot::Launching)
        } else {
            None
        };

        draw_focus_frame(
            &grid_painter,
            icon_rect,
            layout_scale,
            if is_selected { focus_t } else { 0.0 },
            layer_t,
        );

        if let Some(icon_tex) = game_icons.get(&game.icon_key()) {
            draw_cover_icon(
                &grid_painter,
                icon_tex,
                icon_rect,
                layout_scale,
                color_with_scaled_alpha(egui::Color32::WHITE, layer_t),
            );
        }

        if hidden_home_game_keys.contains(&game.persistent_key()) {
            if let Some(hide_icon) = hide_icon {
                draw_hidden_home_badge(&grid_painter, hide_icon, icon_rect, layout_scale, layer_t);
            }
        }

        if let Some(status_dot) = status_dot.filter(|_| layer_t > 0.12) {
            super::game_list::draw_status_dot(&grid_painter, icon_rect, layout_scale, status_dot);
        }

        if is_selected {
            let mut overlay_drawn = false;
            if let Some((
                notice_index,
                notice_text,
                notice_overlay_t,
                notice_color,
                use_action_icon,
            )) = &launch_notice
            {
                if index == *notice_index {
                    super::game_list::draw_game_icon_overlay(
                        ui,
                        &grid_painter,
                        icon_rect,
                        layout_scale,
                        notice_text,
                        action_icon_a,
                        *use_action_icon,
                        *notice_overlay_t,
                        *notice_color,
                        layer_t,
                    );
                    overlay_drawn = true;
                }
            }

            if !overlay_drawn {
                if let Some((notice_index, notice_text, notice_color)) = &steam_update_notice {
                    if index == *notice_index {
                        super::game_list::draw_game_icon_overlay(
                            ui,
                            &grid_painter,
                            icon_rect,
                            layout_scale,
                            notice_text,
                            None,
                            false,
                            1.0,
                            *notice_color,
                            layer_t,
                        );
                        overlay_drawn = true;
                    }
                }
            }

            if !overlay_drawn {
                super::game_list::draw_game_icon_overlay(
                    ui,
                    &grid_painter,
                    icon_rect,
                    layout_scale,
                    &game.name,
                    None,
                    false,
                    focus_t,
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 154),
                    layer_t,
                );
            }
        }
    }

    if selected_row > max_visible_rows && content_t > 0.001 {
        ui.ctx().request_repaint();
    }
}
