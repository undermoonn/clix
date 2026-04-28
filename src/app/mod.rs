// Local helpers and flows.
mod achievements;
mod artwork;
mod dlss_state;
mod external_app_icons;
mod flow_of_focus;
mod flow_of_input;
mod flow_of_launch;
mod flow_of_render;
mod flow_of_settings;
mod game_icons;
mod install_size;
mod playtime;
mod runtime_helpers;
mod state;
mod steam_update;
mod ui_assets;
mod view_helpers;

pub(crate) use self::state::{PowerMenuLayout, PowerMenuOption};

// Standard library.
use eframe::egui;
use std::collections::HashMap;
use std::time::{Duration, Instant};

// Crate modules.
use crate::animation;
use crate::config::PromptIconTheme;
use crate::game::{self, Game, GameSource};
use crate::i18n::{AppLanguage, AppLanguageSetting};
use crate::input::{self, InputController};
use crate::launch::{self, LaunchState};
use crate::system::{
    display_mode::{self, DisplayModeSetting, DisplayScaleOptions, ResolutionOptions},
    external_apps::{self, ExternalApp},
    startup,
};
use crate::ui;

// Local app modules.
use self::achievements::AchievementState;
use self::artwork::ArtworkState;
use self::dlss_state::DlssState;
use self::external_app_icons::ExternalAppIconState;
use self::flow_of_focus::FocusFlowOutcome;
use self::flow_of_input::InputFlowOutcome;
use self::flow_of_launch::{
    LaunchNotice, LaunchPressFeedback, PendingLaunchRequest, SteamClientStateMonitor,
};
use self::game_icons::GameIconState;
use self::install_size::InstallSizeState;
use self::playtime::PlaytimeState;
use self::state::{PageState, RuntimeState};
use self::steam_update::SteamUpdateState;
use self::ui_assets::{
    load_playstation_home_icon, load_power_off_icon, load_power_reboot_icon,
    load_power_sleep_icon, load_settings_apps_icon, load_settings_exit_icon,
    load_settings_icon, load_settings_screen_icon, load_settings_system_icon,
    load_xbox_guide_icon,
};
use self::view_helpers::ViewRenderState;

const PASSIVE_REPAINT_INTERVAL: Duration = Duration::from_secs(1);
const CONTROLLER_IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const IDLE_DIM_OVERLAY_ALPHA: u8 = 200;
const IDLE_DIM_ANIMATION_SPEED: f32 = 4.0;
const LAUNCH_PRESS_FEEDBACK_DURATION: Duration = Duration::from_millis(200);
const STEAM_PROMPT_VISIBLE_DURATION: Duration = Duration::from_secs(5);
const STEAM_READY_VISIBLE_DURATION: Duration = Duration::from_secs(1);
const STEAM_NOTICE_ANIMATION_DURATION: Duration = Duration::from_millis(300);
const STEAM_STATUS_POLL_INTERVAL: Duration = Duration::from_millis(1_000);

pub struct LauncherApp {
    // Core app context.
    language: AppLanguage,
    games: Vec<Game>,
    steam_paths: Vec<std::path::PathBuf>,

    // Runtime coordinators.
    input: InputController,
    artwork: ArtworkState,
    page: PageState,
    runtime: RuntimeState,

    // Persisted settings and system options.
    hint_icon_theme: PromptIconTheme,
    language_setting: AppLanguageSetting,
    display_mode_setting: DisplayModeSetting,
    game_scan_options: game::GameScanOptions,
    launch_on_startup_enabled: bool,
    background_home_wake_enabled: bool,
    controller_vibration_enabled: bool,
    resolution_options: ResolutionOptions,
    display_scale_options: DisplayScaleOptions,
    power_menu_external_apps: Vec<ExternalApp>,

    // UI assets and icon caches.
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

    // Launch flow state.
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

    // Async data and derived status.
    running_games: HashMap<usize, launch::RunningGameState>,
    achievements: AchievementState,
    playtime: PlaytimeState,
    install_size: InstallSizeState,
    steam_update: SteamUpdateState,
    dlss: DlssState,

