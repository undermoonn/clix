use eframe::egui;

use super::anim::{lerp_f32, smoothstep01};
use super::text::scale_alpha;

pub fn draw_background(
    ctx: &egui::Context,
    cover: &Option<(u32, egui::TextureHandle)>,
    cover_prev: &Option<(u32, egui::TextureHandle)>,
    logo: &Option<(u32, egui::TextureHandle)>,
    logo_prev: &Option<(u32, egui::TextureHandle)>,
    cover_fade: f32,
    cover_nav_dir: f32,
    achievement_panel_anim: f32,
    wake_anim: f32,
) {
    let screen = ctx.screen_rect();
    let bg_painter = ctx.layer_painter(egui::LayerId::background());
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let base_alpha: f32 = 60.0;
    let hero_ratio = 1240.0 / 3840.0;
    let page_scroll_t = smoothstep01(achievement_panel_anim);
    let wake_t = smoothstep01(wake_anim);
    let wake_inv = 1.0 - wake_t;
    let page_offset_y = -screen.height() * page_scroll_t;
    let wake_expand = egui::vec2(screen.width() * 0.02 * wake_inv, 22.0 * wake_inv);
    let wake_shift_y = -16.0 * wake_inv;
    let wake_alpha_scale = lerp_f32(0.72, 1.0, wake_t);

    bg_painter.rect_filled(screen, egui::Rounding::ZERO, egui::Color32::from_rgb(18, 18, 18));

    let top_rect = |tex: &egui::TextureHandle, dx: f32| -> egui::Rect {
        let tex_size = tex.size_vec2();
        let scale = screen.width() / tex_size.x;
        let img_h = tex_size.y * scale;
        egui::Rect::from_min_size(
            egui::pos2(screen.min.x + dx, screen.min.y + page_offset_y),
            egui::vec2(screen.width(), img_h),
        )
        .expand2(wake_expand)
        .translate(egui::vec2(0.0, wake_shift_y))
    };

    let fallback_hero_rect = |dx: f32| -> egui::Rect {
        egui::Rect::from_min_size(
            egui::pos2(screen.min.x + dx, screen.min.y + page_offset_y),
            egui::vec2(screen.width(), screen.width() * hero_ratio),
        )
        .expand2(wake_expand)
        .translate(egui::vec2(0.0, wake_shift_y))
    };

    let draw_logo = |texture: &egui::TextureHandle, hero_rect: egui::Rect, alpha_scale: f32| {
        let tex_size = texture.size_vec2();
        if tex_size.x <= 0.0 || tex_size.y <= 0.0 {
            return;
        }

        let draw_size = tex_size;
        let margin_x = hero_rect.width() * 0.038;
        let margin_bottom = hero_rect.height() * 0.085;
        let logo_rect = egui::Rect::from_min_size(
            egui::pos2(
                hero_rect.min.x + margin_x,
                hero_rect.max.y - margin_bottom - draw_size.y,
            ),
            draw_size,
        );
        let logo_tint = egui::Color32::from_rgba_unmultiplied(
            255,
            255,
            255,
            (255.0 * alpha_scale).round() as u8,
        );

        bg_painter.image(texture.id(), logo_rect, uv, logo_tint);
    };

    let slide_distance = 18.0;
    let ease_t = 1.0 - (1.0 - cover_fade) * (1.0 - cover_fade);

    if cover_fade < 1.0 {
        if let Some((_id, tex)) = cover_prev {
            let hero_rect = top_rect(tex, 0.0);
            let alpha = (base_alpha * (1.0 - cover_fade) * wake_alpha_scale) as u8;
            let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
            bg_painter.image(tex.id(), hero_rect, uv, tint);
        }
    }

    let current_dx = cover_nav_dir * slide_distance * (1.0 - ease_t);
    if let Some((_id, tex)) = cover {
        let hero_rect = top_rect(tex, current_dx);
        let alpha = (base_alpha * cover_fade * wake_alpha_scale) as u8;
        let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
        bg_painter.image(tex.id(), hero_rect, uv, tint);
    }

    if cover_fade < 1.0 {
        if let Some((_id, tex)) = logo_prev {
            let hero_rect = if let Some((_cover_id, cover_tex)) = cover_prev {
                top_rect(cover_tex, 0.0)
            } else {
                fallback_hero_rect(0.0)
            };
            draw_logo(tex, hero_rect, 1.0 - cover_fade);
        }
    }

    if let Some((_id, tex)) = logo {
        let hero_rect = if let Some((_cover_id, cover_tex)) = cover {
            top_rect(cover_tex, current_dx)
        } else {
            fallback_hero_rect(current_dx)
        };
        draw_logo(tex, hero_rect, cover_fade);
    }

    let wake_overlay_alpha = scale_alpha(120, wake_inv);
    if wake_overlay_alpha > 0 {
        bg_painter.rect_filled(
            screen,
            egui::Rounding::ZERO,
            egui::Color32::from_rgba_unmultiplied(8, 10, 14, wake_overlay_alpha),
        );
    }
}
