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

pub struct HintIcons {
    pub btn_a: egui::TextureHandle,
    pub btn_b: egui::TextureHandle,
    pub dpad_right: egui::TextureHandle,
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
    let dpad_right = png_bytes_to_texture(
        ctx,
        include_bytes!("icons/Xbox/T_X_Dpad_Right_Alt.png"),
        "icon_dpad_right",
    )?;
    Some(HintIcons {
        btn_a,
        btn_b,
        dpad_right,
    })
}

pub fn draw_background(
    ctx: &egui::Context,
    cover: &Option<(u32, egui::TextureHandle)>,
    cover_prev: &Option<(u32, egui::TextureHandle)>,
    cover_fade: f32,
    cover_nav_dir: f32,
) {
    let screen = ctx.screen_rect();
    let bg_painter = ctx.layer_painter(egui::LayerId::background());
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let base_alpha: f32 = 60.0;

    // Solid dark background
    bg_painter.rect_filled(screen, egui::Rounding::ZERO, egui::Color32::from_rgb(18, 18, 18));

    // Image rect: fill screen width, pin to top, keep aspect ratio
    let top_rect = |tex: &egui::TextureHandle, dy: f32| -> egui::Rect {
        let tex_size = tex.size_vec2();
        let scale = screen.width() / tex_size.x;
        let img_h = tex_size.y * scale;
        egui::Rect::from_min_size(
            egui::pos2(screen.min.x, screen.min.y + dy),
            egui::vec2(screen.width(), img_h),
        )
    };

    let slide_distance = 4.0;
    let ease_t = 1.0 - (1.0 - cover_fade) * (1.0 - cover_fade);

    // Previous cover (fading out)
    if cover_fade < 1.0 {
        if let Some((_id, tex)) = cover_prev {
            let alpha = (base_alpha * (1.0 - cover_fade)) as u8;
            let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
            bg_painter.image(tex.id(), top_rect(tex, 0.0), uv, tint);
        }
    }

    // Current cover (fading in, sliding in)
    if let Some((_id, tex)) = cover {
        let alpha = (base_alpha * cover_fade) as u8;
        let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
        let dy = cover_nav_dir * slide_distance * (1.0 - ease_t);
        bg_painter.image(tex.id(), top_rect(tex, dy), uv, tint);
    }
}

