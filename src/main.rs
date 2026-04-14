use eframe::{egui};
use gilrs::{EventType, Gilrs};
#[cfg(target_os = "windows")]
use libloading::Library;
#[cfg(target_os = "windows")]
use winapi::shared::minwindef::DWORD;
#[cfg(target_os = "windows")]
use winapi::um::xinput::{XINPUT_STATE, XINPUT_VIBRATION, XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_DPAD_UP, XINPUT_GAMEPAD_DPAD_DOWN};
use std::path::PathBuf;
use std::process::Command;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::Instant;

struct Game {
    name: String,
    path: PathBuf,
    app_id: Option<u32>,
    last_played: u64,
}

struct LauncherApp {
    games: Vec<Game>,
    selected: usize,
    gilrs: Option<Gilrs>,
    #[cfg(target_os = "windows")]
    xinput: Option<XInput>,
    mapping: Mapping,
    remap_target: Option<String>,
    /// Tracks which actions are currently held and when they last fired,
    /// enabling initial-delay + repeat-interval navigation.
    nav_held: HashMap<&'static str, NavState>,
    /// Whether the window had focus on the previous frame.
    had_focus: bool,
    /// Ignore all input until this instant (cooldown after regaining focus).
    focus_cooldown_until: Option<Instant>,
    /// Steam install paths (cached for cover lookup).
    steam_paths: Vec<PathBuf>,
    /// Currently loaded cover texture + the app_id it belongs to.
    cover: Option<(u32, egui::TextureHandle)>,
    /// Previous cover texture for crossfade transition.
    cover_prev: Option<(u32, egui::TextureHandle)>,
    /// Crossfade progress 0.0 (showing prev) to 1.0 (fully showing new).
    cover_fade: f32,
    /// Navigation direction for background slide: -1.0 = up, 1.0 = down.
    cover_nav_dir: f32,
    /// Tracks which selected index the cover was last loaded for.
    cover_loaded_for: Option<usize>,
    /// Receiver for async cover image bytes from background thread.
    cover_pending: Arc<Mutex<Option<(u32, Vec<u8>)>>>,
    /// Animated font scale for the selected item (0.0 to 1.0).
    select_anim: f32,
    /// The index that the animation is tracking.
    select_anim_target: Option<usize>,
}

struct NavState {
    /// When the button/axis was first detected as held
    since: Instant,
    /// When the action last fired
    last_fire: Instant,
    /// Whether the initial delay has passed
    past_initial: bool,
}

const NAV_INITIAL_DELAY_MS: u128 = 350;
const NAV_REPEAT_INTERVAL_MS: u128 = 120;
/// How long to ignore controller input after the window regains focus (ms).
const FOCUS_COOLDOWN_MS: u128 = 500;

impl LauncherApp {
    fn new() -> Self {
        let steam_paths = find_steam_paths();
        let games = scan_games_with_paths(&steam_paths);
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
            focus_cooldown_until: None,
            steam_paths,
            cover: None,
            cover_prev: None,
            cover_fade: 1.0,
            cover_nav_dir: 0.0,
            cover_loaded_for: None,
            cover_pending: Arc::new(Mutex::new(None)),
            select_anim: 0.0,
            select_anim_target: None,
        }
    }

    fn launch_selected(&self) {
        if let Some(g) = self.games.get(self.selected) {
            if let Some(app_id) = g.app_id {
                // Launch via Steam protocol to preserve overlay/achievements/cloud saves
                let url = format!("steam://rungameid/{}", app_id);
                let _ = Command::new("cmd").args(["/C", "start", "", &url]).spawn();
            } else {
                let _ = Command::new(&g.path).spawn();
            }
        }
    }
}

// XInput dynamic loader + simple polling
#[cfg(target_os = "windows")]
struct XInput {
    _lib: Library,
    get_state: unsafe extern "system" fn(DWORD, *mut XINPUT_STATE) -> DWORD,
    _set_state: Option<unsafe extern "system" fn(DWORD, *mut XINPUT_VIBRATION) -> DWORD>,
}

