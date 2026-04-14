use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::time::Instant;

#[cfg(target_os = "windows")]
use libloading::Library;
#[cfg(target_os = "windows")]
use winapi::shared::minwindef::DWORD;
#[cfg(target_os = "windows")]
use winapi::um::xinput::{XINPUT_STATE, XINPUT_VIBRATION};

pub const NAV_INITIAL_DELAY_MS: u128 = 350;
pub const NAV_REPEAT_INTERVAL_MS: u128 = 120;
pub const FOCUS_COOLDOWN_MS: u128 = 500;

pub enum ControllerAction {
    Up,
    Down,
    Launch,
}

pub struct NavState {
    pub since: Instant,
    pub last_fire: Instant,
    pub past_initial: bool,
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

    pub fn get_states(&self) -> Vec<(u16, i32)> {
        let mut resvec = Vec::new();
        for idx in 0..4 {
            let mut state: XINPUT_STATE = unsafe { std::mem::zeroed() };
            let res = unsafe { (self.get_state)(idx, &mut state as *mut XINPUT_STATE) };
            if res == 0 {
                let gp = state.Gamepad;
                resvec.push((gp.wButtons as u16, gp.sThumbLY as i32));
            }
        }
        resvec
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub enum InputToken {
    #[default]
    None,
    XButton(u16),
    XAxis(i8),
    GilrsButton(u8),
    GilrsAxis(String, i8),
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Mapping {
    pub map: HashMap<String, InputToken>,
}

impl Mapping {
    pub fn load() -> Option<Self> {
        let path = "mapping.json";
        if let Ok(s) = fs::read_to_string(path) {
            serde_json::from_str(&s).ok()
        } else {
            None
        }
    }

    pub fn save(&self) {
        let _ = fs::write(
            "mapping.json",
            serde_json::to_string_pretty(self).unwrap_or_default(),
        );
    }
}
