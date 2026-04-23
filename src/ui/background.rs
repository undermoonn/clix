use eframe::egui;

use super::anim::{lerp_f32, smoothstep01};
use super::text::{
    color_with_scaled_alpha, corner_radius, draw_main_clock, layout_main_clock, main_clock_color,
    scale_alpha,
};

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
    let vignette_rect = egui::Rect::from_min_max(
        egui::pos2(hero_rect.min.x - hero_rect.width() * 0.12, hero_rect.min.y),
        hero_rect.max,
    );
    painter.image(texture.id(), vignette_rect, uv, tint);
}

pub fn draw_background(
    ctx: &egui::Context,
    vignette: Option<&egui::TextureHandle>,
    show_clock: bool,
    settings_icon: Option<&egui::TextureHandle>,
    power_icon: Option<&egui::TextureHandle>,
    show_settings_button: bool,
    settings_button_focus_anim: f32,
    power_button_visibility_anim: f32,
    power_button_focus_anim: f32,
    power_button_above_mask: bool,
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

        let clock_galley = layout_main_clock(&bg_painter, wake_t);
        let margin_x = clock_anchor_rect.width() * 0.042;
        let margin_y = hero_rect.height() * 0.075;
        let clock_pos = egui::pos2(
            clock_anchor_rect.max.x - margin_x - clock_galley.size().x,
            hero_rect.min.y + margin_y,
        );

        if show_settings_button {
            if let Some(texture) = settings_icon {
                let focus_t = smoothstep01(settings_button_focus_anim);
                let power_t = smoothstep01(power_button_visibility_anim);
                let power_focus_t = smoothstep01(power_button_focus_anim);
                let icon_size = clock_galley.size().y * 0.63;
                let icon_offset_x = 56.0;
                let icon_pos = egui::pos2(
                    clock_pos.x - icon_size - icon_offset_x,
                    clock_pos.y + (clock_galley.size().y - icon_size) * 0.5,
                );
                let icon_rect = egui::Rect::from_min_size(icon_pos, egui::vec2(icon_size, icon_size));
                let power_gap = 54.0;

                if power_t > 0.001 {
                    if let Some(power_icon) = power_icon {
                        let power_painter = if power_button_above_mask {
                            ctx.layer_painter(egui::LayerId::new(
                                egui::Order::Foreground,
                                egui::Id::new("home_power_trigger_icon"),
                            ))
                        } else {
                            bg_painter.clone()
                        };
                        let power_icon_size = icon_size * 1.18;
                        let power_y = icon_rect.center().y - power_icon_size * 0.5;
                        let power_rect = egui::Rect::from_min_size(
                            egui::pos2(icon_rect.min.x - power_icon_size - power_gap, power_y),
                            egui::vec2(power_icon_size, power_icon_size),
                        );
                        if power_focus_t > 0.001 {
                            let highlight_radius = power_icon_size * 0.56;
                            let highlight_center = power_rect.center();
                            let highlight_rect = egui::Rect::from_center_size(
                                highlight_center,
                                egui::vec2(highlight_radius * 2.0, highlight_radius * 2.0),
                            );
                            let fill_clip_top = egui::lerp(
                                highlight_rect.bottom()..=highlight_rect.top(),
                                power_focus_t,
                            );
                            let fill_clip_rect = egui::Rect::from_min_max(
                                egui::pos2(highlight_rect.left(), fill_clip_top),
                                egui::pos2(highlight_rect.right(), highlight_rect.bottom()),
                            );
                            let fill_painter = power_painter.with_clip_rect(fill_clip_rect);
                            fill_painter.circle_filled(
                                highlight_center,
                                highlight_radius,
                                color_with_scaled_alpha(
                                    egui::Color32::from_rgba_unmultiplied(248, 250, 255, 58),
                                    wake_t * power_focus_t,
                                ),
                            );
                            power_painter.circle_stroke(
                                highlight_center,
                                highlight_radius,
                                egui::Stroke::new(
                                    lerp_f32(1.0, 2.8, power_focus_t),
                                    color_with_scaled_alpha(
                                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 164),
                                        wake_t * power_focus_t,
                                    ),
                                ),
                            );
                        }
                        power_painter.image(
                            power_icon.id(),
                            power_rect,
                            uv,
                            color_with_scaled_alpha(main_clock_color(wake_t), power_t),
                        );
                    }
                }

                if focus_t > 0.001 {
                    let highlight_radius = icon_size * 0.58;
                    let highlight_center = icon_rect.center();
                    let highlight_rect = egui::Rect::from_center_size(
                        highlight_center,
                        egui::vec2(highlight_radius * 2.0, highlight_radius * 2.0),
                    );
                    let fill_clip_top = egui::lerp(
                        highlight_rect.bottom()..=highlight_rect.top(),
                        focus_t,
                    );
                    let fill_clip_rect = egui::Rect::from_min_max(
                        egui::pos2(highlight_rect.left(), fill_clip_top),
                        egui::pos2(highlight_rect.right(), highlight_rect.bottom()),
                    );
                    let fill_painter = bg_painter.with_clip_rect(fill_clip_rect);
                    fill_painter.circle_filled(
                        highlight_center,
                        highlight_radius,
                        color_with_scaled_alpha(
                            egui::Color32::from_rgba_unmultiplied(248, 250, 255, 58),
                            wake_t * focus_t,
                        ),
                    );
                    bg_painter.circle_stroke(
                        highlight_center,
                        highlight_radius,
                        egui::Stroke::new(
                            lerp_f32(1.0, 2.8, focus_t),
                            color_with_scaled_alpha(
                                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 164),
                                wake_t * focus_t,
                            ),
                        ),
                    );
                }

                bg_painter.image(
                    texture.id(),
                    icon_rect,
                    uv,
                    main_clock_color(wake_t),
                );
            }
        }

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
