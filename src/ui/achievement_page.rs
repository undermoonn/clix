use std::collections::HashMap;

use eframe::egui;

use crate::game::Game;
use crate::i18n::AppLanguage;
use crate::steam::AchievementSummary;

use super::hint_icons::HintIcons;
use super::anim::{lerp_f32, smoothstep01};
use super::header::{
    build_selected_game_header, dlss_tag_text, draw_selected_game_summary,
    draw_selected_game_text_badge, draw_selected_game_text_badge_with_style,
    draw_selected_game_title, game_source_badge_text, installed_size_tag_text,
    measure_selected_game_text_badge,
    SelectedGameBadgeStyle, SelectedGameSummaryStyle,
};
use super::text::{
    build_wrapped_galley, color_with_scaled_alpha, corner_radius, format_achievement_status,
    PANEL_CORNER_RADIUS,
};

fn draw_achievement_icon(
    painter: &egui::Painter,
    texture: &egui::TextureHandle,
    icon_rect: egui::Rect,
    tint: egui::Color32,
    reveal: f32,
) {
    const ACHIEVEMENT_ICON_ROUNDING: f32 = 4.0;
    let [tex_w, tex_h] = texture.size();
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

    let draw_rect = if tex_w > 0 && tex_h > 0 {
        let tex_w = tex_w as f32;
        let tex_h = tex_h as f32;
        let aspect = tex_w / tex_h;
        let icon_size = icon_rect.width().min(icon_rect.height());

        let (scaled_w, scaled_h) = if aspect > 1.0 {
            (icon_size, icon_size / aspect)
        } else {
            (icon_size * aspect, icon_size)
        };

        let center = icon_rect.center();
        egui::Rect::from_center_size(center, egui::vec2(scaled_w, scaled_h))
    } else {
        icon_rect
    };

    let reveal = reveal.clamp(0.0, 1.0);
    let alpha = ((tint.a() as f32) * reveal).round() as u8;
    let fade_tint = egui::Color32::from_rgba_unmultiplied(tint.r(), tint.g(), tint.b(), alpha);
    painter.add(egui::Shape::Rect(
        egui::epaint::RectShape::filled(draw_rect, corner_radius(ACHIEVEMENT_ICON_ROUNDING), fade_tint)
            .with_texture(texture.id(), uv),
    ));
}

fn draw_centered_achievement_loading(ui: &egui::Ui, rect: egui::Rect) {
    let painter = ui.painter().clone();
    let time = ui.input(|input| input.time) as f32;
    let center = rect.center();
    let spacing = 24.0;
    let radius = 5.5;
    let jump = 10.0;
    let color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10);

    for index in 0..3 {
        let phase = time * 5.4 - index as f32 * 0.32;
        let bounce = phase.sin().max(0.0);
        let x = center.x + (index as f32 - 1.0) * spacing;
        let y = center.y - bounce * jump;
        painter.circle_filled(egui::pos2(x, y), radius, color);
    }

    ui.ctx().request_repaint();
}

fn draw_centered_achievement_empty(
    painter: &egui::Painter,
    rect: egui::Rect,
    language: AppLanguage,
) {
    let empty_galley = painter.layout_no_wrap(
        language.achievement_empty_text().to_string(),
        egui::FontId::proportional(18.0),
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10),
    );
    painter.galley(
        egui::pos2(
            rect.center().x - empty_galley.size().x * 0.5,
            rect.center().y - empty_galley.size().y * 0.5,
        ),
        empty_galley,
        egui::Color32::WHITE,
    );
}

fn format_achievement_percent(global_percent: Option<f32>) -> Option<f32> {
    global_percent
        .filter(|value| value.is_finite())
}

fn achievement_content_split_x(content_rect: egui::Rect) -> f32 {
    lerp_f32(content_rect.min.x, content_rect.max.x, 0.618)
}

