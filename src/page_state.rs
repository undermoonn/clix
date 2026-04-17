use eframe::egui;

use crate::input::ControllerAction;

pub struct PageActionResult {
    pub open_achievement_panel: bool,
    pub launch_selected: bool,
    pub selected_changed: bool,
    pub close_frame: bool,
    pub suppress_quit_hold_until_release: bool,
}

pub struct PageState {
    selected: usize,
    cover_nav_dir: f32,
    select_anim: f32,
    select_anim_target: Option<usize>,
    wake_anim: f32,
    wake_anim_running: bool,
    scroll_offset: f32,
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

    pub fn handle_action(
        &mut self,
        action: &ControllerAction,
        games_len: usize,
        can_open_achievement_panel: bool,
        achievement_len: usize,
    ) -> PageActionResult {
        let mut result = PageActionResult {
            open_achievement_panel: false,
            launch_selected: false,
            selected_changed: false,
            close_frame: false,
            suppress_quit_hold_until_release: false,
        };

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
                    result.suppress_quit_hold_until_release = true;
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
            ControllerAction::Quit => {
                result.close_frame = true;
            }
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
}

#[cfg(test)]
mod tests {
    use super::PageState;
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
        assert!(!up_result.suppress_quit_hold_until_release);
        assert!(!page.show_achievement_panel());
        assert_eq!(page.achievement_selected(), 0);
    }
}