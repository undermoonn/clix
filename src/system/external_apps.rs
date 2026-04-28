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

    use winapi::shared::minwindef::{BOOL, DWORD, LPARAM, TRUE};
    use winapi::shared::windef::HWND;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::{GetCurrentThreadId, OpenProcess};
    use winapi::um::psapi::GetModuleFileNameExW;
    use winapi::um::shellapi::ShellExecuteW;
    use winapi::um::winnt::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
    use winapi::um::winuser::{
        AttachThreadInput, BringWindowToTop, EnumWindows, GetForegroundWindow,
        GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetActiveWindow, SetFocus,
        SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOWNORMAL,
    };
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    const DLSS_SWAPPER_UNINSTALL_KEY: &str =
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall\DLSS Swapper";
    const NVIDIA_NVAPP_KEY: &str = r"SOFTWARE\NVIDIA Corporation\Global\NvApp";

    pub fn detect_installed() -> Vec<ExternalApp> {
        let mut apps = Vec::new();

        if let Some(launch_target) = find_dlss_swapper() {
            apps.push(build_external_app(
                ExternalAppKind::DlssSwapper,
                launch_target,
            ));
        }

        if let Some(launch_target) = find_nvidia_app() {
            apps.push(build_external_app(
                ExternalAppKind::NvidiaApp,
                launch_target,
            ));
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
        if focus_running_instance(path) {
            return true;
        }

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

    fn focus_running_instance(path: &Path) -> bool {
        let target_norm = normalize_windows_path(path);

        for (hwnd, pid) in collect_visible_windows() {
            let Some(exe_path) = process_image_path(pid) else {
                continue;
            };

            if normalize_windows_path(&exe_path) == target_norm {
                bring_window_to_foreground(hwnd);
                return true;
            }
        }

        false
    }

    fn normalize_windows_path(path: &Path) -> String {
        path.to_string_lossy()
            .replace('/', "\\")
            .to_ascii_lowercase()
    }

    fn process_image_path(pid: u32) -> Option<PathBuf> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid as DWORD);
            if handle.is_null() {
                return None;
            }

            let mut buf = vec![0_u16; 1024];
            let len = GetModuleFileNameExW(
                handle,
                std::ptr::null_mut(),
                buf.as_mut_ptr(),
                buf.len() as DWORD,
            );
            CloseHandle(handle);

            if len == 0 {
                return None;
            }

            let path = String::from_utf16_lossy(&buf[..len as usize]);
            Some(PathBuf::from(path))
        }
    }

    fn collect_visible_windows() -> Vec<(HWND, u32)> {
        struct WindowCollector {
            windows: Vec<(HWND, u32)>,
        }

        unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let collector = &mut *(lparam as *mut WindowCollector);

            if IsWindowVisible(hwnd) == 0 {
                return TRUE;
            }

            let mut pid: DWORD = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == 0 {
                return TRUE;
            }

            collector.windows.push((hwnd, pid as u32));
            TRUE
        }

        let mut collector = WindowCollector {
            windows: Vec::new(),
        };
        unsafe {
            EnumWindows(
                Some(enum_windows_proc),
                &mut collector as *mut WindowCollector as LPARAM,
            );
        }

        collector.windows
    }

    fn bring_window_to_foreground(hwnd: HWND) {
        unsafe {
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_RESTORE);
            }

            let current_thread_id = GetCurrentThreadId();
            let foreground_hwnd = GetForegroundWindow();
            let foreground_thread_id = if foreground_hwnd.is_null() {
                0
            } else {
                GetWindowThreadProcessId(foreground_hwnd, std::ptr::null_mut())
            };
            let target_thread_id = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());

            let attach_foreground = foreground_thread_id != 0
                && foreground_thread_id != current_thread_id
                && AttachThreadInput(current_thread_id, foreground_thread_id, TRUE) != 0;
            let attach_target = target_thread_id != 0
                && target_thread_id != current_thread_id
                && AttachThreadInput(current_thread_id, target_thread_id, TRUE) != 0;

            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
            SetActiveWindow(hwnd);
            SetFocus(hwnd);

            if attach_target {
                AttachThreadInput(current_thread_id, target_thread_id, 0);
            }
            if attach_foreground {
                AttachThreadInput(current_thread_id, foreground_thread_id, 0);
            }
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