fn achievement_row_background_color(
    unlocked: Option<bool>,
    is_selected: bool,
) -> egui::Color32 {
    if unlocked == Some(true) {
        if is_selected {
            egui::Color32::from_rgba_unmultiplied(148, 212, 176, 132)
        } else {
            egui::Color32::from_rgba_unmultiplied(132, 194, 160, 112)
        }
    } else if is_selected {
        egui::Color32::from_rgba_unmultiplied(224, 230, 238, 108)
    } else {
        egui::Color32::from_rgba_unmultiplied(206, 214, 224, 88)
    }
}

fn masked_achievement_text(source: &str) -> String {
    let glyph_count = source
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .count()
        .clamp(14, 42);
    let mut masked = String::new();
    for index in 0..glyph_count {
        if index > 0 && index % 6 == 0 {
            masked.push(' ');
        }
        masked.push('•');
    }
    masked
}

fn draw_hidden_achievement_overlay(
    painter: &egui::Painter,
    row_rect: egui::Rect,
    language: AppLanguage,
    show_prompt: bool,
    icons: Option<&HintIcons>,
    reveal_progress: f32,
    alpha_scale: f32,
) {
    let overlay_alpha = (1.0 - reveal_progress).clamp(0.0, 1.0) * alpha_scale;
    if overlay_alpha <= 0.001 {
        return;
    }

    let overlay_rect = row_rect;
    let overlay_painter = painter.with_clip_rect(overlay_rect);
    overlay_painter.rect_filled(
        overlay_rect,
        corner_radius(6.0),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(22, 24, 28, 176),
            overlay_alpha,
        ),
    );

    if !show_prompt {
        return;
    }

    let title = painter.layout_no_wrap(
        language.achievement_hidden_text().to_string(),
        egui::FontId::new(24.0, egui::FontFamily::Name("Bold".into())),
        color_with_scaled_alpha(egui::Color32::from_rgb(244, 246, 248), overlay_alpha),
    );
    let title_size = title.size();
    let icon_size = 39.0;
    let icon_gap = 12.0;
    let group_width = title_size.x + if icons.is_some() { icon_gap + icon_size } else { 0.0 };
    let title_pos = egui::pos2(
        overlay_rect.center().x - group_width * 0.5,
        overlay_rect.center().y - title_size.y * 0.5,
    );
    overlay_painter.galley(title_pos, title, egui::Color32::WHITE);

    if let Some(icons) = icons {
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(
                title_pos.x + title_size.x + icon_gap,
                overlay_rect.center().y - icon_size * 0.5,
            ),
            egui::vec2(icon_size, icon_size),
        );
        overlay_painter.image(
            icons.btn_a.id(),
            icon_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            color_with_scaled_alpha(egui::Color32::WHITE, overlay_alpha),
        );
    }
}

fn draw_achievement_row_focus_frame(
    painter: &egui::Painter,
    row_rect: egui::Rect,
    focus_t: f32,
    wake_t: f32,
) {
    if focus_t <= 0.001 {
        return;
    }

    let focus_rect = row_rect.expand(5.0);
    painter.rect_filled(
        focus_rect,
        corner_radius(12.0),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(248, 250, 255, 54),
            wake_t * focus_t,
        ),
    );
    painter.rect_stroke(
        focus_rect,
        corner_radius(12.0),
        egui::Stroke::new(
            lerp_f32(1.2, 3.0, focus_t),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 168),
                wake_t * focus_t,
            ),
        ),
        egui::StrokeKind::Outside,
    );
}

