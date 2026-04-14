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

    // Dark gradient background
    {
        let top_col = egui::Color32::from_rgb(10, 15, 30);
        let bot_col = egui::Color32::from_rgb(20, 25, 50);
        let mut m = egui::Mesh::default();
        m.colored_vertex(screen.left_top(), top_col);
        m.colored_vertex(screen.right_top(), top_col);
        m.colored_vertex(screen.right_bottom(), bot_col);
        m.colored_vertex(screen.left_bottom(), bot_col);
        m.add_triangle(0, 1, 2);
        m.add_triangle(0, 2, 3);
        bg_painter.add(egui::Shape::mesh(m));
    }

    let contain_rect_offset = |tex: &egui::TextureHandle, dy: f32| -> egui::Rect {
        let tex_size = tex.size_vec2();
        let scale = (screen.width() / tex_size.x).min(screen.height() / tex_size.y);
        let img_w = tex_size.x * scale;
        let img_h = tex_size.y * scale;
        let offset_x = (screen.width() - img_w) * 0.5;
        let offset_y = (screen.height() - img_h) * 0.5 + dy;
        egui::Rect::from_min_size(
            egui::pos2(screen.min.x + offset_x, screen.min.y + offset_y),
            egui::vec2(img_w, img_h),
        )
    };

    let slide_distance = 12.0;
    let ease_t = 1.0 - (1.0 - cover_fade) * (1.0 - cover_fade);

    // Previous cover (fading out)
    if cover_fade < 1.0 {
        if let Some((_id, tex)) = cover_prev {
            let alpha = (base_alpha * (1.0 - cover_fade)) as u8;
            let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
            bg_painter.image(tex.id(), contain_rect_offset(tex, 0.0), uv, tint);
        }
    }

    // Current cover (fading in, sliding in)
    if let Some((_id, tex)) = cover {
        let alpha = (base_alpha * cover_fade) as u8;
        let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
        let dy = cover_nav_dir * slide_distance * (1.0 - ease_t);
        bg_painter.image(tex.id(), contain_rect_offset(tex, dy), uv, tint);
    }
}

pub fn draw_game_list(
    ui: &mut egui::Ui,
    games: &[crate::steam::Game],
    selected: usize,
    select_anim: f32,
) {
    let panel_rect = ui.available_rect_before_wrap();
    let padding = 50.0;
    let padded_rect = panel_rect.shrink(padding);
    let center_y = padded_rect.center().y;
    let left_x = padded_rect.min.x + 20.0;

    let selected_size = 30.0;
    let base_size = 18.0;
    let row_spacing = 52.0;
    let visible_above = 6;
    let visible_below = 6;

    let painter = ui.painter();

    for (i, g) in games.iter().enumerate() {
        let offset = i as isize - selected as isize;
        if offset < -(visible_above as isize) || offset > visible_below as isize {
            continue;
        }

        let dist = (offset as f32).abs();
        let sign = if offset >= 0 { 1.0 } else { -1.0 };
        let y_pos = center_y + sign * dist * row_spacing * (1.0 - dist * 0.03).max(0.7);

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

        let font_id = egui::FontId::proportional(font_size);
        let galley = painter.layout_no_wrap(g.name.clone(), font_id.clone(), text_color);
        let text_y = y_pos - galley.size().y * 0.5;

        // Selected highlight bar
        if offset == 0 {
            let bar_h = galley.size().y + 16.0;
            let bar_pad_x = 12.0;
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(left_x - bar_pad_x, y_pos - bar_h * 0.5),
                egui::vec2(galley.size().x + bar_pad_x * 2.0, bar_h),
            );
            let glow_alpha = (40.0 * select_anim) as u8;
            let glow_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, glow_alpha);
            painter.rect_filled(bar_rect, egui::Rounding::same(4.0), glow_color);
        }

        // Text shadow
        let shadow_alpha = if offset == 0 {
            200
        } else {
            (120.0 * alpha_factor) as u8
        };
        let shadow_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, shadow_alpha);
        let shadow_galley = painter.layout_no_wrap(g.name.clone(), font_id, shadow_color);
        for off in [
            egui::vec2(1.0, 1.0),
            egui::vec2(-1.0, 1.0),
            egui::vec2(1.0, -1.0),
            egui::vec2(-1.0, -1.0),
        ] {
            painter.galley(egui::pos2(left_x, text_y) + off, shadow_galley.clone());
        }

        // Foreground text
        painter.galley(egui::pos2(left_x, text_y), galley);
    }
}

pub fn draw_hint_bar(ui: &mut egui::Ui, icons: &HintIcons) {
    let panel_rect = ui.available_rect_before_wrap();
    let padding = 50.0;
    let padded_rect = panel_rect.shrink(padding);
    let hint_y = padded_rect.max.y + 10.0;
    let hint_font = egui::FontId::proportional(14.0);
    let hint_color = egui::Color32::from_rgba_unmultiplied(200, 200, 210, 160);
    let icon_h = 24.0_f32;
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

    let painter = ui.painter();
    let mut hx = padded_rect.min.x;

    let draw_icon = |painter: &egui::Painter, tex: &egui::TextureHandle, x: f32| {
        painter.image(
            tex.id(),
            egui::Rect::from_min_size(
                egui::pos2(x, hint_y + (20.0 - icon_h) * 0.5),
                egui::vec2(icon_h, icon_h),
            ),
            uv,
            egui::Color32::WHITE,
        );
    };

    // A button
    draw_icon(painter, &icons.btn_a, hx);
    hx += icon_h + 6.0;

    // "启动"
    let g = painter.layout_no_wrap("启动".to_string(), hint_font.clone(), hint_color);
    let gy = hint_y + (20.0 - g.size().y) * 0.5;
    let g_width = g.size().x;
    painter.galley(egui::pos2(hx, gy), g);
    hx += g_width + 20.0;

    // B button
    draw_icon(painter, &icons.btn_b, hx);
    hx += icon_h + 6.0;

    // "退出"
    let g = painter.layout_no_wrap("退出".to_string(), hint_font, hint_color);
    let gy = hint_y + (20.0 - g.size().y) * 0.5;
    painter.galley(egui::pos2(hx, gy), g);
}
