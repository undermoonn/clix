use std::time::Instant;

use eframe::{self, egui};

use crate::system::{external_apps, startup};
use crate::{input, launch};

use super::LauncherApp;

pub(super) struct InputFlowOutcome {
    pub has_controller_activity: bool,
    pub launch_held: bool,
    pub selected_running: bool,
}

impl LauncherApp {
    pub(super) fn handle_input_flow(
        &mut self,
        process_input: bool,
        now: Instant,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
    ) -> InputFlowOutcome {
        let modal_open = self.page.show_achievement_panel()
            || self.page.show_power_menu()
            || self.page.show_settings_page()
            || self.page.home_top_button_selected();
        let input_frame = self.input.poll(process_input, modal_open);
        if let Some(theme) = input_frame.prompt_icon_theme {
            self.set_hint_icon_theme(theme, ctx);
        }

        let guide_held = input::home_held();
        let has_controller_activity = input_frame.has_controller_activity || guide_held;
        if has_controller_activity {
            self.last_controller_activity_at = now;
        }

        if self.pending_send_to_background {
            if input_frame.launch_held {
                ctx.request_repaint();
            } else {
                self.pending_send_to_background = false;
                self.input.clear_held();
                self.page.prepare_wake_animation();
                self.send_to_background_after_frame = true;
                ctx.request_repaint();
            }
        }

        let selected_running = self.running_games.contains_key(&self.page.selected());
        self.runtime
            .update_home_button(process_input, self.page.show_power_menu(), guide_held);

        let can_force_close =
            !self.page.show_achievement_panel() && !self.page.show_settings_page();
        let force_close_hold = self.runtime.update_force_close_hold(
            process_input && can_force_close,
            selected_running && can_force_close,
            input_frame.force_close_held && can_force_close,
            now,
        );
        if force_close_hold.trigger_force_close {
            if let Some(state) = self.running_games.get_mut(&self.page.selected()) {
                if launch::close_running_game(state) {
                    ctx.request_repaint();
                }
            }
        }
        if force_close_hold.should_repaint {
            ctx.request_repaint();
        }

        let launch_held = input_frame.launch_held;
        let actions = input_frame.actions;
        if !self.send_to_background_after_frame {
            for action in &actions {
                let previous_selected = self.page.selected();
                let previous_achievement_panel = self.page.show_achievement_panel();
                let previous_achievement_selected = self.page.achievement_selected();
                let achievement_len = self
                    .achievements
                    .detail_len_for_selected(self.games.get(previous_selected));
                let result = self.page.handle_action(
                    action,
                    self.games.len(),
                    self.can_open_achievement_panel_for_selected(),
                    achievement_len,
                );
                if result.open_power_menu {
                    self.refresh_power_menu_state();
                    ctx.request_repaint();
                }

                let achievement_selection_changed = previous_achievement_panel
                    && self.page.show_achievement_panel()
                    && self.page.achievement_selected() != previous_achievement_selected;
                let achievement_panel_closed =
                    previous_achievement_panel && !self.page.show_achievement_panel();
                let refresh_requested = result.refresh_achievements
                    && self
                        .achievements
                        .can_refresh_for_selected(self.games.get(self.page.selected()));

                if result.selected_changed
                    || result.open_achievement_panel
                    || refresh_requested
                    || achievement_selection_changed
                    || achievement_panel_closed
                {
                    self.achievements
                        .clear_revealed_hidden_for_selected(self.games.get(previous_selected));
                }

                if result.open_achievement_panel {
                    self.achievements.refresh_details_for_selected(
                        self.games.get(self.page.selected()),
                        &self.steam_paths,
                        self.language,
                        ctx,
                    );
                    self.refresh_selected_install_size(ctx);
                    self.refresh_selected_dlss(ctx);
                }
                if result.reveal_hidden_achievement
                    && self.achievements.reveal_hidden_description_for_selected(
                        self.games.get(self.page.selected()),
                        self.page.achievement_selected(),
                    )
                {
                    ctx.request_repaint();
                }
                if refresh_requested {
                    self.achievements.force_refresh_details_for_selected(
                        self.games.get(self.page.selected()),
                        &self.steam_paths,
                        self.language,
                        ctx,
                    );
                }
                if result.toggle_launch_on_startup {
                    if startup::set_enabled(!self.launch_on_startup_enabled) {
                        self.launch_on_startup_enabled = !self.launch_on_startup_enabled;
                    } else {
                        self.launch_on_startup_enabled = startup::is_enabled();
                    }
                    ctx.request_repaint();
                }
                if result.toggle_background_home_wake {
                    self.set_background_home_wake_mode(self.background_home_wake_mode.next(), ctx);
                }
                if result.toggle_controller_vibration_feedback {
                    self.set_controller_vibration_enabled(!self.controller_vibration_enabled, ctx);
                }
                if result.cycle_display_mode_setting {
                    self.cycle_display_mode_setting(ctx);
                }
                if result.cycle_language_setting {
                    self.cycle_language_setting(ctx);
                }
                if result.toggle_detect_steam_games {
                    let mut options = self.game_scan_options;
                    options.detect_steam_games = !options.detect_steam_games;
                    self.set_game_scan_options(options, ctx);
                }
                if result.toggle_detect_epic_games {
                    let mut options = self.game_scan_options;
                    options.detect_epic_games = !options.detect_epic_games;
                    self.set_game_scan_options(options, ctx);
                }
                if result.toggle_detect_xbox_games {
                    let mut options = self.game_scan_options;
                    options.detect_xbox_games = !options.detect_xbox_games;
                    self.set_game_scan_options(options, ctx);
                }
                if achievement_selection_changed {
                    self.input.pulse_selection_change();
                }
                if result.selected_changed {
                    self.input.pulse_selection_change();
                    self.refresh_selected_playtime(ctx);
                }
                if result.launch_selected && self.launch_press_feedback.is_none() {
                    let selected = self.page.selected();
                    if self.selected_launch_pending() {
                        self.set_launch_press_feedback(selected);
                        ctx.request_repaint();
                    } else if self.pending_launch_request.is_none() {
                        if self.should_queue_launch_feedback(selected) {
                            self.queue_launch_selected(ctx);
                        } else {
                            self.launch_selected(selected, ctx);
                        }
                    }
                }
                if let Some(kind) = result.launch_external_app {
                    let _ = external_apps::launch(kind, &self.power_menu_external_apps);
                }
                if result.send_app_to_background {
                    self.pending_send_to_background = true;
                    self.input.clear_held();
                    ctx.request_repaint();
                }
                if let Some(screen_settings_action) = result.screen_settings_action {
                    self.apply_screen_settings_action(screen_settings_action);
                }
                if let Some(power_action) = result.power_action {
                    self.apply_power_action(power_action, ctx, frame);
                }
                if result.close_frame {
                    self.close_root_viewport(ctx);
                }
            }
        }

        InputFlowOutcome {
            has_controller_activity,
            launch_held,
            selected_running,
        }
    }
}
