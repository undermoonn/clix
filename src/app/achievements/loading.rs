use eframe::egui;
use std::sync::Arc;
use std::time::Instant;

use crate::game::Game;
use crate::i18n::AppLanguage;
use crate::steam;

use super::{
    overview_from_summary, preserve_missing_global_percents, AchievementState,
    PendingAchievementKind, MIN_REFRESH_RING_DURATION,
};

impl AchievementState {
    pub fn refresh_summary_for_selected(
        &mut self,
        selected_game: Option<&Game>,
        steam_paths: &[std::path::PathBuf],
        language: AppLanguage,
        ctx: &egui::Context,
    ) {
        self.refresh_summary_for_selected_inner(selected_game, steam_paths, language, ctx, false);
    }

    pub fn force_refresh_summary_for_selected(
        &mut self,
        selected_game: Option<&Game>,
        steam_paths: &[std::path::PathBuf],
        language: AppLanguage,
        ctx: &egui::Context,
    ) {
        self.refresh_summary_for_selected_inner(selected_game, steam_paths, language, ctx, true);
    }

    fn refresh_summary_for_selected_inner(
        &mut self,
        selected_game: Option<&Game>,
        steam_paths: &[std::path::PathBuf],
        language: AppLanguage,
        ctx: &egui::Context,
        force_refresh: bool,
    ) {
        let Some(steam_app_id) = selected_game.and_then(|game| game.steam_app_id) else {
            self.checked_overview_for = None;
            self.sync_summary_scope(None);
            return;
        };

        self.language = language;
        self.sync_summary_scope(Some(steam_app_id));

        if !force_refresh && self.overview_cache.contains_key(&steam_app_id) {
            self.no_data.remove(&steam_app_id);
            self.checked_overview_for = Some(steam_app_id);
            return;
        }

        if !force_refresh && self.checked_overview_for == Some(steam_app_id) {
            return;
        }
        self.checked_overview_for = Some(steam_app_id);

        if !force_refresh {
            if let Some(summary) = steam::load_cached_achievement_overview(steam_app_id, language) {
                self.no_data.remove(&steam_app_id);
                self.store_overview_summary(steam_app_id, summary, true);
                return;
            }
        }

        self.no_data.remove(&steam_app_id);
        if self.overview_loading.contains(&steam_app_id) {
            return;
        }

        self.overview_loading.insert(steam_app_id);
        let pending = Arc::clone(&self.pending);
        let paths = steam_paths.to_vec();
        let ctx_clone = ctx.clone();

        std::thread::spawn(move || {
            let data = steam::load_achievement_summary(steam_app_id, &paths, language, false);
            if let Ok(mut lock) = pending.lock() {
                lock.push((PendingAchievementKind::Overview, steam_app_id, data));
            }
            ctx_clone.request_repaint();
        });
    }

    pub fn refresh_after_global_percentage_update(
        &mut self,
        selected_game: Option<&Game>,
        steam_paths: &[std::path::PathBuf],
        language: AppLanguage,
        achievement_panel_open: bool,
        updated_app_ids: &[u32],
        ctx: &egui::Context,
    ) {
        let Some(steam_app_id) = selected_game.and_then(|game| game.steam_app_id) else {
            return;
        };

        if !updated_app_ids.contains(&steam_app_id) {
            return;
        }

        if achievement_panel_open {
            self.force_refresh_details_for_selected(selected_game, steam_paths, language, ctx);
        } else {
            self.force_refresh_summary_for_selected(selected_game, steam_paths, language, ctx);
        }
    }

    pub fn refresh_details_for_selected(
        &mut self,
        selected_game: Option<&Game>,
        steam_paths: &[std::path::PathBuf],
        language: AppLanguage,
        ctx: &egui::Context,
    ) {
        self.refresh_details_for_selected_inner(selected_game, steam_paths, language, ctx, false);
    }

    pub fn force_refresh_details_for_selected(
        &mut self,
        selected_game: Option<&Game>,
        steam_paths: &[std::path::PathBuf],
        language: AppLanguage,
        ctx: &egui::Context,
    ) {
        self.refresh_details_for_selected_inner(selected_game, steam_paths, language, ctx, true);
    }

