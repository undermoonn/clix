use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GameSource {
    #[default]
    Steam,
    Epic,
}

impl GameSource {
    pub fn badge_label(self) -> &'static str {
        match self {
            Self::Steam => "STEAM",
            Self::Epic => "EPIC",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GameIconKey {
    AppId(u32),
    InstallPath(PathBuf),
}

pub struct Game {
    pub source: GameSource,
    pub name: String,
    pub path: PathBuf,
    pub launch_target: Option<PathBuf>,
    pub app_id: Option<u32>,
    pub persistent_id: Option<String>,
    pub last_played: u64,
    pub playtime_minutes: u32,
    pub installed_size_bytes: Option<u64>,
    pub dlss_version: Option<String>,
}

impl Game {
    pub fn icon_key(&self) -> GameIconKey {
        self.app_id
            .map(GameIconKey::AppId)
            .unwrap_or_else(|| GameIconKey::InstallPath(self.path.clone()))
    }

    pub fn persistent_key(&self) -> String {
        let source = match self.source {
            GameSource::Steam => "steam",
            GameSource::Epic => "epic",
        };

        if let Some(app_id) = self.app_id {
            format!("{}:app:{}", source, app_id)
        } else if let Some(persistent_id) = self.persistent_id.as_deref() {
            format!("{}:id:{}", source, normalize_identifier_key(persistent_id))
        } else {
            format!("{}:path:{}", source, normalize_path_key(&self.path))
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

pub fn scan_installed_games(steam_paths: &[PathBuf]) -> Vec<Game> {
    let mut games = Vec::new();
    games.extend(crate::steam::scan_games_with_paths(steam_paths));
    games.extend(crate::epic::scan_games());
    crate::game_last_played::merge_into_games(&mut games);
    sort_games_by_last_played(&mut games);
    games
}