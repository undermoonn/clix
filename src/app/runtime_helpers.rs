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
    pub(super) fn home_game_indices(&self) -> Vec<usize> {
        let indices = self.games.iter().enumerate().filter_map(|(index, game)| {
            (!self.hidden_home_game_keys.contains(&game.persistent_key())).then_some(index)
        });

        match self.home_game_limit {
            crate::config::HomeGameLimit::Limited(limit) => indices.take(limit).collect(),
            crate::config::HomeGameLimit::Unlimited => indices.collect(),
        }
    }

    pub(super) fn home_items_len(&self) -> usize {
        self.home_game_indices().len() + 1
    }

    pub(super) fn selected_home_game_index(&self) -> Option<usize> {
        self.home_game_indices().get(self.page.selected()).copied()
    }

    pub(super) fn selected_game_index(&self) -> Option<usize> {
        if self.page.show_game_library_page() {
            (!self.games.is_empty()).then_some(self.page.game_library_selected())
        } else {
            self.selected_home_game_index()
        }
    }

    pub(super) fn selected_game(&self) -> Option<&crate::game::Game> {
        self.selected_game_index()
            .and_then(|index| self.games.get(index))
    }

    pub(super) fn game_hidden_from_home(&self, game_index: usize) -> bool {
        self.games
            .get(game_index)
            .map(|game| self.hidden_home_game_keys.contains(&game.persistent_key()))
            .unwrap_or(false)
    }

    pub(super) fn set_game_home_hidden(
        &mut self,
        game_index: usize,
        hidden: bool,
        ctx: &egui::Context,
    ) {
        let Some(key) = self
            .games
            .get(game_index)
            .map(crate::game::Game::persistent_key)
        else {
            return;
        };

        let changed = if hidden {
            self.hidden_home_game_keys.insert(key)
        } else {
            self.hidden_home_game_keys.remove(&key)
        };
        if !changed {
            return;
        }

        crate::game_home_visibility::store_hidden_keys(&self.hidden_home_game_keys);
        self.page.clamp_home_selection(self.home_items_len());
        self.page.clamp_library_selection(self.games.len());
        self.refresh_selected_playtime(ctx);
        self.refresh_selected_install_size(ctx);
        self.refresh_selected_dlss(ctx);
        ctx.request_repaint();
    }

    pub(super) fn can_open_achievement_panel_for_selected(&self) -> bool {
        self.selected_game()
            .map(|game| matches!(game.source, GameSource::Steam))
            .unwrap_or(false)
    }

    pub(super) fn controller_idle_active(&self, now: Instant) -> bool {
        now.duration_since(self.last_controller_activity_at) >= CONTROLLER_IDLE_TIMEOUT
    }

    pub(super) fn refresh_selected_playtime(&mut self, ctx: &egui::Context) {
        let selected_game_index = self.selected_game_index();
        let selected_game = selected_game_index.and_then(|index| self.games.get(index));
        self.playtime
            .refresh_for_selected(selected_game, &self.steam_paths, ctx);
    }

    pub(super) fn refresh_selected_install_size(&mut self, ctx: &egui::Context) {
        let selected_game_index = self.selected_game_index();
        let selected_game = selected_game_index.and_then(|index| self.games.get(index));
        self.install_size.refresh_for_selected(selected_game, ctx);
    }

    pub(super) fn refresh_selected_dlss(&mut self, ctx: &egui::Context) {
        let selected_game_index = self.selected_game_index();
        let selected_game = selected_game_index.and_then(|index| self.games.get(index));
        self.dlss.refresh_for_selected(selected_game, ctx);
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
