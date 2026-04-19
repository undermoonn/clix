use eframe::egui;
use std::collections::HashMap;
use std::time::Instant;

use crate::achievements::AchievementState;
use crate::artwork::ArtworkState;
use crate::display_mode::{self, ResolutionOptions};
use crate::game_icons::GameIconState;
use crate::i18n::AppLanguage;
use crate::input::InputController;
use crate::install_size::InstallSizeState;
use crate::launch::{self, LaunchState};
use crate::page_state::{PageState, ResolutionPreset};
use crate::playtime::PlaytimeState;
use crate::runtime_state::RuntimeState;
use crate::steam::{self, Game};
use crate::ui;

pub struct LauncherApp {
    language: AppLanguage,
    games: Vec<Game>,
    input: InputController,
    steam_paths: Vec<std::path::PathBuf>,
    artwork: ArtworkState,
    page: PageState,
    hint_icons: Option<ui::HintIcons>,
    game_icons: GameIconState,
    launch_state: Option<LaunchState>,
    running_games: HashMap<usize, launch::RunningGameState>,
    achievements: AchievementState,
    playtime: PlaytimeState,
    install_size: InstallSizeState,
    runtime: RuntimeState,
    resolution_options: ResolutionOptions,
    launch_on_startup_enabled: bool,
    wake_focus_pending: bool,
    pending_send_to_background: bool,
}

impl LauncherApp {
    pub fn new(language: AppLanguage, ctx: &egui::Context) -> Self {
        #[cfg(target_os = "windows")]
        crate::xbox_home::start(ctx.clone());
        #[cfg(not(target_os = "windows"))]
        let _ = ctx;

        let steam_paths = steam::find_steam_paths();
        let games = steam::scan_games_with_paths(&steam_paths);
        let mut app = LauncherApp {
            language,
            games,
            input: InputController::new(),
            steam_paths,
            artwork: ArtworkState::new(),
            page: PageState::new(),
            hint_icons: ui::load_hint_icons(ctx),
            game_icons: GameIconState::new(),
            launch_state: None,
            running_games: HashMap::new(),
            achievements: AchievementState::new(),
            playtime: PlaytimeState::new(),
            install_size: InstallSizeState::new(),
            runtime: RuntimeState::new(),
            resolution_options: crate::display_mode::detect_resolution_options(),
            launch_on_startup_enabled: crate::startup::is_enabled(),
            wake_focus_pending: false,
            pending_send_to_background: false,
        };
        app.refresh_selected_install_size(ctx);
        app
    }

