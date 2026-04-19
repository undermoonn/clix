use std::sync::Arc;

use eframe::egui;

use crate::i18n::AppLanguage;
use crate::steam::{AchievementSummary, Game};

use super::text::{build_wrapped_galley, corner_radius, scale_alpha};

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

pub(crate) fn draw_title_tag(
    painter: &egui::Painter,
    text: &str,
    title_pos: egui::Pos2,
    title_size: egui::Vec2,
    opacity: f32,
    x_offset: f32,
    fill_color: egui::Color32,
    text_color: egui::Color32,
) -> f32 {
    let alpha = (255.0 * opacity.clamp(0.0, 1.0)).round() as u8;
    if alpha == 0 {
        return 0.0;
    }

    let tag_font = egui::FontId::new(
        (title_size.y * 0.42).clamp(11.0, 14.0),
        egui::FontFamily::Name("Bold".into()),
    );
    let text_color = egui::Color32::from_rgba_unmultiplied(
        text_color.r(),
        text_color.g(),
        text_color.b(),
        alpha,
    );
    let galley = painter.layout_no_wrap(text.to_owned(), tag_font, text_color);
    let padding_x = 11.0;
    let padding_y = 4.0;
    let tag_rect = egui::Rect::from_min_size(
        egui::pos2(
            title_pos.x + title_size.x + 14.0 + x_offset,
            title_pos.y + title_size.y * 0.5 - galley.size().y * 0.5 - padding_y,
        ),
        egui::vec2(
            galley.size().x + padding_x * 2.0,
            galley.size().y + padding_y * 2.0,
        ),
    );

    painter.rect_filled(
        tag_rect,
        corner_radius((tag_rect.height() * 0.5).min(10.0)),
        egui::Color32::from_rgba_unmultiplied(
            fill_color.r(),
            fill_color.g(),
            fill_color.b(),
            ((alpha as f32) * 0.9).round() as u8,
        ),
    );
    painter.galley(
        egui::pos2(tag_rect.min.x + padding_x, tag_rect.min.y + padding_y),
        galley,
        egui::Color32::WHITE,
    );

    tag_rect.width()
}

pub(crate) struct SelectedGameHeaderContent {
    pub(crate) title_galley: Arc<egui::Galley>,
    pub(crate) primary_meta_galley: Option<Arc<egui::Galley>>,
    pub(crate) achievement_galley: Option<Arc<egui::Galley>>,
    pub(crate) achievement_prev_galley: Option<Arc<egui::Galley>>,
    pub(crate) title_font: egui::FontId,
}

impl SelectedGameHeaderContent {
    pub(crate) fn total_height(&self) -> f32 {
        let meta_height = self
            .primary_meta_galley
            .as_ref()
            .map(|galley| galley.size().y)
            .into_iter()
            .chain(self.achievement_galley.as_ref().map(|galley| galley.size().y))
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
    let playtime_color = egui::Color32::from_rgba_unmultiplied(
        180,
        180,
        190,
        meta_alpha.clamp(0.0, 255.0) as u8,
    );
    let primary_meta_galley = has_primary_meta.then(|| {
        painter.layout_no_wrap(primary_meta_text, meta_font.clone(), playtime_color)
    });
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

pub(crate) fn draw_selected_game_header(
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
        painter.galley(title_pos + off, outline_galley.clone(), egui::Color32::WHITE);
    }

    painter.galley(title_pos, content.title_galley.clone(), egui::Color32::WHITE);

    if content.primary_meta_galley.is_some()
        || content.achievement_galley.is_some()
        || content.achievement_prev_galley.is_some()
    {
        let meta_pos = egui::pos2(title_pos.x, title_pos.y + content.title_galley.size().y + 6.0);
        let mut meta_x = meta_pos.x;
        if let Some(primary_meta_galley) = &content.primary_meta_galley {
            painter.galley(
                egui::pos2(meta_x, meta_pos.y),
                primary_meta_galley.clone(),
                egui::Color32::WHITE,
            );
            meta_x += primary_meta_galley.size().x;
        }
        if let Some(achievement_prev_galley) = &content.achievement_prev_galley {
            painter.galley(
                egui::pos2(meta_x, meta_pos.y),
                achievement_prev_galley.clone(),
                egui::Color32::WHITE,
            );
        }
        if let Some(achievement_galley) = &content.achievement_galley {
            painter.galley(
                egui::pos2(meta_x, meta_pos.y),
                achievement_galley.clone(),
                egui::Color32::WHITE,
            );
        }
    }
}
