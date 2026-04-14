use eframe::egui;

pub struct HintIcons {
    pub btn_a: egui::TextureHandle,
    pub btn_b: egui::TextureHandle,
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
    Some(HintIcons {
        btn_a,
        btn_b,
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
    game_icons: &std::collections::HashMap<u32, egui::TextureHandle>,
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
        let offset = i as isize - selected as isize;
        if offset < -(visible_above as isize) || offset > visible_below as isize {
            continue;
        }

        let dist = (offset as f32).abs();
        let sign = if offset >= 0 { 1.0 } else { -1.0 };
        // Extra gap around selected item
        let selected_gap = 16.0;
        let extra = if offset > 0 { selected_gap } else if offset < 0 { -selected_gap } else { 0.0 };
        let y_pos = center_y + sign * dist * row_spacing * (1.0 - dist * 0.03).max(0.7) + extra;

        // Skip items that would overlap the cover image area
        if y_pos < clip_y {
            continue;
        }

        let alpha_factor = (1.0 - dist * 0.13).max(0.0);
        let font_size = if offset == 0 {
            let t = 1.0 - (1.0 - select_anim) * (1.0 - select_anim);
            base_size + (selected_size - base_size) * t
        } else {
            base_size
        };

        let text_alpha = if offset == 0 {
            255
        } else {
            (220.0 * alpha_factor) as u8
        };
        let text_color = if offset == 0 {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255)
        } else {
            egui::Color32::from_rgba_unmultiplied(200, 200, 210, text_alpha)
        };

        // Compute icon offset
        let selected_icon_size = selected_size + 26.0;
        let base_icon_size = base_size + 4.0;
        let icon_size = if offset == 0 {
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

        let font_id = if offset == 0 {
            egui::FontId::new(font_size, egui::FontFamily::Name("Bold".into()))
        } else {
            egui::FontId::proportional(font_size)
        };
        let galley = painter.layout_no_wrap(g.name.clone(), font_id.clone(), text_color);

        // Playtime string
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

        // Measure playtime galley (only shown when selected)
        let pt_font_size = font_size * 0.5;
        let pt_font = egui::FontId::proportional(pt_font_size);
        let pt_color = egui::Color32::from_rgba_unmultiplied(180, 180, 190, 140);
        let pt_galley = if offset == 0 && !playtime_str.is_empty() {
            Some(painter.layout_no_wrap(playtime_str, pt_font, pt_color))
        } else {
            None
        };
        let pt_row_h = pt_galley.as_ref().map_or(0.0, |g| g.size().y + 2.0);

        // Layout: name + playtime stacked, centered on y_pos
        let total_h = galley.size().y + pt_row_h;
        let text_y = y_pos - total_h * 0.5;

        // Selected highlight bar
        if offset == 0 {
            let bar_pad_x = 20.0;
            let bar_pad_y = 12.0;
            let content_w = galley.size().x.max(pt_galley.as_ref().map_or(0.0, |g| g.size().x));
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(left_x - bar_pad_x, y_pos - total_h * 0.5 - bar_pad_y),
                egui::vec2((text_x - left_x) + content_w + bar_pad_x * 2.0, total_h + bar_pad_y * 2.0),
            );
            let glow_alpha = (20.0 * select_anim) as u8;
            let glow_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, glow_alpha);
            painter.rect_filled(bar_rect, egui::Rounding::same(4.0), glow_color);
        }

        // Draw game icon (after highlight bar so it appears on top)
        if let Some(app_id) = g.app_id {
            if let Some(icon_tex) = game_icons.get(&app_id) {
                let icon_tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, text_alpha);
                let icon_rect = egui::Rect::from_min_size(
                    egui::pos2(left_x, y_pos - icon_size * 0.5),
                    egui::vec2(icon_size, icon_size),
                );
                painter.add(egui::Shape::Rect(egui::epaint::RectShape {
                    rect: icon_rect,
                    rounding: egui::Rounding::same(4.0),
                    fill: icon_tint,
                    stroke: egui::Stroke::NONE,
                    fill_texture_id: icon_tex.id(),
                    uv,
                }));
            }
        }

        // Text outline (2-pass for smooth stroke)
        let outline_alpha = if offset == 0 {
            200
        } else {
            (120.0 * alpha_factor) as u8
        };
        let outline_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, outline_alpha);
        let outline_galley = painter.layout_no_wrap(g.name.clone(), font_id, outline_color);
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

pub fn draw_hint_bar(ui: &mut egui::Ui, icons: &HintIcons) {
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

    // Measure total width first (right-aligned)
    let g_launch = painter.layout_no_wrap("Start".to_string(), hint_font.clone(), hint_color);
    let g_quit = painter.layout_no_wrap("Quit".to_string(), hint_font.clone(), hint_color);
    let total_w = icon_h + 6.0 + g_launch.size().x + 20.0 + icon_h + 6.0 + g_quit.size().x;
    let mut hx = padded_rect.max.x - total_w;

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
