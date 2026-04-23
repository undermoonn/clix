use std::time::Instant;

use crate::input::FOCUS_COOLDOWN_MS;

pub const HOLD_TO_FORCE_CLOSE_GAME_MS: f32 = 2000.0;

pub struct RuntimeState {
    had_focus: bool,
    focus_cooldown_until: Option<Instant>,
    home_button_was_held: bool,
    force_close_hold_started_at: Option<Instant>,
    force_close_hold_progress: f32,
    force_close_hold_consumed: bool,
    suppress_home_press_until_release: bool,
}

pub struct FocusUpdate {
    pub should_clear_input: bool,
    pub in_cooldown: bool,
    pub did_gain_focus: bool,
    pub did_lose_focus: bool,
}

pub struct ForceCloseHoldUpdate {
    pub trigger_force_close: bool,
    pub should_repaint: bool,
}

impl RuntimeState {
    pub fn new() -> Self {
        Self {
            had_focus: true,
            focus_cooldown_until: None,
            home_button_was_held: false,
            force_close_hold_started_at: None,
            force_close_hold_progress: 0.0,
            force_close_hold_consumed: false,
            suppress_home_press_until_release: false,
        }
    }

    pub fn update_focus(&mut self, has_focus: bool, now: Instant) -> FocusUpdate {
        let mut should_clear_input = false;
        let did_gain_focus = has_focus && !self.had_focus;
        let did_lose_focus = !has_focus && self.had_focus;

        if has_focus {
            if !self.had_focus {
                self.focus_cooldown_until = Some(now);
                should_clear_input = true;
            }
        } else {
            should_clear_input = true;
        }

        self.had_focus = has_focus;

        let in_cooldown = match self.focus_cooldown_until {
            Some(timestamp) => {
                if now.duration_since(timestamp).as_millis() < FOCUS_COOLDOWN_MS {
                    true
                } else {
                    self.focus_cooldown_until = None;
                    false
                }
            }
            None => false,
        };

        FocusUpdate {
            should_clear_input,
            in_cooldown,
            did_gain_focus,
            did_lose_focus,
        }
    }

    pub fn update_home_button(
        &mut self,
        process_input: bool,
        power_menu_open: bool,
        guide_held: bool,
    ) {
        let just_pressed = guide_held && !self.home_button_was_held;
        self.home_button_was_held = guide_held;

        if self.suppress_home_press_until_release {
            if !guide_held {
                self.suppress_home_press_until_release = false;
            }

            return;
        }

        if process_input && !power_menu_open && just_pressed {
            self.suppress_home_press_until_release = true;
            return;
        }
    }

    pub fn update_force_close_hold(
        &mut self,
        process_input: bool,
        force_close_available: bool,
        force_close_held: bool,
        now: Instant,
    ) -> ForceCloseHoldUpdate {
        if process_input && force_close_available && force_close_held {
            if self.force_close_hold_consumed {
                self.force_close_hold_progress = 1.0;
                return ForceCloseHoldUpdate {
                    trigger_force_close: false,
                    should_repaint: false,
                };
            }

            let started_at = self.force_close_hold_started_at.get_or_insert(now);
            let held_ms = now.duration_since(*started_at).as_secs_f32() * 1000.0;
            self.force_close_hold_progress = (held_ms / HOLD_TO_FORCE_CLOSE_GAME_MS).clamp(0.0, 1.0);

            if self.force_close_hold_progress >= 1.0 {
                self.force_close_hold_consumed = true;
                ForceCloseHoldUpdate {
                    trigger_force_close: true,
                    should_repaint: false,
                }
            } else {
                ForceCloseHoldUpdate {
                    trigger_force_close: false,
                    should_repaint: true,
                }
            }
        } else {
            self.force_close_hold_started_at = None;
            self.force_close_hold_progress = 0.0;
            self.force_close_hold_consumed = false;
            ForceCloseHoldUpdate {
                trigger_force_close: false,
                should_repaint: false,
            }
        }
    }

    pub fn suppress_home_hold_until_release(&mut self) {
        self.suppress_home_press_until_release = true;
    }

    pub fn force_close_hold_progress(&self) -> f32 {
        self.force_close_hold_progress
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::RuntimeState;

    #[test]
    fn home_button_press_is_suppressed_until_release() {
        let mut runtime = RuntimeState::new();

        runtime.update_home_button(true, false, true);
        assert!(runtime.suppress_home_press_until_release);

        runtime.update_home_button(true, false, true);
        assert!(runtime.suppress_home_press_until_release);

        runtime.update_home_button(true, false, false);
        assert!(!runtime.suppress_home_press_until_release);
    }

    #[test]
    fn focus_update_reports_focus_loss_once() {
        let mut runtime = RuntimeState::new();
        let start = Instant::now();

        let lost = runtime.update_focus(false, start);
        assert!(lost.did_lose_focus);
        assert!(lost.should_clear_input);
        assert!(!lost.did_gain_focus);

        let still_unfocused = runtime.update_focus(false, start + Duration::from_millis(16));
        assert!(!still_unfocused.did_lose_focus);
        assert!(still_unfocused.should_clear_input);
    }

}