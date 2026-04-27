mod animation;
mod icons;
mod loading;
mod query;
mod store;

use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::i18n::AppLanguage;
use crate::steam::AchievementSummary;

struct HiddenRevealState {
    api_name: String,
    progress: f32,
    started_at: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingAchievementKind {
    Overview,
    Details,
}

type PendingAchievementResult = (PendingAchievementKind, u32, Option<AchievementSummary>);
type PendingAchievementIcon = (u64, String, Option<Vec<u8>>);

const MIN_REFRESH_RING_DURATION: Duration = Duration::from_millis(1000);

pub struct AchievementState {
    language: AppLanguage,
    overview_cache: HashMap<u32, AchievementSummary>,
    displayed_overview_app_id: Option<u32>,
    previous_overview: Option<AchievementSummary>,
    previous_overview_reveal: f32,
    previous_overview_reveal_started_at: Option<Instant>,
    detail_cache: Option<(u32, AchievementSummary)>,
    revealed_hidden: HashMap<u32, HiddenRevealState>,
    pending: Arc<Mutex<Vec<PendingAchievementResult>>>,
    overview_loading: HashSet<u32>,
    detail_loading: HashSet<u32>,
    refreshing: HashSet<u32>,
    refresh_indicator_until: HashMap<u32, Instant>,
    no_data: HashSet<u32>,
    icon_cache: HashMap<String, egui::TextureHandle>,
    icon_pending: Arc<Mutex<Vec<PendingAchievementIcon>>>,
    icon_loading: HashSet<String>,
    icon_failed: HashSet<String>,
    icon_reveal: HashMap<String, f32>,
    icon_reveal_started_at: HashMap<String, Instant>,
    percent_reveal: HashMap<String, f32>,
    percent_reveal_started_at: HashMap<String, Instant>,
    icon_scope_app_id: Option<u32>,
    icon_generation: u64,
    text_reveal: HashMap<u32, f32>,
    text_reveal_started_at: HashMap<u32, Instant>,
    checked_overview_for: Option<u32>,
    checked_detail_for: Option<u32>,
}

impl AchievementState {
    pub fn new() -> Self {
        Self {
            language: AppLanguage::English,
            overview_cache: HashMap::new(),
            displayed_overview_app_id: None,
            previous_overview: None,
            previous_overview_reveal: 0.0,
            previous_overview_reveal_started_at: None,
            detail_cache: None,
            revealed_hidden: HashMap::new(),
            pending: Arc::new(Mutex::new(Vec::new())),
            overview_loading: HashSet::new(),
            detail_loading: HashSet::new(),
            refreshing: HashSet::new(),
            refresh_indicator_until: HashMap::new(),
            no_data: HashSet::new(),
            icon_cache: HashMap::new(),
            icon_pending: Arc::new(Mutex::new(Vec::new())),
            icon_loading: HashSet::new(),
            icon_failed: HashSet::new(),
            icon_reveal: HashMap::new(),
            icon_reveal_started_at: HashMap::new(),
            percent_reveal: HashMap::new(),
            percent_reveal_started_at: HashMap::new(),
            icon_scope_app_id: None,
            icon_generation: 0,
            text_reveal: HashMap::new(),
            text_reveal_started_at: HashMap::new(),
            checked_overview_for: None,
            checked_detail_for: None,
        }
    }
}

fn overview_from_summary(summary: &AchievementSummary) -> AchievementSummary {
    AchievementSummary {
        unlocked: summary.unlocked,
        total: summary.total,
        items: Vec::new(),
    }
}

fn preserve_missing_global_percents(
    summary: &mut AchievementSummary,
    previous_summary: &AchievementSummary,
) {
    let previous_percents: HashMap<&str, f32> = previous_summary
        .items
        .iter()
        .filter_map(|item| item.global_percent.map(|percent| (item.api_name.as_str(), percent)))
        .collect();

    for item in &mut summary.items {
        if item.global_percent.is_none() {
            if let Some(previous_percent) = previous_percents.get(item.api_name.as_str()) {
                item.global_percent = Some(*previous_percent);
            }
        }
    }
}