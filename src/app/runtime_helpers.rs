use std::time::{Duration, Instant};

use eframe::egui;

use crate::game::GameSource;

use super::{LauncherApp, CONTROLLER_IDLE_TIMEOUT, PASSIVE_REPAINT_INTERVAL};

impl LauncherApp {
    pub(super) fn can_open_achievement_panel_for_selected(&self) -> bool {
        self.games
            .get(self.page.selected())
            .map(|game| matches!(game.source, GameSource::Steam))
            .unwrap_or(false)
    }

    pub(super) fn controller_idle_active(&self, now: Instant) -> bool {
        now.duration_since(self.last_controller_activity_at) >= CONTROLLER_IDLE_TIMEOUT
    }

    pub(super) fn refresh_selected_playtime(&mut self, ctx: &egui::Context) {
        self.playtime.refresh_for_selected(
            self.games.get(self.page.selected()),
            &self.steam_paths,
            ctx,
        );
    }

    pub(super) fn refresh_selected_install_size(&mut self, ctx: &egui::Context) {
        self.install_size
            .refresh_for_selected(self.games.get(self.page.selected()), ctx);
    }

    pub(super) fn refresh_selected_dlss(&mut self, ctx: &egui::Context) {
        self.dlss
            .refresh_for_selected(self.games.get(self.page.selected()), ctx);
    }

    pub(super) fn close_root_viewport(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    pub(super) fn schedule_input_repaint(
        &self,
        ctx: &egui::Context,
        has_focus: bool,
        has_input_activity: bool,
    ) {
        if !has_focus {
            return;
        }

        if has_input_activity {
            ctx.request_repaint();
            return;
        }

        if self.idle_frame_rate_reduction_enabled {
            ctx.request_repaint_after(PASSIVE_REPAINT_INTERVAL);
            return;
        }

        ctx.request_repaint();
    }

    pub(super) fn update_cursor_visibility(&mut self, ctx: &egui::Context) {
        const HIDE_AFTER: Duration = Duration::from_secs(1);

        let now = Instant::now();
        let activity = ctx.input(|i| {
            let mut active = false;
            let pos = i.pointer.latest_pos();
            if let Some(pos) = pos {
                if self.last_pointer_pos.map_or(true, |prev| prev != pos) {
                    active = true;
                }
                self.last_pointer_pos = Some(pos);
            }
            if !active {
                for event in &i.events {
                    match event {
                        egui::Event::PointerMoved(_)
                        | egui::Event::PointerButton { .. }
                        | egui::Event::MouseWheel { .. } => {
                            active = true;
                            break;
                        }
                        _ => {}
                    }
                }
            }
            active
        });

        if activity {
            self.last_pointer_activity = Some(now);
        }

        let visible = match self.last_pointer_activity {
            Some(t) => now.duration_since(t) < HIDE_AFTER,
            None => false,
        };

        if !visible {
            ctx.output_mut(|output| output.cursor_icon = egui::CursorIcon::None);
        } else if let Some(t) = self.last_pointer_activity {
            let elapsed = now.duration_since(t);
            if elapsed < HIDE_AFTER {
                ctx.request_repaint_after(HIDE_AFTER - elapsed);
            }
        }
    }
}
