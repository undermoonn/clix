#[cfg(target_os = "windows")]
use std::time::Instant;
#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "windows")]
use std::sync::{Mutex, Once, OnceLock};

#[cfg(target_os = "windows")]
use crc32fast::Hasher;
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
const DUALSENSE_SELECTION_RUMBLE_DURATION_MS: u128 = 30;
#[cfg(target_os = "windows")]
const DUALSENSE_SELECTION_RUMBLE_WEAK: u8 = 1;
#[cfg(target_os = "windows")]
const DUALSENSE_SELECTION_RUMBLE_STRONG: u8 = 0;
#[cfg(target_os = "windows")]
const USB_OUTPUT_REPORT_SIZE: usize = 48;
#[cfg(target_os = "windows")]
const BLUETOOTH_OUTPUT_REPORT_SIZE: usize = 78;
#[cfg(target_os = "windows")]
const BLUETOOTH_OUTPUT_REPORT_TAG: u8 = 0x02;
#[cfg(target_os = "windows")]
const EFFECT_ENABLE_RUMBLE_EMULATION: u8 = 0x01;
#[cfg(target_os = "windows")]
const EFFECT_DISABLE_AUDIO_HAPTICS: u8 = 0x02;
#[cfg(target_os = "windows")]
const EFFECT_ENABLE_IMPROVED_RUMBLE: u8 = 0x04;
#[cfg(target_os = "windows")]
const USB_EFFECTS_OFFSET: usize = 1;
#[cfg(target_os = "windows")]
const BLUETOOTH_EFFECTS_OFFSET: usize = 3;
#[cfg(target_os = "windows")]
const EFFECT_ENABLE_BITS1_INDEX: usize = 0;
#[cfg(target_os = "windows")]
const EFFECT_RUMBLE_RIGHT_INDEX: usize = 2;
#[cfg(target_os = "windows")]
const EFFECT_RUMBLE_LEFT_INDEX: usize = 3;
#[cfg(target_os = "windows")]
const EFFECT_ENABLE_BITS3_INDEX: usize = 38;

#[cfg(target_os = "windows")]
static WAKE_PENDING: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static SNAPSHOT: OnceLock<Mutex<DualSenseSnapshot>> = OnceLock::new();
#[cfg(target_os = "windows")]
static RUMBLE: OnceLock<Mutex<Option<DualSenseRumble>>> = OnceLock::new();

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
    let settings = selection_rumble_settings();
    let mut rumble = rumble_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if start_rumble_with_state(&mut rumble, settings) {
        return true;
    }

    *rumble = None;
    start_rumble_with_state(&mut rumble, settings)
}

#[cfg(not(target_os = "windows"))]
pub(super) fn start_selection_rumble() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub(super) fn tick_rumble() {
    let should_stop = rumble_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
        .and_then(|rumble| rumble.active_until)
        .map(|active_until| Instant::now() >= active_until)
        .unwrap_or(false);

    if should_stop {
        stop_rumble();
    }
}

#[cfg(not(target_os = "windows"))]
pub(super) fn tick_rumble() {}

#[cfg(target_os = "windows")]
pub(super) fn stop_rumble() {
    let mut rumble = rumble_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(state) = rumble.as_mut() else {
        return;
    };

    if state.connection.write_rumble(0, 0).is_err() {
        *rumble = None;
        return;
    }

    state.active_until = None;
}

#[cfg(not(target_os = "windows"))]
pub(super) fn stop_rumble() {}

#[cfg(target_os = "windows")]
fn snapshot_state() -> &'static Mutex<DualSenseSnapshot> {
    SNAPSHOT.get_or_init(|| Mutex::new(DualSenseSnapshot::default()))
}

#[cfg(target_os = "windows")]
fn rumble_state() -> &'static Mutex<Option<DualSenseRumble>> {
    RUMBLE.get_or_init(|| Mutex::new(None))
}

