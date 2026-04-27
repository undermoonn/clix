use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::super::RunningGameState;

use winapi::shared::minwindef::{BOOL, DWORD, LPARAM, TRUE};
use winapi::shared::windef::HWND;
use winapi::um::handleapi::CloseHandle;
use winapi::um::processthreadsapi::{GetCurrentProcessId, OpenProcess, TerminateProcess};
use winapi::um::psapi::{EnumProcesses, GetModuleFileNameExW};
use winapi::um::winnt::{PROCESS_QUERY_INFORMATION, PROCESS_TERMINATE, PROCESS_VM_READ};
use winapi::um::winuser::{
    EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible, PostMessageW, WM_CLOSE,
};

pub(crate) fn normalize_windows_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_ascii_lowercase()
}

pub(crate) fn process_image_path(pid: u32) -> Option<PathBuf> {
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

pub(crate) fn collect_process_ids() -> HashSet<u32> {
    unsafe {
        let mut pids = vec![0_u32; 8192];
        let mut needed_bytes: DWORD = 0;

        if EnumProcesses(
            pids.as_mut_ptr(),
            (pids.len() * std::mem::size_of::<u32>()) as DWORD,
            &mut needed_bytes,
        ) == 0
        {
            return HashSet::new();
        }

        let count = (needed_bytes as usize) / std::mem::size_of::<u32>();
        pids.into_iter()
            .take(count)
            .filter(|pid| *pid != 0)
            .collect::<HashSet<u32>>()
    }
}

fn collect_windows(visible_only: bool) -> Vec<(HWND, u32)> {
    struct WindowCollector {
        windows: Vec<(HWND, u32)>,
    }

    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let visible_only = (lparam & 1) != 0;
        let collector = &mut *((lparam & !1) as *mut WindowCollector);

        if visible_only && IsWindowVisible(hwnd) == 0 {
            return TRUE;
        }
        if GetWindowTextLengthW(hwnd) <= 0 {
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

    let mut collector = WindowCollector { windows: Vec::new() };
    unsafe {
        EnumWindows(
            Some(enum_windows_proc),
            (&mut collector as *mut WindowCollector as LPARAM) | visible_only as LPARAM,
        );
    }
    collector.windows
}

pub(crate) fn collect_visible_windows() -> Vec<(HWND, u32)> {
    collect_windows(true)
}

pub(crate) fn collect_titled_windows() -> Vec<(HWND, u32)> {
    collect_windows(false)
}

pub(crate) fn window_title(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0_u16; (len as usize) + 1];
        let copied = GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
        if copied <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..copied as usize])
    }
}

pub(super) fn matched_running_game_pids(state: &RunningGameState) -> HashSet<u32> {
    collect_matching_process_ids(
        state.target_path.as_deref(),
        &state.game_name,
        &state.tracked_pids,
    )
}

pub(crate) fn collect_matching_process_ids(
    target_path: Option<&Path>,
    game_name: &str,
    tracked_pids: &HashSet<u32>,
) -> HashSet<u32> {
    let current_pids = collect_process_ids();
    let visible_windows = collect_visible_windows();
    let current_pid = unsafe { GetCurrentProcessId() } as u32;
    let target_norm = target_path.map(normalize_windows_path);
    let game_name_lower = game_name.to_ascii_lowercase();
    let mut matched = HashSet::new();

    for pid in current_pids {
        if pid == 0 || pid == current_pid {
            continue;
        }

        if tracked_pids.contains(&pid) {
            matched.insert(pid);
            continue;
        }

        if let Some(target_norm) = &target_norm {
            if let Some(exe_path) = process_image_path(pid) {
                let exe_norm = normalize_windows_path(&exe_path);
                if exe_norm.starts_with(target_norm) {
                    matched.insert(pid);
                    continue;
                }
            }
        }

        if !game_name_lower.is_empty()
            && visible_windows.iter().any(|(hwnd, window_pid)| {
                *window_pid == pid
                    && window_title(*hwnd)
                        .to_ascii_lowercase()
                        .contains(&game_name_lower)
            })
        {
            matched.insert(pid);
        }
    }

    matched
}

pub(crate) fn detect_launched_window(
    baseline_pids: &HashSet<u32>,
    baseline_hwnds: &HashSet<isize>,
    target_path: Option<&Path>,
    game_name: &str,
) -> Option<(HWND, u32)> {
    let target_norm = target_path.map(normalize_windows_path);
    let game_name_lower = game_name.to_ascii_lowercase();

    for (hwnd, pid) in collect_visible_windows() {
        let hwnd_key = hwnd as isize;
        let is_new_pid = !baseline_pids.contains(&pid);
        let is_new_window = !baseline_hwnds.contains(&hwnd_key);

        if !is_new_pid && !is_new_window {
            continue;
        }

        let title = window_title(hwnd).to_ascii_lowercase();
        let title_matches = !game_name_lower.is_empty() && title.contains(&game_name_lower);

        if let Some(target_norm) = &target_norm {
            if let Some(exe_path) = process_image_path(pid) {
                let exe_norm = normalize_windows_path(&exe_path);
                if exe_norm.starts_with(target_norm) {
                    return Some((hwnd, pid));
                }
            }
            if title_matches {
                return Some((hwnd, pid));
            }
            if is_new_pid && !title.is_empty() {
                return Some((hwnd, pid));
            }
        } else {
            return Some((hwnd, pid));
        }

        if is_new_window && title_matches {
            return Some((hwnd, pid));
        }
    }

    None
}

pub(super) fn request_close_for_pids(pids: &HashSet<u32>) {
    for (hwnd, pid) in collect_visible_windows() {
        if pids.contains(&pid) {
            unsafe {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            }
        }
    }
}

pub(super) fn terminate_pids(pids: &HashSet<u32>) {
    for pid in pids {
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, 0, *pid as DWORD);
            if handle.is_null() {
                continue;
            }

            let _ = TerminateProcess(handle, 1);
            CloseHandle(handle);
        }
    }
}