    // Transient UI state.
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
            crate::system::overlay::start(language);
            input::set_background_home_wake_enabled(background_home_wake_enabled);
            input::start_watchers(ctx.clone());
        }
        #[cfg(not(target_os = "windows"))]
        let _ = ctx;

        // Core app context.
        let steam_paths = crate::game_scan::steam::find_steam_paths();
        let game_scan_options = crate::config::load_game_scan_options();
        let games = game::scan_installed_games(&steam_paths, &game_scan_options);

        // Persisted settings and system options.
        let launch_on_startup_enabled = startup::is_enabled();
        let hint_icon_theme = crate::config::load_hint_icon_theme();
        let language_setting = crate::config::load_app_language_setting();
        let display_mode_setting = crate::config::load_display_mode_setting();
        let controller_vibration_enabled = crate::config::load_controller_vibration_enabled();
        let resolution_options = display_mode::detect_resolution_options();
        let display_scale_options = display_mode::detect_display_scale_options();
        let power_menu_external_apps = external_apps::detect_installed();

        // Runtime coordinators.
        let mut input = InputController::new();
        input.set_selection_vibration_enabled(controller_vibration_enabled);
        let artwork = ArtworkState::new(ctx);
        let page = PageState::new();
        let runtime = RuntimeState::new();

        // UI assets and icon caches.
        let hint_icons = ui::load_hint_icons(ctx, hint_icon_theme);
        let settings_icon = load_settings_icon(ctx);
        let settings_system_icon = load_settings_system_icon(ctx);
        let settings_screen_icon = load_settings_screen_icon(ctx);
        let settings_apps_icon = load_settings_apps_icon(ctx);
        let settings_exit_icon = load_settings_exit_icon(ctx);
        let xbox_guide_icon = load_xbox_guide_icon(ctx);
        let playstation_home_icon = load_playstation_home_icon(ctx);
        let power_sleep_icon = load_power_sleep_icon(ctx);
        let power_reboot_icon = load_power_reboot_icon(ctx);
        let power_off_icon = load_power_off_icon(ctx);
        let game_icons = GameIconState::new();
        let external_app_icons = ExternalAppIconState::new();

        // Launch flow state.
        let steam_client_state_monitor = SteamClientStateMonitor::new();

        // Async data and derived status.
        let achievements = AchievementState::new();
        let playtime = PlaytimeState::new();
        let install_size = InstallSizeState::new();
        let steam_update = SteamUpdateState::new();
        let dlss = DlssState::new();

        let mut app = LauncherApp {
            language,
            games,
            steam_paths,

            input,
            artwork,
            page,
            runtime,

            hint_icon_theme,
            language_setting,
            display_mode_setting,
            game_scan_options,
            launch_on_startup_enabled,
            background_home_wake_enabled,
            controller_vibration_enabled,
            resolution_options,
            display_scale_options,
            power_menu_external_apps,

            hint_icons,
            settings_icon,
            settings_system_icon,
            settings_screen_icon,
            settings_apps_icon,
            settings_exit_icon,
            xbox_guide_icon,
            playstation_home_icon,
            power_sleep_icon,
            power_reboot_icon,
            power_off_icon,
            game_icons,
            external_app_icons,

            steam_client_state_monitor,
            launch_state: None,
            launch_notice: None,
            launch_press_feedback: None,
            pending_launch_request: None,
            steam_update_launch_requested_app_id: None,
            pending_promotion: None,

            running_games: HashMap::new(),
            achievements,
            playtime,
            install_size,
            steam_update,
            dlss,

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

}

impl eframe::App for LauncherApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.update_cursor_visibility(&ctx);
        let Some(FocusFlowOutcome {
            has_focus,
            now,
            steam_launch_flow_active,
            idle_dim_target,
            process_input,
        }) = self.handle_focus_flow(&ctx) else {
            return;
        };

        let InputFlowOutcome {
            has_controller_activity,
            launch_held,
            selected_running,
        } = self.handle_input_flow(process_input, now, &ctx, frame);

        self.drain_pending_launch_request(now, &ctx);
        let selected_steam_app_id = self
            .games
            .get(self.page.selected())
            .filter(|game| matches!(game.source, GameSource::Steam))
            .and_then(|game| game.steam_app_id);
        self.sync_steam_update_state(selected_steam_app_id, now, &ctx);
        self.tick_launch_progress(&ctx, launch_held);
        self.tick_running_game_state();
        self.achievements.drain_results();
        self.achievements.drain_icon_results(&ctx);
        self.playtime.drain_results(&mut self.games);
        self.install_size.drain_results(&mut self.games);
        self.dlss.drain_results(&mut self.games);

        // Refresh `now` here: the value captured at the top of `update()` may
        // be tens of milliseconds stale after the input/IO work above. Feeding
        // a stale instant into the exponential animations would back-date their
        // `started_at`, so the next frame's wall-clock delta would advance the
        // curve by a huge chunk in one step (visible as the very first
        // post-idle game switch "swallowing" the icon scale animation).
        let ViewRenderState {
            achievement_icon_scope,
            idle_dim_t,
            launch_feedback,
            launch_notice,
            steam_update_notice,
            launching_index,
            render_wake_anim,
            running_indices,
        } = self.prepare_view_render_state(
            &ctx,
            has_focus,
            steam_launch_flow_active,
            idle_dim_target,
        );

        self.render_main_view(
            ui,
            &ctx,
            has_focus,
            has_controller_activity,
            selected_running,
            ViewRenderState {
                achievement_icon_scope,
                idle_dim_t,
                launch_feedback,
                launch_notice,
                steam_update_notice,
                launching_index,
                render_wake_anim,
                running_indices,
            },
        );
    }
}

