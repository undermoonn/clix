use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::game::{self, GameSource};
use crate::{game_last_played, launch};

use super::{
    LauncherApp, LAUNCH_PRESS_FEEDBACK_DURATION, PASSIVE_REPAINT_INTERVAL,
    STEAM_NOTICE_ANIMATION_DURATION, STEAM_PROMPT_VISIBLE_DURATION, STEAM_READY_VISIBLE_DURATION,
    STEAM_STATUS_POLL_INTERVAL,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LaunchNoticeKind {
    PromptStartSteam,
    SteamStarting,
    SteamStarted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LaunchNoticeStage {
    Entering,
    Visible,
    Exiting,
}

pub(super) struct LaunchNotice {
    pub(super) game_index: usize,
    pub(super) kind: LaunchNoticeKind,
    pub(super) stage: LaunchNoticeStage,
    pub(super) stage_started_at: Instant,
    pub(super) last_state_check_at: Option<Instant>,
    pub(super) queued_kind: Option<LaunchNoticeKind>,
    pub(super) launch_game_after_exit: bool,
}

pub(super) struct LaunchPressFeedback {
    pub(super) game_index: usize,
    pub(super) started_at: Instant,
}

#[derive(Clone, Copy)]
pub(super) struct PendingLaunchRequest {
    pub(super) game_index: usize,
}

pub(super) struct SteamClientStateMonitor {
    pending: Arc<Mutex<Option<launch::SteamClientState>>>,
    in_flight: bool,
    cached: launch::SteamClientState,
}

impl SteamClientStateMonitor {
    pub(super) fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(None)),
            in_flight: false,
            cached: launch::SteamClientState::NotRunning,
        }
    }

    fn cached(&self) -> launch::SteamClientState {
        self.cached
    }

    fn drain(&mut self) {
        let Ok(mut lock) = self.pending.lock() else {
            return;
        };
        let Some(state) = lock.take() else {
            return;
        };

        self.in_flight = false;
        self.cached = state;
    }

    fn poll_if_due(
        &mut self,
        now: Instant,
        last_polled_at: &mut Option<Instant>,
        ctx: &egui::Context,
    ) {
        if self.in_flight {
            return;
        }

        if last_polled_at.is_some_and(|last| now.duration_since(last) < STEAM_STATUS_POLL_INTERVAL)
        {
            return;
        }

        *last_polled_at = Some(now);
        self.in_flight = true;

        let pending = Arc::clone(&self.pending);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let state = launch::steam_client_state();
            if let Ok(mut lock) = pending.lock() {
                *lock = Some(state);
            }
            ctx.request_repaint();
        });
    }

    fn reset(&mut self, cached: launch::SteamClientState) {
        self.in_flight = false;
        self.cached = cached;
        if let Ok(mut lock) = self.pending.lock() {
            *lock = None;
        }
    }
}

impl LauncherApp {
    pub(super) fn selected_launch_pending(&self) -> bool {
        self.launch_state
            .as_ref()
            .map(|state| state.game_index == self.page.selected())
            .unwrap_or(false)
    }

    fn game_is_steam(&self, game_index: usize) -> bool {
        self.games
            .get(game_index)
            .map(|game| matches!(game.source, GameSource::Steam))
            .unwrap_or(false)
    }

    pub(super) fn should_queue_launch_feedback(&self, game_index: usize) -> bool {
        if !self.game_is_steam(game_index) {
            return true;
        }

        let Some(notice) = self.launch_notice.as_ref() else {
            return true;
        };

        notice.game_index == game_index
    }

    pub(super) fn steam_launch_flow_active(&self) -> bool {
        self.launch_notice.is_some()
            || self.launch_press_feedback.is_some()
            || self.launch_state.is_some()
    }

    pub(super) fn set_launch_press_feedback(&mut self, game_index: usize) {
        self.launch_press_feedback = Some(LaunchPressFeedback {
            game_index,
            started_at: Instant::now(),
        });
    }

    pub(super) fn queue_launch_selected(&mut self, ctx: &egui::Context) {
        let game_index = self.page.selected();
        self.pending_launch_request = Some(PendingLaunchRequest { game_index });
        self.set_launch_press_feedback(game_index);
        ctx.request_repaint();
    }

