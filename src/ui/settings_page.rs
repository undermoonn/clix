use eframe::egui;

use crate::config::BackgroundHomeWakeMode;
use crate::game::GameSource;
use crate::i18n::{AppLanguage, AppLanguageSetting};
use crate::system::display_mode::{DisplayModeSetting, DisplayScaleOptions, ResolutionOptions};

use super::header::{draw_selected_game_text_badge, measure_selected_game_text_badge};
use super::text::{color_with_scaled_alpha, corner_radius, PANEL_CORNER_RADIUS};
use super::{design_units, lerp_f32, smoothstep01, viewport_layout_scale};

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
const OPEN_DROPDOWN_PARENT_FOCUS_T: f32 = 0.32;
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const SETTINGS_SCROLL_ANIM_SPEED: f32 = 14.0;
const SETTINGS_SCROLL_EPSILON: f32 = 0.5;
const SETTINGS_PAGE_HEADER_HEIGHT: f32 = 78.0;
const SETTINGS_PAGE_HEADER_GAP: f32 = 28.0;
const SETTINGS_DROPDOWN_MASK_ANIM_SPEED: f32 = 18.0;
const SETTINGS_DROPDOWN_MASK_EPSILON: f32 = 0.01;
const SETTINGS_DROPDOWN_MASK_MAX_DT: f32 = 1.0 / 60.0;

fn settings_header_height(layout_scale: f32) -> f32 {
    design_units(SETTINGS_PAGE_HEADER_HEIGHT, layout_scale)
}

fn settings_header_gap(layout_scale: f32) -> f32 {
    design_units(SETTINGS_PAGE_HEADER_GAP, layout_scale)
}

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
    layout_scale: f32,
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
            design_units(1.2, layout_scale),
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
    layout_scale: f32,
) {
    if focus_t <= 0.001 {
        return;
    }

    let focus_rect = rect.expand(design_units(5.0, layout_scale));
    painter.rect_filled(
        focus_rect,
        corner_radius(design_units(12.0, layout_scale)),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(248, 250, 255, 54),
            layer_t * focus_t,
        ),
    );
    painter.rect_stroke(
        focus_rect,
        corner_radius(design_units(12.0, layout_scale)),
        egui::Stroke::new(
            lerp_f32(
                design_units(1.2, layout_scale),
                design_units(3.0, layout_scale),
                focus_t,
            ),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 168),
                layer_t * focus_t,
            ),
        ),
        egui::StrokeKind::Outside,
    );
}

fn draw_settings_dropdown_trigger_pill(
    painter: &egui::Painter,
    rect: egui::Rect,
    value: &str,
    open: bool,
    settings_t: f32,
    layout_scale: f32,
) {
    let value_galley = painter.layout_no_wrap(
        value.to_string(),
        egui::FontId::proportional(design_units(20.0, layout_scale)),
        color_with_scaled_alpha(
            if open {
                egui::Color32::from_rgba_unmultiplied(248, 250, 255, 255)
            } else {
                egui::Color32::from_rgba_unmultiplied(232, 236, 242, 240)
            },
            settings_t,
        ),
    );
    painter.galley(
        egui::pos2(
            rect.min.x + design_units(24.0, layout_scale),
            rect.center().y - value_galley.size().y * 0.5,
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
    layout_scale: f32,
) -> egui::Rect {
    painter.rect_filled(
        row_rect,
        corner_radius(design_units(9.0, layout_scale)),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(206, 214, 224, 88),
            settings_t,
        ),
    );
    painter.rect_stroke(
        row_rect,
        corner_radius(design_units(9.0, layout_scale)),
        egui::Stroke::new(
            design_units(1.0, layout_scale),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 34),
                settings_t,
            ),
        ),
        egui::StrokeKind::Middle,
    );

    let effective_focus_t = if open {
        OPEN_DROPDOWN_PARENT_FOCUS_T
    } else {
        focus_t
    };

    if selected || open || effective_focus_t > 0.001 {
        if open {
            draw_settings_focus_frame(
                painter,
                row_rect,
                lerp_f32(0.45, 1.0, effective_focus_t),
                settings_t * lerp_f32(0.2, 0.35, effective_focus_t),
                layout_scale,
            );
        } else {
            draw_settings_focus_frame(
                painter,
                row_rect,
                effective_focus_t,
                settings_t,
                layout_scale,
            );
        }
    }

    let content_rect = row_rect.shrink2(egui::vec2(
        design_units(34.0, layout_scale),
        design_units(18.0, layout_scale),
    ));
    let pill_height = design_units(50.0, layout_scale);
    let pill_width = (content_rect.width() * 0.36).clamp(
        design_units(220.0, layout_scale),
        design_units(320.0, layout_scale),
    );
    let pill_rect = egui::Rect::from_min_size(
        egui::pos2(
            content_rect.max.x - pill_width,
            content_rect.center().y - pill_height * 0.5,
        ),
        egui::vec2(pill_width, pill_height),
    );

    let label_color = if open {
        egui::Color32::from_rgba_unmultiplied(244, 247, 252, 255)
    } else {
        egui::Color32::from_rgba_unmultiplied(242, 245, 248, 255)
    };
    let title_galley = painter.layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(design_units(26.0, layout_scale)),
        color_with_scaled_alpha(label_color, settings_t),
    );
    painter.galley(
        egui::pos2(
            content_rect.min.x,
            content_rect.center().y - title_galley.size().y * 0.5,
        ),
        title_galley,
        egui::Color32::WHITE,
    );

    draw_settings_dropdown_trigger_pill(painter, pill_rect, value, open, settings_t, layout_scale);

    pill_rect
}

