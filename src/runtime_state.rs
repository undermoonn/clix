use std::time::Instant;

use crate::input::FOCUS_COOLDOWN_MS;

pub const HOLD_TO_EXIT_APP_MS: f32 = 800.0;
pub const HOLD_TO_FORCE_CLOSE_GAME_MS: f32 = 2000.0;

pub struct RuntimeState {
    had_focus: bool,
    focus_cooldown_until: Option<Instant>,
    quit_hold_started_at: Option<Instant>,
    quit_hold_progress: f32,
    quit_hold_consumed: bool,
    force_close_hold_started_at: Option<Instant>,
    force_close_hold_progress: f32,
    force_close_hold_consumed: bool,
    suppress_quit_hold_until_release: bool,
}

pub struct FocusUpdate {
    pub should_clear_input: bool,
    pub in_cooldown: bool,
}

pub struct QuitHoldUpdate {
    pub trigger_quit: bool,
    pub should_repaint: bool,
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
            quit_hold_started_at: None,
            quit_hold_progress: 0.0,
            quit_hold_consumed: false,
            force_close_hold_started_at: None,
            force_close_hold_progress: 0.0,
            force_close_hold_consumed: false,
            suppress_quit_hold_until_release: false,
        }
    }

    pub fn update_focus(&mut self, has_focus: bool, now: Instant) -> FocusUpdate {
        let mut should_clear_input = false;

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
        }
    }

    pub fn update_quit_hold(
        &mut self,
        process_input: bool,
        achievement_panel_open: bool,
        quit_held: bool,
        now: Instant,
    ) -> QuitHoldUpdate {
        if self.suppress_quit_hold_until_release {
            if quit_held {
                self.quit_hold_started_at = None;
                self.quit_hold_progress = 0.0;
                self.quit_hold_consumed = false;
            } else {
                self.suppress_quit_hold_until_release = false;
            }

            return QuitHoldUpdate {
                trigger_quit: false,
                should_repaint: false,
            };
        }

        if process_input && !achievement_panel_open && quit_held {
            if self.quit_hold_consumed {
                self.quit_hold_progress = 1.0;
                return QuitHoldUpdate {
                    trigger_quit: false,
                    should_repaint: false,
                };
            }

            let started_at = self.quit_hold_started_at.get_or_insert(now);
            let held_ms = now.duration_since(*started_at).as_secs_f32() * 1000.0;
            self.quit_hold_progress = (held_ms / HOLD_TO_EXIT_APP_MS).clamp(0.0, 1.0);

            if self.quit_hold_progress >= 1.0 {
                self.quit_hold_consumed = true;
                QuitHoldUpdate {
                    trigger_quit: true,
                    should_repaint: false,
                }
            } else {
                QuitHoldUpdate {
                    trigger_quit: false,
                    should_repaint: true,
                }
            }
        } else {
            self.quit_hold_started_at = None;
            self.quit_hold_progress = 0.0;
            self.quit_hold_consumed = false;
            QuitHoldUpdate {
                trigger_quit: false,
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

    pub fn suppress_quit_hold_until_release(&mut self) {
        self.suppress_quit_hold_until_release = true;
    }

    pub fn quit_hold_progress(&self) -> f32 {
        self.quit_hold_progress
    }

    pub fn force_close_hold_progress(&self) -> f32 {
        self.force_close_hold_progress
    }
}