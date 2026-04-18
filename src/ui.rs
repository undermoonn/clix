use eframe::egui;

use crate::i18n::AppLanguage;

fn format_achievement_status(unlocked: Option<bool>, unlock_time: Option<u64>) -> Option<String> {
    match unlocked {
        Some(true) => unlock_time
            .and_then(|value| i64::try_from(value).ok())
            .and_then(|timestamp| chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0))
            .map(|datetime| datetime.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string()),
        _ => None,
    }
}

fn build_wrapped_galley(
    ui: &egui::Ui,
    text: String,
    font: egui::FontId,
    color: egui::Color32,
    max_width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = max_width;
    job.append(
        &text,
        0.0,
        egui::TextFormat {
            font_id: font,
            color,
            ..Default::default()
        },
    );
    ui.ctx().fonts(|fonts| fonts.layout_job(job))
}

fn smoothstep01(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp_f32(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

fn scale_alpha(alpha: u8, scale: f32) -> u8 {
    ((alpha as f32) * scale.clamp(0.0, 1.0))
        .round()
        .clamp(0.0, 255.0) as u8
}

fn color_with_scaled_alpha(color: egui::Color32, scale: f32) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        scale_alpha(color.a(), scale),
    )
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

fn draw_game_icon(
    painter: &egui::Painter,
    texture: &egui::TextureHandle,
    icon_rect: egui::Rect,
    tint: egui::Color32,
) {
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    painter.add(egui::Shape::Rect(egui::epaint::RectShape {
        rect: icon_rect,
        rounding: egui::Rounding::same(8.0),
        fill: tint,
        stroke: egui::Stroke::NONE,
        fill_texture_id: texture.id(),
        uv,
    }));
}

fn draw_running_status_dot(painter: &egui::Painter, icon_rect: egui::Rect) {
    let radius = (icon_rect.width().min(icon_rect.height()) * 0.055).clamp(4.0, 7.0);
    let inset = (radius * 0.9).clamp(6.0, 10.0);
    let center = egui::pos2(icon_rect.max.x - inset - radius, icon_rect.min.y + inset + radius);
    let time = painter.ctx().input(|input| input.time) as f32;
    let pulse = smoothstep01((time * 3.1).sin() * 0.5 + 0.5);
    let flash = pulse * pulse;
    let fill_alpha = (110.0 + flash * 145.0).round() as u8;
    let halo_alpha = (28.0 + flash * 140.0).round() as u8;
    let halo_radius = radius + 2.0 + flash * 2.6;

    painter.circle_filled(
        center,
        halo_radius,
        egui::Color32::from_rgba_unmultiplied(12, 20, 14, halo_alpha),
    );
    painter.circle_filled(
        center,
        radius,
        egui::Color32::from_rgba_unmultiplied(78, 201, 108, fill_alpha),
    );
}

fn draw_main_clock(
    painter: &egui::Painter,
    time_pos: egui::Pos2,
    wake_t: f32,
) {
    if wake_t <= 0.001 {
        return;
    }

    let time_text = chrono::Local::now().format("%H:%M").to_string();
    let time_font = egui::FontId::new(40.0, egui::FontFamily::Name("Bold".into()));
    let time_galley = painter.layout_no_wrap(
        time_text.clone(),
        time_font.clone(),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(245, 247, 252, 168),
            wake_t,
        ),
    );
    let outline = painter.layout_no_wrap(
        time_text,
        time_font,
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 132),
            wake_t,
        ),
    );
    let offset = 0.9_f32;
    for delta in [
        egui::vec2(offset, 0.0),
        egui::vec2(-offset, 0.0),
        egui::vec2(0.0, offset),
        egui::vec2(0.0, -offset),
        egui::vec2(offset, offset),
        egui::vec2(-offset, offset),
        egui::vec2(offset, -offset),
        egui::vec2(-offset, -offset),
    ] {
        painter.galley(time_pos + delta, outline.clone());
    }
    painter.galley(time_pos, time_galley);
}

fn dlss_tag_text(game: &crate::steam::Game) -> Option<String> {
    game.dlss_version.as_ref().map(|version| {
        let version = version.trim();
        if version.is_empty() {
            "DLSS".to_owned()
        } else {
            format!("DLSS {}", version)
        }
    })
}

fn draw_title_tag(
    painter: &egui::Painter,
    text: &str,
    title_pos: egui::Pos2,
    title_size: egui::Vec2,
    opacity: f32,
    x_offset: f32,
    fill_color: egui::Color32,
    text_color: egui::Color32,
) -> f32 {
    let alpha = (255.0 * opacity.clamp(0.0, 1.0)).round() as u8;
    if alpha == 0 {
        return 0.0;
    }

    let tag_font = egui::FontId::new(
        (title_size.y * 0.42).clamp(11.0, 14.0),
        egui::FontFamily::Name("Bold".into()),
    );
    let text_color = egui::Color32::from_rgba_unmultiplied(
        text_color.r(),
        text_color.g(),
        text_color.b(),
        alpha,
    );
    let galley = painter.layout_no_wrap(text.to_owned(), tag_font, text_color);
    let padding_x = 11.0;
    let padding_y = 4.0;
    let tag_rect = egui::Rect::from_min_size(
        egui::pos2(title_pos.x + title_size.x + 14.0 + x_offset, title_pos.y + title_size.y * 0.5 - galley.size().y * 0.5 - padding_y),
        egui::vec2(galley.size().x + padding_x * 2.0, galley.size().y + padding_y * 2.0),
    );

    painter.rect_filled(
        tag_rect,
        egui::Rounding::same((tag_rect.height() * 0.5).min(10.0)),
        egui::Color32::from_rgba_unmultiplied(
            fill_color.r(),
            fill_color.g(),
            fill_color.b(),
            ((alpha as f32) * 0.9).round() as u8,
        ),
    );
    painter.galley(
        egui::pos2(tag_rect.min.x + padding_x, tag_rect.min.y + padding_y),
        galley,
    );

    tag_rect.width()
}

