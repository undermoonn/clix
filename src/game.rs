use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameScanOptions {
    pub detect_steam_games: bool,
    pub detect_epic_games: bool,
    pub detect_xbox_games: bool,
}

impl Default for GameScanOptions {
    fn default() -> Self {
        Self {
            detect_steam_games: true,
            detect_epic_games: false,
            detect_xbox_games: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GameSource {
    #[default]
    Steam,
    Epic,
    Xbox,
}

impl GameSource {
    pub fn badge_label(self) -> &'static str {
        match self {
            Self::Steam => "STEAM",
            Self::Epic => "EPIC",
            Self::Xbox => "XBOX",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GameIconKey {
    SteamAppId(u32),
    InstallPath(PathBuf),
}

pub struct Game {
    pub source: GameSource,
    pub name: String,
    pub install_path: PathBuf,
    pub launch_target: Option<PathBuf>,
    // Steam app id used for launching, icon cache keys, and stable game identity.
    // Populated from Steam library metadata or Windows uninstall registry entries.
    pub steam_app_id: Option<u32>,
    // Xbox Appx manifest Application Id used for shell activation.
    pub appx_id: Option<String>,
    // Epic AppName used as the stable identity when no Steam app id is available.
    pub epic_app_name: Option<String>,
    // Xbox package family name used as the stable identity when no Steam app id is available.
    pub xbox_package_family_name: Option<String>,
    pub last_played: u64,
    pub playtime_minutes: u32,
    pub installed_size_bytes: Option<u64>,
    pub dlss_version: Option<String>,
}

impl Game {
    pub fn icon_key(&self) -> GameIconKey {
        self.steam_app_id
            .map(GameIconKey::SteamAppId)
            .unwrap_or_else(|| GameIconKey::InstallPath(self.install_path.clone()))
    }

    pub fn persistent_key(&self) -> String {
        let source = match self.source {
            GameSource::Steam => "steam",
            GameSource::Epic => "epic",
            GameSource::Xbox => "xbox",
        };

        if let Some(steam_app_id) = self.steam_app_id {
            format!("{}:app:{}", source, steam_app_id)
        } else if let Some(platform_id) = match self.source {
            GameSource::Steam => None,
            GameSource::Epic => self.epic_app_name.as_deref(),
            GameSource::Xbox => self.xbox_package_family_name.as_deref(),
        } {
            format!("{}:id:{}", source, normalize_identifier_key(platform_id))
        } else {
            format!("{}:path:{}", source, normalize_path_key(&self.install_path))
        }
    }
}

fn normalize_identifier_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_path_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_ascii_lowercase()
}

pub fn sort_games_by_last_played(games: &mut [Game]) {
    games.sort_by(|a, b| {
        b.last_played
            .cmp(&a.last_played)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

pub fn scan_installed_games(steam_paths: &[PathBuf], options: &GameScanOptions) -> Vec<Game> {
    let mut games = Vec::new();
    if options.detect_steam_games {
        games.extend(scan_platform_games("steam", || {
            crate::steam::scan_games_with_paths(steam_paths)
        }));
    }
    if options.detect_epic_games {
        games.extend(scan_platform_games(
            "epic",
            crate::game_platforms::epic::scan_games,
        ));
    }
    if options.detect_xbox_games {
        games.extend(scan_platform_games(
            "xbox",
            crate::game_platforms::xbox::scan_games,
        ));
    }
    crate::game_last_played::merge_into_games(&mut games);
    sort_games_by_last_played(&mut games);
    games
}

fn scan_platform_games<F>(platform: &str, scan: F) -> Vec<Game>
where
    F: FnOnce() -> Vec<Game>,
{
    let started_at = Instant::now();
    let games = scan();
    log_platform_scan_time(platform, started_at.elapsed(), games.len());
    games
}

fn log_platform_scan_time(platform: &str, elapsed: Duration, game_count: usize) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!(
        "[{}] {} scan took {} ms ({} games)\n",
        timestamp,
        platform,
        elapsed.as_millis(),
        game_count
    );

    eprint!("{}", line);

    let log_path = crate::assets::cache::logs_dir().join("scan_timings.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = file.write_all(line.as_bytes());
    }
}