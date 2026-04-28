use std::time::Instant;

use eframe::egui;

use crate::animation;
use crate::game::GameSource;
use crate::steam;

use super::flow_of_launch::LaunchNoticeKind;
use super::ui_assets::achievement_panel_scope_steam_app_id;
use super::{LauncherApp, IDLE_DIM_ANIMATION_SPEED, LAUNCH_PRESS_FEEDBACK_DURATION};

pub(super) struct ViewRenderState {
    pub achievement_icon_scope: Option<u32>,
    pub idle_dim_t: f32,
    pub launch_feedback: Option<(usize, f32)>,
    pub launch_notice: Option<(usize, String, f32, egui::Color32, bool)>,
    pub steam_update_notice: Option<(usize, String, egui::Color32)>,
    pub launching_index: Option<usize>,
    pub render_wake_anim: f32,
    pub running_indices: Vec<usize>,
}

impl LauncherApp {
    pub(super) fn prepare_view_render_state(
        &mut self,
        ctx: &egui::Context,
        has_focus: bool,
        steam_launch_flow_active: bool,
        idle_dim_target: f32,
    ) -> ViewRenderState {
        let selected_steam_app_id = self
            .games
            .get(self.page.selected())
            .and_then(|game| game.steam_app_id);
        let achievement_icon_scope = achievement_panel_scope_steam_app_id(
            selected_steam_app_id,
            self.page.show_achievement_panel(),
            self.page.achievement_panel_anim(),
        );
        let updated_global_percentages = steam::take_updated_global_achievement_percentages();

        self.achievements.sync_summary_scope(selected_steam_app_id);
        self.achievements.sync_detail_scope(achievement_icon_scope);
        self.achievements.sync_icon_scope(achievement_icon_scope);
        self.achievements.refresh_after_global_percentage_update(
            self.games.get(self.page.selected()),
            &self.steam_paths,
            self.language,
            self.page.show_achievement_panel(),
            &updated_global_percentages,
            ctx,
        );
        if self.artwork.tick_selection(
            self.page.selected(),
            selected_steam_app_id,
            &self.steam_paths,
            ctx,
        ) {
            self.achievements.refresh_summary_for_selected(
                self.games.get(self.page.selected()),
                &self.steam_paths,
                self.language,
                ctx,
            );
        }
        self.artwork.drain_pending(selected_steam_app_id, ctx);

        let now = Instant::now();
        self.idle_dim_anim
            .animate_to(idle_dim_target, IDLE_DIM_ANIMATION_SPEED, now, 0.001);
        if self.idle_dim_anim.update(now, 0.001) {
            ctx.request_repaint();
        }
        self.artwork.tick_fade(ctx, now);
        self.page.tick_animations(ctx, now);
        self.achievements.animate_reveals(ctx, now);

        self.game_icons
            .ensure_loaded(ctx, &self.steam_paths, &self.games, self.page.selected());
        self.external_app_icons
            .ensure_loaded(ctx, &self.power_menu_external_apps);

        let launch_feedback = self.current_launch_feedback(now);
        let launch_notice = self.current_launch_notice(now);
        let steam_update_notice =
            self.current_steam_update_notice(selected_steam_app_id, launch_notice.is_some());
        let launching_index = self.launch_state.as_ref().map(|state| state.game_index);
        let render_wake_anim =
            if (has_focus || steam_launch_flow_active) && !self.send_to_background_after_frame {
                self.page.wake_anim()
            } else {
                0.0
            };

        ViewRenderState {
            achievement_icon_scope,
            idle_dim_t: self.idle_dim_anim.value(),
            launch_feedback,
            launch_notice,
            steam_update_notice,
            launching_index,
            render_wake_anim,
            running_indices: self.running_games.keys().copied().collect(),
        }
    }

    fn current_launch_feedback(&mut self, now: Instant) -> Option<(usize, f32)> {
        let press_feedback = self.launch_press_feedback.as_ref().and_then(|feedback| {
            let elapsed =
                animation::scale_seconds(now.duration_since(feedback.started_at).as_secs_f32());
            (elapsed <= LAUNCH_PRESS_FEEDBACK_DURATION.as_secs_f32())
                .then_some((feedback.game_index, elapsed))
        });
        if press_feedback.is_none() {
            self.launch_press_feedback = None;
        }
        press_feedback
    }

    fn current_launch_notice(
        &self,
        now: Instant,
    ) -> Option<(usize, String, f32, egui::Color32, bool)> {
        self.launch_notice.as_ref().and_then(|notice| {
            self.games
                .get(self.page.selected())
                .filter(|game| matches!(game.source, GameSource::Steam))
                .map(|_| {
                    (
                        self.page.selected(),
                        self.launch_notice_text(notice),
                        Self::launch_notice_overlay_t(notice, now),
                        Self::launch_notice_color(notice),
                        notice.kind == LaunchNoticeKind::PromptStartSteam,
                    )
                })
        })
    }

    fn current_steam_update_notice(
        &self,
        selected_steam_app_id: Option<u32>,
        launch_notice_visible: bool,
    ) -> Option<(usize, String, egui::Color32)> {
        if launch_notice_visible {
            return None;
        }

        let needs_update = self
            .steam_update
            .status_for_steam_app_id(selected_steam_app_id)
            .is_some_and(|progress| progress.needs_update());
        if !needs_update {
            return None;
        }

        Some((
            self.page.selected(),
            self.steam_update_overlay_text(selected_steam_app_id),
            Self::steam_update_overlay_color(),
        ))
    }
}
