use eframe::egui;

use crate::i18n::AppLanguage;
use crate::system::display_mode::ResolutionOptions;

use super::anim::{lerp_f32, smoothstep01};
use super::text::{color_with_scaled_alpha, corner_radius};

struct SettingsLayerState {
    top_layer_t: f32,
    submenu_layer_t: f32,
    top_motion_t: f32,
    submenu_motion_t: f32,
}

fn settings_layer_state(
    settings_t: f32,
    submenu_t: f32,
    settings_in_submenu: bool,
) -> SettingsLayerState {
    let (top_layer_t, top_motion_t, submenu_layer_t, submenu_motion_t) = if settings_in_submenu {
        let top_exit_t = (submenu_t * 2.0).clamp(0.0, 1.0);
        let submenu_enter_t = ((submenu_t - 0.5) * 2.0).clamp(0.0, 1.0);
        (
            settings_t * (1.0 - top_exit_t),
            top_exit_t,
            settings_t * submenu_enter_t,
            submenu_enter_t,
        )
    } else {
        let top_enter_t = (1.0 - submenu_t * 2.0).clamp(0.0, 1.0);
        let submenu_exit_t = ((submenu_t - 0.5) * 2.0).clamp(0.0, 1.0);
        (
            settings_t * top_enter_t,
            1.0 - top_enter_t,
            settings_t * submenu_exit_t,
            submenu_exit_t,
        )
    };

    SettingsLayerState {
        top_layer_t,
        submenu_layer_t,
        top_motion_t,
        submenu_motion_t,
    }
}

fn draw_settings_list_row(
    painter: &egui::Painter,
    rect: egui::Rect,
    leading_icon: Option<&egui::TextureHandle>,
    title: &str,
    subtitle: Option<&str>,
    trailing: Option<&str>,
    focus_t: f32,
    show_focus_outline: bool,
    settings_t: f32,
) {
    let card_corner = corner_radius(9.0);
    let fill = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(206, 214, 224, 88),
        settings_t,
    );
    painter.rect_filled(rect, card_corner, fill);
    painter.rect_stroke(
        rect,
        card_corner,
        egui::Stroke::new(
            1.0,
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 34),
                settings_t,
            ),
        ),
        egui::StrokeKind::Middle,
    );

    if focus_t > 0.001 {
        let focus_rect = rect.expand(5.0);
        painter.rect_filled(
            focus_rect,
            corner_radius(12.0),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(248, 250, 255, 54),
                settings_t * focus_t,
            ),
        );
        if show_focus_outline {
            painter.rect_stroke(
                focus_rect,
                corner_radius(12.0),
                egui::Stroke::new(
                    lerp_f32(1.2, 3.0, focus_t),
                    color_with_scaled_alpha(
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 168),
                        settings_t * focus_t,
                    ),
                ),
                egui::StrokeKind::Outside,
            );
        }
    }

    let content_padding_x = 34.0;
    let content_padding_y = 18.0;
    let content_rect = rect.shrink2(egui::vec2(content_padding_x, content_padding_y));
    let icon_slot_size = 54.0;
    let icon_render_size = 36.0;
    let icon_gap = 26.0;
    let text_start_x = if let Some(icon) = leading_icon {
        let icon_badge_center = egui::pos2(content_rect.min.x + icon_slot_size * 0.5, content_rect.center().y);
        let icon_rect = egui::Rect::from_center_size(
            icon_badge_center,
            egui::vec2(icon_render_size, icon_render_size),
        );
        painter.image(
            icon.id(),
            icon_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(242, 245, 248, 255),
                settings_t * lerp_f32(0.9, 1.0, focus_t),
            ),
        );
        content_rect.min.x + icon_slot_size + icon_gap
    } else {
        content_rect.min.x
    };
    let title_galley = painter.layout_no_wrap(
        title.to_string(),
        egui::FontId::proportional(29.0),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(242, 245, 248, 255),
            settings_t,
        ),
    );
    let subtitle_galley = subtitle.map(|subtitle| {
        painter.layout_no_wrap(
            subtitle.to_string(),
            egui::FontId::proportional(19.0),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(196, 202, 212, 220),
                settings_t,
            ),
        )
    });
    let title_height = title_galley.size().y;

    if let Some(trailing) = trailing {
        let status_galley = painter.layout_no_wrap(
            trailing.to_string(),
            egui::FontId::proportional(15.0),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(232, 236, 242, 220),
                settings_t,
            ),
        );
        let badge_size = status_galley.size() + egui::vec2(24.0, 12.0);
        let badge_rect = egui::Rect::from_min_size(
            egui::pos2(content_rect.max.x - badge_size.x, content_rect.min.y),
            badge_size,
        );
        painter.rect_filled(
            badge_rect,
            corner_radius(7.0),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(40, 42, 46, 184),
                settings_t,
            ),
        );
        painter.rect_stroke(
            badge_rect,
            corner_radius(7.0),
            egui::Stroke::new(
                1.0,
                color_with_scaled_alpha(
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 34),
                    settings_t,
                ),
            ),
            egui::StrokeKind::Middle,
        );
        painter.galley(
            egui::pos2(
                badge_rect.center().x - status_galley.size().x * 0.5,
                badge_rect.center().y - status_galley.size().y * 0.5,
            ),
            status_galley,
            egui::Color32::WHITE,
        );
    }

    let subtitle_gap = 8.0;
    let text_block_height = if let Some(subtitle_galley) = &subtitle_galley {
        title_height + subtitle_gap + subtitle_galley.size().y
    } else {
        title_height
    };
    let title_y = content_rect.center().y - text_block_height * 0.5;
    painter.galley(
        egui::pos2(text_start_x, title_y),
        title_galley,
        egui::Color32::WHITE,
    );
    if let Some(subtitle_galley) = subtitle_galley {
        let subtitle_y = title_y + title_height + subtitle_gap;
        painter.galley(
            egui::pos2(text_start_x, subtitle_y),
            subtitle_galley,
            egui::Color32::WHITE,
        );
    }

}