fn draw_settings_dropdown_menu(
    painter: &egui::Painter,
    rect: egui::Rect,
    options: &[String],
    base_focus_key: u16,
    selected_index: usize,
    text_left_padding: f32,
    settings_t: f32,
    current_focus_t: f32,
    current_focus_key: Option<u16>,
    layout_scale: f32,
) -> usize {
    let max_visible_items = 4;
    let menu_color = egui::Color32::from_rgba_unmultiplied(18, 19, 22, 255);
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
    let menu_t = settings_t;

    painter.rect_filled(
        rect,
        corner_radius(PANEL_CORNER_RADIUS),
        color_with_scaled_alpha(menu_color, menu_t),
    );
    painter.rect_stroke(
        rect,
        corner_radius(PANEL_CORNER_RADIUS),
        egui::Stroke::new(
            design_units(1.0, layout_scale),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 36),
                menu_t,
            ),
        ),
        egui::StrokeKind::Middle,
    );

    let padding = egui::vec2(
        design_units(14.0, layout_scale),
        design_units(14.0, layout_scale),
    );
    let option_height = design_units(64.0, layout_scale);
    let option_gap = design_units(8.0, layout_scale);
    let option_font = egui::FontId::proportional(design_units(22.0, layout_scale));
    let option_width = rect.width() - padding.x * 2.0;

    for (visible_index, option_index) in (start_index..end_index).enumerate() {
        let option = &options[option_index];
        let option_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.min.x + padding.x,
                rect.min.y + padding.y + visible_index as f32 * (option_height + option_gap),
            ),
            egui::vec2(option_width, option_height),
        );
        let option_focus_key = base_focus_key + option_index as u16;
        let selected = option_index == selected_index;
        draw_settings_focus_frame(
            painter,
            option_rect,
            settings_focus_t_for_key(option_focus_key, current_focus_key, current_focus_t),
            menu_t,
            layout_scale,
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
                menu_t,
            ),
        );
        painter.galley(
            egui::pos2(
                rect.min.x + text_left_padding,
                option_rect.center().y - option_galley.size().y * 0.5,
            ),
            option_galley,
            egui::Color32::WHITE,
        );
    }

    start_index
}

fn settings_dropdown_menu_rect(
    button_rect: egui::Rect,
    menu_height: f32,
    menu_gap: f32,
    viewport_rect: egui::Rect,
    layout_scale: f32,
) -> egui::Rect {
    let viewport_margin = design_units(8.0, layout_scale);
    let min_top = viewport_rect.min.y + viewport_margin;
    let max_top = (viewport_rect.max.y - viewport_margin - menu_height).max(min_top);
    let open_below_top = button_rect.max.y + menu_gap;
    let open_above_top = button_rect.min.y - menu_gap - menu_height;

    let top = if open_below_top + menu_height <= viewport_rect.max.y - viewport_margin {
        open_below_top
    } else if open_above_top >= min_top {
        open_above_top
    } else {
        open_below_top.clamp(min_top, max_top)
    };

    egui::Rect::from_min_size(
        egui::pos2(button_rect.min.x, top),
        egui::vec2(button_rect.width(), menu_height),
    )
}

fn animate_settings_dropdown_mask(
    ui: &egui::Ui,
    mask_id: egui::Id,
    open: bool,
    active_rect: Option<egui::Rect>,
    fallback_rect: egui::Rect,
) -> (f32, egui::Rect) {
    let stable_dt = ui
        .ctx()
        .input(|input| input.stable_dt)
        .min(SETTINGS_DROPDOWN_MASK_MAX_DT);
    let alpha_id = mask_id.with("alpha");
    let rect_id = mask_id.with("rect");
    let mut should_repaint = false;

    let (alpha, rect) = ui.ctx().data_mut(|data| {
        let current_alpha = data.get_temp::<f32>(alpha_id).unwrap_or(0.0);
        let target_alpha = if open { 1.0 } else { 0.0 };
        let next_alpha = if (current_alpha - target_alpha).abs() <= SETTINGS_DROPDOWN_MASK_EPSILON {
            target_alpha
        } else {
            let next_alpha = eased_scroll_offset(
                current_alpha,
                target_alpha,
                stable_dt,
                SETTINGS_DROPDOWN_MASK_ANIM_SPEED,
            );
            if (next_alpha - target_alpha).abs() > SETTINGS_DROPDOWN_MASK_EPSILON {
                should_repaint = true;
                next_alpha
            } else {
                target_alpha
            }
        };

        let rect = active_rect
            .or_else(|| data.get_temp::<egui::Rect>(rect_id))
            .unwrap_or(fallback_rect);
        data.insert_temp(rect_id, rect);
        data.insert_temp(alpha_id, next_alpha);

        (next_alpha, rect)
    });

    if should_repaint {
        ui.ctx().request_repaint();
    }

    (alpha, rect)
}

fn draw_settings_dropdown_mask(
    painter: &egui::Painter,
    viewport_rect: egui::Rect,
    clear_rect: Option<egui::Rect>,
    mask_t: f32,
    settings_t: f32,
    layout_scale: f32,
) {
    let mask_t = (mask_t * settings_t).clamp(0.0, 1.0);
    if mask_t <= 0.001 {
        return;
    }

    let mask_color =
        color_with_scaled_alpha(egui::Color32::from_rgba_unmultiplied(6, 7, 9, 168), mask_t);

    let Some(clear_rect) = clear_rect.map(|clear_rect| {
        intersect_rects(
            clear_rect.expand(design_units(6.0, layout_scale)),
            viewport_rect,
        )
    }) else {
        painter.rect_filled(viewport_rect, egui::CornerRadius::ZERO, mask_color);
        return;
    };

    if clear_rect.min.y > viewport_rect.min.y {
        painter.rect_filled(
            egui::Rect::from_min_max(
                viewport_rect.min,
                egui::pos2(viewport_rect.max.x, clear_rect.min.y),
            ),
            egui::CornerRadius::ZERO,
            mask_color,
        );
    }
    if clear_rect.max.y < viewport_rect.max.y {
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(viewport_rect.min.x, clear_rect.max.y),
                viewport_rect.max,
            ),
            egui::CornerRadius::ZERO,
            mask_color,
        );
    }
    if clear_rect.min.x > viewport_rect.min.x {
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(viewport_rect.min.x, clear_rect.min.y),
                egui::pos2(clear_rect.min.x, clear_rect.max.y),
            ),
            egui::CornerRadius::ZERO,
            mask_color,
        );
    }
    if clear_rect.max.x < viewport_rect.max.x {
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(clear_rect.max.x, clear_rect.min.y),
                egui::pos2(viewport_rect.max.x, clear_rect.max.y),
            ),
            egui::CornerRadius::ZERO,
            mask_color,
        );
    }
}

