use eframe::egui;
use gilrs::{EventType, Gilrs};
use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[cfg(target_os = "windows")]
use winapi::um::xinput::{XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_UP};

use crate::cover;
use crate::input::*;
use crate::steam::{self, Game};
use crate::ui;

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
    focus_cooldown_until: Option<Instant>,
    steam_paths: Vec<std::path::PathBuf>,
    cover: Option<(u32, egui::TextureHandle)>,
    cover_prev: Option<(u32, egui::TextureHandle)>,
    cover_fade: f32,
    cover_nav_dir: f32,
    cover_loaded_for: Option<usize>,
    cover_pending: Arc<Mutex<Option<(u32, Vec<u8>)>>>,
    select_anim: f32,
    select_anim_target: Option<usize>,
    hint_icons: Option<ui::HintIcons>,
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
            hint_icons: None,
        }
    }

    fn launch_selected(&self) {
        if let Some(g) = self.games.get(self.selected) {
            if let Some(app_id) = g.app_id {
                let url = format!("steam://rungameid/{}", app_id);
                let _ = Command::new("cmd")
                    .args(["/C", "start", "", &url])
                    .spawn();
            } else {
                let _ = Command::new(&g.path).spawn();
            }
        }
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let has_focus = ctx.input(|i| i.focused);

        if has_focus {
            ctx.request_repaint();
            if !self.had_focus {
                self.focus_cooldown_until = Some(Instant::now());
                self.nav_held.clear();
            }
        } else {
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
        let process_input = has_focus && !in_cooldown;

        let mut raw_held: HashSet<&'static str> = HashSet::new();
        let mut actions: Vec<ControllerAction> = Vec::new();

        if process_input {
            // XInput polling
            #[cfg(target_os = "windows")]
            {
                if let Some(xi) = &self.xinput {
                    let states = xi.get_states();

                    if let Some(target) = &self.remap_target {
                        for (buttons, ly) in states.iter() {
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
                            if (buttons & XINPUT_GAMEPAD_B) != 0 {
                                raw_held.insert("quit");
                            }
                            if *ly > 16000 {
                                raw_held.insert("up");
                            } else if *ly < -16000 {
                                raw_held.insert("down");
                            }
                        }

                        for (k, v) in self.mapping.map.iter() {
                            match v {
                                InputToken::XButton(mask) => {
                                    for (buttons, _) in states.iter() {
                                        if (buttons & mask) != 0 {
                                            match k.as_str() {
                                                "up" => {
                                                    raw_held.insert("up");
                                                }
                                                "down" => {
                                                    raw_held.insert("down");
                                                }
                                                "launch" => {
                                                    raw_held.insert("launch");
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
                                    for (_, ly) in states.iter() {
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
                                                "launch" => {
                                                    raw_held.insert("launch");
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
        } // end if process_input

        // Debounce / repeat-delay
        let now = Instant::now();
        self.nav_held.retain(|k, _| raw_held.contains(k));

        for action_name in &["up", "down", "launch", "quit"] {
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
                    } else if now.duration_since(state.last_fire).as_millis()
                        >= NAV_REPEAT_INTERVAL_MS
                    {
                        state.last_fire = now;
                        true
                    } else {
                        false
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
                        "launch" => actions.push(ControllerAction::Launch),
                        "quit" => actions.push(ControllerAction::Quit),
                        _ => {}
                    }
                }
            }
        }

        // Apply actions
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
                ControllerAction::Quit => {
                    frame.close();
                }
            }
        }

        // Cover loading (async)
        if self.cover_loaded_for != Some(self.selected) {
            self.cover_loaded_for = Some(self.selected);
            self.cover_prev = self.cover.take();
            self.cover_fade = 0.0;
            if let Some(game) = self.games.get(self.selected) {
                if let Some(app_id) = game.app_id {
                    let pending = Arc::clone(&self.cover_pending);
                    let paths = self.steam_paths.clone();
                    let ctx_clone = ctx.clone();
                    if let Ok(mut lock) = pending.lock() {
                        *lock = None;
                    }
                    std::thread::spawn(move || {
                        let bytes = cover::load_cover_bytes(&paths, app_id);
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

        if self.cover.is_none() {
            let result = self
                .cover_pending
                .lock()
                .ok()
                .and_then(|mut lock| lock.take());
            if let Some((app_id, bytes)) = result {
                if let Some(tex) =
                    cover::bytes_to_texture(ctx, &bytes, format!("cover_{}", app_id))
                {
                    self.cover = Some((app_id, tex));
                    self.cover_fade = 0.0;
                }
            }
        }

        // Advance crossfade
        if self.cover_fade < 1.0 {
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
        self.select_anim = (self.select_anim + dt * 5.0).min(1.0);
        if self.select_anim < 1.0 {
            ctx.request_repaint();
        }

        // Load hint icons lazily
        if self.hint_icons.is_none() {
            self.hint_icons = ui::load_hint_icons(ctx);
        }

        // Draw UI
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                ui::draw_background(
                    ctx,
                    &self.cover,
                    &self.cover_prev,
                    self.cover_fade,
                    self.cover_nav_dir,
                );

                ui::draw_game_list(ui, &self.games, self.selected, self.select_anim);

                if let Some(icons) = &self.hint_icons {
                    ui::draw_hint_bar(ui, icons);
                }
            });
    }
}
