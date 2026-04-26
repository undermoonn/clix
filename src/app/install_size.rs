use eframe::egui;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::game::{Game, GameIconKey};
use crate::steam;

pub struct InstallSizeState {
    pending: Arc<Mutex<Vec<(GameIconKey, Option<u64>)>>>,
    loading: HashSet<GameIconKey>,
    checked: HashSet<GameIconKey>,
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
        let game_key = game.icon_key();

        if game.installed_size_bytes.is_some()
            || self.loading.contains(&game_key)
            || self.checked.contains(&game_key)
        {
            return;
        }

        self.loading.insert(game_key.clone());
        self.checked.insert(game_key.clone());
        let pending = Arc::clone(&self.pending);
        let install_path = game.install_path.clone();
        let ctx_clone = ctx.clone();

        std::thread::spawn(move || {
            let installed_size_bytes = steam::load_game_installed_size(&install_path);
            if let Ok(mut lock) = pending.lock() {
                lock.push((game_key, installed_size_bytes));
            }
            ctx_clone.request_repaint();
        });
    }

    pub fn drain_results(&mut self, games: &mut [Game]) {
        let Ok(mut lock) = self.pending.lock() else {
            return;
        };

        for (game_key, installed_size_bytes) in lock.drain(..) {
            self.loading.remove(&game_key);

            let Some(game) = games.iter_mut().find(|game| game.icon_key() == game_key) else {
                continue;
            };

            game.installed_size_bytes = installed_size_bytes;
        }
    }
}