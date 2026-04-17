use std::collections::HashMap;
use std::path::PathBuf;

use eframe::egui;

use crate::cover;
use crate::steam::Game;

pub struct GameIconState {
    textures: HashMap<u32, egui::TextureHandle>,
    loaded: bool,
}

impl GameIconState {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            loaded: false,
        }
    }

    pub fn ensure_loaded(
        &mut self,
        ctx: &egui::Context,
        steam_paths: &[PathBuf],
        games: &[Game],
    ) {
        if self.loaded {
            return;
        }

        self.loaded = true;
        for game in games {
            if let Some(app_id) = game.app_id {
                if let Some(bytes) = cover::load_game_icon_bytes(steam_paths, game) {
                    if let Some(texture) =
                        cover::bytes_to_texture(ctx, &bytes, format!("icon_{}", app_id))
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