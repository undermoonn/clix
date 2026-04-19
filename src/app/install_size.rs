use eframe::egui;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::steam::{self, Game};

pub struct InstallSizeState {
    pending: Arc<Mutex<Vec<(u32, Option<u64>)>>>,
    loading: HashSet<u32>,
    checked: HashSet<u32>,
}

impl InstallSizeState {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(Vec::new())),
            loading: HashSet::new(),
            checked: HashSet::new(),
        }
    }

    pub fn refresh_for_selected(&mut self, selected_game: Option<&Game>, ctx: &egui::Context) {
        let Some(game) = selected_game else {
            return;
        };
        let Some(app_id) = game.app_id else {
            return;
        };

        if game.installed_size_bytes.is_some()
            || self.loading.contains(&app_id)
            || self.checked.contains(&app_id)
        {
            return;
        }

        self.loading.insert(app_id);
        self.checked.insert(app_id);
        let pending = Arc::clone(&self.pending);
        let game_path = game.path.clone();
        let ctx_clone = ctx.clone();

        std::thread::spawn(move || {
            let installed_size_bytes = steam::load_game_installed_size(&game_path);
            if let Ok(mut lock) = pending.lock() {
                lock.push((app_id, installed_size_bytes));
            }
            ctx_clone.request_repaint();
        });
    }

    pub fn drain_results(&mut self, games: &mut [Game]) {
        let Ok(mut lock) = self.pending.lock() else {
            return;
        };

        for (app_id, installed_size_bytes) in lock.drain(..) {
            self.loading.remove(&app_id);

            let Some(game) = games.iter_mut().find(|game| game.app_id == Some(app_id)) else {
                continue;
            };

            game.installed_size_bytes = installed_size_bytes;
        }
    }
}