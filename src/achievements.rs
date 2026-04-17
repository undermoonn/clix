use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::cover;
use crate::i18n::AppLanguage;
use crate::steam::{self, AchievementSummary, Game};

pub struct AchievementState {
    language: AppLanguage,
    cache: HashMap<u32, AchievementSummary>,
    pending: Arc<Mutex<Vec<(u32, Option<AchievementSummary>)>>>,
    loading: HashSet<u32>,
    no_data: HashSet<u32>,
    icon_cache: HashMap<String, egui::TextureHandle>,
    icon_pending: Arc<Mutex<Vec<(String, Option<Vec<u8>>)>>>,
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
            pending: Arc::new(Mutex::new(Vec::new())),
            loading: HashSet::new(),
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
        let Some(app_id) = selected_game.and_then(|game| game.app_id) else {
            self.checked_for = None;
            return;
        };

        if self.checked_for == Some(app_id) && self.language == language {
            return;
        }

        self.language = language;
        self.checked_for = Some(app_id);

        if let Some(summary) = steam::load_cached_achievement_summary(app_id, language) {
            self.no_data.remove(&app_id);
            self.cache.insert(app_id, summary);
            self.text_reveal.insert(app_id, 1.0);
        }

        self.no_data.remove(&app_id);

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

        for (app_id, summary) in lock.drain(..) {
            self.loading.remove(&app_id);
            match summary {
                Some(summary) => {
                    let had_summary = self.cache.contains_key(&app_id);
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