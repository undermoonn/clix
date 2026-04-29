use std::sync::Arc;

use eframe::egui;

use crate::game::{Game, GameSource};
use crate::i18n::AppLanguage;
use crate::steam::AchievementSummary;

use super::text::{build_wrapped_galley, color_with_scaled_alpha, corner_radius, scale_alpha};
use super::{design_units, lerp_f32};

pub(crate) fn dlss_tag_text(game: &Game) -> Option<String> {
    game.dlss_version.as_ref().map(|version| {
        let version = version.trim();
        if version.is_empty() {
            "DLSS".to_owned()
        } else {
            format!("DLSS {}", version)
        }
    })
}

pub(crate) fn installed_size_tag_text(language: AppLanguage, game: &Game) -> Option<String> {
    game.installed_size_bytes
        .map(|size_bytes| language.format_installed_size(size_bytes))
        .filter(|text| !text.is_empty())
}

pub(crate) fn game_source_badge_text(source: GameSource) -> &'static str {
    source.badge_label()
}

pub(crate) struct SelectedGameBadgeStyle {
    pub(crate) fill_color: egui::Color32,
    pub(crate) text_color: egui::Color32,
    pub(crate) stroke_color: Option<egui::Color32>,
}

impl SelectedGameBadgeStyle {
    pub(crate) fn steam() -> Self {
        Self {
            fill_color: egui::Color32::from_rgb(245, 245, 245),
            text_color: egui::Color32::from_rgb(20, 20, 20),
            stroke_color: None,
        }
    }

    pub(crate) fn detail_tag(fill_color: egui::Color32) -> Self {
        Self {
            fill_color,
            text_color: egui::Color32::from_rgb(244, 244, 246),
            stroke_color: Some(egui::Color32::WHITE),
        }
    }
}

pub(crate) fn draw_selected_game_text_badge(
    painter: &egui::Painter,
    text: &str,
    title_pos: egui::Pos2,
    title_size: egui::Vec2,
    layout_scale: f32,
    alpha_scale: f32,
) -> egui::Vec2 {
    draw_selected_game_text_badge_with_style(
        painter,
        text,
        title_pos,
        title_size,
        layout_scale,
        alpha_scale,
        &SelectedGameBadgeStyle::steam(),
    )
}

pub(crate) fn measure_selected_game_text_badge(
    painter: &egui::Painter,
    text: &str,
    title_size: egui::Vec2,
    layout_scale: f32,
) -> egui::Vec2 {
    let badge_font = egui::FontId::new(
        (title_size.y * 0.46).clamp(
            design_units(13.0, layout_scale),
            design_units(17.0, layout_scale),
        ),
        egui::FontFamily::Name("Bold".into()),
    );
    let badge_galley =
        painter.layout_no_wrap(text.to_owned(), badge_font, egui::Color32::TRANSPARENT);
    let padding_x = design_units(14.0, layout_scale);
    let padding_y = design_units(6.0, layout_scale);
    let gap = design_units(12.0, layout_scale);

    egui::vec2(
        badge_galley.size().x + padding_x * 2.0 + gap,
        badge_galley.size().y + padding_y * 2.0,
    )
}

