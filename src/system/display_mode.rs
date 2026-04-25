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
    use super::ResolutionOptions;

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
}