fn draw_settings_section_header(
    painter: &egui::Painter,
    rect: egui::Rect,
    title: &str,
    subtitle: Option<&str>,
    settings_t: f32,
) -> f32 {
    let title_galley = painter.layout_no_wrap(
        title.to_string(),
        egui::FontId::proportional(24.0),
        color_with_scaled_alpha(egui::Color32::WHITE, settings_t),
    );
    let title_height = title_galley.size().y;
    painter.galley(rect.min, title_galley, egui::Color32::WHITE);

    let mut height = 36.0;
    if let Some(subtitle) = subtitle {
        let subtitle_galley = painter.layout_no_wrap(
            subtitle.to_string(),
            egui::FontId::proportional(18.0),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(178, 184, 192, 210),
                settings_t,
            ),
        );
        let subtitle_height = subtitle_galley.size().y;
        let subtitle_y = rect.min.y + title_height + 10.0;
        painter.galley(
            egui::pos2(rect.min.x, subtitle_y),
            subtitle_galley,
            egui::Color32::WHITE,
        );
        height = title_height + subtitle_height + 18.0;
    }

    painter.line_segment(
        [
            egui::pos2(rect.min.x, rect.min.y + height + 10.0),
            egui::pos2(rect.max.x, rect.min.y + height + 10.0),
        ],
        egui::Stroke::new(
            1.0,
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
                settings_t,
            ),
        ),
    );

    height + 22.0
}

