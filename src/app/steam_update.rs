use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::steam::{self, SteamUpdateProgress};

const STEAM_UPDATE_POLL_INTERVAL: Duration = Duration::from_secs(1);

pub struct SteamUpdateState {
    pending: Arc<Mutex<Vec<(u32, Option<SteamUpdateProgress>)>>>,
    loading: HashSet<u32>,
    latest: HashMap<u32, SteamUpdateProgress>,
    last_polled_at: HashMap<u32, Instant>,
}

impl SteamUpdateState {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(Vec::new())),
            loading: HashSet::new(),
            latest: HashMap::new(),
            last_polled_at: HashMap::new(),
        }
    }

    pub fn refresh_for_steam_app_id(
        &mut self,
        steam_app_id: Option<u32>,
        steam_paths: &[std::path::PathBuf],
        now: Instant,
        ctx: &egui::Context,
    ) {
        let Some(steam_app_id) = steam_app_id else {
            return;
        };

        if self.loading.contains(&steam_app_id) {
            return;
        }

        if self
            .last_polled_at
            .get(&steam_app_id)
            .is_some_and(|last| now.duration_since(*last) < STEAM_UPDATE_POLL_INTERVAL)
        {
            return;
        }

        self.loading.insert(steam_app_id);
        self.last_polled_at.insert(steam_app_id, now);

        let pending = Arc::clone(&self.pending);
        let paths = steam_paths.to_vec();
        let ctx_clone = ctx.clone();

        std::thread::spawn(move || {
            let progress = steam::load_game_update_progress(steam_app_id, &paths);
            if let Ok(mut lock) = pending.lock() {
                lock.push((steam_app_id, progress));
            }
            ctx_clone.request_repaint();
        });
    }

    pub fn drain_results(&mut self) {
        let Ok(mut lock) = self.pending.lock() else {
            return;
        };

        for (steam_app_id, progress) in lock.drain(..) {
            self.loading.remove(&steam_app_id);

            if let Some(progress) = progress {
                self.latest.insert(steam_app_id, progress);
            } else {
                self.latest.remove(&steam_app_id);
            }
        }
    }

    pub fn status_for_steam_app_id(
        &self,
        steam_app_id: Option<u32>,
    ) -> Option<&SteamUpdateProgress> {
        steam_app_id.and_then(|steam_app_id| self.latest.get(&steam_app_id))
    }
}
