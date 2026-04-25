use eframe::egui;

use crate::game::GameSource;
use crate::i18n::{AppLanguage, AppLanguageSetting};
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

const SETTINGS_BACKDROP_PHASE: f32 = 0.18;
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn build_time() -> &'static str {
    option_env!("BIG_SCREEN_LAUNCHER_BUILD_TIME").unwrap_or("unknown")
}

fn git_commit() -> &'static str {
    option_env!("BIG_SCREEN_LAUNCHER_GIT_COMMIT").unwrap_or("unknown")
}

fn should_force_top_level_entry_layer(
    show_settings_page: bool,
    settings_in_submenu: bool,
    submenu_t: f32,
    content_anim_t: f32,
) -> bool {
    show_settings_page
        && !settings_in_submenu
        && submenu_t <= 0.001
        && content_anim_t > 0.001
        && content_anim_t < 1.0
}

fn staged_settings_entry_progress(settings_anim: f32) -> (f32, f32) {
    let settings_anim = settings_anim.clamp(0.0, 1.0);
    let backdrop_t = smoothstep01((settings_anim / SETTINGS_BACKDROP_PHASE).clamp(0.0, 1.0));
    let content_t = ((settings_anim - SETTINGS_BACKDROP_PHASE) / (1.0 - SETTINGS_BACKDROP_PHASE))
        .clamp(0.0, 1.0);

    (backdrop_t, content_t)
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

fn draw_settings_focus_frame(
    painter: &egui::Painter,
    rect: egui::Rect,
    focus_t: f32,
    layer_t: f32,
) {
    if focus_t <= 0.001 {
        return;
    }

    let focus_rect = rect.expand(5.0);
    painter.rect_filled(
        focus_rect,
        corner_radius(12.0),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(248, 250, 255, 54),
            layer_t * focus_t,
        ),
    );
    painter.rect_stroke(
        focus_rect,
        corner_radius(12.0),
        egui::Stroke::new(
            lerp_f32(1.2, 3.0, focus_t),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 168),
                layer_t * focus_t,
            ),
        ),
        egui::StrokeKind::Outside,
    );
}

fn draw_settings_dropdown_button(
    painter: &egui::Painter,
    label_rect: egui::Rect,
    button_rect: egui::Rect,
    label: &str,
    value: &str,
    open: bool,
    settings_t: f32,
) {
    let label_color = if open {
        egui::Color32::from_rgba_unmultiplied(244, 247, 252, 255)
    } else {
        egui::Color32::from_rgba_unmultiplied(212, 219, 228, 240)
    };
    let label_galley = painter.layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(24.0),
        color_with_scaled_alpha(label_color, settings_t),
    );
    painter.galley(
        egui::pos2(label_rect.min.x, label_rect.center().y - label_galley.size().y * 0.5),
        label_galley,
        egui::Color32::WHITE,
    );

    let value_galley = painter.layout_no_wrap(
        value.to_string(),
        egui::FontId::proportional(22.0),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(236, 240, 246, 255),
            settings_t,
        ),
    );
    painter.galley(
        egui::pos2(
            button_rect.min.x + 22.0,
            button_rect.center().y - value_galley.size().y * 0.5,
        ),
        value_galley,
        egui::Color32::WHITE,
    );
}

fn draw_settings_dropdown_row(
    painter: &egui::Painter,
    row_rect: egui::Rect,
    label: &str,
    value: &str,
    selected: bool,
    open: bool,
    settings_t: f32,
    focus_t: f32,
) -> egui::Rect {
    painter.rect_filled(
        row_rect,
        corner_radius(9.0),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(206, 214, 224, 88),
            settings_t,
        ),
    );
    painter.rect_stroke(
        row_rect,
        corner_radius(9.0),
        egui::Stroke::new(
            1.0,
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 34),
                settings_t,
            ),
        ),
        egui::StrokeKind::Middle,
    );

    if selected && !open {
        draw_settings_focus_frame(painter, row_rect, focus_t, settings_t);
    }

    let inner_rect = row_rect.shrink2(egui::vec2(26.0, 16.0));
    let button_width = (inner_rect.width() * 0.46).clamp(320.0, 460.0);
    let button_rect = egui::Rect::from_min_size(
        egui::pos2(inner_rect.max.x - button_width, inner_rect.center().y - 29.0),
        egui::vec2(button_width, 58.0),
    );
    let label_rect = egui::Rect::from_min_max(
        egui::pos2(inner_rect.min.x, inner_rect.min.y),
        egui::pos2(button_rect.min.x - 24.0, inner_rect.max.y),
    );

    draw_settings_dropdown_button(
        painter,
        label_rect,
        button_rect,
        label,
        value,
        open,
        settings_t,
    );

    button_rect
}

