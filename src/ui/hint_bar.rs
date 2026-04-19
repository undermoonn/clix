use std::sync::Arc;

use eframe::egui;

use crate::i18n::AppLanguage;

use super::hint_icons::HintIcons;
use super::anim::{lerp_f32, smoothstep01};
use super::text::{color_with_scaled_alpha, draw_main_clock};

pub fn draw_hint_bar(
    ui: &mut egui::Ui,
    language: AppLanguage,
    icons: &HintIcons,
    achievement_panel_active: bool,
    _home_menu_active: bool,
    can_open_achievement_panel: bool,
    achievement_refresh_loading: bool,
    game_running: bool,
    force_close_hold_progress: f32,
    wake_anim: f32,
) {
    let panel_rect = ui.available_rect_before_wrap();
    let padding = 50.0;
    let padded_rect = panel_rect.shrink(padding);
    let wake_t = smoothstep01(wake_anim);
    let hint_font = egui::FontId::proportional(20.0);
    let hint_color = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(200, 200, 210, 160),
        wake_t,
    );
    let action_icon_h = 40.0_f32;
    let row_h = action_icon_h;
    let hint_y = padded_rect.max.y - 10.0 + lerp_f32(24.0, 0.0, wake_t);
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

    let painter = ui.painter();
    let show_clock = true;
    let clock_gap = 34.0;
    let clock_font = egui::FontId::new(40.0, egui::FontFamily::Name("Bold".into()));
    let clock_galley = show_clock.then(|| {
        painter.layout_no_wrap(
            chrono::Local::now().format("%H:%M").to_string(),
            clock_font,
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(245, 247, 252, 168),
                wake_t,
            ),
        )
    });
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
    let draw_progress_ring = |painter: &egui::Painter,
                              center: egui::Pos2,
                              radius: f32,
                              progress: f32| {
        if progress <= 0.0 {
            return;
        }

        let clamped = progress.clamp(0.0, 1.0);
        let bg_stroke = egui::Stroke::new(
            2.0,
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 40),
                wake_t,
            ),
        );
        let fg_stroke = egui::Stroke::new(
            2.5,
            color_with_scaled_alpha(egui::Color32::from_rgb(255, 255, 255), wake_t),
        );
        painter.circle_stroke(center, radius, bg_stroke);

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
    let draw_loading_ring = |painter: &egui::Painter, center: egui::Pos2, radius: f32| {
        let time = painter.ctx().input(|input| input.time) as f32;
        let sweep = std::f32::consts::TAU * 0.26;
        let rotation = time * 4.8;
        let start_angle = rotation - std::f32::consts::FRAC_PI_2;
        let end_angle = start_angle + sweep;
        let bg_stroke = egui::Stroke::new(
            1.8,
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 36),
                wake_t,
            ),
        );
        let fg_stroke = egui::Stroke::new(
            2.4,
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

    let g_back = painter.layout_no_wrap(language.back_text().to_string(), hint_font.clone(), hint_color);
    let g_force_close = painter.layout_no_wrap(
        language.hold_close_game_text().to_string(),
        hint_font.clone(),
        hint_color,
    );
    let home_menu_group_w = action_icon_h;
    let clock_reserved_w = clock_galley
        .as_ref()
        .map(|galley| galley.size().x + clock_gap)
        .unwrap_or(0.0);
    let home_menu_x = padded_rect.max.x - clock_reserved_w - home_menu_group_w;
    let b_label_reserve = g_back.size().x;
    let b_icon_x = home_menu_x - 20.0 - b_label_reserve - 6.0 - action_icon_h;
    let b_label_x = b_icon_x + action_icon_h + 6.0;

    if let Some(clock_galley) = &clock_galley {
        let clock_pos = egui::pos2(
            home_menu_x + action_icon_h + clock_gap,
            hint_y + row_h * 0.5 - clock_galley.size().y * 0.5,
        );
        draw_main_clock(painter, clock_pos, wake_t);
    }

    if achievement_panel_active {
        let g_scroll =
            painter.layout_no_wrap(language.scroll_text().to_string(), hint_font.clone(), hint_color);
        let g_refresh =
            painter.layout_no_wrap(language.refresh_text().to_string(), hint_font.clone(), hint_color);

        let group_width = |galley: &Arc<egui::Galley>| action_icon_h + 6.0 + galley.size().x;
        let mut cursor_x = b_icon_x - 20.0;

        let refresh_x = cursor_x - group_width(&g_refresh);
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
                refresh_x + action_icon_h + 6.0,
                hint_y + (row_h - g_refresh.size().y) * 0.5,
            ),
            g_refresh,
            egui::Color32::WHITE,
        );
        cursor_x = refresh_x - 20.0;

        let scroll_x = cursor_x - group_width(&g_scroll);
        draw_icon(painter, &icons.dpad_down, scroll_x, action_icon_h);
        painter.galley(
            egui::pos2(
                scroll_x + action_icon_h + 6.0,
                hint_y + (row_h - g_scroll.size().y) * 0.5,
            ),
            g_scroll,
            egui::Color32::WHITE,
        );

        draw_icon(painter, &icons.btn_b, b_icon_x, action_icon_h);

        let gy = hint_y + (row_h - g_back.size().y) * 0.5;
        painter.galley(egui::pos2(b_label_x, gy), g_back, egui::Color32::WHITE);

        draw_icon(painter, &icons.guide, home_menu_x, action_icon_h);
        return;
    }

    let g_launch = painter.layout_no_wrap(language.start_text().to_string(), hint_font.clone(), hint_color);
    let g_achievements = painter.layout_no_wrap(
        language.achievements_text().to_string(),
        hint_font.clone(),
        hint_color,
    );
    let force_close_group_w = if game_running {
        action_icon_h + 6.0 + g_force_close.size().x
    } else {
        0.0
    };
    let launch_group_w = action_icon_h + 6.0 + g_launch.size().x;
    let launch_x = home_menu_x - 20.0 - launch_group_w;
    let force_close_x = if game_running {
        launch_x - 20.0 - force_close_group_w
    } else {
        launch_x
    };

    if can_open_achievement_panel {
        let achievements_group_w = action_icon_h + 6.0 + g_achievements.size().x;
        let achievements_x = force_close_x - 20.0 - achievements_group_w;
        draw_icon(painter, &icons.dpad_down, achievements_x, action_icon_h);

        let gy = hint_y + (row_h - g_achievements.size().y) * 0.5;
        painter.galley(
            egui::pos2(achievements_x + action_icon_h + 6.0, gy),
            g_achievements,
            egui::Color32::WHITE,
        );
    }

    if game_running {
        draw_icon(painter, &icons.btn_x, force_close_x, action_icon_h);
        draw_progress_ring(
            painter,
            egui::pos2(force_close_x + action_icon_h * 0.5, hint_y + row_h * 0.5),
            action_icon_h * 0.48,
            force_close_hold_progress,
        );

        let gy = hint_y + (row_h - g_force_close.size().y) * 0.5;
        painter.galley(
            egui::pos2(force_close_x + action_icon_h + 6.0, gy),
            g_force_close,
            egui::Color32::WHITE,
        );
    }

    draw_icon(painter, &icons.btn_a, launch_x, action_icon_h);

    let gy = hint_y + (row_h - g_launch.size().y) * 0.5;
    painter.galley(
        egui::pos2(launch_x + action_icon_h + 6.0, gy),
        g_launch,
        egui::Color32::WHITE,
    );

    draw_icon(painter, &icons.guide, home_menu_x, action_icon_h);
}
