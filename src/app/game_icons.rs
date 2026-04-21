use std::collections::HashMap;
use std::path::PathBuf;

use eframe::egui;

use crate::assets::cover;
use crate::steam::Game;

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
        _selected: usize,
    ) {
        if games.is_empty() {
            self.textures.clear();
            return;
        }

        for game in games {
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