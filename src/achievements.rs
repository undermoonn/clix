use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::cover;
use crate::i18n::AppLanguage;
use crate::steam::{self, AchievementSummary, Game};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AchievementSortOrder {
    UnlockRateDesc,
    UnlockRateAsc,
}

impl AchievementSortOrder {
    pub fn is_descending(self) -> bool {
        matches!(self, Self::UnlockRateDesc)
    }

    fn toggled(self) -> Self {
        match self {
            Self::UnlockRateDesc => Self::UnlockRateAsc,
            Self::UnlockRateAsc => Self::UnlockRateDesc,
        }
    }
}

struct HiddenRevealState {
    api_name: String,
    progress: f32,
}

pub struct AchievementState {
    language: AppLanguage,
    cache: HashMap<u32, AchievementSummary>,
    revealed_hidden: HashMap<u32, HiddenRevealState>,
    sort_order: AchievementSortOrder,
    pending: Arc<Mutex<Vec<(u32, Option<AchievementSummary>)>>>,
    loading: HashSet<u32>,
    refreshing: HashSet<u32>,
    no_data: HashSet<u32>,
    icon_cache: HashMap<String, egui::TextureHandle>,
    icon_pending: Arc<Mutex<Vec<(String, Option<Vec<u8>>)>>> ,
    icon_loading: HashSet<String>,
    icon_failed: HashSet<String>,
    icon_reveal: HashMap<String, f32>,
    text_reveal: HashMap<u32, f32>,
    checked_for: Option<u32>,
}

impl AchievementState {
    pub fn new() -> Self {
        Self {
            language: AppLanguage::English,
            cache: HashMap::new(),
            revealed_hidden: HashMap::new(),
            sort_order: AchievementSortOrder::UnlockRateDesc,
            pending: Arc::new(Mutex::new(Vec::new())),
            loading: HashSet::new(),
            refreshing: HashSet::new(),
            no_data: HashSet::new(),
            icon_cache: HashMap::new(),
            icon_pending: Arc::new(Mutex::new(Vec::new())),
            icon_loading: HashSet::new(),
            icon_failed: HashSet::new(),
            icon_reveal: HashMap::new(),
            text_reveal: HashMap::new(),
            checked_for: None,
        }
    }

    pub fn refresh_for_selected(
        &mut self,
        selected_game: Option<&Game>,
        steam_paths: &[std::path::PathBuf],
        language: AppLanguage,
        ctx: &egui::Context,
    ) {
        self.refresh_for_selected_inner(selected_game, steam_paths, language, ctx, false);
    }

    pub fn force_refresh_for_selected(
        &mut self,
        selected_game: Option<&Game>,
        steam_paths: &[std::path::PathBuf],
        language: AppLanguage,
        ctx: &egui::Context,
    ) {
        self.refresh_for_selected_inner(selected_game, steam_paths, language, ctx, true);
    }

    fn refresh_for_selected_inner(
        &mut self,
        selected_game: Option<&Game>,
        steam_paths: &[std::path::PathBuf],
        language: AppLanguage,
        ctx: &egui::Context,
        force_refresh: bool,
    ) {
        let Some(app_id) = selected_game.and_then(|game| game.app_id) else {
            self.checked_for = None;
            return;
        };

        if !force_refresh && self.checked_for == Some(app_id) && self.language == language {
            return;
        }

        self.language = language;
        self.checked_for = Some(app_id);

        if !force_refresh {
            if let Some(mut summary) = steam::load_cached_achievement_summary(app_id, language) {
                steam::sort_achievement_items(&mut summary.items, self.sort_order.is_descending());
                self.no_data.remove(&app_id);
                self.cache.insert(app_id, summary);
                self.text_reveal.insert(app_id, 1.0);
            }
        }

        self.no_data.remove(&app_id);

        if force_refresh {
            self.refreshing.insert(app_id);
        }

        if self.loading.contains(&app_id) {
            return;
        }

        self.loading.insert(app_id);
        let pending = Arc::clone(&self.pending);
        let paths = steam_paths.to_vec();
        let ctx_clone = ctx.clone();

        std::thread::spawn(move || {
            let data = steam::load_achievement_summary(app_id, &paths, language);
            if let Ok(mut lock) = pending.lock() {
                lock.push((app_id, data));
            }
            ctx_clone.request_repaint();
        });
    }

