use eframe::egui;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::game::{Game, GameIconKey};

pub struct DlssState {
    pending: Arc<Mutex<Vec<(GameIconKey, Option<String>)>>>,
    loading: HashSet<GameIconKey>,
    checked: HashSet<GameIconKey>,
}

impl DlssState {
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

        if game.dlss_version.is_some()
            || self.loading.contains(&game_key)
            || self.checked.contains(&game_key)
        {
            return;
        }

        self.loading.insert(game_key.clone());
        self.checked.insert(game_key.clone());
        let pending = Arc::clone(&self.pending);
        let game_path = game.path.clone();
        let app_id = game.app_id;
        let ctx_clone = ctx.clone();

        std::thread::spawn(move || {
            let dlss_version = crate::assets::dlss::detect_version(&game_path, app_id);
            if let Ok(mut lock) = pending.lock() {
                lock.push((game_key, dlss_version));
            }
            ctx_clone.request_repaint();
        });
    }

    pub fn drain_results(&mut self, games: &mut [Game]) {
        let Ok(mut lock) = self.pending.lock() else {
            return;
        };

        for (game_key, dlss_version) in lock.drain(..) {
            self.loading.remove(&game_key);

            let Some(game) = games.iter_mut().find(|game| game.icon_key() == game_key) else {
                continue;
            };

            game.dlss_version = dlss_version;
        }
    }
}