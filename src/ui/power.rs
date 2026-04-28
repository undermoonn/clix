use std::borrow::Cow;

use eframe::egui;

use crate::app::{PowerMenuLayout, PowerMenuOption};
use crate::i18n::AppLanguage;
use crate::ui::{
    color_with_scaled_alpha, corner_radius, design_units, layout_main_clock, lerp_f32,
    main_clock_right_edge, smoothstep01, viewport_layout_scale, HintIcons, PANEL_CORNER_RADIUS,
};

const POWER_MENU_SELECTION_CORNER_RADIUS: f32 = 12.0;

pub fn draw_power_menu(
    ui: &mut egui::Ui,
    language: AppLanguage,
    layout: &PowerMenuLayout,
    power_sleep_icon: Option<&egui::TextureHandle>,
    power_reboot_icon: Option<&egui::TextureHandle>,
    power_off_icon: Option<&egui::TextureHandle>,
    _icons: Option<&HintIcons>,
    _current_mode_label: &str,
    _half_refresh_label: &str,
    _max_refresh_label: &str,
    _show_power_options: bool,
    menu_anim: f32,
    select_anim: f32,
    selected_option_t: f32,
    _power_focus_anim: f32,
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

    let clock_galley = layout_main_clock(&painter, wake_t, layout_scale);
    let clock_anchor_rect = egui::Rect::from_min_size(
        panel_rect.min,
        egui::vec2(panel_rect.width(), panel_rect.width() * (1240.0 / 3840.0)),
    );
    let clock_margin_y = clock_anchor_rect.height() * 0.075;
    let clock_pos = egui::pos2(
        main_clock_right_edge(clock_anchor_rect) - clock_galley.size().x,
        clock_anchor_rect.min.y + clock_margin_y,
    );
    let top_icon_size = clock_galley.size().y * 0.63;
    let power_icon_size = top_icon_size * 1.18;
    let settings_icon_offset_x = design_units(56.0, layout_scale);
    let settings_icon_pos = egui::pos2(
        clock_pos.x - top_icon_size - settings_icon_offset_x,
        clock_pos.y + (clock_galley.size().y - top_icon_size) * 0.5,
    );
    let power_icon_gap = design_units(54.0, layout_scale);
    let power_anchor_rect = egui::Rect::from_min_size(
        egui::pos2(
            settings_icon_pos.x - power_icon_size - power_icon_gap,
            settings_icon_pos.y + (top_icon_size - power_icon_size) * 0.5,
        ),
        egui::vec2(power_icon_size, power_icon_size),
    );

    let option_height = design_units(72.0, layout_scale);
    let option_gap = design_units(10.0, layout_scale);
    let dropdown_padding = egui::vec2(
        design_units(14.0, layout_scale),
        design_units(14.0, layout_scale),
    );
    let option_count = layout.options().len() as f32;
    let dropdown_size = egui::vec2(
        design_units(232.0, layout_scale),
        dropdown_padding.y * 2.0
            + option_count * option_height
            + (option_count - 1.0).max(0.0) * option_gap,
    );
    let dropdown_origin_x = (power_anchor_rect.center().x - dropdown_size.x * 0.5).clamp(
        panel_rect.min.x + design_units(40.0, layout_scale),
        panel_rect.max.x - dropdown_size.x - design_units(40.0, layout_scale),
    );
    let dropdown_rect = egui::Rect::from_min_size(
        egui::pos2(
            dropdown_origin_x,
            power_anchor_rect.max.y + design_units(18.0, layout_scale),
        ),
        dropdown_size,
    )
    .translate(egui::vec2(
        0.0,
        lerp_f32(design_units(8.0, layout_scale), 0.0, sheet_t),
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
            let option_offset = egui::vec2(
                0.0,
                lerp_f32(design_units(10.0, layout_scale), 0.0, option_t),
            );
            let rect = egui::Rect::from_min_size(
                egui::pos2(
                    dropdown_rect.min.x + dropdown_padding.x,
                    dropdown_rect.min.y + dropdown_padding.y + row * (option_height + option_gap),
                ),
                egui::vec2(
                    dropdown_rect.width() - dropdown_padding.x * 2.0,
                    option_height,
                ),
            )
            .translate(option_offset);
            (index, option, rect, option_t)
        })
        .collect();

    let selected_index = layout.clamp_selected(selected_option_t.round().max(0.0) as usize);
    let highlight_offset = egui::vec2(
        0.0,
        lerp_f32(design_units(6.0, layout_scale), 0.0, highlight_t),
    );
    let Some(selected_option_rect): Option<egui::Rect> = option_rects
        .iter()
        .find(|(index, _, _, _)| *index == selected_index)
        .map(|(_, _, rect, _)| *rect)
    else {
        return;
    };
    let selected_rect = selected_option_rect.translate(highlight_offset);

    let focus_t = smoothstep01(select_anim) * highlight_t;
    if focus_t > 0.001 {
        let focus_rect = selected_rect.expand(design_units(5.0, layout_scale));
        painter.rect_filled(
            focus_rect,
            corner_radius(design_units(
                POWER_MENU_SELECTION_CORNER_RADIUS,
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
                POWER_MENU_SELECTION_CORNER_RADIUS,
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

    let centered_content_left_bias = design_units(12.0, layout_scale);
    for (index, option, option_rect, option_t) in &option_rects {
        let selectedness = if selected_index == *index { 1.0 } else { 0.0 };
        let text_color = egui::Color32::from_rgb(
            lerp_f32(214.0, 248.0, selectedness).round() as u8,
            lerp_f32(218.0, 249.0, selectedness).round() as u8,
            lerp_f32(226.0, 252.0, selectedness).round() as u8,
        );
        let label = match option {
            PowerMenuOption::Sleep => Cow::Borrowed(language.sleep_text()),
            PowerMenuOption::Reboot => Cow::Borrowed(language.reboot_text()),
            PowerMenuOption::Shutdown => Cow::Borrowed(language.shutdown_text()),
        };
        let leading_icon = match option {
            PowerMenuOption::Sleep => power_sleep_icon,
            PowerMenuOption::Reboot => power_reboot_icon,
            PowerMenuOption::Shutdown => power_off_icon,
        };
        let leading_icon_size = top_icon_size;
        let icon_text_gap = design_units(16.0, layout_scale);
        let option_text = painter.layout_no_wrap(
            label.into_owned(),
            option_font.clone(),
            color_with_scaled_alpha(text_color, *option_t),
        );
        let content_width = leading_icon_size + icon_text_gap + option_text.size().x;
        let content_rect = egui::Rect::from_min_max(
            egui::pos2(option_rect.min.x + option_inner_padding, option_rect.min.y),
            egui::pos2(option_rect.max.x - option_inner_padding, option_rect.max.y),
        );
        let content_start_x = if language == AppLanguage::English {
            content_rect.min.x
        } else {
            (content_rect.center().x - content_width * 0.5 - centered_content_left_bias)
                .max(content_rect.min.x)
        };
        let leading_icon_rect = egui::Rect::from_min_size(
            egui::pos2(
                content_start_x,
                option_rect.center().y - leading_icon_size * 0.5,
            ),
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
