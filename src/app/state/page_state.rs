use eframe::egui;

use crate::power_menu_structure::{PowerMenuLayout, PowerMenuOption};
use crate::input::ControllerAction;
use crate::system::external_apps::ExternalAppKind;

const SETTINGS_PAGE_ENTER_ANIM_SPEED: f32 = 5.0;
const SETTINGS_PAGE_EXIT_ANIM_SPEED: f32 = 8.0;
const SETTINGS_SUBMENU_ENTER_ANIM_SPEED: f32 = 4.0;
const SETTINGS_SUBMENU_EXIT_ANIM_SPEED: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolutionPreset {
    HalfMaxRefresh,
    MaxRefresh,
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
    pub toggle_launch_on_startup: bool,
    pub launch_selected: bool,
    pub launch_external_app: Option<ExternalAppKind>,
    pub selected_changed: bool,
    pub close_frame: bool,
    pub send_app_to_background: bool,
    pub set_resolution: Option<ResolutionPreset>,
    pub power_action: Option<PowerAction>,
}

pub struct PageState {
    selected: usize,
    home_settings_selected: bool,
    home_settings_focus_anim: f32,
    home_power_selected: bool,
    home_power_focus_anim: f32,
    cover_nav_dir: f32,
    select_anim: f32,
    select_anim_target: Option<usize>,
    summary_cards_visibility: f32,
    wake_anim: f32,
    wake_anim_running: bool,
    scroll_offset: f32,
    show_power_menu: bool,
    power_menu_anim: f32,
    power_menu_selected: usize,
    power_menu_select_anim: f32,
    power_menu_select_anim_target: Option<usize>,
    power_menu_layout: PowerMenuLayout,
    power_menu_scroll_offset: f32,
    show_settings_page: bool,
    settings_page_anim: f32,
    settings_submenu_anim: f32,
    settings_select_anim: f32,
    settings_select_anim_target: Option<u8>,
    settings_section: SettingsSection,
    settings_in_submenu: bool,
    settings_system_selected: usize,
    settings_screen_selected: usize,
    settings_apps_selected: usize,
    show_achievement_panel: bool,
    achievement_panel_anim: f32,
    achievement_selected: usize,
    achievement_select_anim: f32,
    achievement_select_anim_target: Option<usize>,
    achievement_scroll_offset: f32,
}

