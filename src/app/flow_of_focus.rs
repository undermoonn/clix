use std::time::Instant;

use eframe::egui;

use crate::{input, launch};

use super::LauncherApp;

pub(super) struct FocusFlowOutcome {
    pub has_focus: bool,
    pub now: Instant,
    pub steam_launch_flow_active: bool,
    pub idle_dim_target: f32,
    pub process_input: bool,
}

impl LauncherApp {
    pub(super) fn handle_focus_flow(
        &mut self,
        ctx: &egui::Context,
    ) -> Option<FocusFlowOutcome> {
        let has_focus = ctx.input(|input| input.focused);

        if self.send_to_background_commit_pending {
            if has_focus {
                self.send_to_background_commit_pending = false;
                self.send_to_background_after_frame = false;
            } else {
                self.send_to_background_commit_pending = false;
                self.send_to_background_after_frame = false;
                if launch::send_current_app_to_background() {
                    return None;
                }
            }
        }

        let now = Instant::now();
        self.update_launch_notice(now, ctx);
        let steam_launch_flow_active = self.steam_launch_flow_active();
        let controller_idle_active = self.controller_idle_active(now);
        let idle_dim_target = if controller_idle_active && has_focus {
            1.0
        } else {
            0.0
        };

        if self.wake_focus_pending {
            self.wake_focus_pending = false;
            self.page.start_wake_animation(now);
            ctx.request_repaint();
        }

        let focus = self.runtime.update_focus(has_focus, now);

        if focus.did_gain_focus {
            self.pending_send_to_background = false;
            self.send_to_background_after_frame = false;
            self.send_to_background_commit_pending = false;
            self.apply_pending_promotion();
            self.page.start_wake_animation(now);
            self.refresh_selected_playtime(ctx);
            self.refresh_selected_install_size(ctx);
            ctx.request_repaint();
        }

        if focus.did_lose_focus && !steam_launch_flow_active {
            self.page.prepare_wake_animation();
            ctx.request_repaint();
        }

        let wake_requested = input::take_wake_request();
        if wake_requested && self.background_home_wake_enabled {
            if let Some(state) = self.running_games.get(&self.page.selected()) {
                let _ = launch::minimize_running_game(state);
            }
            self.page.prepare_wake_animation();
            self.wake_focus_pending = true;
            self.runtime.suppress_home_hold_until_release();
            ctx.request_repaint();
        }

        if focus.should_clear_input {
            self.input.clear_held();
        }
        self.input.tick();

        let process_input = has_focus && !focus.in_cooldown && !self.pending_send_to_background;

        Some(FocusFlowOutcome {
            has_focus,
            now,
            steam_launch_flow_active,
            idle_dim_target,
            process_input,
        })
    }
}