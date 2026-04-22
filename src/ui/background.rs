use eframe::egui;

use super::anim::{lerp_f32, smoothstep01};
use super::text::{corner_radius, draw_main_clock, scale_alpha};

fn draw_top_right_vignette(
    painter: &egui::Painter,
    hero_rect: egui::Rect,
    texture: Option<&egui::TextureHandle>,
    alpha_scale: f32,
) {
    let Some(texture) = texture else {
        return;
    };
    if alpha_scale <= 0.001 {
        return;
    };

    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let tint = egui::Color32::from_rgba_unmultiplied(
        255,
        255,
        255,
        scale_alpha(255, alpha_scale),
    );
    painter.image(texture.id(), hero_rect, uv, tint);
}

pub fn draw_background(
    ctx: &egui::Context,
    vignette: Option<&egui::TextureHandle>,
    show_clock: bool,
    cover: &Option<(u32, egui::TextureHandle)>,
    cover_prev: &Option<(u32, egui::TextureHandle)>,
    logo: &Option<(u32, egui::TextureHandle)>,
    logo_prev: &Option<(u32, egui::TextureHandle)>,
    cover_fade: f32,
    cover_nav_dir: f32,
    achievement_panel_anim: f32,
    wake_anim: f32,
) {
    let screen = ctx.content_rect();
    let bg_painter = ctx.layer_painter(egui::LayerId::background());
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let base_alpha: f32 = 170.0;
    let hero_ratio = 1240.0 / 3840.0;
    let page_scroll_t = smoothstep01(achievement_panel_anim);
    let wake_t = smoothstep01(wake_anim);
    let wake_inv = 1.0 - wake_t;
    let page_offset_y = -screen.height() * page_scroll_t;
    let wake_expand = egui::vec2(screen.width() * 0.02 * wake_inv, 22.0 * wake_inv);
    let wake_shift_y = -16.0 * wake_inv;
    let wake_alpha_scale = lerp_f32(0.72, 1.0, wake_t);

    bg_painter.rect_filled(screen, egui::CornerRadius::ZERO, egui::Color32::from_rgb(18, 18, 18));

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
    let previous_hero_rect = if cover_fade < 1.0 {
        cover_prev.as_ref().map(|(_id, tex)| top_rect(tex, 0.0))
    } else {
        None
    };
    let current_dx = cover_nav_dir * slide_distance * (1.0 - ease_t);
    let current_hero_rect = if let Some((_id, tex)) = cover {
        top_rect(tex, current_dx)
    } else {
        fallback_hero_rect(current_dx)
    };
    let current_vignette_alpha = if cover.is_some() { cover_fade } else { 1.0 };
    let clock_anchor_rect = fallback_hero_rect(0.0);
    let draw_clock = |hero_rect: egui::Rect| {
        if !show_clock {
            return;
        }

        let clock_font = egui::FontId::new(40.0, egui::FontFamily::Name("Bold".into()));
        let clock_galley = bg_painter.layout_no_wrap(
            chrono::Local::now().format("%H:%M").to_string(),
            clock_font,
            egui::Color32::WHITE,
        );
        let margin_x = clock_anchor_rect.width() * 0.042;
        let margin_y = hero_rect.height() * 0.075;
        let clock_pos = egui::pos2(
            clock_anchor_rect.max.x - margin_x - clock_galley.size().x,
            hero_rect.min.y + margin_y,
        );

        draw_main_clock(&bg_painter, clock_pos, wake_t);
    };

    if cover_fade < 1.0 {
        if let Some((_id, tex)) = cover_prev {
            let alpha = (base_alpha * (1.0 - cover_fade) * wake_alpha_scale) as u8;
            let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
            bg_painter.image(tex.id(), previous_hero_rect.unwrap(), uv, tint);
        }
    }

    if let Some((_id, tex)) = cover {
        let alpha = (base_alpha * cover_fade * wake_alpha_scale) as u8;
        let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
        bg_painter.image(tex.id(), current_hero_rect, uv, tint);
    }

    if let Some(hero_rect) = previous_hero_rect {
        draw_top_right_vignette(&bg_painter, hero_rect, vignette, 1.0 - cover_fade);
    }

    draw_top_right_vignette(&bg_painter, current_hero_rect, vignette, current_vignette_alpha);

    if cover_fade < 1.0 {
        if let Some((_id, tex)) = logo_prev {
            let hero_rect = previous_hero_rect.unwrap();
            draw_logo(tex, hero_rect, 1.0 - cover_fade);
        }
    }

    if let Some((_id, tex)) = logo {
        draw_logo(tex, current_hero_rect, cover_fade);
    }

    draw_clock(current_hero_rect);

    let wake_overlay_alpha = scale_alpha(120, wake_inv);
    if wake_overlay_alpha > 0 {
        bg_painter.rect_filled(
            screen,
            corner_radius(0.0),
            egui::Color32::from_rgba_unmultiplied(8, 10, 14, wake_overlay_alpha),
        );
    }

}