#[cfg(target_os = "windows")]
impl XInput {
    fn new() -> Result<Self, ()> {
        // try common xinput DLL names
        let names = ["xinput1_4.dll", "xinput1_3.dll", "xinput9_1_0.dll"];
        for name in names {
            if let Ok(lib) = unsafe { Library::new(name) } {
                unsafe {
                    // copy function pointers out of Symbols so they don't borrow `lib`
                    let gs_sym: libloading::Symbol<unsafe extern "system" fn(DWORD, *mut XINPUT_STATE) -> DWORD> = lib.get(b"XInputGetState\0").map_err(|_| ())?;
                    let get_state_fn = *gs_sym;
                    let set_state_fn = match lib.get(b"XInputSetState\0") {
                        Ok(s) => Some(*s),
                        Err(_) => None,
                    };

                    return Ok(XInput { _lib: lib, get_state: get_state_fn, _set_state: set_state_fn });
                }
            }
        }
        Err(())
    }

    fn poll_actions(&self) -> Vec<ControllerAction> {
        let mut actions = Vec::new();
        for idx in 0..4 {
            let mut state: XINPUT_STATE = unsafe { std::mem::zeroed() };
            let res = unsafe { (self.get_state)(idx, &mut state as *mut XINPUT_STATE) };
            if res == 0 {
                let gp = state.Gamepad;
                let buttons = gp.wButtons;
                if (buttons & XINPUT_GAMEPAD_DPAD_UP) != 0 {
                    actions.push(ControllerAction::Up);
                }
                if (buttons & XINPUT_GAMEPAD_DPAD_DOWN) != 0 {
                    actions.push(ControllerAction::Down);
                }
                if (buttons & XINPUT_GAMEPAD_A) != 0 {
                    actions.push(ControllerAction::Launch);
                }
                // left thumb Y axis (positive = up, negative = down)
                let ly = gp.sThumbLY as i32;
                if ly > 16000 {
                    actions.push(ControllerAction::Up);
                } else if ly < -16000 {
                    actions.push(ControllerAction::Down);
                }
            }
        }
        actions
    }

    fn get_states(&self) -> Vec<(u16, i32)> {
        let mut resvec = Vec::new();
        for idx in 0..4 {
            let mut state: XINPUT_STATE = unsafe { std::mem::zeroed() };
            let res = unsafe { (self.get_state)(idx, &mut state as *mut XINPUT_STATE) };
            if res == 0 {
                let gp = state.Gamepad;
                let buttons = gp.wButtons as u16;
                let ly = gp.sThumbLY as i32;
                resvec.push((buttons, ly));
            }
        }
        resvec
    }
}

// shared action enum used by both XInput and gilrs polling
enum ControllerAction {
    Up,
    Down,
    Launch,
}

#[derive(Serialize, Deserialize, Debug, Default)]
enum InputToken {
    #[default]
    None,
    XButton(u16),
    XAxis(i8),
    GilrsButton(u8),
    GilrsAxis(String, i8),
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Mapping {
    map: HashMap<String, InputToken>,
}

impl Mapping {
    fn load() -> Option<Self> {
        let path = "mapping.json";
        if let Ok(s) = fs::read_to_string(path) {
            serde_json::from_str(&s).ok()
        } else {
            None
        }
    }

