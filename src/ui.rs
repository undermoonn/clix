use eframe::egui;

use crate::i18n::AppLanguage;
use crate::input::ControllerBrand;

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
) {
    let outline_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200);
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
    pub dpad_down: egui::TextureHandle,
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

pub fn load_hint_icons(ctx: &egui::Context, brand: ControllerBrand) -> Option<HintIcons> {
    let (btn_a_bytes, btn_b_bytes, btn_x_bytes, dpad_down_bytes, label_prefix) = match brand {
        ControllerBrand::Xbox => (
            include_bytes!("icons/Xbox/T_X_A_White_Alt.png") as &[u8],
            include_bytes!("icons/Xbox/T_X_B_White_Alt.png") as &[u8],
            include_bytes!("icons/Xbox/T_X_X_White_Alt.png") as &[u8],
            include_bytes!("icons/Xbox/T_X_Dpad_Down_Alt.png") as &[u8],
            "xbox",
        ),
        ControllerBrand::PlayStation => (
            include_bytes!("icons/DualSence/T_P4_Cross.png") as &[u8],
            include_bytes!("icons/DualSence/T_P4_Circle.png") as &[u8],
            include_bytes!("icons/DualSence/T_P4_Square.png") as &[u8],
            include_bytes!("icons/DualSence/T_P4_Dpad_Down.png") as &[u8],
            "playstation",
        ),
    };

    let btn_a = png_bytes_to_texture(ctx, btn_a_bytes, &format!("{}_icon_btn_a", label_prefix))?;
    let btn_b = png_bytes_to_texture(ctx, btn_b_bytes, &format!("{}_icon_btn_b", label_prefix))?;
    let btn_x = png_bytes_to_texture(ctx, btn_x_bytes, &format!("{}_icon_btn_x", label_prefix))?;
    let dpad_down = png_bytes_to_texture(
        ctx,
        dpad_down_bytes,
        &format!("{}_icon_dpad_down", label_prefix),
    )?;
    Some(HintIcons {
        btn_a,
        btn_b,
        btn_x,
        dpad_down,
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
) {
    let screen = ctx.screen_rect();
    let bg_painter = ctx.layer_painter(egui::LayerId::background());
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let base_alpha: f32 = 60.0;
    let hero_ratio = 1240.0 / 3840.0;
    let page_scroll_t = smoothstep01(achievement_panel_anim);
    let page_offset_y = -screen.height() * page_scroll_t;

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
    };

    let fallback_hero_rect = |dx: f32| -> egui::Rect {
        egui::Rect::from_min_size(
            egui::pos2(screen.min.x + dx, screen.min.y + page_offset_y),
            egui::vec2(screen.width(), screen.width() * hero_ratio),
        )
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
            let alpha = (base_alpha * (1.0 - cover_fade)) as u8;
            let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
            bg_painter.image(tex.id(), hero_rect, uv, tint);
        }
    }

    // Current cover (fading in, sliding in)
    let current_dx = cover_nav_dir * slide_distance * (1.0 - ease_t);
    if let Some((_id, tex)) = cover {
        let hero_rect = top_rect(tex, current_dx);
        let alpha = (base_alpha * cover_fade) as u8;
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
) {
    let base_icon_size: f32 = 122.0;
    let selected_icon_size: f32 = 256.0;
    let selected_icon_extra = selected_icon_size - base_icon_size;

    let panel_rect = ui.available_rect_before_wrap();
    let padding = 50.0;
    let padded_rect = panel_rect.shrink(padding);
    let page_scroll_t = smoothstep01(achievement_panel_anim);
    let page_offset_y = -panel_rect.height() * page_scroll_t;

    let selected_size = 30.0;
    let base_size = 18.0;
    let column_spacing = 152.0;

    let hero_ratio = 1240.0 / 3840.0;
    let img_bottom = panel_rect.min.y + panel_rect.width() * hero_ratio;
    let content_top = img_bottom + 32.0 + page_offset_y;
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
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255)
        } else {
            egui::Color32::from_rgba_unmultiplied(200, 200, 210, text_alpha)
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
                let icon_alpha = 255;
                let icon_tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, icon_alpha);
                let icon_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        item_left + (icon_slot_size - icon_size) * 0.5,
                        content_top + (icon_slot_size - icon_size) + icon_offset_y,
                    ),
                    egui::vec2(icon_size, icon_size),
                );
                draw_game_icon(&painter, icon_tex, icon_rect, icon_tint);

                if show_running_status {
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

pub fn draw_achievement_page(
    ui: &mut egui::Ui,
    language: AppLanguage,
    game: &crate::steam::Game,
    summary: Option<&crate::steam::AchievementSummary>,
    is_loading: bool,
    has_no_data: bool,
    achievement_summary_reveal_for_selected: f32,
    selected_index: usize,
    _achievement_select_anim: f32,
    achievement_panel_anim: f32,
    _selected_game_index: usize,
    game_select_anim: f32,
    _game_scroll_offset: f32,
    scroll_offset: f32,
    _game_icon: Option<&egui::TextureHandle>,
    achievement_icon_cache: &std::collections::HashMap<String, egui::TextureHandle>,
    achievement_icon_reveal: &std::collections::HashMap<String, f32>,
) -> Vec<String> {
    let mut visible_icon_urls = Vec::new();
    let panel_rect = ui.available_rect_before_wrap();
    let padding = 50.0;
    let padded_rect = panel_rect.shrink(padding);
    let painter = ui.painter().with_clip_rect(panel_rect);
    let panel_t = smoothstep01(achievement_panel_anim);
    let page_enter_offset_y = lerp_f32(panel_rect.height() + 28.0, 0.0, panel_t);
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
    draw_selected_game_header(&painter, &header, &game.name, title_pos);
    if let Some(tag_text) = dlss_tag_text(game) {
        let _ = draw_title_tag(
            &painter,
            &tag_text,
            title_pos,
            header.title_galley.size(),
            panel_t,
            0.0,
            egui::Color32::from_rgb(228, 228, 220),
            egui::Color32::from_rgb(18, 18, 18),
        );
    }

    painter.rect_filled(
        list_rect,
        egui::Rounding::same(8.0),
        egui::Color32::from_rgb(14, 14, 14),
    );

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

    let item_gap_y = 16.0;
    let row_spacing = 96.0;
    let list_inner_rect = list_rect.shrink2(egui::vec2(18.0, item_gap_y));
    let list_painter = painter.with_clip_rect(list_inner_rect);
    let visible_rows = (list_inner_rect.height() / row_spacing).ceil() as i32 + 2;
    let base_y = list_inner_rect.min.y - scroll_offset * row_spacing;

    for (idx, item) in summary.items.iter().enumerate() {
        let row_offset = idx as f32 - scroll_offset;
        if row_offset < -1.5 || row_offset > visible_rows as f32 {
            continue;
        }

        let is_selected = idx == selected_index;
        let row_top = base_y + idx as f32 * row_spacing;
        if row_top > list_inner_rect.max.y || row_top + row_spacing < list_inner_rect.min.y {
            continue;
        }

        let row_height = row_spacing - item_gap_y;
        let row_rect = egui::Rect::from_min_max(
            egui::pos2(list_inner_rect.min.x, row_top),
            egui::pos2(list_inner_rect.max.x, row_top + row_height),
        );
        let bg_color = if is_selected {
            egui::Color32::from_rgb(36, 36, 36)
        } else {
            egui::Color32::from_rgb(24, 24, 24)
        };
        list_painter.rect_filled(row_rect, egui::Rounding::same(6.0), bg_color);

        let icon_column_width = if is_selected { 52.0 } else { 44.0 };
        let text_x = row_rect.min.x + 16.0 + icon_column_width + 16.0;
        let percent_text = item.global_percent.map(|value| format!("{:.1}%", value));
        let percent_galley = percent_text.as_ref().map(|text| {
            painter.layout_no_wrap(
                text.clone(),
                egui::FontId::proportional(14.0),
                egui::Color32::from_rgba_unmultiplied(186, 190, 198, 220),
            )
        });
        let unlock_time_text = format_achievement_status(item.unlocked, item.unlock_time);
        let unlock_time_galley = unlock_time_text.as_ref().map(|text| {
            painter.layout_no_wrap(
                text.clone(),
                egui::FontId::proportional(12.0),
                egui::Color32::from_rgba_unmultiplied(150, 154, 162, 220),
            )
        });
        let right_galley_width = unlock_time_galley
            .as_ref()
            .map(|galley| galley.size().x + 24.0)
            .or_else(|| percent_galley.as_ref().map(|galley| galley.size().x + 24.0))
            .unwrap_or(0.0);
        let text_width = (row_rect.width() - (text_x - row_rect.min.x) - right_galley_width - 18.0).max(180.0);
        let name = item.display_name.as_deref().unwrap_or(&item.api_name);
        let title_galley = build_wrapped_galley(
            ui,
            name.to_string(),
            if is_selected {
                egui::FontId::new(19.0, egui::FontFamily::Name("Bold".into()))
            } else {
                egui::FontId::proportional(17.0)
            },
            egui::Color32::from_rgba_unmultiplied(222, 224, 228, if is_selected { 255 } else { 228 }),
            text_width,
        );
        let description_text = item
            .description
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .unwrap_or(language.no_description_text());
        let description_galley = build_wrapped_galley(
            ui,
            description_text.to_string(),
            egui::FontId::proportional(13.0),
            egui::Color32::from_rgba_unmultiplied(148, 152, 160, 220),
            text_width,
        );
        let text_block_height = title_galley.size().y + 6.0 + description_galley.size().y;
        let icon_size = text_block_height
            .min(row_height - 12.0)
            .clamp(36.0, icon_column_width);
        let content_top = row_rect.min.y + (row_height - text_block_height.max(icon_size)) * 0.5;
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(row_rect.min.x + 16.0 + (icon_column_width - icon_size) * 0.5, content_top),
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
            draw_achievement_icon(&list_painter, tex, icon_rect, egui::Color32::WHITE, reveal);
        } else {
            let fill = match item.unlocked {
                Some(true) => egui::Color32::from_rgb(86, 172, 132),
                Some(false) => egui::Color32::from_rgb(108, 112, 122),
                None => egui::Color32::from_rgb(82, 88, 102),
            };
            list_painter.rect_filled(icon_rect, egui::Rounding::same(4.0), fill);
        }

        let text_top = content_top;
        list_painter.galley(egui::pos2(text_x, text_top), title_galley.clone());
        list_painter.galley(
            egui::pos2(text_x, text_top + title_galley.size().y + 6.0),
            description_galley,
        );

        if let Some(unlock_time_galley) = unlock_time_galley {
            list_painter.galley(
                egui::pos2(
                    row_rect.max.x - unlock_time_galley.size().x - 16.0,
                    row_rect.min.y + 16.0,
                ),
                unlock_time_galley,
            );
        } else if let Some(percent_galley) = percent_galley {
            list_painter.galley(
                egui::pos2(
                    row_rect.max.x - percent_galley.size().x - 16.0,
                    row_rect.min.y + 16.0,
                ),
                percent_galley,
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
    can_open_achievement_panel: bool,
    game_running: bool,
    quit_hold_progress: f32,
    force_close_hold_progress: f32,
) {
    let panel_rect = ui.available_rect_before_wrap();
    let padding = 50.0;
    let padded_rect = panel_rect.shrink(padding);
    let hint_font = egui::FontId::proportional(20.0);
    let hint_color = egui::Color32::from_rgba_unmultiplied(200, 200, 210, 160);
    let icon_h = 32.0_f32;
    let action_icon_h = icon_h + 8.0;
    let row_h = action_icon_h;
    let hint_y = padded_rect.max.y - 10.0;
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
            egui::Color32::WHITE,
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
        let bg_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 40));
        let fg_stroke = egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 255, 255));
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

    let g_back = painter.layout_no_wrap(language.back_text().to_string(), hint_font.clone(), hint_color);
    let g_quit = painter.layout_no_wrap(language.hold_quit_text().to_string(), hint_font.clone(), hint_color);
    let g_force_close = painter.layout_no_wrap(language.hold_close_game_text().to_string(), hint_font.clone(), hint_color);
    let b_label_reserve = g_back.size().x.max(g_quit.size().x);
    let b_icon_x = padded_rect.max.x - b_label_reserve - 6.0 - action_icon_h;
    let b_label_x = b_icon_x + action_icon_h + 6.0;

    if achievement_panel_active {
        let g_scroll = painter.layout_no_wrap(language.scroll_text().to_string(), hint_font.clone(), hint_color);
        let scroll_group_w = icon_h + 6.0 + g_scroll.size().x;
        let hx = b_icon_x - 20.0 - scroll_group_w;

        draw_icon(painter, &icons.dpad_down, hx, icon_h);
        let text_x = hx + icon_h + 6.0;

        let gy = hint_y + (row_h - g_scroll.size().y) * 0.5;
        painter.galley(egui::pos2(text_x, gy), g_scroll);

        draw_icon(painter, &icons.btn_b, b_icon_x, action_icon_h);

        let gy = hint_y + (row_h - g_back.size().y) * 0.5;
        painter.galley(egui::pos2(b_label_x, gy), g_back);
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
    let launch_x = b_icon_x - 20.0 - launch_group_w;
    let force_close_x = if game_running {
        launch_x - 20.0 - force_close_group_w
    } else {
        launch_x
    };

    if can_open_achievement_panel {
        let achievements_group_w = icon_h + 6.0 + g_achievements.size().x;
        let achievements_x = force_close_x - 20.0 - achievements_group_w;
        draw_icon(painter, &icons.dpad_down, achievements_x, icon_h);

        let gy = hint_y + (row_h - g_achievements.size().y) * 0.5;
        painter.galley(
            egui::pos2(achievements_x + icon_h + 6.0, gy),
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

    draw_icon(painter, &icons.btn_b, b_icon_x, action_icon_h);
    draw_progress_ring(
        painter,
        egui::pos2(
            b_icon_x + action_icon_h * 0.5,
            hint_y + row_h * 0.5,
        ),
        action_icon_h * 0.48,
        quit_hold_progress,
    );
    let gy = hint_y + (row_h - g_quit.size().y) * 0.5;
    painter.galley(egui::pos2(b_label_x, gy), g_quit);
}
