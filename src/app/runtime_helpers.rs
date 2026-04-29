use std::time::{Duration, Instant};

use eframe::egui;

use crate::game::GameSource;
use crate::launch;

use super::{LauncherApp, CONTROLLER_IDLE_TIMEOUT, PASSIVE_REPAINT_INTERVAL};

fn scheduled_repaint_delay(
    has_focus: bool,
    has_input_activity: bool,
    idle_frame_rate_reduction_enabled: bool,
    running_game_in_foreground: bool,
) -> Option<Duration> {
    if !has_focus && running_game_in_foreground {
        return Some(PASSIVE_REPAINT_INTERVAL);
    }

    if has_focus && has_input_activity {
        return None;
    }

    if idle_frame_rate_reduction_enabled {
        Some(PASSIVE_REPAINT_INTERVAL)
    } else {
        None
    }
}

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
        let running_game_in_foreground = self
            .running_games
            .values()
            .any(launch::running_game_is_foreground);
        let delay = scheduled_repaint_delay(
            has_focus,
            has_input_activity,
            self.idle_frame_rate_reduction_enabled,
            running_game_in_foreground,
        );

        if let Some(delay) = delay {
            ctx.request_repaint_after(delay);
        } else {
            ctx.request_repaint();
        }
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

#[cfg(test)]
mod tests {
    use super::scheduled_repaint_delay;
    use crate::app::PASSIVE_REPAINT_INTERVAL;

    #[test]
    fn focused_input_activity_keeps_immediate_repaint() {
        assert_eq!(scheduled_repaint_delay(true, true, true, false), None);
        assert_eq!(scheduled_repaint_delay(true, true, false, false), None);
    }

    #[test]
    fn focused_idle_repaint_respects_setting() {
        assert_eq!(
            scheduled_repaint_delay(true, false, true, false),
            Some(PASSIVE_REPAINT_INTERVAL)
        );
        assert_eq!(scheduled_repaint_delay(true, false, false, false), None);
    }

    #[test]
    fn unfocused_repaint_also_respects_setting() {
        assert_eq!(
            scheduled_repaint_delay(false, false, true, false),
            Some(PASSIVE_REPAINT_INTERVAL)
        );
        assert_eq!(scheduled_repaint_delay(false, false, false, false), None);
    }

    #[test]
    fn unfocused_foreground_game_forces_low_frequency_repaint() {
        assert_eq!(
            scheduled_repaint_delay(false, false, false, true),
            Some(PASSIVE_REPAINT_INTERVAL)
        );
        assert_eq!(
            scheduled_repaint_delay(false, true, false, true),
            Some(PASSIVE_REPAINT_INTERVAL)
        );
    }
}
