#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "windows")]
use std::sync::{Mutex, Once, OnceLock};

#[cfg(target_os = "windows")]
use eframe::egui;
#[cfg(target_os = "windows")]
use hidapi::{HidApi, HidDevice};
use super::{buttons::Buttons, InputAggregateState};

#[cfg(target_os = "windows")]
const SONY_VENDOR_ID: u16 = 0x054c;
#[cfg(target_os = "windows")]
const DUALSENSE_PRODUCT_ID: u16 = 0x0ce6;
#[cfg(target_os = "windows")]
const DUALSENSE_EDGE_PRODUCT_ID: u16 = 0x0df2;
#[cfg(target_os = "windows")]
const REPORT_BUFFER_SIZE: usize = 128;
#[cfg(target_os = "windows")]
const STICK_ACTIVITY_THRESHOLD: i32 = 16_000;

#[cfg(target_os = "windows")]
static WAKE_PENDING: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static SNAPSHOT: OnceLock<Mutex<DualSenseSnapshot>> = OnceLock::new();

#[cfg(target_os = "windows")]
#[derive(Clone, Copy, Default)]
pub struct DualSenseSnapshot {
    pub buttons: Buttons,
    pub left_stick_x: i32,
    pub left_stick_y: i32,
    pub has_input_activity: bool,
}

#[cfg(target_os = "windows")]
pub fn start(ctx: egui::Context) {
    static START_ONCE: Once = Once::new();

    START_ONCE.call_once(move || {
        std::thread::spawn(move || run_dualsense_watcher(ctx));
    });
}

#[cfg(not(target_os = "windows"))]
pub fn start(_ctx: eframe::egui::Context) {}

#[cfg(target_os = "windows")]
pub fn snapshot() -> DualSenseSnapshot {
    *snapshot_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(not(target_os = "windows"))]
pub fn snapshot() {}

#[cfg(target_os = "windows")]
pub fn take_wake_request() -> bool {
    WAKE_PENDING.swap(false, Ordering::AcqRel)
}

#[cfg(not(target_os = "windows"))]
pub fn take_wake_request() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub(super) fn home_held() -> bool {
    snapshot().buttons.intersects(Buttons::HOME)
}

#[cfg(not(target_os = "windows"))]
pub(super) fn home_held() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub(super) fn aggregate_state() -> Option<InputAggregateState> {
    let snapshot = snapshot();
    if !snapshot.has_input_activity {
        None
    } else {
        Some(InputAggregateState::from_state(
            snapshot.buttons,
            snapshot.left_stick_y,
            snapshot.left_stick_x,
        ))
    }
}

#[cfg(target_os = "windows")]
pub(super) fn remap_state() -> Option<(Buttons, i32)> {
    let snapshot = snapshot();
    if !snapshot.has_input_activity {
        None
    } else {
        Some((snapshot.buttons, snapshot.left_stick_y))
    }
}

#[cfg(target_os = "windows")]
pub(super) fn start_selection_rumble() -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub(super) fn start_selection_rumble() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub(super) fn tick_rumble() {}

#[cfg(not(target_os = "windows"))]
pub(super) fn tick_rumble() {}

#[cfg(target_os = "windows")]
pub(super) fn stop_rumble() {}

#[cfg(not(target_os = "windows"))]
pub(super) fn stop_rumble() {}

#[cfg(target_os = "windows")]
fn snapshot_state() -> &'static Mutex<DualSenseSnapshot> {
    SNAPSHOT.get_or_init(|| Mutex::new(DualSenseSnapshot::default()))
}

#[cfg(target_os = "windows")]
fn store_snapshot(snapshot: DualSenseSnapshot) {
    *snapshot_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = snapshot;
}