    pub fn drain_results(&mut self) {
        let Ok(mut lock) = self.pending.lock() else {
            return;
        };

        let sort_descending = self.sort_order.is_descending();

        for (app_id, summary) in lock.drain(..) {
            self.loading.remove(&app_id);
            self.refreshing.remove(&app_id);
            match summary {
                Some(mut summary) => {
                    let had_summary = self.cache.contains_key(&app_id);
                    if let Some(previous_summary) = self.cache.get(&app_id) {
                        preserve_missing_global_percents(&mut summary, previous_summary);
                    }
                    steam::sort_achievement_items(&mut summary.items, sort_descending);
                    steam::store_cached_achievement_summary(app_id, &summary, self.language);
                    self.no_data.remove(&app_id);
                    self.cache.insert(app_id, summary);
                    if !had_summary {
                        self.text_reveal.insert(app_id, 0.0);
                    } else {
                        self.text_reveal.entry(app_id).or_insert(1.0);
                    }
                }
                None => {
                    if !self.cache.contains_key(&app_id) {
                        self.no_data.insert(app_id);
                    }
                }
            }
        }
    }

    pub fn toggle_sort_order(&mut self) {
        self.sort_order = self.sort_order.toggled();
        let descending = self.sort_order.is_descending();
        for summary in self.cache.values_mut() {
            steam::sort_achievement_items(&mut summary.items, descending);
        }
    }

    pub fn sort_order(&self) -> AchievementSortOrder {
        self.sort_order
    }

    pub fn reveal_hidden_description_for_selected(
        &mut self,
        selected_game: Option<&Game>,
        selected_index: usize,
    ) -> bool {
        let Some(app_id) = selected_game.and_then(|game| game.app_id) else {
            return false;
        };
        let Some(api_name) = self
            .cache
            .get(&app_id)
            .and_then(|summary| summary.items.get(selected_index))
            .filter(|item| item.is_hidden && item.unlocked != Some(true))
            .map(|item| item.api_name.clone())
        else {
            return false;
        };

        if self
            .revealed_hidden
            .get(&app_id)
            .is_some_and(|state| state.api_name == api_name)
        {
            return false;
        }

        self.revealed_hidden.insert(
            app_id,
            HiddenRevealState {
                api_name,
                progress: 0.0,
            },
        );
        true
    }

    pub fn clear_revealed_hidden_for_selected(&mut self, selected_game: Option<&Game>) {
        if let Some(app_id) = selected_game.and_then(|game| game.app_id) {
            self.revealed_hidden.remove(&app_id);
        }
    }

    pub fn revealed_hidden_for_selected(&self, selected_game: Option<&Game>) -> Option<&str> {
        selected_game
            .and_then(|game| game.app_id)
            .and_then(|app_id| self.revealed_hidden.get(&app_id).map(|state| state.api_name.as_str()))
    }

    pub fn hidden_reveal_progress_for_selected(&self, selected_game: Option<&Game>) -> f32 {
        selected_game
            .and_then(|game| game.app_id)
            .and_then(|app_id| self.revealed_hidden.get(&app_id).map(|state| state.progress))
            .unwrap_or(0.0)
    }

    pub fn ensure_icons_for_urls(
        &mut self,
        ctx: &egui::Context,
        visible_icon_urls: &[String],
    ) {
        for url in visible_icon_urls {
            if self.icon_cache.contains_key(url)
                || self.icon_loading.contains(url)
                || self.icon_failed.contains(url)
            {
                continue;
            }

            if let Some(bytes) = cover::load_cached_achievement_icon_bytes(url) {
                self.icon_loading.insert(url.clone());
                if let Ok(mut lock) = self.icon_pending.lock() {
                    lock.push((url.clone(), Some(bytes)));
                }
                ctx.request_repaint();
                continue;
            }

            self.icon_loading.insert(url.clone());
            let pending = Arc::clone(&self.icon_pending);
            let ctx_clone = ctx.clone();
            let url_clone = url.clone();
            std::thread::spawn(move || {
                let bytes = cover::load_achievement_icon_bytes(&url_clone);
                if let Ok(mut lock) = pending.lock() {
                    lock.push((url_clone, bytes));
                }
                ctx_clone.request_repaint();
            });
        }
    }

