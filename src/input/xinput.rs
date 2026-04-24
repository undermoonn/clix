use std::time::Instant;

#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "windows")]
use std::sync::Once;

#[cfg(target_os = "windows")]
use eframe::egui;
#[cfg(target_os = "windows")]
use libloading::Library;
#[cfg(target_os = "windows")]
use winapi::shared::minwindef::DWORD;
#[cfg(target_os = "windows")]
use winapi::um::libloaderapi::{GetProcAddress, LoadLibraryW};
#[cfg(target_os = "windows")]
use winapi::um::xinput::{
    XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_BACK, XINPUT_GAMEPAD_DPAD_DOWN,
    XINPUT_GAMEPAD_DPAD_LEFT, XINPUT_GAMEPAD_DPAD_RIGHT, XINPUT_GAMEPAD_DPAD_UP,
    XINPUT_GAMEPAD_LEFT_SHOULDER, XINPUT_GAMEPAD_LEFT_THUMB, XINPUT_GAMEPAD_RIGHT_SHOULDER,
    XINPUT_GAMEPAD_RIGHT_THUMB, XINPUT_GAMEPAD_START, XINPUT_GAMEPAD_X, XINPUT_GAMEPAD_Y,
    XINPUT_STATE, XINPUT_VIBRATION,
};

use super::{buttons::Buttons, InputAggregateState};

#[cfg(target_os = "windows")]
const XINPUT_SELECTION_RUMBLE_DURATION_MS: u128 = 40;
#[cfg(target_os = "windows")]
const XINPUT_SELECTION_RUMBLE_LEFT_STRENGTH: u16 = 0;
#[cfg(target_os = "windows")]
const XINPUT_SELECTION_RUMBLE_RIGHT_STRENGTH: u16 = 10_000;

#[cfg(target_os = "windows")]
static HOME_WAKE_PENDING: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static HOME_GUIDE_HELD: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
type XInputGetStateExFn = unsafe extern "system" fn(DWORD, *mut XINPUT_STATE) -> DWORD;

#[cfg(target_os = "windows")]
pub(super) fn start(ctx: egui::Context) {
    start_repaint_watcher(ctx.clone());
    start_home_watcher(ctx);
}

#[cfg(not(target_os = "windows"))]
pub(super) fn start(_ctx: eframe::egui::Context) {}

#[cfg(target_os = "windows")]
pub fn take_wake_request() -> bool {
    HOME_WAKE_PENDING.swap(false, Ordering::AcqRel)
}

#[cfg(not(target_os = "windows"))]
pub fn take_wake_request() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub(super) fn home_held() -> bool {
    HOME_GUIDE_HELD.load(Ordering::Acquire)
}

