use std::sync::Arc;

use eframe::egui;

use crate::i18n::AppLanguage;

use super::hint_icons::HintIcons;
use super::text::{color_with_scaled_alpha, main_clock_right_edge};
use super::{design_units, lerp_f32, smoothstep01, viewport_layout_scale};

pub fn draw_hint_bar(
    ui: &mut egui::Ui,
    language: AppLanguage,
    icons: &HintIcons,
    achievement_panel_active: bool,
    power_menu_active: bool,
    game_menu_active: bool,
    game_library_active: bool,
    settings_active: bool,
    home_top_button_selected: bool,
    home_top_action_label: Option<&str>,
    settings_action_label: Option<&str>,
    can_open_achievement_panel: bool,
    can_open_game_menu: bool,
    achievement_refresh_loading: bool,
    wake_anim: f32,
) {
    let panel_rect = ui.available_rect_before_wrap();
    let layout_scale = viewport_layout_scale(panel_rect);
    let padding = design_units(50.0, layout_scale);
    let padded_rect = panel_rect.shrink(padding);
    let wake_t = smoothstep01(wake_anim);
    let hint_font = egui::FontId::proportional(design_units(24.0, layout_scale));
    let hint_color = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(236, 238, 244, 220),
        wake_t,
    );
    let action_icon_h = design_units(40.0, layout_scale);
    let group_gap = design_units(30.0, layout_scale);
    let row_h = action_icon_h;
    let hint_y = padded_rect.max.y - design_units(10.0, layout_scale)
        + lerp_f32(design_units(24.0, layout_scale), 0.0, wake_t);
    let clock_anchor_rect = egui::Rect::from_min_size(
        panel_rect.min,
        egui::vec2(panel_rect.width(), panel_rect.width() * (1240.0 / 3840.0)),
    );
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

    let painter = ui.painter();
    let draw_icon = |painter: &egui::Painter, tex: &egui::TextureHandle, x: f32, size: f32| {
        painter.image(
            tex.id(),
            egui::Rect::from_min_size(
                egui::pos2(x, hint_y + (row_h - size) * 0.5),
                egui::vec2(size, size),
            ),
            uv,
            color_with_scaled_alpha(egui::Color32::WHITE, wake_t),
        );
    };
    let label_y = |galley: &Arc<egui::Galley>| {
        hint_y + (row_h - galley.size().y) * 0.5 - design_units(2.0, layout_scale)
    };
    let draw_loading_ring = |painter: &egui::Painter, center: egui::Pos2, radius: f32| {
        let time = painter.ctx().input(|input| input.time) as f32;
        let sweep = std::f32::consts::TAU * 0.26;
        let rotation = time * 4.8;
        let start_angle = rotation - std::f32::consts::FRAC_PI_2;
        let end_angle = start_angle + sweep;
        let bg_stroke = egui::Stroke::new(
            design_units(1.8, layout_scale),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 36),
                wake_t,
            ),
        );
        let fg_stroke = egui::Stroke::new(
            design_units(2.4, layout_scale),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220),
                wake_t,
            ),
        );

        painter.circle_stroke(center, radius, bg_stroke);

        let segments = 24;
        let mut points = Vec::with_capacity(segments + 1);
        for index in 0..=segments {
            let t = index as f32 / segments as f32;
            let angle = start_angle + (end_angle - start_angle) * t;
            points.push(center + egui::vec2(angle.cos() * radius, angle.sin() * radius));
        }

        painter.add(egui::Shape::line(points, fg_stroke));
        painter.ctx().request_repaint();
    };

    let draw_labeled_icon_group = |tex: &egui::TextureHandle, x: f32, label: &Arc<egui::Galley>| {
        draw_icon(painter, tex, x, action_icon_h);
        painter.galley(
            egui::pos2(
                x + action_icon_h + design_units(6.0, layout_scale),
                label_y(label),
            ),
            label.clone(),
            egui::Color32::WHITE,
        );
    };
    let group_width = |galley: &Arc<egui::Galley>| {
        action_icon_h + design_units(6.0, layout_scale) + galley.size().x
    };
    let mut next_group_x = main_clock_right_edge(clock_anchor_rect);
    let mut reserve_group = |galley: &Arc<egui::Galley>| {
        let x = next_group_x - group_width(galley);
        next_group_x = x - group_gap;
        x
    };

    let g_back = painter.layout_no_wrap(
        language.back_text().to_string(),
        hint_font.clone(),
        hint_color,
    );
    let g_menu = painter.layout_no_wrap(
        language.menu_text().to_string(),
        hint_font.clone(),
        hint_color,
    );

    let reserve_menu_hint = |reserve_group: &mut dyn FnMut(&Arc<egui::Galley>) -> f32| {
        if can_open_game_menu {
            let menu_x = reserve_group(&g_menu);
            draw_labeled_icon_group(&icons.btn_menu, menu_x, &g_menu);
        }
    };

    if settings_active {
        if let Some(action_label) = settings_action_label {
            let g_action =
                painter.layout_no_wrap(action_label.to_string(), hint_font.clone(), hint_color);
            let action_x = reserve_group(&g_action);
            draw_labeled_icon_group(&icons.btn_a, action_x, &g_action);
        }

        let back_x = reserve_group(&g_back);
        draw_labeled_icon_group(&icons.btn_b, back_x, &g_back);

        return;
    }

    if power_menu_active || game_menu_active {
        let g_confirm = painter.layout_no_wrap(
            language.confirm_text().to_string(),
            hint_font.clone(),
            hint_color,
        );

        let confirm_x = reserve_group(&g_confirm);
        draw_labeled_icon_group(&icons.btn_a, confirm_x, &g_confirm);

        let back_x = reserve_group(&g_back);
        draw_labeled_icon_group(&icons.btn_b, back_x, &g_back);

        return;
    }

    if achievement_panel_active {
        let g_refresh = painter.layout_no_wrap(
            language.refresh_text().to_string(),
            hint_font.clone(),
            hint_color,
        );

        let refresh_x = reserve_group(&g_refresh);
        draw_icon(painter, &icons.btn_x, refresh_x, action_icon_h);
        if achievement_refresh_loading {
            draw_loading_ring(
                painter,
                egui::pos2(refresh_x + action_icon_h * 0.5, hint_y + row_h * 0.5),
                action_icon_h * 0.49,
            );
        }
        painter.galley(
            egui::pos2(
                refresh_x + action_icon_h + design_units(6.0, layout_scale),
                label_y(&g_refresh),
            ),
            g_refresh,
            egui::Color32::WHITE,
        );

        let back_x = reserve_group(&g_back);
        draw_labeled_icon_group(&icons.btn_b, back_x, &g_back);

        return;
    }

    if game_library_active {
        let g_launch = painter.layout_no_wrap(
            language.start_text().to_string(),
            hint_font.clone(),
            hint_color,
        );
        let launch_x = reserve_group(&g_launch);
        draw_labeled_icon_group(&icons.btn_a, launch_x, &g_launch);

        let back_x = reserve_group(&g_back);
        draw_labeled_icon_group(&icons.btn_b, back_x, &g_back);

        reserve_menu_hint(&mut reserve_group);

        return;
    }

    if home_top_button_selected {
        let top_action_label = home_top_action_label.unwrap_or_else(|| language.settings_text());
        let g_top_action =
            painter.layout_no_wrap(top_action_label.to_string(), hint_font.clone(), hint_color);
        let settings_x = reserve_group(&g_top_action);
        draw_labeled_icon_group(&icons.btn_a, settings_x, &g_top_action);

        let back_x = reserve_group(&g_back);
        draw_labeled_icon_group(&icons.btn_b, back_x, &g_back);

        return;
    }

    let g_launch = painter.layout_no_wrap(
        language.start_text().to_string(),
        hint_font.clone(),
        hint_color,
    );

    let launch_x = reserve_group(&g_launch);
    draw_labeled_icon_group(&icons.btn_a, launch_x, &g_launch);

    reserve_menu_hint(&mut reserve_group);

    let _ = can_open_achievement_panel;
}
