use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolutionChoice {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolutionEntry {
    pub width: u32,
    pub height: u32,
    pub label: String,
    pub refresh_rates: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct ResolutionOptions {
    pub current: ResolutionChoice,
    pub resolutions: Vec<ResolutionEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayScaleChoice {
    pub scale_percent: u32,
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct DisplayScaleOptions {
    pub current: DisplayScaleChoice,
    pub recommended_scale_percent: u32,
    pub scales: Vec<DisplayScaleChoice>,
}

impl ResolutionChoice {
    fn new(width: u32, height: u32, refresh_hz: u32) -> Self {
        Self {
            width,
            height,
            refresh_hz,
            label: format!("{}×{} {}Hz", width, height, refresh_hz),
        }
    }
}

impl ResolutionEntry {
    fn new(width: u32, height: u32, mut refresh_rates: Vec<u32>) -> Self {
        refresh_rates.retain(|refresh_hz| *refresh_hz > 0);
        refresh_rates.sort_unstable_by(|left, right| right.cmp(left));
        refresh_rates.dedup();

        Self {
            width,
            height,
            label: format!("{}×{}", width, height),
            refresh_rates,
        }
    }

    fn choice(&self, refresh_index: usize) -> Option<ResolutionChoice> {
        self.refresh_rates
            .get(refresh_index)
            .copied()
            .map(|refresh_hz| ResolutionChoice::new(self.width, self.height, refresh_hz))
    }
}

impl ResolutionOptions {
    fn fallback() -> Self {
        let current = ResolutionChoice::new(3840, 2160, 60);
        let resolutions = vec![ResolutionEntry::new(3840, 2160, vec![120, 60])];

        Self {
            current,
            resolutions,
        }
    }

    fn from_modes(
        current_width: u32,
        current_height: u32,
        current_refresh_hz: u32,
        modes: impl IntoIterator<Item = (u32, u32, u32)>,
    ) -> Self {
        let mut grouped_modes: BTreeMap<(u32, u32), Vec<u32>> = BTreeMap::new();

        for (width, height, refresh_hz) in modes {
            if width == 0 || height == 0 || refresh_hz == 0 {
                continue;
            }

            grouped_modes
                .entry((width, height))
                .or_default()
                .push(refresh_hz);
        }

        if current_width > 0 && current_height > 0 && current_refresh_hz > 0 {
            grouped_modes
                .entry((current_width, current_height))
                .or_default()
                .push(current_refresh_hz);
        }

        if grouped_modes.is_empty() {
            return Self::fallback();
        }

        let mut resolutions: Vec<_> = grouped_modes
            .into_iter()
            .map(|((width, height), refresh_rates)| ResolutionEntry::new(width, height, refresh_rates))
            .filter(|entry| !entry.refresh_rates.is_empty())
            .collect();

        resolutions.sort_unstable_by(|left, right| {
            let left_area = left.width as u64 * left.height as u64;
            let right_area = right.width as u64 * right.height as u64;
            right_area
                .cmp(&left_area)
                .then_with(|| right.width.cmp(&left.width))
                .then_with(|| right.height.cmp(&left.height))
        });

        if resolutions.is_empty() {
            return Self::fallback();
        }

        let current = if current_width > 0 && current_height > 0 && current_refresh_hz > 0 {
            ResolutionChoice::new(current_width, current_height, current_refresh_hz)
        } else {
            resolutions
                .first()
                .and_then(|entry| entry.choice(0))
                .unwrap_or_else(|| ResolutionChoice::new(3840, 2160, 60))
        };

        Self {
            current,
            resolutions,
        }
    }

    pub fn current_resolution_index(&self) -> usize {
        self.resolutions
            .iter()
            .position(|entry| entry.width == self.current.width && entry.height == self.current.height)
            .unwrap_or(0)
    }

    pub fn current_refresh_index_for(&self, resolution_index: usize) -> usize {
        self.resolutions
            .get(resolution_index)
            .and_then(|entry| {
                entry
                    .refresh_rates
                    .iter()
                    .position(|refresh_hz| *refresh_hz == self.current.refresh_hz)
            })
            .unwrap_or(0)
    }

    pub fn refresh_rates_for(&self, resolution_index: usize) -> &[u32] {
        self.resolutions
            .get(resolution_index)
            .map(|entry| entry.refresh_rates.as_slice())
            .unwrap_or(&[])
    }

    pub fn choice_for_indices(
        &self,
        resolution_index: usize,
        refresh_index: usize,
    ) -> Option<ResolutionChoice> {
        self.resolutions
            .get(resolution_index)
            .and_then(|entry| entry.choice(refresh_index))
    }
}

impl Default for ResolutionOptions {
    fn default() -> Self {
        Self::fallback()
    }
}

impl DisplayScaleChoice {
    fn new(scale_percent: u32) -> Self {
        Self {
            scale_percent,
            label: format!("{}%", scale_percent),
        }
    }
}

impl DisplayScaleOptions {
    fn fallback() -> Self {
        let scales = [100, 125, 150, 175]
            .into_iter()
            .map(DisplayScaleChoice::new)
            .collect::<Vec<_>>();

        Self {
            current: DisplayScaleChoice::new(100),
            recommended_scale_percent: 100,
            scales,
        }
    }

    fn from_relative_values(min_scale_rel: i32, cur_scale_rel: i32, max_scale_rel: i32) -> Self {
        const DISPLAY_SCALE_PERCENTAGES: [u32; 12] =
            [100, 125, 150, 175, 200, 225, 250, 300, 350, 400, 450, 500];

        let recommended_index = min_scale_rel.unsigned_abs() as usize;
        if recommended_index >= DISPLAY_SCALE_PERCENTAGES.len() {
            return Self::fallback();
        }

        let max_index = (recommended_index as i32 + max_scale_rel)
            .clamp(0, (DISPLAY_SCALE_PERCENTAGES.len() - 1) as i32)
            as usize;
        let current_index = (recommended_index as i32 + cur_scale_rel)
            .clamp(0, max_index as i32)
            as usize;
        let scales = DISPLAY_SCALE_PERCENTAGES[..=max_index]
            .iter()
            .copied()
            .map(DisplayScaleChoice::new)
            .collect::<Vec<_>>();

        Self {
            current: DisplayScaleChoice::new(DISPLAY_SCALE_PERCENTAGES[current_index]),
            recommended_scale_percent: DISPLAY_SCALE_PERCENTAGES[recommended_index],
            scales,
        }
    }

    pub fn current_scale_index(&self) -> usize {
        self.scales
            .iter()
            .position(|choice| choice.scale_percent == self.current.scale_percent)
            .unwrap_or(0)
    }

    pub fn choice_at(&self, scale_index: usize) -> Option<&DisplayScaleChoice> {
        self.scales.get(scale_index)
    }
}

impl Default for DisplayScaleOptions {
    fn default() -> Self {
        Self::fallback()
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

    let mut modes = Vec::new();
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
        modes.push((width, height, refresh_hz));
    }

    ResolutionOptions::from_modes(current_width, current_height, current_refresh_hz, modes)
}

#[cfg(not(target_os = "windows"))]
pub fn detect_resolution_options() -> ResolutionOptions {
    ResolutionOptions::default()
}

#[cfg(target_os = "windows")]
mod windows_scale {
    use std::mem::{size_of, zeroed};
    use std::ptr::null_mut;

    use winapi::shared::basetsd::UINT32;
    use winapi::shared::minwindef::DWORD;
    use winapi::shared::ntdef::{LONG, LUID};
    use winapi::shared::windef::POINT;
    use winapi::um::wingdi::{
        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_HEADER,
        DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
        QDC_ONLY_ACTIVE_PATHS, QDC_VIRTUAL_MODE_AWARE,
    };
    use winapi::um::winuser::{
        GetMonitorInfoW, MonitorFromPoint, MONITOR_DEFAULTTOPRIMARY, MONITORINFOEXW,
    };

    use super::DisplayScaleOptions;

    const DISPLAYCONFIG_DEVICE_INFO_GET_DPI_SCALE: i32 = -3;
    const DISPLAYCONFIG_DEVICE_INFO_SET_DPI_SCALE: i32 = -4;

    #[repr(C)]
    struct DisplayConfigDeviceInfoHeaderRaw {
        type_id: i32,
        size: UINT32,
        adapter_id: LUID,
        source_id: UINT32,
    }

    #[repr(C)]
    struct DisplayConfigSourceDpiScaleGet {
        header: DisplayConfigDeviceInfoHeaderRaw,
        min_scale_rel: i32,
        cur_scale_rel: i32,
        max_scale_rel: i32,
    }

    #[repr(C)]
    struct DisplayConfigSourceDpiScaleSet {
        header: DisplayConfigDeviceInfoHeaderRaw,
        scale_rel: i32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetDisplayConfigBufferSizes(
            flags: UINT32,
            num_path_array_elements: *mut UINT32,
            num_mode_info_array_elements: *mut UINT32,
        ) -> LONG;
        fn QueryDisplayConfig(
            flags: UINT32,
            num_path_array_elements: *mut UINT32,
            path_array: *mut DISPLAYCONFIG_PATH_INFO,
            num_mode_info_array_elements: *mut UINT32,
            mode_info_array: *mut DISPLAYCONFIG_MODE_INFO,
            current_topology_id: *mut u32,
        ) -> LONG;
        fn DisplayConfigGetDeviceInfo(request_packet: *mut DISPLAYCONFIG_DEVICE_INFO_HEADER) -> LONG;
        fn DisplayConfigSetDeviceInfo(set_packet: *mut DISPLAYCONFIG_DEVICE_INFO_HEADER) -> LONG;
    }

    fn wide_slice_len(value: &[u16]) -> usize {
        value.iter().position(|code| *code == 0).unwrap_or(value.len())
    }

    fn wide_slices_equal(left: &[u16], right: &[u16]) -> bool {
        let left_len = wide_slice_len(left);
        let right_len = wide_slice_len(right);
        left[..left_len] == right[..right_len]
    }

    fn primary_source() -> Option<(LUID, u32)> {
        let primary_monitor = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
        if primary_monitor.is_null() {
            return None;
        }

        let mut primary_monitor_info: MONITORINFOEXW = unsafe { zeroed() };
        primary_monitor_info.cbSize = size_of::<MONITORINFOEXW>() as DWORD;
        let monitor_info_ok = unsafe {
            GetMonitorInfoW(
                primary_monitor,
                &mut primary_monitor_info as *mut _ as *mut _,
            )
        };
        if monitor_info_ok == 0 {
            return None;
        }

        let flags = QDC_ONLY_ACTIVE_PATHS | QDC_VIRTUAL_MODE_AWARE;
        let mut path_count = 0_u32;
        let mut mode_count = 0_u32;
        if unsafe { GetDisplayConfigBufferSizes(flags, &mut path_count, &mut mode_count) } != 0 {
            return None;
        }

        let mut paths = (0..path_count)
            .map(|_| unsafe { zeroed::<DISPLAYCONFIG_PATH_INFO>() })
            .collect::<Vec<_>>();
        let mut modes = (0..mode_count)
            .map(|_| unsafe { zeroed::<DISPLAYCONFIG_MODE_INFO>() })
            .collect::<Vec<_>>();

        if unsafe {
            QueryDisplayConfig(
                flags,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                null_mut(),
            )
        } != 0
        {
            return None;
        }

        for path in paths.into_iter().take(path_count as usize) {
            let mut source_name: DISPLAYCONFIG_SOURCE_DEVICE_NAME = unsafe { zeroed() };
            source_name.header.size = size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as UINT32;
            source_name.header.adapterId = path.sourceInfo.adapterId;
            source_name.header.id = path.sourceInfo.id;
            source_name.header._type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;

            let source_name_ok = unsafe { DisplayConfigGetDeviceInfo(&mut source_name.header) };
            if source_name_ok != 0 {
                continue;
            }

            if wide_slices_equal(
                &source_name.viewGdiDeviceName,
                &primary_monitor_info.szDevice,
            ) {
                return Some((path.sourceInfo.adapterId, path.sourceInfo.id));
            }
        }

        None
    }

    pub fn detect() -> DisplayScaleOptions {
        let Some((adapter_id, source_id)) = primary_source() else {
            return DisplayScaleOptions::fallback();
        };

        let mut request = DisplayConfigSourceDpiScaleGet {
            header: DisplayConfigDeviceInfoHeaderRaw {
                type_id: DISPLAYCONFIG_DEVICE_INFO_GET_DPI_SCALE,
                size: size_of::<DisplayConfigSourceDpiScaleGet>() as UINT32,
                adapter_id,
                source_id,
            },
            min_scale_rel: 0,
            cur_scale_rel: 0,
            max_scale_rel: 0,
        };

        let status = unsafe {
            DisplayConfigGetDeviceInfo(
                &mut request as *mut _ as *mut DISPLAYCONFIG_DEVICE_INFO_HEADER,
            )
        };
        if status != 0 {
            return DisplayScaleOptions::fallback();
        }

        DisplayScaleOptions::from_relative_values(
            request.min_scale_rel,
            request.cur_scale_rel,
            request.max_scale_rel,
        )
    }

    pub fn apply(scale_percent: u32) -> bool {
        let current_options = detect();
        let recommended_index = current_options
            .scales
            .iter()
            .position(|choice| choice.scale_percent == current_options.recommended_scale_percent)
            .unwrap_or(0);
        let Some(target_index) = current_options
            .scales
            .iter()
            .position(|choice| choice.scale_percent == scale_percent)
        else {
            return false;
        };
        let Some((adapter_id, source_id)) = primary_source() else {
            return false;
        };

        let mut request = DisplayConfigSourceDpiScaleSet {
            header: DisplayConfigDeviceInfoHeaderRaw {
                type_id: DISPLAYCONFIG_DEVICE_INFO_SET_DPI_SCALE,
                size: size_of::<DisplayConfigSourceDpiScaleSet>() as UINT32,
                adapter_id,
                source_id,
            },
            scale_rel: target_index as i32 - recommended_index as i32,
        };

        (unsafe {
            DisplayConfigSetDeviceInfo(&mut request as *mut _ as *mut DISPLAYCONFIG_DEVICE_INFO_HEADER)
        }) == 0
    }
}

#[cfg(target_os = "windows")]
pub fn detect_display_scale_options() -> DisplayScaleOptions {
    windows_scale::detect()
}

#[cfg(not(target_os = "windows"))]
pub fn detect_display_scale_options() -> DisplayScaleOptions {
    DisplayScaleOptions::default()
}

#[cfg(target_os = "windows")]
pub fn apply_display_scale_choice(choice: &DisplayScaleChoice) -> bool {
    windows_scale::apply(choice.scale_percent)
}

#[cfg(not(target_os = "windows"))]
pub fn apply_display_scale_choice(_choice: &DisplayScaleChoice) -> bool {
    false
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

#[cfg(test)]
mod tests {
    use super::{DisplayScaleOptions, ResolutionOptions};

    #[test]
    fn groups_refresh_rates_by_resolution() {
        let options = ResolutionOptions::from_modes(
            2560,
            1440,
            120,
            [
                (1920, 1080, 60),
                (2560, 1440, 60),
                (2560, 1440, 120),
                (1920, 1080, 60),
            ],
        );

        assert_eq!(options.resolutions.len(), 2);
        assert_eq!(options.resolutions[0].label, "2560×1440");
        assert_eq!(options.resolutions[0].refresh_rates, vec![120, 60]);
        assert_eq!(options.current_resolution_index(), 0);
        assert_eq!(options.current_refresh_index_for(0), 0);
    }

    #[test]
    fn falls_back_when_modes_are_empty() {
        let options = ResolutionOptions::from_modes(0, 0, 0, []);

        assert_eq!(options.current.label, "3840×2160 60Hz");
        assert_eq!(options.resolutions[0].refresh_rates, vec![120, 60]);
    }

    #[test]
    fn scale_options_build_from_relative_values() {
        let options = DisplayScaleOptions::from_relative_values(-2, 1, 3);

        assert_eq!(options.recommended_scale_percent, 150);
        assert_eq!(options.current.scale_percent, 175);
        assert_eq!(
            options
                .scales
                .iter()
                .map(|choice| choice.scale_percent)
                .collect::<Vec<_>>(),
            vec![100, 125, 150, 175, 200, 225]
        );
    }
}