    pub(super) fn drain_pending_launch_request(&mut self, now: Instant, ctx: &egui::Context) {
        let Some(request) = self.pending_launch_request else {
            return;
        };

        if let Some(feedback) = self.launch_press_feedback.as_ref() {
            if feedback.game_index == request.game_index {
                let elapsed = now.duration_since(feedback.started_at);
                if elapsed < LAUNCH_PRESS_FEEDBACK_DURATION {
                    ctx.request_repaint_after(LAUNCH_PRESS_FEEDBACK_DURATION - elapsed);
                    return;
                }
            }
        }

        self.pending_launch_request = None;
        self.launch_selected(request.game_index, ctx);
    }

    fn show_launch_notice(
        &mut self,
        game_index: usize,
        kind: LaunchNoticeKind,
        ctx: &egui::Context,
    ) {
        let cached_state = match kind {
            LaunchNoticeKind::PromptStartSteam => launch::SteamClientState::NotRunning,
            LaunchNoticeKind::SteamStarting => launch::SteamClientState::Loading,
            LaunchNoticeKind::SteamStarted => launch::SteamClientState::Ready,
        };
        self.steam_client_state_monitor.reset(cached_state);
        self.launch_notice = Some(LaunchNotice {
            game_index,
            kind,
            stage: LaunchNoticeStage::Entering,
            stage_started_at: Instant::now(),
            last_state_check_at: None,
            queued_kind: None,
            launch_game_after_exit: false,
        });
        ctx.request_repaint();
    }

    fn launch_notice_visible_duration(kind: LaunchNoticeKind) -> Option<Duration> {
        match kind {
            LaunchNoticeKind::PromptStartSteam => Some(STEAM_PROMPT_VISIBLE_DURATION),
            LaunchNoticeKind::SteamStarting => None,
            LaunchNoticeKind::SteamStarted => Some(STEAM_READY_VISIBLE_DURATION),
        }
    }

    pub(super) fn launch_notice_overlay_t(notice: &LaunchNotice, now: Instant) -> f32 {
        let stage_elapsed = now.saturating_duration_since(notice.stage_started_at);
        let duration = STEAM_NOTICE_ANIMATION_DURATION.as_secs_f32();
        let t = (stage_elapsed.as_secs_f32() / duration).clamp(0.0, 1.0);
        let smootherstep = |value: f32| {
            let value = value.clamp(0.0, 1.0);
            value * value * value * (value * (value * 6.0 - 15.0) + 10.0)
        };

        match notice.stage {
            LaunchNoticeStage::Entering => smootherstep(t),
            LaunchNoticeStage::Visible => 1.0,
            LaunchNoticeStage::Exiting => 1.0 - smootherstep(t),
        }
    }

    pub(super) fn launch_notice_text(&self, notice: &LaunchNotice) -> String {
        match notice.kind {
            LaunchNoticeKind::PromptStartSteam => {
                self.language.steam_start_action_text().to_string()
            }
            LaunchNoticeKind::SteamStarting => self.language.steam_starting_text().to_string(),
            LaunchNoticeKind::SteamStarted => self.language.steam_started_text().to_string(),
        }
    }

