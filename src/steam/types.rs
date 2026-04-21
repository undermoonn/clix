use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub struct Game {
    pub name: String,
    pub path: PathBuf,
    pub app_id: Option<u32>,
    pub last_played: u64,
    pub playtime_minutes: u32,
    pub installed_size_bytes: Option<u64>,
    pub dlss_version: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AchievementItem {
    pub api_name: String,
    #[serde(default)]
    pub group_key: Option<String>,
    #[serde(default)]
    pub bit_index: Option<u32>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub is_hidden: bool,
    pub unlocked: Option<bool>,
    pub unlock_time: Option<u64>,
    pub global_percent: Option<f32>,
    pub icon_url: Option<String>,
    pub icon_gray_url: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AchievementSummary {
    pub unlocked: Option<u32>,
    pub total: u32,
    pub items: Vec<AchievementItem>,
}