use eframe::egui;

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
    let btn_a_bytes = include_bytes!("../icons/Xbox Series/xbox_button_a_outline.png") as &[u8];
    let btn_b_bytes = include_bytes!("../icons/Xbox Series/xbox_button_b_outline.png") as &[u8];
    let btn_x_bytes = include_bytes!("../icons/Xbox Series/xbox_button_x_outline.png") as &[u8];
    let btn_y_bytes = include_bytes!("../icons/Xbox Series/xbox_button_y_outline.png") as &[u8];
    let dpad_down_bytes = include_bytes!("../icons/Xbox Series/xbox_dpad_down_outline.png") as &[u8];
    let guide_bytes = include_bytes!("../icons/Xbox Series/xbox_guide_outline.png") as &[u8];
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