pub fn draw_game_list(
    ui: &mut egui::Ui,
    games: &[crate::steam::Game],
    selected: usize,
    select_anim: f32,
    scroll_offset: f32,
    game_icons: &std::collections::HashMap<u32, egui::TextureHandle>,
    loading_index: Option<usize>,
    achievement_panel_active: bool,
    achievement_summary_for_selected: Option<&crate::steam::AchievementSummary>,
) {
    let panel_rect = ui.available_rect_before_wrap();
    let padding = 50.0;
    let padded_rect = panel_rect.shrink(padding);

    let selected_size = 30.0;
    let base_size = 18.0;
    let row_spacing = 52.0;

    // Position list starting below the hero image
    let hero_ratio = 1240.0 / 3840.0;
    let img_bottom = panel_rect.min.y + panel_rect.width() * hero_ratio;
    let clip_y = img_bottom + 20.0; // items above this line are hidden
    let center_y = img_bottom + row_spacing + 30.0; // selected item: leave room for 1 item above
    let left_x = padded_rect.min.x + 20.0;

    let visible_above = 1;
    let remaining_below = panel_rect.max.y - center_y;
    let visible_below = (remaining_below / row_spacing).ceil() as usize + 1;
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

    let painter = ui.painter();

    for (i, g) in games.iter().enumerate() {
        let offset_f = i as f32 - scroll_offset;
        let is_selected = i == selected;
        if achievement_panel_active && !is_selected {
            continue;
        }
        if offset_f < -(visible_above as f32 + 1.0) || offset_f > (visible_below as f32 + 1.0) {
            continue;
        }

        let dist = offset_f.abs();
        let sign = if offset_f >= 0.0 { 1.0 } else { -1.0 };
        // Extra gap around selected item (smooth transition)
        let selected_gap = 16.0;
        let extra = offset_f.clamp(-1.0, 1.0) * selected_gap;
        let y_pos = center_y + sign * dist * row_spacing * (1.0 - dist * 0.03).max(0.7) + extra;

        // Skip items that would overlap the cover image area
        if y_pos < clip_y {
            continue;
        }

        let alpha_factor = (1.0 - dist * 0.13).max(0.0);
        let dim_factor = if achievement_panel_active && !is_selected {
            0.35
        } else {
            1.0
        };
        let font_size = if is_selected {
            let t = 1.0 - (1.0 - select_anim) * (1.0 - select_anim);
            base_size + (selected_size - base_size) * t
        } else {
            base_size
        };

        let text_alpha = if is_selected {
            255
        } else {
            (220.0 * alpha_factor * dim_factor) as u8
        };
        let text_color = if is_selected {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255)
        } else {
            egui::Color32::from_rgba_unmultiplied(200, 200, 210, text_alpha)
        };

        // Compute icon offset
        let selected_icon_size = selected_size + 26.0;
        let base_icon_size = base_size + 4.0;
        let icon_size = if is_selected {
            let t = 1.0 - (1.0 - select_anim) * (1.0 - select_anim);
            base_icon_size + (selected_icon_size - base_icon_size) * t
        } else {
            base_icon_size
        };
        let icon_gap = 8.0;
        let mut text_x = left_x;
        let has_icon = g.app_id.and_then(|id| game_icons.get(&id)).is_some();
        if has_icon {
            text_x = left_x + icon_size + icon_gap;
        }

        let font_id = if is_selected {
            egui::FontId::new(font_size, egui::FontFamily::Name("Bold".into()))
        } else {
            egui::FontId::proportional(font_size)
        };
        let mut display_name = g.name.clone();
        if loading_index == Some(i) {
            display_name.push_str(" ...");
        }
        let galley = painter.layout_no_wrap(display_name.clone(), font_id.clone(), text_color);

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

        let mut meta_parts: Vec<String> = Vec::new();
        if !playtime_str.is_empty() {
            meta_parts.push(playtime_str);
        }
        if is_selected {
            if let Some(summary) = achievement_summary_for_selected {
                if summary.total > 0 {
                    let ach_text = if let Some(u) = summary.unlocked {
                        format!("{}/{} achievements", u, summary.total)
                    } else {
                        format!("--/{} achievements", summary.total)
                    };
                    meta_parts.push(ach_text);
                }
            }
        }
        let meta_str = meta_parts.join("  •  ");

        // Measure playtime galley (only shown when selected)
        let pt_font_size = font_size * 0.5;
        let pt_font = egui::FontId::proportional(pt_font_size);
        let pt_color = egui::Color32::from_rgba_unmultiplied(180, 180, 190, 140);
        let pt_galley = if is_selected && !meta_str.is_empty() {
            Some(painter.layout_no_wrap(meta_str, pt_font, pt_color))
        } else {
            None
        };
        let pt_row_h = pt_galley.as_ref().map_or(0.0, |g| g.size().y + 2.0);

        // Layout: name + playtime stacked, centered on y_pos
        let total_h = galley.size().y + pt_row_h;
        let text_y = y_pos - total_h * 0.5;

        // Draw game icon
        if let Some(app_id) = g.app_id {
            if let Some(icon_tex) = game_icons.get(&app_id) {
                let icon_tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, text_alpha);
                let icon_rect = egui::Rect::from_min_size(
                    egui::pos2(left_x, y_pos - icon_size * 0.5),
                    egui::vec2(icon_size, icon_size),
                );
                painter.add(egui::Shape::Rect(egui::epaint::RectShape {
                    rect: icon_rect,
                    rounding: egui::Rounding::same(8.0),
                    fill: icon_tint,
                    stroke: egui::Stroke::NONE,
                    fill_texture_id: icon_tex.id(),
                    uv,
                }));
            }
        }

        // Text outline (2-pass for smooth stroke)
        let outline_alpha = if is_selected {
            200
        } else {
            (120.0 * alpha_factor * dim_factor) as u8
        };
        let outline_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, outline_alpha);
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
            painter.galley(egui::pos2(text_x, text_y) + off, outline_galley.clone());
        }

        // Foreground text
        painter.galley(egui::pos2(text_x, text_y), galley.clone());

        // Playtime text (below game name)
        if let Some(pt_g) = pt_galley {
            let pt_y = text_y + galley.size().y + 2.0;
            painter.galley(egui::pos2(text_x, pt_y), pt_g);
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
    let uv_rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let reveal = reveal.clamp(0.0, 1.0);
    let alpha = ((tint.a() as f32) * reveal).round() as u8;
    let fade_tint = egui::Color32::from_rgba_unmultiplied(tint.r(), tint.g(), tint.b(), alpha);
    painter.image(texture.id(), icon_rect, uv_rect, fade_tint);
}

