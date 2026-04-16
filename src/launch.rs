use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crate::steam::Game;

pub struct LaunchState {
    pub game_index: usize,
    game_name: String,
    started_at: Instant,
    launch_app_id: Option<u32>,
    #[cfg(target_os = "windows")]
    baseline_pids: HashSet<u32>,
    #[cfg(target_os = "windows")]
    baseline_hwnds: HashSet<isize>,
    #[cfg(target_os = "windows")]
    target_path: Option<PathBuf>,
    #[cfg(target_os = "windows")]
    last_keep_foreground_at: Instant,
}

pub fn begin_launch(game_index: usize, game: &Game, steam_paths: &[PathBuf]) -> Option<LaunchState> {
    let target_path = game.path.clone();
    let game_name = game.name.clone();
    let launch_app_id = game.app_id;
    #[cfg(target_os = "windows")]
    let baseline_pids = collect_process_ids();
    #[cfg(target_os = "windows")]
    let baseline_hwnds: HashSet<isize> = collect_visible_windows()
        .into_iter()
        .map(|(hwnd, _)| hwnd as isize)
        .collect();

    let launched = if let Some(app_id) = game.app_id {
        #[cfg(target_os = "windows")]
        {
            let steam_exe = steam_paths
                .iter()
                .map(|path| path.join("steam.exe"))
                .find(|path| path.exists());

            if let Some(steam_exe) = steam_exe {
                Command::new(steam_exe)
                    .args(["-applaunch", &app_id.to_string()])
                    .spawn()
                    .is_ok()
            } else {
                let url = format!("steam://rungameid/{}", app_id);
                Command::new("cmd")
                    .args(["/C", "start", "", &url])
                    .spawn()
                    .is_ok()
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = steam_paths;
            false
        }
    } else {
        Command::new(&game.path).spawn().is_ok()
    };

    if !launched {
        return None;
    }

    Some(LaunchState {
        game_index,
        game_name,
        started_at: Instant::now(),
        launch_app_id,
        #[cfg(target_os = "windows")]
        baseline_pids,
        #[cfg(target_os = "windows")]
        baseline_hwnds,
        #[cfg(target_os = "windows")]
        target_path: if target_path.exists() { Some(target_path) } else { None },
        #[cfg(target_os = "windows")]
        last_keep_foreground_at: Instant::now() - Duration::from_millis(400),
    })
}

pub fn tick_launch_progress(state: &mut LaunchState) -> bool {
    #[cfg(target_os = "windows")]
    {
        let now = Instant::now();

        if now.duration_since(state.last_keep_foreground_at) >= Duration::from_millis(250) {
            bring_current_app_to_foreground();
            state.last_keep_foreground_at = now;
        }

        if let Some(hwnd) = detect_launched_window(
            &state.baseline_pids,
            &state.baseline_hwnds,
            state.target_path.as_deref(),
            &state.game_name,
        ) {
            maybe_align_window_top_left(hwnd, state.launch_app_id);
            bring_window_to_foreground(hwnd);
            return true;
        }

        now.duration_since(state.started_at) >= Duration::from_secs(25)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Instant::now().duration_since(state.started_at) >= Duration::from_secs(5)
    }
}

#[cfg(target_os = "windows")]
use winapi::shared::minwindef::{BOOL, DWORD, LPARAM, TRUE};
#[cfg(target_os = "windows")]
use winapi::shared::windef::HWND;
#[cfg(target_os = "windows")]
use winapi::um::handleapi::CloseHandle;
#[cfg(target_os = "windows")]
use winapi::um::processthreadsapi::{GetCurrentProcessId, OpenProcess};
#[cfg(target_os = "windows")]
use winapi::um::psapi::{EnumProcesses, GetModuleFileNameExW};
#[cfg(target_os = "windows")]
use winapi::um::winnt::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
#[cfg(target_os = "windows")]
use winapi::um::winuser::{
    EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
    SetForegroundWindow, SetWindowPos, ShowWindow, SWP_NOSIZE, SWP_NOZORDER, SW_RESTORE,
};

#[cfg(target_os = "windows")]
const ALIGN_TOP_LEFT_APP_ID: u32 = 601150;

#[cfg(target_os = "windows")]
fn normalize_windows_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_ascii_lowercase()
}

#[cfg(target_os = "windows")]
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

#[cfg(target_os = "windows")]
fn collect_process_ids() -> HashSet<u32> {
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

#[cfg(target_os = "windows")]
fn collect_visible_windows() -> Vec<(HWND, u32)> {
    struct WindowCollector {
        windows: Vec<(HWND, u32)>,
    }

    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if IsWindowVisible(hwnd) == 0 {
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

        let collector = &mut *(lparam as *mut WindowCollector);
        collector.windows.push((hwnd, pid as u32));
        TRUE
    }

    let mut collector = WindowCollector { windows: Vec::new() };
    unsafe {
        EnumWindows(
            Some(enum_windows_proc),
            &mut collector as *mut WindowCollector as LPARAM,
        );
    }
    collector.windows
}

#[cfg(target_os = "windows")]
fn window_title(hwnd: HWND) -> String {
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

#[cfg(target_os = "windows")]
fn bring_window_to_foreground(hwnd: HWND) {
    unsafe {
        ShowWindow(hwnd, SW_RESTORE);
        SetForegroundWindow(hwnd);
    }
}

#[cfg(target_os = "windows")]
fn maybe_align_window_top_left(hwnd: HWND, app_id: Option<u32>) {
    if app_id != Some(ALIGN_TOP_LEFT_APP_ID) {
        return;
    }

    unsafe {
        SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
    }
}

#[cfg(target_os = "windows")]
fn bring_current_app_to_foreground() {
    let current_pid = unsafe { GetCurrentProcessId() } as u32;
    for (hwnd, pid) in collect_visible_windows() {
        if pid == current_pid {
            bring_window_to_foreground(hwnd);
            break;
        }
    }
}

#[cfg(target_os = "windows")]
fn detect_launched_window(
    baseline_pids: &HashSet<u32>,
    baseline_hwnds: &HashSet<isize>,
    target_path: Option<&Path>,
    game_name: &str,
) -> Option<HWND> {
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
                    return Some(hwnd);
                }
            }
            if title_matches {
                return Some(hwnd);
            }
            if is_new_pid && !title.is_empty() {
                return Some(hwnd);
            }
        } else {
            return Some(hwnd);
        }

        if is_new_window && title_matches {
            return Some(hwnd);
        }
    }

    None
}