use eframe::egui;

use crate::system::power;
use crate::ui;

use super::view_helpers::ViewRenderState;
use super::{LauncherApp, IDLE_DIM_OVERLAY_ALPHA};

impl LauncherApp {
    pub(super) fn render_main_view(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        has_focus: bool,
        has_controller_activity: bool,
        _selected_running: bool,
        view_state: ViewRenderState,
    ) {
        let ViewRenderState {
            achievement_icon_scope,
            idle_dim_t,
            launch_feedback,
            launch_notice,
            steam_update_notice,
            launching_index,
            render_wake_anim,
            running_indices,
        } = view_state;

        let selected_game_index = self.selected_game_index();
        let selected_game = selected_game_index.and_then(|index| self.games.get(index));
        let home_game_indices = self.home_game_indices();
        let selected_achievement_summary = self.achievements.summary_for_selected(selected_game);
        let selected_achievement_detail = self.achievements.detail_for_selected(selected_game);
        let selected_achievement_reveal = self.achievements.text_reveal_for_selected(selected_game);
        let previous_achievement_summary = self.achievements.previous_summary_for_display();
        let previous_achievement_reveal = self.achievements.previous_summary_reveal();
        let summary_cards_visibility = self.page.summary_cards_visibility();
        let can_open_achievement_panel = self.can_open_achievement_panel_for_selected();
        let achievement_loading = self.achievements.loading_for_selected(selected_game);
        let achievement_refresh_loading = self
            .achievements
            .refresh_loading_for_selected(selected_game);
        let achievement_has_no_data = self.achievements.has_no_data_for_selected(selected_game);
        let mut visible_achievement_icon_urls = Vec::new();

        egui::Frame::new()
            .fill(egui::Color32::TRANSPARENT)
            .show(ui, |ui| {
                ui::draw_background(
                    ctx,
                    self.artwork.vignette(),
                    !self.page.show_achievement_panel(),
                    self.settings_icon.as_ref(),
                    self.power_off_icon.as_ref(),
                    !self.page.show_achievement_panel()
                        && !self.page.show_settings_page()
                        && !self.page.show_game_library_page(),
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
                    &home_game_indices,
                    self.page.selected(),
                    self.page.select_anim(),
                    self.page.home_top_focus_anim(),
                    self.page.achievement_panel_anim(),
                    self.page.scroll_offset(),
                    self.game_icons.textures(),
                    self.shelf_icon.as_ref(),
                    self.hint_icons.as_ref().map(|icons| &icons.btn_a),
                    launch_feedback,
                    launch_notice.clone(),
                    steam_update_notice.clone(),
                    launching_index,
                    &running_indices,
                    summary_cards_visibility,
                    selected_achievement_summary,
                    selected_achievement_reveal,
                    previous_achievement_summary,
                    previous_achievement_reveal,
                    render_wake_anim,
                );

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
                    self.home_game_limit,
                    self.background_home_wake_mode,
                    self.controller_vibration_enabled,
                    self.idle_frame_rate_reduction_enabled,
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
                    self.page.settings_home_game_limit_dropdown_open(),
                    self.page.settings_screen_dropdown_selected_index(),
                    self.page.settings_home_game_limit_dropdown_selected_index(),
                    self.page.settings_section_index(),
                    self.page.settings_selected_item_index(),
                    self.page.show_settings_page(),
                    self.page.settings_in_submenu(),
                    self.page.settings_page_anim(),
                    self.page.settings_submenu_anim(),
                    self.page.settings_select_anim(),
                    self.page.settings_focus_key(),
                );

                ui::draw_game_library_page(
                    ui,
                    self.language,
                    &self.games,
                    self.page.game_library_selected(),
                    self.page.game_library_page_anim(),
                    self.page.game_library_select_anim(),
                    self.page.game_library_scroll_offset(),
                    self.game_library_background.as_ref(),
                    self.game_icons.textures(),
                    self.hint_icons.as_ref().map(|icons| &icons.btn_a),
                    launch_feedback,
                    launch_notice,
                    steam_update_notice,
                    launching_index,
                    &running_indices,
                    &self.hidden_home_game_keys,
                    self.hide_icon.as_ref(),
                    render_wake_anim,
                );

                if self.page.achievement_panel_anim() > 0.001 && can_open_achievement_panel {
                    if let Some((game_index, game)) = selected_game_index
                        .and_then(|index| self.games.get(index).map(|game| (index, game)))
                    {
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
                            game_index,
                            self.page.select_anim(),
                            self.page.scroll_offset(),
                            self.page.achievement_scroll_offset(),
                            self.page.achievement_from_game_library(),
                            self.page.show_achievement_panel(),
                            game_icon,
                            self.hint_icons.as_ref(),
                            self.achievements
                                .revealed_hidden_for_selected(selected_game),
                            self.achievements
                                .hidden_reveal_progress_for_selected(selected_game),
                            self.achievements.icon_cache(),
                            self.achievements.icon_reveal(),
                            self.achievements.percent_reveal(),
                        )
                        .into_iter()
                        .for_each(|url| visible_achievement_icon_urls.push(url));
                    }
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

                if let Some(game_index) = self.page.game_menu_game_index() {
                    if let Some(game) = self.games.get(game_index) {
                        ui::draw_game_menu(
                            ui,
                            self.language,
                            self.page.game_menu_layout(),
                            &game.name,
                            self.close_icon.as_ref(),
                            self.detail_icon.as_ref(),
                            self.hide_icon.as_ref(),
                            self.show_icon.as_ref(),
                            self.page.game_menu_home_hidden(),
                            self.page.game_menu_anim(),
                            self.page.game_menu_select_anim(),
                            self.page.game_menu_scroll_offset(),
                            render_wake_anim,
                        );
                    }
                }

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
                        self.page.show_game_menu(),
                        self.page.show_game_library_page(),
                        self.page.show_settings_page(),
                        self.page.home_top_button_selected(),
                        home_top_action_label,
                        settings_action_label,
                        can_open_achievement_panel,
                        selected_game.is_some(),
                        achievement_refresh_loading,
                        render_wake_anim,
                    );
                }

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
                                .clamp(0.0, IDLE_DIM_OVERLAY_ALPHA as f32)
                                as u8,
                        ),
                    );
                }
            });

        if self.send_to_background_after_frame {
            self.send_to_background_after_frame = false;
            self.send_to_background_commit_pending = true;
            ctx.request_repaint();
        }

        self.schedule_input_repaint(ctx, has_focus, has_controller_activity);

        if !visible_achievement_icon_urls.is_empty() {
            self.achievements.ensure_icons_for_urls(
                achievement_icon_scope,
                ctx,
                &visible_achievement_icon_urls,
            );
        }
    }
}
