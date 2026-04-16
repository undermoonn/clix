#[cfg(target_os = "windows")]
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use std::ffi::OsString;
#[cfg(target_os = "windows")]
use std::mem::{size_of, zeroed};
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStringExt;
#[cfg(target_os = "windows")]
use std::ptr::{null, null_mut};
#[cfg(target_os = "windows")]
use std::slice;
#[cfg(target_os = "windows")]
use std::sync::Once;

#[cfg(target_os = "windows")]
use eframe::egui;
#[cfg(target_os = "windows")]
use winapi::shared::minwindef::{DWORD, LPARAM, LRESULT, TRUE, UINT, WPARAM};
#[cfg(target_os = "windows")]
use winapi::shared::windef::HWND;
#[cfg(target_os = "windows")]
use winapi::um::libloaderapi::GetModuleHandleW;
#[cfg(target_os = "windows")]
use winapi::um::winnt::HANDLE;
#[cfg(target_os = "windows")]
use winapi::um::winuser::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetRawInputData,
    GetRawInputDeviceInfoW, GetWindowLongPtrW, HRAWINPUT, PostQuitMessage, RAWINPUT,
    RAWINPUTDEVICE, RAWINPUTHEADER, RAWHID, RID_INPUT, RIDI_DEVICENAME, RIDEV_DEVNOTIFY,
    RIDEV_INPUTSINK, RIM_TYPEHID, RegisterClassW, RegisterRawInputDevices,
    SetWindowLongPtrW, TranslateMessage, CREATESTRUCTW, GIDC_REMOVAL, GWLP_USERDATA,
    HWND_MESSAGE, MSG, WM_DESTROY, WM_INPUT, WM_INPUT_DEVICE_CHANGE, WM_NCCREATE,
    WNDCLASSW,
};

#[cfg(target_os = "windows")]
const HID_USAGE_PAGE_GENERIC: u16 = 0x01;
#[cfg(target_os = "windows")]
const HID_USAGE_GENERIC_JOYSTICK: u16 = 0x04;
#[cfg(target_os = "windows")]
const HID_USAGE_GENERIC_GAMEPAD: u16 = 0x05;
#[cfg(target_os = "windows")]
const XBOX_VENDOR_ID_TOKEN: &str = "VID_045E";
#[cfg(target_os = "windows")]
const XBOX_PRODUCT_ID_TOKEN: &str = "PID_0B12";
#[cfg(target_os = "windows")]
const GUIDE_REPORT_BIT: u8 = 0x04;

#[cfg(target_os = "windows")]
pub fn start(ctx: egui::Context) {
    static START_ONCE: Once = Once::new();

    START_ONCE.call_once(move || {
        std::thread::spawn(move || unsafe {
            run_raw_input_watcher(ctx);
        });
    });
}

#[cfg(not(target_os = "windows"))]
pub fn start(_ctx: eframe::egui::Context) {}

#[cfg(target_os = "windows")]
struct XboxHomeWatcher {
    ctx: egui::Context,
    supported_devices: HashMap<isize, bool>,
    guide_pressed: HashMap<isize, bool>,
}

#[cfg(target_os = "windows")]
unsafe fn run_raw_input_watcher(ctx: egui::Context) {
    let class_name = wide_null("clix-xbox-home-listener");
    let hinstance = GetModuleHandleW(null());

    let class = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(raw_input_wndproc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: null_mut(),
        hCursor: null_mut(),
        hbrBackground: null_mut(),
        lpszMenuName: null(),
        lpszClassName: class_name.as_ptr(),
    };

    if RegisterClassW(&class) == 0 {
        return;
    }

    let watcher = Box::new(XboxHomeWatcher {
        ctx,
        supported_devices: HashMap::new(),
        guide_pressed: HashMap::new(),
    });
    let watcher_ptr = Box::into_raw(watcher);

    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        class_name.as_ptr(),
        0,
        0,
        0,
        0,
        0,
        HWND_MESSAGE,
        null_mut(),
        hinstance,
        watcher_ptr.cast(),
    );

    if hwnd.is_null() {
        drop(Box::from_raw(watcher_ptr));
        return;
    }

    if !register_raw_input(hwnd) {
        return;
    }

    let mut msg: MSG = zeroed();
    while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn raw_input_wndproc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let create = &*(lparam as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
            TRUE as LRESULT
        }
        WM_INPUT => {
            if let Some(watcher) = watcher_from_hwnd(hwnd) {
                process_raw_input(watcher, lparam as HRAWINPUT);
            }
            0
        }
        WM_INPUT_DEVICE_CHANGE => {
            if wparam as DWORD == GIDC_REMOVAL {
                if let Some(watcher) = watcher_from_hwnd(hwnd) {
                    let device_key = lparam as isize;
                    watcher.supported_devices.remove(&device_key);
                    watcher.guide_pressed.remove(&device_key);
                }
            }
            0
        }
        WM_DESTROY => {
            let watcher_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut XboxHomeWatcher;
            if !watcher_ptr.is_null() {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(Box::from_raw(watcher_ptr));
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(target_os = "windows")]
unsafe fn watcher_from_hwnd(hwnd: HWND) -> Option<&'static mut XboxHomeWatcher> {
    let watcher_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut XboxHomeWatcher;
    watcher_ptr.as_mut()
}

