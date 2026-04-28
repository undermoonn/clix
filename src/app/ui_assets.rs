use eframe::egui;

pub(super) fn load_settings_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_generated_icon(
        ctx,
        include_bytes!(concat!(env!("OUT_DIR"), "/settings-icon-ui.png")),
        "home_settings_icon",
    )
}

pub(super) fn load_settings_system_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_generated_icon(
        ctx,
        include_bytes!(concat!(env!("OUT_DIR"), "/system-icon-ui.png")),
        "settings_system_icon",
    )
}

pub(super) fn load_settings_screen_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_generated_icon(
        ctx,
        include_bytes!(concat!(env!("OUT_DIR"), "/screen-icon-ui.png")),
        "settings_screen_icon",
    )
}

pub(super) fn load_settings_apps_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_generated_icon(
        ctx,
        include_bytes!(concat!(env!("OUT_DIR"), "/apps-icon-ui.png")),
        "settings_apps_icon",
    )
}

pub(super) fn load_settings_exit_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_generated_icon(
        ctx,
        include_bytes!(concat!(env!("OUT_DIR"), "/exit-icon-ui.png")),
        "settings_exit_icon",
    )
}

pub(super) fn load_xbox_guide_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_embedded_png_texture(
        ctx,
        include_bytes!("../icons/Xbox Series/xbox_guide.png"),
        "settings_xbox_guide_icon",
    )
}

pub(super) fn load_playstation_home_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_embedded_png_texture(
        ctx,
        include_bytes!("../icons/PlayStation Series/playstation_home.png"),
        "settings_playstation_home_icon",
    )
}

pub(super) fn load_power_sleep_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_generated_icon(
        ctx,
        include_bytes!(concat!(env!("OUT_DIR"), "/power-sleep-icon-ui.png")),
        "home_power_sleep_icon",
    )
}

pub(super) fn load_power_reboot_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_generated_icon(
        ctx,
        include_bytes!(concat!(env!("OUT_DIR"), "/power-reboot-icon-ui.png")),
        "home_power_reboot_icon",
    )
}

pub(super) fn load_power_off_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_generated_icon(
        ctx,
        include_bytes!(concat!(env!("OUT_DIR"), "/power-off-icon-ui.png")),
        "home_power_off_icon",
    )
}

fn load_generated_icon(
    ctx: &egui::Context,
    bytes: &[u8],
    texture_name: &str,
) -> Option<egui::TextureHandle> {
    load_embedded_png_texture(ctx, bytes, texture_name)
}

fn load_embedded_png_texture(
    ctx: &egui::Context,
    bytes: &[u8],
    texture_name: &str,
) -> Option<egui::TextureHandle> {
    let dyn_img = image::load_from_memory(bytes).ok()?;
    let rgba = dyn_img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    Some(ctx.load_texture(texture_name, image, egui::TextureOptions::LINEAR))
}

pub(super) fn achievement_panel_scope_steam_app_id(
    selected_steam_app_id: Option<u32>,
    achievement_panel_open: bool,
    achievement_panel_anim: f32,
) -> Option<u32> {
    if achievement_panel_open || achievement_panel_anim > 0.001 {
        selected_steam_app_id
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::achievement_panel_scope_steam_app_id;

    #[test]
    fn keeps_achievement_scope_while_close_animation_is_still_visible() {
        assert_eq!(
            achievement_panel_scope_steam_app_id(Some(42), false, 0.25),
            Some(42)
        );
    }

    #[test]
    fn clears_achievement_scope_after_close_animation_finishes() {
        assert_eq!(
            achievement_panel_scope_steam_app_id(Some(42), false, 0.001),
            None
        );
        assert_eq!(
            achievement_panel_scope_steam_app_id(Some(42), false, 0.0),
            None
        );
    }

    #[test]
    fn keeps_achievement_scope_while_panel_is_open() {
        assert_eq!(
            achievement_panel_scope_steam_app_id(Some(42), true, 0.0),
            Some(42)
        );
    }
}
