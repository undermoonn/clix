use eframe::egui;

use crate::input::ControllerAction;

#[cfg(target_os = "windows")]
const HOME_MENU_HAS_POWER_OPTIONS: bool = true;
#[cfg(not(target_os = "windows"))]
const HOME_MENU_HAS_POWER_OPTIONS: bool = false;

#[cfg(target_os = "windows")]
const HOME_MENU_OPTION_COUNT: usize = 7;
#[cfg(not(target_os = "windows"))]
const HOME_MENU_OPTION_COUNT: usize = 4;
const HOME_MENU_COLUMNS: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolutionPreset {
    HalfMaxRefresh,
    MaxRefresh,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerAction {
    Sleep,
    Shutdown,
}

pub struct PageActionResult {
    pub effective_input: bool,
    pub open_achievement_panel: bool,
    pub reveal_hidden_achievement: bool,
    pub refresh_achievements: bool,
    pub toggle_achievement_sort: bool,
    pub toggle_launch_on_startup: bool,
    pub launch_selected: bool,
    pub selected_changed: bool,
    pub close_frame: bool,
    pub send_app_to_background: bool,
    pub set_resolution: Option<ResolutionPreset>,
    pub power_action: Option<PowerAction>,
}

pub struct PageState {
    selected: usize,
    cover_nav_dir: f32,
    select_anim: f32,
    select_anim_target: Option<usize>,
    wake_anim: f32,
    wake_anim_running: bool,
    scroll_offset: f32,
    show_home_menu: bool,
    home_menu_anim: f32,
    home_menu_selected: usize,
    home_menu_scroll_offset: f32,
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
            cover_nav_dir: 0.0,
            select_anim: 0.0,
            select_anim_target: None,
            wake_anim: 1.0,
            wake_anim_running: false,
            scroll_offset: 0.0,
            show_home_menu: false,
            home_menu_anim: 0.0,
            home_menu_selected: 0,
            home_menu_scroll_offset: 0.0,
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

    pub fn show_achievement_panel(&self) -> bool {
        self.show_achievement_panel
    }

    pub fn show_home_menu(&self) -> bool {
        self.show_home_menu
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

    pub fn wake_anim(&self) -> f32 {
        self.wake_anim
    }

    pub fn home_menu_anim(&self) -> f32 {
        self.home_menu_anim
    }

    pub fn home_menu_scroll_offset(&self) -> f32 {
        self.home_menu_scroll_offset
    }

    pub fn home_menu_shutdown_selected(&self) -> bool {
        HOME_MENU_HAS_POWER_OPTIONS && self.show_home_menu && self.home_menu_selected == 3
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

    pub fn open_home_menu(&mut self) {
        self.home_menu_anim = 0.0;
        self.home_menu_selected = 0;
        self.home_menu_scroll_offset = 0.0;
        self.show_home_menu = true;
    }

    pub fn handle_action(
        &mut self,
        action: &ControllerAction,
        games_len: usize,
        can_open_achievement_panel: bool,
        achievement_len: usize,
    ) -> PageActionResult {
        let mut result = PageActionResult {
            effective_input: false,
            open_achievement_panel: false,
            reveal_hidden_achievement: false,
            refresh_achievements: false,
            toggle_achievement_sort: false,
            toggle_launch_on_startup: false,
            launch_selected: false,
            selected_changed: false,
            close_frame: false,
            send_app_to_background: false,
            set_resolution: None,
            power_action: None,
        };

        if self.show_home_menu {
            match action {
                ControllerAction::Left => {
                    if self.home_menu_selected % HOME_MENU_COLUMNS > 0 {
                        self.home_menu_selected -= 1;
                        result.effective_input = true;
                    }
                }
                ControllerAction::Right => {
                    if self.home_menu_selected % HOME_MENU_COLUMNS + 1 < HOME_MENU_COLUMNS
                        && self.home_menu_selected + 1 < HOME_MENU_OPTION_COUNT
                    {
                        self.home_menu_selected += 1;
                        result.effective_input = true;
                    }
                }
                ControllerAction::Up => {
                    if self.home_menu_selected >= HOME_MENU_COLUMNS {
                        self.home_menu_selected -= HOME_MENU_COLUMNS;
                        result.effective_input = true;
                    }
                }
                ControllerAction::Down => {
                    let target = self.home_menu_selected + HOME_MENU_COLUMNS;
                    if target < HOME_MENU_OPTION_COUNT {
                        self.home_menu_selected = target;
                        result.effective_input = true;
                    }
                }
                ControllerAction::Launch => {
                    let selected_option = self.home_menu_selected;
                    if HOME_MENU_HAS_POWER_OPTIONS {
                        match selected_option {
                            0 => {
                                self.close_home_menu();
                                result.effective_input = true;
                                result.send_app_to_background = true;
                            }
                            1 => {
                                self.close_home_menu();
                                result.effective_input = true;
                                result.close_frame = true;
                            }
                            2 => {
                                self.close_home_menu();
                                result.effective_input = true;
                                result.power_action = Some(PowerAction::Sleep);
                            }
                            3 => {}
                            4 => {
                                self.close_home_menu();
                                result.effective_input = true;
                                result.set_resolution = Some(ResolutionPreset::HalfMaxRefresh);
                            }
                            5 => {
                                self.close_home_menu();
                                result.effective_input = true;
                                result.set_resolution = Some(ResolutionPreset::MaxRefresh);
                            }
                            6 => {
                                result.effective_input = true;
                                result.toggle_launch_on_startup = true;
                            }
                            _ => {}
                        }
                    } else {
                        match selected_option {
                            0 => {
                                self.close_home_menu();
                                result.effective_input = true;
                                result.send_app_to_background = true;
                            }
                            1 => {
                                self.close_home_menu();
                                result.effective_input = true;
                                result.close_frame = true;
                            }
                            2 => {
                                self.close_home_menu();
                                result.effective_input = true;
                                result.set_resolution = Some(ResolutionPreset::HalfMaxRefresh);
                            }
                            3 => {
                                self.close_home_menu();
                                result.effective_input = true;
                                result.set_resolution = Some(ResolutionPreset::MaxRefresh);
                            }
                            _ => {}
                        }
                    }
                }
                ControllerAction::Quit => {
                    self.close_home_menu();
                    result.effective_input = true;
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
                        result.effective_input = true;
                    } else {
                        self.close_achievement_panel();
                        result.effective_input = true;
                    }
                }
                ControllerAction::Down => {
                    if self.achievement_selected + 1 < achievement_len {
                        self.achievement_selected += 1;
                        result.effective_input = true;
                    }
                }
                ControllerAction::Quit => {
                    self.close_achievement_panel();
                    result.effective_input = true;
                }
                ControllerAction::Launch => {
                    result.effective_input = true;
                    result.reveal_hidden_achievement = true;
                }
                ControllerAction::Refresh => {
                    result.effective_input = true;
                    result.refresh_achievements = true;
                }
                ControllerAction::Sort => {
                    self.reset_achievement_selection();
                    result.effective_input = true;
                    result.toggle_achievement_sort = true;
                }
                _ => {}
            }
            return result;
        }

        match action {
            ControllerAction::Left => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.cover_nav_dir = -1.0;
                    self.reset_achievement_selection();
                    result.effective_input = true;
                    result.selected_changed = true;
                }
            }
            ControllerAction::Right => {
                if self.selected + 1 < games_len {
                    self.selected += 1;
                    self.cover_nav_dir = 1.0;
                    self.reset_achievement_selection();
                    result.effective_input = true;
                    result.selected_changed = true;
                }
            }
            ControllerAction::Down => {
                if can_open_achievement_panel {
                    self.show_achievement_panel = true;
                    self.reset_achievement_selection();
                    result.effective_input = true;
                    result.open_achievement_panel = true;
                }
            }
            ControllerAction::Launch => {
                result.effective_input = true;
                result.launch_selected = true;
            }
            ControllerAction::Refresh => {}
            ControllerAction::Sort => {}
            ControllerAction::Quit => {}
            ControllerAction::Up => {}
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

        if self.show_home_menu {
            let home_menu_diff = 1.0 - self.home_menu_anim;
            if home_menu_diff.abs() > 0.001 {
                self.home_menu_anim += home_menu_diff * (1.0 - (-7.5 * dt).exp());
                ctx.request_repaint();
            } else {
                self.home_menu_anim = 1.0;
            }

            self.home_menu_scroll_offset = self.home_menu_selected as f32;
        } else {
            self.home_menu_anim = 0.0;
            self.home_menu_scroll_offset = self.home_menu_selected as f32;
        }

        let scroll_target = self.selected as f32;
        let scroll_diff = scroll_target - self.scroll_offset;
        if scroll_diff.abs() > 0.001 {
            self.scroll_offset += scroll_diff * (1.0 - (-14.0 * dt).exp());
            ctx.request_repaint();
        } else {
            self.scroll_offset = scroll_target;
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

    fn close_home_menu(&mut self) {
        self.show_home_menu = false;
        self.home_menu_anim = 0.0;
        self.home_menu_selected = 0;
        self.home_menu_scroll_offset = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::{PageState, PowerAction, ResolutionPreset};
    use crate::input::ControllerAction;

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
        assert!(!page.show_home_menu());
    }

    #[test]
    fn quit_on_home_menu_closes_immediately_without_animation() {
        let mut page = PageState::new();
        page.open_home_menu();

        let _ = page.handle_action(&ControllerAction::Quit, 3, true, 4);

        assert!(!page.show_home_menu());
        assert_eq!(page.home_menu_anim(), 0.0);
    }

    #[test]
    fn home_menu_navigates_to_close_app_option_with_right() {
        let mut page = PageState::new();
        page.open_home_menu();

        let _ = page.handle_action(&ControllerAction::Right, 3, true, 4);

        assert_eq!(page.home_menu_selected, 1);
    }

    #[test]
    fn launch_on_close_app_option_closes_frame() {
        let mut page = PageState::new();
        page.open_home_menu();
        let _ = page.handle_action(&ControllerAction::Right, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(!result.send_app_to_background);
        assert_eq!(result.set_resolution, None);
        assert!(result.close_frame);
        assert!(!page.show_home_menu());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn launch_on_startup_option_toggles_without_closing_menu() {
        let mut page = PageState::new();
        page.open_home_menu();

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(result.toggle_launch_on_startup);
        assert!(page.show_home_menu());
    }

    #[test]
    fn home_menu_selection_stops_at_last_option() {
        let mut page = PageState::new();
        page.open_home_menu();

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Right, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Right, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        #[cfg(target_os = "windows")]
        assert_eq!(page.home_menu_selected, 6);

        #[cfg(not(target_os = "windows"))]
        assert_eq!(page.home_menu_selected, 3);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn down_moves_home_menu_selection_to_power_row() {
        let mut page = PageState::new();
        page.open_home_menu();

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        assert_eq!(page.home_menu_selected, 2);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn second_down_moves_home_menu_selection_to_resolution_row() {
        let mut page = PageState::new();
        page.open_home_menu();

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        assert_eq!(page.home_menu_selected, 4);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn down_moves_home_menu_selection_to_resolution_row() {
        let mut page = PageState::new();
        page.open_home_menu();

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        assert_eq!(page.home_menu_selected, 2);
    }

    #[test]
    fn up_returns_home_menu_selection_to_top_row() {
        let mut page = PageState::new();
        page.open_home_menu();
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Up, 3, true, 4);

        assert_eq!(page.home_menu_selected, 0);
    }

    #[test]
    fn launch_on_default_home_menu_option_sends_app_to_background() {
        let mut page = PageState::new();
        page.open_home_menu();

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(result.send_app_to_background);
        assert_eq!(result.set_resolution, None);
        assert!(!result.close_frame);
        assert!(!page.show_home_menu());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn launch_on_sleep_option_requests_sleep() {
        let mut page = PageState::new();
        page.open_home_menu();
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(result.power_action, Some(PowerAction::Sleep));
        assert_eq!(result.set_resolution, None);
        assert!(!result.close_frame);
        assert!(!page.show_home_menu());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn launch_on_shutdown_option_waits_for_hold() {
        let mut page = PageState::new();
        page.open_home_menu();
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Right, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(result.power_action, None);
        assert_eq!(result.set_resolution, None);
        assert!(!result.close_frame);
        assert!(page.show_home_menu());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn launch_on_half_refresh_option_requests_resolution_change() {
        let mut page = PageState::new();
        page.open_home_menu();
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(
            result.set_resolution,
            Some(ResolutionPreset::HalfMaxRefresh)
        );
        assert_eq!(result.power_action, None);
        assert!(!result.send_app_to_background);
        assert!(!result.close_frame);
        assert!(!page.show_home_menu());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn launch_on_half_refresh_option_requests_resolution_change() {
        let mut page = PageState::new();
        page.open_home_menu();
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(
            result.set_resolution,
            Some(ResolutionPreset::HalfMaxRefresh)
        );
        assert!(!result.send_app_to_background);
        assert!(!result.close_frame);
        assert!(!page.show_home_menu());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn launch_on_max_refresh_option_requests_resolution_change() {
        let mut page = PageState::new();
        page.open_home_menu();
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Right, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(
            result.set_resolution,
            Some(ResolutionPreset::MaxRefresh)
        );
        assert_eq!(result.power_action, None);
        assert!(!result.send_app_to_background);
        assert!(!result.close_frame);
        assert!(!page.show_home_menu());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn launch_on_max_refresh_option_requests_resolution_change() {
        let mut page = PageState::new();
        page.open_home_menu();
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Right, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(
            result.set_resolution,
            Some(ResolutionPreset::MaxRefresh)
        );
        assert!(!result.send_app_to_background);
        assert!(!result.close_frame);
        assert!(!page.show_home_menu());
    }

    #[test]
    fn achievement_panel_launch_marks_hidden_reveal_action() {
        let mut page = PageState::new();
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(result.reveal_hidden_achievement);
        assert!(!result.launch_selected);
    }

    #[test]
    fn achievement_panel_sort_resets_selection_to_top() {
        let mut page = PageState::new();
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        assert_eq!(page.achievement_selected(), 2);

        let result = page.handle_action(&ControllerAction::Sort, 3, true, 4);

        assert!(result.toggle_achievement_sort);
        assert_eq!(page.achievement_selected(), 0);
    }
}