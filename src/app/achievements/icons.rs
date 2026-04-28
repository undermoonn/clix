use eframe::egui;
use std::sync::Arc;

use crate::assets::cover;

use super::AchievementState;

impl AchievementState {
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
                self.icon_reveal_started_at
                    .insert(url.clone(), std::time::Instant::now());
                self.icon_cache.insert(url, texture);
            } else {
                cover::clear_cached_achievement_icon(&url);
                self.icon_failed.insert(url);
            }
        }
    }
}