struct SelectedGameHeaderContent {
    title_galley: std::sync::Arc<egui::Galley>,
    playtime_galley: Option<std::sync::Arc<egui::Galley>>,
    achievement_galley: Option<std::sync::Arc<egui::Galley>>,
    title_font: egui::FontId,
}

impl SelectedGameHeaderContent {
    fn total_height(&self) -> f32 {
        let meta_height = self
            .playtime_galley
            .as_ref()
            .map(|galley| galley.size().y)
            .into_iter()
            .chain(
                self.achievement_galley
                    .as_ref()
                    .map(|galley| galley.size().y),
            )
            .fold(0.0, f32::max);

        self.title_galley.size().y
            + if meta_height > 0.0 {
                2.0 + meta_height
            } else {
                0.0
            }
    }
}

fn build_selected_game_header(
    ui: &egui::Ui,
    painter: &egui::Painter,
    language: AppLanguage,
    game: &crate::steam::Game,
    summary: Option<&crate::steam::AchievementSummary>,
    achievement_summary_reveal: f32,
    title_font: egui::FontId,
    title_color: egui::Color32,
    meta_font_size: f32,
    meta_alpha: f32,
    meta_max_width: f32,
) -> SelectedGameHeaderContent {
    let title_galley = painter.layout_no_wrap(game.name.clone(), title_font.clone(), title_color);
    let playtime_str = language.format_playtime(game.playtime_minutes);
    let has_playtime_meta = !playtime_str.is_empty();
    let achievement_text = summary.and_then(|achievement_summary| {
        (achievement_summary.total > 0).then(|| {
            language.format_achievement_progress(
                achievement_summary.unlocked,
                achievement_summary.total,
            )
        })
    });
    let achievement_meta_reveal = achievement_summary_reveal.clamp(0.0, 1.0);
    let meta_font = egui::FontId::proportional(meta_font_size);
    let playtime_color = egui::Color32::from_rgba_unmultiplied(
        180,
        180,
        190,
        meta_alpha.clamp(0.0, 255.0) as u8,
    );
    let playtime_galley = has_playtime_meta.then(|| {
        painter.layout_no_wrap(playtime_str, meta_font.clone(), playtime_color)
    });
    let achievement_color = egui::Color32::from_rgba_unmultiplied(
        180,
        180,
        190,
        (meta_alpha * achievement_meta_reveal).clamp(0.0, 255.0) as u8,
    );
    let achievement_galley = achievement_text.map(|text| {
        let prefixed = if has_playtime_meta {
            format!("  •  {}", text)
        } else {
            text
        };
        build_wrapped_galley(ui, prefixed, meta_font, achievement_color, meta_max_width)
    });

    SelectedGameHeaderContent {
        title_galley,
        playtime_galley,
        achievement_galley,
        title_font,
    }
}

fn draw_selected_game_header(
    painter: &egui::Painter,
    content: &SelectedGameHeaderContent,
    game_name: &str,
    title_pos: egui::Pos2,
    alpha_scale: f32,
) {
    let outline_alpha = scale_alpha(200, alpha_scale);
    if outline_alpha == 0 {
        return;
    }

    let outline_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, outline_alpha);
    let outline_galley = painter.layout_no_wrap(
        game_name.to_owned(),
        content.title_font.clone(),
        outline_color,
    );
    let d = 0.8_f32;
    for off in [
        egui::vec2(d, 0.0),
        egui::vec2(-d, 0.0),
        egui::vec2(0.0, d),
        egui::vec2(0.0, -d),
        egui::vec2(d, d),
        egui::vec2(-d, d),
        egui::vec2(d, -d),
        egui::vec2(-d, -d),
    ] {
        painter.galley(title_pos + off, outline_galley.clone());
    }

    painter.galley(title_pos, content.title_galley.clone());

    if content.playtime_galley.is_some() || content.achievement_galley.is_some() {
        let meta_pos = egui::pos2(title_pos.x, title_pos.y + content.title_galley.size().y + 2.0);
        let mut meta_x = meta_pos.x;
        if let Some(playtime_galley) = &content.playtime_galley {
            painter.galley(egui::pos2(meta_x, meta_pos.y), playtime_galley.clone());
            meta_x += playtime_galley.size().x;
        }
        if let Some(achievement_galley) = &content.achievement_galley {
            painter.galley(egui::pos2(meta_x, meta_pos.y), achievement_galley.clone());
        }
    }
}

pub struct HintIcons {
    pub btn_a: egui::TextureHandle,
    pub btn_b: egui::TextureHandle,
    pub btn_x: egui::TextureHandle,
    pub btn_y: egui::TextureHandle,
    pub dpad_down: egui::TextureHandle,
    pub guide: egui::TextureHandle,
}

fn png_bytes_to_texture(
    ctx: &egui::Context,
    bytes: &[u8],
    label: &str,
) -> Option<egui::TextureHandle> {
    let dyn_img = image::load_from_memory(bytes).ok()?;
    let rgba = dyn_img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels: Vec<egui::Color32> = rgba
        .pixels()
        .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();
    let image = egui::ColorImage { size, pixels };
    Some(ctx.load_texture(label, image, egui::TextureOptions::LINEAR))
}

