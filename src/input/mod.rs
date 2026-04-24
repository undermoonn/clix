mod buttons;
mod dualsense;
mod xinput;

use std::collections::HashMap;
use std::time::Instant;
#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};

use buttons::Buttons;
use crate::config::PromptIconTheme;

#[cfg(target_os = "windows")]
use eframe::egui;
#[cfg(target_os = "windows")]
use winapi::shared::minwindef::DWORD;

pub const NAV_INITIAL_DELAY_MS: u128 = 350;
pub const NAV_REPEAT_INTERVAL_MS: u128 = 120;
pub const NAV_REPEAT_ACCEL_STAGE1_AFTER_MS: u128 = 500;
pub const NAV_REPEAT_ACCEL_STAGE2_AFTER_MS: u128 = 1300;
pub const NAV_REPEAT_INTERVAL_STAGE1_MS: u128 = 80;
pub const NAV_REPEAT_INTERVAL_STAGE2_MS: u128 = 45;
pub const FOCUS_COOLDOWN_MS: u128 = 100;

#[cfg(target_os = "windows")]
static BACKGROUND_HOME_WAKE_ENABLED: AtomicBool = AtomicBool::new(true);

#[cfg(target_os = "windows")]
pub fn start_watchers(ctx: egui::Context) {
    xinput::start(ctx.clone());
    dualsense::start(ctx);
}

#[cfg(not(target_os = "windows"))]
pub fn start_watchers(_ctx: eframe::egui::Context) {}

#[cfg(target_os = "windows")]
pub fn set_background_home_wake_enabled(enabled: bool) {
    BACKGROUND_HOME_WAKE_ENABLED.store(enabled, Ordering::Release);
}

#[cfg(not(target_os = "windows"))]
pub fn set_background_home_wake_enabled(_enabled: bool) {}

#[cfg(target_os = "windows")]
pub(super) fn background_home_wake_enabled() -> bool {
    BACKGROUND_HOME_WAKE_ENABLED.load(Ordering::Acquire)
}

#[cfg(not(target_os = "windows"))]
pub(super) fn background_home_wake_enabled() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn take_wake_request() -> bool {
    xinput::take_wake_request() || dualsense::take_wake_request()
}

#[cfg(not(target_os = "windows"))]
pub fn take_wake_request() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn home_held() -> bool {
    xinput::home_held() || dualsense::home_held()
}

#[cfg(not(target_os = "windows"))]
pub fn home_held() -> bool {
    false
}

pub enum ControllerAction {
    Up,
    Down,
    Left,
    Right,
    Launch,
    Refresh,
    Settings,
    Quit,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum InputAction {
    Up,
    Down,
    Left,
    Right,
    Launch,
    Refresh,
    Settings,
    Quit,
    ForceClose,
}

impl InputAction {
    const COUNT: usize = 9;
    const ALL: [Self; Self::COUNT] = [
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::Launch,
        Self::Refresh,
        Self::Settings,
        Self::Quit,
        Self::ForceClose,
    ];
    const POLLABLE_ACTIONS: [Self; 7] = [
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::Launch,
        Self::Refresh,
        Self::Settings,
    ];
    const POLLABLE_ACTIONS_WITH_QUIT: [Self; 8] = [
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::Launch,
        Self::Refresh,
        Self::Settings,
        Self::Quit,
    ];

    fn index(self) -> usize {
        self as usize
    }

    fn bit(self) -> u16 {
        1u16 << (self as u16)
    }

    fn from_mapping_key(value: &str) -> Option<Self> {
        match value {
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "launch" => Some(Self::Launch),
            "refresh" => Some(Self::Refresh),
            "settings" => Some(Self::Settings),
            "quit" => Some(Self::Quit),
            "force_close" => Some(Self::ForceClose),
            _ => None,
        }
    }

    fn from_vertical_mapping_key(value: &str) -> Option<Self> {
        match value {
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            _ => None,
        }
    }