pub fn draw_settings_page(
    ui: &mut egui::Ui,
    language: AppLanguage,
    system_icon: Option<&egui::TextureHandle>,
    screen_icon: Option<&egui::TextureHandle>,
    apps_icon: Option<&egui::TextureHandle>,
    exit_icon: Option<&egui::TextureHandle>,
    launch_on_startup_enabled: bool,
    resolution_options: &ResolutionOptions,
    selected_section_index: usize,
    selected_item_index: usize,
    settings_in_submenu: bool,
    settings_anim: f32,
    submenu_anim: f32,
    settings_select_anim: f32,
) {
    let settings_t = smoothstep01(settings_anim);
    if settings_t <= 0.001 {
        return;
    }

    let scale_t = lerp_f32(0.94, 1.0, settings_t);

    let panel_rect = ui.available_rect_before_wrap();
    let painter = ui.painter();
    painter.rect_filled(panel_rect, egui::CornerRadius::ZERO, egui::Color32::from_rgb(18, 18, 18));

    let base_content_rect = panel_rect.shrink2(egui::vec2(52.0, 52.0));
    let page_rect = egui::Rect::from_center_size(
        base_content_rect.center(),
        base_content_rect.size() * scale_t,
    )
    .translate(egui::vec2(0.0, lerp_f32(18.0, 0.0, settings_t)));
    let submenu_t = smoothstep01(submenu_anim);

    let draw_page_shell = |content_rect: egui::Rect, layer_t: f32, title_text: &str| {
        let header_rect = egui::Rect::from_min_max(
            egui::pos2(content_rect.min.x, content_rect.min.y),
            egui::pos2(content_rect.max.x, content_rect.min.y + 78.0),
        );
        let body_rect = egui::Rect::from_min_max(
            egui::pos2(content_rect.min.x, header_rect.max.y + 28.0),
            egui::pos2(content_rect.max.x, content_rect.max.y - 56.0),
        );
        painter.rect_filled(
            body_rect,
            corner_radius(8.0),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(14, 14, 14, 236),
                layer_t,
            ),
        );
        painter.rect_stroke(
            body_rect,
            corner_radius(8.0),
            egui::Stroke::new(
                1.2,
                color_with_scaled_alpha(
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 42),
                    layer_t,
                ),
            ),
            egui::StrokeKind::Middle,
        );

        let title = painter.layout_no_wrap(
            title_text.to_string(),
            egui::FontId::proportional(34.0),
            color_with_scaled_alpha(egui::Color32::WHITE, layer_t),
        );
        let top_left = header_rect.min + egui::vec2(8.0, 8.0);
        painter.galley(top_left, title, egui::Color32::WHITE);

        egui::Rect::from_min_max(
            egui::pos2(body_rect.min.x + 6.0, body_rect.min.y + 14.0),
            egui::pos2(body_rect.max.x - 6.0, body_rect.max.y - 16.0),
        )
    };

    let draw_row = |list_inner_rect: egui::Rect,
                    rows_origin_y: f32,
                    row_spacing: f32,
                    row_height: f32,
                    index: usize,
                    leading_icon: Option<&egui::TextureHandle>,
                    title: &str,
                    subtitle: Option<&str>,
                    trailing: Option<&str>,
                    selected: bool,
                    show_focus_outline: bool,
                    layer_t: f32| {
        let row_top = rows_origin_y + index as f32 * row_spacing;
        let list_painter = painter.with_clip_rect(list_inner_rect);
        let row_side_inset = 6.0;
        let unselected_row_shrink_x = 7.0;
        let row_slot_rect = egui::Rect::from_min_max(
            egui::pos2(list_inner_rect.min.x + row_side_inset, row_top),
            egui::pos2(list_inner_rect.max.x - row_side_inset, row_top + row_height),
        );
        let row_rect = row_slot_rect.shrink2(egui::vec2(unselected_row_shrink_x, 0.0));
        draw_settings_list_row(
            &list_painter,
            row_rect,
            leading_icon,
            title,
            subtitle,
            trailing,
            if selected {
                smoothstep01(settings_select_anim)
            } else {
                0.0
            },
            show_focus_outline,
            layer_t,
        );
    };

    let layer_state = settings_layer_state(settings_t, submenu_t, settings_in_submenu);
    let top_layer_t = layer_state.top_layer_t;
    let submenu_layer_t = layer_state.submenu_layer_t;

    if top_layer_t > 0.001 {
        let top_content_rect = egui::Rect::from_center_size(
            page_rect.center(),
            page_rect.size() * lerp_f32(1.0, 0.985, layer_state.top_motion_t),
        )
        .translate(egui::vec2(0.0, lerp_f32(0.0, -10.0, layer_state.top_motion_t)));
        let top_list_inner_rect = draw_page_shell(top_content_rect, top_layer_t, language.settings_text());
        let initial_row_offset_y = 8.0;
        let top_rows_origin_y = top_list_inner_rect.min.y + initial_row_offset_y;
        let top_row_spacing = 104.0;
        let top_row_height = 90.0;
        draw_row(
            top_list_inner_rect,
            top_rows_origin_y,
            top_row_spacing,
            top_row_height,
            0,
            system_icon,
            language.system_text(),
            None,
            None,
            selected_section_index == 0,
            true,
            top_layer_t,
        );
        draw_row(
            top_list_inner_rect,
            top_rows_origin_y,
            top_row_spacing,
            top_row_height,
            1,
            screen_icon,
            language.screen_text(),
            None,
            None,
            selected_section_index == 1,
            true,
            top_layer_t,
        );
        draw_row(
            top_list_inner_rect,
            top_rows_origin_y,
            top_row_spacing,
            top_row_height,
            2,
            apps_icon,
            language.apps_text(),
            None,
            None,
            selected_section_index == 2,
            true,
            top_layer_t,
        );
        draw_row(
            top_list_inner_rect,
            top_rows_origin_y,
            top_row_spacing,
            top_row_height,
            3,
            exit_icon,
            language.close_app_text(),
            None,
            None,
            selected_section_index == 3,
            true,
            top_layer_t,
        );
    }

    if submenu_layer_t > 0.001 {
        let submenu_content_rect = egui::Rect::from_center_size(
            page_rect.center(),
            page_rect.size() * lerp_f32(0.982, 1.0, layer_state.submenu_motion_t),
        )
        .translate(egui::vec2(0.0, lerp_f32(10.0, 0.0, layer_state.submenu_motion_t)));
        let summary_text = match selected_section_index {
            0 => None,
            1 => Some(format!(
                "{} {}",
                language.current_display_mode_text(),
                resolution_options.current.label
            )),
            _ => None,
        };
        let breadcrumb_section_name = match selected_section_index {
            0 => language.system_text(),
            1 => language.screen_text(),
            2 => language.apps_text(),
            _ => language.close_app_text(),
        };
        let section_name = match selected_section_index {
            0 => language.startup_options_text(),
            1 => language.resolution_settings_text(),
            2 => language.installed_app_options_text(),
            _ => language.close_app_text(),
        };
        let page_title = format!("{} / {}", language.settings_text(), breadcrumb_section_name);
        let submenu_list_inner_rect = draw_page_shell(submenu_content_rect, submenu_layer_t, &page_title);
        let header_rect = egui::Rect::from_min_max(
            egui::pos2(submenu_list_inner_rect.min.x + 18.0, submenu_list_inner_rect.min.y + 8.0),
            egui::pos2(submenu_list_inner_rect.max.x - 18.0, submenu_list_inner_rect.min.y + 96.0),
        );
        let submenu_list_painter = painter.with_clip_rect(submenu_list_inner_rect);
        let initial_row_offset_y = 8.0;
        let rows_origin_y = submenu_list_inner_rect.min.y
            + draw_settings_section_header(
                &submenu_list_painter,
                header_rect,
                section_name,
                summary_text.as_deref(),
                submenu_layer_t,
            )
            + initial_row_offset_y;

        let submenu_row_spacing = 112.0;
        let submenu_row_height = 98.0;
        match selected_section_index {
            0 => {
                draw_row(
                    submenu_list_inner_rect,
                    rows_origin_y,
                    submenu_row_spacing,
                    submenu_row_height,
                    0,
                    None,
                    language.launch_on_startup_text(),
                    Some(if launch_on_startup_enabled {
                        language.enabled_text()
                    } else {
                        language.disabled_text()
                    }),
                    None,
                    selected_item_index == 0,
                    true,
                    submenu_layer_t,
                );
            }
            1 => {
                draw_row(
                    submenu_list_inner_rect,
                    rows_origin_y,
                    submenu_row_spacing,
                    submenu_row_height,
                    0,
                    None,
                    &resolution_options.half_refresh.label,
                    None,
                    None,
                    selected_item_index == 0,
                    true,
                    submenu_layer_t,
                );
                draw_row(
                    submenu_list_inner_rect,
                    rows_origin_y,
                    submenu_row_spacing,
                    submenu_row_height,
                    1,
                    None,
                    &resolution_options.max_refresh.label,
                    None,
                    None,
                    selected_item_index == 1,
                    true,
                    submenu_layer_t,
                );
            }
            _ => {
                draw_row(
                    submenu_list_inner_rect,
                    rows_origin_y,
                    submenu_row_spacing,
                    submenu_row_height,
                    0,
                    None,
                    language.dlss_swapper_text(),
                    None,
                    None,
                    selected_item_index == 0,
                    true,
                    submenu_layer_t,
                );
                draw_row(
                    submenu_list_inner_rect,
                    rows_origin_y,
                    submenu_row_spacing,
                    submenu_row_height,
                    1,
                    None,
                    language.nvidia_app_text(),
                    None,
                    None,
                    selected_item_index == 1,
                    true,
                    submenu_layer_t,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::settings_layer_state;

    #[test]
    fn returning_from_submenu_hides_top_level_until_exit_finishes() {
        let layer_state = settings_layer_state(1.0, 0.6, false);

        assert!(layer_state.top_layer_t.abs() < f32::EPSILON);
        assert!((layer_state.top_motion_t - 1.0).abs() < f32::EPSILON);
        assert!((layer_state.submenu_motion_t - 0.2).abs() < 1e-6);
        assert!((layer_state.submenu_layer_t - 0.2).abs() < 1e-6);
    }

    #[test]
    fn top_level_returns_after_submenu_exit_finishes() {
        let layer_state = settings_layer_state(1.0, 0.0, false);

        assert!((layer_state.top_layer_t - 1.0).abs() < f32::EPSILON);
        assert!(layer_state.top_motion_t.abs() < f32::EPSILON);
        assert!(layer_state.submenu_layer_t.abs() < f32::EPSILON);
    }

    #[test]
    fn entering_submenu_hides_submenu_until_top_level_exits() {
        let layer_state = settings_layer_state(1.0, 0.4, true);

        assert!((layer_state.top_layer_t - 0.2).abs() < 1e-6);
        assert!((layer_state.top_motion_t - 0.8).abs() < 1e-6);
        assert!(layer_state.submenu_motion_t.abs() < f32::EPSILON);
        assert!(layer_state.submenu_layer_t.abs() < f32::EPSILON);
    }

    #[test]
    fn submenu_enters_after_top_level_exit_finishes() {
        let layer_state = settings_layer_state(1.0, 0.6, true);

        assert!(layer_state.top_layer_t.abs() < f32::EPSILON);
        assert!((layer_state.top_motion_t - 1.0).abs() < f32::EPSILON);
        assert!((layer_state.submenu_motion_t - 0.2).abs() < 1e-6);
        assert!((layer_state.submenu_layer_t - 0.2).abs() < 1e-6);
    }
}