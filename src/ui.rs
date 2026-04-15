use eframe::egui;

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

fn launch_title_wave_offset(time_seconds: f32, char_index: usize) -> f32 {
    let phase = time_seconds * 7.2 - char_index as f32 * 0.55;
    -(phase.sin().max(0.0) * 11.0)
}

fn draw_launching_title(
    painter: &egui::Painter,
    text: &str,
    pos: egui::Pos2,
    font_id: &egui::FontId,
    text_color: egui::Color32,
    time_seconds: f32,
) {
    let outline_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200);
    let d = 0.8_f32;
    let mut cursor_x = pos.x;

    for (char_index, ch) in text.chars().enumerate() {
        let glyph = ch.to_string();
        let galley = painter.layout_no_wrap(glyph.clone(), font_id.clone(), text_color);
        let outline_galley = painter.layout_no_wrap(glyph.clone(), font_id.clone(), outline_color);
        let char_pos = egui::pos2(
            cursor_x,
            pos.y + launch_title_wave_offset(time_seconds, char_index),
        );

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
            painter.galley(char_pos + off, outline_galley.clone());
        }
        painter.galley(char_pos, galley.clone());
        cursor_x += galley.size().x;
    }
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
) {
    let alpha = (255.0 * opacity.clamp(0.0, 1.0)).round() as u8;
    if alpha == 0 {
        return;
    }

    let tag_font = egui::FontId::new(
        (title_size.y * 0.42).clamp(11.0, 14.0),
        egui::FontFamily::Name("Bold".into()),
    );
    let text_color = egui::Color32::from_rgba_unmultiplied(18, 18, 18, alpha);
    let galley = painter.layout_no_wrap(text.to_owned(), tag_font, text_color);
    let padding_x = 11.0;
    let padding_y = 4.0;
    let tag_rect = egui::Rect::from_min_size(
        egui::pos2(title_pos.x + title_size.x + 14.0, title_pos.y + title_size.y * 0.5 - galley.size().y * 0.5 - padding_y),
        egui::vec2(galley.size().x + padding_x * 2.0, galley.size().y + padding_y * 2.0),
    );

    painter.rect_filled(
        tag_rect,
        egui::Rounding::same((tag_rect.height() * 0.5).min(10.0)),
        egui::Color32::from_rgba_unmultiplied(228, 228, 220, ((alpha as f32) * 0.72).round() as u8),
    );
    painter.galley(
        egui::pos2(tag_rect.min.x + padding_x, tag_rect.min.y + padding_y),
        galley,
    );
}

