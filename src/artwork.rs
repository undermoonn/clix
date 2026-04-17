use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::cover;

struct PendingBackgroundAssets {
    app_id: u32,
    cover_bytes: Option<Vec<u8>>,
    logo_bytes: Option<Vec<u8>>,
}

pub struct ArtworkState {
    cover: Option<(u32, egui::TextureHandle)>,
    cover_prev: Option<(u32, egui::TextureHandle)>,
    logo: Option<(u32, egui::TextureHandle)>,
    logo_prev: Option<(u32, egui::TextureHandle)>,
    fade: f32,
    transition_ready: bool,
    loaded_for: Option<usize>,
    debounce_until: Option<Instant>,
    debounce_for: Option<usize>,
    pending: Arc<Mutex<Option<PendingBackgroundAssets>>>,
}

impl ArtworkState {
    pub fn new() -> Self {
        Self {
            cover: None,
            cover_prev: None,
            logo: None,
            logo_prev: None,
            fade: 1.0,
            transition_ready: true,
            loaded_for: None,
            debounce_until: None,
            debounce_for: None,
            pending: Arc::new(Mutex::new(None)),
        }
    }

    pub fn tick_selection(
        &mut self,
        selected: usize,
        selected_app_id: Option<u32>,
        steam_paths: &[PathBuf],
        ctx: &egui::Context,
    ) -> bool {
        if self.loaded_for != Some(selected) {
            if self.debounce_for != Some(selected) {
                self.debounce_for = Some(selected);
                self.debounce_until = Some(Instant::now() + Duration::from_millis(300));
            }
        }

        if let Some(deadline) = self.debounce_until {
            if Instant::now() >= deadline {
                self.debounce_until = None;
                if self.loaded_for != Some(selected) {
                    self.loaded_for = Some(selected);
                    self.refresh(selected_app_id, steam_paths, ctx);
                    return true;
                }
            } else {
                ctx.request_repaint();
            }
        }

        false
    }

    fn refresh(&mut self, selected_app_id: Option<u32>, steam_paths: &[PathBuf], ctx: &egui::Context) {
        self.cover_prev = self.cover.take();
        self.logo_prev = self.logo.take();
        self.fade = 0.0;
        self.transition_ready = false;

        let Some(app_id) = selected_app_id else {
            self.transition_ready = true;
            return;
        };

        let pending = Arc::clone(&self.pending);
        let paths = steam_paths.to_vec();
        let ctx_clone = ctx.clone();
        if let Ok(mut lock) = pending.lock() {
            *lock = None;
        }
        std::thread::spawn(move || {
            let cover_bytes = cover::load_cover_bytes(&paths, app_id);
            let logo_bytes = cover::load_logo_bytes(&paths, app_id);
            if let Ok(mut lock) = pending.lock() {
                *lock = Some(PendingBackgroundAssets {
                    app_id,
                    cover_bytes,
                    logo_bytes,
                });
            }
            ctx_clone.request_repaint();
        });
    }

    pub fn drain_pending(&mut self, selected_app_id: Option<u32>, ctx: &egui::Context) {
        let result = self.pending.lock().ok().and_then(|mut lock| lock.take());
        if let Some(assets) = result {
            if Some(assets.app_id) == selected_app_id {
                let mut loaded_any = false;

                self.cover = assets.cover_bytes.and_then(|bytes| {
                    cover::bytes_to_texture(ctx, &bytes, format!("cover_{}", assets.app_id)).map(
                        |texture| {
                            loaded_any = true;
                            (assets.app_id, texture)
                        },
                    )
                });

                self.logo = assets.logo_bytes.and_then(|bytes| {
                    cover::bytes_to_texture(ctx, &bytes, format!("logo_{}", assets.app_id)).map(
                        |texture| {
                            loaded_any = true;
                            (assets.app_id, texture)
                        },
                    )
                });

                if loaded_any || self.cover_prev.is_some() || self.logo_prev.is_some() {
                    self.fade = 0.0;
                }
                self.transition_ready = true;
            }
        }
    }

    pub fn tick_fade(&mut self, ctx: &egui::Context, dt: f32) {
        if self.transition_ready && self.fade < 1.0 {
            self.fade = (self.fade + dt * 3.0).min(1.0);
            ctx.request_repaint();
        }
    }

    pub fn cover(&self) -> &Option<(u32, egui::TextureHandle)> {
        &self.cover
    }

    pub fn cover_prev(&self) -> &Option<(u32, egui::TextureHandle)> {
        &self.cover_prev
    }

    pub fn logo(&self) -> &Option<(u32, egui::TextureHandle)> {
        &self.logo
    }

    pub fn logo_prev(&self) -> &Option<(u32, egui::TextureHandle)> {
        &self.logo_prev
    }

    pub fn fade(&self) -> f32 {
        self.fade
    }
}