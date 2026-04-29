use std::time::Instant;

use crate::input::FOCUS_COOLDOWN_MS;

pub struct RuntimeState {
    had_focus: bool,
    focus_cooldown_until: Option<Instant>,
    home_button_was_held: bool,
    suppress_home_press_until_release: bool,
}

pub struct FocusUpdate {
    pub should_clear_input: bool,
    pub in_cooldown: bool,
    pub did_gain_focus: bool,
    pub did_lose_focus: bool,
}

impl RuntimeState {
    pub fn new() -> Self {
        Self {
            had_focus: true,
            focus_cooldown_until: None,
            home_button_was_held: false,
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

    pub fn suppress_home_hold_until_release(&mut self) {
        self.suppress_home_press_until_release = true;
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