#[cfg(target_os = "windows")]
fn start_rumble_with_state(
    rumble: &mut Option<DualSenseRumble>,
    settings: RumbleSettings,
) -> bool {
    let state = match rumble.as_mut() {
        Some(state) => state,
        None => {
            *rumble = DualSenseRumble::open();
            let Some(state) = rumble.as_mut() else {
                return false;
            };
            state
        }
    };

    if state
        .connection
        .write_rumble(settings.weak_motor, settings.strong_motor)
        .is_err()
    {
        return false;
    }

    state.active_until = Some(
        Instant::now() + std::time::Duration::from_millis(settings.duration_ms as u64),
    );
    true
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
                    if app_is_background && super::background_home_wake_enabled() {
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
    transport: DualSenseTransport,
    bluetooth_sequence: u8,
}

#[cfg(target_os = "windows")]
struct DualSenseRumble {
    connection: ConnectedDualSense,
    active_until: Option<Instant>,
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum DualSenseTransport {
    #[default]
    Usb,
    Bluetooth,
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
struct RumbleSettings {
    duration_ms: u128,
    weak_motor: u8,
    strong_motor: u8,
}

#[cfg(target_os = "windows")]
impl ConnectedDualSense {
    fn open() -> Option<Self> {
        let api = HidApi::new().ok()?;
        let device = api
            .open(SONY_VENDOR_ID, DUALSENSE_PRODUCT_ID)
            .or_else(|_| api.open(SONY_VENDOR_ID, DUALSENSE_EDGE_PRODUCT_ID))
            .ok()?;

        Some(Self {
            device,
            transport: DualSenseTransport::Usb,
            bluetooth_sequence: 0,
        })
    }

    fn read_snapshot(&mut self) -> Result<Option<DualSenseSnapshot>, hidapi::HidError> {
        let mut report = [0u8; REPORT_BUFFER_SIZE];
        let bytes_read = self.device.read_timeout(&mut report, 16)?;
        if bytes_read == 0 {
            return try_get_input_report(&self.device);
        }

        Ok(self.parse_report(&report[..bytes_read]))
    }

    fn parse_report(&mut self, report: &[u8]) -> Option<DualSenseSnapshot> {
        let (snapshot, transport) = parse_input_report(report)?;
        self.transport = transport;
        Some(snapshot)
    }

    fn write_rumble(&mut self, weak_motor: u8, strong_motor: u8) -> Result<(), hidapi::HidError> {
        let bytes = match self.transport {
            DualSenseTransport::Usb => build_usb_output_report(weak_motor, strong_motor).to_vec(),
            DualSenseTransport::Bluetooth => {
                self.bluetooth_sequence = self.bluetooth_sequence.wrapping_add(1);
                build_bluetooth_output_report(
                    weak_motor,
                    strong_motor,
                    self.bluetooth_sequence,
                )
                .to_vec()
            }
        };

        let _ = self.device.write(&bytes)?;
        Ok(())
    }
}

#[cfg(target_os = "windows")]
impl DualSenseRumble {
    fn open() -> Option<Self> {
        Some(Self {
            connection: ConnectedDualSense::open()?,
            active_until: None,
        })
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

        if let Some((snapshot, _)) = parse_input_report(&report[..bytes_read]) {
            return Ok(Some(snapshot));
        }
    }

    Ok(None)
}

#[cfg(target_os = "windows")]
fn parse_input_report(report: &[u8]) -> Option<(DualSenseSnapshot, DualSenseTransport)> {
    let report_id = *report.first()?;

    match report_id {
        0x01 if report.len() >= 64 => parse_full_state(&report[1..64]).map(|snapshot| {
            (snapshot, DualSenseTransport::Usb)
        }),
        0x31 if report.len() >= 65 => parse_full_state(&report[2..65]).map(|snapshot| {
            (snapshot, DualSenseTransport::Bluetooth)
        }),
        0x01 if report.len() >= 10 => parse_simple_bluetooth_state(&report[1..10]).map(
            |snapshot| (snapshot, DualSenseTransport::Bluetooth),
        ),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn selection_rumble_settings() -> RumbleSettings {
    RumbleSettings {
        duration_ms: DUALSENSE_SELECTION_RUMBLE_DURATION_MS,
        weak_motor: DUALSENSE_SELECTION_RUMBLE_WEAK,
        strong_motor: DUALSENSE_SELECTION_RUMBLE_STRONG,
    }
}

#[cfg(target_os = "windows")]
fn build_usb_output_report(weak_motor: u8, strong_motor: u8) -> [u8; USB_OUTPUT_REPORT_SIZE] {
    let mut report = [0u8; USB_OUTPUT_REPORT_SIZE];
    report[0] = 0x02;
    populate_effect_state(&mut report[USB_EFFECTS_OFFSET..], weak_motor, strong_motor);
    report
}

#[cfg(target_os = "windows")]
fn build_bluetooth_output_report(
    weak_motor: u8,
    strong_motor: u8,
    sequence: u8,
) -> [u8; BLUETOOTH_OUTPUT_REPORT_SIZE] {
    let mut report = [0u8; BLUETOOTH_OUTPUT_REPORT_SIZE];
    report[0] = 0x31;
    report[1] = BLUETOOTH_OUTPUT_REPORT_TAG;
    report[2] = sequence << 4;
    populate_effect_state(&mut report[BLUETOOTH_EFFECTS_OFFSET..], weak_motor, strong_motor);

    let crc = dualsense_bluetooth_crc(&report[..BLUETOOTH_OUTPUT_REPORT_SIZE - 4]);
    report[BLUETOOTH_OUTPUT_REPORT_SIZE - 4..].copy_from_slice(&crc.to_le_bytes());
    report
}

#[cfg(target_os = "windows")]
fn populate_effect_state(report: &mut [u8], weak_motor: u8, strong_motor: u8) {
    report[EFFECT_ENABLE_BITS1_INDEX] =
        EFFECT_ENABLE_RUMBLE_EMULATION | EFFECT_DISABLE_AUDIO_HAPTICS;
    report[EFFECT_RUMBLE_RIGHT_INDEX] = strong_motor;
    report[EFFECT_RUMBLE_LEFT_INDEX] = weak_motor;
    report[EFFECT_ENABLE_BITS3_INDEX] = EFFECT_ENABLE_IMPROVED_RUMBLE;
}

#[cfg(target_os = "windows")]
fn dualsense_bluetooth_crc(report: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(&[0xA2]);
    hasher.update(report);
    hasher.finalize()
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
    use super::{
        build_bluetooth_output_report, build_usb_output_report, dualsense_bluetooth_crc,
        parse_full_state, parse_simple_bluetooth_state,
    };
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

    #[test]
    fn builds_usb_rumble_report() {
        let report = build_usb_output_report(1, 0);

        assert_eq!(report[0], 0x02);
        assert_eq!(report[1], 0x03);
        assert_eq!(report[3], 0);
        assert_eq!(report[4], 1);
        assert_eq!(report[39], 0x04);
    }

    #[test]
    fn builds_bluetooth_rumble_report_with_crc() {
        let report = build_bluetooth_output_report(1, 0, 1);
        let expected_crc = dualsense_bluetooth_crc(&report[..report.len() - 4]);

        assert_eq!(report[0], 0x31);
        assert_eq!(report[1], 0x02);
        assert_eq!(report[2], 0x10);
        assert_eq!(report[3], 0x03);
        assert_eq!(report[5], 0);
        assert_eq!(report[6], 1);
        assert_eq!(report[41], 0x04);
        assert_eq!(
            u32::from_le_bytes(report[report.len() - 4..].try_into().unwrap()),
            expected_crc,
        );
    }
}