use eframe::egui;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::game::Game;
use crate::steam;

pub struct PlaytimeState {
    pending: Arc<Mutex<Vec<(u32, Option<u32>)>>>,
    loading: HashSet<u32>,
}

impl PlaytimeState {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(Vec::new())),
            loading: HashSet::new(),
        }
    }

    pub fn refresh_for_selected(
        &mut self,
        selected_game: Option<&Game>,
        steam_paths: &[std::path::PathBuf],
        ctx: &egui::Context,
    ) {
        let Some(steam_app_id) = selected_game.and_then(|game| game.steam_app_id) else {
            return;
        };

        if self.loading.contains(&steam_app_id) {
            return;
        }

        self.loading.insert(steam_app_id);
        let pending = Arc::clone(&self.pending);
        let paths = steam_paths.to_vec();
        let ctx_clone = ctx.clone();

        std::thread::spawn(move || {
            let playtime_minutes = steam::load_game_playtime_minutes(steam_app_id, &paths);
            if let Ok(mut lock) = pending.lock() {
                lock.push((steam_app_id, playtime_minutes));
            }
            ctx_clone.request_repaint();
        });
    }

    pub fn drain_results(&mut self, games: &mut [Game]) {
        let Ok(mut lock) = self.pending.lock() else {
            return;
        };

        for (steam_app_id, playtime_minutes) in lock.drain(..) {
            self.loading.remove(&steam_app_id);

            let Some(game) = games
                .iter_mut()
                .find(|game| game.steam_app_id == Some(steam_app_id))
            else {
                continue;
            };

            if let Some(playtime_minutes) = playtime_minutes {
                game.playtime_minutes = playtime_minutes;
            }
        }
    }
}