pub(crate) fn draw_selected_game_text_badge_with_style(
    painter: &egui::Painter,
    text: &str,
    title_pos: egui::Pos2,
    title_size: egui::Vec2,
    layout_scale: f32,
    alpha_scale: f32,
    style: &SelectedGameBadgeStyle,
) -> egui::Vec2 {
    let alpha = scale_alpha(255, alpha_scale);
    if alpha == 0 {
        return egui::Vec2::ZERO;
    }

    let badge_font = egui::FontId::new(
        (title_size.y * 0.46).clamp(
            design_units(13.0, layout_scale),
            design_units(17.0, layout_scale),
        ),
        egui::FontFamily::Name("Bold".into()),
    );
    let badge_galley = painter.layout_no_wrap(
        text.to_owned(),
        badge_font,
        egui::Color32::from_rgba_unmultiplied(
            style.text_color.r(),
            style.text_color.g(),
            style.text_color.b(),
            alpha,
        ),
    );
    let padding_x = design_units(14.0, layout_scale);
    let padding_y = design_units(6.0, layout_scale);
    let gap = design_units(12.0, layout_scale);
    let badge_width = badge_galley.size().x + padding_x * 2.0;
    let badge_height = badge_galley.size().y + padding_y * 2.0;
    let badge_rect = egui::Rect::from_min_size(
        egui::pos2(
            title_pos.x,
            title_pos.y + title_size.y * 0.5 - badge_galley.size().y * 0.5 - padding_y,
        ),
        egui::vec2(badge_width, badge_height),
    );

    painter.rect_filled(
        badge_rect,
        corner_radius((badge_rect.height() * 0.5).min(design_units(10.0, layout_scale))),
        egui::Color32::from_rgba_unmultiplied(
            style.fill_color.r(),
            style.fill_color.g(),
            style.fill_color.b(),
            alpha,
        ),
    );
    if let Some(stroke_color) = style.stroke_color {
        painter.rect_stroke(
            badge_rect,
            corner_radius((badge_rect.height() * 0.5).min(design_units(10.0, layout_scale))),
            egui::Stroke::new(
                design_units(1.0, layout_scale),
                egui::Color32::from_rgba_unmultiplied(
                    stroke_color.r(),
                    stroke_color.g(),
                    stroke_color.b(),
                    ((alpha as f32) * 0.22).round() as u8,
                ),
            ),
            egui::StrokeKind::Middle,
        );
    }
    painter.galley(
        egui::pos2(badge_rect.min.x + padding_x, badge_rect.min.y + padding_y),
        badge_galley,
        egui::Color32::WHITE,
    );

    egui::vec2(badge_width + gap, badge_height)
}

pub(crate) struct SelectedGameHeaderContent {
    pub(crate) title_galley: Arc<egui::Galley>,
    pub(crate) primary_meta_galley: Option<Arc<egui::Galley>>,
    pub(crate) achievement_galley: Option<Arc<egui::Galley>>,
    pub(crate) achievement_prev_galley: Option<Arc<egui::Galley>>,
    pub(crate) title_font: egui::FontId,
}

pub(crate) struct SelectedGameSummaryStyle {
    pub(crate) show_playtime: bool,
    pub(crate) show_achievement_title: bool,
    pub(crate) hide_empty_achievement_card: bool,
    pub(crate) card_height: f32,
    pub(crate) layout_scale: f32,
}

impl Default for SelectedGameSummaryStyle {
    fn default() -> Self {
        Self {
            show_playtime: true,
            show_achievement_title: true,
            hide_empty_achievement_card: true,
            card_height: 106.0,
            layout_scale: 1.0,
        }
    }
}

impl SelectedGameHeaderContent {
    pub(crate) fn total_height(&self) -> f32 {
        let meta_height = self
            .primary_meta_galley
            .as_ref()
            .map(|galley| galley.size().y)
            .into_iter()
            .chain(
                self.achievement_galley
                    .as_ref()
                    .map(|galley| galley.size().y),
            )
            .chain(
                self.achievement_prev_galley
                    .as_ref()
                    .map(|galley| galley.size().y),
            )
            .fold(0.0, f32::max);

        self.title_galley.size().y
            + if meta_height > 0.0 {
                6.0 + meta_height
            } else {
                0.0
            }
    }
}