#[cfg(target_os = "windows")]
fn run_dualsense_watcher(ctx: egui::Context) {
    let mut connection: Option<ConnectedDualSense> = None;
    let mut previous = DualSenseSnapshot::default();

    loop {
        if connection.is_none() {
            connection = ConnectedDualSense::open();

            if connection.is_none() {
                if previous.has_input_activity {
                    previous = DualSenseSnapshot::default();
                    store_snapshot(previous);
                }

                std::thread::sleep(std::time::Duration::from_millis(500));
                continue;
            }
        }

        let result = connection.as_mut().unwrap().read_snapshot();

        match result {
            Ok(Some(snapshot)) => {
                let app_is_background = crate::launch::current_app_window_is_background();
                let pressed_now = snapshot.buttons.intersects(Buttons::HOME)
                    && !previous.buttons.intersects(Buttons::HOME);

                previous = snapshot;
                store_snapshot(snapshot);

                if pressed_now {
                    if app_is_background {
                        WAKE_PENDING.store(true, Ordering::Release);
                        let _ = crate::launch::focus_current_app_window();
                    }

                    ctx.request_repaint();
                }

                if snapshot.has_input_activity && !app_is_background {
                    ctx.request_repaint();
                }
            }
            Ok(None) => {}
            Err(_) => {
                previous = DualSenseSnapshot::default();
                store_snapshot(previous);
                connection = None;
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
        }
    }
}

#[cfg(target_os = "windows")]
struct ConnectedDualSense {
    device: HidDevice,
}

#[cfg(target_os = "windows")]
impl ConnectedDualSense {
    fn open() -> Option<Self> {
        let api = HidApi::new().ok()?;
        let device = api
            .open(SONY_VENDOR_ID, DUALSENSE_PRODUCT_ID)
            .or_else(|_| api.open(SONY_VENDOR_ID, DUALSENSE_EDGE_PRODUCT_ID))
            .ok()?;

        Some(Self { device })
    }

    fn read_snapshot(&mut self) -> Result<Option<DualSenseSnapshot>, hidapi::HidError> {
        let mut report = [0u8; REPORT_BUFFER_SIZE];
        let bytes_read = self.device.read_timeout(&mut report, 16)?;
        if bytes_read == 0 {
            return try_get_input_report(&self.device);
        }

        Ok(parse_input_report(&report[..bytes_read]))
    }
}

#[cfg(target_os = "windows")]
fn try_get_input_report(device: &HidDevice) -> Result<Option<DualSenseSnapshot>, hidapi::HidError> {
    for report_id in [0x01u8, 0x31u8] {
        let mut report = [0u8; REPORT_BUFFER_SIZE];
        report[0] = report_id;

        let bytes_read = device.get_input_report(&mut report)?;
        if bytes_read <= 1 {
            continue;
        }

        if let Some(snapshot) = parse_input_report(&report[..bytes_read]) {
            return Ok(Some(snapshot));
        }
    }

    Ok(None)
}

#[cfg(target_os = "windows")]
fn parse_input_report(report: &[u8]) -> Option<DualSenseSnapshot> {
    let report_id = *report.first()?;

    match report_id {
        0x01 if report.len() >= 64 => parse_full_state(&report[1..64]),
        0x31 if report.len() >= 65 => parse_full_state(&report[2..65]),
        0x01 if report.len() >= 10 => parse_simple_bluetooth_state(&report[1..10]),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn parse_full_state(data: &[u8]) -> Option<DualSenseSnapshot> {
    if data.len() < 10 {
        return None;
    }

    let face_bits = data[7];
    let misc_bits = data[8];
    let special_bits = data[9];

    let mut buttons = Buttons::EMPTY;
    apply_dpad_bits(face_bits & 0x0f, &mut buttons);

    if (face_bits & 0x10) != 0 {
        buttons |= Buttons::X;
    }
    if (face_bits & 0x20) != 0 {
        buttons |= Buttons::A;
    }
    if (face_bits & 0x40) != 0 {
        buttons |= Buttons::B;
    }
    if (face_bits & 0x80) != 0 {
        buttons |= Buttons::Y;
    }
    if (misc_bits & 0x01) != 0 {
        buttons |= Buttons::LEFT_SHOULDER;
    }
    if (misc_bits & 0x02) != 0 {
        buttons |= Buttons::RIGHT_SHOULDER;
    }
    if (misc_bits & 0x10) != 0 {
        buttons |= Buttons::BACK;
    }
    if (misc_bits & 0x20) != 0 {
        buttons |= Buttons::START;
    }
    if (misc_bits & 0x40) != 0 {
        buttons |= Buttons::LEFT_THUMB;
    }
    if (misc_bits & 0x80) != 0 {
        buttons |= Buttons::RIGHT_THUMB;
    }
    if (special_bits & 0x01) != 0 {
        buttons |= Buttons::HOME;
    }

    Some(build_snapshot(
        buttons,
        scale_stick_axis(data[0]),
        scale_inverted_stick_axis(data[1]),
    ))
}

#[cfg(target_os = "windows")]
fn parse_simple_bluetooth_state(data: &[u8]) -> Option<DualSenseSnapshot> {
    if data.len() < 9 {
        return None;
    }

    let face_bits = data[4];
    let misc_bits = data[5];
    let special_bits = data[6];

    let mut buttons = Buttons::EMPTY;
    apply_dpad_bits(face_bits & 0x0f, &mut buttons);

    if (face_bits & 0x10) != 0 {
        buttons |= Buttons::X;
    }
    if (face_bits & 0x20) != 0 {
        buttons |= Buttons::A;
    }
    if (face_bits & 0x40) != 0 {
        buttons |= Buttons::B;
    }
    if (face_bits & 0x80) != 0 {
        buttons |= Buttons::Y;
    }
    if (misc_bits & 0x01) != 0 {
        buttons |= Buttons::LEFT_SHOULDER;
    }
    if (misc_bits & 0x02) != 0 {
        buttons |= Buttons::RIGHT_SHOULDER;
    }
    if (misc_bits & 0x10) != 0 {
        buttons |= Buttons::BACK;
    }
    if (misc_bits & 0x20) != 0 {
        buttons |= Buttons::START;
    }
    if (misc_bits & 0x40) != 0 {
        buttons |= Buttons::LEFT_THUMB;
    }
    if (misc_bits & 0x80) != 0 {
        buttons |= Buttons::RIGHT_THUMB;
    }
    if (special_bits & 0x02) != 0 {
        buttons |= Buttons::HOME;
    }

    Some(build_snapshot(
        buttons,
        scale_stick_axis(data[0]),
        scale_inverted_stick_axis(data[1]),
    ))
}

#[cfg(target_os = "windows")]
fn build_snapshot(buttons: Buttons, left_stick_x: i32, left_stick_y: i32) -> DualSenseSnapshot {
    let has_input_activity = !buttons.is_empty()
        || left_stick_x.abs() > STICK_ACTIVITY_THRESHOLD
        || left_stick_y.abs() > STICK_ACTIVITY_THRESHOLD;

    DualSenseSnapshot {
        buttons,
        left_stick_x,
        left_stick_y,
        has_input_activity,
    }
}

#[cfg(target_os = "windows")]
fn apply_dpad_bits(direction: u8, buttons: &mut Buttons) {
    match direction {
        0 => *buttons |= Buttons::DPAD_UP,
        1 => *buttons |= Buttons::DPAD_UP | Buttons::DPAD_RIGHT,
        2 => *buttons |= Buttons::DPAD_RIGHT,
        3 => *buttons |= Buttons::DPAD_RIGHT | Buttons::DPAD_DOWN,
        4 => *buttons |= Buttons::DPAD_DOWN,
        5 => *buttons |= Buttons::DPAD_DOWN | Buttons::DPAD_LEFT,
        6 => *buttons |= Buttons::DPAD_LEFT,
        7 => *buttons |= Buttons::DPAD_LEFT | Buttons::DPAD_UP,
        _ => {}
    }
}

#[cfg(target_os = "windows")]
fn scale_stick_axis(value: u8) -> i32 {
    ((((value as i32) - 128) * 32_767) / 127).clamp(-32_767, 32_767)
}

#[cfg(target_os = "windows")]
fn scale_inverted_stick_axis(value: u8) -> i32 {
    (((127 - value as i32) * 32_767) / 127).clamp(-32_767, 32_767)
}

#[cfg(test)]
mod tests {
    use super::{parse_full_state, parse_simple_bluetooth_state};
    use crate::input::buttons::Buttons;

    #[test]
    fn parses_full_dualsense_usb_layout() {
        let mut data = [0u8; 63];
        data[0] = 255;
        data[1] = 0;
        data[7] = 0x10 | 0x01;
        data[8] = 0x10 | 0x20;
        data[9] = 0x01;

        let snapshot = parse_full_state(&data).unwrap();

        assert!(snapshot.buttons.intersects(Buttons::X));
        assert!(snapshot.buttons.intersects(Buttons::DPAD_UP));
        assert!(snapshot.buttons.intersects(Buttons::DPAD_RIGHT));
        assert!(snapshot.buttons.intersects(Buttons::BACK));
        assert!(snapshot.buttons.intersects(Buttons::START));
        assert!(snapshot.buttons.intersects(Buttons::HOME));
        assert!(snapshot.left_stick_x > 0);
        assert!(snapshot.left_stick_y > 0);
    }

    #[test]
    fn parses_simple_bluetooth_layout() {
        let mut data = [0u8; 9];
        data[0] = 0;
        data[1] = 255;
        data[4] = 0x20 | 0x06;
        data[5] = 0x10;
        data[6] = 0x02;

        let snapshot = parse_simple_bluetooth_state(&data).unwrap();

        assert!(snapshot.buttons.intersects(Buttons::A));
        assert!(snapshot.buttons.intersects(Buttons::DPAD_LEFT));
        assert!(snapshot.buttons.intersects(Buttons::BACK));
        assert!(snapshot.buttons.intersects(Buttons::HOME));
        assert!(snapshot.left_stick_x < 0);
        assert!(snapshot.left_stick_y < 0);
    }
}