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
    awaiting_launch_release: bool,
    #[cfg(target_os = "windows")]
    baseline_pids: HashSet<u32>,
    #[cfg(target_os = "windows")]
    baseline_hwnds: HashSet<isize>,
    #[cfg(target_os = "windows")]
    target_path: Option<PathBuf>,
    #[cfg(target_os = "windows")]
    last_keep_foreground_at: Instant,
    #[cfg(target_os = "windows")]
    focus_pids: Option<HashSet<u32>>,
    #[cfg(target_os = "windows")]
    current_app_hwnd: Option<isize>,
    #[cfg(target_os = "windows")]
    transition: Option<WindowTransition>,
}

pub struct RunningGameState {
    pub game_index: usize,
    app_id: Option<u32>,
    game_name: String,
    #[cfg(target_os = "windows")]
    target_path: Option<PathBuf>,
    #[cfg(target_os = "windows")]
    tracked_pids: HashSet<u32>,
}

impl LaunchState {
    pub fn elapsed_seconds(&self) -> f32 {
        self.started_at.elapsed().as_secs_f32()
    }
}

impl RunningGameState {
    pub fn matches_game(&self, game: &Game) -> bool {
        if let Some(app_id) = self.app_id {
            return game.app_id == Some(app_id);
        }

        game.name == self.game_name
    }

    pub fn with_game_index(mut self, game_index: usize) -> Self {
        self.game_index = game_index;
        self
    }
}

pub enum LaunchTickResult {
    Pending,
    Ready(RunningGameState),
    TimedOut,
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
        awaiting_launch_release: false,
        #[cfg(target_os = "windows")]
        baseline_pids,
        #[cfg(target_os = "windows")]
        baseline_hwnds,
        #[cfg(target_os = "windows")]
        target_path: if target_path.exists() { Some(target_path) } else { None },
        #[cfg(target_os = "windows")]
        last_keep_foreground_at: Instant::now() - Duration::from_millis(400),
        #[cfg(target_os = "windows")]
        focus_pids: None,
        #[cfg(target_os = "windows")]
        current_app_hwnd: find_current_app_window().map(|hwnd| hwnd as isize),
        #[cfg(target_os = "windows")]
        transition: None,
    })
}