    fn to_controller_action(self) -> Option<ControllerAction> {
        match self {
            Self::Up => Some(ControllerAction::Up),
            Self::Down => Some(ControllerAction::Down),
            Self::Left => Some(ControllerAction::Left),
            Self::Right => Some(ControllerAction::Right),
            Self::Launch => Some(ControllerAction::Launch),
            Self::Refresh => Some(ControllerAction::Refresh),
            Self::Settings => Some(ControllerAction::Settings),
            Self::Quit => Some(ControllerAction::Quit),
            Self::ForceClose => None,
        }
    }

    fn repeats(self) -> bool {
        !matches!(self, Self::Refresh | Self::Settings)
    }

    fn pollable_actions(include_quit_action: bool) -> &'static [Self] {
        if include_quit_action {
            &Self::POLLABLE_ACTIONS_WITH_QUIT
        } else {
            &Self::POLLABLE_ACTIONS
        }
    }
}

#[derive(Clone, Copy, Default)]
struct RawHeldState(u16);

impl RawHeldState {
    fn insert(&mut self, action: InputAction) {
        self.0 |= action.bit();
    }

    fn contains(&self, action: InputAction) -> bool {
        (self.0 & action.bit()) != 0
    }
}

#[cfg(target_os = "windows")]
#[derive(Default)]
struct InputAggregateState {
    buttons: Buttons,
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

#[cfg(target_os = "windows")]
impl InputAggregateState {
    fn from_state(buttons: Buttons, ly: i32, lx: i32) -> Self {
        Self {
            buttons,
            up: ly > 16000,
            down: ly < -16000,
            left: lx < -16000,
            right: lx > 16000,
        }
    }

    fn from_states(states: &[(DWORD, Buttons, i32, i32)]) -> Self {
        let mut aggregate = Self::default();

        for (_, buttons, ly, lx) in states {
            aggregate.buttons |= *buttons;
            aggregate.up |= *ly > 16000;
            aggregate.down |= *ly < -16000;
            aggregate.right |= *lx > 16000;
            aggregate.left |= *lx < -16000;
        }

        aggregate
    }

    fn populate_raw_held(&self, raw_held: &mut RawHeldState) {
        if self.buttons.intersects(Buttons::DPAD_UP) || self.up {
            raw_held.insert(InputAction::Up);
        }
        if self.buttons.intersects(Buttons::DPAD_DOWN) || self.down {
            raw_held.insert(InputAction::Down);
        }
        if self.buttons.intersects(Buttons::DPAD_LEFT) || self.left {
            raw_held.insert(InputAction::Left);
        }
        if self.buttons.intersects(Buttons::DPAD_RIGHT) || self.right {
            raw_held.insert(InputAction::Right);
        }
        if self.buttons.intersects(Buttons::A) {
            raw_held.insert(InputAction::Launch);
        }
        if self.buttons.intersects(Buttons::B) {
            raw_held.insert(InputAction::Quit);
        }
        if self.buttons.intersects(Buttons::X) {
            raw_held.insert(InputAction::Refresh);
            raw_held.insert(InputAction::ForceClose);
        }
        if self.buttons.intersects(Buttons::Y) {
            raw_held.insert(InputAction::Settings);
        }
    }

    fn has_repaint_activity(&self) -> bool {
        !self.buttons.is_empty() || self.up || self.down || self.left || self.right
    }
}

#[derive(Clone, Copy)]
pub struct NavState {
    pub since: Instant,
    pub last_fire: Instant,
    pub past_initial: bool,
}

pub struct InputFrame {
    pub actions: Vec<ControllerAction>,
    pub launch_held: bool,
    pub force_close_held: bool,
    pub has_controller_activity: bool,
    pub prompt_icon_theme: Option<PromptIconTheme>,
}

pub struct InputController {
    #[cfg(target_os = "windows")]
    xinput: Option<xinput::XInput>,
    current_prompt_icon_theme: Option<PromptIconTheme>,
    current_rumble_prompt_icon_theme: Option<PromptIconTheme>,
    selection_vibration_enabled: bool,
    mapping: Mapping,
    remap_target: Option<String>,
    nav_held: [Option<NavState>; InputAction::COUNT],
}

#[derive(Debug, Clone, Copy)]
pub enum VerticalAxisDirection {
    Negative,
    Positive,
}

impl VerticalAxisDirection {
    #[cfg(target_os = "windows")]
    fn from_thumb_ly(ly: i32) -> Option<Self> {
        if ly < -16000 {
            Some(Self::Negative)
        } else if ly > 16000 {
            Some(Self::Positive)
        } else {
            None
        }
    }