fn settings_focus_t_for_key(
    focus_key: u16,
    current_focus_key: Option<u16>,
    current_focus_t: f32,
) -> f32 {
    if current_focus_key == Some(focus_key) {
        current_focus_t
    } else {
        0.0
    }
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

fn intersect_rects(a: egui::Rect, b: egui::Rect) -> egui::Rect {
    let min_x = a.min.x.max(b.min.x);
    let min_y = a.min.y.max(b.min.y);
    let max_x = a.max.x.min(b.max.x).max(min_x);
    let max_y = a.max.y.min(b.max.y).max(min_y);

    egui::Rect::from_min_max(egui::pos2(min_x, min_y), egui::pos2(max_x, max_y))
}

fn scroll_offset_to_keep_focus_visible(
    content_height: f32,
    viewport_height: f32,
    focus_top: f32,
    focus_height: f32,
    top_padding: f32,
    bottom_padding: f32,
) -> f32 {
    let max_scroll = (content_height - viewport_height).max(0.0);
    if max_scroll <= 0.0 {
        return 0.0;
    }

    let visible_top = top_padding.max(0.0);
    let visible_bottom = (viewport_height - bottom_padding).max(visible_top);
    let focus_bottom = focus_top + focus_height;
    let mut scroll_offset = 0.0;

    if focus_bottom > visible_bottom {
        scroll_offset = focus_bottom - visible_bottom;
    }
    if focus_top - scroll_offset < visible_top {
        scroll_offset = (focus_top - visible_top).max(0.0);
    }

    scroll_offset.clamp(0.0, max_scroll)
}

fn system_settings_row_top(
    selected_item_index: usize,
    header_height: f32,
    header_gap: f32,
    top_body_height: f32,
    section_gap: f32,
    body_top_padding: f32,
    initial_row_offset_y: f32,
    row_spacing: f32,
) -> f32 {
    let body_top = header_height + header_gap;
    let row_index = selected_item_index.min(7);
    if row_index <= 2 {
        body_top + body_top_padding + initial_row_offset_y + row_index as f32 * row_spacing
    } else {
        body_top
            + top_body_height
            + section_gap
            + body_top_padding
            + initial_row_offset_y
            + (row_index - 3) as f32 * row_spacing
    }
}

fn draw_settings_page_header(
    painter: &egui::Painter,
    content_rect: egui::Rect,
    layer_t: f32,
    title_text: &str,
    layout_scale: f32,
) -> egui::Rect {
    let header_rect = egui::Rect::from_min_max(
        egui::pos2(content_rect.min.x, content_rect.min.y),
        egui::pos2(
            content_rect.max.x,
            content_rect.min.y + settings_header_height(layout_scale),
        ),
    );

    let title = painter.layout_no_wrap(
        title_text.to_string(),
        egui::FontId::proportional(design_units(34.0, layout_scale)),
        color_with_scaled_alpha(egui::Color32::WHITE, layer_t),
    );
    let top_left = header_rect.min
        + egui::vec2(
            design_units(8.0, layout_scale),
            design_units(8.0, layout_scale),
        );
    painter.galley(top_left, title, egui::Color32::WHITE);

    header_rect
}

fn eased_scroll_offset(current: f32, target: f32, stable_dt: f32, speed: f32) -> f32 {
    if !current.is_finite() || !target.is_finite() {
        return target;
    }

    let stable_dt = stable_dt.max(0.0);
    if stable_dt <= f32::EPSILON || speed <= 0.0 {
        return current;
    }

    let blend = 1.0 - (-speed * stable_dt).exp();
    lerp_f32(current, target, blend.clamp(0.0, 1.0))
}

fn animate_settings_scroll_offset(
    ui: &egui::Ui,
    scroll_id: egui::Id,
    target: f32,
    allow_animation: bool,
) -> f32 {
    let stable_dt = if allow_animation {
        ui.ctx().input(|input| input.stable_dt)
    } else {
        0.0
    };
    let mut should_repaint = false;

    let next = ui.ctx().data_mut(|data| {
        let current = data.get_temp::<f32>(scroll_id).unwrap_or(target);
        let next = if allow_animation {
            let next = eased_scroll_offset(current, target, stable_dt, SETTINGS_SCROLL_ANIM_SPEED);
            if (next - target).abs() > SETTINGS_SCROLL_EPSILON {
                should_repaint = true;
                next
            } else {
                target
            }
        } else {
            target
        };
        data.insert_temp(scroll_id, next);
        next
    });

    if should_repaint {
        ui.ctx().request_repaint();
    }

    next
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
    layout_scale: f32,
) {
    let card_corner = corner_radius(design_units(9.0, layout_scale));
    let fill = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(206, 214, 224, 88),
        settings_t,
    );
    painter.rect_filled(rect, card_corner, fill);
    painter.rect_stroke(
        rect,
        card_corner,
        egui::Stroke::new(
            design_units(1.0, layout_scale),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 34),
                settings_t,
            ),
        ),
        egui::StrokeKind::Middle,
    );

    if show_focus_outline {
        draw_settings_focus_frame(painter, rect, focus_t, settings_t, layout_scale);
    }

    let content_padding_x = design_units(34.0, layout_scale);
    let content_padding_y = design_units(18.0, layout_scale);
    let content_rect = rect.shrink2(egui::vec2(content_padding_x, content_padding_y));
    let icon_slot_size = design_units(54.0, layout_scale);
    let icon_render_size = design_units(36.0, layout_scale);
    let icon_gap = design_units(26.0, layout_scale);
    let text_start_x = if let Some(icon) = leading_icon {
        let icon_badge_center = egui::pos2(
            content_rect.min.x + icon_slot_size * 0.5,
            content_rect.center().y,
        );
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
    let title_font = egui::FontId::proportional(design_units(26.0, layout_scale));
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
            egui::FontId::proportional(design_units(19.0, layout_scale)),
            color_with_scaled_alpha(
                subtitle_color.unwrap_or(egui::Color32::from_rgba_unmultiplied(196, 202, 212, 220)),
                settings_t,
            ),
        )
    });
    let left_inline_icon_size = design_units(30.0, layout_scale);
    let right_inline_icon_size = design_units(36.0, layout_scale);
    let title_height =
        if let (Some(prefix), Some(suffix)) = (&inline_prefix_galley, &inline_suffix_galley) {
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
            egui::FontId::proportional(design_units(15.0, layout_scale)),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(232, 236, 242, 220),
                settings_t,
            ),
        );
        let badge_size = status_galley.size()
            + egui::vec2(
                design_units(24.0, layout_scale),
                design_units(12.0, layout_scale),
            );
        let badge_rect = egui::Rect::from_min_size(
            egui::pos2(content_rect.max.x - badge_size.x, content_rect.min.y),
            badge_size,
        );
        painter.rect_filled(
            badge_rect,
            corner_radius(design_units(7.0, layout_scale)),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(40, 42, 46, 184),
                settings_t,
            ),
        );
        painter.rect_stroke(
            badge_rect,
            corner_radius(design_units(7.0, layout_scale)),
            egui::Stroke::new(
                design_units(1.0, layout_scale),
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

    let subtitle_gap = design_units(8.0, layout_scale);
    let text_block_height = if let Some(subtitle_galley) = &subtitle_galley {
        title_height + subtitle_gap + subtitle_galley.size().y
    } else {
        title_height
    };
    let title_y = content_rect.center().y - text_block_height * 0.5;
    let title_x = text_start_x;
    if let Some(title_tag) = title_tag {
        let title_tag_gap = design_units(16.0, layout_scale);
        let _ = title_tag_align_width;
        painter.galley(
            egui::pos2(title_x, title_y),
            title_galley.clone(),
            egui::Color32::WHITE,
        );
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
        let text_gap = design_units(10.0, layout_scale);
        let icon_gap = design_units(10.0, layout_scale);
        let mut cursor_x = title_x;
        let prefix_width = prefix.size().x;

        painter.galley(
            egui::pos2(cursor_x, title_y + (title_height - prefix.size().y) * 0.5),
            prefix,
            egui::Color32::WHITE,
        );
        cursor_x += prefix_width + text_gap;

        let left_icon_rect = egui::Rect::from_min_size(
            egui::pos2(
                cursor_x,
                title_y + (title_height - left_inline_icon_size) * 0.5,
            ),
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
            egui::pos2(
                cursor_x,
                title_y + (title_height - right_inline_icon_size) * 0.5,
            ),
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
        painter.galley(
            egui::pos2(title_x, title_y),
            title_galley,
            egui::Color32::WHITE,
        );
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
    layout_scale: f32,
) -> f32 {
    let title_galley = painter.layout_no_wrap(
        title.to_string(),
        egui::FontId::proportional(design_units(24.0, layout_scale)),
        color_with_scaled_alpha(egui::Color32::WHITE, settings_t),
    );
    let title_height = title_galley.size().y;
    painter.galley(rect.min, title_galley, egui::Color32::WHITE);

    let mut height = design_units(36.0, layout_scale);
    if let Some(subtitle) = subtitle {
        let subtitle_galley = painter.layout_no_wrap(
            subtitle.to_string(),
            egui::FontId::proportional(design_units(18.0, layout_scale)),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(178, 184, 192, 210),
                settings_t,
            ),
        );
        let subtitle_height = subtitle_galley.size().y;
        let subtitle_y = rect.min.y + title_height + design_units(10.0, layout_scale);
        painter.galley(
            egui::pos2(rect.min.x, subtitle_y),
            subtitle_galley,
            egui::Color32::WHITE,
        );
        height = title_height + subtitle_height + design_units(18.0, layout_scale);
    }

    painter.line_segment(
        [
            egui::pos2(rect.min.x, rect.min.y + height + 10.0),
            egui::pos2(
                rect.max.x,
                rect.min.y + height + design_units(10.0, layout_scale),
            ),
        ],
        egui::Stroke::new(
            design_units(1.0, layout_scale),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
                settings_t,
            ),
        ),
    );

    height + design_units(22.0, layout_scale)
}

fn draw_settings_build_footer(
    painter: &egui::Painter,
    rect: egui::Rect,
    language: AppLanguage,
    settings_t: f32,
    layout_scale: f32,
) {
    let (version_label, build_label, commit_label) = match language {
        AppLanguage::English => ("Version", "Build Time", "Commit"),
        AppLanguage::SimplifiedChinese => ("版本号", "构建时间", "提交"),
    };
    let footer_font = egui::FontId::proportional(design_units(15.0, layout_scale));
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
    let line_gap = design_units(4.0, layout_scale);
    let version_size = version_galley.size();
    let build_size = build_galley.size();
    let commit_size = commit_galley.size();
    let max_width = version_size.x.max(build_size.x).max(commit_size.x);
    let total_height = version_size.y + line_gap + build_size.y + line_gap + commit_size.y;
    let footer_origin = egui::pos2(
        rect.max.x - design_units(26.0, layout_scale) - max_width,
        rect.max.y - design_units(18.0, layout_scale) - total_height,
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
    selected_display_mode_setting: DisplayModeSetting,
    system_icon: Option<&egui::TextureHandle>,
    screen_icon: Option<&egui::TextureHandle>,
    apps_icon: Option<&egui::TextureHandle>,
    exit_icon: Option<&egui::TextureHandle>,
    xbox_guide_icon: Option<&egui::TextureHandle>,
    playstation_home_icon: Option<&egui::TextureHandle>,
    launch_on_startup_enabled: bool,
    background_home_wake_mode: BackgroundHomeWakeMode,
    controller_vibration_enabled: bool,
    detect_steam_games_enabled: bool,
    detect_epic_games_enabled: bool,
    detect_xbox_games_enabled: bool,
    resolution_options: &ResolutionOptions,
    display_scale_options: &DisplayScaleOptions,
    current_resolution_index: usize,
    current_refresh_index: usize,
    current_scale_index: usize,
    screen_resolution_dropdown_open: bool,
    screen_refresh_dropdown_open: bool,
    screen_scale_dropdown_open: bool,
    screen_dropdown_selected_index: usize,
    selected_section_index: usize,
    selected_item_index: usize,
    show_settings_page: bool,
    settings_in_submenu: bool,
    settings_anim: f32,
    submenu_anim: f32,
    settings_select_anim: f32,
    current_settings_focus_key: Option<u16>,
) {
    let (backdrop_t, content_anim_t) = staged_settings_entry_progress(settings_anim);
    if backdrop_t <= 0.001 {
        return;
    }

    let settings_t = smoothstep01(content_anim_t);

    let scale_t = lerp_f32(0.94, 1.0, settings_t);

    let panel_rect = ui.available_rect_before_wrap();
    let layout_scale = viewport_layout_scale(panel_rect);
    let su = |value: f32| design_units(value, layout_scale);
    let header_height = settings_header_height(layout_scale);
    let header_gap = settings_header_gap(layout_scale);
    let painter = ui.painter();
    painter.rect_filled(
        panel_rect,
        egui::CornerRadius::ZERO,
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(18, 18, 18, 255),
            backdrop_t,
        ),
    );

    let base_content_rect = panel_rect.shrink2(egui::vec2(su(52.0), su(52.0)));
    let page_rect = egui::Rect::from_center_size(
        base_content_rect.center(),
        base_content_rect.size() * scale_t,
    )
    .translate(egui::vec2(0.0, lerp_f32(su(18.0), 0.0, settings_t)));
    let submenu_t = smoothstep01(submenu_anim);
    let current_focus_t = smoothstep01(settings_select_anim);
    let row_focus_t = |row_key: u16| {
        settings_focus_t_for_key(row_key, current_settings_focus_key, current_focus_t)
    };

    let draw_page_header = |content_rect: egui::Rect, layer_t: f32, title_text: &str| {
        draw_settings_page_header(painter, content_rect, layer_t, title_text, layout_scale)
    };

    let draw_page_shell = |content_rect: egui::Rect, layer_t: f32, title_text: &str| {
        let header_rect = draw_page_header(content_rect, layer_t, title_text);
        let body_rect = egui::Rect::from_min_max(
            egui::pos2(content_rect.min.x, header_rect.max.y + header_gap),
            egui::pos2(content_rect.max.x, content_rect.max.y - su(56.0)),
        );
        draw_settings_page_body_container(painter, body_rect, layer_t, layout_scale);

        egui::Rect::from_min_max(
            egui::pos2(body_rect.min.x + su(6.0), body_rect.min.y + su(14.0)),
            egui::pos2(body_rect.max.x - su(6.0), body_rect.max.y - su(16.0)),
        )
    };

    let draw_row = |list_inner_rect: egui::Rect,
                    viewport_clip_rect: Option<egui::Rect>,
                    rows_origin_y: f32,
                    row_spacing: f32,
                    row_height: f32,
                    index: usize,
                    row_key: u16,
                    leading_icon: Option<&egui::TextureHandle>,
                    title_tag: Option<&str>,
                    title_tag_align_width: Option<f32>,
                    title: &str,
                    use_inline_button_title: bool,
                    subtitle: Option<&str>,
                    subtitle_color: Option<egui::Color32>,
                    trailing: Option<&str>,
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
        let clip_rect = viewport_clip_rect
            .map(|viewport_clip_rect| intersect_rects(list_inner_rect, viewport_clip_rect))
            .unwrap_or(list_inner_rect);
        let list_painter = painter.with_clip_rect(clip_rect);
        let row_side_inset = su(6.0);
        let unselected_row_shrink_x = su(7.0);
        let row_slot_rect = egui::Rect::from_min_max(
            egui::pos2(list_inner_rect.min.x + row_side_inset, row_top),
            egui::pos2(list_inner_rect.max.x - row_side_inset, row_top + row_height),
        );
        let row_rect = row_slot_rect.shrink2(egui::vec2(unselected_row_shrink_x, 0.0));
        let focus_t = row_focus_t(row_key);
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
            focus_t,
            show_focus_outline && focus_t > 0.001,
            layer_t,
            layout_scale,
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
        .translate(egui::vec2(
            0.0,
            lerp_f32(0.0, -su(10.0), layer_state.top_motion_t),
        ));
        let top_list_inner_rect =
            draw_page_shell(top_content_rect, top_layer_t, language.settings_text());
        let initial_row_offset_y = su(8.0);
        let top_rows_origin_y = top_list_inner_rect.min.y + initial_row_offset_y;
        let top_row_spacing = su(104.0);
        let top_row_height = su(90.0);
        draw_row(
            top_list_inner_rect,
            None,
            top_rows_origin_y,
            top_row_spacing,
            top_row_height,
            0,
            0,
            system_icon,
            None,
            None,
            language.system_text(),
            false,
            None,
            None,
            None,
            true,
            top_layer_t,
        );
        draw_row(
            top_list_inner_rect,
            None,
            top_rows_origin_y,
            top_row_spacing,
            top_row_height,
            1,
            1,
            screen_icon,
            None,
            None,
            language.screen_text(),
            false,
            None,
            None,
            None,
            true,
            top_layer_t,
        );
        draw_row(
            top_list_inner_rect,
            None,
            top_rows_origin_y,
            top_row_spacing,
            top_row_height,
            2,
            2,
            apps_icon,
            None,
            None,
            language.apps_text(),
            false,
            None,
            None,
            None,
            true,
            top_layer_t,
        );
        draw_row(
            top_list_inner_rect,
            None,
            top_rows_origin_y,
            top_row_spacing,
            top_row_height,
            3,
            3,
            exit_icon,
            None,
            None,
            language.close_app_text(),
            false,
            None,
            None,
            None,
            true,
            top_layer_t,
        );
        draw_settings_build_footer(
            painter,
            top_list_inner_rect,
            language,
            top_layer_t,
            layout_scale,
        );
    }

    if submenu_layer_t > 0.001 {
        let submenu_content_rect = egui::Rect::from_center_size(
            page_rect.center(),
            page_rect.size() * lerp_f32(0.982, 1.0, layer_state.submenu_motion_t),
        )
        .translate(egui::vec2(
            0.0,
            lerp_f32(su(10.0), 0.0, layer_state.submenu_motion_t),
        ));
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
        let submenu_row_spacing = su(112.0);
        let submenu_row_height = su(98.0);
        let enabled_subtitle_color = egui::Color32::from_rgba_unmultiplied(122, 214, 145, 255);
        match selected_section_index {
            0 => {
                let system_row_spacing = submenu_row_spacing;
                let section_gap = su(28.0);
                let initial_row_offset_y = su(8.0);
                let final_row_offset_y = su(8.0);
                let body_top_padding = su(14.0);
                let body_bottom_padding = su(12.0);
                let scroll_top_y = lerp_f32(
                    submenu_content_rect.min.y,
                    panel_rect.min.y,
                    layer_state.submenu_motion_t,
                );
                let scroll_viewport_rect = egui::Rect::from_min_max(
                    egui::pos2(submenu_content_rect.min.x, scroll_top_y),
                    egui::pos2(submenu_content_rect.max.x, submenu_content_rect.max.y),
                );
                let scroll_top_inset =
                    (submenu_content_rect.min.y - scroll_viewport_rect.min.y).max(0.0);
                let top_body_height = system_row_spacing * 2.0
                    + submenu_row_height
                    + initial_row_offset_y
                    + final_row_offset_y
                    + body_top_padding
                    + body_bottom_padding;
                let lower_body_height = system_row_spacing * 4.0
                    + submenu_row_height
                    + initial_row_offset_y
                    + final_row_offset_y
                    + body_top_padding
                    + body_bottom_padding;
                let content_height = scroll_top_inset
                    + header_height
                    + header_gap
                    + top_body_height
                    + section_gap
                    + lower_body_height;
                let scroll_offset = scroll_offset_to_keep_focus_visible(
                    content_height,
                    scroll_viewport_rect.height(),
                    scroll_top_inset
                        + system_settings_row_top(
                            selected_item_index,
                            header_height,
                            header_gap,
                            top_body_height,
                            section_gap,
                            body_top_padding,
                            initial_row_offset_y,
                            system_row_spacing,
                        ),
                    submenu_row_height,
                    su(20.0),
                    su(24.0),
                );
                let scroll_offset = animate_settings_scroll_offset(
                    ui,
                    egui::Id::new("settings_system_scroll_offset"),
                    scroll_offset,
                    layer_state.submenu_motion_t >= 0.999,
                );
                let scroll_painter = painter.with_clip_rect(scroll_viewport_rect);
                let header_rect = egui::Rect::from_min_max(
                    egui::pos2(submenu_content_rect.min.x, submenu_content_rect.min.y),
                    egui::pos2(
                        submenu_content_rect.max.x,
                        submenu_content_rect.min.y + header_height,
                    ),
                )
                .translate(egui::vec2(0.0, -scroll_offset));
                draw_settings_page_header(
                    &scroll_painter,
                    header_rect,
                    submenu_layer_t,
                    &page_title,
                    layout_scale,
                );
                let top_body_rect = egui::Rect::from_min_max(
                    egui::pos2(
                        submenu_content_rect.min.x,
                        submenu_content_rect.min.y + header_height + header_gap,
                    ),
                    egui::pos2(
                        submenu_content_rect.max.x,
                        submenu_content_rect.min.y + header_height + header_gap + top_body_height,
                    ),
                )
                .translate(egui::vec2(0.0, -scroll_offset));
                let lower_body_rect = egui::Rect::from_min_max(
                    egui::pos2(
                        submenu_content_rect.min.x,
                        submenu_content_rect.min.y
                            + header_height
                            + header_gap
                            + top_body_height
                            + section_gap,
                    ),
                    egui::pos2(
                        submenu_content_rect.max.x,
                        submenu_content_rect.min.y
                            + header_height
                            + header_gap
                            + top_body_height
                            + section_gap
                            + lower_body_height,
                    ),
                )
                .translate(egui::vec2(0.0, -scroll_offset));
                draw_settings_page_body_container(
                    &scroll_painter,
                    top_body_rect,
                    submenu_layer_t,
                    layout_scale,
                );
                draw_settings_page_body_container(
                    &scroll_painter,
                    lower_body_rect,
                    submenu_layer_t,
                    layout_scale,
                );
                let top_list_inner_rect = egui::Rect::from_min_max(
                    egui::pos2(
                        top_body_rect.min.x + su(6.0),
                        top_body_rect.min.y + body_top_padding,
                    ),
                    egui::pos2(
                        top_body_rect.max.x - su(6.0),
                        top_body_rect.max.y - body_bottom_padding,
                    ),
                );
                let lower_list_inner_rect = egui::Rect::from_min_max(
                    egui::pos2(
                        lower_body_rect.min.x + su(6.0),
                        lower_body_rect.min.y + body_top_padding,
                    ),
                    egui::pos2(
                        lower_body_rect.max.x - su(6.0),
                        lower_body_rect.max.y - body_bottom_padding,
                    ),
                );
                let submenu_list_painter = painter.with_clip_rect(intersect_rects(
                    top_list_inner_rect.union(lower_list_inner_rect),
                    scroll_viewport_rect,
                ));
                let system_rows_origin_y = top_list_inner_rect.min.y + initial_row_offset_y;
                let lower_rows_origin_y = lower_list_inner_rect.min.y + initial_row_offset_y;
                let detection_title_align_width =
                    [GameSource::Steam, GameSource::Epic, GameSource::Xbox]
                        .into_iter()
                        .map(|source| {
                            measure_selected_game_text_badge(
                                &submenu_list_painter,
                                source.badge_label(),
                                egui::vec2(0.0, su(26.0)),
                            )
                            .x
                        })
                        .fold(0.0, f32::max);
                draw_row(
                    top_list_inner_rect,
                    Some(scroll_viewport_rect),
                    system_rows_origin_y,
                    system_row_spacing,
                    submenu_row_height,
                    0,
                    10,
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
                    true,
                    submenu_layer_t,
                );

                draw_row(
                    top_list_inner_rect,
                    Some(scroll_viewport_rect),
                    system_rows_origin_y,
                    system_row_spacing,
                    submenu_row_height,
                    1,
                    11,
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
                    true,
                    submenu_layer_t,
                );

                draw_row(
                    top_list_inner_rect,
                    Some(scroll_viewport_rect),
                    system_rows_origin_y,
                    system_row_spacing,
                    submenu_row_height,
                    2,
                    12,
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
                    true,
                    submenu_layer_t,
                );

                draw_row(
                    lower_list_inner_rect,
                    Some(scroll_viewport_rect),
                    lower_rows_origin_y,
                    system_row_spacing,
                    submenu_row_height,
                    0,
                    13,
                    None,
                    None,
                    None,
                    "",
                    true,
                    Some(background_home_wake_mode.display_text(language)),
                    if background_home_wake_mode != BackgroundHomeWakeMode::Off {
                        Some(enabled_subtitle_color)
                    } else {
                        None
                    },
                    None,
                    true,
                    submenu_layer_t,
                );

                draw_row(
                    lower_list_inner_rect,
                    Some(scroll_viewport_rect),
                    lower_rows_origin_y,
                    system_row_spacing,
                    submenu_row_height,
                    1,
                    14,
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
                    true,
                    submenu_layer_t,
                );

                draw_row(
                    lower_list_inner_rect,
                    Some(scroll_viewport_rect),
                    lower_rows_origin_y,
                    system_row_spacing,
                    submenu_row_height,
                    2,
                    15,
                    None,
                    None,
                    None,
                    language.display_mode_setting_text(),
                    false,
                    Some(selected_display_mode_setting.display_text(language)),
                    None,
                    None,
                    true,
                    submenu_layer_t,
                );

                draw_row(
                    lower_list_inner_rect,
                    Some(scroll_viewport_rect),
                    lower_rows_origin_y,
                    system_row_spacing,
                    submenu_row_height,
                    3,
                    16,
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
                    true,
                    submenu_layer_t,
                );

                draw_row(
                    lower_list_inner_rect,
                    Some(scroll_viewport_rect),
                    lower_rows_origin_y,
                    system_row_spacing,
                    submenu_row_height,
                    4,
                    17,
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
                        submenu_list_inner_rect.min.x + su(18.0),
                        submenu_list_inner_rect.min.y + su(8.0),
                    ),
                    egui::pos2(
                        submenu_list_inner_rect.max.x - su(18.0),
                        submenu_list_inner_rect.min.y + su(96.0),
                    ),
                );
                let rows_origin_y = submenu_list_inner_rect.min.y
                    + draw_settings_section_header(
                        &submenu_list_painter,
                        header_rect,
                        section_name,
                        summary_text.as_deref(),
                        submenu_layer_t,
                        layout_scale,
                    )
                    + su(16.0);
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
                    .unwrap_or_else(|| {
                        vec![format!("{}Hz", resolution_options.current.refresh_hz)]
                    });
                let resolution_value = resolution_values
                    .get(current_resolution_index)
                    .map(String::as_str)
                    .unwrap_or(resolution_options.current.label.as_str());
                let current_refresh_value = format!("{}Hz", resolution_options.current.refresh_hz);
                let refresh_value = refresh_values
                    .get(current_refresh_index)
                    .map(String::as_str)
                    .unwrap_or(current_refresh_value.as_str());
                let scale_values: Vec<String> = display_scale_options
                    .scales
                    .iter()
                    .map(|choice| choice.label.clone())
                    .collect();
                let scale_value = scale_values
                    .get(current_scale_index)
                    .map(String::as_str)
                    .unwrap_or(display_scale_options.current.label.as_str());
                let content_left = submenu_list_inner_rect.min.x + su(18.0);
                let content_right = submenu_list_inner_rect.max.x - su(18.0);
                let row_width = content_right - content_left;
                let row_height = su(98.0);
                let row_gap = su(18.0);
                let menu_gap = su(14.0);
                let option_height = su(64.0);
                let option_gap = su(8.0);
                let menu_padding_y = su(14.0);
                let max_visible_items = 4;
                let screen_dropdown_open = screen_resolution_dropdown_open
                    || screen_refresh_dropdown_open
                    || screen_scale_dropdown_open;
                let mut active_dropdown_mask_rect = None;
                let mut active_dropdown_menu: Option<(egui::Rect, &[String], u16, usize)> = None;
                let mut active_dropdown_trigger: Option<(egui::Rect, &str, &str, u16)> = None;
                let resolution_row_rect = egui::Rect::from_min_size(
                    egui::pos2(content_left, rows_origin_y),
                    egui::vec2(row_width, row_height),
                );
                let resolution_focus_t = row_focus_t(20);
                if !screen_resolution_dropdown_open {
                    let _ = draw_settings_dropdown_row(
                        &submenu_list_painter,
                        resolution_row_rect,
                        language.resolution_text(),
                        resolution_value,
                        current_settings_focus_key == Some(20),
                        screen_resolution_dropdown_open,
                        submenu_layer_t,
                        resolution_focus_t,
                        layout_scale,
                    );
                }
                let refresh_row_top = rows_origin_y + row_height + row_gap;
                let refresh_row_rect = egui::Rect::from_min_size(
                    egui::pos2(content_left, refresh_row_top),
                    egui::vec2(row_width, row_height),
                );
                let refresh_focus_t = row_focus_t(21);
                if !screen_refresh_dropdown_open {
                    let _ = draw_settings_dropdown_row(
                        &submenu_list_painter,
                        refresh_row_rect,
                        language.refresh_rate_text(),
                        refresh_value,
                        current_settings_focus_key == Some(21),
                        screen_refresh_dropdown_open,
                        submenu_layer_t,
                        refresh_focus_t,
                        layout_scale,
                    );
                }
                let scale_row_top = refresh_row_top + row_height + row_gap;
                let scale_row_rect = egui::Rect::from_min_size(
                    egui::pos2(content_left, scale_row_top),
                    egui::vec2(row_width, row_height),
                );
                let scale_focus_t = row_focus_t(22);
                if !screen_scale_dropdown_open {
                    let _ = draw_settings_dropdown_row(
                        &submenu_list_painter,
                        scale_row_rect,
                        language.scale_text(),
                        scale_value,
                        current_settings_focus_key == Some(22),
                        screen_scale_dropdown_open,
                        submenu_layer_t,
                        scale_focus_t,
                        layout_scale,
                    );
                }

                let dropdown_painter = painter.with_clip_rect(submenu_content_rect);

                if screen_resolution_dropdown_open {
                    let resolution_button_rect = draw_settings_dropdown_row(
                        &dropdown_painter,
                        resolution_row_rect,
                        language.resolution_text(),
                        resolution_value,
                        current_settings_focus_key == Some(20),
                        screen_resolution_dropdown_open,
                        submenu_layer_t,
                        current_focus_t,
                        layout_scale,
                    );
                    let visible_count = resolution_values.len().min(max_visible_items);
                    let height = menu_padding_y * 2.0
                        + visible_count as f32 * option_height
                        + visible_count.saturating_sub(1) as f32 * option_gap;
                    let menu_rect = settings_dropdown_menu_rect(
                        resolution_button_rect,
                        height,
                        menu_gap,
                        submenu_content_rect,
                        layout_scale,
                    );
                    active_dropdown_mask_rect = Some(resolution_row_rect);
                    active_dropdown_trigger = Some((
                        resolution_row_rect,
                        language.resolution_text(),
                        resolution_value,
                        20,
                    ));
                    active_dropdown_menu = Some((
                        menu_rect,
                        resolution_values.as_slice(),
                        100,
                        screen_dropdown_selected_index
                            .min(resolution_values.len().saturating_sub(1)),
                    ));
                }

                if screen_refresh_dropdown_open {
                    let refresh_button_rect = draw_settings_dropdown_row(
                        &dropdown_painter,
                        refresh_row_rect,
                        language.refresh_rate_text(),
                        refresh_value,
                        current_settings_focus_key == Some(21),
                        screen_refresh_dropdown_open,
                        submenu_layer_t,
                        current_focus_t,
                        layout_scale,
                    );
                    let visible_count = refresh_values.len().min(max_visible_items);
                    let height = menu_padding_y * 2.0
                        + visible_count as f32 * option_height
                        + visible_count.saturating_sub(1) as f32 * option_gap;
                    let menu_rect = settings_dropdown_menu_rect(
                        refresh_button_rect,
                        height,
                        menu_gap,
                        submenu_content_rect,
                        layout_scale,
                    );
                    active_dropdown_mask_rect = Some(refresh_row_rect);
                    active_dropdown_trigger = Some((
                        refresh_row_rect,
                        language.refresh_rate_text(),
                        refresh_value,
                        21,
                    ));
                    active_dropdown_menu = Some((
                        menu_rect,
                        refresh_values.as_slice(),
                        300,
                        screen_dropdown_selected_index.min(refresh_values.len().saturating_sub(1)),
                    ));
                }

                if screen_scale_dropdown_open {
                    let scale_button_rect = draw_settings_dropdown_row(
                        &dropdown_painter,
                        scale_row_rect,
                        language.scale_text(),
                        scale_value,
                        current_settings_focus_key == Some(22),
                        screen_scale_dropdown_open,
                        submenu_layer_t,
                        current_focus_t,
                        layout_scale,
                    );
                    let visible_count = scale_values.len().min(max_visible_items);
                    let height = menu_padding_y * 2.0
                        + visible_count as f32 * option_height
                        + visible_count.saturating_sub(1) as f32 * option_gap;
                    let menu_rect = settings_dropdown_menu_rect(
                        scale_button_rect,
                        height,
                        menu_gap,
                        submenu_content_rect,
                        layout_scale,
                    );
                    active_dropdown_mask_rect = Some(scale_row_rect);
                    active_dropdown_trigger =
                        Some((scale_row_rect, language.scale_text(), scale_value, 22));
                    active_dropdown_menu = Some((
                        menu_rect,
                        scale_values.as_slice(),
                        500,
                        screen_dropdown_selected_index.min(scale_values.len().saturating_sub(1)),
                    ));
                }

                let (dropdown_mask_alpha, _dropdown_mask_rect) = animate_settings_dropdown_mask(
                    ui,
                    egui::Id::new("settings_screen_dropdown_mask"),
                    screen_dropdown_open,
                    active_dropdown_mask_rect,
                    panel_rect,
                );
                if dropdown_mask_alpha > SETTINGS_DROPDOWN_MASK_EPSILON {
                    draw_settings_dropdown_mask(
                        painter,
                        panel_rect,
                        None,
                        dropdown_mask_alpha,
                        submenu_layer_t,
                        layout_scale,
                    );
                }
                if let Some((row_rect, label, value, focus_key)) = active_dropdown_trigger {
                    let _ = draw_settings_dropdown_row(
                        &dropdown_painter,
                        row_rect,
                        label,
                        value,
                        current_settings_focus_key == Some(focus_key),
                        true,
                        submenu_layer_t,
                        current_focus_t,
                        layout_scale,
                    );
                }
                if let Some((menu_rect, menu_options, base_focus_key, selected_index)) =
                    active_dropdown_menu
                {
                    let _ = draw_settings_dropdown_menu(
                        &dropdown_painter,
                        menu_rect,
                        menu_options,
                        base_focus_key,
                        selected_index,
                        su(24.0),
                        submenu_layer_t,
                        current_focus_t,
                        current_settings_focus_key,
                        layout_scale,
                    );
                }
            }
            _ => {
                let submenu_list_inner_rect =
                    draw_page_shell(submenu_content_rect, submenu_layer_t, &page_title);
                let submenu_list_painter = painter.with_clip_rect(submenu_list_inner_rect);
                let header_rect = egui::Rect::from_min_max(
                    egui::pos2(
                        submenu_list_inner_rect.min.x + su(18.0),
                        submenu_list_inner_rect.min.y + su(8.0),
                    ),
                    egui::pos2(
                        submenu_list_inner_rect.max.x - su(18.0),
                        submenu_list_inner_rect.min.y + su(96.0),
                    ),
                );
                let rows_origin_y = submenu_list_inner_rect.min.y
                    + draw_settings_section_header(
                        &submenu_list_painter,
                        header_rect,
                        section_name,
                        summary_text.as_deref(),
                        submenu_layer_t,
                        layout_scale,
                    )
                    + su(16.0);
                draw_row(
                    submenu_list_inner_rect,
                    None,
                    rows_origin_y,
                    submenu_row_spacing,
                    submenu_row_height,
                    0,
                    700,
                    None,
                    None,
                    None,
                    language.dlss_swapper_text(),
                    false,
                    None,
                    None,
                    None,
                    true,
                    submenu_layer_t,
                );
                draw_row(
                    submenu_list_inner_rect,
                    None,
                    rows_origin_y,
                    submenu_row_spacing,
                    submenu_row_height,
                    1,
                    701,
                    None,
                    None,
                    None,
                    language.nvidia_app_text(),
                    false,
                    None,
                    None,
                    None,
                    true,
                    submenu_layer_t,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui;

    use super::{
        eased_scroll_offset, scroll_offset_to_keep_focus_visible, settings_dropdown_menu_rect,
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

    #[test]
    fn focus_scroll_stays_zero_when_content_fits() {
        let scroll_offset =
            scroll_offset_to_keep_focus_visible(320.0, 420.0, 180.0, 98.0, 20.0, 24.0);

        assert!(scroll_offset.abs() < f32::EPSILON);
    }

    #[test]
    fn focus_scroll_moves_row_back_inside_viewport() {
        let scroll_offset =
            scroll_offset_to_keep_focus_visible(900.0, 500.0, 460.0, 98.0, 20.0, 24.0);

        assert!((scroll_offset - 82.0).abs() < 1e-6);
    }

    #[test]
    fn focus_scroll_clamps_to_available_content_range() {
        let scroll_offset =
            scroll_offset_to_keep_focus_visible(540.0, 500.0, 520.0, 98.0, 20.0, 24.0);

        assert!((scroll_offset - 40.0).abs() < 1e-6);
    }

    #[test]
    fn eased_scroll_offset_uses_stable_dt_progress() {
        let next = eased_scroll_offset(0.0, 100.0, 1.0 / 60.0, 14.0);
        let expected = 100.0 * (1.0 - (-(14.0_f32 / 60.0)).exp());

        assert!((next - expected).abs() < 1e-6);
    }

    #[test]
    fn eased_scroll_offset_handles_non_positive_dt_without_jumping() {
        assert!((eased_scroll_offset(12.0, 80.0, 0.0, 14.0) - 12.0).abs() < f32::EPSILON);
    }

    #[test]
    fn dropdown_menu_opens_below_when_space_allows() {
        let button_rect =
            egui::Rect::from_min_size(egui::pos2(40.0, 120.0), egui::vec2(200.0, 98.0));
        let viewport_rect =
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 600.0));
        let menu_rect = settings_dropdown_menu_rect(button_rect, 220.0, 14.0, viewport_rect, 1.0);

        assert!((menu_rect.min.y - 232.0).abs() < 1e-6);
    }

    #[test]
    fn dropdown_menu_flips_up_when_bottom_would_clip() {
        let button_rect =
            egui::Rect::from_min_size(egui::pos2(40.0, 420.0), egui::vec2(200.0, 98.0));
        let viewport_rect =
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 600.0));
        let menu_rect = settings_dropdown_menu_rect(button_rect, 220.0, 14.0, viewport_rect, 1.0);

        assert!((menu_rect.min.y - 186.0).abs() < 1e-6);
        assert!(menu_rect.max.y <= 592.0);
    }
}