pub fn load_hint_icons(ctx: &egui::Context) -> Option<HintIcons> {
    let btn_a_bytes = include_bytes!("icons/Xbox Series/xbox_button_a_outline.png") as &[u8];
    let btn_b_bytes = include_bytes!("icons/Xbox Series/xbox_button_b_outline.png") as &[u8];
    let btn_x_bytes = include_bytes!("icons/Xbox Series/xbox_button_x_outline.png") as &[u8];
    let btn_y_bytes = include_bytes!("icons/Xbox Series/xbox_button_y_outline.png") as &[u8];
    let dpad_down_bytes = include_bytes!("icons/Xbox Series/xbox_dpad_down_outline.png") as &[u8];
    let guide_bytes = include_bytes!("icons/Xbox Series/xbox_guide_outline.png") as &[u8];
    let label_prefix = "xbox_series";

    let btn_a = png_bytes_to_texture(ctx, btn_a_bytes, &format!("{}_icon_btn_a", label_prefix))?;
    let btn_b = png_bytes_to_texture(ctx, btn_b_bytes, &format!("{}_icon_btn_b", label_prefix))?;
    let btn_x = png_bytes_to_texture(ctx, btn_x_bytes, &format!("{}_icon_btn_x", label_prefix))?;
    let btn_y = png_bytes_to_texture(ctx, btn_y_bytes, &format!("{}_icon_btn_y", label_prefix))?;
    let dpad_down = png_bytes_to_texture(
        ctx,
        dpad_down_bytes,
        &format!("{}_icon_dpad_down", label_prefix),
    )?;
    let guide = png_bytes_to_texture(ctx, guide_bytes, &format!("{}_icon_guide", label_prefix))?;
    Some(HintIcons {
        btn_a,
        btn_b,
        btn_x,
        btn_y,
        dpad_down,
        guide,
    })
}

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

    // Solid dark background
    bg_painter.rect_filled(screen, egui::Rounding::ZERO, egui::Color32::from_rgb(18, 18, 18));

    // Image rect: fill screen width, pin to top, keep aspect ratio
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

    let draw_logo = |texture: &egui::TextureHandle,
                     hero_rect: egui::Rect,
                     alpha_scale: f32| {
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

    // Previous cover (fading out)
    if cover_fade < 1.0 {
        if let Some((_id, tex)) = cover_prev {
            let hero_rect = top_rect(tex, 0.0);
            let alpha = (base_alpha * (1.0 - cover_fade) * wake_alpha_scale) as u8;
            let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
            bg_painter.image(tex.id(), hero_rect, uv, tint);
        }
    }

    // Current cover (fading in, sliding in)
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

pub fn draw_game_list(
    ui: &mut egui::Ui,
    language: AppLanguage,
    games: &[crate::steam::Game],
    selected: usize,
    select_anim: f32,
    achievement_panel_anim: f32,
    scroll_offset: f32,
    game_icons: &std::collections::HashMap<u32, egui::TextureHandle>,
    launch_feedback: Option<(usize, f32)>,
    running_indices: &[usize],
    _achievement_panel_active: bool,
    achievement_summary_for_selected: Option<&crate::steam::AchievementSummary>,
    achievement_summary_reveal_for_selected: f32,
    wake_anim: f32,
) {
    let base_icon_size: f32 = 122.0;
    let selected_icon_size: f32 = 256.0;
    let selected_icon_extra = selected_icon_size - base_icon_size;

    let panel_rect = ui.available_rect_before_wrap();
    let padding = 50.0;
    let padded_rect = panel_rect.shrink(padding);
    let page_scroll_t = smoothstep01(achievement_panel_anim);
    let page_offset_y = -panel_rect.height() * page_scroll_t;
    let wake_t = smoothstep01(wake_anim);
    let wake_offset_y = lerp_f32(42.0, 0.0, wake_t);

    let selected_size = 30.0;
    let base_size = 18.0;
    let column_spacing = 152.0;

    let hero_ratio = 1240.0 / 3840.0;
    let img_bottom = panel_rect.min.y + panel_rect.width() * hero_ratio;
    let content_top = img_bottom + 32.0 + page_offset_y + wake_offset_y;
    let anchor_x = padded_rect.min.x + 24.0;
    let painter = ui.painter().with_clip_rect(panel_rect);

    if !running_indices.is_empty() || launch_feedback.is_some() {
        ui.ctx().request_repaint();
    }

    for (i, g) in games.iter().enumerate() {
        let offset_f = i as f32 - scroll_offset;
        let is_selected = i == selected;

        let dist = offset_f.abs();
        let sign = if offset_f >= 0.0 { 1.0 } else { -1.0 };
        let icon_focus_t = smoothstep01((1.0 - dist).clamp(0.0, 1.0));
        let selection_t = if is_selected {
            smoothstep01(select_anim)
        } else {
            0.0
        };
        // Keep the enlarged item and the shrinking follower sharing a constant
        // total extra width so later items do not jitter during horizontal motion.
        let right_side_compensation = if offset_f > 0.0 {
            selected_icon_extra * smoothstep01(offset_f.clamp(0.0, 1.0))
        } else {
            0.0
        };
        let x_pos = anchor_x + sign * dist * column_spacing + right_side_compensation;
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
        let icon_scale = launch_elapsed_seconds
            .map(launch_icon_scale)
            .unwrap_or(1.0);
        let icon_size = icon_slot_size * icon_scale;
        let icon_offset_y = launch_elapsed_seconds
            .map(launch_icon_offset_y)
            .unwrap_or(0.0);
        let meta_text_width = (icon_slot_size + 58.0).max(160.0);
        let item_left = x_pos;
        let text_x = item_left;

        let font_id = if is_selected {
            egui::FontId::new(font_size, egui::FontFamily::Name("Bold".into()))
        } else {
            egui::FontId::proportional(font_size)
        };
        let text_y = content_top + icon_slot_size + 20.0;

        if let Some(app_id) = g.app_id {
            if let Some(icon_tex) = game_icons.get(&app_id) {
                let icon_tint = color_with_scaled_alpha(egui::Color32::WHITE, wake_t);
                let icon_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        item_left + (icon_slot_size - icon_size) * 0.5,
                        content_top + (icon_slot_size - icon_size) + icon_offset_y,
                    ),
                    egui::vec2(icon_size, icon_size),
                );
                draw_game_icon(&painter, icon_tex, icon_rect, icon_tint);

                if show_running_status && wake_t > 0.12 {
                    draw_running_status_dot(&painter, icon_rect);
                }
            }
        }

        if is_selected {
            let header = build_selected_game_header(
                ui,
                &painter,
                language,
                g,
                achievement_summary_for_selected,
                achievement_summary_reveal_for_selected,
                font_id,
                text_color,
                selected_size * 0.5,
                140.0 * meta_t,
                meta_text_width,
            );
            let normal_title_pos = egui::pos2(text_x, text_y);
            draw_selected_game_header(
                &painter,
                &header,
                &g.name,
                normal_title_pos,
                wake_t,
            );

        }
    }
}

