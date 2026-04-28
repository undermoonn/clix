use std::collections::HashSet;
use std::path::PathBuf;

use super::{LaunchBlockedReason, LaunchState, LaunchTickResult, RunningGameState, SteamClientState};

#[path = "windows/focus.rs"]
mod focus;
#[path = "windows/process.rs"]
mod process;
#[path = "windows/steam.rs"]
mod steam;

pub(super) use self::focus::WindowTransition;

pub(super) fn capture_launch_baseline() -> (HashSet<u32>, HashSet<isize>, Option<isize>) {
    focus::capture_launch_baseline()
}

pub(super) fn launch_steam_game(
    steam_app_id: u32,
    steam_paths: &[PathBuf],
) -> Result<(), Option<LaunchBlockedReason>> {
    steam::launch_steam_game(steam_app_id, steam_paths)
}

pub(super) fn begin_refocus_transition(game_index: usize, state: &RunningGameState) -> LaunchState {
    focus::begin_refocus_transition(game_index, state)
}

pub(super) fn tick_launch_progress(
    state: &mut LaunchState,
    launch_held: bool,
) -> LaunchTickResult {
    focus::tick_launch_progress(state, launch_held)
}

pub(super) fn refresh_running_game(state: &mut RunningGameState) -> bool {
    state.tracked_pids = process::matched_running_game_pids(state);
    !state.tracked_pids.is_empty()
}

pub(super) fn close_running_game(state: &mut RunningGameState) -> bool {
    let matched = process::matched_running_game_pids(state);
    if matched.is_empty() {
        state.tracked_pids.clear();
        return false;
    }

    process::request_close_for_pids(&matched);
    process::terminate_pids(&matched);
    state.tracked_pids = matched;
    true
}

pub(super) fn start_steam_client(steam_paths: &[PathBuf]) -> bool {
    steam::start_steam_client(steam_paths)
}

pub(super) fn steam_client_state() -> SteamClientState {
    steam::steam_client_state()
}

pub(super) fn set_current_app_hwnd(hwnd: isize) {
    focus::set_current_app_hwnd(hwnd)
}

pub(super) fn current_app_window_is_background() -> bool {
    focus::current_app_window_is_background()
}

pub(super) fn focus_current_app_window() -> bool {
    focus::focus_current_app_window()
}

pub(super) fn send_current_app_to_background() -> bool {
    focus::send_current_app_to_background()
}

pub(super) fn refocus_running_game(state: &RunningGameState) -> bool {
    focus::refocus_running_game(state)
}

pub(super) fn minimize_running_game(state: &RunningGameState) -> bool {
    focus::minimize_running_game(state)
}