    #[cfg(target_os = "windows")]
    fn is_active(self, aggregate: &InputAggregateState) -> bool {
        match self {
            Self::Negative => aggregate.down,
            Self::Positive => aggregate.up,
        }
    }
}

impl InputController {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            xinput: xinput::XInput::new().ok(),
            current_prompt_icon_theme: None,
            current_rumble_prompt_icon_theme: None,
            selection_vibration_enabled: false,
            mapping: Mapping::default(),
            remap_target: None,
            nav_held: [None; InputAction::COUNT],
        }
    }

    pub fn set_selection_vibration_enabled(&mut self, enabled: bool) {
        self.selection_vibration_enabled = enabled;

        if !enabled {
            #[cfg(target_os = "windows")]
            self.stop_rumble();
        }
    }

    pub fn clear_held(&mut self) {
        self.nav_held.fill(None);
        #[cfg(target_os = "windows")]
        self.stop_rumble();
    }

    pub fn tick(&mut self) {
        #[cfg(target_os = "windows")]
        self.tick_rumble();
    }

    pub fn pulse_selection_change(&mut self) {
        if !self.selection_vibration_enabled {
            return;
        }

        #[cfg(target_os = "windows")]
        self.start_selection_rumble();
    }

    pub fn poll(&mut self, process_input: bool, include_quit_action: bool) -> InputFrame {
        let mut raw_held = RawHeldState::default();
        let (prompt_icon_theme, has_controller_activity) = self.collect_raw_held(&mut raw_held);

        self.poll_with_raw_held(
            raw_held,
            process_input,
            include_quit_action,
            has_controller_activity,
            prompt_icon_theme,
            Instant::now(),
        )
    }

    fn poll_with_raw_held(
        &mut self,
        raw_held: RawHeldState,
        process_input: bool,
        include_quit_action: bool,
        has_controller_activity: bool,
        prompt_icon_theme: Option<PromptIconTheme>,
        now: Instant,
    ) -> InputFrame {
        let mut actions = Vec::new();

        if !process_input {
            self.nav_held.fill(None);
        }

        if process_input {
            for action in InputAction::ALL {
                if !raw_held.contains(action) {
                    self.nav_held[action.index()] = None;
                }
            }
        }

        if process_input {
            for action in InputAction::pollable_actions(include_quit_action) {
                if !raw_held.contains(*action) {
                    continue;
                }

                let state_slot = &mut self.nav_held[action.index()];
                let should_fire = if let Some(state) = state_slot.as_mut() {
                    if !action.repeats() {
                        false
                    } else if !state.past_initial {
                        if now.duration_since(state.since).as_millis() >= NAV_INITIAL_DELAY_MS {
                            state.past_initial = true;
                            state.last_fire = now;
                            true
                        } else {
                            false
                        }
                    } else {
                        let held_ms = now.duration_since(state.since).as_millis();
                        let repeat_interval_ms = nav_repeat_interval_ms(*action, held_ms);

                        if now.duration_since(state.last_fire).as_millis() >= repeat_interval_ms {
                            state.last_fire = now;
                            true
                        } else {
                            false
                        }
                    }
                } else {
                    *state_slot = Some(NavState {
                        since: now,
                        last_fire: now,
                        past_initial: false,
                    });
                    true
                };

                if should_fire {
                    if let Some(action) = action.to_controller_action() {
                        actions.push(action);
                    }
                }
            }
        }

        InputFrame {
            actions,
            launch_held: raw_held.contains(InputAction::Launch),
            force_close_held: raw_held.contains(InputAction::ForceClose),
            has_controller_activity,
            prompt_icon_theme,
        }
    }

    fn collect_raw_held(&mut self, raw_held: &mut RawHeldState) -> (Option<PromptIconTheme>, bool) {
        let mut prompt_icon_theme = None;
        let mut has_controller_activity = false;

        #[cfg(target_os = "windows")]
        {
            if self.collect_xinput(raw_held) {
                prompt_icon_theme = Some(PromptIconTheme::Xbox);
                has_controller_activity = true;
            }
            if self.collect_dualsense(raw_held) {
                prompt_icon_theme = Some(PromptIconTheme::PlayStation);
                has_controller_activity = true;
            }

            self.current_prompt_icon_theme = prompt_icon_theme;
        }

        (prompt_icon_theme, has_controller_activity)
    }

    #[cfg(target_os = "windows")]
    fn start_selection_rumble(&mut self) {
        self.stop_rumble();

        match self.current_prompt_icon_theme {
            Some(PromptIconTheme::Xbox) => {
                if let Some(xinput) = self.xinput.as_mut() {
                    if xinput.start_selection_rumble() {
                        self.current_rumble_prompt_icon_theme = Some(PromptIconTheme::Xbox);
                    }
                }
            }
            Some(PromptIconTheme::PlayStation) => {
                if dualsense::start_selection_rumble() {
                    self.current_rumble_prompt_icon_theme = Some(PromptIconTheme::PlayStation);
                }
            }
            None => {}
        }
    }

    #[cfg(target_os = "windows")]
    fn tick_rumble(&mut self) {
        match self.current_rumble_prompt_icon_theme {
            Some(PromptIconTheme::Xbox) => {
                if let Some(xinput) = self.xinput.as_mut() {
                    xinput.tick_rumble();
                }
            }
            Some(PromptIconTheme::PlayStation) => {
                dualsense::tick_rumble();
            }
            None => {}
        }
    }

    #[cfg(target_os = "windows")]
    fn stop_rumble(&mut self) {
        match self.current_rumble_prompt_icon_theme {
            Some(PromptIconTheme::Xbox) => {
                if let Some(xinput) = self.xinput.as_mut() {
                    xinput.stop_rumble();
                }
            }
            Some(PromptIconTheme::PlayStation) => {
                dualsense::stop_rumble();
            }
            None => {}
        }

        self.current_rumble_prompt_icon_theme = None;
    }

    #[cfg(target_os = "windows")]
    fn collect_xinput(&mut self, raw_held: &mut RawHeldState) -> bool {
        let remap_state = {
            let Some(xinput) = self.xinput.as_mut() else {
                return false;
            };
            xinput::remap_state(xinput)
        };
        let has_remap_input = remap_state.is_some();

        if self.try_remap_input_state(remap_state) {
            return has_remap_input;
        }

        let aggregate = {
            let Some(xinput) = self.xinput.as_mut() else {
                return false;
            };
            xinput::aggregate_state(xinput)
        };

        let Some(aggregate) = aggregate else {
            return false;
        };
        aggregate.populate_raw_held(raw_held);

        self.apply_mapping(raw_held, &aggregate);

        aggregate.has_repaint_activity()
    }

    #[cfg(target_os = "windows")]
    fn collect_dualsense(&mut self, raw_held: &mut RawHeldState) -> bool {
        let remap_state = dualsense::remap_state();
        let has_remap_input = remap_state.is_some();

        if self.try_remap_input_state(remap_state) {
            return has_remap_input;
        }

        let Some(aggregate) = dualsense::aggregate_state() else {
            return false;
        };
        aggregate.populate_raw_held(raw_held);
        self.apply_mapping(raw_held, &aggregate);

        aggregate.has_repaint_activity()
    }

    #[cfg(target_os = "windows")]
    fn try_remap_input_state(&mut self, state: Option<(Buttons, i32)>) -> bool {
        if self.remap_target.is_none() {
            return false;
        }

        let Some((buttons, ly)) = state else {
            return true;
        };

        if let Some(token) = InputToken::detect_input(buttons, ly) {
            let target = self.remap_target.take().unwrap();
            self.mapping.map.insert(target, token);
            return true;
        }

        true
    }

    #[cfg(target_os = "windows")]
    fn apply_mapping(
        &self,
        raw_held: &mut RawHeldState,
        aggregate: &InputAggregateState,
    ) {

        for (key, token) in &self.mapping.map {
            if !token.is_active(aggregate) {
                continue;
            }

            match token {
                InputToken::VerticalAxis(_) => {
                    if let Some(action) = InputAction::from_vertical_mapping_key(key) {
                        raw_held.insert(action);
                    }
                }
                _ => Self::insert_mapped_action(raw_held, key),
            }
        }
    }

    fn insert_mapped_action(
        raw_held: &mut RawHeldState,
        key: &str,
    ) {
        if let Some(action) = InputAction::from_mapping_key(key) {
            raw_held.insert(action);
        }
    }
}

