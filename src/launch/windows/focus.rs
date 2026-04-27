use std::collections::HashSet;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::{Duration, Instant};

use crate::animation;

use super::process::{
    collect_matching_process_ids, collect_process_ids, collect_visible_windows,
    detect_launched_window, matched_running_game_pids, window_title,
};
use super::super::{LaunchState, LaunchTickResult, RunningGameState};

use winapi::shared::minwindef::TRUE;
use winapi::shared::windef::HWND;
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::winuser::{
    AttachThreadInput, BringWindowToTop, GetForegroundWindow, GetWindowLongW, SetActiveWindow,
    SetFocus, SetForegroundWindow, SetLayeredWindowAttributes, SetWindowLongW,
    SetWindowPos, ShowWindow, GetWindowThreadProcessId, GWL_EXSTYLE, LWA_ALPHA,
    SW_MINIMIZE, SW_RESTORE, SWP_NOSIZE, SWP_NOZORDER, WS_EX_LAYERED,
};

const ALIGN_TOP_LEFT_STEAM_APP_ID: u32 = 601150;
const WINDOW_TRANSITION_MS: u64 = 100;

static CURRENT_APP_HWND: AtomicIsize = AtomicIsize::new(0);

pub(in crate::launch) struct WindowTransition {
    target_hwnd: HWND,
    pub(super) target_pid: u32,
    started_at: Instant,
    target_ex_style: i32,
}

pub(super) fn capture_launch_baseline() -> (HashSet<u32>, HashSet<isize>, Option<isize>) {
    let baseline_pids = collect_process_ids();
    let baseline_hwnds = collect_visible_windows()
        .into_iter()
        .map(|(hwnd, _)| hwnd as isize)
        .collect();
    let current_app_hwnd = find_current_app_window().map(|hwnd| hwnd as isize);

    (baseline_pids, baseline_hwnds, current_app_hwnd)
}

pub(super) fn begin_focus_transition(game_index: usize, state: &RunningGameState) -> LaunchState {
    LaunchState {
        game_index,
        game_name: state.game_name.clone(),
        started_at: Instant::now(),
        launch_steam_app_id: state.steam_app_id,
        awaiting_launch_release: true,
        baseline_pids: HashSet::new(),
        baseline_hwnds: HashSet::new(),
        target_path: state.target_path.clone(),
        focus_pids: Some(state.tracked_pids.clone()),
        current_app_hwnd: find_current_app_window().map(|hwnd| hwnd as isize),
        transition: None,
    }
}

pub(super) fn tick_launch_progress(
    state: &mut LaunchState,
    launch_held: bool,
) -> LaunchTickResult {
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
            maybe_align_window_top_left(hwnd, state.launch_steam_app_id);
            bring_window_to_foreground(hwnd);
            return LaunchTickResult::Ready(build_running_game_state(state, pid));
        }

        if now.duration_since(state.started_at) >= Duration::from_secs(3) {
            return LaunchTickResult::TimedOut;
        }

        return LaunchTickResult::Pending;
    }

    if let Some((hwnd, pid)) = detect_launched_window(
        &state.baseline_pids,
        &state.baseline_hwnds,
        state.target_path.as_deref(),
        &state.game_name,
    ) {
        maybe_align_window_top_left(hwnd, state.launch_steam_app_id);
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

pub(super) fn set_current_app_hwnd(hwnd: isize) {
    CURRENT_APP_HWND.store(hwnd, Ordering::Release);
}

pub(super) fn current_app_window_is_background() -> bool {
    if let Some(hwnd) = find_current_app_window() {
        unsafe { GetForegroundWindow() != hwnd }
    } else {
        false
    }
}

pub(super) fn focus_current_app_window() -> bool {
    if let Some(hwnd) = find_current_app_window() {
        let was_background = unsafe { GetForegroundWindow() != hwnd };
        bring_window_to_foreground(hwnd);
        was_background
    } else {
        false
    }
}

pub(super) fn send_current_app_to_background() -> bool {
    if let Some(hwnd) = find_current_app_window() {
        unsafe {
            ShowWindow(hwnd, SW_MINIMIZE);
        }
        true
    } else {
        false
    }
}

pub(super) fn focus_running_game(state: &RunningGameState) -> bool {
    let matched = matched_running_game_pids(state);
    focus_best_window_for_pids(&matched, &state.game_name)
}

fn cached_current_app_window() -> Option<HWND> {
    let hwnd = CURRENT_APP_HWND.load(Ordering::Acquire);
    (hwnd != 0).then_some(hwnd as HWND)
}

fn find_current_app_window() -> Option<HWND> {
    cached_current_app_window()
}

fn bring_window_to_foreground(hwnd: HWND) {
    unsafe {
        ShowWindow(hwnd, SW_RESTORE);

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
        steam_app_id: state.launch_steam_app_id,
        game_name: state.game_name.clone(),
        target_path: state.target_path.clone(),
        tracked_pids,
    }
}

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

fn restore_window_ex_style(hwnd: HWND, ex_style: i32) {
    unsafe {
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style);
    }
}

fn set_window_alpha(hwnd: HWND, alpha: u8) -> bool {
    unsafe { SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA) != 0 }
}

fn start_window_transition(
    _source_hwnd: HWND,
    target_hwnd: HWND,
    target_pid: u32,
) -> Option<WindowTransition> {
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

fn finish_window_transition(transition: &WindowTransition) {
    let _ = set_window_alpha(transition.target_hwnd, 255);
    restore_window_ex_style(transition.target_hwnd, transition.target_ex_style);
    bring_window_to_foreground(transition.target_hwnd);
}

fn tick_window_transition(transition: &mut WindowTransition) -> bool {
    let elapsed = Instant::now().duration_since(transition.started_at);
    let progress = (animation::scale_seconds(elapsed.as_secs_f32())
        / (WINDOW_TRANSITION_MS as f32 / 1000.0))
        .clamp(0.0, 1.0);
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

fn maybe_align_window_top_left(hwnd: HWND, steam_app_id: Option<u32>) {
    if steam_app_id != Some(ALIGN_TOP_LEFT_STEAM_APP_ID) {
        return;
    }

    unsafe {
        SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
    }
}

fn focus_best_window_for_pids(pids: &HashSet<u32>, game_name: &str) -> bool {
    if let Some((hwnd, _)) = find_best_window_for_pids(pids, game_name) {
        bring_window_to_foreground(hwnd);
        return true;
    }

    false
}

fn find_best_window_for_pids(pids: &HashSet<u32>, game_name: &str) -> Option<(HWND, u32)> {
    if pids.is_empty() {
        return None;
    }

    let game_name_lower = game_name.to_ascii_lowercase();
    let windows = collect_visible_windows();

    for &(hwnd, pid) in &windows {
        if pids.contains(&pid)
            && !game_name_lower.is_empty()
            && window_title(hwnd)
                .to_ascii_lowercase()
                .contains(&game_name_lower)
        {
            return Some((hwnd, pid));
        }
    }

    for (hwnd, pid) in windows {
        if pids.contains(&pid) {
            return Some((hwnd, pid));
        }
    }

    None
}
