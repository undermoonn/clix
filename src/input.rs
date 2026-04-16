use std::collections::HashMap;
use std::time::Instant;

use gilrs::{Axis, Button, EventType, Gilrs};

#[cfg(target_os = "windows")]
use winapi::um::xinput::{
    XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_LEFT,
    XINPUT_GAMEPAD_DPAD_RIGHT, XINPUT_GAMEPAD_DPAD_UP, XINPUT_GAMEPAD_X,
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
const GILRS_AXIS_THRESHOLD: f32 = 0.45;
#[cfg(target_os = "windows")]
const XINPUT_SELECTION_RUMBLE_DURATION_MS: u128 = 40;
#[cfg(target_os = "windows")]
const XINPUT_SELECTION_RUMBLE_LEFT_STRENGTH: u16 = 0;
#[cfg(target_os = "windows")]
const XINPUT_SELECTION_RUMBLE_RIGHT_STRENGTH: u16 = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerBrand {
    Xbox,
    PlayStation,
}

impl ControllerBrand {
    pub fn as_ini_value(self) -> &'static str {
        match self {
            ControllerBrand::Xbox => "xbox",
            ControllerBrand::PlayStation => "playstation",
        }
    }

    pub fn from_ini_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "xbox" => Some(ControllerBrand::Xbox),
            "playstation" | "ps" | "ps5" | "dualsense" | "dualsence" => {
                Some(ControllerBrand::PlayStation)
            }
            _ => None,
        }
    }
}

pub enum ControllerAction {
    Up,
    Down,
    Left,
    Right,
    Launch,
    Quit,
}

impl ControllerAction {
    fn from_str(value: &str) -> Option<Self> {
        match value {
            "up" => Some(ControllerAction::Up),
            "down" => Some(ControllerAction::Down),
            "left" => Some(ControllerAction::Left),
            "right" => Some(ControllerAction::Right),
            "launch" => Some(ControllerAction::Launch),
            "quit" => Some(ControllerAction::Quit),
            _ => None,
        }
    }
}

pub struct NavState {
    pub since: Instant,
    pub last_fire: Instant,
    pub past_initial: bool,
}

pub struct InputFrame {
    pub actions: Vec<ControllerAction>,
    pub launch_held: bool,
    pub quit_held: bool,
    pub force_close_held: bool,
}

pub struct InputController {
    gilrs: Option<Gilrs>,
    #[cfg(target_os = "windows")]
    xinput: Option<XInput>,
    #[cfg(target_os = "windows")]
    rumble_state: Option<RumbleState>,
    mapping: Mapping,
    remap_target: Option<String>,
    nav_held: HashMap<&'static str, NavState>,
    controller_brand: ControllerBrand,
}

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

impl InputController {
    pub fn new(controller_brand: ControllerBrand) -> Self {
        Self {
            gilrs: Gilrs::new().ok(),
            #[cfg(target_os = "windows")]
            xinput: XInput::new().ok(),
            #[cfg(target_os = "windows")]
            rumble_state: None,
            mapping: Mapping::default(),
            remap_target: None,
            nav_held: HashMap::new(),
            controller_brand,
        }
    }

    pub fn controller_brand(&self) -> ControllerBrand {
        self.controller_brand
    }

    pub fn clear_held(&mut self) {
        self.nav_held.clear();
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
        let mut raw_held: std::collections::HashSet<&'static str> =
            std::collections::HashSet::new();
        let mut actions = Vec::new();

        self.collect_raw_held(&mut raw_held);

        if !process_input {
            self.nav_held.clear();
        }

        let now = Instant::now();
        if process_input {
            self.nav_held.retain(|key, _| raw_held.contains(key));
        }

        if process_input {
            let action_names: &[&str] = if include_quit_action {
                &["up", "down", "left", "right", "launch", "quit"]
            } else {
                &["up", "down", "left", "right", "launch"]
            };

            for action_name in action_names {
                if !raw_held.contains(action_name) {
                    continue;
                }

                let should_fire = if let Some(state) = self.nav_held.get_mut(action_name) {
                    if !state.past_initial {
                        if now.duration_since(state.since).as_millis() >= NAV_INITIAL_DELAY_MS {
                            state.past_initial = true;
                            state.last_fire = now;
                            true
                        } else {
                            false
                        }
                    } else {
                        let held_ms = now.duration_since(state.since).as_millis();
                        let repeat_interval_ms = nav_repeat_interval_ms(action_name, held_ms);

                        if now.duration_since(state.last_fire).as_millis() >= repeat_interval_ms {
                            state.last_fire = now;
                            true
                        } else {
                            false
                        }
                    }
                } else {
                    self.nav_held.insert(
                        action_name,
                        NavState {
                            since: now,
                            last_fire: now,
                            past_initial: false,
                        },
                    );
                    true
                };

                if should_fire {
                    if let Some(action) = ControllerAction::from_str(action_name) {
                        actions.push(action);
                    }
                }
            }
        }

        InputFrame {
            actions,
            launch_held: raw_held.contains("launch"),
            quit_held: raw_held.contains("quit"),
            force_close_held: raw_held.contains("force_close"),
        }
    }

