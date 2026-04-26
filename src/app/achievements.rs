use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::assets::cover;
use crate::animation;
use crate::game::Game;
use crate::i18n::AppLanguage;
use crate::steam::{self, AchievementSummary};

struct HiddenRevealState {
    api_name: String,
    progress: f32,
    started_at: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingAchievementKind {
    Overview,
    Details,
}

type PendingAchievementResult = (PendingAchievementKind, u32, Option<AchievementSummary>);
type PendingAchievementIcon = (u64, String, Option<Vec<u8>>);

const MIN_REFRESH_RING_DURATION: Duration = Duration::from_millis(1000);

pub struct AchievementState {
    language: AppLanguage,
    overview_cache: HashMap<u32, AchievementSummary>,
    displayed_overview_app_id: Option<u32>,
    previous_overview: Option<AchievementSummary>,
    previous_overview_reveal: f32,
    previous_overview_reveal_started_at: Option<Instant>,
    detail_cache: Option<(u32, AchievementSummary)>,
    revealed_hidden: HashMap<u32, HiddenRevealState>,
    pending: Arc<Mutex<Vec<PendingAchievementResult>>>,
    overview_loading: HashSet<u32>,
    detail_loading: HashSet<u32>,
    refreshing: HashSet<u32>,
    refresh_indicator_until: HashMap<u32, Instant>,
    no_data: HashSet<u32>,
    icon_cache: HashMap<String, egui::TextureHandle>,
    icon_pending: Arc<Mutex<Vec<PendingAchievementIcon>>>,
    icon_loading: HashSet<String>,
    icon_failed: HashSet<String>,
    icon_reveal: HashMap<String, f32>,
    icon_reveal_started_at: HashMap<String, Instant>,
    percent_reveal: HashMap<String, f32>,
    percent_reveal_started_at: HashMap<String, Instant>,
    icon_scope_app_id: Option<u32>,
    icon_generation: u64,
    text_reveal: HashMap<u32, f32>,
    text_reveal_started_at: HashMap<u32, Instant>,
    checked_overview_for: Option<u32>,
    checked_detail_for: Option<u32>,
}

impl AchievementState {
    pub fn new() -> Self {
        Self {
            language: AppLanguage::English,
            overview_cache: HashMap::new(),
            displayed_overview_app_id: None,
            previous_overview: None,
            previous_overview_reveal: 0.0,
            previous_overview_reveal_started_at: None,
            detail_cache: None,
            revealed_hidden: HashMap::new(),
            pending: Arc::new(Mutex::new(Vec::new())),
            overview_loading: HashSet::new(),
            detail_loading: HashSet::new(),
            refreshing: HashSet::new(),
            refresh_indicator_until: HashMap::new(),
            no_data: HashSet::new(),
            icon_cache: HashMap::new(),
            icon_pending: Arc::new(Mutex::new(Vec::new())),
            icon_loading: HashSet::new(),
            icon_failed: HashSet::new(),
            icon_reveal: HashMap::new(),
            icon_reveal_started_at: HashMap::new(),
            percent_reveal: HashMap::new(),
            percent_reveal_started_at: HashMap::new(),
            icon_scope_app_id: None,
            icon_generation: 0,
            text_reveal: HashMap::new(),
            text_reveal_started_at: HashMap::new(),
            checked_overview_for: None,
            checked_detail_for: None,
        }
    }

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
            if let Some(summary) =
                steam::load_cached_achievement_overview(steam_app_id, language)
            {
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
            && self.detail_cache
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

    pub fn reveal_hidden_description_for_selected(
        &mut self,
        selected_game: Option<&Game>,
        selected_index: usize,
    ) -> bool {
        let Some(steam_app_id) = selected_game.and_then(|game| game.steam_app_id) else {
            return false;
        };
        let Some(api_name) = self
            .detail_cache
            .as_ref()
            .filter(|(detail_app_id, _)| *detail_app_id == steam_app_id)
            .map(|(_, summary)| summary)
            .and_then(|summary| summary.items.get(selected_index))
            .filter(|item| item.is_hidden && item.unlocked != Some(true))
            .map(|item| item.api_name.clone())
        else {
            return false;
        };

        if self
            .revealed_hidden
            .get(&steam_app_id)
            .is_some_and(|state| state.api_name == api_name)
        {
            return false;
        }

        self.revealed_hidden.insert(
            steam_app_id,
            HiddenRevealState {
                api_name,
                progress: 0.0,
                started_at: Instant::now(),
            },
        );
        true
    }

    pub fn clear_revealed_hidden_for_selected(&mut self, selected_game: Option<&Game>) {
        if let Some(steam_app_id) = selected_game.and_then(|game| game.steam_app_id) {
            self.revealed_hidden.remove(&steam_app_id);
        }
    }

    pub fn revealed_hidden_for_selected(&self, selected_game: Option<&Game>) -> Option<&str> {
        selected_game
            .and_then(|game| game.steam_app_id)
            .and_then(|steam_app_id| {
                self.revealed_hidden
                    .get(&steam_app_id)
                    .map(|state| state.api_name.as_str())
            })
    }

    pub fn hidden_reveal_progress_for_selected(&self, selected_game: Option<&Game>) -> f32 {
        selected_game
            .and_then(|game| game.steam_app_id)
            .and_then(|steam_app_id| {
                self.revealed_hidden
                    .get(&steam_app_id)
                    .map(|state| state.progress)
            })
            .unwrap_or(0.0)
    }

    pub fn sync_icon_scope(&mut self, steam_app_id: Option<u32>) {
        self.reset_icon_scope(steam_app_id);
    }

    pub fn ensure_icons_for_urls(
        &mut self,
        steam_app_id: Option<u32>,
        ctx: &egui::Context,
        visible_icon_urls: &[String],
    ) {
        self.reset_icon_scope(steam_app_id);
        let generation = self.icon_generation;

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
                    lock.push((generation, url.clone(), Some(bytes)));
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
                    lock.push((generation, url_clone, bytes));
                }
                ctx_clone.request_repaint();
            });
        }
    }

    pub fn drain_icon_results(&mut self, ctx: &egui::Context) {
        let pending_icons = {
            let Ok(mut lock) = self.icon_pending.lock() else {
                return;
            };
            lock.drain(..).collect::<Vec<_>>()
        };

        let mut hasher_seed = self.icon_cache.len();
        for (generation, url, bytes) in pending_icons {
            if generation != self.icon_generation {
                continue;
            }

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

            if let Some(texture) = cover::bytes_to_achievement_icon_texture(ctx, &bytes, label) {
                self.icon_reveal.insert(url.clone(), 0.0);
                self.icon_reveal_started_at.insert(url.clone(), Instant::now());
                self.icon_cache.insert(url, texture);
            } else {
                cover::clear_cached_achievement_icon(&url);
                self.icon_failed.insert(url);
            }
        }
    }

    pub fn animate_reveals(&mut self, ctx: &egui::Context, now: Instant) {
        const ACHIEVEMENT_ICON_FADE_IN_SECONDS: f32 = 0.3;
        const ACHIEVEMENT_PERCENT_FADE_IN_SECONDS: f32 = 0.3;
        const HIDDEN_REVEAL_SECONDS: f32 = 0.24;
        const ACHIEVEMENT_TEXT_FADE_IN_SECONDS: f32 = 0.35;
        const ACHIEVEMENT_TEXT_FADE_OUT_SECONDS: f32 = 0.22;

        let mut finished_icon_reveals = Vec::new();
        for (url, progress) in &mut self.icon_reveal {
            let Some(started_at) = self.icon_reveal_started_at.get(url).copied() else {
                finished_icon_reveals.push(url.clone());
                continue;
            };

            *progress = animation::linear_progress(started_at, now, ACHIEVEMENT_ICON_FADE_IN_SECONDS);
            if *progress < 0.999 {
                ctx.request_repaint();
            } else {
                finished_icon_reveals.push(url.clone());
            }
        }
        for url in finished_icon_reveals {
            self.icon_reveal.remove(&url);
            self.icon_reveal_started_at.remove(&url);
        }

        let mut finished_percent_reveals = Vec::new();
        for (api_name, progress) in &mut self.percent_reveal {
            let Some(started_at) = self.percent_reveal_started_at.get(api_name).copied() else {
                finished_percent_reveals.push(api_name.clone());
                continue;
            };

            *progress =
                animation::linear_progress(started_at, now, ACHIEVEMENT_PERCENT_FADE_IN_SECONDS);
            if *progress < 0.999 {
                ctx.request_repaint();
            } else {
                finished_percent_reveals.push(api_name.clone());
            }
        }
        for api_name in finished_percent_reveals {
            self.percent_reveal.remove(&api_name);
            self.percent_reveal_started_at.remove(&api_name);
        }

        for state in self.revealed_hidden.values_mut() {
            state.progress = animation::linear_progress(state.started_at, now, HIDDEN_REVEAL_SECONDS);
            if state.progress < 0.999 {
                ctx.request_repaint();
            }
        }

        let mut finished_text_reveals = Vec::new();
        for (steam_app_id, progress) in &mut self.text_reveal {
            let Some(started_at) = self.text_reveal_started_at.get(steam_app_id).copied() else {
                continue;
            };

            *progress = animation::linear_progress(started_at, now, ACHIEVEMENT_TEXT_FADE_IN_SECONDS);
            if *progress < 0.999 {
                ctx.request_repaint();
            } else {
                *progress = 1.0;
                finished_text_reveals.push(*steam_app_id);
            }
        }
        for steam_app_id in finished_text_reveals {
            self.text_reveal_started_at.remove(&steam_app_id);
        }

        if let Some(started_at) = self.previous_overview_reveal_started_at {
            self.previous_overview_reveal =
                (1.0 - animation::linear_progress(started_at, now, ACHIEVEMENT_TEXT_FADE_OUT_SECONDS))
                    .max(0.0);
            if self.previous_overview_reveal > 0.001 {
                ctx.request_repaint();
            } else {
                self.previous_overview = None;
                self.previous_overview_reveal = 0.0;
                self.previous_overview_reveal_started_at = None;
            }
        }

        let now = Instant::now();
        self.refresh_indicator_until.retain(|_, until| {
            let keep = *until > now;
            if keep {
                ctx.request_repaint();
            }
            keep
        });
    }

    pub fn summary_for_selected(&self, selected_game: Option<&Game>) -> Option<&AchievementSummary> {
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

    pub fn sync_summary_scope(&mut self, steam_app_id: Option<u32>) {
        if self.displayed_overview_app_id != steam_app_id {
            if let Some(previous_app_id) = self.displayed_overview_app_id {
                if let Some(previous_summary) = self.overview_cache.get(&previous_app_id) {
                    if previous_summary.total > 0 {
                        self.previous_overview = Some(previous_summary.clone());
                        self.previous_overview_reveal = 1.0;
                        self.previous_overview_reveal_started_at = Some(Instant::now());
                    } else {
                        self.previous_overview = None;
                        self.previous_overview_reveal = 0.0;
                        self.previous_overview_reveal_started_at = None;
                    }
                } else {
                    self.previous_overview = None;
                    self.previous_overview_reveal = 0.0;
                    self.previous_overview_reveal_started_at = None;
                }
            } else {
                self.previous_overview = None;
                self.previous_overview_reveal = 0.0;
                self.previous_overview_reveal_started_at = None;
            }
            self.displayed_overview_app_id = steam_app_id;
        }
        if steam_app_id.is_none() {
            self.displayed_overview_app_id = None;
            self.checked_overview_for = None;
        }
    }

    pub fn sync_detail_scope(&mut self, steam_app_id: Option<u32>) {
        if self
            .detail_cache
            .as_ref()
            .is_some_and(|(detail_app_id, _)| Some(*detail_app_id) != steam_app_id)
        {
            self.detail_cache = None;
            self.percent_reveal.clear();
            self.percent_reveal_started_at.clear();
        }
        if steam_app_id.is_none() {
            self.checked_detail_for = None;
            self.percent_reveal.clear();
            self.percent_reveal_started_at.clear();
        }
    }

    fn reset_icon_scope(&mut self, steam_app_id: Option<u32>) {
        if self.icon_scope_app_id == steam_app_id {
            return;
        }

        self.icon_scope_app_id = steam_app_id;
        self.icon_generation = self.icon_generation.wrapping_add(1);
        self.icon_cache.clear();
        self.icon_loading.clear();
        self.icon_failed.clear();
        self.icon_reveal.clear();
        self.icon_reveal_started_at.clear();
    }

    fn store_overview_summary(
        &mut self,
        steam_app_id: u32,
        summary: AchievementSummary,
        animate_reveal: bool,
    ) {
        let had_summary = self.overview_cache.contains_key(&steam_app_id);
        self.overview_cache.insert(steam_app_id, summary);
        if animate_reveal && !had_summary {
            self.text_reveal.insert(steam_app_id, 0.0);
            self.text_reveal_started_at.insert(steam_app_id, Instant::now());
        } else {
            self.text_reveal.insert(steam_app_id, 1.0);
            self.text_reveal_started_at.remove(&steam_app_id);
        }
    }

    fn store_detail_summary(
        &mut self,
        steam_app_id: u32,
        summary: AchievementSummary,
        animate_reveal: bool,
    ) {
        let previous_percents: HashMap<String, f32> = self
            .detail_cache
            .as_ref()
            .filter(|(detail_app_id, _)| *detail_app_id == steam_app_id)
            .map(|(_, detail)| {
                detail
                    .items
                    .iter()
                    .filter_map(|item| item.global_percent.map(|percent| (item.api_name.clone(), percent)))
                    .collect()
            })
            .unwrap_or_default();
        self.update_percent_reveal(&summary, &previous_percents, animate_reveal);
        let overview = overview_from_summary(&summary);
        self.detail_cache = Some((steam_app_id, summary));
        self.store_overview_summary(steam_app_id, overview, animate_reveal);
    }

    fn update_percent_reveal(
        &mut self,
        summary: &AchievementSummary,
        previous_percents: &HashMap<String, f32>,
        animate_reveal: bool,
    ) {
        let now = Instant::now();
        let mut next_reveal = HashMap::new();
        let mut next_started_at = HashMap::new();
        for item in &summary.items {
            let Some(_) = item.global_percent else {
                continue;
            };

            let progress = if previous_percents.contains_key(&item.api_name) {
                self.percent_reveal
                    .get(&item.api_name)
                    .copied()
                    .unwrap_or(1.0)
            } else if animate_reveal {
                0.0
            } else {
                1.0
            };
            next_reveal.insert(item.api_name.clone(), progress);

            if progress < 0.999 {
                let started_at = self
                    .percent_reveal_started_at
                    .get(&item.api_name)
                    .copied()
                    .unwrap_or(now);
                next_started_at.insert(item.api_name.clone(), started_at);
            }
        }

        self.percent_reveal = next_reveal;
        self.percent_reveal_started_at = next_started_at;
    }
}

fn overview_from_summary(summary: &AchievementSummary) -> AchievementSummary {
    AchievementSummary {
        unlocked: summary.unlocked,
        total: summary.total,
        items: Vec::new(),
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
