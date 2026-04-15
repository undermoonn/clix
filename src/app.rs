use eframe::egui;
use gilrs::{EventType, Gilrs};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use winapi::um::xinput::{XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_LEFT, XINPUT_GAMEPAD_DPAD_RIGHT, XINPUT_GAMEPAD_DPAD_UP};
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
    SetForegroundWindow, ShowWindow, SW_RESTORE,
};

use crate::cover;
use crate::input::*;
use crate::steam::{self, Game};
use crate::ui;

const QUIT_HOLD_TO_EXIT_SECONDS: f32 = 0.8;

struct LaunchState {
    game_index: usize,
    game_name: String,
    started_at: Instant,
    #[cfg(target_os = "windows")]
    baseline_pids: HashSet<u32>,
    #[cfg(target_os = "windows")]
    baseline_hwnds: HashSet<isize>,
    #[cfg(target_os = "windows")]
    target_path: Option<std::path::PathBuf>,
    #[cfg(target_os = "windows")]
    last_keep_foreground_at: Instant,
}

struct PendingBackgroundAssets {
    app_id: u32,
    cover_bytes: Option<Vec<u8>>,
    logo_bytes: Option<Vec<u8>>,
}

#[cfg(target_os = "windows")]
fn normalize_windows_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_ascii_lowercase()
}

