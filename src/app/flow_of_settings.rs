use eframe::{self, egui};

use crate::system::{display_mode, external_apps, power, startup};

use super::state::{PowerAction, ScreenSettingsAction};
use super::{LauncherApp, PowerMenuLayout};

impl LauncherApp {
    pub(super) fn set_hint_icon_theme(
        &mut self,
        theme: crate::config::PromptIconTheme,
        ctx: &egui::Context,
    ) {
        if self.hint_icon_theme == theme {
            return;
        }

        self.hint_icon_theme = theme;
        self.hint_icons = crate::ui::load_hint_icons(ctx, theme);
        crate::config::store_hint_icon_theme(theme);
        ctx.request_repaint();
    }

    pub(super) fn cycle_language_setting(&mut self, ctx: &egui::Context) {
        let next_setting = self.language_setting.next();
        self.language_setting = next_setting;
        self.language = next_setting.resolve();
        crate::config::store_app_language_setting(next_setting);
        crate::configure_fonts(ctx, self.language);
        ctx.request_repaint();
    }

    fn set_display_mode_setting(
        &mut self,
        setting: display_mode::DisplayModeSetting,
        ctx: &egui::Context,
    ) {
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

    pub(super) fn cycle_display_mode_setting(&mut self, ctx: &egui::Context) {
        self.set_display_mode_setting(self.display_mode_setting.next(), ctx);
    }

    pub(super) fn set_idle_frame_rate_reduction_enabled(
        &mut self,
        enabled: bool,
        ctx: &egui::Context,
    ) {
        if self.idle_frame_rate_reduction_enabled == enabled {
            return;
        }

        self.idle_frame_rate_reduction_enabled = enabled;
        crate::config::store_idle_frame_rate_reduction_enabled(enabled);
        ctx.request_repaint();
    }

    pub(super) fn set_controller_vibration_enabled(&mut self, enabled: bool, ctx: &egui::Context) {
        if self.controller_vibration_enabled == enabled {
            return;
        }

        self.controller_vibration_enabled = enabled;
        self.input.set_selection_vibration_enabled(enabled);
        crate::config::store_controller_vibration_enabled(enabled);
        ctx.request_repaint();
    }

    pub(super) fn set_background_home_wake_mode(
        &mut self,
        mode: crate::config::BackgroundHomeWakeMode,
        ctx: &egui::Context,
    ) {
        if self.background_home_wake_mode == mode {
            return;
        }

        self.background_home_wake_mode = mode;
        crate::input::set_background_home_wake_mode(mode);
        crate::config::store_background_home_wake_mode(mode);
        ctx.request_repaint();
    }

    pub(super) fn set_game_scan_options(
        &mut self,
        options: crate::game::GameScanOptions,
        ctx: &egui::Context,
    ) {
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
        let selected_key = self
            .games
            .get(previous_selected)
            .map(crate::game::Game::persistent_key);

        self.games = crate::game::scan_installed_games(&self.steam_paths, &self.game_scan_options);
        self.running_games.clear();
        self.launch_state = None;
        self.launch_press_feedback = None;
        self.pending_launch_request = None;
        self.pending_promotion = None;

        if let Some(index) = selected_key.as_ref().and_then(|key| {
            self.games
                .iter()
                .position(|game| game.persistent_key() == *key)
        }) {
            self.page.relocate_selection(index);
        } else if !self.games.is_empty() {
            self.page
                .relocate_selection(previous_selected.min(self.games.len() - 1));
        }

        self.refresh_selected_playtime(ctx);
        self.refresh_selected_install_size(ctx);
        self.refresh_selected_dlss(ctx);
    }

    pub(super) fn sync_screen_settings_state(&mut self) {
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

    pub(super) fn apply_screen_settings_action(&mut self, action: ScreenSettingsAction) {
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
                let Some(choice) = self.display_scale_options.choice_at(scale_index).cloned()
                else {
                    return;
                };

                let _ = display_mode::apply_display_scale_choice(&choice);
                self.refresh_screen_settings_options();
            }
        }
    }

    pub(super) fn refresh_power_menu_state(&mut self) {
        self.refresh_screen_settings_options();
        self.launch_on_startup_enabled = startup::is_enabled();
        self.power_menu_external_apps = external_apps::detect_installed();
        let layout = PowerMenuLayout::new(power::supported());
        if layout.is_empty() {
            return;
        }
        self.page.open_power_menu(layout);
    }

    pub(super) fn apply_power_action(
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
