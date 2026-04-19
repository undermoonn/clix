#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "windows")]
use std::sync::Once;

#[cfg(target_os = "windows")]
use eframe::egui;
#[cfg(target_os = "windows")]
use winapi::shared::minwindef::DWORD;
#[cfg(target_os = "windows")]
use winapi::um::libloaderapi::{GetProcAddress, LoadLibraryW};
#[cfg(target_os = "windows")]
use winapi::um::xinput::XINPUT_STATE;

#[cfg(target_os = "windows")]
const XINPUT_GAMEPAD_GUIDE: u16 = 0x0400;
#[cfg(target_os = "windows")]
static HOME_WAKE_PENDING: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static HOME_GUIDE_HELD: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
type XInputGetStateExFn = unsafe extern "system" fn(DWORD, *mut XINPUT_STATE) -> DWORD;

#[cfg(target_os = "windows")]
pub fn start(ctx: egui::Context) {
    static START_ONCE: Once = Once::new();

    START_ONCE.call_once(move || {
        std::thread::spawn(move || unsafe {
            run_xinput_watcher(ctx);
        });
    });
}

#[cfg(not(target_os = "windows"))]
pub fn start(_ctx: eframe::egui::Context) {}

#[cfg(target_os = "windows")]
pub fn take_wake_request() -> bool {
    HOME_WAKE_PENDING.swap(false, Ordering::AcqRel)
}

#[cfg(not(target_os = "windows"))]
pub fn take_wake_request() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn guide_held() -> bool {
    HOME_GUIDE_HELD.load(Ordering::Acquire)
}

#[cfg(not(target_os = "windows"))]
pub fn guide_held() -> bool {
    false
}

#[cfg(target_os = "windows")]
unsafe fn run_xinput_watcher(ctx: egui::Context) {
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
                let pressed = state.Gamepad.wButtons & XINPUT_GAMEPAD_GUIDE != 0;
                if pressed {
                    any_held = true;
                }
                if pressed && !prev_guide[i as usize] {
                    if app_is_background {
                        HOME_WAKE_PENDING.store(true, Ordering::Release);
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
