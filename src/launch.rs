use std::collections::HashSet;
#[cfg(target_os = "windows")]
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::Write;
#[cfg(target_os = "windows")]
use std::iter;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(not(target_os = "windows"))]
use std::time::Duration;
use std::time::Instant;

use crate::game::{Game, GameSource};

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
use self::windows::WindowTransition;
#[cfg(target_os = "windows")]
use winapi::um::shellapi::ShellExecuteW;
#[cfg(target_os = "windows")]
use winapi::um::winuser::SW_SHOWNORMAL;

pub struct LaunchState {
    pub game_index: usize,
    game_name: String,
    started_at: Instant,
    launch_steam_app_id: Option<u32>,
    awaiting_launch_release: bool,
    #[cfg(target_os = "windows")]
    baseline_pids: HashSet<u32>,
    #[cfg(target_os = "windows")]
    baseline_hwnds: HashSet<isize>,
    #[cfg(target_os = "windows")]
    target_path: Option<PathBuf>,
    #[cfg(target_os = "windows")]
    focus_pids: Option<HashSet<u32>>,
    #[cfg(target_os = "windows")]
    current_app_hwnd: Option<isize>,
    #[cfg(target_os = "windows")]
    transition: Option<WindowTransition>,
}

pub struct RunningGameState {
    pub game_index: usize,
    steam_app_id: Option<u32>,
    game_name: String,
    #[cfg(target_os = "windows")]
    target_path: Option<PathBuf>,
    #[cfg(target_os = "windows")]
    tracked_pids: HashSet<u32>,
}

pub enum LaunchTickResult {
    Pending,
    Ready(RunningGameState),
    TimedOut,
}

pub enum LaunchBlockedReason {
    SteamClientNotRunning,
    SteamClientLoading,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SteamClientState {
    Ready,
    Loading,
    NotRunning,
}

pub enum LaunchAttemptResult {
    Started(LaunchState),
    Blocked(LaunchBlockedReason),
    Failed,
}

pub fn begin_launch(
    game_index: usize,
    game: &Game,
    steam_paths: &[PathBuf],
) -> LaunchAttemptResult {
    let target_path = game.install_path.clone();
    let launch_target = game.launch_target.clone();
    let appx_id = game.appx_id.clone();
    let xbox_package_family_name = game.xbox_package_family_name.clone();
    let game_name = game.name.clone();
    let launch_steam_app_id = game.steam_app_id;
    let mut blocked_reason: Option<LaunchBlockedReason> = None;
    #[cfg(target_os = "windows")]
    let (baseline_pids, baseline_hwnds, current_app_hwnd) = windows::capture_launch_baseline();

    let launched = match game.source {
        GameSource::Steam => {
            if let Some(steam_app_id) = game.steam_app_id {
                #[cfg(target_os = "windows")]
                {
                    match windows::launch_steam_game(steam_app_id, steam_paths) {
                        Ok(()) => true,
                        Err(reason) => {
                            blocked_reason = reason;
                            false
                        }
                    }
                }

                #[cfg(not(target_os = "windows"))]
                {
                    let _ = steam_paths;
                    false
                }
            } else {
                launch_target
                    .as_ref()
                    .or(Some(&game.install_path))
                    .and_then(|path| spawn_direct_game(path, &game.install_path).ok())
                    .is_some()
            }
        }
        GameSource::Epic => launch_epic_game(&game.install_path),
        GameSource::Xbox => xbox_package_family_name
            .as_deref()
            .zip(appx_id.as_deref())
            .is_some_and(|(family_name, application_id)| {
                launch_xbox_app(family_name, application_id)
            }),
    };

    if !launched {
        return blocked_reason
            .map(LaunchAttemptResult::Blocked)
            .unwrap_or(LaunchAttemptResult::Failed);
    }

    LaunchAttemptResult::Started(LaunchState {
        game_index,
        game_name,
        started_at: Instant::now(),
        launch_steam_app_id,
        awaiting_launch_release: false,
        #[cfg(target_os = "windows")]
        baseline_pids,
        #[cfg(target_os = "windows")]
        baseline_hwnds,
        #[cfg(target_os = "windows")]
        target_path: if target_path.exists() { Some(target_path) } else { None },
        #[cfg(target_os = "windows")]
        focus_pids: None,
        #[cfg(target_os = "windows")]
        current_app_hwnd,
        #[cfg(target_os = "windows")]
        transition: None,
    })
}

fn spawn_direct_game(path: &Path, install_dir: &Path) -> std::io::Result<std::process::Child> {
    let mut command = Command::new(path);

    if install_dir.is_dir() {
        command.current_dir(install_dir);
    } else if let Some(parent) = path.parent().filter(|parent| parent.is_dir()) {
        command.current_dir(parent);
    }

    command.spawn()
}

#[cfg(target_os = "windows")]
fn launch_epic_game(install_dir: &Path) -> bool {
    if install_dir.is_dir() {
        let uri = format!(
            "com.epicgames.launcher://apps/{}?action=launch&silent=true",
            percent_encode_uri_path(install_dir)
        );

        return shell_open(&uri);
    }

    false
}

#[cfg(not(target_os = "windows"))]
fn launch_epic_game(_install_dir: &Path) -> bool {
    false
}

#[cfg(target_os = "windows")]
fn shell_open(target: &str) -> bool {
    unsafe {
        let operation = wide("open");
        let target = wide(target);
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

#[cfg(target_os = "windows")]
fn percent_encode_uri_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    let mut encoded = String::with_capacity(value.len());

    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char);
            }
            _ => {
                let _ = std::fmt::Write::write_fmt(&mut encoded, format_args!("%{:02X}", byte));
            }
        }
    }

    encoded
}