fn nav_repeat_interval_ms(action: InputAction, held_ms: u128) -> u128 {
    match action {
        InputAction::Up | InputAction::Down | InputAction::Left | InputAction::Right => {
            if held_ms >= NAV_REPEAT_ACCEL_STAGE2_AFTER_MS {
                NAV_REPEAT_INTERVAL_STAGE2_MS
            } else if held_ms >= NAV_REPEAT_ACCEL_STAGE1_AFTER_MS {
                NAV_REPEAT_INTERVAL_STAGE1_MS
            } else {
                NAV_REPEAT_INTERVAL_MS
            }
        }
        _ => NAV_REPEAT_INTERVAL_MS,
    }
}

#[cfg(target_os = "windows")]
impl Drop for InputController {
    fn drop(&mut self) {
        self.stop_rumble();
    }
}

#[derive(Debug, Default)]
pub enum InputToken {
    #[default]
    None,
    Button(Buttons),
    VerticalAxis(VerticalAxisDirection),
}

impl InputToken {
    #[cfg(target_os = "windows")]
    fn detect_input(buttons: Buttons, ly: i32) -> Option<Self> {
        if !buttons.is_empty() {
            Some(Self::Button(buttons))
        } else {
            VerticalAxisDirection::from_thumb_ly(ly).map(Self::VerticalAxis)
        }
    }

