use eframe::egui;
use std::collections::HashMap;
use std::time::Instant;

use crate::game::Game;
use crate::steam::AchievementSummary;

use super::AchievementState;

impl AchievementState {
    pub fn summary_for_selected(
        &self,
        selected_game: Option<&Game>,
    ) -> Option<&AchievementSummary> {
        selected_game
            .and_then(|game| game.steam_app_id)
            .and_then(|steam_app_id| self.overview_cache.get(&steam_app_id))
    }

    pub fn detail_for_selected(&self, selected_game: Option<&Game>) -> Option<&AchievementSummary> {
        let steam_app_id = selected_game.and_then(|game| game.steam_app_id)?;
        self.detail_cache
            .as_ref()
            .and_then(|(detail_app_id, summary)| {
                (*detail_app_id == steam_app_id).then_some(summary)
            })
    }

    pub fn detail_len_for_selected(&self, selected_game: Option<&Game>) -> usize {
        self.detail_for_selected(selected_game)
            .map(|summary| summary.items.len())
            .unwrap_or(0)
    }

    pub fn previous_summary_for_display(&self) -> Option<&AchievementSummary> {
        self.previous_overview.as_ref()
    }

    pub fn previous_summary_reveal(&self) -> f32 {
        self.previous_overview_reveal
    }

    pub fn text_reveal_for_selected(&self, selected_game: Option<&Game>) -> f32 {
        selected_game
            .and_then(|game| game.steam_app_id)
            .and_then(|steam_app_id| self.text_reveal.get(&steam_app_id).copied())
            .unwrap_or(1.0)
    }

    pub fn loading_for_selected(&self, selected_game: Option<&Game>) -> bool {
        selected_game
            .and_then(|game| game.steam_app_id)
            .map(|steam_app_id| {
                let has_current_detail = self
                    .detail_cache
                    .as_ref()
                    .is_some_and(|(detail_app_id, _)| *detail_app_id == steam_app_id);

                !has_current_detail
                    && (self.detail_loading.contains(&steam_app_id)
                        || !self.no_data.contains(&steam_app_id))
            })
            .unwrap_or(false)
    }

    pub fn refresh_loading_for_selected(&self, selected_game: Option<&Game>) -> bool {
        selected_game
            .and_then(|game| game.steam_app_id)
            .map(|steam_app_id| {
                self.refreshing.contains(&steam_app_id)
                    || self
                        .refresh_indicator_until
                        .get(&steam_app_id)
                        .is_some_and(|until| *until > Instant::now())
            })
            .unwrap_or(false)
    }

    pub fn can_refresh_for_selected(&self, selected_game: Option<&Game>) -> bool {
        !self.refresh_loading_for_selected(selected_game)
    }

    pub fn has_no_data_for_selected(&self, selected_game: Option<&Game>) -> bool {
        selected_game
            .and_then(|game| game.steam_app_id)
            .map(|steam_app_id| self.no_data.contains(&steam_app_id))
            .unwrap_or(true)
    }

    pub fn icon_cache(&self) -> &HashMap<String, egui::TextureHandle> {
        &self.icon_cache
    }

    pub fn icon_reveal(&self) -> &HashMap<String, f32> {
        &self.icon_reveal
    }

    pub fn percent_reveal(&self) -> &HashMap<String, f32> {
        &self.percent_reveal
    }
}