#[cfg(not(target_os = "windows"))]
pub(super) fn home_held() -> bool {
    false
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

#[cfg(target_os = "windows")]
fn start_repaint_watcher(ctx: egui::Context) {
    static START_ONCE: Once = Once::new();

    START_ONCE.call_once(move || {
        std::thread::spawn(move || run_xinput_repaint_watcher(ctx));
    });
}

#[cfg(not(target_os = "windows"))]
fn start_repaint_watcher(_ctx: eframe::egui::Context) {}

#[cfg(target_os = "windows")]
fn start_home_watcher(ctx: egui::Context) {
    static START_ONCE: Once = Once::new();

    START_ONCE.call_once(move || {
        std::thread::spawn(move || unsafe {
            run_home_watcher(ctx);
        });
    });
}

#[cfg(not(target_os = "windows"))]
fn start_home_watcher(_ctx: eframe::egui::Context) {}

#[cfg(target_os = "windows")]
fn run_xinput_repaint_watcher(ctx: egui::Context) {
    let Ok(mut xinput) = XInput::new() else {
        return;
    };

    loop {
        let aggregate = InputAggregateState::from_states(&xinput.get_states());
        if aggregate.has_repaint_activity() && !crate::launch::current_app_window_is_background() {
            ctx.request_repaint();
        }

        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

#[cfg(target_os = "windows")]
unsafe fn run_home_watcher(ctx: egui::Context) {
    let dll_name = wide_null("xinput1_4.dll");
    let lib = LoadLibraryW(dll_name.as_ptr());
    if lib.is_null() {
        return;
    }

    let func = GetProcAddress(lib, 100usize as *const i8);
    if func.is_null() {
        return;
    }

    let get_state_ex: XInputGetStateExFn = std::mem::transmute(func);
    let mut prev_guide = [false; 4];

    loop {
        let mut any_held = false;
        let app_is_background = crate::launch::current_app_window_is_background();

        for i in 0..4u32 {
            let mut state: XINPUT_STATE = std::mem::zeroed();
            if get_state_ex(i, &mut state) == 0 {
                let pressed = normalize_buttons(state.Gamepad.wButtons as u16).intersects(Buttons::HOME);
                if pressed {
                    any_held = true;
                }
                if pressed && !prev_guide[i as usize] {
                    if app_is_background && super::background_home_wake_enabled() {
                        HOME_WAKE_PENDING.store(true, Ordering::Release);
                        let _ = crate::launch::focus_current_app_window();
                    }

                    ctx.request_repaint();
                }
                prev_guide[i as usize] = pressed;
            } else {
                prev_guide[i as usize] = false;
            }
        }

        HOME_GUIDE_HELD.store(any_held, Ordering::Release);
        if any_held && !app_is_background {
            ctx.request_repaint();
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
pub(super) struct XInput {
    _lib: Library,
    get_state: unsafe extern "system" fn(DWORD, *mut XINPUT_STATE) -> DWORD,
    _set_state: Option<unsafe extern "system" fn(DWORD, *mut XINPUT_VIBRATION) -> DWORD>,
    rumble_state: Option<RumbleState>,
    last_connected_controller_index: Option<DWORD>,
}

#[cfg(target_os = "windows")]
impl XInput {
    pub(super) fn new() -> Result<Self, ()> {
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
                    return Ok(Self {
                        _lib: lib,
                        get_state: get_state_fn,
                        _set_state: set_state_fn,
                        rumble_state: None,
                        last_connected_controller_index: None,
                    });
                }
            }
        }
        Err(())
    }

    pub(super) fn get_states(&mut self) -> Vec<(DWORD, Buttons, i32, i32)> {
        let mut states = Vec::new();
        for idx in 0..4 {
            let mut state: XINPUT_STATE = unsafe { std::mem::zeroed() };
            let result = unsafe { (self.get_state)(idx, &mut state as *mut XINPUT_STATE) };
            if result == 0 {
                let gamepad = state.Gamepad;
                states.push((
                    idx,
                    normalize_buttons(gamepad.wButtons as u16),
                    gamepad.sThumbLY as i32,
                    gamepad.sThumbLX as i32,
                ));
            }
        }

        self.last_connected_controller_index = states.first().map(|(index, _, _, _)| *index);
        states
    }

    pub(super) fn start_selection_rumble(&mut self) -> bool {
        self.stop_rumble();

        let settings = xinput_rumble_settings();
        if let Some(controller_index) = self.last_connected_controller_index {
            if self.start_rumble(controller_index, settings) {
                return true;
            }
        }

        let Some(controller_index) = self.first_connected_index() else {
            return false;
        };

        self.start_rumble(controller_index, settings)
    }

    pub(super) fn tick_rumble(&mut self) {
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

    pub(super) fn stop_rumble(&mut self) {
        let Some(state) = self.rumble_state.take() else {
            return;
        };

        match state {
            RumbleState::XInput { controller_index, .. } => {
                let _ = self.set_state(controller_index, 0, 0);
            }
        }
    }

    fn first_connected_index(&mut self) -> Option<DWORD> {
        self.get_states().into_iter().map(|(index, _, _, _)| index).next()
    }

    fn start_rumble(&mut self, controller_index: DWORD, settings: RumbleSettings) -> bool {
        if self
            .set_state(controller_index, settings.left_strength, settings.right_strength)
            .is_err()
        {
            return false;
        }

        self.last_connected_controller_index = Some(controller_index);
        self.rumble_state = Some(RumbleState::XInput {
            controller_index,
            active_until: Instant::now()
                + std::time::Duration::from_millis(settings.duration_ms as u64),
        });
        true
    }

    fn set_state(&self, index: DWORD, left_motor: u16, right_motor: u16) -> Result<(), ()> {
        let Some(set_state) = self._set_state else {
            return Err(());
        };

        let mut vibration = XINPUT_VIBRATION {
            wLeftMotorSpeed: left_motor,
            wRightMotorSpeed: right_motor,
        };
        let result = unsafe { set_state(index, &mut vibration as *mut XINPUT_VIBRATION) };
        if result == 0 {
            Ok(())
        } else {
            Err(())
        }
    }
}

#[cfg(target_os = "windows")]
pub(super) fn aggregate_state(xinput: &mut XInput) -> Option<InputAggregateState> {
    let states = xinput.get_states();
    if states.is_empty() {
        None
    } else {
        Some(InputAggregateState::from_states(&states))
    }
}

#[cfg(target_os = "windows")]
pub(super) fn remap_state(xinput: &mut XInput) -> Option<(Buttons, i32)> {
    xinput.get_states().into_iter().find_map(|(_, buttons, ly, _)| {
        if !buttons.is_empty() || ly < -16000 || ly > 16000 {
            Some((buttons, ly))
        } else {
            None
        }
    })
}

#[cfg(target_os = "windows")]
fn xinput_rumble_settings() -> RumbleSettings {
    RumbleSettings {
        duration_ms: XINPUT_SELECTION_RUMBLE_DURATION_MS,
        left_strength: XINPUT_SELECTION_RUMBLE_LEFT_STRENGTH,
        right_strength: XINPUT_SELECTION_RUMBLE_RIGHT_STRENGTH,
    }
}

#[cfg(target_os = "windows")]
fn normalize_buttons(buttons: u16) -> Buttons {
    let mut normalized = Buttons::EMPTY;

    if (buttons & XINPUT_GAMEPAD_DPAD_UP) != 0 {
        normalized |= Buttons::DPAD_UP;
    }
    if (buttons & XINPUT_GAMEPAD_DPAD_DOWN) != 0 {
        normalized |= Buttons::DPAD_DOWN;
    }
    if (buttons & XINPUT_GAMEPAD_DPAD_LEFT) != 0 {
        normalized |= Buttons::DPAD_LEFT;
    }
    if (buttons & XINPUT_GAMEPAD_DPAD_RIGHT) != 0 {
        normalized |= Buttons::DPAD_RIGHT;
    }
    if (buttons & XINPUT_GAMEPAD_START) != 0 {
        normalized |= Buttons::START;
    }
    if (buttons & XINPUT_GAMEPAD_BACK) != 0 {
        normalized |= Buttons::BACK;
    }
    if (buttons & XINPUT_GAMEPAD_LEFT_THUMB) != 0 {
        normalized |= Buttons::LEFT_THUMB;
    }
    if (buttons & XINPUT_GAMEPAD_RIGHT_THUMB) != 0 {
        normalized |= Buttons::RIGHT_THUMB;
    }
    if (buttons & XINPUT_GAMEPAD_LEFT_SHOULDER) != 0 {
        normalized |= Buttons::LEFT_SHOULDER;
    }
    if (buttons & XINPUT_GAMEPAD_RIGHT_SHOULDER) != 0 {
        normalized |= Buttons::RIGHT_SHOULDER;
    }
    if (buttons & 0x0400) != 0 {
        normalized |= Buttons::HOME;
    }
    if (buttons & XINPUT_GAMEPAD_A) != 0 {
        normalized |= Buttons::A;
    }
    if (buttons & XINPUT_GAMEPAD_B) != 0 {
        normalized |= Buttons::B;
    }
    if (buttons & XINPUT_GAMEPAD_X) != 0 {
        normalized |= Buttons::X;
    }
    if (buttons & XINPUT_GAMEPAD_Y) != 0 {
        normalized |= Buttons::Y;
    }

    normalized
}