#[cfg(target_os = "windows")]
fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(iter::once(0))
        .collect()
}

#[cfg(target_os = "windows")]
fn launch_xbox_app(family_name: &str, application_id: &str) -> bool {
    let apps_folder_target = format!(r"shell:AppsFolder\{}!{}", family_name, application_id);
    Command::new("explorer.exe")
        .arg(apps_folder_target)
        .spawn()
        .is_ok()
}

#[cfg(not(target_os = "windows"))]
fn launch_xbox_app(_family_name: &str, _application_id: &str) -> bool {
    false
}

pub fn begin_focus_transition(game_index: usize, state: &RunningGameState) -> Option<LaunchState> {
    #[cfg(target_os = "windows")]
    {
        Some(windows::begin_focus_transition(game_index, state))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = game_index;
        let _ = state;
        None
    }
}

pub fn restart_launch_timeout(state: &mut LaunchState) {
    state.started_at = Instant::now();
}

pub fn tick_launch_progress(state: &mut LaunchState, launch_held: bool) -> LaunchTickResult {
    #[cfg(target_os = "windows")]
    {
        windows::tick_launch_progress(state, launch_held)
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
        windows::refresh_running_game(state)
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
        windows::close_running_game(state)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        false
    }
}

pub fn start_steam_client(steam_paths: &[PathBuf]) -> bool {
    #[cfg(target_os = "windows")]
    {
        windows::start_steam_client(steam_paths)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = steam_paths;
        false
    }
}

fn append_steam_client_state_log(state: SteamClientState, elapsed_ms: u128) {
    if !crate::config::load_steam_client_state_logging_enabled() {
        return;
    }

    let line = format!(
        "[steam_client_state] state={:?} elapsed_ms={}\n",
        state, elapsed_ms
    );

    let log_path = crate::assets::cache::logs_dir().join("steam_client_state.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = file.write_all(line.as_bytes());
    }
}

pub fn steam_client_state() -> SteamClientState {
    #[cfg(target_os = "windows")]
    {
        windows::steam_client_state()
    }

    #[cfg(not(target_os = "windows"))]
    {
        let started_at = Instant::now();
        let state = SteamClientState::NotRunning;
        append_steam_client_state_log(state, started_at.elapsed().as_millis());
        state
    }
}

pub fn set_current_app_hwnd(hwnd: isize) {
    #[cfg(target_os = "windows")]
    {
        windows::set_current_app_hwnd(hwnd);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = hwnd;
    }
}

#[cfg(target_os = "windows")]
pub fn current_app_window_is_background() -> bool {
    windows::current_app_window_is_background()
}

#[cfg(not(target_os = "windows"))]
pub fn current_app_window_is_background() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn focus_current_app_window() -> bool {
    windows::focus_current_app_window()
}

#[cfg(not(target_os = "windows"))]
pub fn focus_current_app_window() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn send_current_app_to_background() -> bool {
    windows::send_current_app_to_background()
}

#[cfg(not(target_os = "windows"))]
pub fn send_current_app_to_background() -> bool {
    false
}

pub fn focus_running_game(state: &RunningGameState) -> bool {
    #[cfg(target_os = "windows")]
    {
        windows::focus_running_game(state)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        false
    }
}