pub fn begin_focus_transition(game_index: usize, state: &RunningGameState) -> Option<LaunchState> {
    #[cfg(target_os = "windows")]
    {
        Some(LaunchState {
            game_index,
            game_name: state.game_name.clone(),
            started_at: Instant::now(),
            launch_app_id: state.app_id,
            awaiting_launch_release: true,
            baseline_pids: HashSet::new(),
            baseline_hwnds: HashSet::new(),
            target_path: state.target_path.clone(),
            last_keep_foreground_at: Instant::now(),
            focus_pids: Some(state.tracked_pids.clone()),
            current_app_hwnd: find_current_app_window().map(|hwnd| hwnd as isize),
            transition: None,
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = game_index;
        let _ = state;
        None
    }
}

pub fn tick_launch_progress(state: &mut LaunchState, launch_held: bool) -> LaunchTickResult {
    #[cfg(target_os = "windows")]
    {
        let now = Instant::now();

        if state.awaiting_launch_release {
            if launch_held {
                return LaunchTickResult::Pending;
            }
            state.awaiting_launch_release = false;
        }

        if let Some(mut transition) = state.transition.take() {
            if tick_window_transition(&mut transition) {
                return LaunchTickResult::Ready(build_running_game_state(state, transition.target_pid));
            }
            state.transition = Some(transition);
            return LaunchTickResult::Pending;
        }

        if let Some(focus_pids) = &state.focus_pids {
            if let Some((hwnd, pid)) = find_best_window_for_pids(focus_pids, &state.game_name) {
                maybe_align_window_top_left(hwnd, state.launch_app_id);

                if let Some(current_hwnd) = state.current_app_hwnd.map(|hwnd| hwnd as HWND) {
                    if current_hwnd != hwnd {
                        if let Some(transition) = start_window_transition(current_hwnd, hwnd, pid) {
                            state.transition = Some(transition);
                            return LaunchTickResult::Pending;
                        }
                    }
                }

                bring_window_to_foreground(hwnd);
                return LaunchTickResult::Ready(build_running_game_state(state, pid));
            }

            if now.duration_since(state.started_at) >= Duration::from_secs(3) {
                return LaunchTickResult::TimedOut;
            }

            return LaunchTickResult::Pending;
        }

        if now.duration_since(state.last_keep_foreground_at) >= Duration::from_millis(250) {
            bring_current_app_to_foreground();
            state.last_keep_foreground_at = now;
        }

        if let Some((hwnd, pid)) = detect_launched_window(
            &state.baseline_pids,
            &state.baseline_hwnds,
            state.target_path.as_deref(),
            &state.game_name,
        ) {
            maybe_align_window_top_left(hwnd, state.launch_app_id);
            if let Some(current_hwnd) = state.current_app_hwnd.map(|hwnd| hwnd as HWND) {
                if current_hwnd != hwnd {
                    if let Some(transition) = start_window_transition(current_hwnd, hwnd, pid) {
                        state.transition = Some(transition);
                        return LaunchTickResult::Pending;
                    }
                }
            }

            bring_window_to_foreground(hwnd);
            return LaunchTickResult::Ready(build_running_game_state(state, pid));
        }

        if now.duration_since(state.started_at) >= Duration::from_secs(25) {
            LaunchTickResult::TimedOut
        } else {
            LaunchTickResult::Pending
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = launch_held;
        if Instant::now().duration_since(state.started_at) >= Duration::from_secs(5) {
            LaunchTickResult::TimedOut
        } else {
            LaunchTickResult::Pending
        }
    }
}

pub fn refresh_running_game(state: &mut RunningGameState) -> bool {
    #[cfg(target_os = "windows")]
    {
        let matched = collect_matching_process_ids(
            state.target_path.as_deref(),
            &state.game_name,
            &state.tracked_pids,
        );
        state.tracked_pids = matched;
        !state.tracked_pids.is_empty()
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        false
    }
}

pub fn close_running_game(state: &mut RunningGameState) -> bool {
    #[cfg(target_os = "windows")]
    {
        let matched = collect_matching_process_ids(
            state.target_path.as_deref(),
            &state.game_name,
            &state.tracked_pids,
        );
        if matched.is_empty() {
            state.tracked_pids.clear();
            return false;
        }

        request_close_for_pids(&matched);
        terminate_pids(&matched);
        state.tracked_pids = matched;
        true
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        false
    }
}

#[cfg(target_os = "windows")]
use winapi::shared::minwindef::{BOOL, DWORD, LPARAM, TRUE};
#[cfg(target_os = "windows")]
use winapi::shared::windef::HWND;
#[cfg(target_os = "windows")]
use winapi::um::handleapi::CloseHandle;
#[cfg(target_os = "windows")]
use winapi::um::processthreadsapi::{GetCurrentProcessId, OpenProcess, TerminateProcess};
#[cfg(target_os = "windows")]
use winapi::um::psapi::{EnumProcesses, GetModuleFileNameExW};
#[cfg(target_os = "windows")]
use winapi::um::winnt::{PROCESS_QUERY_INFORMATION, PROCESS_TERMINATE, PROCESS_VM_READ};
#[cfg(target_os = "windows")]
use winapi::um::winuser::{
    EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
    GetForegroundWindow, GetWindowLongW, PostMessageW, SetForegroundWindow,
    SetLayeredWindowAttributes,
    SetWindowLongW, SetWindowPos, ShowWindow, GWL_EXSTYLE, LWA_ALPHA, SWP_NOSIZE,
    SWP_NOZORDER, SW_RESTORE, WM_CLOSE, WS_EX_LAYERED,
};

#[cfg(target_os = "windows")]
const ALIGN_TOP_LEFT_APP_ID: u32 = 601150;
#[cfg(target_os = "windows")]
const WINDOW_TRANSITION_MS: u64 = 100;

#[cfg(target_os = "windows")]
struct WindowTransition {
    target_hwnd: HWND,
    target_pid: u32,
    started_at: Instant,
    target_ex_style: i32,
}

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
fn build_running_game_state(state: &LaunchState, fallback_pid: u32) -> RunningGameState {
    let tracked_pids = if let Some(tracked_pids) = &state.focus_pids {
        let matched = collect_matching_process_ids(
            state.target_path.as_deref(),
            &state.game_name,
            tracked_pids,
        );
        if matched.is_empty() {
            tracked_pids.clone()
        } else {
            matched
        }
    } else {
        std::iter::once(fallback_pid).collect()
    };

    RunningGameState {
        game_index: state.game_index,
        app_id: state.launch_app_id,
        game_name: state.game_name.clone(),
        target_path: state.target_path.clone(),
        tracked_pids,
    }
}

#[cfg(target_os = "windows")]
fn enable_layered_alpha(hwnd: HWND) -> Option<i32> {
    unsafe {
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED as i32);
        let updated_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        if (updated_style & WS_EX_LAYERED as i32) == 0 {
            None
        } else {
            Some(ex_style)
        }
    }
}

#[cfg(target_os = "windows")]
fn restore_window_ex_style(hwnd: HWND, ex_style: i32) {
    unsafe {
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style);
    }
}

#[cfg(target_os = "windows")]
fn set_window_alpha(hwnd: HWND, alpha: u8) -> bool {
    unsafe { SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA) != 0 }
}

