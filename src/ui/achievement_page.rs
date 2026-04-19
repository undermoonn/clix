use std::collections::HashMap;

use eframe::egui;

use crate::i18n::AppLanguage;
use crate::steam::{AchievementSummary, Game};

use super::assets::HintIcons;
use super::anim::{lerp_f32, smoothstep01};
use super::header::{
    build_selected_game_header, dlss_tag_text, draw_selected_game_header, draw_title_tag,
};
use super::text::{
    build_wrapped_galley, color_with_scaled_alpha, format_achievement_status,
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
    painter.add(egui::Shape::Rect(egui::epaint::RectShape {
        rect: draw_rect,
        rounding: egui::Rounding::same(ACHIEVEMENT_ICON_ROUNDING),
        fill: fade_tint,
        stroke: egui::Stroke::NONE,
        fill_texture_id: texture.id(),
        uv,
    }));
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
    );
}

fn format_achievement_percent(global_percent: Option<f32>) -> String {
    match global_percent.filter(|value| value.is_finite()) {
        Some(value) => format!("{:.1}%", value),
        None => "--.-%".to_string(),
    }
}

fn achievement_percent_fill_t(global_percent: Option<f32>) -> f32 {
    global_percent
        .filter(|value| value.is_finite())
        .map(|value| (value / 100.0).clamp(0.0, 1.0))
        .unwrap_or(0.0)
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

fn draw_badge(
    painter: &egui::Painter,
    text: &str,
    top_left: egui::Pos2,
    fill: egui::Color32,
    text_color: egui::Color32,
    alpha_scale: f32,
) -> egui::Vec2 {
    let alpha_fill = color_with_scaled_alpha(fill, alpha_scale);
    let alpha_text = color_with_scaled_alpha(text_color, alpha_scale);
    let font = egui::FontId::new(12.5, egui::FontFamily::Name("Bold".into()));
    let galley = painter.layout_no_wrap(text.to_string(), font, alpha_text);
    let size = egui::vec2(galley.size().x + 18.0, galley.size().y + 9.0);
    let rect = egui::Rect::from_min_size(top_left, size);
    painter.rect_filled(
        rect,
        egui::Rounding::same((rect.height() * 0.5).min(9.0)),
        alpha_fill,
    );
    painter.galley(
        egui::pos2(rect.min.x + 9.0, rect.min.y + (rect.height() - galley.size().y) * 0.5),
        galley,
    );
    size
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
        egui::Rounding::same(6.0),
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(20, 22, 26, 232),
            overlay_alpha,
        ),
    );

    if !show_prompt {
        return;
    }

    let title = painter.layout_no_wrap(
        language.achievement_hidden_text().to_string(),
        egui::FontId::new(16.0, egui::FontFamily::Name("Bold".into())),
        color_with_scaled_alpha(egui::Color32::from_rgb(236, 239, 242), overlay_alpha),
    );
    let title_size = title.size();
    let icon_size = 26.0;
    let icon_gap = 8.0;
    let group_width = title_size.x + if icons.is_some() { icon_gap + icon_size } else { 0.0 };
    let title_pos = egui::pos2(
        overlay_rect.center().x - group_width * 0.5,
        overlay_rect.center().y - title_size.y * 0.5,
    );
    overlay_painter.galley(title_pos, title);

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
    wake_anim: f32,
    _game_icon: Option<&egui::TextureHandle>,
    hint_icons: Option<&HintIcons>,
    revealed_hidden: Option<&str>,
    hidden_reveal_progress: f32,
    sort_high_to_low: bool,
    achievement_icon_cache: &HashMap<String, egui::TextureHandle>,
    achievement_icon_reveal: &HashMap<String, f32>,
) -> Vec<String> {
    let mut visible_icon_urls = Vec::new();
    let panel_rect = ui.available_rect_before_wrap();
    let padding = 50.0;
    let padded_rect = panel_rect.shrink(padding);
    let painter = ui.painter().with_clip_rect(panel_rect);
    let panel_t = smoothstep01(achievement_panel_anim);
    let wake_t = smoothstep01(wake_anim);
    let page_enter_offset_y = lerp_f32(panel_rect.height() + 28.0, 0.0, panel_t)
        + lerp_f32(30.0, 0.0, wake_t);
    let content_top = padded_rect.min.y + 18.0;
    let title_font_size = 20.0 + (34.0 - 20.0) * smoothstep01(game_select_anim);
    let title_font = egui::FontId::new(title_font_size, egui::FontFamily::Name("Bold".into()));
    let header = build_selected_game_header(
        ui,
        &painter,
        language,
        game,
        summary,
        achievement_summary_reveal_for_selected,
        None,
        0.0,
        title_font,
        egui::Color32::WHITE,
        17.0,
        140.0,
        (padded_rect.width() - 96.0).max(220.0),
    );
    let header_text_x = padded_rect.min.x + 24.0;
    let text_block_height = header.total_height();
    let text_top = content_top + 64.0 - text_block_height;
    let title_base_pos = egui::pos2(header_text_x, text_top);
    let header_bottom = title_base_pos.y + text_block_height;
    let header_base_rect = egui::Rect::from_min_max(
        egui::pos2(padded_rect.min.x + 8.0, content_top),
        egui::pos2(padded_rect.max.x - 8.0, header_bottom + 26.0),
    );
    let list_base_rect = egui::Rect::from_min_max(
        egui::pos2(padded_rect.min.x + 8.0, header_base_rect.max.y + 24.0),
        egui::pos2(padded_rect.max.x - 8.0, padded_rect.max.y - 52.0),
    );
    let content_offset = egui::vec2(0.0, page_enter_offset_y);
    let list_rect = list_base_rect.translate(content_offset);
    let title_pos = title_base_pos + content_offset;
    draw_selected_game_header(&painter, &header, &game.name, title_pos, wake_t);
    if let Some(tag_text) = dlss_tag_text(game) {
        let _ = draw_title_tag(
            &painter,
            &tag_text,
            title_pos,
            header.title_galley.size(),
            panel_t * wake_t,
            0.0,
            egui::Color32::from_rgb(228, 228, 220),
            egui::Color32::from_rgb(18, 18, 18),
        );
    }

    painter.rect_filled(
        list_rect,
        egui::Rounding::same(8.0),
        color_with_scaled_alpha(egui::Color32::from_rgb(14, 14, 14), wake_t),
    );

    let list_inner_rect = egui::Rect::from_min_max(
        egui::pos2(list_rect.min.x + 10.0, list_rect.min.y + 16.0),
        egui::pos2(list_rect.max.x - 18.0, list_rect.max.y - 16.0),
    );
    let sort_badge_text = if sort_high_to_low {
        language.unlock_rate_high_to_low_text()
    } else {
        language.unlock_rate_low_to_high_text()
    };
    let row_side_inset = 6.0;
    let unselected_row_shrink_x = 7.0;
    let sort_badge_x = list_inner_rect.min.x + row_side_inset + unselected_row_shrink_x;
    let sort_badge_size = draw_badge(
        &painter,
        sort_badge_text,
        egui::pos2(sort_badge_x, list_inner_rect.min.y),
        egui::Color32::from_rgba_unmultiplied(70, 86, 104, 190),
        egui::Color32::from_rgb(226, 232, 240),
        wake_t,
    );
    if let Some(icons) = hint_icons {
        let icon_size = 28.0;
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(
                sort_badge_x + sort_badge_size.x + 10.0,
                list_inner_rect.min.y + (sort_badge_size.y - icon_size) * 0.5,
            ),
            egui::vec2(icon_size, icon_size),
        );
        painter.image(
            icons.btn_y.id(),
            icon_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            color_with_scaled_alpha(egui::Color32::WHITE, wake_t),
        );
    }

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

    let item_gap_y = 14.0;
    let row_spacing = 116.0;
    let header_band_height = 42.0;
    let list_body_rect = egui::Rect::from_min_max(
        egui::pos2(list_inner_rect.min.x, list_inner_rect.min.y + header_band_height),
        list_inner_rect.max,
    );
    let list_painter = painter.with_clip_rect(list_body_rect);
    let visible_rows = (list_body_rect.height() / row_spacing).ceil() as i32 + 2;
    let base_y = list_body_rect.min.y - scroll_offset * row_spacing;

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
        let row_rect = row_slot_rect.shrink2(egui::vec2(
            lerp_f32(unselected_row_shrink_x, 0.0, selection_t),
            0.0,
        ));
        let content_padding_x = 14.0;
        let content_padding_y = 12.0;
        let icon_gap = 14.0;
        let right_padding = 18.0;
        let hidden_state = item.is_hidden && item.unlocked != Some(true);
        let hidden_revealing = hidden_state
            && revealed_hidden.is_some_and(|revealed_api_name| revealed_api_name == item.api_name);
        let hidden_masked = hidden_state && !hidden_revealing;
        let bg_color = if item.unlocked == Some(true) {
            if is_selected {
                egui::Color32::from_rgb(28, 35, 31)
            } else {
                egui::Color32::from_rgb(21, 27, 23)
            }
        } else if is_selected {
            egui::Color32::from_rgb(30, 32, 36)
        } else {
            egui::Color32::from_rgb(22, 24, 28)
        };
        list_painter.rect_filled(
            row_rect,
            egui::Rounding::same(6.0),
            color_with_scaled_alpha(bg_color, wake_t),
        );

        let fill_t = achievement_percent_fill_t(item.global_percent);
        if fill_t > 0.001 {
            let fill_color = if item.unlocked == Some(true) {
                if is_selected {
                    egui::Color32::from_rgba_unmultiplied(96, 156, 124, 62)
                } else {
                    egui::Color32::from_rgba_unmultiplied(82, 140, 110, 50)
                }
            } else if is_selected {
                egui::Color32::from_rgba_unmultiplied(162, 166, 172, 32)
            } else {
                egui::Color32::from_rgba_unmultiplied(144, 148, 154, 24)
            };
            let fill_max_x = lerp_f32(row_rect.min.x, row_rect.max.x, fill_t);
            let fill_clip_rect = egui::Rect::from_min_max(
                row_rect.min,
                egui::pos2(fill_max_x.max(row_rect.min.x), row_rect.max.y),
            );
            list_painter.with_clip_rect(fill_clip_rect).rect_filled(
                row_rect,
                egui::Rounding::same(6.0),
                color_with_scaled_alpha(fill_color, wake_t),
            );
        }

        let icon_column_width = lerp_f32(48.0, 56.0, selection_t);
        let left_content_inset = content_padding_x;
        let text_x = row_rect.min.x + left_content_inset + icon_column_width + icon_gap;
        let right_column_width = 150.0;
        let percent_galley = painter.layout_no_wrap(
            format_achievement_percent(item.global_percent),
            egui::FontId::new(17.0, egui::FontFamily::Name("Bold".into())),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(230, 232, 236, 230),
                wake_t,
            ),
        );
        let unlock_time_text = format_achievement_status(item.unlocked, item.unlock_time);
        let unlock_time_galley = unlock_time_text.as_ref().map(|text| {
            painter.layout_no_wrap(
                text.clone(),
                egui::FontId::proportional(13.0),
                color_with_scaled_alpha(
                    egui::Color32::from_rgba_unmultiplied(150, 154, 162, 220),
                    wake_t,
                ),
            )
        });
        let text_width = (row_rect.width() - (text_x - row_rect.min.x) - right_column_width - 18.0)
            .max(180.0);
        let name = item
            .display_name
            .as_deref()
            .filter(|text| !text.trim().is_empty())
            .unwrap_or(&item.api_name);
        let title_galley = build_wrapped_galley(
            ui,
            name.to_string(),
            if is_selected {
                egui::FontId::new(
                    lerp_f32(18.0, 20.0, selection_t),
                    egui::FontFamily::Name("Bold".into()),
                )
            } else {
                egui::FontId::proportional(18.0)
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
            text_width,
        );
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
            egui::FontId::proportional(14.0),
            color_with_scaled_alpha(
                if hidden_masked {
                    egui::Color32::from_rgba_unmultiplied(150, 154, 160, 176)
                } else {
                    egui::Color32::from_rgba_unmultiplied(148, 152, 160, 220)
                },
                wake_t,
            ),
            text_width,
        );
        let text_block_height = title_galley.size().y + 6.0 + description_galley.size().y;
        let icon_size = text_block_height
            .min(row_rect.height() - content_padding_y * 2.0)
            .clamp(40.0, icon_column_width);
        let content_top = row_rect.min.y + (row_rect.height() - text_block_height.max(icon_size)) * 0.5;
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(
                row_rect.min.x + left_content_inset + (icon_column_width - icon_size) * 0.5,
                content_top,
            ),
            egui::vec2(icon_size, icon_size),
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
                egui::Color32::from_rgb(102, 106, 112)
            } else {
                match item.unlocked {
                    Some(true) => egui::Color32::from_rgb(86, 172, 132),
                    Some(false) => egui::Color32::from_rgb(108, 112, 122),
                    None => egui::Color32::from_rgb(82, 88, 102),
                }
            };
            list_painter.rect_filled(
                icon_rect,
                egui::Rounding::same(4.0),
                color_with_scaled_alpha(fill, wake_t),
            );
        }

        let text_top = content_top;
        list_painter.galley(egui::pos2(text_x, text_top), title_galley.clone());
        let description_pos = egui::pos2(text_x, text_top + title_galley.size().y + 6.0);
        list_painter.galley(description_pos, description_galley.clone());
        let right_column_rect = egui::Rect::from_min_max(
            egui::pos2(row_rect.max.x - right_padding - right_column_width, row_rect.min.y),
            egui::pos2(row_rect.max.x - right_padding, row_rect.max.y),
        );
        let right_block_spacing = 8.0;
        let right_block_height = percent_galley.size().y
            + unlock_time_galley
                .as_ref()
                .map(|galley| right_block_spacing + galley.size().y)
                .unwrap_or(0.0);
        let right_block_top = right_column_rect.center().y - right_block_height * 0.5;
        let right_column_x = right_column_rect.max.x;
        let percent_pos = egui::pos2(right_column_x - percent_galley.size().x, right_block_top);
        list_painter.galley(percent_pos, percent_galley.clone());
        if let Some(unlock_time_galley) = unlock_time_galley {
            list_painter.galley(
                egui::pos2(
                    right_column_x - unlock_time_galley.size().x,
                    percent_pos.y + percent_galley.size().y + right_block_spacing,
                ),
                unlock_time_galley,
            );
        }

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
