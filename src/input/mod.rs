pub(crate) mod xbox_home;

use std::collections::HashMap;
use std::time::Instant;

#[cfg(target_os = "windows")]
use std::sync::Once;

#[cfg(target_os = "windows")]
use eframe::egui;

#[cfg(target_os = "windows")]
use winapi::um::xinput::{
    XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_LEFT,
    XINPUT_GAMEPAD_DPAD_RIGHT, XINPUT_GAMEPAD_DPAD_UP, XINPUT_GAMEPAD_X, XINPUT_GAMEPAD_Y,
};

#[cfg(target_os = "windows")]
use libloading::Library;
#[cfg(target_os = "windows")]
use winapi::shared::minwindef::DWORD;
#[cfg(target_os = "windows")]
use winapi::um::xinput::{XINPUT_STATE, XINPUT_VIBRATION};

pub const NAV_INITIAL_DELAY_MS: u128 = 350;
pub const NAV_REPEAT_INTERVAL_MS: u128 = 120;
pub const NAV_REPEAT_ACCEL_STAGE1_AFTER_MS: u128 = 500;
pub const NAV_REPEAT_ACCEL_STAGE2_AFTER_MS: u128 = 1300;
pub const NAV_REPEAT_INTERVAL_STAGE1_MS: u128 = 80;
pub const NAV_REPEAT_INTERVAL_STAGE2_MS: u128 = 45;
pub const FOCUS_COOLDOWN_MS: u128 = 500;
#[cfg(target_os = "windows")]
const XINPUT_SELECTION_RUMBLE_DURATION_MS: u128 = 40;
#[cfg(target_os = "windows")]
const XINPUT_SELECTION_RUMBLE_LEFT_STRENGTH: u16 = 0;
#[cfg(target_os = "windows")]
const XINPUT_SELECTION_RUMBLE_RIGHT_STRENGTH: u16 = 10_000;