#[cfg(target_os = "windows")]
fn process_image_path(pid: u32) -> Option<std::path::PathBuf> {
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

        let s = String::from_utf16_lossy(&buf[..len as usize]);
        Some(std::path::PathBuf::from(s))
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
fn bring_current_app_to_foreground() {
    let current_pid = unsafe { GetCurrentProcessId() } as u32;
    for (hwnd, pid) in collect_visible_windows().into_iter() {
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

    for (hwnd, pid) in collect_visible_windows().into_iter() {
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

pub struct LauncherApp {
    games: Vec<Game>,
    selected: usize,
    gilrs: Option<Gilrs>,
    #[cfg(target_os = "windows")]
    xinput: Option<XInput>,
    mapping: Mapping,
    remap_target: Option<String>,
    nav_held: HashMap<&'static str, NavState>,
    had_focus: bool,
    lost_focus_at: Option<Instant>,
    focus_cooldown_until: Option<Instant>,
    steam_paths: Vec<std::path::PathBuf>,
    cover: Option<(u32, egui::TextureHandle)>,
    cover_prev: Option<(u32, egui::TextureHandle)>,
    logo: Option<(u32, egui::TextureHandle)>,
    logo_prev: Option<(u32, egui::TextureHandle)>,
    cover_fade: f32,
    cover_transition_ready: bool,
    cover_nav_dir: f32,
    selected_assets_loaded_for: Option<usize>,
    cover_pending: Arc<Mutex<Option<PendingBackgroundAssets>>>,
    selected_assets_debounce_until: Option<Instant>,
    selected_assets_debounce_for: Option<usize>,
    select_anim: f32,
    select_anim_target: Option<usize>,
    scroll_offset: f32,
    hint_icons: Option<ui::HintIcons>,
    nav_input_dir: i8,
    game_icons: HashMap<u32, egui::TextureHandle>,
    icons_loaded: bool,
    launch_state: Option<LaunchState>,
    achievement_cache: HashMap<u32, steam::AchievementSummary>,
    achievement_pending: Arc<Mutex<Vec<(u32, Option<steam::AchievementSummary>)>>>,
    achievement_loading: HashSet<u32>,
    achievement_no_data: HashSet<u32>,
    achievement_icon_cache: HashMap<String, egui::TextureHandle>,
    achievement_icon_pending: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
    achievement_icon_loading: HashSet<String>,
    achievement_icon_reveal: HashMap<String, f32>,
    achievement_text_reveal: HashMap<u32, f32>,
    achievement_checked_for: Option<u32>,
    show_achievement_panel: bool,
    achievement_panel_anim: f32,
    achievement_selected: usize,
    achievement_select_anim: f32,
    achievement_select_anim_target: Option<usize>,
    achievement_scroll_offset: f32,
    quit_hold_started_at: Option<Instant>,
    quit_hold_progress: f32,
    quit_hold_consumed: bool,
    suppress_quit_hold_until_release: bool,
}

impl LauncherApp {
    pub fn new() -> Self {
        let steam_paths = steam::find_steam_paths();
        let games = steam::scan_games_with_paths(&steam_paths);
        let gilrs = Gilrs::new().ok();
        LauncherApp {
            games,
            selected: 0,
            gilrs,
            #[cfg(target_os = "windows")]
            xinput: XInput::new().ok(),
            mapping: Mapping::load().unwrap_or_default(),
            remap_target: None,
            nav_held: HashMap::new(),
            had_focus: true,
            lost_focus_at: None,
            focus_cooldown_until: None,
            steam_paths,
            cover: None,
            cover_prev: None,
            logo: None,
            logo_prev: None,
            cover_fade: 1.0,
            cover_transition_ready: true,
            cover_nav_dir: 0.0,
            selected_assets_loaded_for: None,
            cover_pending: Arc::new(Mutex::new(None)),
            selected_assets_debounce_until: None,
            selected_assets_debounce_for: None,
            select_anim: 0.0,
            select_anim_target: None,
            scroll_offset: 0.0,
            hint_icons: None,
            nav_input_dir: 0,
            game_icons: HashMap::new(),
            icons_loaded: false,
            launch_state: None,
            achievement_cache: HashMap::new(),
            achievement_pending: Arc::new(Mutex::new(Vec::new())),
            achievement_loading: HashSet::new(),
            achievement_no_data: HashSet::new(),
            achievement_icon_cache: HashMap::new(),
            achievement_icon_pending: Arc::new(Mutex::new(Vec::new())),
            achievement_icon_loading: HashSet::new(),
            achievement_icon_reveal: HashMap::new(),
            achievement_text_reveal: HashMap::new(),
            achievement_checked_for: None,
            show_achievement_panel: false,
            achievement_panel_anim: 0.0,
            achievement_selected: 0,
            achievement_select_anim: 0.0,
            achievement_select_anim_target: None,
            achievement_scroll_offset: 0.0,
            quit_hold_started_at: None,
            quit_hold_progress: 0.0,
            quit_hold_consumed: false,
            suppress_quit_hold_until_release: false,
        }
    }

    fn refresh_achievement_for_selected(&mut self, ctx: &egui::Context) {
        let Some(app_id) = self.games.get(self.selected).and_then(|g| g.app_id) else {
            self.achievement_checked_for = None;
            return;
        };

        if self.achievement_checked_for == Some(app_id) {
            return;
        }

        self.achievement_checked_for = Some(app_id);

        if let Some(summary) = steam::load_cached_achievement_summary(app_id) {
            self.achievement_no_data.remove(&app_id);
            self.achievement_cache.insert(app_id, summary);
            self.achievement_text_reveal.insert(app_id, 1.0);
        }

        self.achievement_no_data.remove(&app_id);

        if self.achievement_loading.contains(&app_id) {
            return;
        }

        self.achievement_loading.insert(app_id);
        let pending = Arc::clone(&self.achievement_pending);
        let paths = self.steam_paths.clone();
        let ctx_clone = ctx.clone();

        std::thread::spawn(move || {
            let data = steam::load_achievement_summary(app_id, &paths);
            if let Ok(mut lock) = pending.lock() {
                lock.push((app_id, data));
            }
            ctx_clone.request_repaint();
        });
    }

    fn refresh_cover_for_selected(&mut self, ctx: &egui::Context) {
        self.cover_prev = self.cover.take();
        self.logo_prev = self.logo.take();
        self.cover_fade = 0.0;
        self.cover_transition_ready = false;

        let Some(app_id) = self.games.get(self.selected).and_then(|g| g.app_id) else {
            self.cover_transition_ready = true;
            return;
        };

        let pending = Arc::clone(&self.cover_pending);
        let paths = self.steam_paths.clone();
        let ctx_clone = ctx.clone();
        if let Ok(mut lock) = pending.lock() {
            *lock = None;
        }
        std::thread::spawn(move || {
            let cover_bytes = cover::load_cover_bytes(&paths, app_id);
            let logo_bytes = cover::load_logo_bytes(&paths, app_id);
            if let Ok(mut lock) = pending.lock() {
                *lock = Some(PendingBackgroundAssets {
                    app_id,
                    cover_bytes,
                    logo_bytes,
                });
            }
            ctx_clone.request_repaint();
        });
    }

    fn refresh_assets_for_selected(&mut self, ctx: &egui::Context) {
        self.refresh_achievement_for_selected(ctx);
        self.refresh_cover_for_selected(ctx);
    }

    fn drain_achievement_results(&mut self) {
        let Ok(mut lock) = self.achievement_pending.lock() else {
            return;
        };

        for (app_id, summary) in lock.drain(..) {
            self.achievement_loading.remove(&app_id);
            match summary {
                Some(s) => {
                    let had_summary = self.achievement_cache.contains_key(&app_id);
                    steam::store_cached_achievement_summary(app_id, &s);
                    self.achievement_no_data.remove(&app_id);
                    self.achievement_cache.insert(app_id, s);
                    if !had_summary {
                        self.achievement_text_reveal.insert(app_id, 0.0);
                    } else {
                        self.achievement_text_reveal.entry(app_id).or_insert(1.0);
                    }
                }
                None => {
                    if !self.achievement_cache.contains_key(&app_id) {
                        self.achievement_no_data.insert(app_id);
                    }
                }
            }
        }
    }

    fn can_open_achievement_panel_for_selected(&self) -> bool {
        self.games
            .get(self.selected)
            .and_then(|g| g.app_id)
            .and_then(|id| self.achievement_cache.get(&id))
            .map(|summary| !summary.items.is_empty())
            .unwrap_or(false)
    }

    fn ensure_achievement_icons_for_selected(&mut self, ctx: &egui::Context) {
        let Some(app_id) = self.games.get(self.selected).and_then(|g| g.app_id) else {
            return;
        };
        let Some(summary) = self.achievement_cache.get(&app_id) else {
            return;
        };

        for item in &summary.items {
            let url_opt = match item.unlocked {
                Some(true) => item.icon_url.as_ref().or(item.icon_gray_url.as_ref()),
                _ => item.icon_gray_url.as_ref().or(item.icon_url.as_ref()),
            };
            let Some(url) = url_opt else {
                continue;
            };

            if self.achievement_icon_cache.contains_key(url)
                || self.achievement_icon_loading.contains(url)
            {
                continue;
            }

            if let Some(bytes) = cover::load_cached_achievement_icon_bytes(url) {
                if let Ok(mut lock) = self.achievement_icon_pending.lock() {
                    lock.push((url.clone(), bytes));
                }
                ctx.request_repaint();
                continue;
            }

            self.achievement_icon_loading.insert(url.clone());
            let pending = Arc::clone(&self.achievement_icon_pending);
            let ctx_clone = ctx.clone();
            let url_clone = url.clone();
            std::thread::spawn(move || {
                if let Some(bytes) = cover::load_achievement_icon_bytes(&url_clone) {
                    if let Ok(mut lock) = pending.lock() {
                        lock.push((url_clone, bytes));
                    }
                    ctx_clone.request_repaint();
                }
            });
        }
    }

    fn drain_achievement_icon_results(&mut self, ctx: &egui::Context) {
        let Ok(mut lock) = self.achievement_icon_pending.lock() else {
            return;
        };

        let mut hasher_seed = self.achievement_icon_cache.len();
        for (url, bytes) in lock.drain(..) {
            self.achievement_icon_loading.remove(&url);
            if self.achievement_icon_cache.contains_key(&url) {
                continue;
            }

            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            hasher_seed.hash(&mut hasher);
            url.hash(&mut hasher);
            hasher_seed += 1;
            let label = format!("ach_icon_{:x}", hasher.finish());

            if let Some(tex) = cover::bytes_to_texture(ctx, &bytes, label) {
                self.achievement_icon_reveal.insert(url.clone(), 0.0);
                self.achievement_icon_cache.insert(url, tex);
            }
        }
    }

    fn launch_selected(&mut self) {
        if let Some(g) = self.games.get(self.selected) {
            let target_path = g.path.clone();
            let game_name = g.name.clone();
            #[cfg(target_os = "windows")]
            let baseline_pids = collect_process_ids();
            #[cfg(target_os = "windows")]
            let baseline_hwnds: HashSet<isize> = collect_visible_windows()
                .into_iter()
                .map(|(hwnd, _)| hwnd as isize)
                .collect();

            let launched = if let Some(app_id) = g.app_id {
                #[cfg(target_os = "windows")]
                {
                    let steam_exe = self
                        .steam_paths
                        .iter()
                        .map(|p| p.join("steam.exe"))
                        .find(|p| p.exists());

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
                    let url = format!("steam://rungameid/{}", app_id);
                    Command::new("sh")
                        .args(["-c", &format!("xdg-open '{}'", url)])
                        .spawn()
                        .is_ok()
                }
            } else {
                Command::new(&g.path).spawn().is_ok()
            };

            if !launched {
                return;
            }

            self.launch_state = Some(LaunchState {
                game_index: self.selected,
                game_name,
                started_at: Instant::now(),
                #[cfg(target_os = "windows")]
                baseline_pids,
                #[cfg(target_os = "windows")]
                baseline_hwnds,
                #[cfg(target_os = "windows")]
                target_path: if target_path.exists() { Some(target_path) } else { None },
                #[cfg(target_os = "windows")]
                last_keep_foreground_at: Instant::now() - Duration::from_millis(400),
            });
        }
    }

    fn tick_launch_progress(&mut self, ctx: &egui::Context) {
        let mut should_clear = false;

        if let Some(state) = self.launch_state.as_mut() {
            ctx.request_repaint();

            #[cfg(target_os = "windows")]
            {
                let now = Instant::now();

                if now.duration_since(state.last_keep_foreground_at) >= Duration::from_millis(250)
                {
                    bring_current_app_to_foreground();
                    state.last_keep_foreground_at = now;
                }

                if let Some(hwnd) = detect_launched_window(
                    &state.baseline_pids,
                    &state.baseline_hwnds,
                    state.target_path.as_deref(),
                    &state.game_name,
                )
                {
                    bring_window_to_foreground(hwnd);
                    should_clear = true;
                }

                if now.duration_since(state.started_at) >= Duration::from_secs(25) {
                    should_clear = true;
                }
            }

            #[cfg(not(target_os = "windows"))]
            {
                if Instant::now().duration_since(state.started_at) >= Duration::from_secs(5) {
                    should_clear = true;
                }
            }
        }

        if should_clear {
            self.launch_state = None;
        }
    }

    fn refresh_games_after_resume(&mut self) {
        let selected_key = self
            .games
            .get(self.selected)
            .map(|g| (g.app_id, g.name.clone()));

        self.games = steam::scan_games_with_paths(&self.steam_paths);

        if self.games.is_empty() {
            self.selected = 0;
        } else if let Some((app_id, name)) = selected_key {
            let mut new_selected = None;

            if let Some(id) = app_id {
                new_selected = self.games.iter().position(|g| g.app_id == Some(id));
            }

            if new_selected.is_none() {
                new_selected = self.games.iter().position(|g| g.name == name);
            }

            self.selected = new_selected.unwrap_or_else(|| self.selected.min(self.games.len() - 1));
        } else {
            self.selected = self.selected.min(self.games.len() - 1);
        }

        self.scroll_offset = self.selected as f32;
        self.select_anim_target = Some(self.selected);
        self.achievement_checked_for = None;
        self.select_anim = 1.0;

        // Force assets to refresh for the selected game after resume.
        self.selected_assets_loaded_for = None;
        self.selected_assets_debounce_for = None;
        self.selected_assets_debounce_until = None;
        self.icons_loaded = false;
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::None);

        let has_focus = ctx.input(|i| i.focused);

        if has_focus {
            ctx.request_repaint();
            if !self.had_focus {
                let should_refresh = self
                    .lost_focus_at
                    .take()
                    .map(|lost_at| Instant::now().duration_since(lost_at) >= Duration::from_secs(5))
                    .unwrap_or(false);

                if should_refresh {
                    self.refresh_games_after_resume();
                }
                self.focus_cooldown_until = Some(Instant::now());
                self.nav_held.clear();
            }
        } else {
            if self.had_focus {
                self.lost_focus_at = Some(Instant::now());
            }
            self.nav_held.clear();
        }
        self.had_focus = has_focus;

        let in_cooldown = match self.focus_cooldown_until {
            Some(t) => {
                if Instant::now().duration_since(t).as_millis() < FOCUS_COOLDOWN_MS {
                    true
                } else {
                    self.focus_cooldown_until = None;
                    false
                }
            }
            None => false,
        };
        let process_input = has_focus && !in_cooldown && self.launch_state.is_none();

        let mut raw_held: HashSet<&'static str> = HashSet::new();
        let mut actions: Vec<ControllerAction> = Vec::new();

        if process_input {
            // XInput polling
            #[cfg(target_os = "windows")]
            {
                if let Some(xi) = &self.xinput {
                    let states = xi.get_states();

                    if let Some(target) = &self.remap_target {
                        for (buttons, ly, _) in states.iter() {
                            if *buttons != 0 {
                                self.mapping
                                    .map
                                    .insert(target.clone(), InputToken::XButton(*buttons));
                                self.mapping.save();
                                self.remap_target = None;
                                break;
                            }
                            if *ly < -16000 {
                                self.mapping
                                    .map
                                    .insert(target.clone(), InputToken::XAxis(-1));
                                self.mapping.save();
                                self.remap_target = None;
                                break;
                            }
                            if *ly > 16000 {
                                self.mapping
                                    .map
                                    .insert(target.clone(), InputToken::XAxis(1));
                                self.mapping.save();
                                self.remap_target = None;
                                break;
                            }
                        }
                    } else {
                        for (buttons, ly, lx) in states.iter() {
                            if (buttons & XINPUT_GAMEPAD_DPAD_UP) != 0 {
                                raw_held.insert("up");
                            }
                            if (buttons & XINPUT_GAMEPAD_DPAD_DOWN) != 0 {
                                raw_held.insert("down");
                            }
                            if (buttons & XINPUT_GAMEPAD_DPAD_LEFT) != 0 {
                                raw_held.insert("left");
                            }
                            if (buttons & XINPUT_GAMEPAD_DPAD_RIGHT) != 0 {
                                raw_held.insert("right");
                            }
                            if (buttons & XINPUT_GAMEPAD_A) != 0 {
                                raw_held.insert("launch");
                            }
                            if (buttons & XINPUT_GAMEPAD_B) != 0 {
                                raw_held.insert("quit");
                            }
                            if *ly > 16000 {
                                raw_held.insert("up");
                            } else if *ly < -16000 {
                                raw_held.insert("down");
                            }
                            if *lx > 16000 {
                                raw_held.insert("right");
                            } else if *lx < -16000 {
                                raw_held.insert("left");
                            }
                        }

                        for (k, v) in self.mapping.map.iter() {
                            match v {
                                InputToken::XButton(mask) => {
                                    for (buttons, _, _) in states.iter() {
                                        if (buttons & mask) != 0 {
                                            match k.as_str() {
                                                "up" => {
                                                    raw_held.insert("up");
                                                }
                                                "down" => {
                                                    raw_held.insert("down");
                                                }
                                                "left" => {
                                                    raw_held.insert("left");
                                                }
                                                "launch" => {
                                                    raw_held.insert("launch");
                                                }
                                                "right" => {
                                                    raw_held.insert("right");
                                                }
                                                "quit" => {
                                                    raw_held.insert("quit");
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                InputToken::XAxis(dir) => {
                                    for (_, ly, _) in states.iter() {
                                        if *dir > 0 && *ly > 16000 {
                                            if k == "up" {
                                                raw_held.insert("up");
                                            }
                                            if k == "down" {
                                                raw_held.insert("down");
                                            }
                                        }
                                        if *dir < 0 && *ly < -16000 {
                                            if k == "up" {
                                                raw_held.insert("up");
                                            }
                                            if k == "down" {
                                                raw_held.insert("down");
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            // Gilrs fallback
            if raw_held.is_empty() {
                if let Some(gilrs) = &mut self.gilrs {
                    while let Some(ev) = gilrs.next_event() {
                        if let Some(target) = &self.remap_target {
                            match ev.event {
                                EventType::ButtonPressed(btn, _) => {
                                    let id = btn as u8;
                                    self.mapping
                                        .map
                                        .insert(target.clone(), InputToken::GilrsButton(id));
                                    self.mapping.save();
                                    self.remap_target = None;
                                }
                                EventType::AxisChanged(axis, value, _) => {
                                    let name = format!("{:?}", axis);
                                    let dir = if value < 0.0 { -1 } else { 1 };
                                    self.mapping
                                        .map
                                        .insert(target.clone(), InputToken::GilrsAxis(name, dir));
                                    self.mapping.save();
                                    self.remap_target = None;
                                }
                                _ => {}
                            }
                            continue;
                        }

                        match ev.event {
                            EventType::ButtonPressed(btn, _) => {
                                let bid = btn as u8;
                                for (k, v) in self.mapping.map.iter() {
                                    if let InputToken::GilrsButton(b) = v {
                                        if *b == bid {
                                            match k.as_str() {
                                                "up" => {
                                                    raw_held.insert("up");
                                                }
                                                "down" => {
                                                    raw_held.insert("down");
                                                }
                                                "left" => {
                                                    raw_held.insert("left");
                                                }
                                                "launch" => {
                                                    raw_held.insert("launch");
                                                }
                                                "right" => {
                                                    raw_held.insert("right");
                                                }
                                                "quit" => {
                                                    raw_held.insert("quit");
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                            EventType::AxisChanged(axis, value, _) => {
                                let name = format!("{:?}", axis);
                                for (k, v) in self.mapping.map.iter() {
                                    if let InputToken::GilrsAxis(a, dir) = v {
                                        if a == &name {
                                            if *dir < 0 && value < -0.7 {
                                                if k == "up" {
                                                    raw_held.insert("up");
                                                }
                                                if k == "down" {
                                                    raw_held.insert("down");
                                                }
                                            }
                                            if *dir > 0 && value > 0.7 {
                                                if k == "up" {
                                                    raw_held.insert("up");
                                                }
                                                if k == "down" {
                                                    raw_held.insert("down");
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            if ctx.input(|i| i.key_down(egui::Key::ArrowUp)) {
                raw_held.insert("up");
            }
            if ctx.input(|i| i.key_down(egui::Key::ArrowDown)) {
                raw_held.insert("down");
            }
            if ctx.input(|i| i.key_down(egui::Key::ArrowLeft)) {
                raw_held.insert("left");
            }
            if ctx.input(|i| i.key_down(egui::Key::ArrowRight)) {
                raw_held.insert("right");
            }
            if ctx.input(|i| i.key_down(egui::Key::Enter)) {
                raw_held.insert("launch");
            }
            if ctx.input(|i| i.key_down(egui::Key::Escape)) {
                raw_held.insert("quit");
            }
        } // end if process_input

        // Track nav direction for hint bar icon
        self.nav_input_dir = if raw_held.contains("up") {
            -1
        } else if raw_held.contains("down") {
            1
        } else {
            0
        };

        // Debounce / repeat-delay
        let now = Instant::now();
        self.nav_held.retain(|k, _| raw_held.contains(k));

        let action_names: &[&str] = if self.show_achievement_panel {
            &["up", "down", "left", "right", "launch", "quit"]
        } else {
            &["up", "down", "left", "right", "launch"]
        };

        for action_name in action_names {
            if raw_held.contains(action_name) {
                let should_fire = if let Some(state) = self.nav_held.get_mut(action_name) {
                    if !state.past_initial {
                        if now.duration_since(state.since).as_millis() >= NAV_INITIAL_DELAY_MS {
                            state.past_initial = true;
                            state.last_fire = now;
                            true
                        } else {
                            false
                        }
                    } else {
                        let held_ms = now.duration_since(state.since).as_millis();
                        let repeat_interval_ms = if *action_name == "up" || *action_name == "down"
                        {
                            if held_ms >= NAV_REPEAT_ACCEL_AFTER_MS {
                                NAV_REPEAT_INTERVAL_FAST_MS
                            } else {
                                NAV_REPEAT_INTERVAL_MS
                            }
                        } else {
                            NAV_REPEAT_INTERVAL_MS
                        };

                        if now.duration_since(state.last_fire).as_millis() >= repeat_interval_ms {
                            state.last_fire = now;
                            true
                        } else {
                            false
                        }
                    }
                } else {
                    self.nav_held.insert(
                        action_name,
                        NavState {
                            since: now,
                            last_fire: now,
                            past_initial: false,
                        },
                    );
                    true
                };

                if should_fire {
                    match *action_name {
                        "up" => actions.push(ControllerAction::Up),
                        "down" => actions.push(ControllerAction::Down),
                        "left" => actions.push(ControllerAction::Left),
                        "right" => actions.push(ControllerAction::Right),
                        "launch" => actions.push(ControllerAction::Launch),
                        "quit" => actions.push(ControllerAction::Quit),
                        _ => {}
                    }
                }
            }
        }

        if self.suppress_quit_hold_until_release {
            if raw_held.contains("quit") {
                self.quit_hold_started_at = None;
                self.quit_hold_progress = 0.0;
                self.quit_hold_consumed = false;
            } else {
                self.suppress_quit_hold_until_release = false;
            }
        } else if process_input && !self.show_achievement_panel && raw_held.contains("quit") {
            if self.quit_hold_consumed {
                self.quit_hold_progress = 1.0;
            } else {
                let started_at = self.quit_hold_started_at.get_or_insert(now);
                let held_seconds = now.duration_since(*started_at).as_secs_f32();
                self.quit_hold_progress =
                    (held_seconds / QUIT_HOLD_TO_EXIT_SECONDS).clamp(0.0, 1.0);

                if self.quit_hold_progress >= 1.0 {
                    self.quit_hold_consumed = true;
                    actions.push(ControllerAction::Quit);
                } else {
                    ctx.request_repaint();
                }
            }
        } else {
            self.quit_hold_started_at = None;
            self.quit_hold_progress = 0.0;
            self.quit_hold_consumed = false;
        }

        // Apply actions
        for act in &actions {
            if self.show_achievement_panel {
                match act {
                    ControllerAction::Up => {
                        if self.achievement_selected > 0 {
                            self.achievement_selected -= 1;
                        }
                    }
                    ControllerAction::Down => {
                        let max_len = self
                            .games
                            .get(self.selected)
                            .and_then(|g| g.app_id)
                            .and_then(|id| self.achievement_cache.get(&id))
                            .map(|s| s.items.len())
                            .unwrap_or(0);
                        if self.achievement_selected + 1 < max_len {
                            self.achievement_selected += 1;
                        }
                    }
                    ControllerAction::Quit => {
                        self.show_achievement_panel = false;
                        self.achievement_selected = 0;
                        self.achievement_select_anim = 0.0;
                        self.achievement_select_anim_target = None;
                        self.achievement_scroll_offset = 0.0;
                        self.suppress_quit_hold_until_release = true;
                    }
                    _ => {}
                }
                continue;
            }

            match act {
                ControllerAction::Left => {
                    if self.selected > 0 {
                        self.selected -= 1;
                        self.cover_nav_dir = -1.0;
                        self.achievement_selected = 0;
                        self.achievement_select_anim = 0.0;
                        self.achievement_select_anim_target = None;
                        self.achievement_scroll_offset = 0.0;
                    }
                }
                ControllerAction::Right => {
                    if self.selected + 1 < self.games.len() {
                        self.selected += 1;
                        self.cover_nav_dir = 1.0;
                        self.achievement_selected = 0;
                        self.achievement_select_anim = 0.0;
                        self.achievement_select_anim_target = None;
                        self.achievement_scroll_offset = 0.0;
                    }
                }
                ControllerAction::Down => {
                    if self.can_open_achievement_panel_for_selected() {
                        self.show_achievement_panel = true;
                        self.achievement_selected = 0;
                        self.achievement_select_anim = 0.0;
                        self.achievement_select_anim_target = None;
                        self.achievement_scroll_offset = 0.0;
                    }
                }
                ControllerAction::Up => {}
                ControllerAction::Launch => {
                    self.launch_selected();
                }
                ControllerAction::Quit => {
                    frame.close();
                }
            }
        }

        self.tick_launch_progress(ctx);
        self.drain_achievement_results();
        self.drain_achievement_icon_results(ctx);

        // Cover loading (async with 300ms debounce)
        if self.selected_assets_loaded_for != Some(self.selected) {
            // Only reset timer if selection actually changed since last debounce start
            if self.selected_assets_debounce_for != Some(self.selected) {
                self.selected_assets_debounce_for = Some(self.selected);
                self.selected_assets_debounce_until = Some(Instant::now() + std::time::Duration::from_millis(300));
            }
        }
        if let Some(deadline) = self.selected_assets_debounce_until {
            if Instant::now() >= deadline {
                self.selected_assets_debounce_until = None;
                if self.selected_assets_loaded_for != Some(self.selected) {
                    self.selected_assets_loaded_for = Some(self.selected);
                    self.refresh_assets_for_selected(ctx);
                }
            } else {
                ctx.request_repaint();
            }
        }

        let result = self
            .cover_pending
            .lock()
            .ok()
            .and_then(|mut lock| lock.take());
        if let Some(assets) = result {
            if Some(assets.app_id) == self.games.get(self.selected).and_then(|g| g.app_id) {
                let mut loaded_any = false;

                self.cover = assets.cover_bytes.and_then(|bytes| {
                    cover::bytes_to_texture(ctx, &bytes, format!("cover_{}", assets.app_id))
                        .map(|tex| {
                            loaded_any = true;
                            (assets.app_id, tex)
                        })
                });

                self.logo = assets.logo_bytes.and_then(|bytes| {
                    cover::bytes_to_texture(ctx, &bytes, format!("logo_{}", assets.app_id))
                        .map(|tex| {
                            loaded_any = true;
                            (assets.app_id, tex)
                        })
                });

                if loaded_any || self.cover_prev.is_some() || self.logo_prev.is_some() {
                    self.cover_fade = 0.0;
                }
                self.cover_transition_ready = true;
            }
        }

        // Advance crossfade
        if self.cover_transition_ready && self.cover_fade < 1.0 {
            let dt = ctx.input(|i| i.predicted_dt);
            self.cover_fade = (self.cover_fade + dt * 3.0).min(1.0);
            ctx.request_repaint();
        }

        // Selection animation
        if self.select_anim_target != Some(self.selected) {
            self.select_anim_target = Some(self.selected);
            self.select_anim = 0.0;
        }
        let dt = ctx.input(|i| i.predicted_dt);
        self.select_anim = 1.0 - (1.0 - self.select_anim) * (-10.0 * dt).exp();
        if self.select_anim < 0.999 {
            ctx.request_repaint();
        }

        let panel_target = if self.show_achievement_panel { 1.0 } else { 0.0 };
        let panel_diff = panel_target - self.achievement_panel_anim;
        if panel_diff.abs() > 0.001 {
            self.achievement_panel_anim += panel_diff * (1.0 - (-12.0 * dt).exp());
            ctx.request_repaint();
        } else {
            self.achievement_panel_anim = panel_target;
        }

        self.achievement_icon_reveal.retain(|_, progress| {
            if *progress >= 0.999 {
                return false;
            }

            const ACHIEVEMENT_ICON_FADE_IN_SECONDS: f32 = 0.3;
            *progress = (*progress + dt / ACHIEVEMENT_ICON_FADE_IN_SECONDS).min(1.0);
            if *progress < 0.999 {
                ctx.request_repaint();
                true
            } else {
                false
            }
        });

        for progress in self.achievement_text_reveal.values_mut() {
            if *progress < 0.999 {
                const ACHIEVEMENT_TEXT_FADE_IN_SECONDS: f32 = 0.35;
                *progress = (*progress + dt / ACHIEVEMENT_TEXT_FADE_IN_SECONDS).min(1.0);
                if *progress < 0.999 {
                    ctx.request_repaint();
                }
            }
        }

        // Smooth scroll animation (exponential decay towards target)
        let scroll_target = self.selected as f32;
        let scroll_diff = scroll_target - self.scroll_offset;
        if scroll_diff.abs() > 0.001 {
            self.scroll_offset += scroll_diff * (1.0 - (-14.0 * dt).exp());
            ctx.request_repaint();
        } else {
            self.scroll_offset = scroll_target;
        }

        // Achievement selection animation
        if self.achievement_select_anim_target != Some(self.achievement_selected) {
            self.achievement_select_anim_target = Some(self.achievement_selected);
            self.achievement_select_anim = 0.0;
        }
        self.achievement_select_anim =
            1.0 - (1.0 - self.achievement_select_anim) * (-10.0 * dt).exp();
        if self.achievement_select_anim < 0.999 {
            ctx.request_repaint();
        }

        // Achievement smooth scroll
        let ach_target = self.achievement_selected.saturating_sub(2) as f32;
        let ach_diff = ach_target - self.achievement_scroll_offset;
        if ach_diff.abs() > 0.001 {
            self.achievement_scroll_offset += ach_diff * (1.0 - (-14.0 * dt).exp());
            ctx.request_repaint();
        } else {
            self.achievement_scroll_offset = ach_target;
        }

        // Load hint icons lazily
        if self.hint_icons.is_none() {
            self.hint_icons = ui::load_hint_icons(ctx);
        }

        // Load game icons lazily
        if !self.icons_loaded {
            self.icons_loaded = true;
            for game in &self.games {
                if let Some(app_id) = game.app_id {
                    if let Some(bytes) = cover::load_game_icon_bytes(&self.steam_paths, game) {
                        if let Some(tex) = cover::bytes_to_texture(
                            ctx,
                            &bytes,
                            format!("icon_{}", app_id),
                        ) {
                            self.game_icons.insert(app_id, tex);
                        }
                    }
                }
            }
        }

        let selected_app_id = self.games.get(self.selected).and_then(|g| g.app_id);

        self.ensure_achievement_icons_for_selected(ctx);

        let selected_achievement_summary =
            selected_app_id.and_then(|id| self.achievement_cache.get(&id));
        let selected_achievement_reveal = selected_app_id
            .and_then(|id| self.achievement_text_reveal.get(&id).copied())
            .unwrap_or(1.0);
        let can_open_achievement_panel = selected_achievement_summary
            .map(|summary| !summary.items.is_empty())
            .unwrap_or(false);
        // Draw UI
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                ui::draw_background(
                    ctx,
                    &self.cover,
                    &self.cover_prev,
                    &self.logo,
                    &self.logo_prev,
                    self.cover_fade,
                    self.cover_nav_dir,
                );

                if self.show_achievement_panel {
                    if let Some(game) = self.games.get(self.selected) {
                        let game_icon = game
                            .app_id
                            .and_then(|app_id| self.game_icons.get(&app_id));
                        ui::draw_achievement_page(
                            ui,
                            game,
                            selected_achievement_summary,
                            selected_achievement_reveal,
                            self.achievement_selected,
                            self.achievement_select_anim,
                            self.achievement_panel_anim,
                            self.selected,
                            self.select_anim,
                            self.scroll_offset,
                            self.achievement_scroll_offset,
                            game_icon,
                            &self.achievement_icon_cache,
                            &self.achievement_icon_reveal,
                        );
                    }
                } else {
                    ui::draw_game_list(
                        ui,
                        &self.games,
                        self.selected,
                        self.select_anim,
                        self.achievement_panel_anim,
                        self.scroll_offset,
                        &self.game_icons,
                        self.launch_state.as_ref().map(|s| s.game_index),
                        self.show_achievement_panel,
                        selected_achievement_summary,
                        selected_achievement_reveal,
                    );
                }

                if let Some(icons) = &self.hint_icons {
                    ui::draw_hint_bar(
                        ui,
                        icons,
                        self.show_achievement_panel,
                        can_open_achievement_panel,
                        self.quit_hold_progress,
                    );
                }
            });
    }
}