    pub(super) fn launch_notice_color(notice: &LaunchNotice) -> egui::Color32 {
        match notice.kind {
            LaunchNoticeKind::PromptStartSteam => {
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 168)
            }
            LaunchNoticeKind::SteamStarting => {
                egui::Color32::from_rgba_unmultiplied(154, 120, 18, 196)
            }
            LaunchNoticeKind::SteamStarted => {
                egui::Color32::from_rgba_unmultiplied(34, 122, 72, 196)
            }
        }
    }

    fn launch_state_steam_app_id(&self) -> Option<u32> {
        self.launch_state
            .as_ref()
            .and_then(|state| self.games.get(state.game_index))
            .filter(|game| matches!(game.source, GameSource::Steam))
            .and_then(|game| game.steam_app_id)
    }

    pub(super) fn sync_steam_update_state(
        &mut self,
        selected_steam_app_id: Option<u32>,
        now: Instant,
        ctx: &egui::Context,
    ) {
        self.steam_update.drain_results();

        let launch_state_steam_app_id = self.launch_state_steam_app_id();
        let requested_update_steam_app_id = self.steam_update_launch_requested_app_id;

        self.steam_update.refresh_for_steam_app_id(
            selected_steam_app_id,
            &self.steam_paths,
            now,
            ctx,
        );
        if launch_state_steam_app_id != selected_steam_app_id {
            self.steam_update.refresh_for_steam_app_id(
                launch_state_steam_app_id,
                &self.steam_paths,
                now,
                ctx,
            );
        }
        if requested_update_steam_app_id != selected_steam_app_id
            && requested_update_steam_app_id != launch_state_steam_app_id
        {
            self.steam_update.refresh_for_steam_app_id(
                requested_update_steam_app_id,
                &self.steam_paths,
                now,
                ctx,
            );
        }

        if requested_update_steam_app_id.is_none()
            && self
                .steam_update
                .status_for_steam_app_id(launch_state_steam_app_id)
                .is_some_and(|progress| progress.needs_update())
        {
            self.steam_update_launch_requested_app_id = launch_state_steam_app_id;
        }

        if let Some(steam_app_id) = self.steam_update_launch_requested_app_id {
            let update_still_pending = self
                .steam_update
                .status_for_steam_app_id(Some(steam_app_id))
                .is_some_and(|progress| progress.needs_update());

            if !update_still_pending {
                let should_restart_launch_timeout = launch_state_steam_app_id == Some(steam_app_id);
                self.steam_update_launch_requested_app_id = None;
                if should_restart_launch_timeout {
                    if let Some(state) = self.launch_state.as_mut() {
                        launch::restart_launch_timeout(state);
                    }
                }
            }
        }
    }

    fn should_show_steam_updating(&self, steam_app_id: Option<u32>) -> bool {
        steam_app_id.is_some() && self.steam_update_launch_requested_app_id == steam_app_id
    }

    pub(super) fn steam_update_overlay_text(&self, steam_app_id: Option<u32>) -> String {
        if self.should_show_steam_updating(steam_app_id) {
            self.language.steam_updating_text().to_owned()
        } else {
            self.language.steam_launch_after_update_text().to_owned()
        }
    }

    pub(super) fn steam_update_overlay_color() -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(28, 90, 140, 208)
    }

    fn mark_steam_update_launch_requested(&mut self, game_index: usize) {
        let Some(steam_app_id) = self
            .games
            .get(game_index)
            .and_then(|game| game.steam_app_id)
        else {
            return;
        };

        if self
            .steam_update
            .status_for_steam_app_id(Some(steam_app_id))
            .is_some_and(|progress| progress.needs_update())
        {
            self.steam_update_launch_requested_app_id = Some(steam_app_id);
        }
    }

    fn launch_state_is_steam_update_pending(&self) -> bool {
        self.launch_state_steam_app_id()
            .is_some_and(|steam_app_id| {
                Some(steam_app_id) == self.steam_update_launch_requested_app_id
            })
    }

    fn start_selected_game_launch(&mut self, selected: usize, ctx: &egui::Context) {
        if let Some(game) = self.games.get(selected) {
            match launch::begin_launch(selected, game, &self.steam_paths) {
                launch::LaunchAttemptResult::Started(state) => {
                    self.mark_steam_update_launch_requested(selected);
                    self.launch_state = Some(state);
                    self.launch_notice = None;
                    self.launch_press_feedback = None;
                    self.schedule_promotion(selected);
                }
                launch::LaunchAttemptResult::Blocked(reason) => {
                    self.launch_state = None;
                    match reason {
                        launch::LaunchBlockedReason::SteamClientNotRunning => {
                            self.show_launch_notice(
                                selected,
                                LaunchNoticeKind::PromptStartSteam,
                                ctx,
                            );
                        }
                        launch::LaunchBlockedReason::SteamClientLoading => {
                            self.show_launch_notice(selected, LaunchNoticeKind::SteamStarting, ctx);
                        }
                    }
                }
                launch::LaunchAttemptResult::Failed => {
                    self.launch_state = None;
                    if self.steam_update_launch_requested_app_id == game.steam_app_id {
                        self.steam_update_launch_requested_app_id = None;
                    }
                }
            }
        }
    }

    fn try_advance_launch_notice(&mut self, selected: usize, ctx: &egui::Context) -> bool {
        if !self.game_is_steam(selected) {
            return false;
        }

        let Some(notice) = self.launch_notice.as_mut() else {
            return false;
        };

        notice.game_index = selected;

        if notice.kind != LaunchNoticeKind::PromptStartSteam {
            return true;
        }

        if notice.stage == LaunchNoticeStage::Exiting {
            return true;
        }

        if !launch::start_steam_client(&self.steam_paths) {
            return true;
        }

        notice.stage = LaunchNoticeStage::Exiting;
        notice.stage_started_at = Instant::now();
        notice.queued_kind = Some(LaunchNoticeKind::SteamStarting);
        notice.launch_game_after_exit = false;
        ctx.request_repaint();
        true
    }

    pub(super) fn update_launch_notice(&mut self, now: Instant, ctx: &egui::Context) {
        let mut next_kind: Option<(usize, LaunchNoticeKind)> = None;
        let mut clear_notice = false;
        let mut launch_game_after_clear: Option<usize> = None;

        self.steam_client_state_monitor.drain();

        if let Some(notice) = self.launch_notice.as_mut() {
            if notice.kind == LaunchNoticeKind::SteamStarting
                && notice.stage != LaunchNoticeStage::Exiting
            {
                self.steam_client_state_monitor.poll_if_due(
                    now,
                    &mut notice.last_state_check_at,
                    ctx,
                );
                if self.steam_client_state_monitor.cached() == launch::SteamClientState::Ready {
                    notice.stage = LaunchNoticeStage::Exiting;
                    notice.stage_started_at = now;
                    notice.queued_kind = Some(LaunchNoticeKind::SteamStarted);
                    notice.launch_game_after_exit = false;
                }
            }

            let stage_elapsed = now.saturating_duration_since(notice.stage_started_at);

            match notice.stage {
                LaunchNoticeStage::Entering => {
                    if stage_elapsed >= STEAM_NOTICE_ANIMATION_DURATION {
                        notice.stage = LaunchNoticeStage::Visible;
                        notice.stage_started_at = now;
                    }
                    ctx.request_repaint();
                }
                LaunchNoticeStage::Visible => {
                    if let Some(duration) = Self::launch_notice_visible_duration(notice.kind) {
                        if stage_elapsed >= duration {
                            notice.stage = LaunchNoticeStage::Exiting;
                            notice.stage_started_at = now;
                            notice.launch_game_after_exit =
                                notice.kind == LaunchNoticeKind::SteamStarted;
                            ctx.request_repaint();
                        } else {
                            ctx.request_repaint_after(duration - stage_elapsed);
                        }
                    } else {
                        ctx.request_repaint_after(STEAM_STATUS_POLL_INTERVAL);
                    }
                }
                LaunchNoticeStage::Exiting => {
                    if stage_elapsed >= STEAM_NOTICE_ANIMATION_DURATION {
                        if let Some(kind) = notice.queued_kind.take() {
                            next_kind = Some((notice.game_index, kind));
                        } else {
                            if notice.launch_game_after_exit {
                                launch_game_after_clear = Some(notice.game_index);
                            }
                            clear_notice = true;
                        }
                    } else {
                        ctx.request_repaint();
                    }
                }
            }
        }

        if clear_notice {
            self.launch_notice = None;
        }

        if let Some((game_index, kind)) = next_kind {
            self.show_launch_notice(game_index, kind, ctx);
        }

        if let Some(game_index) = launch_game_after_clear {
            self.start_selected_game_launch(game_index, ctx);
        }
    }

    pub(super) fn launch_selected(&mut self, selected: usize, ctx: &egui::Context) {
        if self.try_advance_launch_notice(selected, ctx) {
            return;
        }

        if let Some(state) = self.running_games.get(&selected) {
            if let Some(refocus_state) = launch::begin_refocus_transition(selected, state) {
                self.launch_state = Some(refocus_state);
                self.launch_notice = None;
                self.schedule_promotion(selected);
            } else if launch::refocus_running_game(state) {
                self.launch_notice = None;
                let _ = self.promote_game_to_front(selected);
            }
            return;
        }

        self.start_selected_game_launch(selected, ctx);
    }

    fn schedule_promotion(&mut self, game_index: usize) {
        let Some(game_key) = self.games.get(game_index).map(|game| game.persistent_key()) else {
            return;
        };
        self.pending_promotion = Some(game_key);
    }

    pub(super) fn apply_pending_promotion(&mut self) {
        let Some(game_key) = self.pending_promotion.take() else {
            return;
        };
        let Some(index) = self
            .games
            .iter()
            .position(|game| game.persistent_key() == game_key)
        else {
            return;
        };
        let _ = self.promote_game_to_front(index);
    }

    pub(super) fn tick_launch_progress(&mut self, ctx: &egui::Context, launch_held: bool) {
        let low_frequency_poll = self.launch_state_is_steam_update_pending();

        if let Some(state) = self.launch_state.as_mut() {
            if low_frequency_poll {
                ctx.request_repaint_after(PASSIVE_REPAINT_INTERVAL);
            } else {
                ctx.request_repaint();
            }
            match launch::tick_launch_progress(state, launch_held) {
                launch::LaunchTickResult::Pending => {}
                launch::LaunchTickResult::Ready(running_game) => {
                    self.running_games
                        .insert(running_game.game_index, running_game);
                    self.launch_state = None;
                }
                launch::LaunchTickResult::TimedOut => {
                    if !low_frequency_poll {
                        self.launch_state = None;
                    }
                }
            }
        }
    }

    pub(super) fn tick_running_game_state(&mut self) {
        self.running_games
            .retain(|_, state| launch::refresh_running_game(state));
    }

    fn promote_game_to_front(&mut self, game_index: usize) -> Option<usize> {
        let game_key = self.games.get(game_index)?.persistent_key();
        let now = game_last_played::now_unix_secs();
        let old_order = self
            .games
            .iter()
            .map(|game| game.persistent_key())
            .collect::<Vec<_>>();

        self.games.get_mut(game_index)?.last_played = now;
        game_last_played::record_for_game(&game_key, now);
        game::sort_games_by_last_played(&mut self.games);
        self.remap_runtime_indices(&old_order);

        let new_index = self
            .games
            .iter()
            .position(|game| game.persistent_key() == game_key)?;
        if game_index == self.page.selected() {
            self.page.relocate_selection(new_index);
        } else {
            self.page.force_select(new_index);
        }
        Some(new_index)
    }

    fn remap_runtime_indices(&mut self, old_order: &[String]) {
        let new_positions = self
            .games
            .iter()
            .enumerate()
            .map(|(index, game)| (game.persistent_key(), index))
            .collect::<HashMap<_, _>>();

        let mut remapped_running_games = HashMap::with_capacity(self.running_games.len());
        for (old_index, mut state) in self.running_games.drain() {
            let Some(game_key) = old_order.get(old_index) else {
                continue;
            };
            let Some(&new_index) = new_positions.get(game_key) else {
                continue;
            };

            state.game_index = new_index;
            remapped_running_games.insert(new_index, state);
        }
        self.running_games = remapped_running_games;

        if let Some(state) = self.launch_state.as_mut() {
            let Some(game_key) = old_order.get(state.game_index) else {
                return;
            };
            if let Some(&new_index) = new_positions.get(game_key) {
                state.game_index = new_index;
            }
        }

        if let Some(notice) = self.launch_notice.as_mut() {
            let Some(game_key) = old_order.get(notice.game_index) else {
                self.launch_notice = None;
                return;
            };
            if let Some(&new_index) = new_positions.get(game_key) {
                notice.game_index = new_index;
            } else {
                self.launch_notice = None;
            }
        }

        if let Some(feedback) = self.launch_press_feedback.as_mut() {
            let Some(game_key) = old_order.get(feedback.game_index) else {
                self.launch_press_feedback = None;
                return;
            };
            if let Some(&new_index) = new_positions.get(game_key) {
                feedback.game_index = new_index;
            } else {
                self.launch_press_feedback = None;
            }
        }

        if let Some(request) = self.pending_launch_request.as_mut() {
            let Some(game_key) = old_order.get(request.game_index) else {
                self.pending_launch_request = None;
                return;
            };
            if let Some(&new_index) = new_positions.get(game_key) {
                request.game_index = new_index;
            } else {
                self.pending_launch_request = None;
            }
        }
    }
}