fn draw_achievement_icon(
    painter: &egui::Painter,
    texture: &egui::TextureHandle,
    icon_rect: egui::Rect,
    tint: egui::Color32,
    reveal: f32,
) {
    const ACHIEVEMENT_ICON_ROUNDING: f32 = 4.0;
    let [tex_w, tex_h] = texture.size();
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

    let draw_rect = if tex_w > 0 && tex_h > 0 {
        let tex_w = tex_w as f32;
        let tex_h = tex_h as f32;
        let aspect = tex_w / tex_h;
        let icon_size = icon_rect.width().min(icon_rect.height());

        let (scaled_w, scaled_h) = if aspect > 1.0 {
            (icon_size, icon_size / aspect)
        } else {
            (icon_size * aspect, icon_size)
        };

        let center = icon_rect.center();
        egui::Rect::from_center_size(center, egui::vec2(scaled_w, scaled_h))
    } else {
        icon_rect
    };

    let reveal = reveal.clamp(0.0, 1.0);
    let alpha = ((tint.a() as f32) * reveal).round() as u8;
    let fade_tint = egui::Color32::from_rgba_unmultiplied(tint.r(), tint.g(), tint.b(), alpha);
    painter.add(egui::Shape::Rect(egui::epaint::RectShape {
        rect: draw_rect,
        rounding: egui::Rounding::same(ACHIEVEMENT_ICON_ROUNDING),
        fill: fade_tint,
        stroke: egui::Stroke::NONE,
        fill_texture_id: texture.id(),
        uv,
    }));
}

fn draw_centered_achievement_loading(ui: &egui::Ui, rect: egui::Rect) {
    let painter = ui.painter().clone();
    let time = ui.input(|input| input.time) as f32;
    let center = rect.center();
    let spacing = 24.0;
    let radius = 5.5;
    let jump = 10.0;
    let color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10);

    for index in 0..3 {
        let phase = time * 5.4 - index as f32 * 0.32;
        let bounce = phase.sin().max(0.0);
        let x = center.x + (index as f32 - 1.0) * spacing;
        let y = center.y - bounce * jump;
        painter.circle_filled(egui::pos2(x, y), radius, color);
    }

    ui.ctx().request_repaint();
}

fn draw_centered_achievement_empty(
    painter: &egui::Painter,
    rect: egui::Rect,
    language: AppLanguage,
) {
    let empty_galley = painter.layout_no_wrap(
        language.achievement_empty_text().to_string(),
        egui::FontId::proportional(18.0),
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10),
    );
    painter.galley(
        egui::pos2(
            rect.center().x - empty_galley.size().x * 0.5,
            rect.center().y - empty_galley.size().y * 0.5,
        ),
        empty_galley,
    );
}

fn format_achievement_percent(global_percent: Option<f32>) -> String {
    match global_percent.filter(|value| value.is_finite()) {
        Some(value) => format!("{:.1}%", value),
        None => "--.-%".to_string(),
    }
}

fn achievement_percent_fill_t(global_percent: Option<f32>) -> f32 {
    global_percent
        .filter(|value| value.is_finite())
        .map(|value| (value / 100.0).clamp(0.0, 1.0))
        .unwrap_or(0.0)
}

fn masked_achievement_text(source: &str) -> String {
    let glyph_count = source
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .count()
        .clamp(14, 42);
    let mut masked = String::new();
    for index in 0..glyph_count {
        if index > 0 && index % 6 == 0 {
            masked.push(' ');
        }
        masked.push('•');
    }
    masked
}

fn draw_badge(
    painter: &egui::Painter,
    text: &str,
    top_left: egui::Pos2,
    fill: egui::Color32,
    text_color: egui::Color32,
    alpha_scale: f32,
) -> egui::Vec2 {
    let alpha_fill = color_with_scaled_alpha(fill, alpha_scale);
    let alpha_text = color_with_scaled_alpha(text_color, alpha_scale);
    let font = egui::FontId::new(12.5, egui::FontFamily::Name("Bold".into()));
    let galley = painter.layout_no_wrap(text.to_string(), font, alpha_text);
    let size = egui::vec2(galley.size().x + 18.0, galley.size().y + 9.0);
    let rect = egui::Rect::from_min_size(top_left, size);
    painter.rect_filled(
        rect,
        egui::Rounding::same((rect.height() * 0.5).min(9.0)),
        alpha_fill,
    );
    painter.galley(
        egui::pos2(rect.min.x + 9.0, rect.min.y + (rect.height() - galley.size().y) * 0.5),
        galley,
    );
    size
}