pub fn draw_achievement_panel(
    ui: &mut egui::Ui,
    game_name: &str,
    selected_game_has_icon: bool,
    selected_game_playtime_minutes: u32,
    summary: Option<&crate::steam::AchievementSummary>,
    _loading: bool,
    selected_index: usize,
    achievement_select_anim: f32,
    scroll_offset: f32,
    game_select_anim: f32,
    active: bool,
    achievement_icon_cache: &std::collections::HashMap<String, egui::TextureHandle>,
    achievement_icon_reveal: &std::collections::HashMap<String, f32>,
) {
    let panel_rect = ui.available_rect_before_wrap();
    let padding = 50.0;
    let padded_rect = panel_rect.shrink(padding);
    let hero_ratio = 1240.0 / 3840.0;
    let img_bottom = panel_rect.min.y + panel_rect.width() * hero_ratio;
    let clip_y = img_bottom + 20.0;
    let panel_right = padded_rect.max.x - 18.0;
    let selected_size = 30.0;
    let base_size = 18.0;
    let selected_icon_size = selected_size + 26.0;
    let base_icon_size = base_size + 4.0;
    let left_x = padded_rect.min.x + 20.0;
    let game_t = 1.0 - (1.0 - game_select_anim) * (1.0 - game_select_anim);
    let game_font_size = base_size + (selected_size - base_size) * game_t;
    let game_icon_size = base_icon_size + (selected_icon_size - base_icon_size) * game_t;
    let game_text_x = if selected_game_has_icon {
        left_x + game_icon_size + 8.0
    } else {
        left_x
    };
    let game_title_font = egui::FontId::new(game_font_size, egui::FontFamily::Name("Bold".into()));
    let game_title_galley = ui
        .painter()
        .layout_no_wrap(game_name.to_string(), game_title_font, egui::Color32::WHITE);
    let center_y = img_bottom + 52.0 + 30.0;
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
    let meta_str = meta_parts.join("  •  ");
    let pt_font_size = game_font_size * 0.5;
    let pt_font = egui::FontId::proportional(pt_font_size);
    let pt_galley = if !meta_str.is_empty() {
        Some(ui.painter().layout_no_wrap(meta_str, pt_font, egui::Color32::WHITE))
    } else {
        None
    };
    let pt_row_h = pt_galley.as_ref().map_or(0.0, |g| g.size().y + 2.0);
    let total_h = game_title_galley.size().y + pt_row_h;
    let panel_top = (center_y + total_h * 0.5 + 18.0).max(clip_y);
    let panel_left = (game_text_x - 52.0).max(padded_rect.min.x + 8.0);
    let rect = egui::Rect::from_min_max(egui::pos2(panel_left, panel_top), egui::pos2(panel_right, padded_rect.max.y));

    let activity_alpha = if active { 1.0 } else { 0.45 };

    match summary {
        Some(s) => {
            let list_left = (game_text_x - 52.0).max(panel_rect.min.x);
            let list_rect = egui::Rect::from_min_max(
                egui::pos2(list_left, rect.min.y),
                egui::pos2(rect.max.x - 10.0, rect.max.y),
            );
            let list_painter = ui.painter().with_clip_rect(list_rect);
            let row_spacing = 40.0;
            let visible_above = 0_i32;
            let visible_below = 8_i32;
            let center_y = list_rect.min.y + 18.0;

            for (idx, item) in s.items.iter().enumerate() {
                let offset_f = idx as f32 - scroll_offset;
                if offset_f < -(visible_above as f32 + 1.0) || offset_f > (visible_below as f32 + 1.0) {
                    continue;
                }

                let is_selected = idx == selected_index;
                let dist = offset_f.abs();
                let sign = if offset_f >= 0.0 { 1.0 } else { -1.0 };
                let selected_gap = 8.0;
                let extra = offset_f.clamp(-1.0, 1.0) * selected_gap;
                let y_pos = center_y + sign * dist * row_spacing * (1.0 - dist * 0.03).max(0.72) + extra;

                if y_pos < list_rect.min.y - 36.0 || y_pos > list_rect.max.y + 36.0 {
                    continue;
                }

                let alpha_factor = (1.0 - dist * 0.15).max(0.24);
                let t = if is_selected {
                    1.0 - (1.0 - achievement_select_anim) * (1.0 - achievement_select_anim)
                } else {
                    0.0
                };
                let icon_size = 20.0 + 8.0 * alpha_factor + 2.0 * t;
                let text_size = 14.0 + 2.5 * alpha_factor + 1.2 * t;

                let text_x = game_text_x;
                let icon_pos = egui::pos2(text_x - icon_size - 10.0, y_pos - icon_size * 0.5);
                let icon_rect = egui::Rect::from_min_size(icon_pos, egui::vec2(icon_size, icon_size));
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
                let title_galley = list_painter.layout_no_wrap(title, title_font, color);
                let status_galley = format_achievement_status(item.unlocked, item.unlock_time)
                    .map(|status| {
                        list_painter.layout_no_wrap(
                            format!("  •  {}", status),
                            status_font,
                            status_color,
                        )
                    });
                let total_width = title_galley.size().x
                    + status_galley.as_ref().map(|galley| galley.size().x).unwrap_or(0.0);
                let total_height = status_galley
                    .as_ref()
                    .map(|galley| title_galley.size().y.max(galley.size().y))
                    .unwrap_or(title_galley.size().y);
                let base_pos = egui::pos2(text_x, y_pos - total_height * 0.5);
                list_painter.galley(
                    egui::pos2(base_pos.x, base_pos.y + (total_height - title_galley.size().y) * 0.5),
                    title_galley,
                );
                if let Some(status_galley) = status_galley {
                    list_painter.galley(
                        egui::pos2(
                            base_pos.x + total_width - status_galley.size().x,
                            base_pos.y + (total_height - status_galley.size().y) * 0.5,
                        ),
                        status_galley,
                    );
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

    // A button
    if can_open_achievement_panel {
        draw_icon(painter, &icons.dpad_right, hx);
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
