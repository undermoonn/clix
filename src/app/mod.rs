mod achievements;
mod artwork;
mod dlss_state;
mod external_app_icons;
mod game_icons;
mod install_size;
mod playtime;
mod state;

use eframe::egui;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::animation;
use crate::config::PromptIconTheme;
use crate::game::{self, Game, GameSource};
use crate::game_last_played;
use crate::i18n::{AppLanguage, AppLanguageSetting};
use crate::input::{self, InputController};
use crate::launch::{self, LaunchState};
use crate::system::{
    display_mode::{self, ResolutionOptions},
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
use self::state::{PageState, PowerAction, ResolutionPreset, RuntimeState};

const IDLE_REPAINT_INTERVAL: Duration = Duration::from_secs(1);

pub struct LauncherApp {
    language: AppLanguage,
    games: Vec<Game>,
    input: InputController,
    steam_paths: Vec<std::path::PathBuf>,
    artwork: ArtworkState,
    page: PageState,
    hint_icon_theme: PromptIconTheme,
    language_setting: AppLanguageSetting,
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
    launch_state: Option<LaunchState>,
    /// Promotion of the launched game to the front of the list is deferred
    /// until the launcher next regains focus (i.e. the user comes back from
    /// the launched game), so reordering the list does not interfere with
    /// the press animation or the focus-out/in transition.
    pending_promotion: Option<String>,
    running_games: HashMap<usize, launch::RunningGameState>,
    achievements: AchievementState,
    playtime: PlaytimeState,
    install_size: InstallSizeState,
    dlss: DlssState,
    runtime: RuntimeState,
    resolution_options: ResolutionOptions,
    game_scan_options: game::GameScanOptions,
    launch_on_startup_enabled: bool,
    background_home_wake_enabled: bool,
    controller_vibration_enabled: bool,
    power_menu_external_apps: Vec<ExternalApp>,
    wake_focus_pending: bool,
    pending_send_to_background: bool,
    send_to_background_after_frame: bool,
    send_to_background_commit_pending: bool,
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
            launch_state: None,
            pending_promotion: None,
            running_games: HashMap::new(),
            achievements: AchievementState::new(),
            playtime: PlaytimeState::new(),
            install_size: InstallSizeState::new(),
            dlss: DlssState::new(),
            runtime: RuntimeState::new(),
            resolution_options: display_mode::detect_resolution_options(),
            game_scan_options,
            launch_on_startup_enabled,
            background_home_wake_enabled,
            controller_vibration_enabled,
            power_menu_external_apps,
            wake_focus_pending: false,
            pending_send_to_background: false,
            send_to_background_after_frame: false,
            send_to_background_commit_pending: false,
        };
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

    fn launch_selected(&mut self) {
        let selected = self.page.selected();
        if let Some(state) = self.running_games.get(&selected) {
            if let Some(focus_state) = launch::begin_focus_transition(selected, state) {
                self.launch_state = Some(focus_state);
                self.schedule_promotion(selected);
            } else {
                if launch::focus_running_game(state) {
                    let _ = self.promote_game_to_front(selected);
                }
            }
            return;
        }

        if let Some(game) = self.games.get(selected) {
            self.launch_state = launch::begin_launch(selected, game, &self.steam_paths);
            if self.launch_state.is_some() {
                self.schedule_promotion(selected);
            }
        }
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
        if let Some(state) = self.launch_state.as_mut() {
            ctx.request_repaint();
            match launch::tick_launch_progress(state, launch_held) {
                launch::LaunchTickResult::Pending => {}
                launch::LaunchTickResult::Ready(running_game) => {
                    self.running_games.insert(running_game.game_index, running_game);
                    self.launch_state = None;
                }
                launch::LaunchTickResult::TimedOut => {
                    self.launch_state = None;
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
    }

    fn apply_resolution_preset(&mut self, preset: ResolutionPreset) {
        let option = match preset {
            ResolutionPreset::HalfMaxRefresh => &self.resolution_options.half_refresh,
            ResolutionPreset::MaxRefresh => &self.resolution_options.max_refresh,
        };

        let _ = display_mode::apply_resolution_choice(option);
        self.resolution_options = display_mode::detect_resolution_options();
    }

    fn close_root_viewport(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn refresh_power_menu_state(&mut self) {
        self.resolution_options = display_mode::detect_resolution_options();
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
        } else {
            ctx.request_repaint_after(IDLE_REPAINT_INTERVAL);
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
        ctx.output_mut(|output| output.cursor_icon = egui::CursorIcon::None);

        if self.send_to_background_commit_pending {
            self.send_to_background_commit_pending = false;
            self.send_to_background_after_frame = false;
            if launch::send_current_app_to_background() {
                return;
            }
        }

        let now = Instant::now();

        if self.wake_focus_pending {
            self.wake_focus_pending = false;
            self.page.start_wake_animation(now);
            ctx.request_repaint();
        }

        let has_focus = ctx.input(|input| input.focused);
        let focus = self.runtime.update_focus(has_focus, now);

        if focus.did_gain_focus {
            self.apply_pending_promotion();
            self.page.start_wake_animation(now);
            self.refresh_selected_playtime(&ctx);
            self.refresh_selected_install_size(&ctx);
            ctx.request_repaint();
        }

        if focus.did_lose_focus {
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

        let process_input = has_focus && !focus.in_cooldown && !self.pending_send_to_background;
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
        let has_controller_activity = input_frame.has_input_activity || guide_held;

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
                if result.launch_selected && !self.selected_launch_pending() {
                    self.launch_selected();
                }
                if let Some(kind) = result.launch_external_app {
                    let _ = external_apps::launch(kind, &self.power_menu_external_apps);
                }
                if result.send_app_to_background {
                    self.pending_send_to_background = true;
                    self.input.clear_held();
                    ctx.request_repaint();
                }
                if let Some(preset) = result.set_resolution {
                    self.apply_resolution_preset(preset);
                }
                if let Some(power_action) = result.power_action {
                    self.apply_power_action(power_action, &ctx, frame);
                }
                if result.close_frame {
                    self.close_root_viewport(&ctx);
                }
            }
        }

        self.tick_launch_progress(&ctx, input_frame.launch_held);
        self.tick_running_game_state();
        self.achievements.drain_results();
        self.achievements.drain_icon_results(&ctx);
        self.playtime.drain_results(&mut self.games);
        self.install_size.drain_results(&mut self.games);
        self.dlss.drain_results(&mut self.games);

        let selected_game = self.games.get(self.page.selected());
        let selected_app_id = selected_game.and_then(|game| game.app_id);
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

        self.artwork.tick_fade(&ctx, now);
        self.page.tick_animations(&ctx, now);
        self.achievements.animate_reveals(&ctx, now);

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
        let launch_feedback = self
            .launch_state
            .as_ref()
            .map(|state| (state.game_index, animation::scale_seconds(state.elapsed_seconds())));
        let render_wake_anim = if has_focus && !self.send_to_background_after_frame {
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
                    launch_feedback,
                    &running_indices,
                    self.page.show_achievement_panel(),
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
                    self.page.settings_section_index(),
                    self.page.settings_selected_item_index(),
                    self.page.settings_in_submenu(),
                    self.page.settings_page_anim(),
                    self.page.settings_submenu_anim(),
                    self.page.settings_select_anim(),
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
                    &self.resolution_options.half_refresh.label,
                    &self.resolution_options.max_refresh.label,
                    power::supported(),
                    self.page.power_menu_anim(),
                    self.page.power_menu_select_anim(),
                    self.page.power_menu_scroll_offset(),
                    self.page.home_power_focus_anim(),
                    render_wake_anim,
                );
            });

        if self.send_to_background_after_frame {
            self.send_to_background_after_frame = false;
            self.send_to_background_commit_pending = true;
            ctx.request_repaint();
        }

        self.schedule_input_repaint(&ctx, has_focus, has_controller_activity);

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