fn draw_hidden_achievement_overlay(
    painter: &egui::Painter,
    row_rect: egui::Rect,
    language: AppLanguage,
    show_prompt: bool,
    icons: Option<&HintIcons>,
    reveal_progress: f32,
    alpha_scale: f32,
) {
    let overlay_alpha = (1.0 - reveal_progress).clamp(0.0, 1.0) * alpha_scale;
    if overlay_alpha <= 0.001 {
        return;
    }

    let overlay_rect = row_rect;
    let overlay_painter = painter.with_clip_rect(overlay_rect);
    overlay_painter.rect_filled(
        overlay_rect,
        egui::Rounding::same(6.0),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(20, 22, 26, 232),
            overlay_alpha,
        ),
    );

    if !show_prompt {
        return;
    }

    let title = painter.layout_no_wrap(
        language.achievement_hidden_text().to_string(),
        egui::FontId::new(16.0, egui::FontFamily::Name("Bold".into())),
        color_with_scaled_alpha(egui::Color32::from_rgb(236, 239, 242), overlay_alpha),
    );
    let title_size = title.size();
    let icon_size = 26.0;
    let icon_gap = 8.0;
    let group_width = title_size.x
        + if icons.is_some() {
            icon_gap + icon_size
        } else {
            0.0
        };
    let title_pos = egui::pos2(
        overlay_rect.center().x - group_width * 0.5,
        overlay_rect.center().y - title_size.y * 0.5,
    );
    overlay_painter.galley(title_pos, title);

    if let Some(icons) = icons {
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(
                title_pos.x + title_size.x + icon_gap,
                overlay_rect.center().y - icon_size * 0.5,
            ),
            egui::vec2(icon_size, icon_size),
        );
        overlay_painter.image(
            icons.btn_a.id(),
            icon_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            color_with_scaled_alpha(egui::Color32::WHITE, overlay_alpha),
        );
    }
}

