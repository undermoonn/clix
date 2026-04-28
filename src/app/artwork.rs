use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::{animation, assets::cover};

const ARTWORK_FADE_SECONDS: f32 = 1.0 / 3.0;

struct PendingBackgroundAssets {
    steam_app_id: u32,
    cover_bytes: Option<Vec<u8>>,
    logo_bytes: Option<Vec<u8>>,
}

pub struct ArtworkState {
    cover: Option<(u32, egui::TextureHandle)>,
    cover_prev: Option<(u32, egui::TextureHandle)>,
    logo: Option<(u32, egui::TextureHandle)>,
    logo_prev: Option<(u32, egui::TextureHandle)>,
    vignette: Option<egui::TextureHandle>,
    fade: f32,
    fade_started_at: Option<Instant>,
    transition_ready: bool,
    loaded_for: Option<usize>,
    debounce_until: Option<Instant>,
    debounce_for: Option<usize>,
    pending: Arc<Mutex<Option<PendingBackgroundAssets>>>,
}

impl ArtworkState {
    pub fn new(ctx: &egui::Context) -> Self {
        let vignette = cover::bytes_to_texture_limited(
            ctx,
            include_bytes!(concat!(env!("OUT_DIR"), "/top-right-vignette.png")),
            "top_right_vignette".to_owned(),
            None,
        );

        Self {
            cover: None,
            cover_prev: None,
            logo: None,
            logo_prev: None,
            vignette,
            fade: 1.0,
            fade_started_at: None,
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
        selected_steam_app_id: Option<u32>,
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
                    self.refresh(selected_steam_app_id, steam_paths, ctx);
                    return true;
                }
            } else {
                ctx.request_repaint();
            }
        }

        false
    }

    fn refresh(
        &mut self,
        selected_steam_app_id: Option<u32>,
        steam_paths: &[PathBuf],
        ctx: &egui::Context,
    ) {
        self.cover_prev = self.cover.take();
        self.logo_prev = self.logo.take();
        self.fade = 0.0;
        self.fade_started_at = None;
        self.transition_ready = false;

        let Some(steam_app_id) = selected_steam_app_id else {
            self.transition_ready = true;
            if self.cover_prev.is_some() || self.logo_prev.is_some() {
                self.fade_started_at = Some(Instant::now());
            } else {
                self.fade = 1.0;
            }
            return;
        };

        let pending = Arc::clone(&self.pending);
        let paths = steam_paths.to_vec();
        let ctx_clone = ctx.clone();
        if let Ok(mut lock) = pending.lock() {
            *lock = None;
        }
        std::thread::spawn(move || {
            let cover_bytes = cover::load_cover_bytes(&paths, steam_app_id);
            let logo_bytes = cover::load_logo_bytes(&paths, steam_app_id);
            if let Ok(mut lock) = pending.lock() {
                *lock = Some(PendingBackgroundAssets {
                    steam_app_id,
                    cover_bytes,
                    logo_bytes,
                });
            }
            ctx_clone.request_repaint();
        });
    }

    pub fn drain_pending(&mut self, selected_steam_app_id: Option<u32>, ctx: &egui::Context) {
        let result = self.pending.lock().ok().and_then(|mut lock| lock.take());
        if let Some(assets) = result {
            if Some(assets.steam_app_id) == selected_steam_app_id {
                let mut loaded_any = false;

                self.cover = assets.cover_bytes.and_then(|bytes| {
                    cover::bytes_to_cover_texture(
                        ctx,
                        &bytes,
                        format!("cover_{}", assets.steam_app_id),
                    )
                    .map(|texture| {
                        loaded_any = true;
                        (assets.steam_app_id, texture)
                    })
                });

                self.logo = assets.logo_bytes.and_then(|bytes| {
                    cover::bytes_to_logo_texture(
                        ctx,
                        &bytes,
                        format!("logo_{}", assets.steam_app_id),
                    )
                    .map(|texture| {
                        loaded_any = true;
                        (assets.steam_app_id, texture)
                    })
                });

                if loaded_any || self.cover_prev.is_some() || self.logo_prev.is_some() {
                    self.fade = 0.0;
                    self.fade_started_at = Some(Instant::now());
                } else {
                    self.fade = 1.0;
                    self.fade_started_at = None;
                }
                self.transition_ready = true;
            }
        }
    }

    pub fn tick_fade(&mut self, ctx: &egui::Context, now: Instant) {
        if self.transition_ready && self.fade < 1.0 {
            let started_at = self.fade_started_at.get_or_insert(now);
            self.fade = animation::linear_progress(*started_at, now, ARTWORK_FADE_SECONDS);
            if self.fade >= 0.999 {
                self.cover_prev = None;
                self.logo_prev = None;
                self.fade_started_at = None;
                self.fade = 1.0;
            } else {
                ctx.request_repaint();
            }
        } else if self.transition_ready && (self.cover_prev.is_some() || self.logo_prev.is_some()) {
            self.cover_prev = None;
            self.logo_prev = None;
            self.fade_started_at = None;
            self.fade = 1.0;
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

    pub fn vignette(&self) -> Option<&egui::TextureHandle> {
        self.vignette.as_ref()
    }

    pub fn fade(&self) -> f32 {
        self.fade
    }
}
