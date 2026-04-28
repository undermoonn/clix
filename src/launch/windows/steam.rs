use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::Instant;

use super::super::{append_steam_client_state_log, LaunchBlockedReason, SteamClientState};
use super::process::{
    collect_process_ids, collect_titled_windows, process_image_path, window_title,
};

use winapi::shared::windef::HWND;

static STEAM_MAIN_HWND: AtomicIsize = AtomicIsize::new(0);

pub(super) fn launch_steam_game(
    steam_app_id: u32,
    steam_paths: &[PathBuf],
) -> Result<(), Option<LaunchBlockedReason>> {
    match steam_client_state() {
        SteamClientState::Ready => {
            let launched = if let Some(steam_exe) = resolve_steam_exe_path(steam_paths) {
                Command::new(steam_exe)
                    .args(["-applaunch", &steam_app_id.to_string()])
                    .spawn()
                    .is_ok()
            } else {
                let url = format!("steam://rungameid/{}", steam_app_id);
                Command::new("cmd")
                    .args(["/C", "start", "", &url])
                    .spawn()
                    .is_ok()
            };

            if launched {
                Ok(())
            } else {
                Err(None)
            }
        }
        SteamClientState::Loading => Err(Some(LaunchBlockedReason::SteamClientLoading)),
        SteamClientState::NotRunning => Err(Some(LaunchBlockedReason::SteamClientNotRunning)),
    }
}

pub(super) fn start_steam_client(steam_paths: &[PathBuf]) -> bool {
    if let Some(steam_exe) = resolve_steam_exe_path(steam_paths) {
        return Command::new(steam_exe).arg("-silent").spawn().is_ok();
    }

    false
}

pub(super) fn steam_client_state() -> SteamClientState {
    let started_at = Instant::now();
    let state = if has_steam_main_window() {
        SteamClientState::Ready
    } else if collect_process_ids().into_iter().any(is_steam_process) {
        SteamClientState::Loading
    } else {
        SteamClientState::NotRunning
    };

    append_steam_client_state_log(state, started_at.elapsed().as_millis());
    state
}

fn resolve_steam_exe_path(steam_paths: &[PathBuf]) -> Option<PathBuf> {
    steam_paths
        .iter()
        .map(|path| path.join("steam.exe"))
        .find(|path| path.exists())
}

fn is_steam_process_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("steam.exe"))
}

fn is_steam_process(pid: u32) -> bool {
    process_image_path(pid)
        .as_deref()
        .is_some_and(is_steam_process_path)
}

fn has_steam_main_window() -> bool {
    let cached_hwnd = STEAM_MAIN_HWND.load(Ordering::Acquire);
    if cached_hwnd != 0 && window_title(cached_hwnd as HWND) == "Steam" {
        return true;
    }

    for (hwnd, _) in collect_titled_windows() {
        if window_title(hwnd) == "Steam" {
            STEAM_MAIN_HWND.store(hwnd as isize, Ordering::Release);
            return true;
        }
    }

    STEAM_MAIN_HWND.store(0, Ordering::Release);
    false
}
