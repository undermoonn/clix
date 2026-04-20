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

    use walkdir::WalkDir;
    use winapi::um::shellapi::ShellExecuteW;
    use winapi::um::winuser::SW_SHOWNORMAL;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    const APP_PATHS_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\App Paths";
    const NVIDIA_NVAPP_KEY: &str = r"SOFTWARE\NVIDIA Corporation\Global\NvApp";
    const UNINSTALL_KEYS: &[&str] = &[
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

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
        find_registered_application(
            &["DLSS Swapper.exe", "dlss swapper.exe"],
            &["dlss swapper"],
        )
    }

    fn find_nvidia_app() -> Option<PathBuf> {
        if let Some(path) = find_nvidia_full_path() {
            return Some(path);
        }

        find_registered_application(
            &["NVIDIA app.exe", "NVIDIA App.exe"],
            &["nvidia app"],
        )
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

    fn find_named_file(roots: &[Option<PathBuf>], names: &[&str], max_depth: usize) -> Option<PathBuf> {
        for root in roots.iter().flatten() {
            if !root.exists() {
                continue;
            }

            for entry in WalkDir::new(root)
                .follow_links(false)
                .max_depth(max_depth)
                .into_iter()
                .filter_map(|entry| entry.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }

                let file_name = entry.file_name().to_string_lossy();
                if names
                    .iter()
                    .any(|candidate| file_name.eq_ignore_ascii_case(candidate))
                {
                    return Some(entry.path().to_path_buf());
                }
            }
        }

        None
    }

    fn find_registered_application(exe_names: &[&str], display_names: &[&str]) -> Option<PathBuf> {
        find_registered_app_path(exe_names).or_else(|| find_registered_uninstall_entry(exe_names, display_names))
    }

    fn find_registered_app_path(exe_names: &[&str]) -> Option<PathBuf> {
        let registry_roots = [
            RegKey::predef(HKEY_CURRENT_USER),
            RegKey::predef(HKEY_LOCAL_MACHINE),
        ];

        for root in registry_roots {
            let Ok(app_paths_key) = root.open_subkey(APP_PATHS_KEY) else {
                continue;
            };

            for exe_name in exe_names {
                let Ok(exe_key) = app_paths_key.open_subkey(exe_name) else {
                    continue;
                };

                let registered: String = exe_key.get_value("").unwrap_or_default();
                let path = PathBuf::from(registered.trim_matches('"').trim());
                if path.is_file() {
                    return Some(path);
                }
            }
        }

        None
    }

    fn find_registered_uninstall_entry(exe_names: &[&str], display_names: &[&str]) -> Option<PathBuf> {
        let registry_roots = [
            RegKey::predef(HKEY_CURRENT_USER),
            RegKey::predef(HKEY_LOCAL_MACHINE),
        ];
        let display_needles = display_names
            .iter()
            .map(|name| name.to_ascii_lowercase())
            .collect::<Vec<_>>();

        for root in registry_roots {
            for uninstall_key in UNINSTALL_KEYS {
                let Ok(uninstall_root) = root.open_subkey(uninstall_key) else {
                    continue;
                };

                for subkey_name in uninstall_root.enum_keys().filter_map(Result::ok) {
                    let Ok(subkey) = uninstall_root.open_subkey(&subkey_name) else {
                        continue;
                    };

                    let display_name: String = subkey.get_value("DisplayName").unwrap_or_default();
                    let display_name_lower = display_name.to_ascii_lowercase();
                    if display_needles
                        .iter()
                        .all(|needle| !display_name_lower.contains(needle))
                    {
                        continue;
                    }

                    if let Some(path) = registered_install_location_path(&subkey, exe_names) {
                        return Some(path);
                    }

                    if let Some(path) = registered_display_icon_path(&subkey) {
                        return Some(path);
                    }
                }
            }
        }

        None
    }

    fn registered_display_icon_path(subkey: &RegKey) -> Option<PathBuf> {
        let display_icon: String = subkey.get_value("DisplayIcon").ok()?;
        let path = parse_icon_location(&display_icon)?;
        path.extension()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case("exe"))
            .unwrap_or(false)
            .then_some(path)
    }

    fn registered_install_location_path(subkey: &RegKey, exe_names: &[&str]) -> Option<PathBuf> {
        let install_location: String = subkey.get_value("InstallLocation").ok()?;
        let install_path = PathBuf::from(install_location.trim_matches('"').trim());
        if !install_path.is_dir() {
            return None;
        }

        for exe_name in exe_names {
            let candidate = install_path.join(exe_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        find_named_file(&[Some(install_path)], exe_names, 4)
    }

    fn parse_icon_location(value: &str) -> Option<PathBuf> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }

        if let Some(stripped) = trimmed.strip_prefix('"') {
            let end = stripped.find('"')?;
            let path = PathBuf::from(&stripped[..end]);
            return path.is_file().then_some(path);
        }

        let split_index = trimmed.char_indices().rev().find_map(|(index, ch)| {
            if ch == ',' {
                Some(index)
            } else if ch == '\\' || ch == '/' {
                None
            } else {
                None
            }
        });

        let path = split_index
            .map(|index| PathBuf::from(trimmed[..index].trim()))
            .unwrap_or_else(|| PathBuf::from(trimmed));
        path.is_file().then_some(path)
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