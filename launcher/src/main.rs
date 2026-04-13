use eframe::{egui, epi};
use gilrs::{Axis, Button, EventType, Gilrs};
use std::path::PathBuf;
use std::process::Command;
use walkdir::WalkDir;

struct Game {
    name: String,
    path: PathBuf,
}

struct LauncherApp {
    games: Vec<Game>,
    selected: usize,
    gilrs: Option<Gilrs>,
}

impl LauncherApp {
    fn new() -> Self {
        let games = scan_games();
        let gilrs = Gilrs::new().ok();
        LauncherApp {
            games,
            selected: 0,
            gilrs,
        }
    }

    fn launch_selected(&self) {
        if let Some(g) = self.games.get(self.selected) {
            let _ = Command::new(&g.path).spawn();
        }
    }
}

impl epi::App for LauncherApp {
    fn name(&self) -> &str {
        "Clix Launcher Prototype"
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        // Poll controller events and map to navigation
        if let Some(gilrs) = &mut self.gilrs {
            while let Some(ev) = gilrs.next_event() {
                match ev.event {
                    EventType::ButtonPressed(btn, _) => match btn {
                        Button::DPadUp => {
                            if self.selected > 0 {
                                self.selected -= 1;
                            }
                        }
                        Button::DPadDown => {
                            if self.selected + 1 < self.games.len() {
                                self.selected += 1;
                            }
                        }
                        Button::South => {
                            self.launch_selected();
                        }
                        _ => {}
                    },
                    EventType::AxisChanged(axis, value, _) => {
                        // simple axis handling for left stick Y
                        if axis == Axis::LeftStickY {
                            if value < -0.7 {
                                if self.selected > 0 {
                                    self.selected -= 1;
                                }
                            } else if value > 0.7 {
                                if self.selected + 1 < self.games.len() {
                                    self.selected += 1;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Clix — 手柄优先游戏启动器 原型");
            ui.label(format!("已发现游戏: {}", self.games.len()));
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, g) in self.games.iter().enumerate() {
                    let selected = i == self.selected;
                    if selected {
                        ui.colored_label(egui::Color32::LIGHT_BLUE, &g.name);
                    } else {
                        ui.label(&g.name);
                    }
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Launch").clicked() {
                    self.launch_selected();
                }
                if ui.button("Refresh").clicked() {
                    self.games = scan_games();
                    if self.selected >= self.games.len() && !self.games.is_empty() {
                        self.selected = self.games.len() - 1;
                    }
                }
            });
        });
    }
}

fn scan_games() -> Vec<Game> {
    let mut games = Vec::new();
    let candidates = vec![
        dirs::program_files().unwrap_or_else(|| PathBuf::from("C:\\Program Files")),
        dirs::program_files_x86().unwrap_or_else(|| PathBuf::from("C:\\Program Files (x86)")),
    ];

    for base in candidates {
        if base.exists() {
            for entry in WalkDir::new(base).max_depth(4).into_iter().filter_map(|e| e.ok()) {
                if entry.path().is_file() {
                    if let Some(ext) = entry.path().extension() {
                        if ext.eq_ignore_ascii_case("exe") {
                            if let Some(name) = entry.path().file_stem().and_then(|s| s.to_str()) {
                                let path = entry.path().to_path_buf();
                                games.push(Game {
                                    name: name.to_string(),
                                    path,
                                });
                                if games.len() >= 200 {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Try Steam common folder as common game installs
    let steam_common = PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common");
    if steam_common.exists() {
        for entry in WalkDir::new(steam_common).max_depth(3).into_iter().filter_map(|e| e.ok()) {
            if entry.path().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext.eq_ignore_ascii_case("exe") {
                        if let Some(name) = entry.path().file_stem().and_then(|s| s.to_str()) {
                            let path = entry.path().to_path_buf();
                            games.push(Game {
                                name: name.to_string(),
                                path,
                            });
                            if games.len() >= 300 {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    games
}

fn main() {
    let options = eframe::NativeOptions::default();
    let app = LauncherApp::new();
    eframe::run_native(Box::new(app), options);
}
