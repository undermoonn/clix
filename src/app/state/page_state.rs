use eframe::egui;

use crate::home_menu_structure::{HomeMenuLayout, HomeMenuMove, HomeMenuOption};
use crate::input::ControllerAction;
use crate::system::external_apps::ExternalAppKind;

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
    pub open_achievement_panel: bool,
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
    cover_nav_dir: f32,
    select_anim: f32,
    select_anim_target: Option<usize>,
    summary_cards_visibility: f32,
    wake_anim: f32,
    wake_anim_running: bool,
    scroll_offset: f32,
    show_home_menu: bool,
    home_menu_anim: f32,
    home_menu_selected: usize,
    home_menu_layout: HomeMenuLayout,
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
            summary_cards_visibility: 1.0,
            wake_anim: 1.0,
            wake_anim_running: false,
            scroll_offset: 0.0,
            show_home_menu: false,
            home_menu_anim: 0.0,
            home_menu_selected: 0,
            home_menu_layout: HomeMenuLayout::default(),
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

    pub fn is_fast_scrolling(&self) -> bool {
        (self.selected as f32 - self.scroll_offset).abs() > 0.1
    }

    pub fn summary_cards_visibility(&self) -> f32 {
        self.summary_cards_visibility
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

    pub fn home_menu_layout(&self) -> &HomeMenuLayout {
        &self.home_menu_layout
    }

    pub fn home_menu_shutdown_selected(&self) -> bool {
        self.show_home_menu && self.home_menu_layout.is_shutdown_selected(self.home_menu_selected)
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

    pub fn open_home_menu(&mut self, layout: HomeMenuLayout) {
        let default_selected = layout.default_selected();
        self.home_menu_layout = layout;
        self.home_menu_anim = 0.0;
        self.home_menu_selected = default_selected;
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
            open_achievement_panel: false,
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

        if self.show_home_menu {
            match action {
                ControllerAction::Left => {
                    let next = self
                        .home_menu_layout
                        .move_selection(self.home_menu_selected, HomeMenuMove::Left);
                    if next != self.home_menu_selected {
                        self.home_menu_selected = next;
                    }
                }
                ControllerAction::Right => {
                    let next = self
                        .home_menu_layout
                        .move_selection(self.home_menu_selected, HomeMenuMove::Right);
                    if next != self.home_menu_selected {
                        self.home_menu_selected = next;
                    }
                }
                ControllerAction::Up => {
                    let next = self
                        .home_menu_layout
                        .move_selection(self.home_menu_selected, HomeMenuMove::Up);
                    if next != self.home_menu_selected {
                        self.home_menu_selected = next;
                    }
                }
                ControllerAction::Down => {
                    let next = self
                        .home_menu_layout
                        .move_selection(self.home_menu_selected, HomeMenuMove::Down);
                    if next != self.home_menu_selected {
                        self.home_menu_selected = next;
                    }
                }
                ControllerAction::Launch => {
                    match self.home_menu_layout.option_at(self.home_menu_selected) {
                        Some(HomeMenuOption::ExternalApp(kind)) => {
                            self.close_home_menu();
                            result.launch_external_app = Some(kind);
                        }
                        Some(HomeMenuOption::MinimizeApp) => {
                            self.close_home_menu();
                            result.send_app_to_background = true;
                        }
                        Some(HomeMenuOption::CloseApp) => {
                            self.close_home_menu();
                            result.close_frame = true;
                        }
                        Some(HomeMenuOption::Sleep) => {
                            self.close_home_menu();
                            result.power_action = Some(PowerAction::Sleep);
                        }
                        Some(HomeMenuOption::Shutdown) => {}
                        Some(HomeMenuOption::HalfMaxRefresh) => {
                            self.close_home_menu();
                            result.set_resolution = Some(ResolutionPreset::HalfMaxRefresh);
                        }
                        Some(HomeMenuOption::MaxRefresh) => {
                            self.close_home_menu();
                            result.set_resolution = Some(ResolutionPreset::MaxRefresh);
                        }
                        Some(HomeMenuOption::LaunchOnStartup) => {
                            result.toggle_launch_on_startup = true;
                        }
                        None => {}
                    }
                }
                ControllerAction::Quit => {
                    self.close_home_menu();
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

    fn close_home_menu(&mut self) {
        self.show_home_menu = false;
        self.home_menu_anim = 0.0;
        self.home_menu_selected = self.home_menu_layout.clamp_selected(0);
        self.home_menu_scroll_offset = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::{PageState, PowerAction, ResolutionPreset};
    use crate::home_menu_structure::HomeMenuLayout;
    use crate::input::ControllerAction;
    use crate::system::external_apps::ExternalAppKind;

    #[cfg(target_os = "windows")]
    fn home_menu_layout() -> HomeMenuLayout {
        HomeMenuLayout::new(&[], true, true)
    }

    #[cfg(not(target_os = "windows"))]
    fn home_menu_layout() -> HomeMenuLayout {
        HomeMenuLayout::new(&[], false, false)
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
        assert!(!page.show_home_menu());
    }

    #[test]
    fn quit_on_home_menu_closes_immediately_without_animation() {
        let mut page = PageState::new();
        page.open_home_menu(home_menu_layout());

        let _ = page.handle_action(&ControllerAction::Quit, 3, true, 4);

        assert!(!page.show_home_menu());
        assert_eq!(page.home_menu_anim(), 0.0);
    }

    #[test]
    fn home_menu_navigates_to_close_app_option_with_right() {
        let mut page = PageState::new();
        page.open_home_menu(home_menu_layout());

        let _ = page.handle_action(&ControllerAction::Right, 3, true, 4);

        assert_eq!(page.home_menu_selected, 1);
    }

    #[test]
    fn home_menu_defaults_to_minimize_when_external_icons_exist() {
        let mut page = PageState::new();
        page.open_home_menu(HomeMenuLayout::new(
            &[ExternalAppKind::DlssSwapper],
            cfg!(target_os = "windows"),
            cfg!(target_os = "windows"),
        ));

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(result.send_app_to_background);
        assert_eq!(result.launch_external_app, None);
    }

    #[test]
    fn left_from_minimize_selects_external_icon() {
        let mut page = PageState::new();
        page.open_home_menu(HomeMenuLayout::new(
            &[ExternalAppKind::DlssSwapper],
            cfg!(target_os = "windows"),
            cfg!(target_os = "windows"),
        ));

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        #[cfg(target_os = "windows")]
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(result.launch_external_app, Some(ExternalAppKind::DlssSwapper));
    }

    #[test]
    fn up_from_minimize_does_not_jump_to_external_icon() {
        let mut page = PageState::new();
        page.open_home_menu(HomeMenuLayout::new(
            &[ExternalAppKind::DlssSwapper],
            cfg!(target_os = "windows"),
            cfg!(target_os = "windows"),
        ));

        let _ = page.handle_action(&ControllerAction::Up, 3, true, 4);
        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(result.send_app_to_background);
        assert_eq!(result.launch_external_app, None);
    }

    #[test]
    fn launch_on_close_app_option_closes_frame() {
        let mut page = PageState::new();
        page.open_home_menu(home_menu_layout());
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
        page.open_home_menu(home_menu_layout());

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Right, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Right, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert!(result.toggle_launch_on_startup);
        assert!(page.show_home_menu());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn right_from_close_app_selects_startup_option() {
        let mut page = PageState::new();
        page.open_home_menu(home_menu_layout());

        let _ = page.handle_action(&ControllerAction::Right, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Right, 3, true, 4);

        let launch_result = page.handle_action(&ControllerAction::Launch, 3, true, 4);
        assert!(launch_result.toggle_launch_on_startup);
    }

    #[test]
    fn home_menu_selection_stops_at_last_option() {
        let mut page = PageState::new();
        page.open_home_menu(home_menu_layout());

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
        page.open_home_menu(home_menu_layout());

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        assert_eq!(page.home_menu_selected, 2);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn second_down_moves_home_menu_selection_to_resolution_row() {
        let mut page = PageState::new();
        page.open_home_menu(home_menu_layout());

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        assert_eq!(page.home_menu_selected, 4);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn down_moves_home_menu_selection_to_resolution_row() {
        let mut page = PageState::new();
        page.open_home_menu(home_menu_layout());

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        assert_eq!(page.home_menu_selected, 2);
    }

    #[test]
    fn up_returns_home_menu_selection_to_top_row() {
        let mut page = PageState::new();
        page.open_home_menu(home_menu_layout());
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Up, 3, true, 4);

        assert_eq!(page.home_menu_selected, 0);
    }

    #[test]
    fn launch_on_default_home_menu_option_sends_app_to_background() {
        let mut page = PageState::new();
        page.open_home_menu(home_menu_layout());

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
        page.open_home_menu(home_menu_layout());
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
        page.open_home_menu(home_menu_layout());
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
        page.open_home_menu(home_menu_layout());
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
        page.open_home_menu(home_menu_layout());
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
        page.open_home_menu(home_menu_layout());
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
        page.open_home_menu(home_menu_layout());
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
    fn single_external_app_is_reachable_from_left_of_main_menu() {
        let mut page = PageState::new();
        page.open_home_menu(HomeMenuLayout::new(
            &[ExternalAppKind::DlssSwapper],
            cfg!(target_os = "windows"),
            cfg!(target_os = "windows"),
        ));

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        #[cfg(target_os = "windows")]
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(result.launch_external_app, Some(ExternalAppKind::DlssSwapper));
        assert!(!page.show_home_menu());
    }

    #[test]
    fn right_column_does_not_jump_to_missing_external_slot() {
        let mut page = PageState::new();
        page.open_home_menu(HomeMenuLayout::new(
            &[ExternalAppKind::DlssSwapper],
            cfg!(target_os = "windows"),
            cfg!(target_os = "windows"),
        ));

        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        #[cfg(target_os = "windows")]
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Down, 3, true, 4);
        let _ = page.handle_action(&ControllerAction::Right, 3, true, 4);

        let result = page.handle_action(&ControllerAction::Launch, 3, true, 4);

        assert_eq!(result.launch_external_app, Some(ExternalAppKind::DlssSwapper));
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