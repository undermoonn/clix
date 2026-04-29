use std::borrow::Cow;

use eframe::egui;

use crate::app::{GameMenuLayout, GameMenuOption};
use crate::i18n::AppLanguage;

use super::{
    color_with_scaled_alpha, corner_radius, design_units, lerp_f32, menu_icon_center_y,
    smoothstep01, viewport_layout_scale, PANEL_CORNER_RADIUS,
};

const GAME_MENU_SELECTION_CORNER_RADIUS: f32 = 12.0;

pub fn draw_game_menu(
    ui: &mut egui::Ui,
    language: AppLanguage,
    layout: &GameMenuLayout,
    game_name: &str,
    close_icon: Option<&egui::TextureHandle>,
    detail_icon: Option<&egui::TextureHandle>,
    hide_icon: Option<&egui::TextureHandle>,
    show_icon: Option<&egui::TextureHandle>,
    home_hidden: bool,
    menu_anim: f32,
    select_anim: f32,
    selected_option_t: f32,
    wake_anim: f32,
) {
    if layout.is_empty() {
        return;
    }

    let wake_t = smoothstep01(wake_anim);
    let menu_t = smoothstep01(menu_anim) * wake_t;
    if menu_t <= 0.001 {
        return;
    }

    let phase_t = |start: f32, end: f32| -> f32 {
        if end <= start {
            return 1.0;
        }
        smoothstep01(((menu_t - start) / (end - start)).clamp(0.0, 1.0))
    };

    let overlay_t = phase_t(0.0, 0.55);
    let sheet_t = phase_t(0.06, 0.68);
    let highlight_t = phase_t(0.22, 1.0);
    let panel_rect = ui.available_rect_before_wrap();
    let layout_scale = viewport_layout_scale(panel_rect);
    let painter = ui.painter();

    painter.rect_filled(
        panel_rect,
        egui::CornerRadius::ZERO,
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(6, 8, 12, 178),
            overlay_t,
        ),
    );

    let option_height = design_units(72.0, layout_scale);
    let option_gap = design_units(10.0, layout_scale);
    let padding = egui::vec2(
        design_units(16.0, layout_scale),
        design_units(16.0, layout_scale),
    );
    let title_height = design_units(68.0, layout_scale);
    let title_option_gap = design_units(18.0, layout_scale);
    let option_count = layout.options().len() as f32;
    let dropdown_size = egui::vec2(
        design_units(344.0, layout_scale),
        padding.y * 2.0
            + title_height
            + title_option_gap
            + option_count * option_height
            + (option_count - 1.0).max(0.0) * option_gap,
    );
    let dropdown_rect =
        egui::Rect::from_center_size(panel_rect.center(), dropdown_size).translate(egui::vec2(
            0.0,
            lerp_f32(design_units(10.0, layout_scale), 0.0, sheet_t),
        ));

    painter.rect_filled(
        dropdown_rect,
        corner_radius(PANEL_CORNER_RADIUS),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(18, 19, 22, 255),
            sheet_t,
        ),
    );
    painter.rect_stroke(
        dropdown_rect,
        corner_radius(PANEL_CORNER_RADIUS),
        egui::Stroke::new(
            design_units(1.0, layout_scale),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 36),
                sheet_t,
            ),
        ),
        egui::StrokeKind::Middle,
    );

    let title_font = egui::FontId::new(
        design_units(25.0, layout_scale),
        egui::FontFamily::Name("Bold".into()),
    );
    let title_galley = super::text::build_wrapped_galley(
        ui,
        game_name.to_owned(),
        title_font,
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(248, 250, 255, 255),
            sheet_t,
        ),
        dropdown_rect.width() - padding.x * 2.0,
    );
    painter.galley(
        egui::pos2(
            dropdown_rect.min.x + padding.x,
            dropdown_rect.min.y + padding.y + (title_height - title_galley.size().y) * 0.45,
        ),
        title_galley,
        egui::Color32::WHITE,
    );

    let option_font = egui::FontId::proportional(design_units(22.0, layout_scale));
    let option_inner_padding = design_units(18.0, layout_scale);
    let option_rects: Vec<_> = layout
        .options()
        .iter()
        .copied()
        .enumerate()
        .map(|(index, option)| {
            let row = index as f32;
            let option_t = phase_t(0.14 + row * 0.1, 0.74 + row * 0.1);
            let rect = egui::Rect::from_min_size(
                egui::pos2(
                    dropdown_rect.min.x + padding.x,
                    dropdown_rect.min.y
                        + padding.y
                        + title_height
                        + title_option_gap
                        + row * (option_height + option_gap),
                ),
                egui::vec2(dropdown_rect.width() - padding.x * 2.0, option_height),
            )
            .translate(egui::vec2(
                0.0,
                lerp_f32(design_units(10.0, layout_scale), 0.0, option_t),
            ));
            (index, option, rect, option_t)
        })
        .collect();

    let selected_index = layout.clamp_selected(selected_option_t.round().max(0.0) as usize);
    if let Some(selected_rect) = option_rects
        .iter()
        .find(|(index, _, _, _)| *index == selected_index)
        .map(|(_, _, rect, _)| *rect)
    {
        let focus_t = smoothstep01(select_anim) * highlight_t;
        if focus_t > 0.001 {
            let focus_rect = selected_rect.expand(design_units(5.0, layout_scale));
            painter.rect_filled(
                focus_rect,
                corner_radius(design_units(
                    GAME_MENU_SELECTION_CORNER_RADIUS,
                    layout_scale,
                )),
                color_with_scaled_alpha(
                    egui::Color32::from_rgba_unmultiplied(248, 250, 255, 54),
                    focus_t,
                ),
            );
            painter.rect_stroke(
                focus_rect,
                corner_radius(design_units(
                    GAME_MENU_SELECTION_CORNER_RADIUS,
                    layout_scale,
                )),
                egui::Stroke::new(
                    lerp_f32(
                        design_units(1.2, layout_scale),
                        design_units(3.0, layout_scale),
                        focus_t,
                    ),
                    color_with_scaled_alpha(
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 168),
                        focus_t,
                    ),
                ),
                egui::StrokeKind::Outside,
            );
        }
    }

    for (index, option, option_rect, option_t) in &option_rects {
        let selectedness = if selected_index == *index { 1.0 } else { 0.0 };
        let text_color = egui::Color32::from_rgb(
            lerp_f32(214.0, 248.0, selectedness).round() as u8,
            lerp_f32(218.0, 249.0, selectedness).round() as u8,
            lerp_f32(226.0, 252.0, selectedness).round() as u8,
        );
        let label = match option {
            GameMenuOption::Details => Cow::Borrowed(language.game_details_text()),
            GameMenuOption::ForceClose => Cow::Borrowed(language.hold_close_game_text()),
            GameMenuOption::ToggleHomeVisibility => Cow::Borrowed(if home_hidden {
                language.show_on_home_text()
            } else {
                language.hide_from_home_text()
            }),
        };
        let leading_icon = match option {
            GameMenuOption::Details => detail_icon,
            GameMenuOption::ForceClose => close_icon,
            GameMenuOption::ToggleHomeVisibility => {
                if home_hidden {
                    show_icon
                } else {
                    hide_icon
                }
            }
        };
        let leading_icon_size = design_units(34.0, layout_scale);
        let icon_text_gap = design_units(16.0, layout_scale);
        let option_text = painter.layout_no_wrap(
            label.into_owned(),
            option_font.clone(),
            color_with_scaled_alpha(text_color, *option_t),
        );
        let content_start_x = option_rect.min.x + option_inner_padding;
        let icon_center_y = menu_icon_center_y(option_rect.center().y, layout_scale);
        let leading_icon_rect = egui::Rect::from_min_size(
            egui::pos2(content_start_x, icon_center_y - leading_icon_size * 0.5),
            egui::vec2(leading_icon_size, leading_icon_size),
        );
        if let Some(icon) = leading_icon {
            painter.image(
                icon.id(),
                leading_icon_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                color_with_scaled_alpha(
                    egui::Color32::from_rgba_unmultiplied(236, 240, 246, 255),
                    option_t * lerp_f32(0.84, 1.0, selectedness),
                ),
            );
        }
        painter.galley(
            egui::pos2(
                leading_icon_rect.max.x + icon_text_gap,
                option_rect.center().y - option_text.size().y * 0.5,
            ),
            option_text,
            egui::Color32::WHITE,
        );
    }
}