pub fn draw_achievement_page(
    ui: &mut egui::Ui,
    language: AppLanguage,
    game: &Game,
    summary: Option<&AchievementSummary>,
    is_loading: bool,
    has_no_data: bool,
    achievement_summary_reveal_for_selected: f32,
    selected_index: usize,
    achievement_select_anim: f32,
    achievement_panel_anim: f32,
    _selected_game_index: usize,
    game_select_anim: f32,
    _game_scroll_offset: f32,
    scroll_offset: f32,
    game_icon: Option<&egui::TextureHandle>,
    hint_icons: Option<&HintIcons>,
    revealed_hidden: Option<&str>,
    hidden_reveal_progress: f32,
    achievement_icon_cache: &HashMap<String, egui::TextureHandle>,
    achievement_icon_reveal: &HashMap<String, f32>,
    achievement_percent_reveal: &HashMap<String, f32>,
) -> Vec<String> {
    let mut visible_icon_urls = Vec::new();
    let panel_rect = ui.available_rect_before_wrap();
    let padding = 50.0;
    let padded_rect = panel_rect.shrink(padding);
    let painter = ui.painter().with_clip_rect(panel_rect);
    let panel_t = smoothstep01(achievement_panel_anim);
    let wake_t = 1.0_f32;
    let page_enter_offset_y = lerp_f32(panel_rect.height() + 28.0, 0.0, panel_t);
    let content_top = padded_rect.min.y + 18.0;
    let title_font_size = 18.0 + (30.0 - 18.0) * smoothstep01(game_select_anim);
    let title_font = egui::FontId::proportional(title_font_size);
    let header = build_selected_game_header(
        ui,
        &painter,
        language,
        game,
        None,
        0.0,
        None,
        0.0,
        title_font,
        egui::Color32::WHITE,
        17.0,
        0.0,
        (padded_rect.width() - 96.0).max(220.0),
    );
    let header_left = padded_rect.min.x + 24.0;
    let title_base_y = content_top + 16.0;
    let title_base_pos = egui::pos2(header_left, title_base_y);
    let detail_summary_style = SelectedGameSummaryStyle {
        show_playtime: false,
        show_achievement_title: false,
        hide_empty_achievement_card: false,
        card_height: 82.0,
        layout_scale: 1.0,
    };
    let playtime_width = 0.0;
    let achievement_width = 292.0;
    let summary_total_width = achievement_width;
    let summary_base_pos = egui::pos2(
        padded_rect.max.x - 24.0 - summary_total_width,
        content_top + 4.0,
    );
    let achievement_x = summary_base_pos.x;
    let header_bottom = (title_base_y + header.total_height()).max(
        summary_base_pos.y + detail_summary_style.card_height,
    );
    let header_base_rect = egui::Rect::from_min_max(
        egui::pos2(padded_rect.min.x + 8.0, content_top),
        egui::pos2(padded_rect.max.x - 8.0, header_bottom + 14.0),
    );
    let list_base_rect = egui::Rect::from_min_max(
        egui::pos2(padded_rect.min.x + 8.0, header_base_rect.max.y + 10.0),
        egui::pos2(padded_rect.max.x - 8.0, padded_rect.max.y - 52.0),
    );
    let content_offset = egui::vec2(0.0, page_enter_offset_y);
    let list_rect = list_base_rect.translate(content_offset);
    let header_visual_offset = egui::vec2(0.0, -14.0);
    let summary_pos = summary_base_pos + content_offset + header_visual_offset;
    let source_badge_text = game_source_badge_text(game.source);
    let steam_badge_size =
        measure_selected_game_text_badge(&painter, source_badge_text, header.title_galley.size());
    let header_stack_height = steam_badge_size.y + 14.0 + header.title_galley.size().y;
    let game_icon_size = header_stack_height;
    let game_icon_gap = 18.0;
    let game_icon_total_width = if game_icon.is_some() {
        game_icon_size + game_icon_gap
    } else {
        0.0
    };
    let header_stack_x =
        title_base_pos.x + content_offset.x + header_visual_offset.x + game_icon_total_width;
    let header_text_offset_x = 10.0;
    let badge_pos = egui::pos2(
        header_stack_x + header_text_offset_x,
        summary_pos.y - header.title_galley.size().y * 0.5 + steam_badge_size.y * 0.5,
    );
    if let Some(game_icon) = game_icon {
        let game_icon_rect = egui::Rect::from_min_size(
            egui::pos2(
                header_stack_x - game_icon_total_width,
                summary_pos.y,
            ),
            egui::vec2(game_icon_size, game_icon_size),
        );
        draw_achievement_icon(
            &painter,
            game_icon,
            game_icon_rect,
            color_with_scaled_alpha(egui::Color32::WHITE, wake_t),
            1.0,
        );
    }
    let mut badge_row_offset = 0.0;
    let steam_badge_size = draw_selected_game_text_badge(
        &painter,
        source_badge_text,
        badge_pos,
        header.title_galley.size(),
        wake_t,
    );
    badge_row_offset += steam_badge_size.x;
    if let Some(tag_text) = installed_size_tag_text(language, game) {
        let badge_size = draw_selected_game_text_badge_with_style(
            &painter,
            &tag_text,
            egui::pos2(badge_pos.x + badge_row_offset, badge_pos.y),
            header.title_galley.size(),
            panel_t * wake_t,
            &SelectedGameBadgeStyle::detail_tag(egui::Color32::from_rgb(28, 30, 34)),
        );
        badge_row_offset += badge_size.x;
    }
    if let Some(tag_text) = dlss_tag_text(game) {
        let _ = draw_selected_game_text_badge_with_style(
            &painter,
            &tag_text,
            egui::pos2(badge_pos.x + badge_row_offset, badge_pos.y),
            header.title_galley.size(),
            panel_t * wake_t,
            &SelectedGameBadgeStyle::detail_tag(egui::Color32::from_rgb(34, 36, 40)),
        );
    }
    let title_pos = egui::pos2(badge_pos.x, badge_pos.y + steam_badge_size.y + 19.0);
    draw_selected_game_title(&painter, &header, &game.name, title_pos, wake_t);
    draw_selected_game_summary(
        &painter,
        language,
        game,
        summary,
        achievement_summary_reveal_for_selected,
        summary_pos,
        playtime_width,
        achievement_x,
        achievement_width,
        &detail_summary_style,
        1.0,
        wake_t,
    );

    painter.rect_filled(
        list_rect,
        corner_radius(PANEL_CORNER_RADIUS),
        color_with_scaled_alpha(egui::Color32::from_rgb(14, 14, 14), wake_t),
    );

    let list_inner_rect = egui::Rect::from_min_max(
        egui::pos2(list_rect.min.x + 6.0, list_rect.min.y + 12.0),
        egui::pos2(list_rect.max.x - 6.0, list_rect.max.y - 16.0),
    );
    let row_side_inset = 6.0;
    let unselected_row_shrink_x = 7.0;

    let Some(summary) = summary else {
        if is_loading && !has_no_data {
            draw_centered_achievement_loading(ui, list_rect);
        } else {
            draw_centered_achievement_empty(&painter, list_rect, language);
        }
        return visible_icon_urls;
    };

    if summary.items.is_empty() {
        if is_loading && !has_no_data {
            draw_centered_achievement_loading(ui, list_rect);
        } else {
            draw_centered_achievement_empty(&painter, list_rect, language);
        }
        return visible_icon_urls;
    }

    let item_gap_y = 16.0;
    let row_spacing = 174.0;
    let list_body_rect = list_inner_rect;
    let list_painter = painter.with_clip_rect(list_body_rect);
    let visible_rows = (list_body_rect.height() / row_spacing).ceil() as i32 + 2;
    let initial_row_offset_y = 8.0;
    let base_y = list_body_rect.min.y + initial_row_offset_y - scroll_offset * row_spacing;

    for (idx, item) in summary.items.iter().enumerate() {
        let row_offset = idx as f32 - scroll_offset;
        if row_offset < -1.5 || row_offset > visible_rows as f32 {
            continue;
        }

        let is_selected = idx == selected_index;
        let selection_t = if is_selected {
            smoothstep01(achievement_select_anim)
        } else {
            0.0
        };
        let row_top = base_y + idx as f32 * row_spacing;
        if row_top > list_body_rect.max.y || row_top + row_spacing < list_body_rect.min.y {
            continue;
        }

        let row_height = row_spacing - item_gap_y;
        let row_slot_rect = egui::Rect::from_min_max(
            egui::pos2(list_body_rect.min.x + row_side_inset, row_top),
            egui::pos2(list_body_rect.max.x - row_side_inset, row_top + row_height),
        );
        let row_rect = row_slot_rect.shrink2(egui::vec2(unselected_row_shrink_x, 0.0));
        let content_padding_y = 18.0;
        let content_padding_x = 33.0;
        let icon_gap = 21.0;
        let right_padding = content_padding_x;
        let hidden_state = item.is_hidden && item.unlocked != Some(true);
        let hidden_revealing = hidden_state
            && revealed_hidden.is_some_and(|revealed_api_name| revealed_api_name == item.api_name);
        let hidden_masked = hidden_state && !hidden_revealing;
        draw_achievement_row_focus_frame(&list_painter, row_rect, selection_t, wake_t);
        list_painter.rect_filled(
            row_rect,
            corner_radius(9.0),
            color_with_scaled_alpha(
                achievement_row_background_color(item.unlocked, is_selected),
                wake_t,
            ),
        );

        let icon_column_width = 78.0;
        let left_content_inset = content_padding_x;
        let text_x = row_rect.min.x + left_content_inset + icon_column_width + icon_gap;
        let content_rect = egui::Rect::from_min_max(
            egui::pos2(text_x, row_rect.min.y + content_padding_y),
            egui::pos2(row_rect.max.x - right_padding, row_rect.max.y - content_padding_y),
        );
        let split_x = achievement_content_split_x(content_rect);
        let split_gap = 24.0;
        let left_column_rect = egui::Rect::from_min_max(
            content_rect.min,
            egui::pos2(split_x - split_gap, content_rect.max.y),
        );
        let right_column_rect = egui::Rect::from_min_max(
            egui::pos2(split_x + split_gap, content_rect.min.y),
            content_rect.max,
        );
        let unlock_time_text = format_achievement_status(item.unlocked, item.unlock_time);
        let unlock_time_galley = unlock_time_text.as_ref().map(|text| {
            painter.layout_no_wrap(
                text.clone(),
                egui::FontId::proportional(21.0),
                color_with_scaled_alpha(
                    egui::Color32::from_rgba_unmultiplied(195, 199, 207, 255),
                    wake_t,
                ),
            )
        });
        let name = item
            .display_name
            .as_deref()
            .filter(|text| !text.trim().is_empty())
            .unwrap_or(&item.api_name);
        let unlock_time_width = unlock_time_galley
            .as_ref()
            .map(|galley| galley.size().x)
            .unwrap_or(0.0);
        let title_gap = if unlock_time_galley.is_some() { 18.0 } else { 0.0 };
        let title_width = (left_column_rect.width() - unlock_time_width - title_gap).max(180.0);
        let title_galley = build_wrapped_galley(
            ui,
            name.to_string(),
            if is_selected {
                egui::FontId::new(28.5, egui::FontFamily::Name("Bold".into()))
            } else {
                egui::FontId::proportional(28.5)
            },
            color_with_scaled_alpha(
                if item.unlocked == Some(true) {
                    egui::Color32::from_rgba_unmultiplied(
                        230,
                        239,
                        232,
                        if is_selected { 255 } else { 235 },
                    )
                } else {
                    egui::Color32::from_rgba_unmultiplied(
                        222,
                        224,
                        228,
                        if is_selected { 255 } else { 228 },
                    )
                },
                wake_t,
            ),
            title_width,
        );
        let unlock_rate_font = egui::FontId::proportional(19.5);
        let unlock_rate_galley = format_achievement_percent(item.global_percent).map(|percent| {
            let reveal = achievement_percent_reveal
                .get(&item.api_name)
                .copied()
                .unwrap_or(1.0);
            painter.layout_no_wrap(
                language.format_achievement_unlock_rate(percent),
                unlock_rate_font.clone(),
                color_with_scaled_alpha(
                    egui::Color32::from_rgba_unmultiplied(186, 192, 200, 228),
                    wake_t * reveal,
                ),
            )
        });
        let unlock_rate_reserved_height = painter
            .layout_no_wrap(
                " ".to_string(),
                unlock_rate_font,
                egui::Color32::TRANSPARENT,
            )
            .size()
            .y;
        let base_description_text = item
            .description
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .unwrap_or(language.no_description_text());
        let description_text = if hidden_masked {
            masked_achievement_text(base_description_text)
        } else {
            base_description_text.to_string()
        };
        let description_galley = build_wrapped_galley(
            ui,
            description_text,
            egui::FontId::proportional(22.5),
            color_with_scaled_alpha(
                if hidden_masked {
                    egui::Color32::from_rgba_unmultiplied(180, 184, 192, 220)
                } else {
                    egui::Color32::from_rgba_unmultiplied(195, 199, 207, 255)
                },
                wake_t,
            ),
            right_column_rect.width().max(180.0),
        );
        let first_row_height = title_galley
            .size()
            .y
            .max(unlock_time_galley.as_ref().map(|galley| galley.size().y).unwrap_or(0.0));
        let unlock_rate_gap = 10.0;
        let left_block_height = first_row_height
            + unlock_rate_gap
            + unlock_rate_galley
                .as_ref()
                .map(|galley| galley.size().y)
                .unwrap_or(unlock_rate_reserved_height);
        let content_block_height = left_block_height.max(description_galley.size().y);
        let icon_size = content_block_height
            .min(row_rect.height() - content_padding_y * 2.0)
            .clamp(66.0, icon_column_width);
        let content_top = row_rect.min.y + (row_rect.height() - content_block_height.max(icon_size)) * 0.5;
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(
                row_rect.min.x + left_content_inset + (icon_column_width - icon_size) * 0.5,
                content_top,
            ),
            egui::vec2(icon_size, icon_size),
        );
        list_painter.line_segment(
            [
                egui::pos2(split_x, icon_rect.min.y),
                egui::pos2(split_x, icon_rect.max.y),
            ],
            egui::Stroke::new(
                2.0,
                color_with_scaled_alpha(
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 89),
                    wake_t,
                ),
            ),
        );

        let icon_key = match item.unlocked {
            Some(true) => item.icon_url.as_ref().or(item.icon_gray_url.as_ref()),
            _ => item.icon_gray_url.as_ref().or(item.icon_url.as_ref()),
        };
        if let Some(key) = icon_key {
            visible_icon_urls.push(key.clone());
        }
        if let Some(tex) = icon_key.and_then(|key| achievement_icon_cache.get(key)) {
            let reveal = icon_key
                .and_then(|key| achievement_icon_reveal.get(key).copied())
                .unwrap_or(1.0);
            draw_achievement_icon(
                &list_painter,
                tex,
                icon_rect,
                color_with_scaled_alpha(
                    if item.unlocked == Some(true) {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgba_unmultiplied(216, 220, 228, 220)
                    },
                    wake_t,
                ),
                reveal,
            );
        } else {
            let fill = if hidden_state {
                egui::Color32::from_rgb(120, 126, 134)
            } else {
                match item.unlocked {
                    Some(true) => egui::Color32::from_rgb(86, 172, 132),
                    Some(false) => egui::Color32::from_rgb(108, 112, 122),
                    None => egui::Color32::from_rgb(82, 88, 102),
                }
            };
            list_painter.rect_filled(
                icon_rect,
                corner_radius(6.0),
                color_with_scaled_alpha(fill, wake_t),
            );
        }

        let text_top = content_top;
        list_painter.galley(
            egui::pos2(left_column_rect.min.x, text_top),
            title_galley.clone(),
            egui::Color32::WHITE,
        );
        if let Some(unlock_time_galley) = unlock_time_galley.as_ref() {
            list_painter.galley(
                egui::pos2(
                    left_column_rect.max.x - unlock_time_galley.size().x,
                    text_top,
                ),
                unlock_time_galley.clone(),
                egui::Color32::WHITE,
            );
        }
        if let Some(unlock_rate_galley) = unlock_rate_galley.as_ref() {
            list_painter.galley(
                egui::pos2(left_column_rect.min.x, text_top + first_row_height + unlock_rate_gap),
                unlock_rate_galley.clone(),
                egui::Color32::WHITE,
            );
        }
        let description_pos = egui::pos2(right_column_rect.min.x, text_top);
        list_painter.galley(description_pos, description_galley.clone(), egui::Color32::WHITE);

        if hidden_state {
            let reveal_progress = if hidden_revealing {
                hidden_reveal_progress
            } else {
                0.0
            };
            draw_hidden_achievement_overlay(
                &list_painter,
                row_rect,
                language,
                is_selected,
                if is_selected { hint_icons } else { None },
                reveal_progress,
                wake_t,
            );
        }
    }

    visible_icon_urls
}
