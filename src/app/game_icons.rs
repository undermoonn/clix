use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use eframe::egui;

use crate::assets::cover;
use crate::steam::Game;

const ICON_WINDOW_RADIUS: usize = 11;

pub struct GameIconState {
    textures: HashMap<u32, egui::TextureHandle>,
}

impl GameIconState {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    pub fn ensure_loaded(
        &mut self,
        ctx: &egui::Context,
        steam_paths: &[PathBuf],
        games: &[Game],
        selected: usize,
    ) {
        if games.is_empty() {
            self.textures.clear();
            return;
        }

        let selected = selected.min(games.len().saturating_sub(1));
        let range_start = selected.saturating_sub(ICON_WINDOW_RADIUS);
        let range_end = (selected + ICON_WINDOW_RADIUS + 1).min(games.len());
        let visible_app_ids: HashSet<u32> = games[range_start..range_end]
            .iter()
            .filter_map(|game| game.app_id)
            .collect();

        self.textures
            .retain(|app_id, _| visible_app_ids.contains(app_id));

        for game in &games[range_start..range_end] {
            if let Some(app_id) = game.app_id {
                if self.textures.contains_key(&app_id) {
                    continue;
                }

                if let Some(bytes) = cover::load_game_icon_bytes(steam_paths, game) {
                    if let Some(texture) = cover::bytes_to_game_icon_texture(
                        ctx,
                        &bytes,
                        format!("icon_{}", app_id),
                    )
                    {
                        self.textures.insert(app_id, texture);
                    }
                }
            }
        }
    }

    pub fn get(&self, app_id: u32) -> Option<&egui::TextureHandle> {
        self.textures.get(&app_id)
    }

    pub fn textures(&self) -> &HashMap<u32, egui::TextureHandle> {
        &self.textures
    }
}