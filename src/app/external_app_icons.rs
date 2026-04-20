use std::collections::{HashMap, HashSet};

use eframe::egui;

use crate::assets::cover;
use crate::system::external_apps::{ExternalApp, ExternalAppKind};

pub struct ExternalAppIconState {
    textures: HashMap<ExternalAppKind, egui::TextureHandle>,
}

impl ExternalAppIconState {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    pub fn ensure_loaded(&mut self, ctx: &egui::Context, apps: &[ExternalApp]) {
        let visible_kinds: HashSet<ExternalAppKind> = apps.iter().map(ExternalApp::kind).collect();
        self.textures
            .retain(|kind, _| visible_kinds.contains(kind));

        for app in apps {
            let kind = app.kind();
            if self.textures.contains_key(&kind) {
                continue;
            }

            if let Some(bytes) = cover::load_file_icon_bytes(app.icon_target()) {
                if let Some(texture) =
                    cover::bytes_to_game_icon_texture(ctx, &bytes, format!("external_icon_{kind:?}"))
                {
                    self.textures.insert(kind, texture);
                }
            }
        }
    }

    pub fn textures(&self) -> &HashMap<ExternalAppKind, egui::TextureHandle> {
        &self.textures
    }
}