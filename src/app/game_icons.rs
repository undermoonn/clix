use std::collections::HashMap;
use std::path::PathBuf;

use eframe::egui;

use crate::assets::cover;
use crate::game::{Game, GameIconKey};

pub struct GameIconState {
    textures: HashMap<GameIconKey, egui::TextureHandle>,
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
        _selected: usize,
    ) {
        if games.is_empty() {
            self.textures.clear();
            return;
        }

        for game in games {
            let icon_key = game.icon_key();
            if self.textures.contains_key(&icon_key) {
                continue;
            }

            if let Some(bytes) = cover::load_game_icon_bytes(steam_paths, game) {
                if let Some(texture) = cover::bytes_to_game_icon_texture(
                    ctx,
                    &bytes,
                    format!("icon_{:?}", icon_key),
                ) {
                    self.textures.insert(icon_key, texture);
                }
            }
        }
    }

    pub fn get(&self, icon_key: &GameIconKey) -> Option<&egui::TextureHandle> {
        self.textures.get(icon_key)
    }

    pub fn textures(&self) -> &HashMap<GameIconKey, egui::TextureHandle> {
        &self.textures
    }
}