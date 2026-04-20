use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExternalAppKind {
    DlssSwapper,
    NvidiaApp,
}

#[derive(Clone, Debug)]
pub struct ExternalApp {
    kind: ExternalAppKind,
    launch_target: PathBuf,
    icon_target: PathBuf,
}

impl ExternalApp {
    pub fn kind(&self) -> ExternalAppKind {
        self.kind
    }

    pub fn launch_target(&self) -> &Path {
        &self.launch_target
    }

    pub fn icon_target(&self) -> &Path {
        &self.icon_target
    }
}

pub fn detect_installed() -> Vec<ExternalApp> {
    imp::detect_installed()
}

pub fn launch(kind: ExternalAppKind, apps: &[ExternalApp]) -> bool {
    let Some(app) = apps.iter().find(|app| app.kind == kind) else {
        return false;
    };

    imp::launch_path(app.launch_target())
}

#[cfg(target_os = "windows")]
mod imp {
    use super::{ExternalApp, ExternalAppKind};
    use std::ffi::OsStr;
    use std::iter;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};

    use winapi::um::shellapi::ShellExecuteW;
    use winapi::um::winuser::SW_SHOWNORMAL;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    const DLSS_SWAPPER_UNINSTALL_KEY: &str =
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall\DLSS Swapper";
    const NVIDIA_NVAPP_KEY: &str = r"SOFTWARE\NVIDIA Corporation\Global\NvApp";

    pub fn detect_installed() -> Vec<ExternalApp> {
        let mut apps = Vec::new();

        if let Some(launch_target) = find_dlss_swapper() {
            apps.push(build_external_app(ExternalAppKind::DlssSwapper, launch_target));
        }

        if let Some(launch_target) = find_nvidia_app() {
            apps.push(build_external_app(ExternalAppKind::NvidiaApp, launch_target));
        }

        apps
    }

    fn build_external_app(kind: ExternalAppKind, launch_target: PathBuf) -> ExternalApp {
        ExternalApp {
            kind,
            icon_target: launch_target.clone(),
            launch_target,
        }
    }

    pub fn launch_path(path: &Path) -> bool {
        unsafe {
            let operation = wide("open");
            let target = wide_os(path.as_os_str());
            (ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                target.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            ) as isize)
                > 32
        }
    }

    fn find_dlss_swapper() -> Option<PathBuf> {
        find_uninstall_display_icon_path(DLSS_SWAPPER_UNINSTALL_KEY)
    }

    fn find_nvidia_app() -> Option<PathBuf> {
        find_nvidia_full_path()
    }

    fn find_nvidia_full_path() -> Option<PathBuf> {
        let registry_roots = [
            RegKey::predef(HKEY_CURRENT_USER),
            RegKey::predef(HKEY_LOCAL_MACHINE),
        ];

        for root in registry_roots {
            let Ok(nvapp_key) = root.open_subkey(NVIDIA_NVAPP_KEY) else {
                continue;
            };

            let installed = nvapp_key.get_value::<u32, _>("Installed").unwrap_or(1);
            if installed == 0 {
                continue;
            }

            let full_path: String = nvapp_key.get_value("FullPath").unwrap_or_default();
            let path = PathBuf::from(full_path.trim_matches('"').trim());
            if path.is_file() {
                return Some(path);
            }
        }

        None
    }

    fn find_uninstall_display_icon_path(key_path: &str) -> Option<PathBuf> {
        let registry_roots = [
            RegKey::predef(HKEY_CURRENT_USER),
            RegKey::predef(HKEY_LOCAL_MACHINE),
        ];

        for root in registry_roots {
            let Ok(subkey) = root.open_subkey(key_path) else {
                continue;
            };

            let Ok(display_icon) = subkey.get_value::<String, _>("DisplayIcon") else {
                continue;
            };
            let Some(path) = parse_icon_location(&display_icon) else {
                continue;
            };
            if path.is_file() {
                return Some(path);
            }
        }

        None
    }
    fn parse_icon_location(value: &str) -> Option<PathBuf> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }

        if let Some(stripped) = trimmed.strip_prefix('"') {
            let end = stripped.find('"')?;
            return Some(PathBuf::from(&stripped[..end]));
        }

        let mut split_index = None;
        for (index, ch) in trimmed.char_indices().rev() {
            if ch == ',' {
                split_index = Some(index);
                break;
            }
            if ch == '\\' || ch == '/' {
                break;
            }
        }

        Some(match split_index {
            Some(index) => PathBuf::from(trimmed[..index].trim()),
            None => PathBuf::from(trimmed),
        })
    }

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value)
            .encode_wide()
            .chain(iter::once(0))
            .collect()
    }

    fn wide_os(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(iter::once(0)).collect()
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::ExternalApp;
    use std::path::Path;

    pub fn detect_installed() -> Vec<ExternalApp> {
        Vec::new()
    }

    pub fn launch_path(_path: &Path) -> bool {
        false
    }
}