    #[cfg(target_os = "windows")]
    fn is_active(&self, aggregate: &InputAggregateState) -> bool {
        match self {
            Self::Button(mask) => aggregate.buttons.intersects(*mask),
            Self::VerticalAxis(direction) => direction.is_active(aggregate),
            Self::None => false,
        }
    }
}

#[derive(Debug, Default)]
pub struct Mapping {
    pub map: HashMap<String, InputToken>,
}

#[cfg(test)]
mod tests {
    use super::{
        ControllerAction, InputAction, InputController, RawHeldState, NAV_INITIAL_DELAY_MS,
        NAV_REPEAT_ACCEL_STAGE1_AFTER_MS, NAV_REPEAT_INTERVAL_MS,
    };
    use std::time::{Duration, Instant};

    fn raw_held(actions: &[InputAction]) -> RawHeldState {
        let mut held = RawHeldState::default();
        for action in actions {
            held.insert(*action);
        }
        held
    }

    #[test]
    fn first_press_fires_immediately() {
        let mut input = InputController::new();
        let now = Instant::now();

        let frame = input.poll_with_raw_held(raw_held(&[InputAction::Down]), true, false, None, now);

        assert!(matches!(frame.actions.as_slice(), [ControllerAction::Down]));
    }

    #[test]
    fn held_navigation_waits_for_initial_delay_before_repeating() {
        let mut input = InputController::new();
        let now = Instant::now();

        let first = input.poll_with_raw_held(raw_held(&[InputAction::Right]), true, false, None, now);
        let before_delay = input.poll_with_raw_held(
            raw_held(&[InputAction::Right]),
            true,
            false,
            None,
            now + Duration::from_millis((NAV_INITIAL_DELAY_MS - 1) as u64),
        );
        let after_delay = input.poll_with_raw_held(
            raw_held(&[InputAction::Right]),
            true,
            false,
            None,
            now + Duration::from_millis((NAV_INITIAL_DELAY_MS + 1) as u64),
        );
        let before_repeat = input.poll_with_raw_held(
            raw_held(&[InputAction::Right]),
            true,
            false,
            None,
            now + Duration::from_millis((NAV_INITIAL_DELAY_MS + NAV_REPEAT_INTERVAL_MS - 1) as u64),
        );
        let after_repeat = input.poll_with_raw_held(
            raw_held(&[InputAction::Right]),
            true,
            false,
            None,
            now + Duration::from_millis((NAV_INITIAL_DELAY_MS + NAV_REPEAT_INTERVAL_MS + 1) as u64),
        );

        assert!(matches!(first.actions.as_slice(), [ControllerAction::Right]));
        assert!(before_delay.actions.is_empty());
        assert!(matches!(after_delay.actions.as_slice(), [ControllerAction::Right]));
        assert!(before_repeat.actions.is_empty());
        assert!(matches!(after_repeat.actions.as_slice(), [ControllerAction::Right]));
    }

