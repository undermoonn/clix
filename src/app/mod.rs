mod achievements;
mod artwork;
mod dlss_state;
mod external_app_icons;
mod game_icons;
mod install_size;
mod playtime;
mod state;
mod steam_update;

use eframe::egui;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::animation;
use crate::config::PromptIconTheme;
use crate::game::{self, Game, GameSource};
use crate::game_last_played;
use crate::i18n::{AppLanguage, AppLanguageSetting};
use crate::input::{self, InputController};
use crate::launch::{self, LaunchState};
use crate::system::{
    display_mode::{self, DisplayModeSetting, DisplayScaleOptions, ResolutionOptions},
    external_apps::{self, ExternalApp},
    power, startup,
};
use crate::steam;
use crate::ui;

use self::achievements::AchievementState;
use self::artwork::ArtworkState;
use self::dlss_state::DlssState;
use self::external_app_icons::ExternalAppIconState;
use self::game_icons::GameIconState;
use self::install_size::InstallSizeState;
use self::playtime::PlaytimeState;
use self::state::{PageState, PowerAction, RuntimeState, ScreenSettingsAction};
use self::steam_update::SteamUpdateState;

const PASSIVE_REPAINT_INTERVAL: Duration = Duration::from_secs(1);
const CONTROLLER_IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const IDLE_DIM_OVERLAY_ALPHA: u8 = 200;
const IDLE_DIM_ANIMATION_SPEED: f32 = 4.0;
const LAUNCH_PRESS_FEEDBACK_DURATION: Duration = Duration::from_millis(200);
const STEAM_PROMPT_VISIBLE_DURATION: Duration = Duration::from_secs(5);
const STEAM_READY_VISIBLE_DURATION: Duration = Duration::from_secs(1);
const STEAM_NOTICE_ANIMATION_DURATION: Duration = Duration::from_millis(300);
const STEAM_STATUS_POLL_INTERVAL: Duration = Duration::from_millis(1_000);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LaunchNoticeKind {
    PromptStartSteam,
    SteamStarting,
    SteamStarted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LaunchNoticeStage {
    Entering,
    Visible,
    Exiting,
}

struct LaunchNotice {
    game_index: usize,
    kind: LaunchNoticeKind,
    stage: LaunchNoticeStage,
    stage_started_at: Instant,
    last_state_check_at: Option<Instant>,
    queued_kind: Option<LaunchNoticeKind>,
    launch_game_after_exit: bool,
}

struct LaunchPressFeedback {
    game_index: usize,
    started_at: Instant,
}

#[derive(Clone, Copy)]
struct PendingLaunchRequest {
    game_index: usize,
}

struct SteamClientStateMonitor {
    pending: Arc<Mutex<Option<launch::SteamClientState>>>,
    in_flight: bool,
    cached: launch::SteamClientState,
}

impl SteamClientStateMonitor {
    fn new() -> Self {
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

pub struct LauncherApp {
    language: AppLanguage,
    games: Vec<Game>,
    input: InputController,
    steam_paths: Vec<std::path::PathBuf>,
    artwork: ArtworkState,
    page: PageState,
    hint_icon_theme: PromptIconTheme,
    language_setting: AppLanguageSetting,
    display_mode_setting: DisplayModeSetting,
    hint_icons: Option<ui::HintIcons>,
    settings_icon: Option<egui::TextureHandle>,
    settings_system_icon: Option<egui::TextureHandle>,
    settings_screen_icon: Option<egui::TextureHandle>,
    settings_apps_icon: Option<egui::TextureHandle>,
    settings_exit_icon: Option<egui::TextureHandle>,
    xbox_guide_icon: Option<egui::TextureHandle>,
    playstation_home_icon: Option<egui::TextureHandle>,
    power_sleep_icon: Option<egui::TextureHandle>,
    power_reboot_icon: Option<egui::TextureHandle>,
    power_off_icon: Option<egui::TextureHandle>,
    game_icons: GameIconState,
    external_app_icons: ExternalAppIconState,
    steam_client_state_monitor: SteamClientStateMonitor,
    launch_state: Option<LaunchState>,
    launch_notice: Option<LaunchNotice>,
    launch_press_feedback: Option<LaunchPressFeedback>,
    pending_launch_request: Option<PendingLaunchRequest>,
    steam_update_launch_requested_app_id: Option<u32>,
    /// Promotion of the launched game to the front of the list is deferred
    /// until the launcher next regains focus (i.e. the user comes back from
    /// the launched game), so reordering the list does not interfere with
    /// the press animation or the focus-out/in transition.
    pending_promotion: Option<String>,
    running_games: HashMap<usize, launch::RunningGameState>,
    achievements: AchievementState,
    playtime: PlaytimeState,
    install_size: InstallSizeState,
    steam_update: SteamUpdateState,
    dlss: DlssState,
    runtime: RuntimeState,
    resolution_options: ResolutionOptions,
    display_scale_options: DisplayScaleOptions,
    game_scan_options: game::GameScanOptions,
    launch_on_startup_enabled: bool,
    background_home_wake_enabled: bool,
    controller_vibration_enabled: bool,
    power_menu_external_apps: Vec<ExternalApp>,
    wake_focus_pending: bool,
    pending_send_to_background: bool,
    send_to_background_after_frame: bool,
    send_to_background_commit_pending: bool,
    idle_dim_anim: animation::ExponentialAnimation,
    last_controller_activity_at: Instant,
    last_pointer_pos: Option<egui::Pos2>,
    last_pointer_activity: Option<Instant>,
}

impl LauncherApp {
    pub fn new(language: AppLanguage, ctx: &egui::Context) -> Self {
        let background_home_wake_enabled = crate::config::load_background_home_wake_enabled();
        #[cfg(target_os = "windows")]
        {
            input::set_background_home_wake_enabled(background_home_wake_enabled);
            input::start_watchers(ctx.clone());
        }
        #[cfg(not(target_os = "windows"))]
        let _ = ctx;

        let steam_paths = steam::find_steam_paths();
        let game_scan_options = crate::config::load_game_scan_options();
        let games = game::scan_installed_games(&steam_paths, &game_scan_options);
        let launch_on_startup_enabled = startup::is_enabled();
        let power_menu_external_apps = external_apps::detect_installed();
        let hint_icon_theme = crate::config::load_hint_icon_theme();
        let language_setting = crate::config::load_app_language_setting();
        let display_mode_setting = crate::config::load_display_mode_setting();
        let controller_vibration_enabled = crate::config::load_controller_vibration_enabled();
        let mut input = InputController::new();
        input.set_selection_vibration_enabled(controller_vibration_enabled);
        let mut app = LauncherApp {
            language,
            games,
            input,
            steam_paths,
            artwork: ArtworkState::new(ctx),
            page: PageState::new(),
            hint_icon_theme,
            language_setting,
            display_mode_setting,
            hint_icons: ui::load_hint_icons(ctx, hint_icon_theme),
            settings_icon: load_settings_icon(ctx),
            settings_system_icon: load_settings_system_icon(ctx),
            settings_screen_icon: load_settings_screen_icon(ctx),
            settings_apps_icon: load_settings_apps_icon(ctx),
            settings_exit_icon: load_settings_exit_icon(ctx),
            xbox_guide_icon: load_xbox_guide_icon(ctx),
            playstation_home_icon: load_playstation_home_icon(ctx),
            power_sleep_icon: load_power_sleep_icon(ctx),
            power_reboot_icon: load_power_reboot_icon(ctx),
            power_off_icon: load_power_off_icon(ctx),
            game_icons: GameIconState::new(),
            external_app_icons: ExternalAppIconState::new(),
            steam_client_state_monitor: SteamClientStateMonitor::new(),
            launch_state: None,
            launch_notice: None,
            launch_press_feedback: None,
            pending_launch_request: None,
            steam_update_launch_requested_app_id: None,
            pending_promotion: None,
            running_games: HashMap::new(),
            achievements: AchievementState::new(),
            playtime: PlaytimeState::new(),
            install_size: InstallSizeState::new(),
            steam_update: SteamUpdateState::new(),
            dlss: DlssState::new(),
            runtime: RuntimeState::new(),
            resolution_options: display_mode::detect_resolution_options(),
            display_scale_options: display_mode::detect_display_scale_options(),
            game_scan_options,
            launch_on_startup_enabled,
            background_home_wake_enabled,
            controller_vibration_enabled,
            power_menu_external_apps,
            wake_focus_pending: false,
            pending_send_to_background: false,
            send_to_background_after_frame: false,
            send_to_background_commit_pending: false,
            idle_dim_anim: animation::ExponentialAnimation::new(0.0),
            last_controller_activity_at: Instant::now(),
            last_pointer_pos: None,
            last_pointer_activity: None,
        };
        app.sync_screen_settings_state();
        app.refresh_selected_install_size(ctx);
        app
    }

    fn set_hint_icon_theme(&mut self, theme: crate::config::PromptIconTheme, ctx: &egui::Context) {
        if self.hint_icon_theme == theme {
            return;
        }

        self.hint_icon_theme = theme;
        self.hint_icons = ui::load_hint_icons(ctx, theme);
        crate::config::store_hint_icon_theme(theme);
        ctx.request_repaint();
    }

    fn cycle_language_setting(&mut self, ctx: &egui::Context) {
        let next_setting = self.language_setting.next();
        self.language_setting = next_setting;
        self.language = next_setting.resolve();
        crate::config::store_app_language_setting(next_setting);
        crate::configure_fonts(ctx, self.language);
        ctx.request_repaint();
    }

    fn set_display_mode_setting(&mut self, setting: DisplayModeSetting, ctx: &egui::Context) {
        if self.display_mode_setting == setting {
            return;
        }

        self.display_mode_setting = setting;
        crate::config::store_display_mode_setting(setting);

        if setting.is_fullscreen() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                display_mode::DEFAULT_WINDOWED_INNER_WIDTH,
                display_mode::DEFAULT_WINDOWED_INNER_HEIGHT,
            )));
        }

        ctx.request_repaint();
    }

    fn cycle_display_mode_setting(&mut self, ctx: &egui::Context) {
        self.set_display_mode_setting(self.display_mode_setting.next(), ctx);
    }

    fn set_controller_vibration_enabled(&mut self, enabled: bool, ctx: &egui::Context) {
        if self.controller_vibration_enabled == enabled {
            return;
        }

        self.controller_vibration_enabled = enabled;
        self.input.set_selection_vibration_enabled(enabled);
        crate::config::store_controller_vibration_enabled(enabled);
        ctx.request_repaint();
    }

    fn set_background_home_wake_enabled(&mut self, enabled: bool, ctx: &egui::Context) {
        if self.background_home_wake_enabled == enabled {
            return;
        }

        self.background_home_wake_enabled = enabled;
        input::set_background_home_wake_enabled(enabled);
        crate::config::store_background_home_wake_enabled(enabled);
        ctx.request_repaint();
    }

    fn set_game_scan_options(&mut self, options: game::GameScanOptions, ctx: &egui::Context) {
        if self.game_scan_options == options {
            return;
        }

        self.game_scan_options = options;
        crate::config::store_game_scan_options(options);
        self.refresh_game_list(ctx);
        ctx.request_repaint();
    }

    fn refresh_game_list(&mut self, ctx: &egui::Context) {
        let previous_selected = self.page.selected();
        let selected_key = self.games.get(previous_selected).map(Game::persistent_key);

        self.games = game::scan_installed_games(&self.steam_paths, &self.game_scan_options);
        self.running_games.clear();
        self.launch_state = None;
        self.launch_press_feedback = None;
        self.pending_launch_request = None;
        self.pending_promotion = None;

        if let Some(index) = selected_key
            .as_ref()
            .and_then(|key| self.games.iter().position(|game| game.persistent_key() == *key))
        {
            self.page.relocate_selection(index);
        } else if !self.games.is_empty() {
            self.page
                .relocate_selection(previous_selected.min(self.games.len() - 1));
        }

        self.refresh_selected_playtime(ctx);
        self.refresh_selected_install_size(ctx);
        self.refresh_selected_dlss(ctx);
    }

    fn can_open_achievement_panel_for_selected(&self) -> bool {
        self.games
            .get(self.page.selected())
            .map(|game| matches!(game.source, GameSource::Steam))
            .unwrap_or(false)
    }

    fn selected_launch_pending(&self) -> bool {
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

    fn should_queue_launch_feedback(&self, game_index: usize) -> bool {
        if !self.game_is_steam(game_index) {
            return true;
        }

        let Some(notice) = self.launch_notice.as_ref() else {
            return true;
        };

        notice.game_index == game_index
    }

    fn steam_launch_flow_active(&self) -> bool {
        self.launch_notice.is_some()
            || self.launch_press_feedback.is_some()
            || self.launch_state.is_some()
    }

    fn controller_idle_active(&self, now: Instant) -> bool {
        now.duration_since(self.last_controller_activity_at) >= CONTROLLER_IDLE_TIMEOUT
    }

    fn set_launch_press_feedback(&mut self, game_index: usize) {
        self.launch_press_feedback = Some(LaunchPressFeedback {
            game_index,
            started_at: Instant::now(),
        });
    }

    fn queue_launch_selected(&mut self, ctx: &egui::Context) {
        let game_index = self.page.selected();
        self.pending_launch_request = Some(PendingLaunchRequest { game_index });
        self.set_launch_press_feedback(game_index);
        ctx.request_repaint();
    }

    fn drain_pending_launch_request(&mut self, now: Instant, ctx: &egui::Context) {
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

    fn launch_notice_overlay_t(notice: &LaunchNotice, now: Instant) -> f32 {
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

    fn launch_notice_text(&self, notice: &LaunchNotice) -> String {
        match notice.kind {
            LaunchNoticeKind::PromptStartSteam => {
                self.language.steam_start_action_text().to_string()
            }
            LaunchNoticeKind::SteamStarting => self.language.steam_starting_text().to_string(),
            LaunchNoticeKind::SteamStarted => self.language.steam_started_text().to_string(),
        }
    }

    fn launch_notice_color(notice: &LaunchNotice) -> egui::Color32 {
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

    fn launch_state_app_id(&self) -> Option<u32> {
        self.launch_state
            .as_ref()
            .and_then(|state| self.games.get(state.game_index))
            .filter(|game| matches!(game.source, GameSource::Steam))
            .and_then(|game| game.app_id)
    }

    fn sync_steam_update_state(
        &mut self,
        selected_steam_app_id: Option<u32>,
        now: Instant,
        ctx: &egui::Context,
    ) {
        self.steam_update.drain_results();

        let launch_state_app_id = self.launch_state_app_id();
        let requested_update_app_id = self.steam_update_launch_requested_app_id;

        self.steam_update
            .refresh_for_app_id(selected_steam_app_id, &self.steam_paths, now, ctx);
        if launch_state_app_id != selected_steam_app_id {
            self.steam_update
                .refresh_for_app_id(launch_state_app_id, &self.steam_paths, now, ctx);
        }
        if requested_update_app_id != selected_steam_app_id
            && requested_update_app_id != launch_state_app_id
        {
            self.steam_update.refresh_for_app_id(
                requested_update_app_id,
                &self.steam_paths,
                now,
                ctx,
            );
        }

        if requested_update_app_id.is_none()
            && self
                .steam_update
                .status_for_app_id(launch_state_app_id)
                .is_some_and(|progress| progress.needs_update())
        {
            self.steam_update_launch_requested_app_id = launch_state_app_id;
        }

        if let Some(app_id) = self.steam_update_launch_requested_app_id {
            let update_still_pending = self
                .steam_update
                .status_for_app_id(Some(app_id))
                .is_some_and(|progress| progress.needs_update());

            if !update_still_pending {
                let should_restart_launch_timeout = launch_state_app_id == Some(app_id);
                self.steam_update_launch_requested_app_id = None;
                if should_restart_launch_timeout {
                    if let Some(state) = self.launch_state.as_mut() {
                        launch::restart_launch_timeout(state);
                    }
                }
            }
        }
    }

    fn should_show_steam_updating(&self, app_id: Option<u32>) -> bool {
        app_id.is_some() && self.steam_update_launch_requested_app_id == app_id
    }

    fn steam_update_overlay_text(&self, app_id: Option<u32>) -> String {
        if self.should_show_steam_updating(app_id) {
            self.language.steam_updating_text().to_owned()
        } else {
            self.language.steam_launch_after_update_text().to_owned()
        }
    }

    fn steam_update_overlay_color() -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(28, 90, 140, 208)
    }

    fn mark_steam_update_launch_requested(&mut self, game_index: usize) {
        let Some(app_id) = self.games.get(game_index).and_then(|game| game.app_id) else {
            return;
        };

        if self
            .steam_update
            .status_for_app_id(Some(app_id))
            .is_some_and(|progress| progress.needs_update())
        {
            self.steam_update_launch_requested_app_id = Some(app_id);
        }
    }

    fn launch_state_is_steam_update_pending(&self) -> bool {
        self.launch_state_app_id()
            .is_some_and(|app_id| Some(app_id) == self.steam_update_launch_requested_app_id)
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
                            self.show_launch_notice(selected, LaunchNoticeKind::PromptStartSteam, ctx);
                        }
                        launch::LaunchBlockedReason::SteamClientLoading => {
                            self.show_launch_notice(selected, LaunchNoticeKind::SteamStarting, ctx);
                        }
                    }
                }
                launch::LaunchAttemptResult::Failed => {
                    self.launch_state = None;
                    if self.steam_update_launch_requested_app_id == game.app_id {
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

    fn update_launch_notice(&mut self, now: Instant, ctx: &egui::Context) {
        let mut next_kind: Option<(usize, LaunchNoticeKind)> = None;
        let mut clear_notice = false;
        let mut launch_game_after_clear: Option<usize> = None;

        self.steam_client_state_monitor.drain();

        if let Some(notice) = self.launch_notice.as_mut() {
            if notice.kind == LaunchNoticeKind::SteamStarting
                && notice.stage != LaunchNoticeStage::Exiting
            {
                self.steam_client_state_monitor
                    .poll_if_due(now, &mut notice.last_state_check_at, ctx);
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

    fn launch_selected(&mut self, selected: usize, ctx: &egui::Context) {
        if self.try_advance_launch_notice(selected, ctx) {
            return;
        }

        if let Some(state) = self.running_games.get(&selected) {
            if let Some(focus_state) = launch::begin_focus_transition(selected, state) {
                self.launch_state = Some(focus_state);
                self.launch_notice = None;
                self.schedule_promotion(selected);
            } else {
                if launch::focus_running_game(state) {
                    self.launch_notice = None;
                    let _ = self.promote_game_to_front(selected);
                }
            }
            return;
        }

        self.start_selected_game_launch(selected, ctx);
    }

    fn schedule_promotion(&mut self, game_index: usize) {
        let Some(game_key) = self.games.get(game_index).map(Game::persistent_key) else {
            return;
        };
        self.pending_promotion = Some(game_key);
    }

    fn apply_pending_promotion(&mut self) {
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

    fn tick_launch_progress(&mut self, ctx: &egui::Context, launch_held: bool) {
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
                    self.running_games.insert(running_game.game_index, running_game);
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

    fn tick_running_game_state(&mut self) {
        self.running_games
            .retain(|_, state| launch::refresh_running_game(state));
    }

    fn refresh_selected_playtime(&mut self, ctx: &egui::Context) {
        self.playtime.refresh_for_selected(
            self.games.get(self.page.selected()),
            &self.steam_paths,
            ctx,
        );
    }

    fn refresh_selected_install_size(&mut self, ctx: &egui::Context) {
        self.install_size
            .refresh_for_selected(self.games.get(self.page.selected()), ctx);
    }

    fn refresh_selected_dlss(&mut self, ctx: &egui::Context) {
        self.dlss
            .refresh_for_selected(self.games.get(self.page.selected()), ctx);
    }

    fn promote_game_to_front(&mut self, game_index: usize) -> Option<usize> {
        let game_key = self.games.get(game_index)?.persistent_key();
        let now = game_last_played::now_unix_secs();
        let old_order = self
            .games
            .iter()
            .map(Game::persistent_key)
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
            // The promoted game is the one the user is currently looking at –
            // keep the selection animation intact so the press feedback only
            // animates the icon, not the title/badge size.
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

    fn sync_screen_settings_state(&mut self) {
        let resolution_index = self.resolution_options.current_resolution_index();
        let refresh_index = self
            .resolution_options
            .current_refresh_index_for(resolution_index);
        let refresh_count = self
            .resolution_options
            .refresh_rates_for(resolution_index)
            .len();
        let scale_index = self.display_scale_options.current_scale_index();

        self.page.sync_screen_settings(
            self.resolution_options.resolutions.len(),
            resolution_index,
            refresh_count,
            refresh_index,
            self.display_scale_options.scales.len(),
            scale_index,
        );
    }

    fn refresh_screen_settings_options(&mut self) {
        self.resolution_options = display_mode::detect_resolution_options();
        self.display_scale_options = display_mode::detect_display_scale_options();
        self.sync_screen_settings_state();
    }

    fn apply_resolution_indices(&mut self, resolution_index: usize, refresh_index: usize) {
        let Some(choice) = self
            .resolution_options
            .choice_for_indices(resolution_index, refresh_index)
        else {
            return;
        };

        let _ = display_mode::apply_resolution_choice(&choice);
        self.refresh_screen_settings_options();
    }

    fn apply_screen_settings_action(&mut self, action: ScreenSettingsAction) {
        let current_resolution_index = self.resolution_options.current_resolution_index();
        let current_refresh_index = self
            .resolution_options
            .current_refresh_index_for(current_resolution_index);

        match action {
            ScreenSettingsAction::SelectResolution(resolution_index) => {
                let preferred_refresh_hz = self
                    .resolution_options
                    .refresh_rates_for(current_resolution_index)
                    .get(current_refresh_index)
                    .copied()
                    .unwrap_or(self.resolution_options.current.refresh_hz);
                let next_refresh_index = self
                    .resolution_options
                    .refresh_rates_for(resolution_index)
                    .iter()
                    .position(|refresh_hz| *refresh_hz == preferred_refresh_hz)
                    .unwrap_or(0);

                self.apply_resolution_indices(resolution_index, next_refresh_index);
            }
            ScreenSettingsAction::SelectRefreshRate(refresh_index) => {
                self.apply_resolution_indices(current_resolution_index, refresh_index);
            }
            ScreenSettingsAction::SelectScale(scale_index) => {
                let Some(choice) = self.display_scale_options.choice_at(scale_index).cloned() else {
                    return;
                };

                let _ = display_mode::apply_display_scale_choice(&choice);
                self.refresh_screen_settings_options();
            }
        }
    }

    fn close_root_viewport(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn refresh_power_menu_state(&mut self) {
        self.refresh_screen_settings_options();
        self.launch_on_startup_enabled = startup::is_enabled();
        self.power_menu_external_apps = external_apps::detect_installed();
        let layout = crate::power_menu_structure::PowerMenuLayout::new(power::supported());
        if layout.is_empty() {
            return;
        }
        self.page.open_power_menu(layout);
    }

    fn schedule_input_repaint(
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

        // Keep a low-frequency tick for the clock and running-game polling
        // without redrawing the focused UI at full frame rate while idle.
        ctx.request_repaint_after(PASSIVE_REPAINT_INTERVAL);
    }

    fn update_cursor_visibility(&mut self, ctx: &egui::Context) {
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

    fn apply_power_action(
        &mut self,
        action: PowerAction,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
    ) {
        self.input.clear_held();
        let _ = frame;

        match action {
            PowerAction::Sleep => {
                let _ = power::sleep_system();
            }
            PowerAction::Reboot => {
                if power::reboot_system() {
                    self.close_root_viewport(ctx);
                }
            }
            PowerAction::Shutdown => {
                if power::shutdown_system() {
                    self.close_root_viewport(ctx);
                }
            }
        }
    }
}

impl eframe::App for LauncherApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.update_cursor_visibility(&ctx);
        let has_focus = ctx.input(|input| input.focused);

        if self.send_to_background_commit_pending {
            if has_focus {
                self.send_to_background_commit_pending = false;
                self.send_to_background_after_frame = false;
            } else {
                self.send_to_background_commit_pending = false;
                self.send_to_background_after_frame = false;
                if launch::send_current_app_to_background() {
                    return;
                }
            }
        }

        let now = Instant::now();
        self.update_launch_notice(now, &ctx);
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
            self.refresh_selected_playtime(&ctx);
            self.refresh_selected_install_size(&ctx);
            ctx.request_repaint();
        }

        if focus.did_lose_focus && !steam_launch_flow_active {
            self.page.prepare_wake_animation();
            ctx.request_repaint();
        }

        let wake_requested = input::take_wake_request();
        if wake_requested && self.background_home_wake_enabled {
            self.page.prepare_wake_animation();
            self.wake_focus_pending = true;
            self.runtime.suppress_home_hold_until_release();
            ctx.request_repaint();
        }

        if focus.should_clear_input {
            self.input.clear_held();
        }
        self.input.tick();

        let process_input = has_focus
            && !focus.in_cooldown
            && !self.pending_send_to_background;
        let modal_open = self.page.show_achievement_panel()
            || self.page.show_power_menu()
            || self.page.show_settings_page()
            || self.page.home_top_button_selected();
        let input_frame = self.input.poll(
            process_input,
            modal_open,
        );
        if let Some(theme) = input_frame.prompt_icon_theme {
            self.set_hint_icon_theme(theme, &ctx);
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

        let actions = input_frame.actions;
        let selected_running = self.running_games.contains_key(&self.page.selected());
        self.runtime
            .update_home_button(process_input, self.page.show_power_menu(), guide_held);
        let can_force_close = !self.page.show_achievement_panel() && !self.page.show_settings_page();
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
                        &ctx,
                    );
                    self.refresh_selected_install_size(&ctx);
                    self.refresh_selected_dlss(&ctx);
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
                        &ctx,
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
                    self.set_background_home_wake_enabled(!self.background_home_wake_enabled, &ctx);
                }
                if result.toggle_controller_vibration_feedback {
                    self.set_controller_vibration_enabled(!self.controller_vibration_enabled, &ctx);
                }
                if result.cycle_display_mode_setting {
                    self.cycle_display_mode_setting(&ctx);
                }
                if result.cycle_language_setting {
                    self.cycle_language_setting(&ctx);
                }
                if result.toggle_detect_steam_games {
                    let mut options = self.game_scan_options;
                    options.detect_steam_games = !options.detect_steam_games;
                    self.set_game_scan_options(options, &ctx);
                }
                if result.toggle_detect_epic_games {
                    let mut options = self.game_scan_options;
                    options.detect_epic_games = !options.detect_epic_games;
                    self.set_game_scan_options(options, &ctx);
                }
                if result.toggle_detect_xbox_games {
                    let mut options = self.game_scan_options;
                    options.detect_xbox_games = !options.detect_xbox_games;
                    self.set_game_scan_options(options, &ctx);
                }
                if achievement_selection_changed {
                    self.input.pulse_selection_change();
                }
                if result.selected_changed {
                    self.input.pulse_selection_change();
                    self.refresh_selected_playtime(&ctx);
                }
                if result.launch_selected && self.launch_press_feedback.is_none() {
                    let selected = self.page.selected();
                    if self.selected_launch_pending() {
                        self.set_launch_press_feedback(selected);
                        ctx.request_repaint();
                    } else {
                        if self.pending_launch_request.is_none() {
                            if self.should_queue_launch_feedback(selected) {
                                self.queue_launch_selected(&ctx);
                            } else {
                                self.launch_selected(selected, &ctx);
                            }
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
                    self.apply_power_action(power_action, &ctx, frame);
                }
                if result.close_frame {
                    self.close_root_viewport(&ctx);
                }
            }
        }

        self.drain_pending_launch_request(now, &ctx);
        let selected_steam_app_id = self
            .games
            .get(self.page.selected())
            .filter(|game| matches!(game.source, GameSource::Steam))
            .and_then(|game| game.app_id);
        self.sync_steam_update_state(selected_steam_app_id, now, &ctx);
        self.tick_launch_progress(&ctx, input_frame.launch_held);
        self.tick_running_game_state();
        self.achievements.drain_results();
        self.achievements.drain_icon_results(&ctx);
        self.playtime.drain_results(&mut self.games);
        self.install_size.drain_results(&mut self.games);
        self.dlss.drain_results(&mut self.games);

        let selected_game = self.games.get(self.page.selected());
        let selected_app_id = selected_game.and_then(|game| game.app_id);
        let selected_steam_update = self
            .steam_update
            .status_for_app_id(selected_steam_app_id)
            .cloned();
        let updated_global_percentages = steam::take_updated_global_achievement_percentages();
        let achievement_icon_scope = achievement_panel_scope_app_id(
            selected_app_id,
            self.page.show_achievement_panel(),
            self.page.achievement_panel_anim(),
        );
        self.achievements.sync_summary_scope(selected_app_id);
        self.achievements.sync_detail_scope(achievement_icon_scope);
        self.achievements.sync_icon_scope(achievement_icon_scope);
        self.achievements.refresh_after_global_percentage_update(
            selected_game,
            &self.steam_paths,
            self.language,
            self.page.show_achievement_panel(),
            &updated_global_percentages,
            &ctx,
        );
        if self
            .artwork
            .tick_selection(self.page.selected(), selected_app_id, &self.steam_paths, &ctx)
        {
            self.achievements
                .refresh_summary_for_selected(selected_game, &self.steam_paths, self.language, &ctx);
        }
        self.artwork.drain_pending(selected_app_id, &ctx);

        // Refresh `now` here: the value captured at the top of `update()` may
        // be tens of milliseconds stale after the input/IO work above. Feeding
        // a stale instant into the exponential animations would back-date their
        // `started_at`, so the next frame's wall-clock delta would advance the
        // curve by a huge chunk in one step (visible as the very first
        // post-idle game switch "swallowing" the icon scale animation).
        let now = Instant::now();
        self.idle_dim_anim
            .animate_to(idle_dim_target, IDLE_DIM_ANIMATION_SPEED, now, 0.001);
        if self.idle_dim_anim.update(now, 0.001) {
            ctx.request_repaint();
        }
        self.artwork.tick_fade(&ctx, now);
        self.page.tick_animations(&ctx, now);
        self.achievements.animate_reveals(&ctx, now);
        let idle_dim_t = self.idle_dim_anim.value();

        self.game_icons
            .ensure_loaded(&ctx, &self.steam_paths, &self.games, self.page.selected());
        self.external_app_icons
            .ensure_loaded(&ctx, &self.power_menu_external_apps);

        let selected_achievement_summary = self.achievements.summary_for_selected(selected_game);
        let selected_achievement_detail = self.achievements.detail_for_selected(selected_game);
        let selected_achievement_reveal = self.achievements.text_reveal_for_selected(selected_game);
        let previous_achievement_summary = self.achievements.previous_summary_for_display();
        let previous_achievement_reveal = self.achievements.previous_summary_reveal();
        let summary_cards_visibility = self.page.summary_cards_visibility();
        let can_open_achievement_panel = self.can_open_achievement_panel_for_selected();
        let achievement_loading = self.achievements.loading_for_selected(selected_game);
        let achievement_refresh_loading = self.achievements.refresh_loading_for_selected(selected_game);
        let achievement_has_no_data = self.achievements.has_no_data_for_selected(selected_game);
        let running_indices: Vec<usize> = self.running_games.keys().copied().collect();
        let press_feedback = self.launch_press_feedback.as_ref().and_then(|feedback| {
            let elapsed = animation::scale_seconds(now.duration_since(feedback.started_at).as_secs_f32());
            (elapsed <= LAUNCH_PRESS_FEEDBACK_DURATION.as_secs_f32())
                .then_some((feedback.game_index, elapsed))
        });
        if press_feedback.is_none() {
            self.launch_press_feedback = None;
        }
        let launch_feedback = press_feedback;
        let launch_notice = self.launch_notice.as_ref().and_then(|notice| {
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
        });
        let steam_update_notice = if launch_notice.is_none() {
            if selected_steam_update
                .as_ref()
                .is_some_and(|progress| progress.needs_update())
            {
                Some((
                    self.page.selected(),
                    self.steam_update_overlay_text(selected_steam_app_id),
                    Self::steam_update_overlay_color(),
                ))
            } else {
                None
            }
        } else {
            None
        };
        let launching_index = self.launch_state.as_ref().map(|state| state.game_index);
        let render_wake_anim = if (has_focus || steam_launch_flow_active)
            && !self.send_to_background_after_frame
        {
            self.page.wake_anim()
        } else {
            0.0
        };
        let mut visible_achievement_icon_urls = Vec::new();

        egui::Frame::new()
            .fill(egui::Color32::TRANSPARENT)
            .show(ui, |ui| {
                ui::draw_background(
                    &ctx,
                    self.artwork.vignette(),
                    !self.page.show_achievement_panel(),
                    self.settings_icon.as_ref(),
                    self.power_off_icon.as_ref(),
                    !self.page.show_achievement_panel()
                        && !self.page.show_settings_page(),
                    self.page.home_settings_focus_anim(),
                    1.0,
                    self.page.home_power_focus_anim(),
                    self.page.power_menu_anim() > 0.001,
                    self.artwork.cover(),
                    self.artwork.cover_prev(),
                    self.artwork.logo(),
                    self.artwork.logo_prev(),
                    self.artwork.fade(),
                    self.page.cover_nav_dir(),
                    self.page.achievement_panel_anim(),
                    render_wake_anim,
                );

                ui::draw_game_list(
                    ui,
                    self.language,
                    &self.games,
                    self.page.selected(),
                    self.page.select_anim(),
                    self.page.home_top_focus_anim(),
                    self.page.achievement_panel_anim(),
                    self.page.scroll_offset(),
                    self.game_icons.textures(),
                    self.hint_icons.as_ref().map(|icons| &icons.btn_a),
                    launch_feedback,
                    launch_notice,
                    steam_update_notice,
                    launching_index,
                    &running_indices,
                    summary_cards_visibility,
                    selected_achievement_summary,
                    selected_achievement_reveal,
                    previous_achievement_summary,
                    previous_achievement_reveal,
                    render_wake_anim,
                );

                if self.page.achievement_panel_anim() > 0.001 && can_open_achievement_panel {
                    if let Some(game) = self.games.get(self.page.selected()) {
                        let game_icon = self.game_icons.get(&game.icon_key());
                        ui::draw_achievement_page(
                            ui,
                            self.language,
                            game,
                            selected_achievement_detail,
                            achievement_loading,
                            achievement_has_no_data,
                            selected_achievement_reveal,
                            self.page.achievement_selected(),
                            self.page.achievement_select_anim(),
                            self.page.achievement_panel_anim(),
                            self.page.selected(),
                            self.page.select_anim(),
                            self.page.scroll_offset(),
                            self.page.achievement_scroll_offset(),
                            game_icon,
                            self.hint_icons.as_ref(),
                            self.achievements.revealed_hidden_for_selected(selected_game),
                            self.achievements.hidden_reveal_progress_for_selected(selected_game),
                            self.achievements.icon_cache(),
                            self.achievements.icon_reveal(),
                            self.achievements.percent_reveal(),
                        )
                        .into_iter()
                        .for_each(|url| visible_achievement_icon_urls.push(url));
                    }
                }

                ui::draw_settings_page(
                    ui,
                    self.language,
                    self.language_setting,
                    self.display_mode_setting,
                    self.settings_system_icon.as_ref(),
                    self.settings_screen_icon.as_ref(),
                    self.settings_apps_icon.as_ref(),
                    self.settings_exit_icon.as_ref(),
                    self.xbox_guide_icon.as_ref(),
                    self.playstation_home_icon.as_ref(),
                    self.launch_on_startup_enabled,
                    self.background_home_wake_enabled,
                    self.controller_vibration_enabled,
                    self.game_scan_options.detect_steam_games,
                    self.game_scan_options.detect_epic_games,
                    self.game_scan_options.detect_xbox_games,
                    &self.resolution_options,
                    &self.display_scale_options,
                    self.resolution_options.current_resolution_index(),
                    self.resolution_options.current_refresh_index_for(
                        self.resolution_options.current_resolution_index(),
                    ),
                    self.display_scale_options.current_scale_index(),
                    self.page.settings_screen_resolution_dropdown_open(),
                    self.page.settings_screen_refresh_dropdown_open(),
                    self.page.settings_screen_scale_dropdown_open(),
                    self.page.settings_screen_dropdown_selected_index(),
                    self.page.settings_section_index(),
                    self.page.settings_selected_item_index(),
                    self.page.show_settings_page(),
                    self.page.settings_in_submenu(),
                    self.page.settings_page_anim(),
                    self.page.settings_submenu_anim(),
                    self.page.settings_select_anim(),
                    self.page.settings_focus_key(),
                );

                if let Some(icons) = &self.hint_icons {
                    let settings_action_label = if self.page.show_settings_page() {
                        Some(self.language.confirm_text())
                    } else {
                        None
                    };
                    let home_top_action_label = if self.page.home_settings_selected() {
                        Some(self.language.confirm_text())
                    } else if self.page.home_power_selected() {
                        Some(self.language.confirm_text())
                    } else {
                        None
                    };

                    ui::draw_hint_bar(
                        ui,
                        self.language,
                        icons,
                        self.page.show_achievement_panel(),
                        self.page.show_power_menu(),
                        self.page.show_settings_page(),
                        self.page.home_top_button_selected(),
                        home_top_action_label,
                        settings_action_label,
                        can_open_achievement_panel,
                        achievement_refresh_loading,
                        selected_running,
                        self.runtime.force_close_hold_progress(),
                        render_wake_anim,
                    );
                }

                ui::draw_power_menu(
                    ui,
                    self.language,
                    self.page.power_menu_layout(),
                    self.power_sleep_icon.as_ref(),
                    self.power_reboot_icon.as_ref(),
                    self.power_off_icon.as_ref(),
                    self.hint_icons.as_ref(),
                    &self.resolution_options.current.label,
                    "",
                    "",
                    power::supported(),
                    self.page.power_menu_anim(),
                    self.page.power_menu_select_anim(),
                    self.page.power_menu_scroll_offset(),
                    self.page.home_power_focus_anim(),
                    render_wake_anim,
                );

                if idle_dim_t > 0.001 {
                    ui.painter().rect_filled(
                        ui.max_rect(),
                        egui::CornerRadius::ZERO,
                        egui::Color32::from_rgba_unmultiplied(
                            0,
                            0,
                            0,
                            ((IDLE_DIM_OVERLAY_ALPHA as f32) * idle_dim_t)
                                .round()
                                .clamp(0.0, IDLE_DIM_OVERLAY_ALPHA as f32) as u8,
                        ),
                    );
                }
            });

        if self.send_to_background_after_frame {
            self.send_to_background_after_frame = false;
            self.send_to_background_commit_pending = true;
            ctx.request_repaint();
        }

        self.schedule_input_repaint(
            &ctx,
            has_focus,
            has_controller_activity,
        );

        if !visible_achievement_icon_urls.is_empty() {
            self.achievements
                .ensure_icons_for_urls(achievement_icon_scope, &ctx, &visible_achievement_icon_urls);
        }
    }
}

fn load_settings_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_generated_icon(
        ctx,
        include_bytes!(concat!(env!("OUT_DIR"), "/settings-icon-ui.png")),
        "home_settings_icon",
    )
}

fn load_settings_system_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_generated_icon(
        ctx,
        include_bytes!(concat!(env!("OUT_DIR"), "/system-icon-ui.png")),
        "settings_system_icon",
    )
}

fn load_settings_screen_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_generated_icon(
        ctx,
        include_bytes!(concat!(env!("OUT_DIR"), "/screen-icon-ui.png")),
        "settings_screen_icon",
    )
}

fn load_settings_apps_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_generated_icon(
        ctx,
        include_bytes!(concat!(env!("OUT_DIR"), "/apps-icon-ui.png")),
        "settings_apps_icon",
    )
}

fn load_settings_exit_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_generated_icon(
        ctx,
        include_bytes!(concat!(env!("OUT_DIR"), "/exit-icon-ui.png")),
        "settings_exit_icon",
    )
}

fn load_xbox_guide_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_embedded_png_texture(
        ctx,
        include_bytes!("../icons/Xbox Series/xbox_guide.png"),
        "settings_xbox_guide_icon",
    )
}