pub fn draw_achievement_page(
    ui: &mut egui::Ui,
    language: AppLanguage,
    game: &crate::steam::Game,
    summary: Option<&crate::steam::AchievementSummary>,
    is_loading: bool,
    has_no_data: bool,
    achievement_summary_reveal_for_selected: f32,
    selected_index: usize,
    achievement_select_anim: f32,
    achievement_panel_anim: f32,
    _selected_game_index: usize,
    game_select_anim: f32,
    _game_scroll_offset: f32,
    scroll_offset: f32,
    wake_anim: f32,
    _game_icon: Option<&egui::TextureHandle>,
    hint_icons: Option<&HintIcons>,
    revealed_hidden: Option<&str>,
    hidden_reveal_progress: f32,
    sort_high_to_low: bool,
    achievement_icon_cache: &std::collections::HashMap<String, egui::TextureHandle>,
    achievement_icon_reveal: &std::collections::HashMap<String, f32>,
) -> Vec<String> {
    let mut visible_icon_urls = Vec::new();
    let panel_rect = ui.available_rect_before_wrap();
    let padding = 50.0;
    let padded_rect = panel_rect.shrink(padding);
    let painter = ui.painter().with_clip_rect(panel_rect);
    let panel_t = smoothstep01(achievement_panel_anim);
    let wake_t = smoothstep01(wake_anim);
    let page_enter_offset_y = lerp_f32(panel_rect.height() + 28.0, 0.0, panel_t)
        + lerp_f32(30.0, 0.0, wake_t);
    let content_top = padded_rect.min.y + 18.0;
    let title_font_size = 18.0 + (30.0 - 18.0) * smoothstep01(game_select_anim);
    let title_font = egui::FontId::new(title_font_size, egui::FontFamily::Name("Bold".into()));
    let header = build_selected_game_header(
        ui,
        &painter,
        language,
        game,
        summary,
        achievement_summary_reveal_for_selected,
        title_font,
        egui::Color32::WHITE,
        15.0,
        140.0,
        (padded_rect.width() - 96.0).max(220.0),
    );
    let header_text_x = padded_rect.min.x + 24.0;
    let text_block_height = header.total_height();
    let text_top = content_top + 64.0 - text_block_height;
    let title_base_pos = egui::pos2(header_text_x, text_top);
    let header_bottom = title_base_pos.y + text_block_height;
    let header_base_rect = egui::Rect::from_min_max(
        egui::pos2(padded_rect.min.x + 8.0, content_top),
        egui::pos2(padded_rect.max.x - 8.0, header_bottom + 26.0),
    );
    let list_base_rect = egui::Rect::from_min_max(
        egui::pos2(padded_rect.min.x + 8.0, header_base_rect.max.y + 24.0),
        egui::pos2(padded_rect.max.x - 8.0, padded_rect.max.y - 52.0),
    );
    let content_offset = egui::vec2(0.0, page_enter_offset_y);
    let list_rect = list_base_rect.translate(content_offset);
    let title_pos = title_base_pos + content_offset;
    draw_selected_game_header(&painter, &header, &game.name, title_pos, wake_t);
    if let Some(tag_text) = dlss_tag_text(game) {
        let _ = draw_title_tag(
            &painter,
            &tag_text,
            title_pos,
            header.title_galley.size(),
            panel_t * wake_t,
            0.0,
            egui::Color32::from_rgb(228, 228, 220),
            egui::Color32::from_rgb(18, 18, 18),
        );
    }

    painter.rect_filled(
        list_rect,
        egui::Rounding::same(8.0),
        color_with_scaled_alpha(egui::Color32::from_rgb(14, 14, 14), wake_t),
    );

    let list_inner_rect = egui::Rect::from_min_max(
        egui::pos2(list_rect.min.x + 10.0, list_rect.min.y + 16.0),
        egui::pos2(list_rect.max.x - 18.0, list_rect.max.y - 16.0),
    );
    let sort_badge_text = if sort_high_to_low {
        language.unlock_rate_high_to_low_text()
    } else {
        language.unlock_rate_low_to_high_text()
    };
    let row_side_inset = 6.0;
    let unselected_row_shrink_x = 7.0;
    let sort_badge_x = list_inner_rect.min.x + row_side_inset + unselected_row_shrink_x;
    let sort_badge_size = draw_badge(
        &painter,
        sort_badge_text,
        egui::pos2(sort_badge_x, list_inner_rect.min.y),
        egui::Color32::from_rgba_unmultiplied(70, 86, 104, 190),
        egui::Color32::from_rgb(226, 232, 240),
        wake_t,
    );
    if let Some(icons) = hint_icons {
        let icon_size = 28.0;
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(
                sort_badge_x + sort_badge_size.x + 10.0,
                list_inner_rect.min.y + (sort_badge_size.y - icon_size) * 0.5,
            ),
            egui::vec2(icon_size, icon_size),
        );
        painter.image(
            icons.btn_y.id(),
            icon_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            color_with_scaled_alpha(egui::Color32::WHITE, wake_t),
        );
    }

    let Some(summary) = summary else {
        if is_loading && !has_no_data {
            draw_centered_achievement_loading(ui, list_rect);
        } else {
            draw_centered_achievement_empty(&painter, list_rect, language);
        }
        return visible_icon_urls;
    };

    if summary.items.is_empty() {
        if is_loading && !has_no_data {
            draw_centered_achievement_loading(ui, list_rect);
        } else {
            draw_centered_achievement_empty(&painter, list_rect, language);
        }
        return visible_icon_urls;
    }

    let item_gap_y = 14.0;
    let row_spacing = 116.0;
    let header_band_height = 42.0;
    let list_body_rect = egui::Rect::from_min_max(
        egui::pos2(list_inner_rect.min.x, list_inner_rect.min.y + header_band_height),
        list_inner_rect.max,
    );
    let list_painter = painter.with_clip_rect(list_body_rect);
    let visible_rows = (list_body_rect.height() / row_spacing).ceil() as i32 + 2;
    let base_y = list_body_rect.min.y - scroll_offset * row_spacing;

    for (idx, item) in summary.items.iter().enumerate() {
        let row_offset = idx as f32 - scroll_offset;
        if row_offset < -1.5 || row_offset > visible_rows as f32 {
            continue;
        }

        let is_selected = idx == selected_index;
        let selection_t = if is_selected {
            smoothstep01(achievement_select_anim)
        } else {
            0.0
        };
        let row_top = base_y + idx as f32 * row_spacing;
        if row_top > list_body_rect.max.y || row_top + row_spacing < list_body_rect.min.y {
            continue;
        }

        let row_height = row_spacing - item_gap_y;
        let row_slot_rect = egui::Rect::from_min_max(
            egui::pos2(list_body_rect.min.x + row_side_inset, row_top),
            egui::pos2(list_body_rect.max.x - row_side_inset, row_top + row_height),
        );
        let row_rect = row_slot_rect.shrink2(egui::vec2(lerp_f32(unselected_row_shrink_x, 0.0, selection_t), 0.0));
        let content_padding_x = 14.0;
        let content_padding_y = 12.0;
        let icon_gap = 14.0;
        let right_padding = 18.0;
        let hidden_state = item.is_hidden && item.unlocked != Some(true);
        let hidden_revealing = hidden_state
            && revealed_hidden.is_some_and(|revealed_api_name| revealed_api_name == item.api_name);
        let hidden_masked = hidden_state && !hidden_revealing;
        let bg_color = if item.unlocked == Some(true) {
            if is_selected {
                egui::Color32::from_rgb(28, 35, 31)
            } else {
                egui::Color32::from_rgb(21, 27, 23)
            }
        } else if is_selected {
            egui::Color32::from_rgb(30, 32, 36)
        } else {
            egui::Color32::from_rgb(22, 24, 28)
        };
        list_painter.rect_filled(
            row_rect,
            egui::Rounding::same(6.0),
            color_with_scaled_alpha(bg_color, wake_t),
        );

        let fill_t = achievement_percent_fill_t(item.global_percent);
        if fill_t > 0.001 {
            let fill_color = if item.unlocked == Some(true) {
                if is_selected {
                    egui::Color32::from_rgba_unmultiplied(96, 156, 124, 62)
                } else {
                    egui::Color32::from_rgba_unmultiplied(82, 140, 110, 50)
                }
            } else if is_selected {
                egui::Color32::from_rgba_unmultiplied(162, 166, 172, 32)
            } else {
                egui::Color32::from_rgba_unmultiplied(144, 148, 154, 24)
            };
            let fill_max_x = lerp_f32(row_rect.min.x, row_rect.max.x, fill_t);
            let fill_clip_rect = egui::Rect::from_min_max(
                row_rect.min,
                egui::pos2(fill_max_x.max(row_rect.min.x), row_rect.max.y),
            );
            list_painter
                .with_clip_rect(fill_clip_rect)
                .rect_filled(
                    row_rect,
                    egui::Rounding::same(6.0),
                    color_with_scaled_alpha(fill_color, wake_t),
                );
        }

        let icon_column_width = lerp_f32(48.0, 56.0, selection_t);
        let left_content_inset = content_padding_x;
        let text_x = row_rect.min.x + left_content_inset + icon_column_width + icon_gap;
        let right_column_width = 150.0;
        let percent_galley = painter.layout_no_wrap(
            format_achievement_percent(item.global_percent),
            egui::FontId::new(17.0, egui::FontFamily::Name("Bold".into())),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(230, 232, 236, 230),
                wake_t,
            ),
        );
        let unlock_time_text = format_achievement_status(item.unlocked, item.unlock_time);
        let unlock_time_galley = unlock_time_text.as_ref().map(|text| {
            painter.layout_no_wrap(
                text.clone(),
                egui::FontId::proportional(13.0),
                color_with_scaled_alpha(
                    egui::Color32::from_rgba_unmultiplied(150, 154, 162, 220),
                    wake_t,
                ),
            )
        });
        let text_width = (row_rect.width()
            - (text_x - row_rect.min.x)
            - right_column_width
            - 18.0)
            .max(180.0);
        let name = item
            .display_name
            .as_deref()
            .filter(|text| !text.trim().is_empty())
            .unwrap_or(&item.api_name);
        let title_galley = build_wrapped_galley(
            ui,
            name.to_string(),
            if is_selected {
                egui::FontId::new(lerp_f32(18.0, 20.0, selection_t), egui::FontFamily::Name("Bold".into()))
            } else {
                egui::FontId::proportional(18.0)
            },
            color_with_scaled_alpha(
                if item.unlocked == Some(true) {
                    egui::Color32::from_rgba_unmultiplied(230, 239, 232, if is_selected { 255 } else { 235 })
                } else {
                    egui::Color32::from_rgba_unmultiplied(222, 224, 228, if is_selected { 255 } else { 228 })
                },
                wake_t,
            ),
            text_width,
        );
        let base_description_text = item
            .description
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .unwrap_or(language.no_description_text());
        let description_text = if hidden_masked {
            masked_achievement_text(base_description_text)
        } else {
            base_description_text.to_string()
        };
        let description_galley = build_wrapped_galley(
            ui,
            description_text,
            egui::FontId::proportional(14.0),
            color_with_scaled_alpha(
                if hidden_masked {
                    egui::Color32::from_rgba_unmultiplied(150, 154, 160, 176)
                } else {
                    egui::Color32::from_rgba_unmultiplied(148, 152, 160, 220)
                },
                wake_t,
            ),
            text_width,
        );
        let text_block_height = title_galley.size().y + 6.0 + description_galley.size().y;
        let icon_size = text_block_height
            .min(row_rect.height() - content_padding_y * 2.0)
            .clamp(40.0, icon_column_width);
        let content_top = row_rect.min.y + (row_rect.height() - text_block_height.max(icon_size)) * 0.5;
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(
                row_rect.min.x + left_content_inset + (icon_column_width - icon_size) * 0.5,
                content_top,
            ),
            egui::vec2(icon_size, icon_size),
        );

        let icon_key = match item.unlocked {
            Some(true) => item.icon_url.as_ref().or(item.icon_gray_url.as_ref()),
            _ => item.icon_gray_url.as_ref().or(item.icon_url.as_ref()),
        };
        if let Some(key) = icon_key {
            visible_icon_urls.push(key.clone());
        }
        if let Some(tex) = icon_key.and_then(|key| achievement_icon_cache.get(key)) {
            let reveal = icon_key
                .and_then(|key| achievement_icon_reveal.get(key).copied())
                .unwrap_or(1.0);
            draw_achievement_icon(
                &list_painter,
                tex,
                icon_rect,
                color_with_scaled_alpha(
                    if item.unlocked == Some(true) {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgba_unmultiplied(216, 220, 228, 220)
                    },
                    wake_t,
                ),
                reveal,
            );
        } else {
            let fill = if hidden_state {
                egui::Color32::from_rgb(102, 106, 112)
            } else {
                match item.unlocked {
                    Some(true) => egui::Color32::from_rgb(86, 172, 132),
                    Some(false) => egui::Color32::from_rgb(108, 112, 122),
                    None => egui::Color32::from_rgb(82, 88, 102),
                }
            };
            list_painter.rect_filled(
                icon_rect,
                egui::Rounding::same(4.0),
                color_with_scaled_alpha(fill, wake_t),
            );
        }

        let text_top = content_top;
        list_painter.galley(egui::pos2(text_x, text_top), title_galley.clone());
        let description_pos = egui::pos2(text_x, text_top + title_galley.size().y + 6.0);
        list_painter.galley(description_pos, description_galley.clone());
        let right_column_rect = egui::Rect::from_min_max(
            egui::pos2(row_rect.max.x - right_padding - right_column_width, row_rect.min.y),
            egui::pos2(row_rect.max.x - right_padding, row_rect.max.y),
        );
        let right_block_spacing = 8.0;
        let right_block_height = percent_galley.size().y
            + unlock_time_galley
                .as_ref()
                .map(|galley| right_block_spacing + galley.size().y)
                .unwrap_or(0.0);
        let right_block_top = right_column_rect.center().y - right_block_height * 0.5;
        let right_column_x = right_column_rect.max.x;
        let percent_pos = egui::pos2(
            right_column_x - percent_galley.size().x,
            right_block_top,
        );
        list_painter.galley(percent_pos, percent_galley.clone());
        if let Some(unlock_time_galley) = unlock_time_galley {
            list_painter.galley(
                egui::pos2(
                    right_column_x - unlock_time_galley.size().x,
                    percent_pos.y + percent_galley.size().y + right_block_spacing,
                ),
                unlock_time_galley,
            );
        }

        if hidden_state {
            let reveal_progress = if hidden_revealing {
                hidden_reveal_progress
            } else {
                0.0
            };
            draw_hidden_achievement_overlay(
                &list_painter,
                row_rect,
                language,
                is_selected,
                if is_selected { hint_icons } else { None },
                reveal_progress,
                wake_t,
            );
        }

    }

    visible_icon_urls
}

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
    let g_force_close = painter.layout_no_wrap(language.hold_close_game_text().to_string(), hint_font.clone(), hint_color);
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
        let g_scroll = painter.layout_no_wrap(language.scroll_text().to_string(), hint_font.clone(), hint_color);
        let g_refresh = painter.layout_no_wrap(language.refresh_text().to_string(), hint_font.clone(), hint_color);

        let group_width = |galley: &std::sync::Arc<egui::Galley>| action_icon_h + 6.0 + galley.size().x;
        let mut cursor_x = b_icon_x - 20.0;

        let refresh_x = cursor_x - group_width(&g_refresh);
        draw_icon(painter, &icons.btn_x, refresh_x, action_icon_h);
        if achievement_refresh_loading {
            draw_loading_ring(
                painter,
                egui::pos2(
                    refresh_x + action_icon_h * 0.5,
                    hint_y + row_h * 0.5,
                ),
                action_icon_h * 0.49,
            );
        }
        painter.galley(
            egui::pos2(
                refresh_x + action_icon_h + 6.0,
                hint_y + (row_h - g_refresh.size().y) * 0.5,
            ),
            g_refresh,
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
        );

        draw_icon(painter, &icons.btn_b, b_icon_x, action_icon_h);

        let gy = hint_y + (row_h - g_back.size().y) * 0.5;
        painter.galley(egui::pos2(b_label_x, gy), g_back);

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
        );
    }

    if game_running {
        draw_icon(painter, &icons.btn_x, force_close_x, action_icon_h);
        draw_progress_ring(
            painter,
            egui::pos2(
                force_close_x + action_icon_h * 0.5,
                hint_y + row_h * 0.5,
            ),
            action_icon_h * 0.48,
            force_close_hold_progress,
        );

        let gy = hint_y + (row_h - g_force_close.size().y) * 0.5;
        painter.galley(
            egui::pos2(force_close_x + action_icon_h + 6.0, gy),
            g_force_close,
        );
    }

    draw_icon(painter, &icons.btn_a, launch_x, action_icon_h);

    let gy = hint_y + (row_h - g_launch.size().y) * 0.5;
    painter.galley(
        egui::pos2(launch_x + action_icon_h + 6.0, gy),
        g_launch,
    );

    draw_icon(painter, &icons.guide, home_menu_x, action_icon_h);
}