impl PageState {
    pub fn new() -> Self {
        Self {
            selected: 0,
            home_settings_selected: false,
            home_settings_focus_anim: 0.0,
            home_power_selected: false,
            home_power_focus_anim: 0.0,
            cover_nav_dir: 0.0,
            select_anim: 0.0,
            select_anim_target: None,
            summary_cards_visibility: 1.0,
            wake_anim: 1.0,
            wake_anim_running: false,
            scroll_offset: 0.0,
            show_power_menu: false,
            power_menu_anim: 0.0,
            power_menu_selected: 0,
            power_menu_select_anim: 0.0,
            power_menu_select_anim_target: None,
            power_menu_layout: PowerMenuLayout::default(),
            power_menu_scroll_offset: 0.0,
            show_settings_page: false,
            settings_page_anim: 0.0,
            settings_submenu_anim: 0.0,
            settings_select_anim: 0.0,
            settings_select_anim_target: None,
            settings_section: SettingsSection::System,
            settings_in_submenu: false,
            settings_system_selected: 0,
            settings_screen_selected: 0,
            settings_apps_selected: 0,
            show_achievement_panel: false,
            achievement_panel_anim: 0.0,
            achievement_selected: 0,
            achievement_select_anim: 0.0,
            achievement_select_anim_target: None,
            achievement_scroll_offset: 0.0,
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
        self.home_settings_focus_anim
    }

    pub fn home_power_focus_anim(&self) -> f32 {
        self.home_power_focus_anim
    }

    pub fn home_top_focus_anim(&self) -> f32 {
        self.home_settings_focus_anim.max(self.home_power_focus_anim)
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
        self.select_anim
    }

    pub fn scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    pub fn is_fast_scrolling(&self) -> bool {
        (self.selected as f32 - self.scroll_offset).abs() > 0.1
    }

    pub fn summary_cards_visibility(&self) -> f32 {
        self.summary_cards_visibility
    }

    pub fn wake_anim(&self) -> f32 {
        self.wake_anim
    }

    pub fn power_menu_anim(&self) -> f32 {
        self.power_menu_anim
    }

    pub fn power_menu_select_anim(&self) -> f32 {
        self.power_menu_select_anim
    }

    pub fn power_menu_scroll_offset(&self) -> f32 {
        self.power_menu_scroll_offset
    }

    pub fn settings_page_anim(&self) -> f32 {
        self.settings_page_anim
    }

    pub fn settings_submenu_anim(&self) -> f32 {
        self.settings_submenu_anim
    }

    pub fn settings_select_anim(&self) -> f32 {
        self.settings_select_anim
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

    pub fn power_menu_layout(&self) -> &PowerMenuLayout {
        &self.power_menu_layout
    }

    pub fn achievement_panel_anim(&self) -> f32 {
        self.achievement_panel_anim
    }

    pub fn achievement_selected(&self) -> usize {
        self.achievement_selected
    }

    pub fn achievement_select_anim(&self) -> f32 {
        self.achievement_select_anim
    }

    pub fn achievement_scroll_offset(&self) -> f32 {
        self.achievement_scroll_offset
    }

    pub fn prepare_wake_animation(&mut self) {
        self.wake_anim = 0.0;
        self.wake_anim_running = false;
    }

    pub fn start_wake_animation(&mut self) {
        self.wake_anim_running = true;
    }

    pub fn force_select(&mut self, selected: usize) {
        self.selected = selected;
        self.clear_home_top_button_selection();
        self.home_settings_focus_anim = 0.0;
        self.home_power_focus_anim = 0.0;
        self.cover_nav_dir = 0.0;
        self.select_anim = 0.0;
        self.select_anim_target = None;
        self.scroll_offset = selected as f32;
        self.show_settings_page = false;
        self.settings_page_anim = 0.0;
        self.settings_select_anim = 0.0;
        self.settings_select_anim_target = None;
        self.reset_settings_navigation();
        self.show_achievement_panel = false;
        self.achievement_panel_anim = 0.0;
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
        self.scroll_offset = selected as f32;
    }

    pub fn open_power_menu(&mut self, layout: PowerMenuLayout) {
        let default_selected = layout.default_selected();
        self.power_menu_layout = layout;
        self.power_menu_anim = 0.0;
        self.power_menu_selected = default_selected;
        self.power_menu_select_anim = 0.0;
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
            toggle_launch_on_startup: false,
            launch_selected: false,
            launch_external_app: None,
            selected_changed: false,
            close_frame: false,
            send_app_to_background: false,
            set_resolution: None,
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
                        self.move_settings_item(false);
                    } else {
                        self.settings_section = self.settings_section.previous();
                    }
                }
                ControllerAction::Down => {
                    if self.settings_in_submenu {
                        self.move_settings_item(true);
                    } else {
                        self.settings_section = self.settings_section.next();
                    }
                }
                ControllerAction::Launch => {
                    if self.settings_in_submenu {
                        match self.settings_section {
                            SettingsSection::System => {
                                result.toggle_launch_on_startup = true;
                            }
                            SettingsSection::Screen => {
                                result.set_resolution = Some(match self.settings_screen_selected {
                                    0 => ResolutionPreset::HalfMaxRefresh,
                                    _ => ResolutionPreset::MaxRefresh,
                                });
                            }
                            SettingsSection::Apps => {
                                result.launch_external_app = Some(match self.settings_apps_selected {
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
                        self.settings_in_submenu = false;
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
                ControllerAction::Down => {
                    self.clear_home_top_button_selection();
                }
                ControllerAction::Right | ControllerAction::Settings => {
                    self.select_home_settings_button();
                }
                ControllerAction::Quit => {
                    self.clear_home_top_button_selection();
                }
                ControllerAction::Left
                | ControllerAction::Refresh
                | ControllerAction::Up => {}
            }
            return result;
        }

        if self.home_settings_selected {
            match action {
                ControllerAction::Left => {
                    self.select_home_power_button();
                }
                ControllerAction::Right => {
                    self.clear_home_top_button_selection();
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
                ControllerAction::Refresh
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

    pub fn tick_animations(&mut self, ctx: &egui::Context, dt: f32) {
        if self.wake_anim_running {
            self.wake_anim = 1.0 - (1.0 - self.wake_anim) * (-8.0 * dt).exp();
            if self.wake_anim < 0.999 {
                ctx.request_repaint();
            } else {
                self.wake_anim = 1.0;
                self.wake_anim_running = false;
            }
        }

        if self.select_anim_target != Some(self.selected) {
            self.select_anim_target = Some(self.selected);
            self.select_anim = 0.0;
        }
        self.select_anim = 1.0 - (1.0 - self.select_anim) * (-10.0 * dt).exp();
        if self.select_anim < 0.999 {
            ctx.request_repaint();
        }

        let panel_target = if self.show_achievement_panel { 1.0 } else { 0.0 };
        let panel_diff = panel_target - self.achievement_panel_anim;
        if panel_diff.abs() > 0.001 {
            self.achievement_panel_anim += panel_diff * (1.0 - (-5.4 * dt).exp());
            ctx.request_repaint();
        } else {
            self.achievement_panel_anim = panel_target;
        }

        let home_settings_target = if self.home_settings_selected { 1.0 } else { 0.0 };
        let home_settings_diff = home_settings_target - self.home_settings_focus_anim;
        if home_settings_diff.abs() > 0.001 {
            self.home_settings_focus_anim +=
                home_settings_diff * (1.0 - (-11.0 * dt).exp());
            ctx.request_repaint();
        } else {
            self.home_settings_focus_anim = home_settings_target;
        }

        let home_power_target = if self.home_power_selected { 1.0 } else { 0.0 };
        let home_power_diff = home_power_target - self.home_power_focus_anim;
        if home_power_diff.abs() > 0.001 {
            self.home_power_focus_anim += home_power_diff * (1.0 - (-11.0 * dt).exp());
            ctx.request_repaint();
        } else {
            self.home_power_focus_anim = home_power_target;
        }

        if self.show_power_menu {
            let power_menu_diff = 1.0 - self.power_menu_anim;
            if power_menu_diff.abs() > 0.001 {
                self.power_menu_anim += power_menu_diff * (1.0 - (-4.8 * dt).exp());
                ctx.request_repaint();
            } else {
                self.power_menu_anim = 1.0;
            }

            let power_menu_select_target = Some(self.power_menu_selected);
            if self.power_menu_select_anim_target != power_menu_select_target {
                self.power_menu_select_anim_target = power_menu_select_target;
                self.power_menu_select_anim = 0.0;
            }
            let power_menu_select_diff = 1.0 - self.power_menu_select_anim;
            if power_menu_select_diff.abs() > 0.001 {
                self.power_menu_select_anim += power_menu_select_diff * (1.0 - (-11.0 * dt).exp());
                ctx.request_repaint();
            } else {
                self.power_menu_select_anim = 1.0;
            }

            self.power_menu_scroll_offset = self.power_menu_selected as f32;
        } else {
            let power_menu_diff = -self.power_menu_anim;
            if power_menu_diff.abs() > 0.001 {
                self.power_menu_anim += power_menu_diff * (1.0 - (-6.5 * dt).exp());
                ctx.request_repaint();
            } else {
                self.power_menu_anim = 0.0;
                self.power_menu_selected = self.power_menu_layout.clamp_selected(0);
                self.power_menu_select_anim = 0.0;
                self.power_menu_select_anim_target = None;
            }
            self.power_menu_scroll_offset = self.power_menu_selected as f32;
        }

        let settings_target = if self.show_settings_page { 1.0 } else { 0.0 };
        let settings_diff = settings_target - self.settings_page_anim;
        if settings_diff.abs() > 0.001 {
            let settings_anim_speed = if settings_diff < 0.0 {
                SETTINGS_PAGE_EXIT_ANIM_SPEED
            } else {
                SETTINGS_PAGE_ENTER_ANIM_SPEED
            };
            self.settings_page_anim += settings_diff * (1.0 - (-settings_anim_speed * dt).exp());
            ctx.request_repaint();
        } else {
            self.settings_page_anim = settings_target;
        }

        let submenu_target = if self.show_settings_page && self.settings_in_submenu {
            1.0
        } else {
            0.0
        };
        let submenu_diff = submenu_target - self.settings_submenu_anim;
        if submenu_diff.abs() > 0.001 {
            let submenu_anim_speed = if submenu_diff < 0.0 {
                SETTINGS_SUBMENU_EXIT_ANIM_SPEED
            } else {
                SETTINGS_SUBMENU_ENTER_ANIM_SPEED
            };
            self.settings_submenu_anim +=
                submenu_diff * (1.0 - (-submenu_anim_speed * dt).exp());
            ctx.request_repaint();
        } else {
            self.settings_submenu_anim = submenu_target;
        }

        let settings_select_target = if self.show_settings_page {
            Some(self.settings_selection_key())
        } else {
            None
        };
        if self.settings_select_anim_target != settings_select_target {
            self.settings_select_anim_target = settings_select_target;
            self.settings_select_anim = 0.0;
        }
        let settings_select_value_target = if settings_select_target.is_some() { 1.0 } else { 0.0 };
        let settings_select_diff = settings_select_value_target - self.settings_select_anim;
        if settings_select_diff.abs() > 0.001 {
            self.settings_select_anim += settings_select_diff * (1.0 - (-11.0 * dt).exp());
            ctx.request_repaint();
        } else {
            self.settings_select_anim = settings_select_value_target;
        }

        let scroll_target = self.selected as f32;
        let scroll_diff = scroll_target - self.scroll_offset;
        if scroll_diff.abs() > 0.001 {
            self.scroll_offset += scroll_diff * (1.0 - (-14.0 * dt).exp());
            ctx.request_repaint();
        } else {
            self.scroll_offset = scroll_target;
        }

        let summary_cards_target = if self.is_fast_scrolling() { 0.0 } else { 1.0 };
        let summary_cards_diff = summary_cards_target - self.summary_cards_visibility;
        if summary_cards_diff.abs() > 0.001 {
            let fade_speed = if summary_cards_diff < 0.0 { 18.0 } else { 10.0 };
            self.summary_cards_visibility +=
                summary_cards_diff * (1.0 - (-fade_speed * dt).exp());
            ctx.request_repaint();
        } else {
            self.summary_cards_visibility = summary_cards_target;
        }

        if self.achievement_select_anim_target != Some(self.achievement_selected) {
            self.achievement_select_anim_target = Some(self.achievement_selected);
            self.achievement_select_anim = 0.0;
        }
        self.achievement_select_anim =
            1.0 - (1.0 - self.achievement_select_anim) * (-10.0 * dt).exp();
        if self.achievement_select_anim < 0.999 {
            ctx.request_repaint();
        }

        let ach_target = self.achievement_selected.saturating_sub(2) as f32;
        let ach_diff = ach_target - self.achievement_scroll_offset;
        if ach_diff.abs() > 0.001 {
            self.achievement_scroll_offset += ach_diff * (1.0 - (-14.0 * dt).exp());
            ctx.request_repaint();
        } else {
            self.achievement_scroll_offset = ach_target;
        }
    }

    fn reset_achievement_selection(&mut self) {
        self.achievement_selected = 0;
        self.achievement_select_anim = 0.0;
        self.achievement_select_anim_target = None;
        self.achievement_scroll_offset = 0.0;
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
        self.settings_page_anim = 0.0;
        self.settings_submenu_anim = 0.0;
        self.reset_settings_navigation();
    }

    fn close_settings_page(&mut self) {
        self.show_settings_page = false;
        self.settings_in_submenu = false;
    }

    fn reset_settings_navigation(&mut self) {
        self.settings_section = SettingsSection::System;
        self.settings_in_submenu = false;
        self.settings_system_selected = 0;
        self.settings_screen_selected = 0;
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

    fn settings_selection_key(&self) -> u8 {
        if self.settings_in_submenu {
            match self.settings_section {
                SettingsSection::System => 10,
                SettingsSection::Screen => 20 + self.settings_screen_selected as u8,
                SettingsSection::Apps => 30 + self.settings_apps_selected as u8,
                SettingsSection::CloseApp => 40,
            }
        } else {
            self.settings_section.index() as u8
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
            SettingsSection::System => 0,
            SettingsSection::Screen => 1,
            SettingsSection::Apps => 1,
            SettingsSection::CloseApp => 0,
        };

        if down {
            *selected = (*selected + 1).min(max_index);
        } else {
            *selected = selected.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PageState, PowerAction, ResolutionPreset, SETTINGS_PAGE_ENTER_ANIM_SPEED,
        SETTINGS_SUBMENU_ENTER_ANIM_SPEED,
    };
    use crate::power_menu_structure::PowerMenuLayout;
    use crate::input::ControllerAction;
    use crate::system::external_apps::ExternalAppKind;

    fn open_settings_page(page: &mut PageState) {
        let _ = page.handle_action(&ControllerAction::Settings, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
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
        page.open_power_menu(power_menu_layout());
        page.tick_animations(&ctx, 1.0 / 60.0);
        assert!(page.power_menu_anim() > 0.0);

        let anim_before_close = page.power_menu_anim();

        let _ = page.handle_action(&ControllerAction::Quit, 3, true, 4);

        assert!(!page.show_power_menu());
        assert_eq!(page.power_menu_anim(), anim_before_close);

        page.tick_animations(&ctx, 1.0 / 60.0);
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
    fn right_from_home_settings_button_returns_focus_to_game_list() {
        let mut page = PageState::new();
        let _ = page.handle_action(&ControllerAction::Up, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Right, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Right, 3, true, 4);

        assert!(!result.open_power_menu);
        assert!(!result.open_achievement_panel);
        assert!(!page.home_settings_selected());
        assert!(!page.home_power_selected());
        assert!(!page.show_achievement_panel());
    }

    #[test]
    fn home_top_focus_animation_tracks_selection_handoff() {
        let mut page = PageState::new();
        let ctx = eframe::egui::Context::default();

        let _ = page.handle_action(&ControllerAction::Settings, 3, true, 4);
        page.tick_animations(&ctx, 1.0 / 60.0);
        assert!(page.home_settings_focus_anim() > 0.0);
        assert_eq!(page.home_power_focus_anim(), 0.0);

        let anim_while_selected = page.home_settings_focus_anim();
        let _ = page.handle_action(&ControllerAction::Left, 3, true, 4);
        page.tick_animations(&ctx, 1.0 / 60.0);

        assert!(!page.home_settings_selected());
        assert!(page.home_power_selected());
        assert!(page.home_settings_focus_anim() < anim_while_selected);
        assert!(page.home_power_focus_anim() > 0.0);
    }

    #[test]
    fn horizontal_navigation_is_disabled_while_home_settings_button_is_selected() {
        let mut page = PageState::new();
        let _ = page.handle_action(&ControllerAction::Up, 3, true, 4);

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
        page.open_power_menu(power_menu_layout());

        page.tick_animations(&ctx, 1.0 / 60.0);
        let first_anim = page.power_menu_select_anim();
        assert!(first_anim > 0.0);

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        page.tick_animations(&ctx, 1.0 / 60.0);

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
    fn launch_in_settings_page_toggles_without_closing_page() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(result.toggle_launch_on_startup);
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

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        page.tick_animations(&ctx, 1.0 / 60.0);

        assert!(page.settings_in_submenu());
        assert!(page.settings_submenu_anim() > 0.0);

        let entered_anim = page.settings_submenu_anim();
        let _ = page.handle_action(&ControllerAction::Quit, 3, true, 4);
        page.tick_animations(&ctx, 1.0 / 60.0);

        assert!(!page.settings_in_submenu());
        assert!(page.settings_submenu_anim() < entered_anim);
    }

    #[test]
    fn settings_page_open_animation_uses_slower_enter_speed() {
        let mut page = PageState::new();
        let ctx = eframe::egui::Context::default();

        open_settings_page(&mut page);
        page.tick_animations(&ctx, 1.0 / 60.0);

        let expected = 1.0 - (-SETTINGS_PAGE_ENTER_ANIM_SPEED / 60.0).exp();
        assert!((page.settings_page_anim() - expected).abs() < 1e-6);
    }

    #[test]
    fn settings_submenu_enter_animation_uses_slower_enter_speed() {
        let mut page = PageState::new();
        let ctx = eframe::egui::Context::default();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        page.tick_animations(&ctx, 1.0 / 60.0);

        let expected = 1.0 - (-SETTINGS_SUBMENU_ENTER_ANIM_SPEED / 60.0).exp();
        assert!((page.settings_submenu_anim() - expected).abs() < 1e-6);
    }

    #[test]
    fn settings_selection_animation_resets_when_selection_changes() {
        let mut page = PageState::new();
        let ctx = eframe::egui::Context::default();

        open_settings_page(&mut page);
        page.tick_animations(&ctx, 1.0 / 60.0);
        let first_anim = page.settings_select_anim();
        assert!(first_anim > 0.0);

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        page.tick_animations(&ctx, 1.0 / 60.0);

        assert!(page.settings_select_anim() > 0.0);
        assert!(page.settings_select_anim() <= first_anim + 0.001);
    }

    #[test]
    fn closing_settings_page_keeps_exit_animation_running() {
        let mut page = PageState::new();
        let ctx = eframe::egui::Context::default();

        open_settings_page(&mut page);
        page.tick_animations(&ctx, 1.0 / 60.0);
        assert!(page.settings_page_anim() > 0.0);

        let anim_before_close = page.settings_page_anim();
        let _ = page.handle_action(&ControllerAction::Quit, 3, true, 4);

        assert!(!page.show_settings_page());
        assert_eq!(page.settings_page_anim(), anim_before_close);

        page.tick_animations(&ctx, 1.0 / 60.0);
        assert!(page.settings_page_anim() < anim_before_close);
        assert!(page.settings_page_anim() > 0.0);
    }

    #[test]
    fn launch_on_settings_nav_does_not_trigger_content_action() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(!result.toggle_launch_on_startup);
        assert_eq!(result.set_resolution, None);
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
        assert_eq!(result.set_resolution, None);
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
        assert_eq!(result.set_resolution, None);
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
        assert_eq!(result.set_resolution, None);
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
    fn right_and_down_select_second_screen_setting() {
        let mut page = PageState::new();

        open_settings_page(&mut page);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(result.set_resolution, Some(ResolutionPreset::MaxRefresh));
        assert!(page.show_settings_page());
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
        assert_eq!(result.set_resolution, None);
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
        assert_eq!(result.set_resolution, None);
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
        assert_eq!(result.set_resolution, None);
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
        assert_eq!(result.set_resolution, None);
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