fn draw_settings_dropdown_menu(
    painter: &egui::Painter,
    rect: egui::Rect,
    options: &[String],
    selected_index: usize,
    settings_t: f32,
    focus_t: f32,
) -> usize {
    let max_visible_items = 4;
    let visible_count = options.len().min(max_visible_items);
    if visible_count == 0 {
        return 0;
    }

    let selected_index = selected_index.min(options.len().saturating_sub(1));
    let max_start = options.len().saturating_sub(visible_count);
    let start_index = selected_index
        .saturating_sub(visible_count.saturating_sub(1))
        .min(max_start)
        .min(selected_index.saturating_sub(visible_count / 2));
    let end_index = start_index + visible_count;

    painter.rect_filled(
        rect,
        corner_radius(PANEL_CORNER_RADIUS),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(18, 19, 22, 255),
            settings_t,
        ),
    );
    painter.rect_stroke(
        rect,
        corner_radius(PANEL_CORNER_RADIUS),
        egui::Stroke::new(
            1.0,
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 36),
                settings_t,
            ),
        ),
        egui::StrokeKind::Middle,
    );

    let padding = egui::vec2(14.0, 14.0);
    let option_height = 64.0;
    let option_gap = 8.0;
    let option_font = egui::FontId::proportional(22.0);

    for (visible_index, option_index) in (start_index..end_index).enumerate() {
        let option = &options[option_index];
        let option_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.min.x + padding.x,
                rect.min.y + padding.y + visible_index as f32 * (option_height + option_gap),
            ),
            egui::vec2(rect.width() - padding.x * 2.0, option_height),
        );
        let selected = option_index == selected_index;
        draw_settings_focus_frame(
            painter,
            option_rect,
            if selected { focus_t } else { 0.0 },
            settings_t,
        );

        let option_galley = painter.layout_no_wrap(
            option.clone(),
            option_font.clone(),
            color_with_scaled_alpha(
                if selected {
                    egui::Color32::from_rgba_unmultiplied(248, 250, 255, 255)
                } else {
                    egui::Color32::from_rgba_unmultiplied(214, 218, 226, 255)
                },
                settings_t,
            ),
        );
        painter.galley(
            egui::pos2(
                option_rect.min.x + 22.0,
                option_rect.center().y - option_galley.size().y * 0.5,
            ),
            option_galley,
            egui::Color32::WHITE,
        );
    }

    if start_index > 0 {
        let mask_height = 42.0;
        let mut mesh = egui::epaint::Mesh::default();
        let base = egui::Color32::from_rgba_unmultiplied(14, 15, 18, 252);
        let transparent = egui::Color32::from_rgba_unmultiplied(18, 19, 22, 0);
        let top_rect = egui::Rect::from_min_max(
            egui::pos2(rect.min.x + 1.0, rect.min.y + 1.0),
            egui::pos2(rect.max.x - 1.0, rect.min.y + 1.0 + mask_height),
        );
        let index = mesh.vertices.len() as u32;
        mesh.vertices.push(egui::epaint::Vertex {
            pos: top_rect.left_top(),
            uv: egui::epaint::WHITE_UV,
            color: base,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: top_rect.right_top(),
            uv: egui::epaint::WHITE_UV,
            color: base,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: top_rect.right_bottom(),
            uv: egui::epaint::WHITE_UV,
            color: transparent,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: top_rect.left_bottom(),
            uv: egui::epaint::WHITE_UV,
            color: transparent,
        });
        mesh.indices
            .extend_from_slice(&[index, index + 1, index + 2, index, index + 2, index + 3]);
        painter.add(egui::Shape::mesh(mesh));
        painter.line_segment(
            [
                egui::pos2(rect.min.x + 12.0, top_rect.max.y - 2.0),
                egui::pos2(rect.max.x - 12.0, top_rect.max.y - 2.0),
            ],
            egui::Stroke::new(
                1.0,
                color_with_scaled_alpha(
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 42),
                    settings_t,
                ),
            ),
        );
    }

    if end_index < options.len() {
        let mask_height = 42.0;
        let mut mesh = egui::epaint::Mesh::default();
        let base = egui::Color32::from_rgba_unmultiplied(14, 15, 18, 252);
        let transparent = egui::Color32::from_rgba_unmultiplied(18, 19, 22, 0);
        let bottom_rect = egui::Rect::from_min_max(
            egui::pos2(rect.min.x + 1.0, rect.max.y - 1.0 - mask_height),
            egui::pos2(rect.max.x - 1.0, rect.max.y - 1.0),
        );
        let index = mesh.vertices.len() as u32;
        mesh.vertices.push(egui::epaint::Vertex {
            pos: bottom_rect.left_top(),
            uv: egui::epaint::WHITE_UV,
            color: transparent,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: bottom_rect.right_top(),
            uv: egui::epaint::WHITE_UV,
            color: transparent,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: bottom_rect.right_bottom(),
            uv: egui::epaint::WHITE_UV,
            color: base,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: bottom_rect.left_bottom(),
            uv: egui::epaint::WHITE_UV,
            color: base,
        });
        mesh.indices
            .extend_from_slice(&[index, index + 1, index + 2, index, index + 2, index + 3]);
        painter.add(egui::Shape::mesh(mesh));
        painter.line_segment(
            [
                egui::pos2(rect.min.x + 12.0, bottom_rect.min.y + 2.0),
                egui::pos2(rect.max.x - 12.0, bottom_rect.min.y + 2.0),
            ],
            egui::Stroke::new(
                1.0,
                color_with_scaled_alpha(
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 42),
                    settings_t,
                ),
            ),
        );
    }

    start_index
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
            if submenu_t <= 0.5 { settings_t } else { 0.0 },
            top_exit_t,
            if submenu_t > 0.5 { settings_t } else { 0.0 },
            submenu_enter_t,
        )
    } else {
        let top_enter_t = (1.0 - submenu_t * 2.0).clamp(0.0, 1.0);
        let submenu_exit_t = ((submenu_t - 0.5) * 2.0).clamp(0.0, 1.0);
        (
            if submenu_t < 0.5 { settings_t } else { 0.0 },
            1.0 - top_enter_t,
            if submenu_t >= 0.5 { settings_t } else { 0.0 },
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

    if show_focus_outline {
        draw_settings_focus_frame(painter, rect, focus_t, settings_t);
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

fn draw_settings_build_footer(
    painter: &egui::Painter,
    rect: egui::Rect,
    language: AppLanguage,
    settings_t: f32,
) {
    let (version_label, build_label, commit_label) = match language {
        AppLanguage::English => ("Version", "Build Time", "Commit"),
        AppLanguage::SimplifiedChinese => ("版本号", "构建时间", "提交"),
    };
    let footer_font = egui::FontId::proportional(15.0);
    let footer_color = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(188, 194, 203, 216),
        settings_t,
    );
    let version_galley = painter.layout_no_wrap(
        format!("{} {}", version_label, APP_VERSION),
        footer_font.clone(),
        footer_color,
    );
    let build_galley = painter.layout_no_wrap(
        format!("{} {}", build_label, build_time()),
        footer_font.clone(),
        footer_color,
    );
    let commit_galley = painter.layout_no_wrap(
        format!("{} {}", commit_label, git_commit()),
        footer_font,
        footer_color,
    );
    let line_gap = 4.0;
    let version_size = version_galley.size();
    let build_size = build_galley.size();
    let commit_size = commit_galley.size();
    let max_width = version_size.x.max(build_size.x).max(commit_size.x);
    let total_height = version_size.y
        + line_gap
        + build_size.y
        + line_gap
        + commit_size.y;
    let footer_origin = egui::pos2(
        rect.max.x - 26.0 - max_width,
        rect.max.y - 18.0 - total_height,
    );

    painter.galley(
        egui::pos2(
            footer_origin.x + max_width - version_size.x,
            footer_origin.y,
        ),
        version_galley,
        egui::Color32::WHITE,
    );
    painter.galley(
        egui::pos2(
            footer_origin.x + max_width - build_size.x,
            footer_origin.y + version_size.y + line_gap,
        ),
        build_galley,
        egui::Color32::WHITE,
    );
    painter.galley(
        egui::pos2(
            footer_origin.x + max_width - commit_size.x,
            footer_origin.y + total_height - commit_size.y,
        ),
        commit_galley,
        egui::Color32::WHITE,
    );
}

pub fn draw_settings_page(
    ui: &mut egui::Ui,
    language: AppLanguage,
    selected_language_setting: AppLanguageSetting,
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
    current_resolution_index: usize,
    current_refresh_index: usize,
    screen_resolution_dropdown_open: bool,
    screen_refresh_dropdown_open: bool,
    screen_dropdown_selected_index: usize,
    selected_section_index: usize,
    selected_item_index: usize,
    show_settings_page: bool,
    settings_in_submenu: bool,
    settings_anim: f32,
    submenu_anim: f32,
    screen_dropdown_overlay_anim: f32,
    settings_select_anim: f32,
) {
    let (backdrop_t, content_anim_t) = staged_settings_entry_progress(settings_anim);
    if backdrop_t <= 0.001 {
        return;
    }

    let settings_t = smoothstep01(content_anim_t);

    let scale_t = lerp_f32(0.94, 1.0, settings_t);

    let panel_rect = ui.available_rect_before_wrap();
    let painter = ui.painter();
    painter.rect_filled(
        panel_rect,
        egui::CornerRadius::ZERO,
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(18, 18, 18, 255),
            backdrop_t,
        ),
    );

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

    let layer_state = settings_layer_state(content_anim_t, submenu_t, settings_in_submenu);
    let top_layer_t = if should_force_top_level_entry_layer(
        show_settings_page,
        settings_in_submenu,
        submenu_t,
        content_anim_t,
    ) {
        1.0
    } else {
        layer_state.top_layer_t
    };
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
        draw_settings_build_footer(painter, top_list_inner_rect, language, top_layer_t);
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
                let top_body_height = system_row_spacing * 2.0
                    + submenu_row_height
                    + initial_row_offset_y
                    + final_row_offset_y
                    + body_top_padding
                    + body_bottom_padding;
                let lower_body_height = system_row_spacing * 3.0
                    + submenu_row_height
                    + initial_row_offset_y
                    + final_row_offset_y
                    + body_top_padding
                    + body_bottom_padding;
                let top_body_rect = egui::Rect::from_min_max(
                    egui::pos2(submenu_content_rect.min.x, header_rect.max.y + 28.0),
                    egui::pos2(
                        submenu_content_rect.max.x,
                        header_rect.max.y + 28.0 + top_body_height,
                    ),
                );
                let lower_body_rect = egui::Rect::from_min_max(
                    egui::pos2(submenu_content_rect.min.x, top_body_rect.max.y + section_gap),
                    egui::pos2(
                        submenu_content_rect.max.x,
                        top_body_rect.max.y + section_gap + lower_body_height,
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
                    language.language_setting_text(),
                    false,
                    Some(selected_language_setting.display_text(language)),
                    if selected_language_setting == AppLanguageSetting::Auto {
                        Some(enabled_subtitle_color)
                    } else {
                        None
                    },
                    None,
                    selected_item_index == 5,
                    true,
                    submenu_layer_t,
                );

                draw_row(
                    lower_list_inner_rect,
                    lower_rows_origin_y,
                    system_row_spacing,
                    submenu_row_height,
                    3,
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
                    selected_item_index == 6,
                    true,
                    submenu_layer_t,
                );
            }
            1 => {
                let submenu_list_inner_rect =
                    draw_page_shell(submenu_content_rect, submenu_layer_t, &page_title);
                let submenu_list_painter = painter.with_clip_rect(submenu_list_inner_rect);
                let header_rect = egui::Rect::from_min_max(
                    egui::pos2(
                        submenu_list_inner_rect.min.x + 18.0,
                        submenu_list_inner_rect.min.y + 8.0,
                    ),
                    egui::pos2(
                        submenu_list_inner_rect.max.x - 18.0,
                        submenu_list_inner_rect.min.y + 96.0,
                    ),
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
                let resolution_values: Vec<String> = resolution_options
                    .resolutions
                    .iter()
                    .map(|entry| entry.label.clone())
                    .collect();
                let refresh_values: Vec<String> = resolution_options
                    .resolutions
                    .get(current_resolution_index)
                    .map(|entry| {
                        entry
                            .refresh_rates
                            .iter()
                            .map(|refresh_hz| format!("{}Hz", refresh_hz))
                            .collect()
                    })
                    .unwrap_or_else(|| vec![format!("{}Hz", resolution_options.current.refresh_hz)]);
                let resolution_value = resolution_values
                    .get(current_resolution_index)
                    .map(String::as_str)
                    .unwrap_or(resolution_options.current.label.as_str());
                let current_refresh_value = format!("{}Hz", resolution_options.current.refresh_hz);
                let refresh_value = refresh_values
                    .get(current_refresh_index)
                    .map(String::as_str)
                    .unwrap_or(current_refresh_value.as_str());
                let focus_t = smoothstep01(settings_select_anim);
                let content_left = submenu_list_inner_rect.min.x + 18.0;
                let content_right = submenu_list_inner_rect.max.x - 18.0;
                let row_width = content_right - content_left;
                let row_height = 88.0;
                let row_gap = 18.0;
                let menu_gap = 14.0;
                let option_height = 64.0;
                let option_gap = 8.0;
                let menu_padding_y = 14.0;
                let max_visible_items = 4;
                let resolution_row_rect = egui::Rect::from_min_size(
                    egui::pos2(content_left, rows_origin_y),
                    egui::vec2(row_width, row_height),
                );
                let _ = draw_settings_dropdown_row(
                    &submenu_list_painter,
                    resolution_row_rect,
                    language.resolution_text(),
                    resolution_value,
                    selected_item_index == 0,
                    screen_resolution_dropdown_open,
                    submenu_layer_t,
                    focus_t,
                );
                let refresh_row_top = rows_origin_y + row_height + row_gap;
                let refresh_row_rect = egui::Rect::from_min_size(
                    egui::pos2(content_left, refresh_row_top),
                    egui::vec2(row_width, row_height),
                );
                let _ = draw_settings_dropdown_row(
                    &submenu_list_painter,
                    refresh_row_rect,
                    language.refresh_rate_text(),
                    refresh_value,
                    selected_item_index == 1,
                    screen_refresh_dropdown_open,
                    submenu_layer_t,
                    focus_t,
                );

                let dropdown_overlay_t = smoothstep01(screen_dropdown_overlay_anim);
                let dropdown_overlay_open = dropdown_overlay_t > 0.001;
                if dropdown_overlay_open {
                    painter.rect_filled(
                        panel_rect,
                        egui::CornerRadius::ZERO,
                        color_with_scaled_alpha(
                            egui::Color32::from_rgba_unmultiplied(6, 8, 12, 178),
                            submenu_layer_t * dropdown_overlay_t,
                        ),
                    );
                }

                if screen_resolution_dropdown_open {
                    let resolution_button_rect = draw_settings_dropdown_row(
                        &submenu_list_painter,
                        resolution_row_rect,
                        language.resolution_text(),
                        resolution_value,
                        selected_item_index == 0,
                        screen_resolution_dropdown_open,
                        submenu_layer_t,
                        focus_t,
                    );
                    let visible_count = resolution_values.len().min(max_visible_items);
                    let height = menu_padding_y * 2.0
                        + visible_count as f32 * option_height
                        + visible_count.saturating_sub(1) as f32 * option_gap;
                    let menu_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            resolution_button_rect.min.x,
                            resolution_button_rect.max.y + menu_gap,
                        ),
                        egui::vec2(resolution_button_rect.width(), height),
                    );
                    let _ = draw_settings_dropdown_menu(
                        &submenu_list_painter,
                        menu_rect,
                        &resolution_values,
                        screen_dropdown_selected_index.min(resolution_values.len().saturating_sub(1)),
                        submenu_layer_t,
                        focus_t,
                    );
                }

                if screen_refresh_dropdown_open {
                    let refresh_button_rect = draw_settings_dropdown_row(
                        &submenu_list_painter,
                        refresh_row_rect,
                        language.refresh_rate_text(),
                        refresh_value,
                        selected_item_index == 1,
                        screen_refresh_dropdown_open,
                        submenu_layer_t,
                        focus_t,
                    );
                    let visible_count = refresh_values.len().min(max_visible_items);
                    let height = menu_padding_y * 2.0
                        + visible_count as f32 * option_height
                        + visible_count.saturating_sub(1) as f32 * option_gap;
                    let menu_rect = egui::Rect::from_min_size(
                        egui::pos2(refresh_button_rect.min.x, refresh_button_rect.max.y + menu_gap),
                        egui::vec2(refresh_button_rect.width(), height),
                    );
                    let _ = draw_settings_dropdown_menu(
                        &submenu_list_painter,
                        menu_rect,
                        &refresh_values,
                        screen_dropdown_selected_index.min(refresh_values.len().saturating_sub(1)),
                        submenu_layer_t,
                        focus_t,
                    );
                }
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
    use super::{
        settings_layer_state, should_force_top_level_entry_layer, staged_settings_entry_progress,
        SETTINGS_BACKDROP_PHASE,
    };

    #[test]
    fn top_level_entry_force_only_applies_to_initial_open() {
        assert!(should_force_top_level_entry_layer(true, false, 0.0, 0.3));
        assert!(!should_force_top_level_entry_layer(true, false, 0.4, 0.3));
        assert!(!should_force_top_level_entry_layer(true, true, 0.0, 0.3));
        assert!(!should_force_top_level_entry_layer(false, false, 0.0, 0.3));
    }

    #[test]
    fn settings_entry_uses_backdrop_before_content() {
        let (backdrop_t, content_t) = staged_settings_entry_progress(SETTINGS_BACKDROP_PHASE * 0.5);

        assert!(backdrop_t > 0.0);
        assert!(content_t.abs() < f32::EPSILON);
    }

    #[test]
    fn settings_entry_starts_content_after_backdrop_phase() {
        let (backdrop_t, content_t) = staged_settings_entry_progress(SETTINGS_BACKDROP_PHASE + 0.1);

        assert!((backdrop_t - 1.0).abs() < f32::EPSILON);
        assert!(content_t > 0.0);
    }

    #[test]
    fn returning_from_submenu_hides_top_level_until_exit_finishes() {
        let layer_state = settings_layer_state(1.0, 0.6, false);

        assert!(layer_state.top_layer_t.abs() < f32::EPSILON);
        assert!((layer_state.top_motion_t - 1.0).abs() < f32::EPSILON);
        assert!((layer_state.submenu_motion_t - 0.2).abs() < 1e-6);
        assert!((layer_state.submenu_layer_t - 1.0).abs() < f32::EPSILON);
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

        assert!((layer_state.top_layer_t - 1.0).abs() < f32::EPSILON);
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
        assert!((layer_state.submenu_layer_t - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn entering_submenu_midpoint_keeps_top_level_visible_without_crossfade() {
        let layer_state = settings_layer_state(1.0, 0.5, true);

        assert!((layer_state.top_layer_t - 1.0).abs() < f32::EPSILON);
        assert!(layer_state.submenu_layer_t.abs() < f32::EPSILON);
    }

    #[test]
    fn returning_from_submenu_midpoint_keeps_submenu_visible_without_crossfade() {
        let layer_state = settings_layer_state(1.0, 0.5, false);

        assert!(layer_state.top_layer_t.abs() < f32::EPSILON);
        assert!((layer_state.submenu_layer_t - 1.0).abs() < f32::EPSILON);
    }
}