pub fn draw_home_menu(
    ui: &mut egui::Ui,
    language: AppLanguage,
    icons: Option<&HintIcons>,
    menu_anim: f32,
    selected_option_t: f32,
    wake_anim: f32,
) {
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

    let panel_rect = ui.available_rect_before_wrap();
    let painter = ui.painter();
    painter.rect_filled(
        panel_rect,
        egui::Rounding::ZERO,
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(6, 8, 12, 178),
            overlay_t,
        ),
    );

    let option_height = 100.0;
    let option_gap = 22.0;
    let content_padding = 28.0;
    let content_height = option_height;
    let sheet_height = (panel_rect.height() * 0.34)
        .clamp(content_height + content_padding * 2.0, 360.0);
    let sheet_rect = egui::Rect::from_center_size(panel_rect.center(), egui::vec2(panel_rect.width(), sheet_height));
    painter.rect_filled(
        sheet_rect,
        egui::Rounding::ZERO,
        color_with_scaled_alpha(egui::Color32::from_rgb(18, 19, 22), sheet_t),
    );

    let option_labels = [
        language.minimize_app_text(),
        language.close_app_text(),
    ];
    let option_font = egui::FontId::new(22.0, egui::FontFamily::Name("Bold".into()));
    let content_width = (sheet_rect.width() * 0.50).clamp(420.0, 760.0);
    let content_rect = egui::Rect::from_center_size(
        sheet_rect.center(),
        egui::vec2(content_width, content_height),
    );
    let option_width = (content_rect.width() - option_gap) * 0.5;
    let option_inner_padding = 24.0;
    let selected_expand = egui::vec2(10.0, 10.0);
    let selected_slide_t = selected_option_t.clamp(0.0, (option_labels.len() - 1) as f32);
    let option_rects: Vec<_> = option_labels
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let option_t = phase_t(0.14 + index as f32 * 0.08, 0.74 + index as f32 * 0.08);
            let option_offset = egui::vec2(0.0, lerp_f32(12.0, 0.0, option_t));
            egui::Rect::from_min_size(
                egui::pos2(
                    content_rect.min.x + index as f32 * (option_width + option_gap),
                    content_rect.min.y,
                ),
                egui::vec2(option_width, option_height),
            )
            .translate(option_offset)
        })
        .collect();
    let highlight_offset = egui::vec2(0.0, lerp_f32(8.0, 0.0, highlight_t));
    let highlight_scale = lerp_f32(0.965, 1.0, highlight_t);
    let selected_rect = egui::Rect::from_center_size(
        egui::pos2(
            content_rect.min.x
                + option_width * 0.5
                + selected_slide_t * (option_width + option_gap),
            content_rect.center().y,
        ),
        egui::vec2(
            (option_width + selected_expand.x) * highlight_scale,
            (option_height + selected_expand.y) * highlight_scale,
        ),
    )
    .translate(highlight_offset);

    for (index, option_rect) in option_rects.iter().enumerate() {
        let option_t = phase_t(0.14 + index as f32 * 0.08, 0.74 + index as f32 * 0.08);
        painter.rect_filled(
            *option_rect,
            egui::Rounding::same(12.0),
            color_with_scaled_alpha(egui::Color32::from_rgb(28, 30, 34), option_t),
        );
    }

    painter.rect_filled(
        selected_rect,
        egui::Rounding::same(14.0),
        color_with_scaled_alpha(egui::Color32::from_rgb(86, 90, 100), highlight_t),
    );

    for (index, (label, option_rect)) in option_labels.iter().zip(option_rects.iter()).enumerate() {
        let option_t = phase_t(0.14 + index as f32 * 0.08, 0.74 + index as f32 * 0.08);
        let selectedness = smoothstep01(1.0 - (selected_slide_t - index as f32).abs().clamp(0.0, 1.0));
        let text_color = egui::Color32::from_rgb(
            lerp_f32(214.0, 248.0, selectedness).round() as u8,
            lerp_f32(218.0, 249.0, selectedness).round() as u8,
            lerp_f32(226.0, 252.0, selectedness).round() as u8,
        );
        let option_text = painter.layout_no_wrap(
            (*label).to_string(),
            option_font.clone(),
            color_with_scaled_alpha(text_color, option_t),
        );
        painter.galley(
            egui::pos2(
                option_rect.min.x + option_inner_padding,
                option_rect.center().y - option_text.size().y * 0.5,
            ),
            option_text,
        );
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
    }
}
