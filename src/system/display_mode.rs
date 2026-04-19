#[derive(Clone, Debug)]
pub struct ResolutionChoice {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct ResolutionOptions {
    pub current: ResolutionChoice,
    pub half_refresh: ResolutionChoice,
    pub max_refresh: ResolutionChoice,
}

impl ResolutionOptions {
    fn choice(width: u32, height: u32, refresh_hz: u32) -> ResolutionChoice {
        ResolutionChoice {
            width,
            height,
            refresh_hz,
            label: format!("{}×{} {}Hz", width, height, refresh_hz),
        }
    }

    fn new(
        current_width: u32,
        current_height: u32,
        current_refresh_hz: u32,
        max_width: u32,
        max_height: u32,
        max_refresh_hz: u32,
    ) -> Self {
        let max_refresh_hz = max_refresh_hz.max(1);
        let half_refresh_hz = ((max_refresh_hz as f32) / 2.0).round() as u32;
        let half_refresh_hz = half_refresh_hz.max(1);

        Self {
            current: Self::choice(current_width, current_height, current_refresh_hz.max(1)),
            half_refresh: Self::choice(max_width, max_height, half_refresh_hz),
            max_refresh: Self::choice(max_width, max_height, max_refresh_hz),
        }
    }
}

impl Default for ResolutionOptions {
    fn default() -> Self {
        Self::new(3840, 2160, 60, 3840, 2160, 120)
    }
}

#[cfg(target_os = "windows")]
pub fn detect_resolution_options() -> ResolutionOptions {
    use std::mem::{size_of, zeroed};
    use std::ptr::null;

    use winapi::um::wingdi::DEVMODEW;
    use winapi::um::winuser::{EnumDisplaySettingsW, ENUM_CURRENT_SETTINGS};

    let mut current_width = 0;
    let mut current_height = 0;
    let mut current_refresh_hz = 0;

    let mut current_mode: DEVMODEW = unsafe { zeroed() };
    current_mode.dmSize = size_of::<DEVMODEW>() as u16;
    let current_ok = unsafe { EnumDisplaySettingsW(null(), ENUM_CURRENT_SETTINGS, &mut current_mode) };
    if current_ok != 0 {
        current_width = current_mode.dmPelsWidth;
        current_height = current_mode.dmPelsHeight;
        current_refresh_hz = current_mode.dmDisplayFrequency;
    }

    let mut best_width = 0;
    let mut best_height = 0;
    let mut best_refresh_hz = 0;
    let mut best_area = 0_u64;
    let mut mode_index = 0;

    loop {
        let mut dev_mode: DEVMODEW = unsafe { zeroed() };
        dev_mode.dmSize = size_of::<DEVMODEW>() as u16;

        let ok = unsafe { EnumDisplaySettingsW(null(), mode_index, &mut dev_mode) };
        if ok == 0 {
            break;
        }

        mode_index += 1;

        let width = dev_mode.dmPelsWidth;
        let height = dev_mode.dmPelsHeight;
        let refresh_hz = dev_mode.dmDisplayFrequency;
        if width == 0 || height == 0 || refresh_hz == 0 {
            continue;
        }

        let area = width as u64 * height as u64;
        let found_larger_resolution = area > best_area
            || (area == best_area
                && (width > best_width || (width == best_width && height > best_height)));

        if found_larger_resolution {
            best_width = width;
            best_height = height;
            best_refresh_hz = refresh_hz;
            best_area = area;
            continue;
        }

        if width == best_width && height == best_height && refresh_hz > best_refresh_hz {
            best_refresh_hz = refresh_hz;
        }
    }

    if best_width > 0 && best_height > 0 && best_refresh_hz > 0 {
        let current_width = if current_width == 0 { best_width } else { current_width };
        let current_height = if current_height == 0 { best_height } else { current_height };
        let current_refresh_hz = if current_refresh_hz == 0 {
            best_refresh_hz
        } else {
            current_refresh_hz
        };

        ResolutionOptions::new(
            current_width,
            current_height,
            current_refresh_hz,
            best_width,
            best_height,
            best_refresh_hz,
        )
    } else {
        ResolutionOptions::default()
    }
}

#[cfg(not(target_os = "windows"))]
pub fn detect_resolution_options() -> ResolutionOptions {
    ResolutionOptions::default()
}

#[cfg(target_os = "windows")]
pub fn apply_resolution_choice(choice: &ResolutionChoice) -> bool {
    use std::mem::{size_of, zeroed};
    use std::ptr::{null, null_mut};

    use winapi::um::wingdi::{DEVMODEW, DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH};
    use winapi::um::winuser::{ChangeDisplaySettingsExW, DISP_CHANGE_SUCCESSFUL};

    let mut dev_mode: DEVMODEW = unsafe { zeroed() };
    dev_mode.dmSize = size_of::<DEVMODEW>() as u16;
    dev_mode.dmPelsWidth = choice.width;
    dev_mode.dmPelsHeight = choice.height;
    dev_mode.dmDisplayFrequency = choice.refresh_hz;
    dev_mode.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;

    let result = unsafe {
        ChangeDisplaySettingsExW(
            null(),
            &mut dev_mode,
            null_mut(),
            0,
            null_mut(),
        )
    };

    result == DISP_CHANGE_SUCCESSFUL
}

#[cfg(not(target_os = "windows"))]
pub fn apply_resolution_choice(_choice: &ResolutionChoice) -> bool {
    false
}