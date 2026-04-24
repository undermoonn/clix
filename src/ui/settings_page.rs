use eframe::egui;

use crate::game::GameSource;
use crate::i18n::AppLanguage;
use crate::system::display_mode::ResolutionOptions;

use super::anim::{lerp_f32, smoothstep01};
use super::header::{draw_selected_game_text_badge, measure_selected_game_text_badge};
use super::text::{color_with_scaled_alpha, corner_radius, PANEL_CORNER_RADIUS};

struct InlineButtonTitle<'a> {
    prefix: &'a str,
    suffix: &'a str,
    left_icon: &'a egui::TextureHandle,
    right_icon: &'a egui::TextureHandle,
}

struct SettingsLayerState {
    top_layer_t: f32,
    submenu_layer_t: f32,
    top_motion_t: f32,
    submenu_motion_t: f32,
}

fn draw_settings_page_body_container(
    painter: &egui::Painter,
    rect: egui::Rect,
    layer_t: f32,
) {
    painter.rect_filled(
        rect,
        corner_radius(PANEL_CORNER_RADIUS),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(14, 14, 14, 255),
            layer_t,
        ),
    );
    painter.rect_stroke(
        rect,
        corner_radius(PANEL_CORNER_RADIUS),
        egui::Stroke::new(
            1.2,
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 42),
                layer_t,
            ),
        ),
        egui::StrokeKind::Middle,
    );
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
    title_tag: Option<&str>,
    title_tag_align_width: Option<f32>,
    title: &str,
    inline_button_title: Option<&InlineButtonTitle<'_>>,
    subtitle: Option<&str>,
    subtitle_color: Option<egui::Color32>,
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
    let title_font = egui::FontId::proportional(26.0);
    let title_color = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(242, 245, 248, 255),
        settings_t,
    );
    let title_galley = painter.layout_no_wrap(title.to_string(), title_font.clone(), title_color);
    let inline_prefix_galley = inline_button_title.map(|inline| {
        painter.layout_no_wrap(inline.prefix.to_string(), title_font.clone(), title_color)
    });
    let inline_suffix_galley = inline_button_title.map(|inline| {
        painter.layout_no_wrap(inline.suffix.to_string(), title_font.clone(), title_color)
    });
    let subtitle_galley = subtitle.map(|subtitle| {
        painter.layout_no_wrap(
            subtitle.to_string(),
            egui::FontId::proportional(19.0),
            color_with_scaled_alpha(
                subtitle_color
                    .unwrap_or(egui::Color32::from_rgba_unmultiplied(196, 202, 212, 220)),
                settings_t,
            ),
        )
    });
    let left_inline_icon_size = 30.0;
    let right_inline_icon_size = 36.0;
    let title_height = if let (Some(prefix), Some(suffix)) = (&inline_prefix_galley, &inline_suffix_galley) {
        prefix
            .size()
            .y
            .max(suffix.size().y)
            .max(left_inline_icon_size)
            .max(right_inline_icon_size)
    } else {
        title_galley.size().y
    };

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
    let title_x = text_start_x;
    if let Some(title_tag) = title_tag {
        let title_tag_gap = 16.0;
        let _ = title_tag_align_width;
        painter.galley(egui::pos2(title_x, title_y), title_galley.clone(), egui::Color32::WHITE);
        let badge_x = title_x + title_galley.size().x + title_tag_gap;
        draw_selected_game_text_badge(
            painter,
            title_tag,
            egui::pos2(badge_x, title_y),
            title_galley.size(),
            settings_t,
        );
    }
    if let (Some(inline_title), Some(prefix), Some(suffix)) = (
        inline_button_title,
        inline_prefix_galley,
        inline_suffix_galley,
    ) {
        let text_gap = 10.0;
        let icon_gap = 10.0;
        let mut cursor_x = title_x;
        let prefix_width = prefix.size().x;

        painter.galley(
            egui::pos2(cursor_x, title_y + (title_height - prefix.size().y) * 0.5),
            prefix,
            egui::Color32::WHITE,
        );
        cursor_x += prefix_width + text_gap;

        let left_icon_rect = egui::Rect::from_min_size(
            egui::pos2(cursor_x, title_y + (title_height - left_inline_icon_size) * 0.5),
            egui::vec2(left_inline_icon_size, left_inline_icon_size),
        );
        painter.image(
            inline_title.left_icon.id(),
            left_icon_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            color_with_scaled_alpha(egui::Color32::WHITE, settings_t),
        );
        cursor_x += left_inline_icon_size + icon_gap;

        let right_icon_rect = egui::Rect::from_min_size(
            egui::pos2(cursor_x, title_y + (title_height - right_inline_icon_size) * 0.5),
            egui::vec2(right_inline_icon_size, right_inline_icon_size),
        );
        painter.image(
            inline_title.right_icon.id(),
            right_icon_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            color_with_scaled_alpha(egui::Color32::WHITE, settings_t),
        );
        cursor_x += right_inline_icon_size;

        if !inline_title.suffix.is_empty() {
            cursor_x += text_gap;
            painter.galley(
                egui::pos2(cursor_x, title_y + (title_height - suffix.size().y) * 0.5),
                suffix,
                egui::Color32::WHITE,
            );
        }
    } else if title_tag.is_none() {
        painter.galley(egui::pos2(title_x, title_y), title_galley, egui::Color32::WHITE);
    }
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
    xbox_guide_icon: Option<&egui::TextureHandle>,
    playstation_home_icon: Option<&egui::TextureHandle>,
    launch_on_startup_enabled: bool,
    background_home_wake_enabled: bool,
    controller_vibration_enabled: bool,
    detect_steam_games_enabled: bool,
    detect_epic_games_enabled: bool,
    detect_xbox_games_enabled: bool,
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

    let draw_page_header = |content_rect: egui::Rect, layer_t: f32, title_text: &str| {
        let header_rect = egui::Rect::from_min_max(
            egui::pos2(content_rect.min.x, content_rect.min.y),
            egui::pos2(content_rect.max.x, content_rect.min.y + 78.0),
        );

        let title = painter.layout_no_wrap(
            title_text.to_string(),
            egui::FontId::proportional(34.0),
            color_with_scaled_alpha(egui::Color32::WHITE, layer_t),
        );
        let top_left = header_rect.min + egui::vec2(8.0, 8.0);
        painter.galley(top_left, title, egui::Color32::WHITE);

        header_rect
    };

    let draw_page_shell = |content_rect: egui::Rect, layer_t: f32, title_text: &str| {
        let header_rect = draw_page_header(content_rect, layer_t, title_text);
        let body_rect = egui::Rect::from_min_max(
            egui::pos2(content_rect.min.x, header_rect.max.y + 28.0),
            egui::pos2(content_rect.max.x, content_rect.max.y - 56.0),
        );
        draw_settings_page_body_container(painter, body_rect, layer_t);

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
                    title_tag: Option<&str>,
                    title_tag_align_width: Option<f32>,
                    title: &str,
                    use_inline_button_title: bool,
                    subtitle: Option<&str>,
                    subtitle_color: Option<egui::Color32>,
                    trailing: Option<&str>,
                    selected: bool,
                    show_focus_outline: bool,
                    layer_t: f32| {
        let inline_button_title = if use_inline_button_title {
            xbox_guide_icon.zip(playstation_home_icon).map(
                |(xbox_guide_icon, playstation_home_icon)| InlineButtonTitle {
                    prefix: language.background_home_wake_prefix_text(),
                    suffix: language.background_home_wake_suffix_text(),
                    left_icon: xbox_guide_icon,
                    right_icon: playstation_home_icon,
                },
            )
        } else {
            None
        };
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
            title_tag,
            title_tag_align_width,
            title,
            inline_button_title.as_ref(),
            subtitle,
            subtitle_color,
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
            None,
            None,
            language.system_text(),
            false,
            None,
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
            None,
            None,
            language.screen_text(),
            false,
            None,
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
            None,
            None,
            language.apps_text(),
            false,
            None,
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
            None,
            None,
            language.close_app_text(),
            false,
            None,
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
            1 => language.resolution_settings_text(),
            2 => language.installed_app_options_text(),
            _ => language.close_app_text(),
        };
        let page_title = format!("{} / {}", language.settings_text(), breadcrumb_section_name);
        let submenu_row_spacing = 112.0;
        let submenu_row_height = 98.0;
        let enabled_subtitle_color = egui::Color32::from_rgba_unmultiplied(122, 214, 145, 255);
        match selected_section_index {
            0 => {
                let header_rect = draw_page_header(submenu_content_rect, submenu_layer_t, &page_title);
                let system_row_spacing = submenu_row_height + 16.0;
                let section_gap = 28.0;
                let initial_row_offset_y = 8.0;
                let final_row_offset_y = 8.0;
                let body_top_padding = 14.0;
                let body_bottom_padding = 12.0;
                let body_height = system_row_spacing * 2.0
                    + submenu_row_height
                    + initial_row_offset_y
                    + final_row_offset_y
                    + body_top_padding
                    + body_bottom_padding;
                let top_body_rect = egui::Rect::from_min_max(
                    egui::pos2(submenu_content_rect.min.x, header_rect.max.y + 28.0),
                    egui::pos2(
                        submenu_content_rect.max.x,
                        header_rect.max.y + 28.0 + body_height,
                    ),
                );
                let lower_body_rect = egui::Rect::from_min_max(
                    egui::pos2(submenu_content_rect.min.x, top_body_rect.max.y + section_gap),
                    egui::pos2(
                        submenu_content_rect.max.x,
                        top_body_rect.max.y + section_gap + body_height,
                    ),
                );
                draw_settings_page_body_container(painter, top_body_rect, submenu_layer_t);
                draw_settings_page_body_container(painter, lower_body_rect, submenu_layer_t);
                let top_list_inner_rect = egui::Rect::from_min_max(
                    egui::pos2(top_body_rect.min.x + 6.0, top_body_rect.min.y + body_top_padding),
                    egui::pos2(top_body_rect.max.x - 6.0, top_body_rect.max.y - body_bottom_padding),
                );
                let lower_list_inner_rect = egui::Rect::from_min_max(
                    egui::pos2(lower_body_rect.min.x + 6.0, lower_body_rect.min.y + body_top_padding),
                    egui::pos2(lower_body_rect.max.x - 6.0, lower_body_rect.max.y - body_bottom_padding),
                );
                let submenu_list_painter = painter.with_clip_rect(top_list_inner_rect.union(lower_list_inner_rect));
                let system_rows_origin_y = top_list_inner_rect.min.y + initial_row_offset_y;
                let lower_rows_origin_y = lower_list_inner_rect.min.y + initial_row_offset_y;
                let detection_title_align_width = [GameSource::Steam, GameSource::Epic, GameSource::Xbox]
                    .into_iter()
                    .map(|source| {
                        measure_selected_game_text_badge(
                            &submenu_list_painter,
                            source.badge_label(),
                            egui::vec2(0.0, 26.0),
                        )
                        .x
                    })
                    .fold(0.0, f32::max);
                draw_row(
                    top_list_inner_rect,
                    system_rows_origin_y,
                    system_row_spacing,
                    submenu_row_height,
                    0,
                    None,
                    Some(GameSource::Steam.badge_label()),
                    Some(detection_title_align_width),
                    language.client_games_detection_text(),
                    false,
                    Some(if detect_steam_games_enabled {
                        language.enabled_text()
                    } else {
                        language.disabled_text()
                    }),
                    if detect_steam_games_enabled {
                        Some(enabled_subtitle_color)
                    } else {
                        None
                    },
                    None,
                    selected_item_index == 0,
                    true,
                    submenu_layer_t,
                );

                draw_row(
                    top_list_inner_rect,
                    system_rows_origin_y,
                    system_row_spacing,
                    submenu_row_height,
                    1,
                    None,
                    Some(GameSource::Epic.badge_label()),
                    Some(detection_title_align_width),
                    language.client_games_detection_text(),
                    false,
                    Some(if detect_epic_games_enabled {
                        language.enabled_text()
                    } else {
                        language.disabled_text()
                    }),
                    if detect_epic_games_enabled {
                        Some(enabled_subtitle_color)
                    } else {
                        None
                    },
                    None,
                    selected_item_index == 1,
                    true,
                    submenu_layer_t,
                );

                draw_row(
                    top_list_inner_rect,
                    system_rows_origin_y,
                    system_row_spacing,
                    submenu_row_height,
                    2,
                    None,
                    Some(GameSource::Xbox.badge_label()),
                    Some(detection_title_align_width),
                    language.client_games_detection_text(),
                    false,
                    Some(if detect_xbox_games_enabled {
                        language.enabled_text()
                    } else {
                        language.disabled_text()
                    }),
                    if detect_xbox_games_enabled {
                        Some(enabled_subtitle_color)
                    } else {
                        None
                    },
                    None,
                    selected_item_index == 2,
                    true,
                    submenu_layer_t,
                );

                draw_row(
                    lower_list_inner_rect,
                    lower_rows_origin_y,
                    system_row_spacing,
                    submenu_row_height,
                    0,
                    None,
                    None,
                    None,
                    "",
                    true,
                    Some(if background_home_wake_enabled {
                        language.enabled_text()
                    } else {
                        language.disabled_text()
                    }),
                    if background_home_wake_enabled {
                        Some(enabled_subtitle_color)
                    } else {
                        None
                    },
                    None,
                    selected_item_index == 3,
                    true,
                    submenu_layer_t,
                );

                draw_row(
                    lower_list_inner_rect,
                    lower_rows_origin_y,
                    system_row_spacing,
                    submenu_row_height,
                    1,
                    None,
                    None,
                    None,
                    language.controller_vibration_feedback_text(),
                    false,
                    Some(if controller_vibration_enabled {
                        language.enabled_text()
                    } else {
                        language.disabled_text()
                    }),
                    if controller_vibration_enabled {
                        Some(enabled_subtitle_color)
                    } else {
                        None
                    },
                    None,
                    selected_item_index == 4,
                    true,
                    submenu_layer_t,
                );

                draw_row(
                    lower_list_inner_rect,
                    lower_rows_origin_y,
                    system_row_spacing,
                    submenu_row_height,
                    2,
                    None,
                    None,
                    None,
                    language.launch_on_startup_text(),
                    false,
                    Some(if launch_on_startup_enabled {
                        language.enabled_text()
                    } else {
                        language.disabled_text()
                    }),
                    if launch_on_startup_enabled {
                        Some(enabled_subtitle_color)
                    } else {
                        None
                    },
                    None,
                    selected_item_index == 5,
                    true,
                    submenu_layer_t,
                );
            }
            1 => {
                let submenu_list_inner_rect = draw_page_shell(submenu_content_rect, submenu_layer_t, &page_title);
                let submenu_list_painter = painter.with_clip_rect(submenu_list_inner_rect);
                let header_rect = egui::Rect::from_min_max(
                    egui::pos2(submenu_list_inner_rect.min.x + 18.0, submenu_list_inner_rect.min.y + 8.0),
                    egui::pos2(submenu_list_inner_rect.max.x - 18.0, submenu_list_inner_rect.min.y + 96.0),
                );
                let rows_origin_y = submenu_list_inner_rect.min.y
                    + draw_settings_section_header(
                        &submenu_list_painter,
                        header_rect,
                        section_name,
                        summary_text.as_deref(),
                        submenu_layer_t,
                    )
                    + 16.0;
                draw_row(
                    submenu_list_inner_rect,
                    rows_origin_y,
                    submenu_row_spacing,
                    submenu_row_height,
                    0,
                    None,
                    None,
                    None,
                    &resolution_options.half_refresh.label,
                    false,
                    None,
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
                    None,
                    None,
                    &resolution_options.max_refresh.label,
                    false,
                    None,
                    None,
                    None,
                    selected_item_index == 1,
                    true,
                    submenu_layer_t,
                );
            }
            _ => {
                let submenu_list_inner_rect = draw_page_shell(submenu_content_rect, submenu_layer_t, &page_title);
                let submenu_list_painter = painter.with_clip_rect(submenu_list_inner_rect);
                let header_rect = egui::Rect::from_min_max(
                    egui::pos2(submenu_list_inner_rect.min.x + 18.0, submenu_list_inner_rect.min.y + 8.0),
                    egui::pos2(submenu_list_inner_rect.max.x - 18.0, submenu_list_inner_rect.min.y + 96.0),
                );
                let rows_origin_y = submenu_list_inner_rect.min.y
                    + draw_settings_section_header(
                        &submenu_list_painter,
                        header_rect,
                        section_name,
                        summary_text.as_deref(),
                        submenu_layer_t,
                    )
                    + 16.0;
                draw_row(
                    submenu_list_inner_rect,
                    rows_origin_y,
                    submenu_row_spacing,
                    submenu_row_height,
                    0,
                    None,
                    None,
                    None,
                    language.dlss_swapper_text(),
                    false,
                    None,
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
                    None,
                    None,
                    language.nvidia_app_text(),
                    false,
                    None,
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