    pub fn drain_icon_results(&mut self, ctx: &egui::Context) {
        let Ok(mut lock) = self.icon_pending.lock() else {
            return;
        };

        let mut hasher_seed = self.icon_cache.len();
        for (url, bytes) in lock.drain(..) {
            self.icon_loading.remove(&url);
            if self.icon_cache.contains_key(&url) {
                continue;
            }

            let Some(bytes) = bytes else {
                self.icon_failed.insert(url);
                continue;
            };

            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            hasher_seed.hash(&mut hasher);
            url.hash(&mut hasher);
            hasher_seed += 1;
            let label = format!("ach_icon_{:x}", hasher.finish());

            if let Some(texture) = cover::bytes_to_texture(ctx, &bytes, label) {
                self.icon_reveal.insert(url.clone(), 0.0);
                self.icon_cache.insert(url, texture);
            } else {
                cover::clear_cached_achievement_icon(&url);
                self.icon_failed.insert(url);
            }
        }
    }

    pub fn animate_reveals(&mut self, ctx: &egui::Context, dt: f32) {
        self.icon_reveal.retain(|_, progress| {
            if *progress >= 0.999 {
                return false;
            }

            const ACHIEVEMENT_ICON_FADE_IN_SECONDS: f32 = 0.3;
            *progress = (*progress + dt / ACHIEVEMENT_ICON_FADE_IN_SECONDS).min(1.0);
            if *progress < 0.999 {
                ctx.request_repaint();
                true
            } else {
                false
            }
        });

        self.revealed_hidden.retain(|_, state| {
            if state.progress >= 0.999 {
                return true;
            }

            const HIDDEN_REVEAL_SECONDS: f32 = 0.24;
            state.progress = (state.progress + dt / HIDDEN_REVEAL_SECONDS).min(1.0);
            if state.progress < 0.999 {
                ctx.request_repaint();
            }
            true
        });

        for progress in self.text_reveal.values_mut() {
            if *progress < 0.999 {
                const ACHIEVEMENT_TEXT_FADE_IN_SECONDS: f32 = 0.35;
                *progress = (*progress + dt / ACHIEVEMENT_TEXT_FADE_IN_SECONDS).min(1.0);
                if *progress < 0.999 {
                    ctx.request_repaint();
                }
            }
        }
    }

    pub fn summary_for_selected(&self, selected_game: Option<&Game>) -> Option<&AchievementSummary> {
        selected_game
            .and_then(|game| game.app_id)
            .and_then(|app_id| self.cache.get(&app_id))
    }

    pub fn text_reveal_for_selected(&self, selected_game: Option<&Game>) -> f32 {
        selected_game
            .and_then(|game| game.app_id)
            .and_then(|app_id| self.text_reveal.get(&app_id).copied())
            .unwrap_or(1.0)
    }

    pub fn loading_for_selected(&self, selected_game: Option<&Game>) -> bool {
        selected_game
            .and_then(|game| game.app_id)
            .map(|app_id| {
                self.loading.contains(&app_id)
                    || (!self.cache.contains_key(&app_id) && !self.no_data.contains(&app_id))
            })
            .unwrap_or(false)
    }

    pub fn refresh_loading_for_selected(&self, selected_game: Option<&Game>) -> bool {
        selected_game
            .and_then(|game| game.app_id)
            .map(|app_id| self.refreshing.contains(&app_id))
            .unwrap_or(false)
    }

    pub fn has_no_data_for_selected(&self, selected_game: Option<&Game>) -> bool {
        selected_game
            .and_then(|game| game.app_id)
            .map(|app_id| self.no_data.contains(&app_id))
            .unwrap_or(true)
    }

    pub fn icon_cache(&self) -> &HashMap<String, egui::TextureHandle> {
        &self.icon_cache
    }

    pub fn icon_reveal(&self) -> &HashMap<String, f32> {
        &self.icon_reveal
    }
}

fn preserve_missing_global_percents(
    summary: &mut AchievementSummary,
    previous_summary: &AchievementSummary,
) {
    let previous_percents: HashMap<&str, f32> = previous_summary
        .items
        .iter()
        .filter_map(|item| item.global_percent.map(|percent| (item.api_name.as_str(), percent)))
        .collect();

    for item in &mut summary.items {
        if item.global_percent.is_none() {
            if let Some(previous_percent) = previous_percents.get(item.api_name.as_str()) {
                item.global_percent = Some(*previous_percent);
            }
        }
    }
}