pub(crate) fn build_selected_game_header(
    ui: &egui::Ui,
    painter: &egui::Painter,
    language: AppLanguage,
    game: &Game,
    summary: Option<&AchievementSummary>,
    achievement_summary_reveal: f32,
    previous_summary: Option<&AchievementSummary>,
    previous_summary_reveal: f32,
    title_font: egui::FontId,
    title_color: egui::Color32,
    meta_font_size: f32,
    meta_alpha: f32,
    meta_max_width: f32,
) -> SelectedGameHeaderContent {
    let title_galley = painter.layout_no_wrap(game.name.clone(), title_font.clone(), title_color);
    let playtime_str = language.format_playtime(game.playtime_minutes);
    let primary_meta_text = [playtime_str]
        .into_iter()
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("  •  ");
    let has_primary_meta = !primary_meta_text.is_empty();
    let current_achievement_text = summary.and_then(|achievement_summary| {
        (achievement_summary.total > 0).then(|| {
            language.format_achievement_progress(
                achievement_summary.unlocked,
                achievement_summary.total,
            )
        })
    });
    let previous_achievement_text = previous_summary.and_then(|achievement_summary| {
        (achievement_summary.total > 0).then(|| {
            language.format_achievement_progress(
                achievement_summary.unlocked,
                achievement_summary.total,
            )
        })
    });
    let achievement_meta_reveal = achievement_summary_reveal.clamp(0.0, 1.0);
    let previous_achievement_meta_reveal = previous_summary_reveal.clamp(0.0, 1.0);
    let meta_font = egui::FontId::proportional(meta_font_size);
    let playtime_color =
        egui::Color32::from_rgba_unmultiplied(180, 180, 190, meta_alpha.clamp(0.0, 255.0) as u8);
    let primary_meta_galley = has_primary_meta
        .then(|| painter.layout_no_wrap(primary_meta_text, meta_font.clone(), playtime_color));
    let achievement_color = egui::Color32::from_rgba_unmultiplied(
        180,
        180,
        190,
        (meta_alpha * achievement_meta_reveal).clamp(0.0, 255.0) as u8,
    );
    let previous_achievement_color = egui::Color32::from_rgba_unmultiplied(
        180,
        180,
        190,
        (meta_alpha * previous_achievement_meta_reveal).clamp(0.0, 255.0) as u8,
    );
    let achievement_galley = current_achievement_text.map(|text| {
        let prefixed = if has_primary_meta {
            format!("  •  {}", text)
        } else {
            text
        };
        build_wrapped_galley(
            ui,
            prefixed,
            meta_font.clone(),
            achievement_color,
            meta_max_width,
        )
    });
    let achievement_prev_galley = previous_achievement_text.map(|text| {
        let prefixed = if has_primary_meta {
            format!("  •  {}", text)
        } else {
            text
        };
        build_wrapped_galley(
            ui,
            prefixed,
            meta_font.clone(),
            previous_achievement_color,
            meta_max_width,
        )
    });

    SelectedGameHeaderContent {
        title_galley,
        primary_meta_galley,
        achievement_galley,
        achievement_prev_galley,
        title_font,
    }
}

pub(crate) fn draw_selected_game_title(
    painter: &egui::Painter,
    content: &SelectedGameHeaderContent,
    game_name: &str,
    title_pos: egui::Pos2,
    alpha_scale: f32,
) {
    let outline_alpha = scale_alpha(200, alpha_scale);
    if outline_alpha == 0 {
        return;
    }

    let outline_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, outline_alpha);
    let outline_galley = painter.layout_no_wrap(
        game_name.to_owned(),
        content.title_font.clone(),
        outline_color,
    );
    let d = 0.8_f32;
    for off in [
        egui::vec2(d, 0.0),
        egui::vec2(-d, 0.0),
        egui::vec2(0.0, d),
        egui::vec2(0.0, -d),
        egui::vec2(d, d),
        egui::vec2(-d, d),
        egui::vec2(d, -d),
        egui::vec2(-d, -d),
    ] {
        painter.galley(
            title_pos + off,
            outline_galley.clone(),
            egui::Color32::WHITE,
        );
    }

    painter.galley(
        title_pos,
        content.title_galley.clone(),
        egui::Color32::WHITE,
    );
}

pub(crate) fn draw_selected_game_badge(
    painter: &egui::Painter,
    game: &Game,
    title_pos: egui::Pos2,
    title_size: egui::Vec2,
    layout_scale: f32,
    alpha_scale: f32,
) -> f32 {
    let badge_text = game_source_badge_text(game.source);
    draw_selected_game_text_badge(
        painter,
        badge_text,
        title_pos,
        title_size,
        layout_scale,
        alpha_scale,
    )
    .x
}

