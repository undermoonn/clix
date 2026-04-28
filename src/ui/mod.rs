mod achievement_page;
mod background;
mod game_list;
mod header;
mod hint_icons;
mod hint_bar;
mod power;
mod settings_page;
mod text;

use eframe::egui;

const VIEWPORT_LAYOUT_BASELINE_WIDTH: f32 = 1920.0;
const VIEWPORT_LAYOUT_BASELINE_HEIGHT: f32 = 1080.0;

pub use achievement_page::draw_achievement_page;
pub use background::draw_background;
pub use game_list::draw_game_list;
pub(crate) use hint_icons::HintIcons;
pub use hint_icons::load_hint_icons;
pub use hint_bar::draw_hint_bar;
pub use power::draw_power_menu;
pub(crate) use crate::animation::easing::{lerp_f32, smoothstep01};
pub use settings_page::draw_settings_page;
pub(crate) fn viewport_layout_scale(panel_rect: egui::Rect) -> f32 {
	(panel_rect.width() / VIEWPORT_LAYOUT_BASELINE_WIDTH)
		.min(panel_rect.height() / VIEWPORT_LAYOUT_BASELINE_HEIGHT)
		.max(0.01)
}

pub(crate) fn design_units(value: f32, layout_scale: f32) -> f32 {
	value * layout_scale
}

pub(crate) use text::{
	color_with_scaled_alpha, corner_radius, layout_main_clock, main_clock_right_edge,
	PANEL_CORNER_RADIUS,
};