use eframe::egui;
use std::collections::HashMap;
use std::time::Instant;

use crate::achievements::AchievementState;
use crate::artwork::ArtworkState;
use crate::game_icons::GameIconState;
use crate::i18n::AppLanguage;
use crate::input::{ControllerBrand, InputController};
use crate::launch::{self, LaunchState};
use crate::page_state::PageState;
use crate::runtime_state::RuntimeState;
use crate::settings;
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
    hint_icon_brand: ControllerBrand,
    game_icons: GameIconState,
    launch_state: Option<LaunchState>,
    running_games: HashMap<usize, launch::RunningGameState>,
    achievements: AchievementState,
    runtime: RuntimeState,
}

impl LauncherApp {
    pub fn new(language: AppLanguage) -> Self {
        let app_settings = settings::load_settings();
        let steam_paths = steam::find_steam_paths();
        let games = steam::scan_games_with_paths(&steam_paths);
        LauncherApp {
            language,
            games,
            input: InputController::new(app_settings.controller_brand),
            steam_paths,
            artwork: ArtworkState::new(),
            page: PageState::new(),
            hint_icons: None,
            hint_icon_brand: app_settings.controller_brand,
            game_icons: GameIconState::new(),
            launch_state: None,
            running_games: HashMap::new(),
            achievements: AchievementState::new(),
            runtime: RuntimeState::new(),
        }
    }

    fn can_open_achievement_panel_for_selected(&self) -> bool {
        self.games.get(self.page.selected()).is_some()
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

    fn refresh_running_game_indices(&mut self) {
        if self.running_games.is_empty() {
            return;
        }

        let previous = std::mem::take(&mut self.running_games);
        let mut remapped = HashMap::new();

        for (_, state) in previous {
            let new_index = self.games.iter().position(|game| state.matches_game(game));

            if let Some(index) = new_index {
                remapped.insert(index, state.with_game_index(index));
            }
        }

        self.running_games = remapped;
    }

    fn refresh_games_after_resume(&mut self) {
        let selected_key = self
            .games
            .get(self.page.selected())
            .map(|g| (g.app_id, g.name.clone()));

        self.games = steam::scan_games_with_paths(&self.steam_paths);

        if self.games.is_empty() {
            self.page.reset_after_resume(0);
        } else if let Some((app_id, name)) = selected_key {
            let mut new_selected = None;

            if let Some(id) = app_id {
                new_selected = self.games.iter().position(|g| g.app_id == Some(id));
            }

            if new_selected.is_none() {
                new_selected = self.games.iter().position(|g| g.name == name);
            }

            let selected = new_selected.unwrap_or_else(|| self.page.selected().min(self.games.len() - 1));
            self.page.reset_after_resume(selected);
        } else {
            self.page
                .reset_after_resume(self.page.selected().min(self.games.len() - 1));
        }
        self.achievements.reset_selected_tracking();

        self.artwork.reset_selection_tracking();
        self.game_icons.reset();
        self.refresh_running_game_indices();
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.output_mut(|output| output.cursor_icon = egui::CursorIcon::None);

        let has_focus = ctx.input(|input| input.focused);
        let now = Instant::now();
        let focus = self.runtime.update_focus(has_focus, now);

        if has_focus {
            ctx.request_repaint();
        }

        if focus.should_refresh_games {
            self.refresh_games_after_resume();
        }
        if focus.should_clear_input {
            self.input.clear_held();
        }
        self.input.tick();

        let process_input = has_focus && !focus.in_cooldown && self.launch_state.is_none();
        let input_frame = self.input.poll(process_input, self.page.show_achievement_panel());
        let actions = input_frame.actions;
        let selected_running = self.running_games.contains_key(&self.page.selected());
        let app_quit_hold = self.runtime.update_quit_hold(
            process_input,
            self.page.show_achievement_panel(),
            input_frame.quit_held,
            now,
        );
        let force_close_hold = self.runtime.update_force_close_hold(
            process_input,
            selected_running,
            input_frame.force_close_held,
            now,
        );
        if app_quit_hold.trigger_quit {
            frame.close();
        }
        if force_close_hold.trigger_force_close {
            if let Some(state) = self.running_games.get_mut(&self.page.selected()) {
                if launch::close_running_game(state) {
                    ctx.request_repaint();
                }
            }
        }
        if app_quit_hold.should_repaint || force_close_hold.should_repaint {
            ctx.request_repaint();
        }

        for action in &actions {
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
            if result.open_achievement_panel {
                self.achievements.refresh_for_selected(
                    self.games.get(self.page.selected()),
                    &self.steam_paths,
                    self.language,
                    ctx,
                );
            }
            if result.selected_changed {
                self.input.pulse_selection_change();
            }
            if result.launch_selected {
                self.launch_selected();
            }
            if result.close_frame {
                frame.close();
            }
            if result.suppress_quit_hold_until_release {
                self.runtime.suppress_quit_hold_until_release();
            }
        }

        self.tick_launch_progress(ctx, input_frame.launch_held);
        self.tick_running_game_state();
        self.achievements.drain_results();
        self.achievements.drain_icon_results(ctx);

        let selected_game = self.games.get(self.page.selected());
        let selected_app_id = selected_game.and_then(|game| game.app_id);
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

        let controller_brand = self.input.controller_brand();
        if self.hint_icons.is_none() || self.hint_icon_brand != controller_brand {
            self.hint_icons = ui::load_hint_icons(ctx, controller_brand);
            self.hint_icon_brand = controller_brand;
            settings::save_controller_brand(controller_brand);
        }

        self.game_icons
            .ensure_loaded(ctx, &self.steam_paths, &self.games);

        let selected_achievement_summary = self.achievements.summary_for_selected(selected_game);
        let selected_achievement_reveal = self.achievements.text_reveal_for_selected(selected_game);
        let can_open_achievement_panel = self.can_open_achievement_panel_for_selected();
        let achievement_loading = self.achievements.loading_for_selected(selected_game);
        let achievement_has_no_data = self.achievements.has_no_data_for_selected(selected_game);
        let running_indices: Vec<usize> = self.running_games.keys().copied().collect();
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
                    self.launch_state.as_ref().map(|state| state.game_index),
                    &running_indices,
                    self.page.show_achievement_panel(),
                    selected_achievement_summary,
                    selected_achievement_reveal,
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
                            game_icon,
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
                        can_open_achievement_panel,
                        selected_running,
                        self.runtime.quit_hold_progress(),
                        self.runtime.force_close_hold_progress(),
                    );
                }
            });

        if !visible_achievement_icon_urls.is_empty() {
            self.achievements
                .ensure_icons_for_urls(ctx, &visible_achievement_icon_urls);
        }
    }
}