    fn collect_raw_held(&mut self, raw_held: &mut std::collections::HashSet<&'static str>) {
        #[cfg(not(target_os = "windows"))]
        let xinput_active = false;

        #[cfg(target_os = "windows")]
        let xinput_active = self.collect_xinput(raw_held);

        if !xinput_active {
            self.collect_gilrs(raw_held);
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
        let Some(xinput) = &self.xinput else {
            return false;
        };

        let Some(controller_index) = xinput.first_connected_index() else {
            return false;
        };

        if xinput
            .set_state(
                controller_index,
                settings.left_strength,
                settings.right_strength,
            )
            .is_ok()
        {
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
    fn collect_xinput(&mut self, raw_held: &mut std::collections::HashSet<&'static str>) -> bool {
        let Some(xinput) = &self.xinput else {
            return false;
        };

        let states = xinput.get_states();
        if states.is_empty() {
            return false;
        }

        self.controller_brand = ControllerBrand::Xbox;

        if let Some(target) = self.remap_target.clone() {
            for (_, buttons, ly, _) in &states {
                if *buttons != 0 {
                    self.mapping.map.insert(target.clone(), InputToken::XButton(*buttons));
                    self.remap_target = None;
                    return true;
                }
                if *ly < -16000 {
                    self.mapping.map.insert(target.clone(), InputToken::XAxis(-1));
                    self.remap_target = None;
                    return true;
                }
                if *ly > 16000 {
                    self.mapping.map.insert(target.clone(), InputToken::XAxis(1));
                    self.remap_target = None;
                    return true;
                }
            }
            return true;
        }

        for (_, buttons, ly, lx) in &states {
            if (buttons & XINPUT_GAMEPAD_DPAD_UP) != 0 {
                raw_held.insert("up");
            }
            if (buttons & XINPUT_GAMEPAD_DPAD_DOWN) != 0 {
                raw_held.insert("down");
            }
            if (buttons & XINPUT_GAMEPAD_DPAD_LEFT) != 0 {
                raw_held.insert("left");
            }
            if (buttons & XINPUT_GAMEPAD_DPAD_RIGHT) != 0 {
                raw_held.insert("right");
            }
            if (buttons & XINPUT_GAMEPAD_A) != 0 {
                raw_held.insert("launch");
            }
            if (buttons & XINPUT_GAMEPAD_B) != 0 {
                raw_held.insert("quit");
            }
            if (buttons & XINPUT_GAMEPAD_X) != 0 {
                raw_held.insert("force_close");
            }
            if *ly > 16000 {
                raw_held.insert("up");
            } else if *ly < -16000 {
                raw_held.insert("down");
            }
            if *lx > 16000 {
                raw_held.insert("right");
            } else if *lx < -16000 {
                raw_held.insert("left");
            }
        }

        for (key, token) in &self.mapping.map {
            match token {
                InputToken::XButton(mask) => {
                    for (_, buttons, _, _) in &states {
                        if (buttons & mask) != 0 {
                            Self::insert_mapped_action(raw_held, key);
                        }
                    }
                }
                InputToken::XAxis(dir) => {
                    for (_, _, ly, _) in &states {
                        if *dir > 0 && *ly > 16000 {
                            Self::insert_mapped_vertical_action(raw_held, key);
                        }
                        if *dir < 0 && *ly < -16000 {
                            Self::insert_mapped_vertical_action(raw_held, key);
                        }
                    }
                }
                _ => {}
            }
        }

        true
    }

    fn collect_gilrs(&mut self, raw_held: &mut std::collections::HashSet<&'static str>) {
        let Some(gilrs) = &mut self.gilrs else {
            return;
        };

        while let Some(event) = gilrs.next_event() {
            if let Some(target) = self.remap_target.clone() {
                match event.event {
                    EventType::ButtonPressed(button, _) => {
                        self.mapping
                            .map
                            .insert(target, InputToken::GilrsButton(button as u8));
                        self.remap_target = None;
                    }
                    EventType::AxisChanged(axis, value, _) => {
                        let axis_name = format!("{:?}", axis);
                        let dir = if value < 0.0 { -1 } else { 1 };
                        self.mapping
                            .map
                            .insert(target, InputToken::GilrsAxis(axis_name, dir));
                        self.remap_target = None;
                    }
                    _ => {}
                }
                continue;
            }

            match event.event {
                EventType::ButtonPressed(button, _) => {
                    let button_id = button as u8;
                    for (key, token) in &self.mapping.map {
                        if let InputToken::GilrsButton(mapped_button) = token {
                            if *mapped_button == button_id {
                                Self::insert_mapped_action(raw_held, key);
                            }
                        }
                    }
                }
                EventType::AxisChanged(axis, value, _) => {
                    let axis_name = format!("{:?}", axis);
                    for (key, token) in &self.mapping.map {
                        if let InputToken::GilrsAxis(mapped_axis, dir) = token {
                            if mapped_axis == &axis_name {
                                if *dir < 0 && value < -0.7 {
                                    Self::insert_mapped_vertical_action(raw_held, key);
                                }
                                if *dir > 0 && value > 0.7 {
                                    Self::insert_mapped_vertical_action(raw_held, key);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let mut active_brand = None;
        let mut single_connected_brand = None;
        let mut connected_count = 0;

        for (_, gamepad) in gilrs.gamepads() {
            connected_count += 1;
            let brand = infer_controller_brand(&gamepad);
            if connected_count == 1 {
                single_connected_brand = Some(brand);
            }

            let mut gamepad_active = false;

            if gamepad.is_pressed(Button::DPadUp) {
                raw_held.insert("up");
                gamepad_active = true;
            }
            if gamepad.is_pressed(Button::DPadDown) {
                raw_held.insert("down");
                gamepad_active = true;
            }
            if gamepad.is_pressed(Button::DPadLeft) {
                raw_held.insert("left");
                gamepad_active = true;
            }
            if gamepad.is_pressed(Button::DPadRight) {
                raw_held.insert("right");
                gamepad_active = true;
            }
            if gamepad.is_pressed(Button::South) {
                raw_held.insert("launch");
                gamepad_active = true;
            }
            if gamepad.is_pressed(Button::East) {
                raw_held.insert("quit");
                gamepad_active = true;
            }
            if gamepad.is_pressed(Button::West) {
                raw_held.insert("force_close");
                gamepad_active = true;
            }

            let left_stick_x = gamepad.value(Axis::LeftStickX);
            if left_stick_x <= -GILRS_AXIS_THRESHOLD {
                raw_held.insert("left");
                gamepad_active = true;
            } else if left_stick_x >= GILRS_AXIS_THRESHOLD {
                raw_held.insert("right");
                gamepad_active = true;
            }

            let left_stick_y = gamepad.value(Axis::LeftStickY);
            if left_stick_y <= -GILRS_AXIS_THRESHOLD {
                raw_held.insert("down");
                gamepad_active = true;
            } else if left_stick_y >= GILRS_AXIS_THRESHOLD {
                raw_held.insert("up");
                gamepad_active = true;
            }

            if gamepad_active {
                active_brand = Some(brand);
            }
        }

        if let Some(brand) = active_brand.or(if connected_count == 1 {
            single_connected_brand
        } else {
            None
        }) {
            self.controller_brand = brand;
        }
    }

    fn insert_mapped_action(
        raw_held: &mut std::collections::HashSet<&'static str>,
        key: &str,
    ) {
        match key {
            "up" => {
                raw_held.insert("up");
            }
            "down" => {
                raw_held.insert("down");
            }
            "left" => {
                raw_held.insert("left");
            }
            "right" => {
                raw_held.insert("right");
            }
            "launch" => {
                raw_held.insert("launch");
            }
            "quit" => {
                raw_held.insert("quit");
            }
            "force_close" => {
                raw_held.insert("force_close");
            }
            _ => {}
        }
    }

    fn insert_mapped_vertical_action(
        raw_held: &mut std::collections::HashSet<&'static str>,
        key: &str,
    ) {
        if key == "up" {
            raw_held.insert("up");
        }
        if key == "down" {
            raw_held.insert("down");
        }
    }
}

fn nav_repeat_interval_ms(action_name: &str, held_ms: u128) -> u128 {
    match action_name {
        "up" | "down" | "left" | "right" => {
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
    XAxis(i8),
    GilrsButton(u8),
    GilrsAxis(String, i8),
}

#[derive(Debug, Default)]
pub struct Mapping {
    pub map: HashMap<String, InputToken>,
}

fn infer_controller_brand(gamepad: &gilrs::Gamepad<'_>) -> ControllerBrand {
    let name = gamepad.name().to_ascii_lowercase();
    let is_playstation = gamepad.vendor_id() == Some(0x054c)
        || name.contains("dualsense")
        || name.contains("dualsence")
        || name.contains("wireless controller")
        || name.contains("playstation")
        || name.contains("ps5");

    if is_playstation {
        ControllerBrand::PlayStation
    } else {
        ControllerBrand::Xbox
    }
}

#[cfg(target_os = "windows")]
fn xinput_rumble_settings() -> RumbleSettings {
    RumbleSettings {
        duration_ms: XINPUT_SELECTION_RUMBLE_DURATION_MS,
        left_strength: XINPUT_SELECTION_RUMBLE_LEFT_STRENGTH,
        right_strength: XINPUT_SELECTION_RUMBLE_RIGHT_STRENGTH,
    }
}

