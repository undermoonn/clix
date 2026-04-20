use std::time::Instant;

use crate::input::FOCUS_COOLDOWN_MS;

pub const HOLD_TO_OPEN_HOME_MENU_MS: f32 = 500.0;
pub const HOLD_TO_FORCE_CLOSE_GAME_MS: f32 = 2000.0;
pub const HOLD_TO_SHUTDOWN_MS: f32 = 500.0;

pub struct RuntimeState {
    had_focus: bool,
    focus_cooldown_until: Option<Instant>,
    home_hold_started_at: Option<Instant>,
    home_hold_consumed: bool,
    shutdown_hold_started_at: Option<Instant>,
    shutdown_hold_progress: f32,
    shutdown_hold_consumed: bool,
    force_close_hold_started_at: Option<Instant>,
    force_close_hold_progress: f32,
    force_close_hold_consumed: bool,
    suppress_home_hold_until_release: bool,
}

pub struct FocusUpdate {
    pub should_clear_input: bool,
    pub in_cooldown: bool,
    pub did_gain_focus: bool,
}

pub struct HomeHoldUpdate {
    pub trigger_menu: bool,
    pub should_repaint: bool,
}

pub struct ForceCloseHoldUpdate {
    pub trigger_force_close: bool,
    pub should_repaint: bool,
}

pub struct ShutdownHoldUpdate {
    pub trigger_shutdown: bool,
    pub should_repaint: bool,
}

impl RuntimeState {
    pub fn new() -> Self {
        Self {
            had_focus: true,
            focus_cooldown_until: None,
            home_hold_started_at: None,
            home_hold_consumed: false,
            shutdown_hold_started_at: None,
            shutdown_hold_progress: 0.0,
            shutdown_hold_consumed: false,
            force_close_hold_started_at: None,
            force_close_hold_progress: 0.0,
            force_close_hold_consumed: false,
            suppress_home_hold_until_release: false,
        }
    }

    pub fn update_focus(&mut self, has_focus: bool, now: Instant) -> FocusUpdate {
        let mut should_clear_input = false;
        let did_gain_focus = has_focus && !self.had_focus;

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
        }
    }

    pub fn update_home_hold(
        &mut self,
        process_input: bool,
        home_menu_open: bool,
        guide_held: bool,
        now: Instant,
    ) -> HomeHoldUpdate {
        if self.suppress_home_hold_until_release {
            if guide_held {
                self.home_hold_started_at = None;
                self.home_hold_consumed = false;
            } else {
                self.suppress_home_hold_until_release = false;
            }

            return HomeHoldUpdate {
                trigger_menu: false,
                should_repaint: guide_held,
            };
        }

        if process_input && !home_menu_open && guide_held {
            if self.home_hold_consumed {
                return HomeHoldUpdate {
                    trigger_menu: false,
                    should_repaint: false,
                };
            }

            let started_at = self.home_hold_started_at.get_or_insert(now);
            let held_ms = now.duration_since(*started_at).as_secs_f32() * 1000.0;

            if held_ms >= HOLD_TO_OPEN_HOME_MENU_MS {
                self.home_hold_consumed = true;
                self.suppress_home_hold_until_release = true;
                HomeHoldUpdate {
                    trigger_menu: true,
                    should_repaint: false,
                }
            } else {
                HomeHoldUpdate {
                    trigger_menu: false,
                    should_repaint: true,
                }
            }
        } else {
            self.home_hold_started_at = None;
            self.home_hold_consumed = false;
            HomeHoldUpdate {
                trigger_menu: false,
                should_repaint: false,
            }
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

    pub fn update_shutdown_hold(
        &mut self,
        process_input: bool,
        home_menu_open: bool,
        shutdown_selected: bool,
        launch_held: bool,
        now: Instant,
    ) -> ShutdownHoldUpdate {
        if process_input && home_menu_open && shutdown_selected && launch_held {
            if self.shutdown_hold_consumed {
                self.shutdown_hold_progress = 1.0;
                return ShutdownHoldUpdate {
                    trigger_shutdown: false,
                    should_repaint: false,
                };
            }

            let started_at = self.shutdown_hold_started_at.get_or_insert(now);
            let held_ms = now.duration_since(*started_at).as_secs_f32() * 1000.0;
            self.shutdown_hold_progress = (held_ms / HOLD_TO_SHUTDOWN_MS).clamp(0.0, 1.0);

            if self.shutdown_hold_progress >= 1.0 {
                self.shutdown_hold_consumed = true;
                ShutdownHoldUpdate {
                    trigger_shutdown: true,
                    should_repaint: false,
                }
            } else {
                ShutdownHoldUpdate {
                    trigger_shutdown: false,
                    should_repaint: true,
                }
            }
        } else {
            self.shutdown_hold_started_at = None;
            self.shutdown_hold_progress = 0.0;
            self.shutdown_hold_consumed = false;
            ShutdownHoldUpdate {
                trigger_shutdown: false,
                should_repaint: false,
            }
        }
    }

    pub fn suppress_home_hold_until_release(&mut self) {
        self.suppress_home_hold_until_release = true;
    }

    pub fn shutdown_hold_progress(&self) -> f32 {
        self.shutdown_hold_progress
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
    fn shutdown_hold_triggers_after_half_second() {
        let mut runtime = RuntimeState::new();
        let start = Instant::now();

        let first = runtime.update_shutdown_hold(true, true, true, true, start);
        assert!(!first.trigger_shutdown);
        assert!(first.should_repaint);
        assert_eq!(runtime.shutdown_hold_progress(), 0.0);

        let mid = runtime.update_shutdown_hold(
            true,
            true,
            true,
            true,
            start + Duration::from_millis(250),
        );
        assert!(!mid.trigger_shutdown);
        assert!(mid.should_repaint);
        assert!(runtime.shutdown_hold_progress() > 0.4);

        let done = runtime.update_shutdown_hold(
            true,
            true,
            true,
            true,
            start + Duration::from_millis(500),
        );
        assert!(done.trigger_shutdown);
        assert!(!done.should_repaint);
        assert_eq!(runtime.shutdown_hold_progress(), 1.0);
    }

    #[test]
    fn shutdown_hold_resets_after_release() {
        let mut runtime = RuntimeState::new();
        let start = Instant::now();

        let _ = runtime.update_shutdown_hold(true, true, true, true, start);
        let _ = runtime.update_shutdown_hold(
            true,
            true,
            true,
            true,
            start + Duration::from_millis(250),
        );
        assert!(runtime.shutdown_hold_progress() > 0.0);

        let reset = runtime.update_shutdown_hold(
            true,
            true,
            true,
            false,
            start + Duration::from_millis(300),
        );
        assert!(!reset.trigger_shutdown);
        assert!(!reset.should_repaint);
        assert_eq!(runtime.shutdown_hold_progress(), 0.0);
    }

    #[test]
    fn home_hold_requests_repaint_while_counting_down() {
        let mut runtime = RuntimeState::new();
        let start = Instant::now();

        let first = runtime.update_home_hold(true, false, true, start);
        assert!(!first.trigger_menu);
        assert!(first.should_repaint);

        let done = runtime.update_home_hold(true, false, true, start + Duration::from_millis(500));
        assert!(done.trigger_menu);
        assert!(!done.should_repaint);
    }

}