#[cfg(target_os = "windows")]
fn start_window_transition(_source_hwnd: HWND, target_hwnd: HWND, target_pid: u32) -> Option<WindowTransition> {
    let target_ex_style = enable_layered_alpha(target_hwnd)?;

    if !set_window_alpha(target_hwnd, 0) {
        restore_window_ex_style(target_hwnd, target_ex_style);
        return None;
    }

    bring_window_to_foreground(target_hwnd);

    Some(WindowTransition {
        target_hwnd,
        target_pid,
        started_at: Instant::now(),
        target_ex_style,
    })
}

#[cfg(target_os = "windows")]
fn finish_window_transition(transition: &WindowTransition) {
    let _ = set_window_alpha(transition.target_hwnd, 255);
    restore_window_ex_style(transition.target_hwnd, transition.target_ex_style);
    bring_window_to_foreground(transition.target_hwnd);
}

#[cfg(target_os = "windows")]
fn tick_window_transition(transition: &mut WindowTransition) -> bool {
    let elapsed = Instant::now().duration_since(transition.started_at);
    let progress = (elapsed.as_secs_f32() / (WINDOW_TRANSITION_MS as f32 / 1000.0)).clamp(0.0, 1.0);
    let target_alpha = (progress * 255.0).round().clamp(0.0, 255.0) as u8;

    if !set_window_alpha(transition.target_hwnd, target_alpha) {
        finish_window_transition(transition);
        return true;
    }

    if progress >= 1.0 {
        finish_window_transition(transition);
        true
    } else {
        false
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
pub fn current_app_window_is_background() -> bool {
    if let Some(hwnd) = find_current_app_window() {
        unsafe { GetForegroundWindow() != hwnd }
    } else {
        false
    }
}

#[cfg(not(target_os = "windows"))]
pub fn current_app_window_is_background() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn focus_current_app_window() -> bool {
    if let Some(hwnd) = find_current_app_window() {
        let was_background = unsafe { GetForegroundWindow() != hwnd };
        bring_window_to_foreground(hwnd);
        was_background
    } else {
        false
    }
}

#[cfg(target_os = "windows")]
fn find_current_app_window() -> Option<HWND> {
    let current_pid = unsafe { GetCurrentProcessId() } as u32;
    collect_visible_windows()
        .into_iter()
        .find_map(|(hwnd, pid)| (pid == current_pid).then_some(hwnd))
}

#[cfg(target_os = "windows")]
fn detect_launched_window(
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

#[cfg(target_os = "windows")]
fn collect_matching_process_ids(
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

#[cfg(target_os = "windows")]
fn request_close_for_pids(pids: &HashSet<u32>) {
    for (hwnd, pid) in collect_visible_windows() {
        if pids.contains(&pid) {
            unsafe {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn terminate_pids(pids: &HashSet<u32>) {
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

pub fn focus_running_game(state: &RunningGameState) -> bool {
    #[cfg(target_os = "windows")]
    {
        let matched = collect_matching_process_ids(
            state.target_path.as_deref(),
            &state.game_name,
            &state.tracked_pids,
        );
        focus_best_window_for_pids(&matched, &state.game_name)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        false
    }
}

#[cfg(target_os = "windows")]
fn focus_best_window_for_pids(pids: &HashSet<u32>, game_name: &str) -> bool {
    if let Some((hwnd, _)) = find_best_window_for_pids(pids, game_name) {
        bring_window_to_foreground(hwnd);
        return true;
    }

    false
}

#[cfg(target_os = "windows")]
fn find_best_window_for_pids(pids: &HashSet<u32>, game_name: &str) -> Option<(HWND, u32)> {
    if pids.is_empty() {
        return None;
    }

    let game_name_lower = game_name.to_ascii_lowercase();

    for (hwnd, pid) in collect_visible_windows() {
        if pids.contains(&pid)
            && !game_name_lower.is_empty()
            && window_title(hwnd)
                .to_ascii_lowercase()
                .contains(&game_name_lower)
        {
            return Some((hwnd, pid));
        }
    }

    for (hwnd, pid) in collect_visible_windows() {
        if pids.contains(&pid) {
            return Some((hwnd, pid));
        }
    }

    None
}