    fn save(&self) {
        let _ = fs::write("mapping.json", serde_json::to_string_pretty(self).unwrap_or_default());
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Only process controller input when the window is focused
        let has_focus = ctx.input(|i| i.focused);

        if has_focus {
            // Request continuous repaint so controller input is always processed
            ctx.request_repaint();

            // Detect focus regain: start a cooldown to ignore stale button presses
            if !self.had_focus {
                self.focus_cooldown_until = Some(Instant::now());
                self.nav_held.clear();
            }
        } else {
            // Window not focused: clear held state so releasing buttons while
            // backgrounded doesn't leave stale repeat-delay entries.
            self.nav_held.clear();
        }
        self.had_focus = has_focus;

        // During cooldown after regaining focus, skip all controller input
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
        let process_input = has_focus && !in_cooldown;

        // 收集控制器原始状态（哪些方向/按钮当前被按住）
        let mut raw_held: HashSet<&'static str> = HashSet::new();
        let mut actions: Vec<ControllerAction> = Vec::new();

        if process_input {

        #[cfg(target_os = "windows")]
        {
            if let Some(xi) = &self.xinput {
                let states = xi.get_states();

                // If remapping active, capture from XInput first
                if let Some(target) = &self.remap_target {
                    for (buttons, ly) in states.iter() {
                        if *buttons != 0 {
                            self.mapping.map.insert(target.clone(), InputToken::XButton(*buttons));
                            self.mapping.save();
                            self.remap_target = None;
                            break;
                        }
                        if *ly < -16000 {
                            self.mapping.map.insert(target.clone(), InputToken::XAxis(-1));
                            self.mapping.save();
                            self.remap_target = None;
                            break;
                        }
                        if *ly > 16000 {
                            self.mapping.map.insert(target.clone(), InputToken::XAxis(1));
                            self.mapping.save();
                            self.remap_target = None;
                            break;
                        }
                    }
                } else {
                    // Default XInput mapping (works without mapping.json)
                    for (buttons, ly) in states.iter() {
                        if (buttons & XINPUT_GAMEPAD_DPAD_UP) != 0 {
                            raw_held.insert("up");
                        }
                        if (buttons & XINPUT_GAMEPAD_DPAD_DOWN) != 0 {
                            raw_held.insert("down");
                        }
                        if (buttons & XINPUT_GAMEPAD_A) != 0 {
                            raw_held.insert("launch");
                        }
                        // Left stick Y (positive = up, negative = down)
                        if *ly > 16000 {
                            raw_held.insert("up");
                        } else if *ly < -16000 {
                            raw_held.insert("down");
                        }
                    }

                    // Also apply custom mapping overrides
                    for (k, v) in self.mapping.map.iter() {
                        match v {
                            InputToken::XButton(mask) => {
                                for (buttons, _) in states.iter() {
                                    if (buttons & mask) != 0 {
                                        match k.as_str() {
                                            "up" => { raw_held.insert("up"); }
                                            "down" => { raw_held.insert("down"); }
                                            "launch" => { raw_held.insert("launch"); }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            InputToken::XAxis(dir) => {
                                for (_, ly) in states.iter() {
                                    if *dir > 0 && *ly > 16000 {
                                        if k == "up" { raw_held.insert("up"); }
                                        if k == "down" { raw_held.insert("down"); }
                                    }
                                    if *dir < 0 && *ly < -16000 {
                                        if k == "up" { raw_held.insert("up"); }
                                        if k == "down" { raw_held.insert("down"); }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // fallback to gilrs if no XInput raw_held detected
        if raw_held.is_empty() {
            if let Some(gilrs) = &mut self.gilrs {
                while let Some(ev) = gilrs.next_event() {
                    // If remapping is active, capture this input
                    if let Some(target) = &self.remap_target {
                        match ev.event {
                            EventType::ButtonPressed(btn, _) => {
                                let id = btn as u8;
                                self.mapping.map.insert(target.clone(), InputToken::GilrsButton(id));
                                self.mapping.save();
                                self.remap_target = None;
                            }
                            EventType::AxisChanged(axis, value, _) => {
                                let name = format!("{:?}", axis);
                                let dir = if value < 0.0 { -1 } else { 1 };
                                self.mapping.map.insert(target.clone(), InputToken::GilrsAxis(name, dir));
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
                                            "up" => { raw_held.insert("up"); }
                                            "down" => { raw_held.insert("down"); }
                                            "launch" => { raw_held.insert("launch"); }
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
                                            if k == "up" { raw_held.insert("up"); }
                                            if k == "down" { raw_held.insert("down"); }
                                        }
                                        if *dir > 0 && value > 0.7 {
                                            if k == "up" { raw_held.insert("up"); }
                                            if k == "down" { raw_held.insert("down"); }
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

        } // end if has_focus

        // ── Debounce / repeat-delay logic ──
        let now = Instant::now();

        // Remove nav_held entries for actions no longer held
        self.nav_held.retain(|k, _| raw_held.contains(k));

        for action_name in &["up", "down", "launch"] {
            if raw_held.contains(action_name) {
                let should_fire = if let Some(state) = self.nav_held.get_mut(action_name) {
                    if !state.past_initial {
                        // Waiting for initial delay
                        if now.duration_since(state.since).as_millis() >= NAV_INITIAL_DELAY_MS {
                            state.past_initial = true;
                            state.last_fire = now;
                            true
                        } else {
                            false
                        }
                    } else {
                        // Repeating
                        if now.duration_since(state.last_fire).as_millis() >= NAV_REPEAT_INTERVAL_MS {
                            state.last_fire = now;
                            true
                        } else {
                            false
                        }
                    }
                } else {
                    // First frame this action is held — fire immediately
                    self.nav_held.insert(action_name, NavState {
                        since: now,
                        last_fire: now,
                        past_initial: false,
                    });
                    true
                };

                if should_fire {
                    match *action_name {
                        "up" => actions.push(ControllerAction::Up),
                        "down" => actions.push(ControllerAction::Down),
                        "launch" => actions.push(ControllerAction::Launch),
                        _ => {}
                    }
                }
            }
        }

        // 应用收集到的动作
        for act in &actions {
            match act {
                ControllerAction::Up => {
                    if self.selected > 0 {
                        self.selected -= 1;
                        self.cover_nav_dir = -1.0;
                    }
                }
                ControllerAction::Down => {
                    if self.selected + 1 < self.games.len() {
                        self.selected += 1;
                        self.cover_nav_dir = 1.0;
                    }
                }
                ControllerAction::Launch => {
                    self.launch_selected();
                }
            }
        }

        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Launch").clicked() {
                    self.launch_selected();
                }
                if ui.button("Refresh").clicked() {
                    self.games = scan_games_with_paths(&self.steam_paths);
                    self.cover = None;
                    self.cover_prev = None;
                    self.cover_loaded_for = None;
                    if self.selected >= self.games.len() && !self.games.is_empty() {
                        self.selected = self.games.len() - 1;
                    }
                }
            });
        });

        // ── Load cover when selection changes (async) ──
        if self.cover_loaded_for != Some(self.selected) {
            self.cover_loaded_for = Some(self.selected);
            // Move current cover to prev for crossfade
            self.cover_prev = self.cover.take();
            self.cover_fade = 0.0;
            if let Some(game) = self.games.get(self.selected) {
                if let Some(app_id) = game.app_id {
                    let pending = Arc::clone(&self.cover_pending);
                    let paths = self.steam_paths.clone();
                    let ctx_clone = ctx.clone();
                    // Clear any stale pending result
                    if let Ok(mut lock) = pending.lock() {
                        *lock = None;
                    }
                    std::thread::spawn(move || {
                        let bytes = load_cover_bytes(&paths, app_id);
                        if let Some(bytes) = bytes {
                            if let Ok(mut lock) = pending.lock() {
                                *lock = Some((app_id, bytes));
                            }
                            ctx_clone.request_repaint();
                        }
                    });
                }
            }
        }

        // Check if async cover load finished
        if self.cover.is_none() {
            let result = self.cover_pending.lock().ok().and_then(|mut lock| lock.take());
            if let Some((app_id, bytes)) = result {
                if let Some(tex) = bytes_to_texture(ctx, &bytes, format!("cover_{}", app_id)) {
                    self.cover = Some((app_id, tex));
                    self.cover_fade = 0.0;
                }
            }
        }

        // Advance crossfade animation
        if self.cover_fade < 1.0 {
            let dt = ctx.input(|i| i.predicted_dt);
            self.cover_fade = (self.cover_fade + dt * 3.0).min(1.0); // ~0.33s fade
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // ── Paint cover as full-screen background with crossfade ──
            let screen = ctx.screen_rect();
            let bg_painter = ctx.layer_painter(egui::LayerId::background());
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            let base_alpha: f32 = 60.0;

            // Helper: compute contain-mode rect for a texture with vertical offset
            let contain_rect_offset = |tex: &egui::TextureHandle, dy: f32| -> egui::Rect {
                let tex_size = tex.size_vec2();
                let scale = (screen.width() / tex_size.x).min(screen.height() / tex_size.y);
                let img_w = tex_size.x * scale;
                let img_h = tex_size.y * scale;
                let offset_x = (screen.width() - img_w) * 0.5;
                let offset_y = (screen.height() - img_h) * 0.5 + dy;
                egui::Rect::from_min_size(
                    egui::pos2(screen.min.x + offset_x, screen.min.y + offset_y),
                    egui::vec2(img_w, img_h),
                )
            };

            let slide_distance = 12.0; // pixels of slide
            let ease_t = 1.0 - (1.0 - self.cover_fade) * (1.0 - self.cover_fade); // ease-out

            // Draw previous cover (fading out, no movement)
            if self.cover_fade < 1.0 {
                if let Some((_id, tex)) = &self.cover_prev {
                    let alpha = (base_alpha * (1.0 - self.cover_fade)) as u8;
                    let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
                    bg_painter.image(tex.id(), contain_rect_offset(tex, 0.0), uv, tint);
                }
            }

            // Draw current cover (fading in, sliding in)
            if let Some((_id, tex)) = &self.cover {
                let alpha = (base_alpha * self.cover_fade) as u8;
                let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
                let dy = self.cover_nav_dir * slide_distance * (1.0 - ease_t);
                bg_painter.image(tex.id(), contain_rect_offset(tex, dy), uv, tint);
            }

            ui.heading("Clix — 手柄优先游戏启动器 原型");
            ui.label(format!("已发现游戏: {}", self.games.len()));
            ui.separator();

            egui::ScrollArea::vertical().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                // Animate selection scale
                if self.select_anim_target != Some(self.selected) {
                    self.select_anim_target = Some(self.selected);
                    self.select_anim = 0.0;
                }
                let dt = ctx.input(|i| i.predicted_dt);
                self.select_anim = (self.select_anim + dt * 5.0).min(1.0); // ~0.2s ease-in

                let base_size = 16.0;
                let selected_size = 28.0;
                // Fixed row height based on the largest possible size to prevent layout jitter
                let fixed_row_h = selected_size * 1.3;

                for (i, g) in self.games.iter().enumerate() {
                    let is_selected = i == self.selected;
                    let is_steam = g.app_id.is_some();

                    let font_size = if is_selected {
                        let t = self.select_anim;
                        // Smooth ease-out interpolation
                        let t = 1.0 - (1.0 - t) * (1.0 - t);
                        base_size + (selected_size - base_size) * t
                    } else {
                        base_size
                    };

                    let text_color = if is_selected {
                        egui::Color32::from_rgb(100, 200, 255)
                    } else {
                        egui::Color32::from_rgb(220, 220, 220)
                    };
                    let shadow_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200);

                    let font_id = egui::FontId::proportional(font_size);
                    let galley = ui.painter().layout_no_wrap(
                        g.name.clone(),
                        font_id.clone(),
                        text_color,
                    );

                    let avail_w = ui.available_width();
                    let (rect, _resp) = ui.allocate_exact_size(
                        egui::vec2(avail_w, fixed_row_h),
                        egui::Sense::hover(),
                    );

                    // Auto-scroll to keep the selected item visible
                    if is_selected {
                        ui.scroll_to_rect(rect, Some(egui::Align::Center));
                    }

                    let text_y = rect.min.y + (fixed_row_h - galley.size().y) * 0.5;
                    let text_x = rect.min.x;

                    // Draw text shadow
                    let shadow_galley = ui.painter().layout_no_wrap(
                        g.name.clone(),
                        font_id,
                        shadow_color,
                    );
                    for offset in [
                        egui::vec2(1.0, 1.0),
                        egui::vec2(-1.0, 1.0),
                        egui::vec2(1.0, -1.0),
                        egui::vec2(-1.0, -1.0),
                        egui::vec2(0.0, 2.0),
                        egui::vec2(2.0, 0.0),
                    ] {
                        ui.painter().galley(
                            egui::pos2(text_x, text_y) + offset,
                            shadow_galley.clone(),
                        );
                    }

                    // Draw foreground text
                    ui.painter().galley(
                        egui::pos2(text_x, text_y),
                        galley,
                    );
                }
            });
        });
    }
}

/// Parse a Valve ACF (KeyValues) file and extract key-value pairs at the top object level.
fn parse_acf_values(content: &str) -> HashMap<String, String> {
    use regex::Regex;
    let re = Regex::new(r#""([^"]+)"\s+"([^"]*)""#).unwrap();
    let mut map = HashMap::new();
    for cap in re.captures_iter(content) {
        if let (Some(k), Some(v)) = (cap.get(1), cap.get(2)) {
            map.insert(k.as_str().to_lowercase(), v.as_str().to_string());
        }
    }
    map
}

/// Load image bytes into an egui texture handle.
fn bytes_to_texture(
    ctx: &egui::Context,
    bytes: &[u8],
    label: String,
) -> Option<egui::TextureHandle> {
    let dyn_img = image::load_from_memory(bytes).ok()?;
    let rgba = dyn_img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    Some(ctx.load_texture(label, color_image, egui::TextureOptions::LINEAR))
}

/// Local HD cache directory for downloaded cover images.
fn hd_cache_dir() -> PathBuf {
    let mut dir = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();
    dir.push("cover_cache");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Try to download a high-resolution hero image from Steam CDN and cache locally.
fn download_hd_cover(app_id: u32) -> Option<Vec<u8>> {
    // Steam CDN serves library_hero at full resolution (typically 1920×620 or larger).
    let url = format!(
        "https://steamcdn-a.akamaihd.net/steam/apps/{}/library_hero.jpg",
        app_id
    );
    let resp = ureq::get(&url).call().ok()?;
    if resp.status() != 200 {
        return None;
    }
    let mut bytes: Vec<u8> = Vec::new();
    // Limit read to 10 MB to stay safe
    resp.into_reader()
        .take(10 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() < 1024 {
        return None; // too small, likely an error page
    }
    // Cache to disk
    let cache_path = hd_cache_dir().join(format!("{}_hero_hd.jpg", app_id));
    let _ = std::fs::write(&cache_path, &bytes);
    Some(bytes)
}

/// Load cover image bytes (runs on background thread, no ctx needed).
/// Priority: local HD cache > Steam CDN download > Steam local cache.
fn load_cover_bytes(
    steam_paths: &[PathBuf],
    app_id: u32,
) -> Option<Vec<u8>> {
    // 1. Check our own HD cache first (previously downloaded)
    let hd_path = hd_cache_dir().join(format!("{}_hero_hd.jpg", app_id));
    if hd_path.exists() {
        if let Ok(bytes) = std::fs::read(&hd_path) {
            if bytes.len() > 1024 {
                return Some(bytes);
            }
        }
    }

    // 2. Try downloading HD version from Steam CDN
    if let Some(bytes) = download_hd_cover(app_id) {
        return Some(bytes);
    }

    // 3. Fallback: local Steam library cache (lower resolution)
    let candidates = [
        "library_hero.jpg",
        "library_hero.png",
        "header.jpg",
        "library_600x900.jpg",
    ];

    for steam_root in steam_paths {
        let app_cache_dir = steam_root
            .join("appcache")
            .join("librarycache")
            .join(app_id.to_string());
        if !app_cache_dir.exists() {
            continue;
        }
        for name in &candidates {
            let img_path = app_cache_dir.join(name);
            if img_path.exists() {
                if let Ok(bytes) = std::fs::read(&img_path) {
                    if bytes.len() > 1024 {
                        return Some(bytes);
                    }
                }
            }
        }
    }
    None
}

/// Discover Steam installation directories from registry and environment.
fn find_steam_paths() -> Vec<PathBuf> {
    let mut steam_paths: Vec<PathBuf> = Vec::new();

    #[cfg(target_os = "windows")]
    {
        if let Ok(hklm) = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
            .open_subkey("SOFTWARE\\WOW6432Node\\Valve\\Steam")
        {
            if let Ok(s) = hklm.get_value::<String, &str>("InstallPath") {
                steam_paths.push(PathBuf::from(s));
            }
        }
        if let Ok(hkcu) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .open_subkey("Software\\Valve\\Steam")
        {
            if let Ok(s) = hkcu.get_value::<String, &str>("SteamPath") {
                steam_paths.push(PathBuf::from(s));
            }
        }
    }

    if let Some(p) = std::env::var_os("ProgramFiles(x86)") {
        steam_paths.push(PathBuf::from(p).join("Steam"));
    }
    if let Some(p) = std::env::var_os("ProgramFiles") {
        steam_paths.push(PathBuf::from(p).join("Steam"));
    }

    steam_paths.retain(|p| p.exists());
    steam_paths.sort();
    steam_paths.dedup();
    steam_paths
}

fn scan_games_with_paths(steam_paths: &[PathBuf]) -> Vec<Game> {
    use regex::Regex;
    let mut games: Vec<Game> = Vec::new();
    let mut seen_app_ids: HashSet<u32> = HashSet::new();

    // ── Step 1: Collect all Steam library folders from libraryfolders.vdf ──
    let vdf_re = Regex::new(r#"[\"]([A-Za-z]:\\[^\"]+)[\"]"#).unwrap();
    let mut library_folders: Vec<PathBuf> = Vec::new();

    for steam_root in steam_paths.iter() {
        let libfile = steam_root.join("steamapps").join("libraryfolders.vdf");
        if libfile.exists() {
            if let Ok(s) = std::fs::read_to_string(&libfile) {
                for cap in vdf_re.captures_iter(&s) {
                    if let Some(m) = cap.get(1) {
                        let p = PathBuf::from(m.as_str());
                        if p.exists() {
                            library_folders.push(p.join("steamapps"));
                        }
                    }
                }
            }
        }
        library_folders.push(steam_root.join("steamapps"));
    }

    library_folders.retain(|p| p.exists());
    library_folders.sort();
    library_folders.dedup();

    // ── Step 2: Parse LastPlayed from userdata/*/config/localconfig.vdf ──
    let last_played_map = parse_last_played_from_userdata(steam_paths);

    // ── Step 3: Parse appmanifest_*.acf files in each library folder ──
    for lib in &library_folders {
        if let Ok(entries) = std::fs::read_dir(lib) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !fname.starts_with("appmanifest_") || !fname.ends_with(".acf") {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let vals = parse_acf_values(&content);
                    let app_id = vals.get("appid").and_then(|v| v.parse::<u32>().ok());
                    let name = vals.get("name").cloned().unwrap_or_default();
                    let install_dir = vals.get("installdir").cloned().unwrap_or_default();
                    let state_flags = vals
                        .get("stateflags")
                        .and_then(|v| v.parse::<u32>().ok())
                        .unwrap_or(0);

                    // StateFlags & 4 == fully installed
                    if (state_flags & 4) == 0 || name.is_empty() {
                        continue;
                    }
                    if let Some(id) = app_id {
                        if !seen_app_ids.insert(id) {
                            continue; // duplicate
                        }
                        let game_path = lib.join("common").join(&install_dir);
                        games.push(Game {
                            name,
                            path: game_path,
                            app_id: Some(id),
                            last_played: last_played_map.get(&id).copied().unwrap_or(0),
                        });
                    }
                }
            }
        }
    }

    // ── Step 4: Supplement from Windows Uninstall registry keys ──
    #[cfg(target_os = "windows")]
    {
        let uninstall_paths = [
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ];
        let steam_app_re = Regex::new(r"^Steam App (\d+)$").unwrap();
        let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);

        for uninstall_path in &uninstall_paths {
            if let Ok(uninstall_key) = hklm.open_subkey(uninstall_path) {
                for subkey_name in uninstall_key.enum_keys().filter_map(|k| k.ok()) {
                    if let Some(caps) = steam_app_re.captures(&subkey_name) {
                        let app_id: u32 = match caps.get(1).and_then(|m| m.as_str().parse().ok())
                        {
                            Some(id) => id,
                            None => continue,
                        };
                        if seen_app_ids.contains(&app_id) {
                            continue;
                        }
                        if let Ok(subkey) = uninstall_key.open_subkey(&subkey_name) {
                            let display_name: String =
                                subkey.get_value("DisplayName").unwrap_or_default();
                            let install_location: String =
                                subkey.get_value("InstallLocation").unwrap_or_default();
                            if display_name.is_empty() {
                                continue;
                            }
                            seen_app_ids.insert(app_id);
                            games.push(Game {
                                name: display_name,
                                path: PathBuf::from(install_location),
                                app_id: Some(app_id),
                                last_played: last_played_map.get(&app_id).copied().unwrap_or(0),
                            });
                        }
                    }
                }
            }
        }
    }

    // ── Step 5: Sort by last played time descending, then by name ──
    games.sort_by(|a, b| {
        b.last_played.cmp(&a.last_played)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    games
}

/// Parse LastPlayed timestamps from Steam's userdata/*/config/localconfig.vdf
fn parse_last_played_from_userdata(steam_paths: &[PathBuf]) -> HashMap<u32, u64> {
    use regex::Regex;
    let mut map: HashMap<u32, u64> = HashMap::new();

    for steam_root in steam_paths {
        let userdata_dir = steam_root.join("userdata");
        if !userdata_dir.exists() {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&userdata_dir) else { continue };
        for entry in entries.filter_map(|e| e.ok()) {
            let cfg = entry.path().join("config").join("localconfig.vdf");
            if !cfg.exists() {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&cfg) else { continue };

            // Find the first "apps" section under Software/Valve/Steam
            // and parse app_id -> LastPlayed pairs.
            // The VDF structure is:
            //   "apps" {
            //     "12345" {
            //       "LastPlayed"  "1700000000"
            //       ...
            //     }
            //   }
            let app_block_re = Regex::new(
                r#"(?m)^\s*"(\d+)"\s*\n\s*\{[^}]*?"LastPlayed"\s+"(\d+)"#
            ).unwrap();

            // We need to find the right "apps" section. Locate it first.
            if let Some(apps_pos) = content.find("\"apps\"") {
                let after_apps = &content[apps_pos..];
                // Find the opening brace
                if let Some(brace_pos) = after_apps.find('{') {
                    let apps_content = &after_apps[brace_pos..];
                    for cap in app_block_re.captures_iter(apps_content) {
                        if let (Some(id_m), Some(ts_m)) = (cap.get(1), cap.get(2)) {
                            if let (Ok(app_id), Ok(ts)) = (
                                id_m.as_str().parse::<u32>(),
                                ts_m.as_str().parse::<u64>(),
                            ) {
                                // Keep the most recent timestamp across user profiles
                                let entry = map.entry(app_id).or_insert(0);
                                if ts > *entry {
                                    *entry = ts;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    map
}

fn main() {
    let options = eframe::NativeOptions {
        fullscreen: true,
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Clix Launcher Prototype",
        options,
        Box::new(|cc| {
            // Load a CJK-capable font for Chinese text rendering
            let mut fonts = egui::FontDefinitions::default();
            // Use Windows built-in Microsoft YaHei
            let font_path = std::path::Path::new("C:\\Windows\\Fonts\\msyh.ttc");
            if font_path.exists() {
                if let Ok(font_data) = std::fs::read(font_path) {
                    fonts.font_data.insert(
                        "msyh".to_owned(),
                        egui::FontData::from_owned(font_data),
                    );
                    // Prepend to proportional and monospace families
                    fonts
                        .families
                        .entry(egui::FontFamily::Proportional)
                        .or_default()
                        .insert(0, "msyh".to_owned());
                    fonts
                        .families
                        .entry(egui::FontFamily::Monospace)
                        .or_default()
                        .push("msyh".to_owned());
                }
            }
            cc.egui_ctx.set_fonts(fonts);
            Box::new(LauncherApp::new())
        }),
    );
}