    fn refresh_details_for_selected_inner(
        &mut self,
        selected_game: Option<&Game>,
        steam_paths: &[std::path::PathBuf],
        language: AppLanguage,
        ctx: &egui::Context,
        force_refresh: bool,
    ) {
        let Some(steam_app_id) = selected_game.and_then(|game| game.steam_app_id) else {
            self.checked_detail_for = None;
            self.sync_detail_scope(None);
            return;
        };

        self.language = language;
        self.sync_detail_scope(Some(steam_app_id));

        if !force_refresh {
            steam::request_global_achievement_percentages_refresh(steam_app_id);
        }

        if !force_refresh
            && self.checked_detail_for == Some(steam_app_id)
            && self
                .detail_cache
                .as_ref()
                .is_some_and(|(detail_app_id, summary)| {
                    *detail_app_id == steam_app_id && !summary.items.is_empty()
                })
        {
            return;
        }

        self.checked_detail_for = Some(steam_app_id);

        if !force_refresh {
            if let Some(mut summary) =
                steam::load_cached_achievement_summary(steam_app_id, language)
            {
                steam::sort_achievement_items(&mut summary.items);
                self.no_data.remove(&steam_app_id);
                self.store_detail_summary(steam_app_id, summary, false);
                return;
            }
        }

        self.no_data.remove(&steam_app_id);

        if force_refresh {
            self.refreshing.insert(steam_app_id);
            self.refresh_indicator_until
                .insert(steam_app_id, Instant::now() + MIN_REFRESH_RING_DURATION);
        }

        if self.detail_loading.contains(&steam_app_id) {
            return;
        }

        self.detail_loading.insert(steam_app_id);
        let pending = Arc::clone(&self.pending);
        let paths = steam_paths.to_vec();
        let ctx_clone = ctx.clone();
        let allow_global_percentage_refresh = force_refresh;

        std::thread::spawn(move || {
            let data = steam::load_achievement_summary(
                steam_app_id,
                &paths,
                language,
                allow_global_percentage_refresh,
            );
            if let Ok(mut lock) = pending.lock() {
                lock.push((PendingAchievementKind::Details, steam_app_id, data));
            }
            ctx_clone.request_repaint();
        });
    }

    pub fn drain_results(&mut self) {
        let pending_results = {
            let Ok(mut lock) = self.pending.lock() else {
                return;
            };
            lock.drain(..).collect::<Vec<_>>()
        };

        for (kind, steam_app_id, summary) in pending_results {
            match kind {
                PendingAchievementKind::Overview => {
                    self.overview_loading.remove(&steam_app_id);
                    match summary {
                        Some(summary) => {
                            steam::store_cached_achievement_summary(
                                steam_app_id,
                                &summary,
                                self.language,
                            );
                            self.no_data.remove(&steam_app_id);
                            self.store_overview_summary(
                                steam_app_id,
                                overview_from_summary(&summary),
                                true,
                            );
                        }
                        None => {
                            if !self.overview_cache.contains_key(&steam_app_id) {
                                self.no_data.insert(steam_app_id);
                            }
                        }
                    }
                }
                PendingAchievementKind::Details => {
                    self.detail_loading.remove(&steam_app_id);
                    self.refreshing.remove(&steam_app_id);
                    match summary {
                        Some(mut summary) => {
                            let previous_detail = self
                                .detail_cache
                                .as_ref()
                                .filter(|(detail_app_id, _)| *detail_app_id == steam_app_id)
                                .map(|(_, detail)| detail);
                            if let Some(previous_summary) = previous_detail {
                                preserve_missing_global_percents(&mut summary, previous_summary);
                            }
                            steam::sort_achievement_items(&mut summary.items);
                            steam::store_cached_achievement_summary(
                                steam_app_id,
                                &summary,
                                self.language,
                            );
                            self.no_data.remove(&steam_app_id);
                            self.store_detail_summary(steam_app_id, summary, true);
                        }
                        None => {
                            let has_detail = self
                                .detail_cache
                                .as_ref()
                                .is_some_and(|(detail_app_id, _)| *detail_app_id == steam_app_id);
                            if !has_detail && !self.overview_cache.contains_key(&steam_app_id) {
                                self.no_data.insert(steam_app_id);
                            }
                        }
                    }
                }
            }
        }
    }
}