pub(crate) fn draw_selected_game_summary(
    painter: &egui::Painter,
    language: AppLanguage,
    game: &Game,
    summary: Option<&AchievementSummary>,
    summary_reveal: f32,
    summary_pos: egui::Pos2,
    playtime_width: f32,
    achievement_x: f32,
    achievement_width: f32,
    style: &SelectedGameSummaryStyle,
    summary_visibility: f32,
    _wake_t: f32,
) {
    let display_summary = summary.filter(|summary| summary.total > 0);
    let show_achievement_card = display_summary.is_some() || !style.hide_empty_achievement_card;
    let panel_alpha = summary_visibility;
    let content_alpha = summary_visibility;
    let layout_scale = style.layout_scale.max(0.01);
    let achievement_reveal = if display_summary.is_some() {
        summary_reveal.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let achievement_panel_alpha = panel_alpha * achievement_reveal;
    let achievement_content_alpha = content_alpha * achievement_reveal;
    let card_height = style.card_height;
    let playtime_rect =
        egui::Rect::from_min_size(summary_pos, egui::vec2(playtime_width, card_height));
    let playtime_fill = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(28, 30, 34, 228),
        panel_alpha,
    );
    let achievement_fill = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(34, 36, 40, 236),
        achievement_panel_alpha,
    );
    let playtime_stroke = egui::Stroke::new(
        1.2 * layout_scale,
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 76),
            panel_alpha,
        ),
    );
    let achievement_stroke = egui::Stroke::new(
        1.2 * layout_scale,
        color_with_scaled_alpha(
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 76),
            achievement_panel_alpha,
        ),
    );
    let title_font = egui::FontId::new(12.0 * layout_scale, egui::FontFamily::Name("Bold".into()));
    let value_font = egui::FontId::new(28.0 * layout_scale, egui::FontFamily::Name("Bold".into()));
    let achievement_count_font =
        egui::FontId::new(23.0 * layout_scale, egui::FontFamily::Name("Bold".into()));
    let achievement_percent_font = egui::FontId::proportional(18.0 * layout_scale);
    let playtime_section_label_color = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(205, 205, 210, 220),
        content_alpha,
    );
    let achievement_section_label_color = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(205, 205, 210, 220),
        achievement_content_alpha,
    );
    let playtime_primary_text_color = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(250, 250, 252, 255),
        content_alpha,
    );
    let achievement_primary_text_color = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(250, 250, 252, 255),
        achievement_content_alpha,
    );
    let secondary_text_color = color_with_scaled_alpha(
        egui::Color32::from_rgba_unmultiplied(220, 220, 224, 232),
        achievement_content_alpha,
    );

    let section_titles = match language {
        AppLanguage::English => ("PLAYTIME", "ACHIEVEMENTS", "No achievements"),
        AppLanguage::SimplifiedChinese => ("游玩时间", "成就", "暂无成就"),
    };
    let playtime_value = {
        let formatted = language.format_playtime(game.playtime_minutes);
        if formatted.is_empty() {
            match language {
                AppLanguage::English => "0 min".to_owned(),
                AppLanguage::SimplifiedChinese => "0 分钟".to_owned(),
            }
        } else {
            formatted
        }
    };

    if style.show_playtime {
        painter.rect_filled(
            playtime_rect,
            corner_radius(14.0 * layout_scale),
            playtime_fill,
        );
        painter.rect_stroke(
            playtime_rect,
            corner_radius(14.0 * layout_scale),
            playtime_stroke,
            egui::StrokeKind::Middle,
        );
        let playtime_label = painter.layout_no_wrap(
            section_titles.0.to_owned(),
            title_font.clone(),
            playtime_section_label_color,
        );
        let playtime_galley =
            painter.layout_no_wrap(playtime_value, value_font, playtime_primary_text_color);
        painter.galley(
            egui::pos2(
                playtime_rect.min.x + 16.0 * layout_scale,
                playtime_rect.min.y + 10.0 * layout_scale,
            ),
            playtime_label,
            egui::Color32::WHITE,
        );
        painter.galley(
            egui::pos2(
                playtime_rect.min.x + 16.0 * layout_scale,
                playtime_rect.min.y + 42.0 * layout_scale,
            ),
            playtime_galley,
            egui::Color32::WHITE,
        );
    }

    if show_achievement_card {
        let achievement_rect = egui::Rect::from_min_size(
            egui::pos2(
                if style.show_playtime {
                    achievement_x
                } else {
                    summary_pos.x
                },
                summary_pos.y,
            ),
            egui::vec2(achievement_width, card_height),
        );
        painter.rect_filled(
            achievement_rect,
            corner_radius(14.0 * layout_scale),
            achievement_fill,
        );
        painter.rect_stroke(
            achievement_rect,
            corner_radius(14.0 * layout_scale),
            achievement_stroke,
            egui::StrokeKind::Middle,
        );

        let title_top = if style.show_achievement_title {
            10.0
        } else {
            0.0
        } * layout_scale;
        let count_top = if style.show_achievement_title {
            42.0
        } else {
            20.0
        } * layout_scale;
        let track_bottom = if style.show_achievement_title {
            19.0
        } else {
            14.0
        } * layout_scale;
        let track_top = if style.show_achievement_title {
            14.0
        } else {
            9.0
        } * layout_scale;

        if style.show_achievement_title {
            let achievement_label = painter.layout_no_wrap(
                section_titles.1.to_owned(),
                title_font,
                achievement_section_label_color,
            );
            painter.galley(
                egui::pos2(
                    achievement_rect.min.x + 16.0 * layout_scale,
                    achievement_rect.min.y + title_top,
                ),
                achievement_label,
                egui::Color32::WHITE,
            );
        }

        let unlocked = display_summary
            .and_then(|summary| summary.unlocked)
            .unwrap_or(0);
        let total = display_summary.map(|summary| summary.total).unwrap_or(0);
        let progress = if total > 0 {
            unlocked as f32 / total as f32
        } else {
            0.0
        };
        let progress_percent_text = if total > 0 {
            format!("{:.0}%", progress * 100.0)
        } else {
            "--".to_owned()
        };
        let achievement_count_text = if total > 0 {
            format!("{}/{}", unlocked, total)
        } else {
            section_titles.2.to_owned()
        };
        let count_galley = painter.layout_no_wrap(
            achievement_count_text,
            achievement_count_font,
            achievement_primary_text_color,
        );
        let percent_galley = painter.layout_no_wrap(
            progress_percent_text,
            achievement_percent_font,
            secondary_text_color,
        );
        let count_pos = egui::pos2(
            achievement_rect.min.x + 16.0 * layout_scale,
            achievement_rect.min.y + count_top,
        );
        let percent_pos = egui::pos2(
            achievement_rect.max.x - 16.0 * layout_scale - percent_galley.size().x,
            count_pos.y + count_galley.size().y - percent_galley.size().y,
        );
        painter.galley(count_pos, count_galley, egui::Color32::WHITE);
        painter.galley(percent_pos, percent_galley, egui::Color32::WHITE);

        let track_rect = egui::Rect::from_min_max(
            egui::pos2(
                achievement_rect.min.x + 16.0 * layout_scale,
                achievement_rect.max.y - track_bottom,
            ),
            egui::pos2(
                achievement_rect.max.x - 16.0 * layout_scale,
                achievement_rect.max.y - track_top,
            ),
        );
        painter.rect_filled(
            track_rect,
            corner_radius(track_rect.height() * 0.5),
            color_with_scaled_alpha(
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 100),
                achievement_content_alpha,
            ),
        );

        if progress > 0.0 {
            let fill_rect = egui::Rect::from_min_max(
                track_rect.min,
                egui::pos2(
                    lerp_f32(track_rect.min.x, track_rect.max.x, progress.clamp(0.0, 1.0)),
                    track_rect.max.y,
                ),
            );
            painter.rect_filled(
                fill_rect,
                corner_radius(fill_rect.height() * 0.5),
                color_with_scaled_alpha(
                    egui::Color32::from_rgba_unmultiplied(241, 241, 244, 188),
                    achievement_content_alpha,
                ),
            );
        }
    }
}
