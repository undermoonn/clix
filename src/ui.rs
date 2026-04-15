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

fn lerp_pos2(start: egui::Pos2, end: egui::Pos2, t: f32) -> egui::Pos2 {
    egui::pos2(lerp_f32(start.x, end.x, t), lerp_f32(start.y, end.y, t))
}

fn selected_game_icon_rect(
    panel_rect: egui::Rect,
    padded_rect: egui::Rect,
    selected: usize,
    _select_anim: f32,
    scroll_offset: f32,
) -> egui::Rect {
    let base_icon_size: f32 = 92.0;
    let selected_icon_size: f32 = 160.0;
    let selected_icon_extra = selected_icon_size - base_icon_size;
    let column_spacing = 112.0;
    let hero_ratio = 1240.0 / 3840.0;
    let img_bottom = panel_rect.min.y + panel_rect.width() * hero_ratio;
    let content_top = img_bottom + 32.0;
    let anchor_x = padded_rect.min.x + 24.0;
    let offset_f = selected as f32 - scroll_offset;
    let selected_dist = offset_f.abs();
    let sign = if offset_f >= 0.0 { 1.0 } else { -1.0 };
    let icon_focus_t = smoothstep01((1.0 - selected_dist).clamp(0.0, 1.0));
    let right_side_compensation = if offset_f > 0.0 {
        selected_icon_extra * smoothstep01(offset_f.clamp(0.0, 1.0))
    } else {
        0.0
    };
    let x_pos = anchor_x + sign * selected_dist * column_spacing + right_side_compensation;
    let icon_size = base_icon_size + selected_icon_extra * icon_focus_t;

    egui::Rect::from_min_size(
        egui::pos2(x_pos, content_top),
        egui::vec2(icon_size, icon_size),
    )
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
    achievement_panel_anim: f32,
    scroll_offset: f32,
    game_icons: &std::collections::HashMap<u32, egui::TextureHandle>,
    loading_index: Option<usize>,
    achievement_panel_active: bool,
    achievement_summary_for_selected: Option<&crate::steam::AchievementSummary>,
    achievement_summary_reveal_for_selected: f32,
) {
    let base_icon_size: f32 = 92.0;
    let selected_icon_size: f32 = 160.0;
    let selected_icon_extra = selected_icon_size - base_icon_size;

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
    let panel_t = smoothstep01(achievement_panel_anim);
    let others_alpha_t = 1.0 - panel_t;

    for (i, g) in games.iter().enumerate() {
        let offset_f = i as f32 - scroll_offset;
        let is_selected = i == selected;
        if achievement_panel_active && !is_selected && others_alpha_t <= 0.001 {
            continue;
        }

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
        let font_size = if is_selected {
            base_size + (selected_size - base_size) * selection_t
        } else {
            base_size
        };

        let item_alpha_t = if is_selected { 1.0 } else { others_alpha_t };
        if item_alpha_t <= 0.001 {
            continue;
        }

        let text_alpha = if is_selected {
            255
        } else {
            (220.0 * item_alpha_t) as u8
        };
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
        let mut display_name = g.name.clone();
        if loading_index == Some(i) {
            display_name.push_str(" ...");
        }
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
                let icon_alpha = if is_selected {
                    255
                } else {
                    (255.0 * item_alpha_t) as u8
                };
                let icon_tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, icon_alpha);
                let icon_rect = egui::Rect::from_min_size(
                    egui::pos2(item_left, content_top),
                    egui::vec2(icon_size, icon_size),
                );
                draw_game_icon(painter, icon_tex, icon_rect, icon_tint);
            }
        }

        if let Some(galley) = galley {
            let outline_alpha = (200.0 * (1.0 - 0.2 * panel_t)) as u8;
            let outline_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, outline_alpha);
            let header_icon_rect = selected_game_icon_rect(
                panel_rect,
                padded_rect,
                selected,
                select_anim,
                scroll_offset,
            );
            let active_text_x = header_icon_rect.max.x + 18.0;
            let normal_title_pos = egui::pos2(text_x, text_y);
            let active_title_pos = egui::pos2(active_text_x, header_icon_rect.min.y + 6.0);
            let title_pos = lerp_pos2(normal_title_pos, active_title_pos, panel_t);
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
                painter.galley(title_pos + off, outline_galley.clone());
            }

            painter.galley(title_pos, galley.clone());

            if playtime_galley.is_some() || achievement_galley.is_some() {
                let normal_meta_pos = egui::pos2(text_x, text_y + galley.size().y + 2.0);
                let active_meta_pos = egui::pos2(
                    active_text_x,
                    active_title_pos.y + galley.size().y + 6.0,
                );
                let meta_pos = lerp_pos2(normal_meta_pos, active_meta_pos, panel_t);
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

pub fn draw_achievement_panel(
    ui: &mut egui::Ui,
    game_name: &str,
    selected_game_playtime_minutes: u32,
    selected_game_index: usize,
    summary: Option<&crate::steam::AchievementSummary>,
    achievement_summary_reveal_for_selected: f32,
    _loading: bool,
    selected_index: usize,
    achievement_select_anim: f32,
    game_select_anim: f32,
    achievement_panel_anim: f32,
    game_scroll_offset: f32,
    scroll_offset: f32,
    active: bool,
    achievement_icon_cache: &std::collections::HashMap<String, egui::TextureHandle>,
    achievement_icon_reveal: &std::collections::HashMap<String, f32>,
) {
    let panel_rect = ui.available_rect_before_wrap();
    let padding = 50.0;
    let padded_rect = panel_rect.shrink(padding);
    let selected_icon_size: f32 = 160.0;
    let hero_ratio = 1240.0 / 3840.0;
    let img_bottom = panel_rect.min.y + panel_rect.width() * hero_ratio;
    let clip_y = img_bottom + 28.0;
    let left_x = padded_rect.min.x + 20.0;
    let header_icon_size = selected_icon_size;
    let header_icon_rect = selected_game_icon_rect(
        panel_rect,
        padded_rect,
        selected_game_index,
        game_select_anim,
        game_scroll_offset,
    );
    let icon_column_right = header_icon_rect.max.x;
    let title_font_size = 18.0 + (30.0 - 18.0) * smoothstep01(game_select_anim);
    let title_font = egui::FontId::new(title_font_size, egui::FontFamily::Name("Bold".into()));
    let title_galley = ui.painter().layout_no_wrap(game_name.to_owned(), title_font, egui::Color32::WHITE);
    let playtime_str = if selected_game_playtime_minutes >= 60 {
        let hours = selected_game_playtime_minutes as f32 / 60.0;
        let s = format!("{:.1}", hours);
        let s = s.trim_end_matches(".0");
        format!("{} hrs", s)
    } else if selected_game_playtime_minutes > 0 {
        format!("{} min", selected_game_playtime_minutes)
    } else {
        String::new()
    };
    let mut meta_parts: Vec<String> = Vec::new();
    if !playtime_str.is_empty() {
        meta_parts.push(playtime_str);
    }
    if let Some(summary) = summary {
        if summary.total > 0 {
            let ach_text = if let Some(u) = summary.unlocked {
                format!("{}/{} achievements", u, summary.total)
            } else {
                format!("--/{} achievements", summary.total)
            };
            meta_parts.push(ach_text);
        }
    }
    let meta_galley = if meta_parts.is_empty() {
        None
    } else {
        Some(ui.painter().layout_no_wrap(
            meta_parts.join("  •  "),
            egui::FontId::proportional(15.0),
            egui::Color32::from_rgba_unmultiplied(
                180,
                180,
                190,
                (140.0 * smoothstep01((game_select_anim - 0.18) / 0.82)
                    * achievement_summary_reveal_for_selected.clamp(0.0, 1.0)) as u8,
            ),
        ))
    };
    let panel_t = smoothstep01(achievement_panel_anim);
    let normal_title_top = clip_y + 10.0 + header_icon_size + 14.0;
    let normal_meta_top = normal_title_top + title_galley.size().y + 2.0;
    let active_title_top = clip_y + 10.0 + 6.0;
    let active_meta_top = active_title_top + title_galley.size().y + 6.0;
    let text_bottom = if let Some(meta_galley) = &meta_galley {
        lerp_f32(
            normal_meta_top + meta_galley.size().y,
            active_meta_top + meta_galley.size().y,
            panel_t,
        )
    } else {
        lerp_f32(
            normal_title_top + title_galley.size().y,
            active_title_top + title_galley.size().y,
            panel_t,
        )
    };
    let list_top = (clip_y + 10.0 + header_icon_size + 26.0).max(text_bottom + 26.0);
    let rect = egui::Rect::from_min_max(
        egui::pos2(left_x, list_top),
        egui::pos2(padded_rect.max.x - 18.0, padded_rect.max.y),
    );

    let activity_alpha = if active { 1.0 } else { 0.45 };

    match summary {
        Some(s) => {
            let list_rect = egui::Rect::from_min_max(
                egui::pos2(rect.min.x, rect.min.y),
                egui::pos2(rect.max.x - 10.0, rect.max.y),
            );
            let list_painter = ui.painter().with_clip_rect(list_rect);
            let row_spacing = 60.0;
            let visible_side = (list_rect.height() / row_spacing).ceil() as i32 + 3;
            let center_y = list_rect.min.y + 20.0;
            let selection_t = 1.0 - (1.0 - achievement_select_anim) * (1.0 - achievement_select_anim);
            let selected_description_galley = s.items.get(selected_index).and_then(|item| {
                item.description.as_ref().and_then(|description| {
                    let text = description.trim();
                    if text.is_empty() {
                        return None;
                    }
                    let desc_font = egui::FontId::proportional(12.0 + 1.3 * selection_t);
                    let desc_color = egui::Color32::from_rgba_unmultiplied(
                        162,
                        168,
                        180,
                        (214.0 * activity_alpha * selection_t) as u8,
                    );
                    let desc_width = (list_rect.max.x - (icon_column_right + 14.0) - 10.0).max(220.0);
                    Some(build_wrapped_galley(
                        ui,
                        text.to_string(),
                        desc_font,
                        desc_color,
                        desc_width,
                    ))
                })
            });
            let selected_description_extra = selected_description_galley
                .as_ref()
                .map(|galley| (galley.size().y + 14.0) * selection_t)
                .unwrap_or(0.0);
            for (idx, item) in s.items.iter().enumerate() {
                let offset_f = idx as f32 - scroll_offset;
                if offset_f < -(visible_side as f32) || offset_f > visible_side as f32 {
                    continue;
                }

                let is_selected = idx == selected_index;
                let dist = offset_f.abs();
                let sign = if offset_f >= 0.0 { 1.0 } else { -1.0 };
                let mut y_pos = center_y + sign * dist * row_spacing * (1.0 - dist * 0.03).max(0.76);
                let below_selected_t = smoothstep01((offset_f + 0.45) / 0.9);
                y_pos += selected_description_extra * below_selected_t;
                if y_pos < list_rect.min.y - row_spacing || y_pos > list_rect.max.y + row_spacing {
                    continue;
                }

                let alpha_factor = (1.0 - dist * 0.15).max(0.24);
                let t = if is_selected { selection_t } else { 0.0 };
                let text_size = 15.0 + 2.2 * alpha_factor + 1.3 * t;
                let icon_size = 36.0 + 10.0 * t;
                let icon_left = icon_column_right - icon_size;
                let text_x = icon_column_right + 14.0;
                let text_width = (list_rect.max.x - text_x - 10.0).max(180.0);
                let row_top = y_pos - (icon_size * 0.5);
                let icon_rect = egui::Rect::from_min_size(
                    egui::pos2(icon_left, row_top),
                    egui::vec2(icon_size, icon_size),
                );

                let name = item.display_name.as_deref().unwrap_or(&item.api_name);
                let mut title = String::from(name);
                if let Some(p) = item.global_percent {
                    title.push_str(&format!("  ({:.1}%)", p));
                }
                let color = egui::Color32::from_rgba_unmultiplied(
                    220,
                    224,
                    236,
                    (236.0 * alpha_factor * activity_alpha) as u8,
                );
                let title_font = if active && is_selected {
                    egui::FontId::new(text_size, egui::FontFamily::Name("Bold".into()))
                } else {
                    egui::FontId::proportional(text_size)
                };
                let status_font = egui::FontId::proportional((text_size * 0.76).max(11.0));
                let status_color = egui::Color32::from_rgba_unmultiplied(
                    178,
                    184,
                    196,
                    (196.0 * alpha_factor * activity_alpha) as u8,
                );
                let title_galley = build_wrapped_galley(ui, title, title_font, color, text_width);
                let status_galley = format_achievement_status(item.unlocked, item.unlock_time)
                    .map(|status| {
                        build_wrapped_galley(
                            ui,
                            status,
                            status_font,
                            status_color,
                            text_width,
                        )
                    });
                let text_top = row_top;
                let status_spacing = if status_galley.is_some() { 12.0 } else { 0.0 };
                let status_x = text_x + title_galley.size().x + status_spacing;
                let total_height = title_galley
                    .size()
                    .y
                    .max(status_galley.as_ref().map(|galley| galley.size().y).unwrap_or(0.0));
                let icon_key = match item.unlocked {
                    Some(true) => item.icon_url.as_ref().or(item.icon_gray_url.as_ref()),
                    _ => item.icon_gray_url.as_ref().or(item.icon_url.as_ref()),
                };
                if let Some(tex) = icon_key.and_then(|k| achievement_icon_cache.get(k)) {
                    let reveal = icon_key
                        .and_then(|k| achievement_icon_reveal.get(k).copied())
                        .unwrap_or(1.0);
                    draw_achievement_icon(
                        &list_painter,
                        tex,
                        icon_rect,
                        egui::Color32::from_rgba_unmultiplied(
                            255,
                            255,
                            255,
                            (255.0 * alpha_factor * activity_alpha) as u8,
                        ),
                        reveal,
                    );
                } else {
                    let fill = match item.unlocked {
                        Some(true) => egui::Color32::from_rgba_unmultiplied(86, 172, 132, (220.0 * alpha_factor * activity_alpha) as u8),
                        Some(false) => egui::Color32::from_rgba_unmultiplied(110, 116, 124, (200.0 * alpha_factor * activity_alpha) as u8),
                        None => egui::Color32::from_rgba_unmultiplied(90, 98, 112, (180.0 * alpha_factor * activity_alpha) as u8),
                    };
                    list_painter.rect_filled(
                        icon_rect,
                        egui::Rounding::ZERO,
                        fill,
                    );
                }
                list_painter.galley(egui::pos2(text_x, text_top), title_galley.clone());
                if let Some(status_galley) = status_galley {
                    list_painter.galley(
                        egui::pos2(
                            status_x,
                            text_top + (title_galley.size().y - status_galley.size().y) * 0.5,
                        ),
                        status_galley,
                    );
                }
                if is_selected {
                    if let Some(description_galley) = &selected_description_galley {
                        list_painter.galley(
                            egui::pos2(text_x, text_top + total_height + 10.0),
                            description_galley.clone(),
                        );
                    }
                }
            }
        }
        None => {}
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
    let row_h = icon_h;
    let hint_y = padded_rect.max.y - 10.0;
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

    let painter = ui.painter();
    let draw_icon = |painter: &egui::Painter, tex: &egui::TextureHandle, x: f32| {
        painter.image(
            tex.id(),
            egui::Rect::from_min_size(
                egui::pos2(x, hint_y + (row_h - icon_h) * 0.5),
                egui::vec2(icon_h, icon_h),
            ),
            uv,
            egui::Color32::WHITE,
        );
    };

    if achievement_panel_active {
        let g_back = painter.layout_no_wrap("Back".to_string(), hint_font, hint_color);
        let total_w = icon_h + 6.0 + g_back.size().x;
        let mut hx = padded_rect.max.x - total_w;

        draw_icon(painter, &icons.btn_b, hx);
        hx += icon_h + 6.0;

        let gy = hint_y + (row_h - g_back.size().y) * 0.5;
        painter.galley(egui::pos2(hx, gy), g_back);
        return;
    }

    // Measure total width first (right-aligned)
    let g_launch = painter.layout_no_wrap("Start".to_string(), hint_font.clone(), hint_color);
    let g_quit = painter.layout_no_wrap("Quit".to_string(), hint_font.clone(), hint_color);
    let g_achievements = painter.layout_no_wrap(
        "Achievements".to_string(),
        hint_font.clone(),
        hint_color,
    );
    let total_w = if can_open_achievement_panel {
        icon_h + 6.0 + g_launch.size().x
            + 20.0
            + icon_h + 6.0 + g_achievements.size().x
            + 20.0
            + icon_h + 6.0 + g_quit.size().x
    } else {
        icon_h + 6.0 + g_launch.size().x + 20.0 + icon_h + 6.0 + g_quit.size().x
    };
    let mut hx = padded_rect.max.x - total_w;

    if can_open_achievement_panel {
        draw_icon(painter, &icons.dpad_down, hx);
        hx += icon_h + 6.0;

        let gy = hint_y + (row_h - g_achievements.size().y) * 0.5;
        let g_width = g_achievements.size().x;
        painter.galley(egui::pos2(hx, gy), g_achievements);
        hx += g_width + 20.0;
    }

    // A button
    draw_icon(painter, &icons.btn_a, hx);
    hx += icon_h + 6.0;

    // "Launch"
    let gy = hint_y + (row_h - g_launch.size().y) * 0.5;
    let g_width = g_launch.size().x;
    painter.galley(egui::pos2(hx, gy), g_launch);
    hx += g_width + 20.0;

    // B button
    draw_icon(painter, &icons.btn_b, hx);
    hx += icon_h + 6.0;

    // "Quit"
    let gy = hint_y + (row_h - g_quit.size().y) * 0.5;
    painter.galley(egui::pos2(hx, gy), g_quit);
}