#[cfg(target_os = "windows")]
unsafe fn process_raw_input(watcher: &mut XboxHomeWatcher, hrawinput: HRAWINPUT) {
    let mut size: UINT = 0;
    if GetRawInputData(
        hrawinput,
        RID_INPUT,
        null_mut(),
        &mut size,
        size_of::<RAWINPUTHEADER>() as UINT,
    ) == u32::MAX
        || size == 0
    {
        return;
    }

    let mut buffer = vec![0_u8; size as usize];
    if GetRawInputData(
        hrawinput,
        RID_INPUT,
        buffer.as_mut_ptr().cast(),
        &mut size,
        size_of::<RAWINPUTHEADER>() as UINT,
    ) == u32::MAX
    {
        return;
    }

    let raw = &*(buffer.as_ptr() as *const RAWINPUT);
    if raw.header.dwType != RIM_TYPEHID {
        return;
    }

    let device = raw.header.hDevice;
    let device_key = device as isize;
    if !device_is_supported(watcher, device, device_key) {
        return;
    }

    let hid = &*((&raw.data as *const _) as *const RAWHID);
    let report_size = hid.dwSizeHid as usize;
    if report_size == 0 {
        return;
    }

    let report_count = hid.dwCount as usize;
    let report_data = hid.bRawData.as_ptr();
    for index in 0..report_count {
        let report = slice::from_raw_parts(report_data.add(index * report_size), report_size);
        let pressed = guide_pressed(report);
        let was_pressed = watcher.guide_pressed.insert(device_key, pressed).unwrap_or(false);
        if pressed && !was_pressed {
            let _ = crate::launch::focus_current_app_window();
            watcher.ctx.request_repaint();
        }
    }
}

#[cfg(target_os = "windows")]
unsafe fn device_is_supported(
    watcher: &mut XboxHomeWatcher,
    device: HANDLE,
    device_key: isize,
) -> bool {
    if let Some(supported) = watcher.supported_devices.get(&device_key) {
        return *supported;
    }

    let device_name = raw_device_name(device);
    let supported = device_name
        .as_ref()
        .map(|name| name.contains(XBOX_VENDOR_ID_TOKEN) && name.contains(XBOX_PRODUCT_ID_TOKEN))
        .unwrap_or(false);
    watcher.supported_devices.insert(device_key, supported);
    supported
}

#[cfg(target_os = "windows")]
unsafe fn raw_device_name(device: HANDLE) -> Option<String> {
    let mut size: UINT = 0;
    if GetRawInputDeviceInfoW(device, RIDI_DEVICENAME, null_mut(), &mut size) == u32::MAX
        || size == 0
    {
        return None;
    }

    let mut buffer = vec![0_u16; size as usize];
    if GetRawInputDeviceInfoW(device, RIDI_DEVICENAME, buffer.as_mut_ptr().cast(), &mut size)
        == u32::MAX
        || size == 0
    {
        return None;
    }

    let end = buffer.iter().position(|ch| *ch == 0).unwrap_or(size as usize);
    Some(
        OsString::from_wide(&buffer[..end])
            .to_string_lossy()
            .to_ascii_uppercase(),
    )
}

#[cfg(target_os = "windows")]
fn register_raw_input(hwnd: HWND) -> bool {
    let devices = [
        RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: HID_USAGE_GENERIC_JOYSTICK,
            dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
            hwndTarget: hwnd,
        },
        RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: HID_USAGE_GENERIC_GAMEPAD,
            dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
            hwndTarget: hwnd,
        },
    ];

    unsafe {
        RegisterRawInputDevices(
            devices.as_ptr(),
            devices.len() as UINT,
            size_of::<RAWINPUTDEVICE>() as UINT,
        ) != 0
    }
}

#[cfg(target_os = "windows")]
fn guide_pressed(report: &[u8]) -> bool {
    guide_bit_set(report)
}

#[cfg(target_os = "windows")]
fn guide_bit_set(report: &[u8]) -> bool {
    match report.len() {
        15 => report.get(11).copied().unwrap_or(0) & GUIDE_REPORT_BIT != 0,
        16 => report.get(12).copied().unwrap_or(0) & GUIDE_REPORT_BIT != 0,
        _ => false,
    }
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
