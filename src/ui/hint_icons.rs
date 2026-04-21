use eframe::egui;

use crate::config::PromptIconTheme;

pub struct HintIcons {
    pub btn_a: egui::TextureHandle,
    pub btn_b: egui::TextureHandle,
    pub btn_x: egui::TextureHandle,
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
    let image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    Some(ctx.load_texture(label, image, egui::TextureOptions::LINEAR))
}

pub fn load_hint_icons(ctx: &egui::Context, theme: PromptIconTheme) -> Option<HintIcons> {
    let (btn_a_bytes, btn_b_bytes, btn_x_bytes, dpad_down_bytes, guide_bytes, label_prefix) =
        match theme {
            PromptIconTheme::Xbox => (
                include_bytes!("../icons/Xbox Series/xbox_button_a_outline.png") as &[u8],
                include_bytes!("../icons/Xbox Series/xbox_button_b_outline.png") as &[u8],
                include_bytes!("../icons/Xbox Series/xbox_button_x_outline.png") as &[u8],
                include_bytes!("../icons/Xbox Series/xbox_dpad_down_outline.png") as &[u8],
                include_bytes!("../icons/Xbox Series/xbox_guide_outline.png") as &[u8],
                "xbox_series",
            ),
            PromptIconTheme::PlayStation => (
                include_bytes!("../icons/PlayStation Series/playstation_button_cross_outline.png") as &[u8],
                include_bytes!("../icons/PlayStation Series/playstation_button_circle_outline.png") as &[u8],
                include_bytes!("../icons/PlayStation Series/playstation_button_square_outline.png") as &[u8],
                include_bytes!("../icons/PlayStation Series/playstation_dpad_down_outline.png") as &[u8],
                include_bytes!("../icons/PlayStation Series/playstation_home.png") as &[u8],
                "playstation_series",
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
    let guide = png_bytes_to_texture(ctx, guide_bytes, &format!("{}_icon_guide", label_prefix))?;
    Some(HintIcons {
        btn_a,
        btn_b,
        btn_x,
        dpad_down,
        guide,
    })
}