    fn can_open_achievement_panel_for_selected(&self) -> bool {
        self.games.get(self.page.selected()).is_some()
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
            } else {
                let _ = launch::focus_running_game(state);
            }
            return;
        }

        if let Some(game) = self.games.get(selected) {
            self.launch_state =
                launch::begin_launch(selected, game, &self.steam_paths);
        }
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

    fn apply_resolution_preset(&self, preset: ResolutionPreset) {
        let option = match preset {
            ResolutionPreset::HalfMaxRefresh => &self.resolution_options.half_refresh,
            ResolutionPreset::MaxRefresh => &self.resolution_options.max_refresh,
        };

        let _ = display_mode::apply_resolution_choice(option);
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.output_mut(|output| output.cursor_icon = egui::CursorIcon::None);

        if self.wake_focus_pending {
            self.wake_focus_pending = false;
            let _ = crate::launch::focus_current_app_window();
            self.page.start_wake_animation();
            ctx.request_repaint();
        }

        let has_focus = ctx.input(|input| input.focused);
        let now = Instant::now();
        let focus = self.runtime.update_focus(has_focus, now);

        if focus.did_gain_focus {
            self.refresh_selected_playtime(ctx);
            self.refresh_selected_install_size(ctx);
        }

        if crate::xbox_home::take_wake_request() {
            self.page.prepare_wake_animation();
            self.wake_focus_pending = true;
            self.runtime.suppress_home_hold_until_release();
            ctx.request_repaint();
        }

        if has_focus {
            ctx.request_repaint();
        }

        if focus.should_clear_input {
            self.input.clear_held();
        }
        self.input.tick();

        let process_input = has_focus && !focus.in_cooldown && !self.pending_send_to_background;
        let input_frame = self.input.poll(
            process_input,
            self.page.show_achievement_panel() || self.page.show_home_menu(),
        );

        if self.pending_send_to_background {
            if input_frame.launch_held {
                ctx.request_repaint();
            } else {
                self.pending_send_to_background = false;
                self.input.clear_held();
                let _ = launch::send_current_app_to_background();
            }
        }

        let actions = input_frame.actions;
        let selected_running = self.running_games.contains_key(&self.page.selected());
        let home_hold = self.runtime.update_home_hold(
            process_input,
            self.page.show_home_menu(),
            crate::xbox_home::guide_held(),
            now,
        );
        let can_force_close = !self.page.show_achievement_panel();
        let force_close_hold = self.runtime.update_force_close_hold(
            process_input && can_force_close,
            selected_running && can_force_close,
            input_frame.force_close_held && can_force_close,
            now,
        );
        if home_hold.trigger_menu {
            self.resolution_options = crate::display_mode::detect_resolution_options();
            self.launch_on_startup_enabled = crate::startup::is_enabled();
            self.page.open_home_menu();
            ctx.request_repaint();
        }
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

        for action in &actions {
            let previous_game = self.games.get(self.page.selected());
            let previous_achievement_panel = self.page.show_achievement_panel();
            let previous_achievement_selected = self.page.achievement_selected();
            let achievement_len = self
                .achievements
                .summary_for_selected(self.games.get(self.page.selected()))
                .map(|summary| summary.items.len())
                .unwrap_or(0);
            let result = self.page.handle_action(
                action,
                self.games.len(),
                self.can_open_achievement_panel_for_selected(),
                achievement_len,
            );
            let achievement_selection_changed = previous_achievement_panel
                && self.page.show_achievement_panel()
                && self.page.achievement_selected() != previous_achievement_selected;
            let achievement_panel_closed = previous_achievement_panel && !self.page.show_achievement_panel();

            if result.selected_changed
                || result.open_achievement_panel
                || result.refresh_achievements
                || result.toggle_achievement_sort
                || achievement_selection_changed
                || achievement_panel_closed
            {
                self.achievements
                    .clear_revealed_hidden_for_selected(previous_game);
            }

            if result.open_achievement_panel {
                self.achievements.refresh_for_selected(
                    self.games.get(self.page.selected()),
                    &self.steam_paths,
                    self.language,
                    ctx,
                );
            }
            if result.reveal_hidden_achievement
                && self.achievements.reveal_hidden_description_for_selected(
                    self.games.get(self.page.selected()),
                    self.page.achievement_selected(),
                )
            {
                ctx.request_repaint();
            }
            if result.refresh_achievements {
                self.achievements.force_refresh_for_selected(
                    self.games.get(self.page.selected()),
                    &self.steam_paths,
                    self.language,
                    ctx,
                );
            }
            if result.toggle_achievement_sort {
                self.achievements.toggle_sort_order();
                ctx.request_repaint();
            }
            if result.toggle_launch_on_startup {
                if crate::startup::set_enabled(!self.launch_on_startup_enabled) {
                    self.launch_on_startup_enabled = !self.launch_on_startup_enabled;
                } else {
                    self.launch_on_startup_enabled = crate::startup::is_enabled();
                }
                ctx.request_repaint();
            }
            if achievement_selection_changed {
                self.input.pulse_selection_change();
            }
            if result.selected_changed {
                self.input.pulse_selection_change();
                self.refresh_selected_playtime(ctx);
                self.refresh_selected_install_size(ctx);
            }
            if result.launch_selected && !self.selected_launch_pending() {
                self.launch_selected();
            }
            if result.send_app_to_background {
                self.pending_send_to_background = true;
                self.input.clear_held();
                ctx.request_repaint();
            }
            if let Some(preset) = result.set_resolution {
                self.apply_resolution_preset(preset);
            }
            if result.close_frame {
                frame.close();
            }
        }

        self.tick_launch_progress(ctx, input_frame.launch_held);
        self.tick_running_game_state();
        self.achievements.drain_results();
        self.achievements.drain_icon_results(ctx);
        self.playtime.drain_results(&mut self.games);
        self.install_size.drain_results(&mut self.games);

        let selected_game = self.games.get(self.page.selected());
        let selected_app_id = selected_game.and_then(|game| game.app_id);
        let achievement_icon_scope = if self.page.show_achievement_panel() {
            selected_app_id
        } else {
            None
        };
        self.achievements.sync_icon_scope(achievement_icon_scope);
        if self
            .artwork
            .tick_selection(self.page.selected(), selected_app_id, &self.steam_paths, ctx)
        {
            self.achievements
                .refresh_for_selected(selected_game, &self.steam_paths, self.language, ctx);
        }
        self.artwork.drain_pending(selected_app_id, ctx);

        let dt = ctx.input(|input| input.predicted_dt);
        self.artwork.tick_fade(ctx, dt);
        self.page.tick_animations(ctx, dt);
        self.achievements.animate_reveals(ctx, dt);

        self.game_icons
            .ensure_loaded(ctx, &self.steam_paths, &self.games);

        let selected_achievement_summary = self.achievements.summary_for_selected(selected_game);
        let selected_achievement_reveal = self.achievements.text_reveal_for_selected(selected_game);
        let can_open_achievement_panel = self.can_open_achievement_panel_for_selected();
        let achievement_loading = self.achievements.loading_for_selected(selected_game);
        let achievement_refresh_loading = self.achievements.refresh_loading_for_selected(selected_game);
        let achievement_has_no_data = self.achievements.has_no_data_for_selected(selected_game);
        let running_indices: Vec<usize> = self.running_games.keys().copied().collect();
        let launch_feedback = self
            .launch_state
            .as_ref()
            .map(|state| (state.game_index, state.elapsed_seconds()));
        let mut visible_achievement_icon_urls = Vec::new();

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                ui::draw_background(
                    ctx,
                    self.artwork.cover(),
                    self.artwork.cover_prev(),
                    self.artwork.logo(),
                    self.artwork.logo_prev(),
                    self.artwork.fade(),
                    self.page.cover_nav_dir(),
                    self.page.achievement_panel_anim(),
                    self.page.wake_anim(),
                );

                ui::draw_game_list(
                    ui,
                    self.language,
                    &self.games,
                    self.page.selected(),
                    self.page.select_anim(),
                    self.page.achievement_panel_anim(),
                    self.page.scroll_offset(),
                    self.game_icons.textures(),
                    launch_feedback,
                    &running_indices,
                    self.page.show_achievement_panel(),
                    selected_achievement_summary,
                    selected_achievement_reveal,
                    self.page.wake_anim(),
                );

                if self.page.achievement_panel_anim() > 0.001 {
                    if let Some(game) = self.games.get(self.page.selected()) {
                        let game_icon = game
                            .app_id
                            .and_then(|app_id| self.game_icons.get(app_id));
                        ui::draw_achievement_page(
                            ui,
                            self.language,
                            game,
                            selected_achievement_summary,
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
                            self.page.wake_anim(),
                            game_icon,
                            self.hint_icons.as_ref(),
                            self.achievements.revealed_hidden_for_selected(selected_game),
                            self.achievements.hidden_reveal_progress_for_selected(selected_game),
                            self.achievements.sort_order().is_descending(),
                            self.achievements.icon_cache(),
                            self.achievements.icon_reveal(),
                        )
                        .into_iter()
                        .for_each(|url| visible_achievement_icon_urls.push(url));
                    }
                }

                if let Some(icons) = &self.hint_icons {
                    ui::draw_hint_bar(
                        ui,
                        self.language,
                        icons,
                        self.page.show_achievement_panel(),
                        self.page.show_home_menu(),
                        can_open_achievement_panel,
                        achievement_refresh_loading,
                        selected_running,
                        self.runtime.force_close_hold_progress(),
                        self.page.wake_anim(),
                    );
                }

                ui::draw_home_menu(
                    ui,
                    self.language,
                    self.hint_icons.as_ref(),
                    &self.resolution_options.current.label,
                    &self.resolution_options.half_refresh.label,
                    &self.resolution_options.max_refresh.label,
                    self.launch_on_startup_enabled,
                    crate::startup::supported(),
                    self.page.home_menu_anim(),
                    self.page.home_menu_scroll_offset(),
                    self.page.wake_anim(),
                );
            });

        if !visible_achievement_icon_urls.is_empty() {
            self.achievements
                .ensure_icons_for_urls(achievement_icon_scope, ctx, &visible_achievement_icon_urls);
        }
    }
}