    #[test]
    fn refresh_does_not_repeat_while_held() {
        let mut input = InputController::new();
        let now = Instant::now();
        let held_until = now + Duration::from_millis((NAV_REPEAT_ACCEL_STAGE1_AFTER_MS + 250) as u64);

        let first = input.poll_with_raw_held(
            raw_held(&[InputAction::Refresh]),
            true,
            false,
            None,
            now,
        );
        let held = input.poll_with_raw_held(
            raw_held(&[InputAction::Refresh]),
            true,
            false,
            None,
            held_until,
        );

        assert!(matches!(first.actions.as_slice(), [ControllerAction::Refresh]));
        assert!(held.actions.is_empty());
    }

    #[test]
    fn settings_does_not_repeat_while_held() {
        let mut input = InputController::new();
        let now = Instant::now();
        let held_until = now + Duration::from_millis((NAV_REPEAT_ACCEL_STAGE1_AFTER_MS + 250) as u64);

        let first = input.poll_with_raw_held(
            raw_held(&[InputAction::Settings]),
            true,
            false,
            None,
            now,
        );
        let held = input.poll_with_raw_held(
            raw_held(&[InputAction::Settings]),
            true,
            false,
            None,
            held_until,
        );

        assert!(matches!(first.actions.as_slice(), [ControllerAction::Settings]));
        assert!(held.actions.is_empty());
    }

    #[test]
    fn quit_is_only_emitted_when_enabled() {
        let mut input = InputController::new();
        let now = Instant::now();

        let blocked = input.poll_with_raw_held(
            raw_held(&[InputAction::Quit]),
            true,
            false,
            None,
            now,
        );
        let allowed = input.poll_with_raw_held(
            raw_held(&[InputAction::Quit]),
            true,
            true,
            None,
            now + Duration::from_millis(1),
        );

        assert!(blocked.actions.is_empty());
        assert!(matches!(allowed.actions.as_slice(), [ControllerAction::Quit]));
    }

    #[test]
    fn clear_held_resets_repeat_state() {
        let mut input = InputController::new();
        let now = Instant::now();

        let _ = input.poll_with_raw_held(raw_held(&[InputAction::Up]), true, false, None, now);
        input.clear_held();
        let frame = input.poll_with_raw_held(
            raw_held(&[InputAction::Up]),
            true,
            false,
            None,
            now + Duration::from_millis((NAV_INITIAL_DELAY_MS / 2) as u64),
        );

        assert!(matches!(frame.actions.as_slice(), [ControllerAction::Up]));
    }
}