fn load_playstation_home_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_embedded_png_texture(
        ctx,
        include_bytes!("../icons/PlayStation Series/playstation_home.png"),
        "settings_playstation_home_icon",
    )
}

fn load_power_sleep_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_generated_icon(
        ctx,
        include_bytes!(concat!(env!("OUT_DIR"), "/power-sleep-icon-ui.png")),
        "home_power_sleep_icon",
    )
}

fn load_power_reboot_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_generated_icon(
        ctx,
        include_bytes!(concat!(env!("OUT_DIR"), "/power-reboot-icon-ui.png")),
        "home_power_reboot_icon",
    )
}

fn load_power_off_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_generated_icon(
        ctx,
        include_bytes!(concat!(env!("OUT_DIR"), "/power-off-icon-ui.png")),
        "home_power_off_icon",
    )
}

fn load_generated_icon(
    ctx: &egui::Context,
    bytes: &[u8],
    texture_name: &str,
) -> Option<egui::TextureHandle> {
    load_embedded_png_texture(ctx, bytes, texture_name)
}

fn load_embedded_png_texture(
    ctx: &egui::Context,
    bytes: &[u8],
    texture_name: &str,
) -> Option<egui::TextureHandle> {
    let dyn_img = image::load_from_memory(bytes).ok()?;
    let rgba = dyn_img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    Some(ctx.load_texture(
        texture_name,
        image,
        egui::TextureOptions::LINEAR,
    ))
}

fn achievement_panel_scope_app_id(
    selected_app_id: Option<u32>,
    achievement_panel_open: bool,
    achievement_panel_anim: f32,
) -> Option<u32> {
    if achievement_panel_open || achievement_panel_anim > 0.001 {
        selected_app_id
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::achievement_panel_scope_app_id;

    #[test]
    fn keeps_achievement_scope_while_close_animation_is_still_visible() {
        assert_eq!(
            achievement_panel_scope_app_id(Some(42), false, 0.25),
            Some(42)
        );
    }

    #[test]
    fn clears_achievement_scope_after_close_animation_finishes() {
        assert_eq!(achievement_panel_scope_app_id(Some(42), false, 0.001), None);
        assert_eq!(achievement_panel_scope_app_id(Some(42), false, 0.0), None);
    }

    #[test]
    fn keeps_achievement_scope_while_panel_is_open() {
        assert_eq!(achievement_panel_scope_app_id(Some(42), true, 0.0), Some(42));
    }
}