pub struct HintIcons {
    pub btn_a: egui::TextureHandle,
    pub btn_b: egui::TextureHandle,
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

pub fn load_hint_icons(ctx: &egui::Context) -> Option<HintIcons> {
    let btn_a = png_bytes_to_texture(
        ctx,
        include_bytes!("icons/Xbox/T_X_A_White_Alt.png"),
        "icon_btn_a",
    )?;
    let btn_b = png_bytes_to_texture(
        ctx,
        include_bytes!("icons/Xbox/T_X_B_White_Alt.png"),
        "icon_btn_b",
    )?;
    let dpad_down = png_bytes_to_texture(
        ctx,
        include_bytes!("icons/Xbox/T_X_Dpad_Down_Alt.png"),
        "icon_dpad_down",
    )?;
    Some(HintIcons {
        btn_a,
        btn_b,
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
) {
    let screen = ctx.screen_rect();
    let bg_painter = ctx.layer_painter(egui::LayerId::background());
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let base_alpha: f32 = 60.0;
    let hero_ratio = 1240.0 / 3840.0;

    // Solid dark background
    bg_painter.rect_filled(screen, egui::Rounding::ZERO, egui::Color32::from_rgb(18, 18, 18));

    // Image rect: fill screen width, pin to top, keep aspect ratio
    let top_rect = |tex: &egui::TextureHandle, dx: f32| -> egui::Rect {
        let tex_size = tex.size_vec2();
        let scale = screen.width() / tex_size.x;
        let img_h = tex_size.y * scale;
        egui::Rect::from_min_size(
            egui::pos2(screen.min.x + dx, screen.min.y),
            egui::vec2(screen.width(), img_h),
        )
    };

    let fallback_hero_rect = |dx: f32| -> egui::Rect {
        egui::Rect::from_min_size(
            egui::pos2(screen.min.x + dx, screen.min.y),
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
    games: &[crate::steam::Game],
    selected: usize,
    select_anim: f32,
    _achievement_panel_anim: f32,
    scroll_offset: f32,
    game_icons: &std::collections::HashMap<u32, egui::TextureHandle>,
    loading_index: Option<usize>,
    _achievement_panel_active: bool,
    achievement_summary_for_selected: Option<&crate::steam::AchievementSummary>,
    achievement_summary_reveal_for_selected: f32,
) {
    let base_icon_size: f32 = 92.0;
    let selected_icon_size: f32 = 160.0;
    let selected_icon_extra = selected_icon_size - base_icon_size;
    let time_seconds = ui.ctx().input(|input| input.time) as f32;

    let panel_rect = ui.available_rect_before_wrap();
    let padding = 50.0;
    let padded_rect = panel_rect.shrink(padding);

    let selected_size = 30.0;
    let base_size = 18.0;
    let column_spacing = 112.0;

    let hero_ratio = 1240.0 / 3840.0;
    let img_bottom = panel_rect.min.y + panel_rect.width() * hero_ratio;
    let content_top = img_bottom + 32.0;
    let anchor_x = padded_rect.min.x + 24.0;
    let painter = ui.painter();

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
        let is_launching = loading_index == Some(i);
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

        let icon_size = base_icon_size + selected_icon_extra * icon_focus_t;
        let meta_text_width = (icon_size + 58.0).max(160.0);
        let item_left = x_pos;
        let text_x = item_left;

        let font_id = if is_selected {
            egui::FontId::new(font_size, egui::FontFamily::Name("Bold".into()))
        } else {
            egui::FontId::proportional(font_size)
        };
        let display_name = g.name.clone();
        let galley = if is_selected {
            Some(painter.layout_no_wrap(display_name.clone(), font_id.clone(), text_color))
        } else {
            None
        };

        // Playtime and achievement progress
        let playtime_str = if g.playtime_minutes >= 60 {
            let hours = g.playtime_minutes as f32 / 60.0;
            let s = format!("{:.1}", hours);
            let s = s.trim_end_matches(".0");
            format!("{} hrs", s)
        } else if g.playtime_minutes > 0 {
            format!("{} min", g.playtime_minutes)
        } else {
            String::new()
        };

        let has_playtime_meta = !playtime_str.is_empty();
        let mut achievement_meta_text: Option<String> = None;
        let mut has_achievement_meta = false;
        if is_selected {
            if let Some(summary) = achievement_summary_for_selected {
                if summary.total > 0 {
                    let ach_text = if let Some(u) = summary.unlocked {
                        format!("{}/{} achievements", u, summary.total)
                    } else {
                        format!("--/{} achievements", summary.total)
                    };
                    achievement_meta_text = Some(ach_text);
                    has_achievement_meta = true;
                }
            }
        }
        let achievement_meta_reveal = if is_selected && has_achievement_meta {
            achievement_summary_reveal_for_selected.clamp(0.0, 1.0)
        } else {
            1.0
        };

        // Subtitle uses fade-in only; keep size and position stable.
        let pt_font_size = selected_size * 0.5;
        let pt_font = egui::FontId::proportional(pt_font_size);
        let pt_color = egui::Color32::from_rgba_unmultiplied(
            180,
            180,
            190,
            (140.0 * meta_t) as u8,
        );
        let playtime_galley = if is_selected && meta_t > 0.0 && has_playtime_meta {
            Some(painter.layout_no_wrap(playtime_str, pt_font.clone(), pt_color))
        } else {
            None
        };
        let achievement_color = egui::Color32::from_rgba_unmultiplied(
            180,
            180,
            190,
            (140.0 * meta_t * achievement_meta_reveal) as u8,
        );
        let achievement_galley = if is_selected && meta_t > 0.0 && has_achievement_meta {
            achievement_meta_text.map(|text| {
                let prefixed = if has_playtime_meta {
                    format!("  •  {}", text)
                } else {
                    text
                };
                build_wrapped_galley(ui, prefixed, pt_font, achievement_color, meta_text_width)
            })
        } else {
            None
        };
        let text_y = content_top + icon_size + 14.0;

        if let Some(app_id) = g.app_id {
            if let Some(icon_tex) = game_icons.get(&app_id) {
                let icon_alpha = 255;
                let icon_tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, icon_alpha);
                let icon_rect = egui::Rect::from_min_size(
                    egui::pos2(item_left, content_top),
                    egui::vec2(icon_size, icon_size),
                );
                draw_game_icon(painter, icon_tex, icon_rect, icon_tint);
            }
        }

        if let Some(galley) = galley {
            let normal_title_pos = egui::pos2(text_x, text_y);
            if is_launching {
                draw_launching_title(
                    painter,
                    &display_name,
                    normal_title_pos,
                    &font_id,
                    text_color,
                    time_seconds,
                );
            } else {
                let outline_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200);
                let outline_galley = painter.layout_no_wrap(display_name, font_id, outline_color);
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
                    painter.galley(normal_title_pos + off, outline_galley.clone());
                }

                painter.galley(normal_title_pos, galley.clone());
            }

            if let Some(tag_text) = dlss_tag_text(g) {
                draw_title_tag(painter, &tag_text, normal_title_pos, galley.size(), 0.94);
            }

            if playtime_galley.is_some() || achievement_galley.is_some() {
                let meta_pos = egui::pos2(text_x, text_y + galley.size().y + 2.0);
                let mut pt_x = meta_pos.x;
                if let Some(pt_g) = playtime_galley {
                    painter.galley(egui::pos2(pt_x, meta_pos.y), pt_g.clone());
                    pt_x += pt_g.size().x;
                }
                if let Some(ach_g) = achievement_galley {
                    painter.galley(egui::pos2(pt_x, meta_pos.y), ach_g);
                }
            }
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
    let [tex_w, tex_h] = texture.size();
    let uv_rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    
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
    painter.image(texture.id(), draw_rect, uv_rect, fade_tint);
}

pub fn draw_achievement_page(
    ui: &mut egui::Ui,
    game: &crate::steam::Game,
    summary: Option<&crate::steam::AchievementSummary>,
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
) {
    let panel_rect = ui.available_rect_before_wrap();
    let padding = 50.0;
    let padded_rect = panel_rect.shrink(padding);
    let painter = ui.painter();
    let panel_t = smoothstep01(achievement_panel_anim);
    let enter_offset_y = lerp_f32(-14.0, 0.0, panel_t);
    let content_top = padded_rect.min.y + 18.0;
    let title_font_size = 18.0 + (30.0 - 18.0) * smoothstep01(game_select_anim);
    let title_font = egui::FontId::new(title_font_size, egui::FontFamily::Name("Bold".into()));
    let title_galley = painter.layout_no_wrap(game.name.clone(), title_font.clone(), egui::Color32::WHITE);
    let playtime_str = if game.playtime_minutes >= 60 {
        let hours = game.playtime_minutes as f32 / 60.0;
        let s = format!("{:.1}", hours);
        let s = s.trim_end_matches(".0");
        format!("{} hrs", s)
    } else if game.playtime_minutes > 0 {
        format!("{} min", game.playtime_minutes)
    } else {
        String::new()
    };
    let mut meta_parts: Vec<String> = Vec::new();
    if !playtime_str.is_empty() {
        meta_parts.push(playtime_str);
    }
    if let Some(summary) = summary {
        if summary.total > 0 {
            let ach_text = if let Some(unlocked) = summary.unlocked {
                format!("{}/{} achievements", unlocked, summary.total)
            } else {
                format!("--/{} achievements", summary.total)
            };
            meta_parts.push(ach_text);
        }
    }
    let meta_galley = if meta_parts.is_empty() {
        None
    } else {
        Some(painter.layout_no_wrap(
            meta_parts.join("  •  "),
            egui::FontId::proportional(15.0),
            egui::Color32::from_rgba_unmultiplied(
                180,
                180,
                190,
                (140.0 * achievement_summary_reveal_for_selected.clamp(0.0, 1.0)) as u8,
            ),
        ))
    };
    let header_text_x = padded_rect.min.x + 24.0;
    let meta_height = meta_galley.as_ref().map(|galley| galley.size().y).unwrap_or(0.0);
    let text_block_height = title_galley.size().y + if meta_height > 0.0 { 6.0 + meta_height } else { 0.0 };
    let text_top = content_top + 64.0 - text_block_height;
    let title_base_pos = egui::pos2(header_text_x, text_top);
    let meta_base_pos = egui::pos2(header_text_x, text_top + title_galley.size().y + 6.0);
    let header_bottom = meta_galley
        .as_ref()
        .map(|galley| meta_base_pos.y + galley.size().y)
        .unwrap_or(title_base_pos.y + title_galley.size().y);
    let header_base_rect = egui::Rect::from_min_max(
        egui::pos2(padded_rect.min.x + 8.0, content_top),
        egui::pos2(padded_rect.max.x - 8.0, header_bottom + 26.0),
    );
    let list_base_rect = egui::Rect::from_min_max(
        egui::pos2(padded_rect.min.x + 8.0, header_base_rect.max.y + 24.0),
        egui::pos2(padded_rect.max.x - 8.0, padded_rect.max.y - 52.0),
    );
    let content_offset = egui::vec2(0.0, enter_offset_y);
    let list_rect = list_base_rect.translate(content_offset);
    let title_pos = title_base_pos + content_offset;
    let meta_pos = meta_base_pos + content_offset;

    painter.rect_filled(
        panel_rect,
        egui::Rounding::ZERO,
        egui::Color32::from_rgba_unmultiplied(18, 18, 18, (255.0 * panel_t) as u8),
    );

    let outline_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 160);
    let outline_galley = painter.layout_no_wrap(game.name.clone(), title_font, outline_color);
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
    painter.galley(title_pos, title_galley.clone());
    if let Some(tag_text) = dlss_tag_text(game) {
        draw_title_tag(painter, &tag_text, title_pos, title_galley.size(), panel_t);
    }
    if let Some(meta_galley) = meta_galley {
        painter.galley(meta_pos, meta_galley);
    }

    painter.rect_filled(
        list_rect,
        egui::Rounding::same(8.0),
        egui::Color32::from_rgb(14, 14, 14),
    );

    let Some(summary) = summary else {
        let empty_galley = painter.layout_no_wrap(
            "No achievement data available".to_string(),
            egui::FontId::proportional(18.0),
            egui::Color32::from_rgba_unmultiplied(194, 198, 208, 170),
        );
        painter.galley(
            egui::pos2(
                list_rect.center().x - empty_galley.size().x * 0.5,
                list_rect.center().y - empty_galley.size().y * 0.5,
            ),
            empty_galley,
        );
        return;
    };

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
            .unwrap_or("No description");
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
        if let Some(tex) = icon_key.and_then(|key| achievement_icon_cache.get(key)) {
            let reveal = icon_key
                .and_then(|key| achievement_icon_reveal.get(key).copied())
                .unwrap_or(1.0);
            draw_achievement_icon(
                &list_painter,
                tex,
                icon_rect,
                egui::Color32::WHITE,
                reveal,
            );
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
}

pub fn draw_hint_bar(
    ui: &mut egui::Ui,
    icons: &HintIcons,
    achievement_panel_active: bool,
    can_open_achievement_panel: bool,
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

    let g_back = painter.layout_no_wrap("Back".to_string(), hint_font.clone(), hint_color);
    let g_quit = painter.layout_no_wrap("Quit".to_string(), hint_font.clone(), hint_color);
    let b_label_reserve = g_back.size().x.max(g_quit.size().x);
    let b_icon_x = padded_rect.max.x - b_label_reserve - 6.0 - action_icon_h;
    let b_label_x = b_icon_x + action_icon_h + 6.0;

    if achievement_panel_active {
        let g_scroll = painter.layout_no_wrap("Scroll".to_string(), hint_font.clone(), hint_color);
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

    let g_launch = painter.layout_no_wrap("Start".to_string(), hint_font.clone(), hint_color);
    let g_achievements = painter.layout_no_wrap(
        "Achievements".to_string(),
        hint_font.clone(),
        hint_color,
    );
    let launch_group_w = action_icon_h + 6.0 + g_launch.size().x;
    let launch_x = b_icon_x - 20.0 - launch_group_w;

    if can_open_achievement_panel {
        let achievements_group_w = icon_h + 6.0 + g_achievements.size().x;
        let achievements_x = launch_x - 20.0 - achievements_group_w;
        draw_icon(painter, &icons.dpad_down, achievements_x, icon_h);

        let gy = hint_y + (row_h - g_achievements.size().y) * 0.5;
        painter.galley(
            egui::pos2(achievements_x + icon_h + 6.0, gy),
            g_achievements,
        );
    }

    draw_icon(painter, &icons.btn_a, launch_x, action_icon_h);

    let gy = hint_y + (row_h - g_launch.size().y) * 0.5;
    painter.galley(
        egui::pos2(launch_x + action_icon_h + 6.0, gy),
        g_launch,
    );

    draw_icon(painter, &icons.btn_b, b_icon_x, action_icon_h);

    // "Quit"
    let gy = hint_y + (row_h - g_quit.size().y) * 0.5;
    painter.galley(egui::pos2(b_label_x, gy), g_quit);
}