pub enum ControllerAction {
    Up,
    Down,
    Left,
    Right,
    Launch,
    Refresh,
    Sort,
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
    Sort,
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
        Self::Sort,
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
        Self::Sort,
    ];
    const POLLABLE_ACTIONS_WITH_QUIT: [Self; 8] = [
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::Launch,
        Self::Refresh,
        Self::Sort,
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
            "sort" => Some(Self::Sort),
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
            Self::Sort => Some(ControllerAction::Sort),
            Self::Quit => Some(ControllerAction::Quit),
            Self::ForceClose => None,
        }
    }

    fn repeats(self) -> bool {
        !matches!(self, Self::Refresh | Self::Sort)
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

    fn has_any(&self) -> bool {
        self.0 != 0
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
    pub has_input_activity: bool,
}

pub struct InputController {
    #[cfg(target_os = "windows")]
    xinput: Option<XInput>,
    #[cfg(target_os = "windows")]
    rumble_state: Option<RumbleState>,
    #[cfg(target_os = "windows")]
    last_connected_controller_index: Option<DWORD>,
    mapping: Mapping,
    remap_target: Option<String>,
    nav_held: [Option<NavState>; InputAction::COUNT],
}

#[cfg(target_os = "windows")]
pub fn start_repaint_watcher(ctx: egui::Context) {
    static START_ONCE: Once = Once::new();

    START_ONCE.call_once(move || {
        std::thread::spawn(move || run_xinput_repaint_watcher(ctx));
    });
}

#[cfg(not(target_os = "windows"))]
pub fn start_repaint_watcher(_ctx: eframe::egui::Context) {}

#[cfg(target_os = "windows")]
enum RumbleState {
    XInput {
        controller_index: DWORD,
        active_until: Instant,
    },
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
struct RumbleSettings {
    duration_ms: u128,
    left_strength: u16,
    right_strength: u16,
}

#[cfg(target_os = "windows")]
#[derive(Default)]
struct XInputAggregateState {
    buttons: u16,
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

#[cfg(target_os = "windows")]
impl XInputAggregateState {
    fn from_states(states: &[(DWORD, u16, i32, i32)]) -> Self {
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
        if (self.buttons & XINPUT_GAMEPAD_DPAD_UP) != 0 || self.up {
            raw_held.insert(InputAction::Up);
        }
        if (self.buttons & XINPUT_GAMEPAD_DPAD_DOWN) != 0 || self.down {
            raw_held.insert(InputAction::Down);
        }
        if (self.buttons & XINPUT_GAMEPAD_DPAD_LEFT) != 0 || self.left {
            raw_held.insert(InputAction::Left);
        }
        if (self.buttons & XINPUT_GAMEPAD_DPAD_RIGHT) != 0 || self.right {
            raw_held.insert(InputAction::Right);
        }
        if (self.buttons & XINPUT_GAMEPAD_A) != 0 {
            raw_held.insert(InputAction::Launch);
        }
        if (self.buttons & XINPUT_GAMEPAD_B) != 0 {
            raw_held.insert(InputAction::Quit);
        }
        if (self.buttons & XINPUT_GAMEPAD_X) != 0 {
            raw_held.insert(InputAction::Refresh);
            raw_held.insert(InputAction::ForceClose);
        }
        if (self.buttons & XINPUT_GAMEPAD_Y) != 0 {
            raw_held.insert(InputAction::Sort);
        }
    }

    fn has_repaint_activity(&self) -> bool {
        self.buttons != 0 || self.up || self.down || self.left || self.right
    }
}

#[cfg(target_os = "windows")]
fn run_xinput_repaint_watcher(ctx: egui::Context) {
    let Ok(xinput) = XInput::new() else {
        return;
    };

    loop {
        let aggregate = XInputAggregateState::from_states(&xinput.get_states());
        if aggregate.has_repaint_activity() && !crate::launch::current_app_window_is_background() {
            ctx.request_repaint();
        }

        std::thread::sleep(std::time::Duration::from_millis(16));
    }
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
    fn is_active(self, aggregate: &XInputAggregateState) -> bool {
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
            xinput: XInput::new().ok(),
            #[cfg(target_os = "windows")]
            rumble_state: None,
            #[cfg(target_os = "windows")]
            last_connected_controller_index: None,
            mapping: Mapping::default(),
            remap_target: None,
            nav_held: [None; InputAction::COUNT],
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
        #[cfg(target_os = "windows")]
        self.start_selection_rumble();
    }

    pub fn poll(&mut self, process_input: bool, include_quit_action: bool) -> InputFrame {
        let mut raw_held = RawHeldState::default();

        self.collect_raw_held(&mut raw_held);

        self.poll_with_raw_held(raw_held, process_input, include_quit_action, Instant::now())
    }

    fn poll_with_raw_held(
        &mut self,
        raw_held: RawHeldState,
        process_input: bool,
        include_quit_action: bool,
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
            has_input_activity: raw_held.has_any(),
        }
    }

    fn collect_raw_held(&mut self, raw_held: &mut RawHeldState) {
        #[cfg(target_os = "windows")]
        {
            let _ = self.collect_xinput(raw_held);
        }
    }

    #[cfg(target_os = "windows")]
    fn start_selection_rumble(&mut self) {
        self.stop_rumble();
        let _ = self.start_xinput_selection_rumble();
    }

    #[cfg(target_os = "windows")]
    fn start_xinput_selection_rumble(&mut self) -> bool {
        let settings = xinput_rumble_settings();
        if self.xinput.is_none() {
            return false;
        }

        if let Some(controller_index) = self.last_connected_controller_index {
            if self.start_xinput_rumble(controller_index, settings) {
                return true;
            }
        }

        let controller_index = {
            let Some(xinput) = self.xinput.as_ref() else {
                return false;
            };
            xinput.first_connected_index()
        };

        let Some(controller_index) = controller_index else {
            return false;
        };

        self.start_xinput_rumble(controller_index, settings)
    }

    #[cfg(target_os = "windows")]
    fn start_xinput_rumble(
        &mut self,
        controller_index: DWORD,
        settings: RumbleSettings,
    ) -> bool {
        let started = {
            let Some(xinput) = self.xinput.as_ref() else {
                return false;
            };

            xinput
                .set_state(
                    controller_index,
                    settings.left_strength,
                    settings.right_strength,
                )
                .is_ok()
        };

        if started {
            self.last_connected_controller_index = Some(controller_index);
            self.rumble_state = Some(RumbleState::XInput {
                controller_index,
                active_until: Instant::now()
                    + std::time::Duration::from_millis(settings.duration_ms as u64),
            });
            true
        } else {
            false
        }
    }

    #[cfg(target_os = "windows")]
    fn tick_rumble(&mut self) {
        let should_stop = self
            .rumble_state
            .as_ref()
            .map(|state| match state {
                RumbleState::XInput { active_until, .. } => Instant::now() >= *active_until,
            })
            .unwrap_or(false);

        if should_stop {
            self.stop_rumble();
        }
    }

    #[cfg(target_os = "windows")]
    fn stop_rumble(&mut self) {
        let Some(state) = self.rumble_state.take() else {
            return;
        };

        match state {
            RumbleState::XInput { controller_index, .. } => {
                if let Some(xinput) = &self.xinput {
                    let _ = xinput.set_state(controller_index, 0, 0);
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn collect_xinput(&mut self, raw_held: &mut RawHeldState) -> bool {
        let Some(xinput) = &self.xinput else {
            self.last_connected_controller_index = None;
            return false;
        };

        let states = xinput.get_states();
        if states.is_empty() {
            self.last_connected_controller_index = None;
            return false;
        }

        self.last_connected_controller_index = states.first().map(|(index, _, _, _)| *index);

        if self.try_remap_xinput(&states) {
            return true;
        }

        let aggregate = XInputAggregateState::from_states(&states);
        aggregate.populate_raw_held(raw_held);

        self.apply_xinput_mapping(raw_held, &aggregate);

        true
    }

    #[cfg(target_os = "windows")]
    fn try_remap_xinput(&mut self, states: &[(DWORD, u16, i32, i32)]) -> bool {
        if self.remap_target.is_none() {
            return false;
        }

        for (_, buttons, ly, _) in states {
            if let Some(token) = InputToken::detect_xinput(*buttons, *ly) {
                let target = self.remap_target.take().unwrap();
                self.mapping.map.insert(target, token);
                return true;
            }
        }

        true
    }

    #[cfg(target_os = "windows")]
    fn apply_xinput_mapping(
        &self,
        raw_held: &mut RawHeldState,
        aggregate: &XInputAggregateState,
    ) {

        for (key, token) in &self.mapping.map {
            if !token.is_active_xinput(aggregate) {
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

#[cfg(target_os = "windows")]
pub struct XInput {
    _lib: Library,
    get_state: unsafe extern "system" fn(DWORD, *mut XINPUT_STATE) -> DWORD,
    _set_state: Option<unsafe extern "system" fn(DWORD, *mut XINPUT_VIBRATION) -> DWORD>,
}

#[cfg(target_os = "windows")]
impl XInput {
    pub fn new() -> Result<Self, ()> {
        let names = ["xinput1_4.dll", "xinput1_3.dll", "xinput9_1_0.dll"];
        for name in names {
            if let Ok(lib) = unsafe { Library::new(name) } {
                unsafe {
                    let gs_sym: libloading::Symbol<
                        unsafe extern "system" fn(DWORD, *mut XINPUT_STATE) -> DWORD,
                    > = lib.get(b"XInputGetState\0").map_err(|_| ())?;
                    let get_state_fn = *gs_sym;
                    let set_state_fn = match lib.get(b"XInputSetState\0") {
                        Ok(s) => Some(*s),
                        Err(_) => None,
                    };
                    return Ok(XInput {
                        _lib: lib,
                        get_state: get_state_fn,
                        _set_state: set_state_fn,
                    });
                }
            }
        }
        Err(())
    }

    pub fn get_states(&self) -> Vec<(DWORD, u16, i32, i32)> {
        let mut resvec = Vec::new();
        for idx in 0..4 {
            let mut state: XINPUT_STATE = unsafe { std::mem::zeroed() };
            let res = unsafe { (self.get_state)(idx, &mut state as *mut XINPUT_STATE) };
            if res == 0 {
                let gp = state.Gamepad;
                resvec.push((idx, gp.wButtons as u16, gp.sThumbLY as i32, gp.sThumbLX as i32));
            }
        }
        resvec
    }

    pub fn first_connected_index(&self) -> Option<DWORD> {
        self.get_states()
            .into_iter()
            .map(|(index, _, _, _)| index)
            .next()
    }

    pub fn set_state(&self, index: DWORD, left_motor: u16, right_motor: u16) -> Result<(), ()> {
        let Some(set_state) = self._set_state else {
            return Err(());
        };

        let mut vibration = XINPUT_VIBRATION {
            wLeftMotorSpeed: left_motor,
            wRightMotorSpeed: right_motor,
        };
        let res = unsafe { set_state(index, &mut vibration as *mut XINPUT_VIBRATION) };
        if res == 0 {
            Ok(())
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Default)]
pub enum InputToken {
    #[default]
    None,
    XButton(u16),
    VerticalAxis(VerticalAxisDirection),
}

impl InputToken {
    #[cfg(target_os = "windows")]
    fn detect_xinput(buttons: u16, ly: i32) -> Option<Self> {
        if buttons != 0 {
            Some(Self::XButton(buttons))
        } else {
            VerticalAxisDirection::from_thumb_ly(ly).map(Self::VerticalAxis)
        }
    }

    #[cfg(target_os = "windows")]
    fn is_active_xinput(&self, aggregate: &XInputAggregateState) -> bool {
        match self {
            Self::XButton(mask) => (aggregate.buttons & mask) != 0,
            Self::VerticalAxis(direction) => direction.is_active(aggregate),
            Self::None => false,
        }
    }
}

#[derive(Debug, Default)]
pub struct Mapping {
    pub map: HashMap<String, InputToken>,
}

#[cfg(target_os = "windows")]
fn xinput_rumble_settings() -> RumbleSettings {
    RumbleSettings {
        duration_ms: XINPUT_SELECTION_RUMBLE_DURATION_MS,
        left_strength: XINPUT_SELECTION_RUMBLE_LEFT_STRENGTH,
        right_strength: XINPUT_SELECTION_RUMBLE_RIGHT_STRENGTH,
    }
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

        let frame = input.poll_with_raw_held(raw_held(&[InputAction::Down]), true, false, now);

        assert!(matches!(frame.actions.as_slice(), [ControllerAction::Down]));
    }

    #[test]
    fn held_navigation_waits_for_initial_delay_before_repeating() {
        let mut input = InputController::new();
        let now = Instant::now();

        let first = input.poll_with_raw_held(raw_held(&[InputAction::Right]), true, false, now);
        let before_delay = input.poll_with_raw_held(
            raw_held(&[InputAction::Right]),
            true,
            false,
            now + Duration::from_millis((NAV_INITIAL_DELAY_MS - 1) as u64),
        );
        let after_delay = input.poll_with_raw_held(
            raw_held(&[InputAction::Right]),
            true,
            false,
            now + Duration::from_millis((NAV_INITIAL_DELAY_MS + 1) as u64),
        );
        let before_repeat = input.poll_with_raw_held(
            raw_held(&[InputAction::Right]),
            true,
            false,
            now + Duration::from_millis((NAV_INITIAL_DELAY_MS + NAV_REPEAT_INTERVAL_MS - 1) as u64),
        );
        let after_repeat = input.poll_with_raw_held(
            raw_held(&[InputAction::Right]),
            true,
            false,
            now + Duration::from_millis((NAV_INITIAL_DELAY_MS + NAV_REPEAT_INTERVAL_MS + 1) as u64),
        );

        assert!(matches!(first.actions.as_slice(), [ControllerAction::Right]));
        assert!(before_delay.actions.is_empty());
        assert!(matches!(after_delay.actions.as_slice(), [ControllerAction::Right]));
        assert!(before_repeat.actions.is_empty());
        assert!(matches!(after_repeat.actions.as_slice(), [ControllerAction::Right]));
    }

    #[test]
    fn refresh_and_sort_do_not_repeat_while_held() {
        let mut input = InputController::new();
        let now = Instant::now();
        let held_until = now + Duration::from_millis((NAV_REPEAT_ACCEL_STAGE1_AFTER_MS + 250) as u64);

        let first = input.poll_with_raw_held(raw_held(&[InputAction::Refresh, InputAction::Sort]), true, false, now);
        let held = input.poll_with_raw_held(
            raw_held(&[InputAction::Refresh, InputAction::Sort]),
            true,
            false,
            held_until,
        );

        assert!(matches!(first.actions.as_slice(), [ControllerAction::Refresh, ControllerAction::Sort]));
        assert!(held.actions.is_empty());
    }

    #[test]
    fn clear_held_resets_repeat_state() {
        let mut input = InputController::new();
        let now = Instant::now();

        let _ = input.poll_with_raw_held(raw_held(&[InputAction::Up]), true, false, now);
        input.clear_held();
        let frame = input.poll_with_raw_held(
            raw_held(&[InputAction::Up]),
            true,
            false,
            now + Duration::from_millis((NAV_INITIAL_DELAY_MS / 2) as u64),
        );

        assert!(matches!(frame.actions.as_slice(), [ControllerAction::Up]));
    }
}

