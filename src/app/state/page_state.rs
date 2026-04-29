use std::time::Instant;

use eframe::egui;

use super::power::{PowerMenuLayout, PowerMenuOption};
use crate::animation::ExponentialAnimation;
use crate::input::ControllerAction;
use crate::system::external_apps::ExternalAppKind;

const ANIMATION_EPSILON: f32 = 0.001;
const SETTINGS_PAGE_ENTER_ANIM_SPEED: f32 = 5.0;
const SETTINGS_PAGE_EXIT_ANIM_SPEED: f32 = 8.0;
const SETTINGS_SUBMENU_ENTER_ANIM_SPEED: f32 = 4.0;
const SETTINGS_SUBMENU_EXIT_ANIM_SPEED: f32 = 4.0;
const SETTINGS_PAGE_ENTER_INITIAL_PROGRESS: f32 = 0.18;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenSettingsAction {
    SelectResolution(usize),
    SelectRefreshRate(usize),
    SelectScale(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerAction {
    Sleep,
    Reboot,
    Shutdown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingsSection {
    System,
    Screen,
    Apps,
    CloseApp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScreenDropdown {
    Resolution,
    RefreshRate,
    Scale,
}

impl SettingsSection {
    fn index(self) -> usize {
        match self {
            Self::System => 0,
            Self::Screen => 1,
            Self::Apps => 2,
            Self::CloseApp => 3,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::System => Self::System,
            Self::Screen => Self::System,
            Self::Apps => Self::Screen,
            Self::CloseApp => Self::Apps,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::System => Self::Screen,
            Self::Screen => Self::Apps,
            Self::Apps => Self::CloseApp,
            Self::CloseApp => Self::CloseApp,
        }
    }
}

pub struct PageActionResult {
    pub open_achievement_panel: bool,
    pub open_power_menu: bool,
    pub reveal_hidden_achievement: bool,
    pub refresh_achievements: bool,
    pub cycle_display_mode_setting: bool,
    pub cycle_language_setting: bool,
    pub toggle_launch_on_startup: bool,
    pub toggle_background_home_wake: bool,
    pub toggle_controller_vibration_feedback: bool,
    pub toggle_idle_frame_rate_reduction: bool,
    pub toggle_detect_steam_games: bool,
    pub toggle_detect_epic_games: bool,
    pub toggle_detect_xbox_games: bool,
    pub launch_selected: bool,
    pub launch_external_app: Option<ExternalAppKind>,
    pub selected_changed: bool,
    pub close_frame: bool,
    pub send_app_to_background: bool,
    pub screen_settings_action: Option<ScreenSettingsAction>,
    pub power_action: Option<PowerAction>,
}

pub struct PageState {
    selected: usize,
    home_settings_selected: bool,
    home_settings_focus_anim: ExponentialAnimation,
    home_power_selected: bool,
    home_power_focus_anim: ExponentialAnimation,
    cover_nav_dir: f32,
    select_anim: ExponentialAnimation,
    select_anim_target: Option<usize>,
    summary_cards_visibility: ExponentialAnimation,
    wake_anim: ExponentialAnimation,
    scroll_offset: ExponentialAnimation,
    show_power_menu: bool,
    power_menu_anim: ExponentialAnimation,
    power_menu_selected: usize,
    power_menu_select_anim: ExponentialAnimation,
    power_menu_select_anim_target: Option<usize>,
    power_menu_layout: PowerMenuLayout,
    power_menu_scroll_offset: f32,
    show_settings_page: bool,
    settings_page_anim: ExponentialAnimation,
    settings_submenu_anim: ExponentialAnimation,
    settings_select_anim: ExponentialAnimation,
    settings_select_anim_target: Option<u16>,
    settings_section: SettingsSection,
    settings_in_submenu: bool,
    settings_system_selected: usize,
    settings_screen_selected: usize,
    settings_screen_dropdown: Option<ScreenDropdown>,
    settings_screen_dropdown_selected: usize,
    settings_screen_resolution_count: usize,
    settings_screen_refresh_count: usize,
    settings_screen_scale_count: usize,
    settings_screen_current_resolution: usize,
    settings_screen_current_refresh: usize,
    settings_screen_current_scale: usize,
    settings_apps_selected: usize,
    show_achievement_panel: bool,
    achievement_panel_anim: ExponentialAnimation,
    achievement_selected: usize,
    achievement_select_anim: ExponentialAnimation,
    achievement_select_anim_target: Option<usize>,
    achievement_scroll_offset: ExponentialAnimation,
}

impl PageState {
    pub fn new() -> Self {
        Self {
            selected: 0,
            home_settings_selected: false,
            home_settings_focus_anim: ExponentialAnimation::new(0.0),
            home_power_selected: false,
            home_power_focus_anim: ExponentialAnimation::new(0.0),
            cover_nav_dir: 0.0,
            select_anim: ExponentialAnimation::new(0.0),
            select_anim_target: None,
            summary_cards_visibility: ExponentialAnimation::new(1.0),
            wake_anim: ExponentialAnimation::new(1.0),
            scroll_offset: ExponentialAnimation::new(0.0),
            show_power_menu: false,
            power_menu_anim: ExponentialAnimation::new(0.0),
            power_menu_selected: 0,
            power_menu_select_anim: ExponentialAnimation::new(0.0),
            power_menu_select_anim_target: None,
            power_menu_layout: PowerMenuLayout::default(),
            power_menu_scroll_offset: 0.0,
            show_settings_page: false,
            settings_page_anim: ExponentialAnimation::new(0.0),
            settings_submenu_anim: ExponentialAnimation::new(0.0),
            settings_select_anim: ExponentialAnimation::new(0.0),
            settings_select_anim_target: None,
            settings_section: SettingsSection::System,
            settings_in_submenu: false,
            settings_system_selected: 0,
            settings_screen_selected: 0,
            settings_screen_dropdown: None,
            settings_screen_dropdown_selected: 0,
            settings_screen_resolution_count: 1,
            settings_screen_refresh_count: 1,
            settings_screen_scale_count: 1,
            settings_screen_current_resolution: 0,
            settings_screen_current_refresh: 0,
            settings_screen_current_scale: 0,
            settings_apps_selected: 0,
            show_achievement_panel: false,
            achievement_panel_anim: ExponentialAnimation::new(0.0),
            achievement_selected: 0,
            achievement_select_anim: ExponentialAnimation::new(0.0),
            achievement_select_anim_target: None,
            achievement_scroll_offset: ExponentialAnimation::new(0.0),
        }
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn home_settings_selected(&self) -> bool {
        self.home_settings_selected
    }

    pub fn home_power_selected(&self) -> bool {
        self.home_power_selected
    }

    pub fn home_top_button_selected(&self) -> bool {
        self.home_settings_selected || self.home_power_selected
    }

    pub fn home_settings_focus_anim(&self) -> f32 {
        self.home_settings_focus_anim.value()
    }

    pub fn home_power_focus_anim(&self) -> f32 {
        self.home_power_focus_anim.value()
    }

    pub fn home_top_focus_anim(&self) -> f32 {
        self.home_settings_focus_anim
            .value()
            .max(self.home_power_focus_anim.value())
    }

    pub fn show_achievement_panel(&self) -> bool {
        self.show_achievement_panel
    }

    pub fn show_power_menu(&self) -> bool {
        self.show_power_menu
    }

    pub fn show_settings_page(&self) -> bool {
        self.show_settings_page
    }

    pub fn cover_nav_dir(&self) -> f32 {
        self.cover_nav_dir
    }

    pub fn select_anim(&self) -> f32 {
        self.select_anim.value()
    }

    pub fn scroll_offset(&self) -> f32 {
        self.scroll_offset.value()
    }

    pub fn is_fast_scrolling(&self) -> bool {
        (self.selected as f32 - self.scroll_offset.value()).abs() > 0.1
    }

    pub fn summary_cards_visibility(&self) -> f32 {
        self.summary_cards_visibility.value()
    }

    pub fn wake_anim(&self) -> f32 {
        self.wake_anim.value()
    }

    pub fn power_menu_anim(&self) -> f32 {
        self.power_menu_anim.value()
    }

    pub fn power_menu_select_anim(&self) -> f32 {
        self.power_menu_select_anim.value()
    }

    pub fn power_menu_scroll_offset(&self) -> f32 {
        self.power_menu_scroll_offset
    }

    pub fn settings_page_anim(&self) -> f32 {
        self.settings_page_anim.value()
    }

    pub fn settings_submenu_anim(&self) -> f32 {
        self.settings_submenu_anim.value()
    }

    pub fn settings_select_anim(&self) -> f32 {
        self.settings_select_anim.value()
    }

    pub fn settings_focus_key(&self) -> Option<u16> {
        if self.show_settings_page {
            Some(self.settings_selection_key())
        } else {
            None
        }
    }

    pub fn settings_section_index(&self) -> usize {
        self.settings_section.index()
    }

    pub fn settings_in_submenu(&self) -> bool {
        self.settings_in_submenu
    }

    pub fn settings_selected_item_index(&self) -> usize {
        match self.settings_section {
            SettingsSection::System => self.settings_system_selected,
            SettingsSection::Screen => self.settings_screen_selected,
            SettingsSection::Apps => self.settings_apps_selected,
            SettingsSection::CloseApp => 0,
        }
    }

    pub fn settings_screen_resolution_dropdown_open(&self) -> bool {
        self.settings_screen_dropdown == Some(ScreenDropdown::Resolution)
    }

    pub fn settings_screen_refresh_dropdown_open(&self) -> bool {
        self.settings_screen_dropdown == Some(ScreenDropdown::RefreshRate)
    }

    pub fn settings_screen_scale_dropdown_open(&self) -> bool {
        self.settings_screen_dropdown == Some(ScreenDropdown::Scale)
    }

    pub fn settings_screen_dropdown_selected_index(&self) -> usize {
        self.settings_screen_dropdown_selected
    }

    pub fn sync_screen_settings(
        &mut self,
        resolution_count: usize,
        selected_resolution_index: usize,
        refresh_count: usize,
        selected_refresh_index: usize,
        scale_count: usize,
        selected_scale_index: usize,
    ) {
        self.settings_screen_resolution_count = resolution_count.max(1);
        self.settings_screen_refresh_count = refresh_count.max(1);
        self.settings_screen_scale_count = scale_count.max(1);
        self.settings_screen_current_resolution =
            selected_resolution_index.min(self.settings_screen_resolution_count.saturating_sub(1));
        self.settings_screen_current_refresh =
            selected_refresh_index.min(self.settings_screen_refresh_count.saturating_sub(1));
        self.settings_screen_current_scale =
            selected_scale_index.min(self.settings_screen_scale_count.saturating_sub(1));

        match self.settings_screen_dropdown {
            Some(ScreenDropdown::Resolution) => {
                self.settings_screen_dropdown_selected = self
                    .settings_screen_dropdown_selected
                    .min(self.settings_screen_resolution_count.saturating_sub(1));
            }
            Some(ScreenDropdown::RefreshRate) => {
                self.settings_screen_dropdown_selected = self
                    .settings_screen_dropdown_selected
                    .min(self.settings_screen_refresh_count.saturating_sub(1));
            }
            Some(ScreenDropdown::Scale) => {
                self.settings_screen_dropdown_selected = self
                    .settings_screen_dropdown_selected
                    .min(self.settings_screen_scale_count.saturating_sub(1));
            }
            None => {}
        }
    }

    pub fn power_menu_layout(&self) -> &PowerMenuLayout {
        &self.power_menu_layout
    }

    pub fn achievement_panel_anim(&self) -> f32 {
        self.achievement_panel_anim.value()
    }

    pub fn achievement_selected(&self) -> usize {
        self.achievement_selected
    }

    pub fn achievement_select_anim(&self) -> f32 {
        self.achievement_select_anim.value()
    }

    pub fn achievement_scroll_offset(&self) -> f32 {
        self.achievement_scroll_offset.value()
    }

    pub fn prepare_wake_animation(&mut self) {
        self.wake_anim.set_immediate(0.0);
    }

    pub fn start_wake_animation(&mut self, now: Instant) {
        self.wake_anim.restart(0.0, 1.0, 8.0, now);
    }

    pub fn force_select(&mut self, selected: usize) {
        self.selected = selected;
        self.clear_home_top_button_selection();
        self.home_settings_focus_anim.set_immediate(0.0);
        self.home_power_focus_anim.set_immediate(0.0);
        self.cover_nav_dir = 0.0;
        self.select_anim.set_immediate(0.0);
        self.select_anim_target = None;
        self.scroll_offset.set_immediate(selected as f32);
        self.show_settings_page = false;
        self.settings_page_anim.set_immediate(0.0);
        self.settings_select_anim.set_immediate(0.0);
        self.settings_select_anim_target = None;
        self.reset_settings_navigation();
        self.show_achievement_panel = false;
        self.achievement_panel_anim.set_immediate(0.0);
        self.reset_achievement_selection();
    }

    /// Relocate the current selection (e.g. after a list reorder that moved the
    /// currently-selected item to a different index) without restarting the
    /// selection animation or affecting achievement panel state.
    pub fn relocate_selection(&mut self, selected: usize) {
        self.selected = selected;
        // Keep `select_anim_target` in sync with the new index so the next
        // animation tick doesn't think the selection just changed and reset
        // `select_anim` back to 0 (which would re-shrink the title/badge).
        self.select_anim_target = Some(selected);
        self.scroll_offset.set_immediate(selected as f32);
    }

    pub fn open_power_menu(&mut self, layout: PowerMenuLayout) {
        let default_selected = layout.default_selected();
        self.power_menu_layout = layout;
        self.power_menu_anim.set_immediate(0.0);
        self.power_menu_selected = default_selected;
        self.power_menu_select_anim.set_immediate(0.0);
        self.power_menu_select_anim_target = None;
        self.power_menu_scroll_offset = 0.0;
        self.show_power_menu = true;
    }

    pub fn handle_action(
        &mut self,
        action: &ControllerAction,
        games_len: usize,
        can_open_achievement_panel: bool,
        achievement_len: usize,
    ) -> PageActionResult {
        let mut result = PageActionResult {
            open_achievement_panel: false,
            open_power_menu: false,
            reveal_hidden_achievement: false,
            refresh_achievements: false,
            cycle_display_mode_setting: false,
            cycle_language_setting: false,
            toggle_launch_on_startup: false,
            toggle_background_home_wake: false,
            toggle_controller_vibration_feedback: false,
            toggle_idle_frame_rate_reduction: false,
            toggle_detect_steam_games: false,
            toggle_detect_epic_games: false,
            toggle_detect_xbox_games: false,
            launch_selected: false,
            launch_external_app: None,
            selected_changed: false,
            close_frame: false,
            send_app_to_background: false,
            screen_settings_action: None,
            power_action: None,
        };

        if self.show_power_menu {
            match action {
                ControllerAction::Left | ControllerAction::Right => {}
                ControllerAction::Up => {
                    let next = self.power_menu_layout.move_up(self.power_menu_selected);
                    if next != self.power_menu_selected {
                        self.power_menu_selected = next;
                    }
                }
                ControllerAction::Down => {
                    let next = self.power_menu_layout.move_down(self.power_menu_selected);
                    if next != self.power_menu_selected {
                        self.power_menu_selected = next;
                    }
                }
                ControllerAction::Launch => {
                    match self.power_menu_layout.option_at(self.power_menu_selected) {
                        Some(PowerMenuOption::Sleep) => {
                            self.close_power_menu();
                            result.power_action = Some(PowerAction::Sleep);
                        }
                        Some(PowerMenuOption::Reboot) => {
                            self.close_power_menu();
                            result.power_action = Some(PowerAction::Reboot);
                        }
                        Some(PowerMenuOption::Shutdown) => {
                            self.close_power_menu();
                            result.power_action = Some(PowerAction::Shutdown);
                        }
                        None => {}
                    }
                }
                ControllerAction::Quit => {
                    self.close_power_menu();
                }
                _ => {}
            }
            return result;
        }

        if self.show_settings_page {
            match action {
                ControllerAction::Left => {}
                ControllerAction::Right => {
                    if self.settings_in_submenu {
                        // No-op: submenu actions are triggered with Launch only.
                    }
                }
                ControllerAction::Up => {
                    if self.settings_in_submenu {
                        if self.settings_section == SettingsSection::Screen
                            && self.settings_screen_dropdown.is_some()
                        {
                            self.move_screen_dropdown(false);
                        } else {
                            self.move_settings_item(false);
                        }
                    } else {
                        self.settings_section = self.settings_section.previous();
                    }
                }
                ControllerAction::Down => {
                    if self.settings_in_submenu {
                        if self.settings_section == SettingsSection::Screen
                            && self.settings_screen_dropdown.is_some()
                        {
                            self.move_screen_dropdown(true);
                        } else {
                            self.move_settings_item(true);
                        }
                    } else {
                        self.settings_section = self.settings_section.next();
                    }
                }
                ControllerAction::Launch => {
                    if self.settings_in_submenu {
                        match self.settings_section {
                            SettingsSection::System => match self.settings_system_selected {
                                0 => result.toggle_detect_steam_games = true,
                                1 => result.toggle_detect_epic_games = true,
                                2 => result.toggle_detect_xbox_games = true,
                                3 => result.toggle_background_home_wake = true,
                                4 => result.toggle_controller_vibration_feedback = true,
                                5 => result.toggle_idle_frame_rate_reduction = true,
                                6 => result.cycle_display_mode_setting = true,
                                7 => result.cycle_language_setting = true,
                                _ => result.toggle_launch_on_startup = true,
                            },
                            SettingsSection::Screen => match self.settings_screen_dropdown {
                                Some(ScreenDropdown::Resolution) => {
                                    result.screen_settings_action =
                                        Some(ScreenSettingsAction::SelectResolution(
                                            self.settings_screen_dropdown_selected,
                                        ));
                                    self.settings_screen_dropdown = None;
                                }
                                Some(ScreenDropdown::RefreshRate) => {
                                    result.screen_settings_action =
                                        Some(ScreenSettingsAction::SelectRefreshRate(
                                            self.settings_screen_dropdown_selected,
                                        ));
                                    self.settings_screen_dropdown = None;
                                }
                                Some(ScreenDropdown::Scale) => {
                                    result.screen_settings_action =
                                        Some(ScreenSettingsAction::SelectScale(
                                            self.settings_screen_dropdown_selected,
                                        ));
                                    self.settings_screen_dropdown = None;
                                }
                                None => {
                                    self.settings_screen_dropdown =
                                        Some(if self.settings_screen_selected == 0 {
                                            self.settings_screen_dropdown_selected =
                                                self.settings_screen_current_resolution;
                                            ScreenDropdown::Resolution
                                        } else if self.settings_screen_selected == 1 {
                                            self.settings_screen_dropdown_selected =
                                                self.settings_screen_current_refresh;
                                            ScreenDropdown::RefreshRate
                                        } else {
                                            self.settings_screen_dropdown_selected =
                                                self.settings_screen_current_scale;
                                            ScreenDropdown::Scale
                                        });
                                }
                            },
                            SettingsSection::Apps => {
                                result.launch_external_app =
                                    Some(match self.settings_apps_selected {
                                        0 => ExternalAppKind::DlssSwapper,
                                        _ => ExternalAppKind::NvidiaApp,
                                    });
                            }
                            SettingsSection::CloseApp => {}
                        }
                    } else {
                        match self.settings_section {
                            SettingsSection::System
                            | SettingsSection::Screen
                            | SettingsSection::Apps => {
                                self.settings_in_submenu = true;
                            }
                            SettingsSection::CloseApp => {
                                self.close_settings_page();
                                result.close_frame = true;
                            }
                        }
                    }
                }
                ControllerAction::Quit => {
                    if self.settings_in_submenu {
                        if self.settings_section == SettingsSection::Screen
                            && self.settings_screen_dropdown.is_some()
                        {
                            self.settings_screen_dropdown = None;
                        } else {
                            self.settings_in_submenu = false;
                        }
                    } else {
                        self.close_settings_page();
                    }
                }
                _ => {}
            }
            return result;
        }

        if self.show_achievement_panel {
            match action {
                ControllerAction::Up => {
                    if self.achievement_selected > 0 {
                        self.achievement_selected -= 1;
                    } else {
                        self.close_achievement_panel();
                    }
                }
                ControllerAction::Down => {
                    if self.achievement_selected + 1 < achievement_len {
                        self.achievement_selected += 1;
                    }
                }
                ControllerAction::Quit => {
                    self.close_achievement_panel();
                }
                ControllerAction::Launch => {
                    result.reveal_hidden_achievement = true;
                }
                ControllerAction::Refresh => {
                    result.refresh_achievements = true;
                }
                _ => {}
            }
            return result;
        }

        if self.home_power_selected {
            match action {
                ControllerAction::Launch => {
                    result.open_power_menu = true;
                }
                ControllerAction::Left => {
                    self.clear_home_top_button_selection();
                }
                ControllerAction::Down => {
                    self.clear_home_top_button_selection();
                }
                ControllerAction::Right | ControllerAction::Settings => {
                    self.select_home_settings_button();
                }
                ControllerAction::Quit => {
                    self.clear_home_top_button_selection();
                }
                ControllerAction::Refresh | ControllerAction::Up => {}
            }
            return result;
        }

        if self.home_settings_selected {
            match action {
                ControllerAction::Left => {
                    self.select_home_power_button();
                }
                ControllerAction::Down => {
                    self.clear_home_top_button_selection();
                }
                ControllerAction::Quit => {
                    self.clear_home_top_button_selection();
                }
                ControllerAction::Launch => {
                    self.open_settings_page();
                }
                ControllerAction::Right
                | ControllerAction::Refresh
                | ControllerAction::Settings
                | ControllerAction::Up => {}
            }
            return result;
        }

        match action {
            ControllerAction::Left => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.cover_nav_dir = -1.0;
                    self.reset_achievement_selection();
                    result.selected_changed = true;
                }
            }
            ControllerAction::Right => {
                if self.selected + 1 < games_len {
                    self.selected += 1;
                    self.cover_nav_dir = 1.0;
                    self.reset_achievement_selection();
                    result.selected_changed = true;
                }
            }
            ControllerAction::Down => {
                if can_open_achievement_panel {
                    self.show_achievement_panel = true;
                    self.reset_achievement_selection();
                    result.open_achievement_panel = true;
                }
            }
            ControllerAction::Launch => {
                result.launch_selected = true;
            }
            ControllerAction::Refresh => {}
            ControllerAction::Settings => {
                self.select_home_settings_button();
            }
            ControllerAction::Quit => {}
            ControllerAction::Up => {
                self.select_home_power_button();
            }
        }

        result
    }

    pub fn tick_animations(&mut self, ctx: &egui::Context, now: Instant) {
        if self.wake_anim.update(now, ANIMATION_EPSILON) {
            ctx.request_repaint();
        }

        if self.select_anim_target != Some(self.selected) {
            self.select_anim_target = Some(self.selected);
            self.select_anim.restart(0.0, 1.0, 10.0, now);
        }
        if self.select_anim.update(now, ANIMATION_EPSILON) {
            ctx.request_repaint();
        }

        let panel_target = if self.show_achievement_panel {
            1.0
        } else {
            0.0
        };
        self.achievement_panel_anim
            .animate_to(panel_target, 5.4, now, ANIMATION_EPSILON);
        if self.achievement_panel_anim.update(now, ANIMATION_EPSILON) {
            ctx.request_repaint();
        }

        let home_settings_target = if self.home_settings_selected {
            1.0
        } else {
            0.0
        };
        self.home_settings_focus_anim.animate_to(
            home_settings_target,
            11.0,
            now,
            ANIMATION_EPSILON,
        );
        if self.home_settings_focus_anim.update(now, ANIMATION_EPSILON) {
            ctx.request_repaint();
        }

        let home_power_target = if self.home_power_selected { 1.0 } else { 0.0 };
        self.home_power_focus_anim
            .animate_to(home_power_target, 11.0, now, ANIMATION_EPSILON);
        if self.home_power_focus_anim.update(now, ANIMATION_EPSILON) {
            ctx.request_repaint();
        }

        if self.show_power_menu {
            self.power_menu_anim
                .animate_to(1.0, 4.8, now, ANIMATION_EPSILON);
            if self.power_menu_anim.update(now, ANIMATION_EPSILON) {
                ctx.request_repaint();
            }

            let power_menu_select_target = Some(self.power_menu_selected);
            if self.power_menu_select_anim_target != power_menu_select_target {
                self.power_menu_select_anim_target = power_menu_select_target;
                self.power_menu_select_anim.restart(0.0, 1.0, 11.0, now);
            }
            if self.power_menu_select_anim.update(now, ANIMATION_EPSILON) {
                ctx.request_repaint();
            }

            self.power_menu_scroll_offset = self.power_menu_selected as f32;
        } else {
            self.power_menu_anim
                .animate_to(0.0, 6.5, now, ANIMATION_EPSILON);
            if self.power_menu_anim.update(now, ANIMATION_EPSILON) {
                ctx.request_repaint();
            } else {
                self.power_menu_selected = self.power_menu_layout.clamp_selected(0);
                self.power_menu_select_anim.set_immediate(0.0);
                self.power_menu_select_anim_target = None;
            }
            self.power_menu_scroll_offset = self.power_menu_selected as f32;
        }

        let settings_target = if self.show_settings_page { 1.0 } else { 0.0 };
        let settings_diff = settings_target - self.settings_page_anim.value_at(now);
        let settings_anim_speed = if settings_diff < 0.0 {
            SETTINGS_PAGE_EXIT_ANIM_SPEED
        } else {
            SETTINGS_PAGE_ENTER_ANIM_SPEED
        };
        self.settings_page_anim.animate_to(
            settings_target,
            settings_anim_speed,
            now,
            ANIMATION_EPSILON,
        );
        if self.settings_page_anim.update(now, ANIMATION_EPSILON) {
            ctx.request_repaint();
        }

        let submenu_target = if self.show_settings_page && self.settings_in_submenu {
            1.0
        } else {
            0.0
        };
        let submenu_diff = submenu_target - self.settings_submenu_anim.value_at(now);
        let submenu_anim_speed = if submenu_diff < 0.0 {
            SETTINGS_SUBMENU_EXIT_ANIM_SPEED
        } else {
            SETTINGS_SUBMENU_ENTER_ANIM_SPEED
        };
        self.settings_submenu_anim.animate_to(
            submenu_target,
            submenu_anim_speed,
            now,
            ANIMATION_EPSILON,
        );
        if self.settings_submenu_anim.update(now, ANIMATION_EPSILON) {
            ctx.request_repaint();
        }

        let settings_select_target = if self.show_settings_page {
            Some(self.settings_selection_key())
        } else {
            None
        };
        if self.settings_select_anim_target != settings_select_target {
            let current_settings_select_anim = self.settings_select_anim.value_at(now);
            self.settings_select_anim_target = settings_select_target;
            if settings_select_target.is_some() {
                self.settings_select_anim
                    .restart(current_settings_select_anim, 1.0, 11.0, now);
            } else {
                self.settings_select_anim.set_immediate(0.0);
            }
        }
        if settings_select_target.is_some()
            && self.settings_select_anim.update(now, ANIMATION_EPSILON)
        {
            ctx.request_repaint();
        }

        let scroll_target = self.selected as f32;
        self.scroll_offset
            .animate_to(scroll_target, 14.0, now, ANIMATION_EPSILON);
        if self.scroll_offset.update(now, ANIMATION_EPSILON) {
            ctx.request_repaint();
        }

        let summary_cards_target = if self.is_fast_scrolling() { 0.0 } else { 1.0 };
        let summary_cards_diff = summary_cards_target - self.summary_cards_visibility.value_at(now);
        let fade_speed = if summary_cards_diff < 0.0 { 18.0 } else { 10.0 };
        self.summary_cards_visibility.animate_to(
            summary_cards_target,
            fade_speed,
            now,
            ANIMATION_EPSILON,
        );
        if self.summary_cards_visibility.update(now, ANIMATION_EPSILON) {
            ctx.request_repaint();
        }

        if self.achievement_select_anim_target != Some(self.achievement_selected) {
            self.achievement_select_anim_target = Some(self.achievement_selected);
            self.achievement_select_anim.restart(0.0, 1.0, 10.0, now);
        }
        if self.achievement_select_anim.update(now, ANIMATION_EPSILON) {
            ctx.request_repaint();
        }

        let ach_target = self.achievement_selected.saturating_sub(2) as f32;
        self.achievement_scroll_offset
            .animate_to(ach_target, 14.0, now, ANIMATION_EPSILON);
        if self
            .achievement_scroll_offset
            .update(now, ANIMATION_EPSILON)
        {
            ctx.request_repaint();
        }
    }

    fn reset_achievement_selection(&mut self) {
        self.achievement_selected = 0;
        self.achievement_select_anim.set_immediate(0.0);
        self.achievement_select_anim_target = None;
        self.achievement_scroll_offset.set_immediate(0.0);
    }

    fn close_achievement_panel(&mut self) {
        self.show_achievement_panel = false;
        self.reset_achievement_selection();
    }

    fn close_power_menu(&mut self) {
        self.show_power_menu = false;
    }

    fn open_settings_page(&mut self) {
        self.close_power_menu();
        self.close_achievement_panel();
        self.select_home_settings_button();
        self.show_settings_page = true;
        self.settings_page_anim
            .set_immediate(SETTINGS_PAGE_ENTER_INITIAL_PROGRESS);
        self.settings_submenu_anim.set_immediate(0.0);
        self.reset_settings_navigation();
    }

    fn close_settings_page(&mut self) {
        self.show_settings_page = false;
        self.settings_in_submenu = false;
        self.settings_screen_dropdown = None;
    }

    fn reset_settings_navigation(&mut self) {
        self.settings_section = SettingsSection::System;
        self.settings_in_submenu = false;
        self.settings_system_selected = 0;
        self.settings_screen_selected = 0;
        self.settings_screen_dropdown = None;
        self.settings_screen_dropdown_selected = 0;
        self.settings_apps_selected = 0;
    }

    fn clear_home_top_button_selection(&mut self) {
        self.home_settings_selected = false;
        self.home_power_selected = false;
    }

    fn select_home_settings_button(&mut self) {
        self.home_settings_selected = true;
        self.home_power_selected = false;
    }

    fn select_home_power_button(&mut self) {
        self.home_settings_selected = false;
        self.home_power_selected = true;
    }

    fn settings_selection_key(&self) -> u16 {
        if self.settings_in_submenu {
            match self.settings_section {
                SettingsSection::System => 10 + self.settings_system_selected as u16,
                SettingsSection::Screen => match self.settings_screen_dropdown {
                    Some(ScreenDropdown::Resolution) => {
                        100 + self.settings_screen_dropdown_selected as u16
                    }
                    Some(ScreenDropdown::RefreshRate) => {
                        300 + self.settings_screen_dropdown_selected as u16
                    }
                    Some(ScreenDropdown::Scale) => {
                        500 + self.settings_screen_dropdown_selected as u16
                    }
                    None => 20 + self.settings_screen_selected as u16,
                },
                SettingsSection::Apps => 700 + self.settings_apps_selected as u16,
                SettingsSection::CloseApp => 800,
            }
        } else {
            self.settings_section.index() as u16
        }
    }

    fn move_settings_item(&mut self, down: bool) {
        if !matches!(
            self.settings_section,
            SettingsSection::System | SettingsSection::Screen | SettingsSection::Apps
        ) {
            return;
        }

        let selected = match self.settings_section {
            SettingsSection::System => &mut self.settings_system_selected,
            SettingsSection::Screen => &mut self.settings_screen_selected,
            SettingsSection::Apps => &mut self.settings_apps_selected,
            SettingsSection::CloseApp => return,
        };
        let max_index = match self.settings_section {
            SettingsSection::System => 8,
            SettingsSection::Screen => 2,
            SettingsSection::Apps => 1,
            SettingsSection::CloseApp => 0,
        };

        if down {
            *selected = (*selected + 1).min(max_index);
        } else {
            *selected = selected.saturating_sub(1);
        }
    }

    fn move_screen_dropdown(&mut self, down: bool) {
        let max_index = match self.settings_screen_dropdown {
            Some(ScreenDropdown::Resolution) => {
                self.settings_screen_resolution_count.saturating_sub(1)
            }
            Some(ScreenDropdown::RefreshRate) => {
                self.settings_screen_refresh_count.saturating_sub(1)
            }
            Some(ScreenDropdown::Scale) => self.settings_screen_scale_count.saturating_sub(1),
            None => return,
        };

        if down {
            self.settings_screen_dropdown_selected =
                (self.settings_screen_dropdown_selected + 1).min(max_index);
        } else {
            self.settings_screen_dropdown_selected =
                self.settings_screen_dropdown_selected.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::super::power::PowerMenuLayout;
    use super::{
        PageState, PowerAction, ScreenSettingsAction, SETTINGS_PAGE_ENTER_ANIM_SPEED,
        SETTINGS_PAGE_ENTER_INITIAL_PROGRESS, SETTINGS_SUBMENU_ENTER_ANIM_SPEED,
    };
    use crate::animation::scale_seconds;
    use crate::input::ControllerAction;
    use crate::system::external_apps::ExternalAppKind;

    fn open_settings_page(page: &mut PageState) {
        let _ = page.handle_action(&ControllerAction::Settings, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        page.sync_screen_settings(2, 0, 3, 1, 4, 1);
    }

    fn tick_animation_frame(page: &mut PageState, ctx: &eframe::egui::Context, now: &mut Instant) {
        *now += Duration::from_secs_f32(1.0 / 60.0);
        page.tick_animations(ctx, *now);
    }

    #[cfg(target_os = "windows")]
    fn power_menu_layout() -> PowerMenuLayout {
        PowerMenuLayout::new(true)
    }

    #[cfg(not(target_os = "windows"))]
    fn power_menu_layout() -> PowerMenuLayout {
        PowerMenuLayout::new(false)
    }

    #[test]
    fn up_on_first_achievement_returns_to_game_list() {
        let mut page = PageState::new();

        let open_result = page.handle_action(&ControllerAction::Down, 3, true, 4);
        assert!(open_result.open_achievement_panel);
        assert!(page.show_achievement_panel());
        assert_eq!(page.achievement_selected(), 0);

        let up_result = page.handle_action(&ControllerAction::Up, 3, true, 4);
        assert!(!up_result.open_achievement_panel);
        assert!(!up_result.selected_changed);
        assert!(!page.show_achievement_panel());
        assert_eq!(page.achievement_selected(), 0);
    }

    #[test]
    fn quit_on_main_page_does_not_close_frame() {
        let mut page = PageState::new();

        let result = page.handle_action(&ControllerAction::Quit, 3, true, 4);

        assert!(!result.close_frame);
        assert!(!page.show_power_menu());
    }

    #[test]
    fn quit_on_power_menu_keeps_exit_animation_running() {
        let mut page = PageState::new();
        let ctx = eframe::egui::Context::default();
        let mut now = Instant::now();
        page.open_power_menu(power_menu_layout());
        tick_animation_frame(&mut page, &ctx, &mut now);
        tick_animation_frame(&mut page, &ctx, &mut now);
        assert!(page.power_menu_anim() > 0.0);

        let anim_before_close = page.power_menu_anim();

        let _ = page.handle_action(&ControllerAction::Quit, 3, true, 4);

        assert!(!page.show_power_menu());
        assert_eq!(page.power_menu_anim(), anim_before_close);

        for _ in 0..10 {
            tick_animation_frame(&mut page, &ctx, &mut now);
        }
        assert!(page.power_menu_anim() < anim_before_close);
        assert!(page.power_menu_anim() > 0.0);
    }

    #[test]
    fn up_on_main_page_selects_home_power_button() {
        let mut page = PageState::new();

        let result = page.handle_action(&ControllerAction::Up, 3, true, 4);

        assert!(!result.selected_changed);
        assert!(page.home_power_selected());
        assert!(!page.home_settings_selected());
        assert!(!page.show_settings_page());
    }

    #[test]
    fn settings_action_selects_home_settings_button_without_opening_page() {
        let mut page = PageState::new();

        let result = page.handle_action(&ControllerAction::Settings, 3, true, 4);

        assert!(!result.launch_selected);
        assert!(page.home_settings_selected());
        assert!(!page.show_settings_page());
    }

    #[test]
    fn left_from_home_settings_button_selects_power_button() {
        let mut page = PageState::new();
        let _ = page.handle_action(&ControllerAction::Settings, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Left, 3, true, 4);

        assert!(!result.open_power_menu);
        assert!(!page.home_settings_selected());
        assert!(page.home_power_selected());
        assert!(!page.show_achievement_panel());
    }

    #[test]
    fn quit_from_home_settings_button_returns_focus_to_game_list() {
        let mut page = PageState::new();
        let _ = page.handle_action(&ControllerAction::Settings, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Quit, 3, true, 4);

        assert!(!result.close_frame);
        assert!(!page.home_settings_selected());
        assert!(!page.show_settings_page());
    }

    #[test]
    fn launch_from_home_settings_button_opens_settings_page() {
        let mut page = PageState::new();
        let _ = page.handle_action(&ControllerAction::Settings, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(!result.launch_selected);
        assert!(page.home_settings_selected());
        assert!(page.show_settings_page());
        assert!(!page.settings_in_submenu());
    }

    #[test]
    fn launch_from_home_power_button_opens_power_menu() {
        let mut page = PageState::new();
        let _ = page.handle_action(&ControllerAction::Up, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(result.open_power_menu);
        assert!(page.home_power_selected());
        assert!(!page.show_settings_page());
    }

    #[test]
    fn left_from_home_power_button_returns_focus_to_game_list() {
        let mut page = PageState::new();
        let _ = page.handle_action(&ControllerAction::Up, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Left, 3, true, 4);

        assert!(!result.open_power_menu);
        assert!(!page.home_power_selected());
        assert!(!page.home_settings_selected());
        assert!(!page.show_achievement_panel());
    }

    #[test]
    fn down_from_home_settings_button_returns_focus_to_game_list() {
        let mut page = PageState::new();
        let _ = page.handle_action(&ControllerAction::Settings, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Down, 3, true, 4);

        assert!(!result.open_power_menu);
        assert!(!page.home_settings_selected());
        assert!(!page.home_power_selected());
    }

    #[test]
    fn down_from_home_power_button_returns_focus_to_game_list() {
        let mut page = PageState::new();
        let _ = page.handle_action(&ControllerAction::Up, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Down, 3, true, 4);

        assert!(!result.open_power_menu);
        assert!(!result.open_achievement_panel);
        assert!(!page.home_settings_selected());
        assert!(!page.home_power_selected());
        assert!(!page.show_achievement_panel());
    }

    #[test]
    fn down_on_main_page_opens_achievement_panel_when_available() {
        let mut page = PageState::new();

        let result = page.handle_action(&ControllerAction::Down, 3, true, 4);

        assert!(!result.open_power_menu);
        assert!(result.open_achievement_panel);
        assert!(!result.selected_changed);
        assert!(!page.home_power_selected());
        assert!(!page.home_settings_selected());
        assert!(page.show_achievement_panel());
    }

    #[test]
    fn down_on_main_page_without_achievements_keeps_focus_on_game_list() {
        let mut page = PageState::new();

        let result = page.handle_action(&ControllerAction::Down, 3, false, 4);

        assert!(!result.open_power_menu);
        assert!(!result.open_achievement_panel);
        assert!(!result.selected_changed);
        assert!(!page.home_settings_selected());
        assert!(!page.home_power_selected());
        assert!(!page.show_achievement_panel());
    }

    #[test]
    fn right_from_home_power_button_returns_to_settings_button() {
        let mut page = PageState::new();
        let _ = page.handle_action(&ControllerAction::Up, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Right, 3, true, 4);

        assert!(!result.open_power_menu);
        assert!(page.home_settings_selected());
        assert!(!page.home_power_selected());
    }

    #[test]
    fn right_from_home_settings_button_keeps_settings_selected() {
        let mut page = PageState::new();
        let _ = page.handle_action(&ControllerAction::Up, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Right, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Right, 3, true, 4);

        assert!(!result.open_power_menu);
        assert!(!result.open_achievement_panel);
        assert!(page.home_settings_selected());
        assert!(!page.home_power_selected());
        assert!(!page.show_achievement_panel());
    }

    #[test]
    fn home_top_focus_animation_tracks_selection_handoff() {
        let mut page = PageState::new();
        let ctx = eframe::egui::Context::default();
        let mut now = Instant::now();

        let _ = page.handle_action(&ControllerAction::Settings, 3, true, 4);
        tick_animation_frame(&mut page, &ctx, &mut now);
        tick_animation_frame(&mut page, &ctx, &mut now);
        assert!(page.home_settings_focus_anim() > 0.0);
        assert_eq!(page.home_power_focus_anim(), 0.0);

        let anim_while_selected = page.home_settings_focus_anim();
        let _ = page.handle_action(&ControllerAction::Left, 3, true, 4);
        for _ in 0..10 {
            tick_animation_frame(&mut page, &ctx, &mut now);
        }

        assert!(!page.home_settings_selected());
        assert!(page.home_power_selected());
        assert!(page.home_settings_focus_anim() < anim_while_selected);
        assert!(page.home_power_focus_anim() > 0.0);
    }

    #[test]
    fn horizontal_navigation_is_disabled_while_home_settings_button_is_selected() {
        let mut page = PageState::new();
        let _ = page.handle_action(&ControllerAction::Settings, 3, true, 4);

        let left_result = page.handle_action(&ControllerAction::Left, 3, true, 4);
        let right_result = page.handle_action(&ControllerAction::Right, 3, true, 4);

        assert!(!left_result.selected_changed);
        assert!(!right_result.selected_changed);
        assert_eq!(page.selected(), 0);
        assert!(page.home_settings_selected());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn power_menu_navigates_to_shutdown_option_with_down() {
        let mut page = PageState::new();
        page.open_power_menu(power_menu_layout());

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        assert_eq!(page.power_menu_selected, 1);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn power_menu_defaults_to_sleep_when_power_options_exist() {
        let mut page = PageState::new();
        page.open_power_menu(power_menu_layout());

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(result.power_action, Some(PowerAction::Sleep));
        assert_eq!(result.launch_external_app, None);
        assert!(!result.close_frame);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn left_from_sleep_keeps_selection_on_sleep() {
        let mut page = PageState::new();
        page.open_power_menu(power_menu_layout());

        let _ = page.handle_action(&ControllerAction::Left, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(result.power_action, Some(PowerAction::Sleep));
        assert_eq!(result.launch_external_app, None);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn up_from_sleep_keeps_selection_on_only_row() {
        let mut page = PageState::new();
        page.open_power_menu(power_menu_layout());

        let _ = page.handle_action(&ControllerAction::Up, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(result.power_action, Some(PowerAction::Sleep));
        assert_eq!(result.launch_external_app, None);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn power_menu_selection_animation_resets_when_selection_changes() {
        let mut page = PageState::new();
        let ctx = eframe::egui::Context::default();
        let mut now = Instant::now();
        page.open_power_menu(power_menu_layout());

        tick_animation_frame(&mut page, &ctx, &mut now);
        tick_animation_frame(&mut page, &ctx, &mut now);
        let first_anim = page.power_menu_select_anim();
        assert!(first_anim > 0.0);

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        tick_animation_frame(&mut page, &ctx, &mut now);
        tick_animation_frame(&mut page, &ctx, &mut now);

        assert!(page.power_menu_select_anim() > 0.0);
        assert!(page.power_menu_select_anim() <= first_anim + 0.001);
    }

    #[test]
    fn settings_action_selects_home_settings_button_from_main_view() {
        let mut page = PageState::new();

        let result = page.handle_action(&ControllerAction::Settings, 3, true, 4);

        assert!(!result.toggle_launch_on_startup);
        assert!(page.home_settings_selected());
        assert!(!page.show_settings_page());
    }

    #[test]
    fn launch_in_settings_page_toggles_steam_game_detection_without_closing_page() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(!result.toggle_launch_on_startup);
        assert!(!result.toggle_background_home_wake);
        assert!(result.toggle_detect_steam_games);
        assert!(page.show_settings_page());
    }

    #[test]
    fn launch_on_second_system_setting_toggles_epic_game_detection() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(!result.toggle_detect_steam_games);
        assert!(result.toggle_detect_epic_games);
        assert!(!result.toggle_detect_xbox_games);
        assert!(page.show_settings_page());
    }

    #[test]
    fn launch_on_third_system_setting_toggles_xbox_game_detection() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(!result.toggle_detect_steam_games);
        assert!(!result.toggle_detect_epic_games);
        assert!(result.toggle_detect_xbox_games);
        assert!(page.show_settings_page());
    }

    #[test]
    fn launch_on_fourth_system_setting_toggles_background_home_wake() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(!result.toggle_launch_on_startup);
        assert!(result.toggle_background_home_wake);
        assert!(!result.toggle_controller_vibration_feedback);
        assert!(page.show_settings_page());
    }

    #[test]
    fn launch_on_fifth_system_setting_toggles_controller_vibration() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(!result.toggle_launch_on_startup);
        assert!(!result.toggle_background_home_wake);
        assert!(result.toggle_controller_vibration_feedback);
        assert!(!result.cycle_language_setting);
        assert!(page.show_settings_page());
    }

    #[test]
    fn launch_on_sixth_system_setting_toggles_idle_frame_rate_reduction() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(!result.toggle_launch_on_startup);
        assert!(!result.toggle_background_home_wake);
        assert!(!result.toggle_controller_vibration_feedback);
        assert!(result.toggle_idle_frame_rate_reduction);
        assert!(page.show_settings_page());
    }

    #[test]
    fn launch_on_seventh_system_setting_cycles_display_mode_setting() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(!result.toggle_launch_on_startup);
        assert!(!result.toggle_background_home_wake);
        assert!(!result.toggle_controller_vibration_feedback);
        assert!(!result.toggle_idle_frame_rate_reduction);
        assert!(result.cycle_display_mode_setting);
        assert!(page.show_settings_page());
    }

    #[test]
    fn launch_on_eighth_system_setting_cycles_language_setting() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(!result.toggle_launch_on_startup);
        assert!(!result.toggle_background_home_wake);
        assert!(!result.toggle_controller_vibration_feedback);
        assert!(result.cycle_language_setting);
        assert!(page.show_settings_page());
    }

    #[test]
    fn launch_on_ninth_system_setting_toggles_launch_on_startup() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(result.toggle_launch_on_startup);
        assert!(!result.toggle_background_home_wake);
        assert!(!result.toggle_controller_vibration_feedback);
        assert!(!result.cycle_language_setting);
        assert!(page.show_settings_page());
    }

    #[test]
    fn launch_on_close_app_entry_closes_frame() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(!result.send_app_to_background);
        assert!(result.close_frame);
        assert!(!page.show_settings_page());
    }

    #[test]
    fn settings_submenu_animation_tracks_submenu_state() {
        let mut page = PageState::new();
        let ctx = eframe::egui::Context::default();
        let mut now = Instant::now();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        tick_animation_frame(&mut page, &ctx, &mut now);
        tick_animation_frame(&mut page, &ctx, &mut now);

        assert!(page.settings_in_submenu());
        assert!(page.settings_submenu_anim() > 0.0);

        let entered_anim = page.settings_submenu_anim();
        let _ = page.handle_action(&ControllerAction::Quit, 3, true, 4);
        for _ in 0..20 {
            tick_animation_frame(&mut page, &ctx, &mut now);
        }

        assert!(!page.settings_in_submenu());
        assert!(page.settings_submenu_anim() < entered_anim);
    }

    #[test]
    fn settings_page_open_animation_uses_slower_enter_speed() {
        let mut page = PageState::new();
        let ctx = eframe::egui::Context::default();
        let mut now = Instant::now();

        open_settings_page(&mut page);
        tick_animation_frame(&mut page, &ctx, &mut now);
        tick_animation_frame(&mut page, &ctx, &mut now);

        let expected = 1.0
            - (1.0 - SETTINGS_PAGE_ENTER_INITIAL_PROGRESS)
                * (-(SETTINGS_PAGE_ENTER_ANIM_SPEED * 1.5) / 60.0).exp();
        assert!((page.settings_page_anim() - expected).abs() < 1e-6);
    }

    #[test]
    fn opening_settings_page_seeds_backdrop_phase_immediately() {
        let mut page = PageState::new();

        open_settings_page(&mut page);

        assert_eq!(
            page.settings_page_anim(),
            SETTINGS_PAGE_ENTER_INITIAL_PROGRESS
        );
    }

    #[test]
    fn settings_submenu_enter_animation_uses_slower_enter_speed() {
        let mut page = PageState::new();
        let ctx = eframe::egui::Context::default();
        let mut now = Instant::now();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        tick_animation_frame(&mut page, &ctx, &mut now);
        tick_animation_frame(&mut page, &ctx, &mut now);

        let expected =
            1.0 - (-(SETTINGS_SUBMENU_ENTER_ANIM_SPEED * scale_seconds(1.0 / 60.0))).exp();
        assert!((page.settings_submenu_anim() - expected).abs() < 1e-6);
    }

    #[test]
    fn settings_selection_animation_keeps_progress_when_selection_changes() {
        let mut page = PageState::new();
        let ctx = eframe::egui::Context::default();
        let mut now = Instant::now();

        open_settings_page(&mut page);
        tick_animation_frame(&mut page, &ctx, &mut now);
        tick_animation_frame(&mut page, &ctx, &mut now);
        let first_anim = page.settings_select_anim();
        assert!(first_anim > 0.0);

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        tick_animation_frame(&mut page, &ctx, &mut now);
        tick_animation_frame(&mut page, &ctx, &mut now);

        assert!(page.settings_select_anim() >= first_anim - 0.001);
    }

    #[test]
    fn closing_settings_page_keeps_exit_animation_running() {
        let mut page = PageState::new();
        let ctx = eframe::egui::Context::default();
        let mut now = Instant::now();

        open_settings_page(&mut page);
        tick_animation_frame(&mut page, &ctx, &mut now);
        tick_animation_frame(&mut page, &ctx, &mut now);
        assert!(page.settings_page_anim() > 0.0);

        let anim_before_close = page.settings_page_anim();
        let _ = page.handle_action(&ControllerAction::Quit, 3, true, 4);

        assert!(!page.show_settings_page());
        assert_eq!(page.settings_page_anim(), anim_before_close);

        for _ in 0..10 {
            tick_animation_frame(&mut page, &ctx, &mut now);
        }
        assert!(page.settings_page_anim() < anim_before_close);
        assert!(page.settings_page_anim() > 0.0);
    }

    #[test]
    fn launch_on_settings_nav_does_not_trigger_content_action() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(!result.toggle_launch_on_startup);
        assert_eq!(result.screen_settings_action, None);
        assert_eq!(result.launch_external_app, None);
        assert!(page.show_settings_page());
        assert!(page.settings_in_submenu());
    }

    #[test]
    fn right_on_settings_nav_does_not_enter_submenu() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let result = page.handle_action(&ControllerAction::Right, 3, true, 4);

        assert!(!result.toggle_launch_on_startup);
        assert_eq!(result.screen_settings_action, None);
        assert_eq!(result.launch_external_app, None);
        assert!(page.show_settings_page());
        assert!(!page.settings_in_submenu());
    }

    #[test]
    fn left_on_settings_nav_does_not_close_page() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let result = page.handle_action(&ControllerAction::Left, 3, true, 4);

        assert!(!result.toggle_launch_on_startup);
        assert_eq!(result.screen_settings_action, None);
        assert_eq!(result.launch_external_app, None);
        assert!(page.show_settings_page());
        assert!(!page.settings_in_submenu());
    }

    #[test]
    fn left_in_settings_submenu_does_not_return_to_top_level() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Left, 3, true, 4);

        assert!(!result.toggle_launch_on_startup);
        assert_eq!(result.screen_settings_action, None);
        assert_eq!(result.launch_external_app, None);
        assert!(page.show_settings_page());
        assert!(page.settings_in_submenu());
    }

    #[test]
    fn quit_in_settings_page_closes_page() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let result = page.handle_action(&ControllerAction::Quit, 3, true, 4);

        assert!(!result.toggle_launch_on_startup);
        assert!(!page.show_settings_page());
    }

    #[test]
    fn quit_in_settings_submenu_returns_to_top_level() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Quit, 3, true, 4);

        assert!(!result.toggle_launch_on_startup);
        assert!(page.show_settings_page());
        assert!(!page.settings_in_submenu());
    }

    #[test]
    fn opening_power_menu_from_settings_keeps_settings_page_visible() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        page.open_power_menu(power_menu_layout());

        assert!(page.show_settings_page());
        assert!(page.show_power_menu());
    }

    #[test]
    fn launch_in_screen_dropdown_selects_second_resolution() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let open_result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(open_result.screen_settings_action, None);
        assert!(page.settings_screen_resolution_dropdown_open());

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(
            result.screen_settings_action,
            Some(ScreenSettingsAction::SelectResolution(1))
        );
        assert!(page.show_settings_page());
    }

    #[test]
    fn launch_in_refresh_rate_dropdown_selects_next_refresh_rate() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let open_result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(open_result.screen_settings_action, None);
        assert!(page.settings_screen_refresh_dropdown_open());

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(
            result.screen_settings_action,
            Some(ScreenSettingsAction::SelectRefreshRate(2))
        );
    }

    #[test]
    fn launch_in_scale_dropdown_selects_next_scale() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let open_result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(open_result.screen_settings_action, None);
        assert!(page.settings_screen_scale_dropdown_open());

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(
            result.screen_settings_action,
            Some(ScreenSettingsAction::SelectScale(2))
        );
    }

    #[test]
    fn apps_section_launches_nvidia_app() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(result.launch_external_app, Some(ExternalAppKind::NvidiaApp));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn power_menu_selection_stops_at_last_option() {
        let mut page = PageState::new();
        page.open_power_menu(power_menu_layout());

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        assert_eq!(page.power_menu_selected, 2);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn down_moves_power_menu_selection_to_shutdown() {
        let mut page = PageState::new();
        page.open_power_menu(power_menu_layout());

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        assert_eq!(page.power_menu_selected, 1);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn second_down_moves_power_menu_selection_to_reboot() {
        let mut page = PageState::new();
        page.open_power_menu(power_menu_layout());

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        assert_eq!(page.power_menu_selected, 2);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn up_from_reboot_returns_to_shutdown() {
        let mut page = PageState::new();
        page.open_power_menu(power_menu_layout());

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Up, 3, true, 4);

        assert_eq!(page.power_menu_selected, 1);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn launch_on_default_power_menu_option_requests_sleep() {
        let mut page = PageState::new();
        page.open_power_menu(power_menu_layout());

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(result.power_action, Some(PowerAction::Sleep));
        assert_eq!(result.screen_settings_action, None);
        assert!(!result.close_frame);
        assert!(!page.show_power_menu());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn launch_on_sleep_option_requests_sleep() {
        let mut page = PageState::new();
        page.open_power_menu(power_menu_layout());

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(result.power_action, Some(PowerAction::Sleep));
        assert_eq!(result.screen_settings_action, None);
        assert!(!result.close_frame);
        assert!(!page.show_power_menu());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn launch_on_reboot_option_requests_reboot() {
        let mut page = PageState::new();
        page.open_power_menu(power_menu_layout());
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(result.power_action, Some(PowerAction::Reboot));
        assert_eq!(result.screen_settings_action, None);
        assert!(!result.close_frame);
        assert!(!page.show_power_menu());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn launch_on_shutdown_option_requests_shutdown() {
        let mut page = PageState::new();
        page.open_power_menu(power_menu_layout());
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(result.power_action, Some(PowerAction::Shutdown));
        assert_eq!(result.screen_settings_action, None);
        assert!(!result.close_frame);
        assert!(!page.show_power_menu());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn right_from_shutdown_keeps_selection_on_shutdown() {
        let mut page = PageState::new();
        page.open_power_menu(power_menu_layout());

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Right, 3, true, 4);

        assert_eq!(page.power_menu_selected, 1);
    }

    #[test]
    fn achievement_panel_launch_marks_hidden_reveal_action() {
        let mut page = PageState::new();
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(result.reveal_hidden_achievement);
        assert!(!result.launch_selected);
    }
}
