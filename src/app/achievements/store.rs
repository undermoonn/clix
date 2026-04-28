use std::collections::HashMap;
use std::time::Instant;

use crate::steam::AchievementSummary;

use super::{overview_from_summary, AchievementState};

impl AchievementState {
    pub fn sync_summary_scope(&mut self, steam_app_id: Option<u32>) {
        if self.displayed_overview_app_id != steam_app_id {
            if let Some(previous_app_id) = self.displayed_overview_app_id {
                if let Some(previous_summary) = self.overview_cache.get(&previous_app_id) {
                    if previous_summary.total > 0 {
                        self.previous_overview = Some(previous_summary.clone());
                        self.previous_overview_reveal = 1.0;
                        self.previous_overview_reveal_started_at = Some(Instant::now());
                    } else {
                        self.previous_overview = None;
                        self.previous_overview_reveal = 0.0;
                        self.previous_overview_reveal_started_at = None;
                    }
                } else {
                    self.previous_overview = None;
                    self.previous_overview_reveal = 0.0;
                    self.previous_overview_reveal_started_at = None;
                }
            } else {
                self.previous_overview = None;
                self.previous_overview_reveal = 0.0;
                self.previous_overview_reveal_started_at = None;
            }
            self.displayed_overview_app_id = steam_app_id;
        }
        if steam_app_id.is_none() {
            self.displayed_overview_app_id = None;
            self.checked_overview_for = None;
        }
    }

    pub fn sync_detail_scope(&mut self, steam_app_id: Option<u32>) {
        if self
            .detail_cache
            .as_ref()
            .is_some_and(|(detail_app_id, _)| Some(*detail_app_id) != steam_app_id)
        {
            self.detail_cache = None;
            self.percent_reveal.clear();
            self.percent_reveal_started_at.clear();
        }
        if steam_app_id.is_none() {
            self.checked_detail_for = None;
            self.percent_reveal.clear();
            self.percent_reveal_started_at.clear();
        }
    }

    pub(super) fn reset_icon_scope(&mut self, steam_app_id: Option<u32>) {
        if self.icon_scope_app_id == steam_app_id {
            return;
        }

        self.icon_scope_app_id = steam_app_id;
        self.icon_generation = self.icon_generation.wrapping_add(1);
        self.icon_cache.clear();
        self.icon_loading.clear();
        self.icon_failed.clear();
        self.icon_reveal.clear();
        self.icon_reveal_started_at.clear();
    }

    pub(super) fn store_overview_summary(
        &mut self,
        steam_app_id: u32,
        summary: AchievementSummary,
        animate_reveal: bool,
    ) {
        let had_summary = self.overview_cache.contains_key(&steam_app_id);
        self.overview_cache.insert(steam_app_id, summary);
        if animate_reveal && !had_summary {
            self.text_reveal.insert(steam_app_id, 0.0);
            self.text_reveal_started_at
                .insert(steam_app_id, Instant::now());
        } else {
            self.text_reveal.insert(steam_app_id, 1.0);
            self.text_reveal_started_at.remove(&steam_app_id);
        }
    }

    pub(super) fn store_detail_summary(
        &mut self,
        steam_app_id: u32,
        summary: AchievementSummary,
        animate_reveal: bool,
    ) {
        let previous_percents: HashMap<String, f32> = self
            .detail_cache
            .as_ref()
            .filter(|(detail_app_id, _)| *detail_app_id == steam_app_id)
            .map(|(_, detail)| {
                detail
                    .items
                    .iter()
                    .filter_map(|item| {
                        item.global_percent
                            .map(|percent| (item.api_name.clone(), percent))
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.update_percent_reveal(&summary, &previous_percents, animate_reveal);
        let overview = overview_from_summary(&summary);
        self.detail_cache = Some((steam_app_id, summary));
        self.store_overview_summary(steam_app_id, overview, animate_reveal);
    }

    pub(super) fn update_percent_reveal(
        &mut self,
        summary: &AchievementSummary,
        previous_percents: &HashMap<String, f32>,
        animate_reveal: bool,
    ) {
        let now = Instant::now();
        let mut next_reveal = HashMap::new();
        let mut next_started_at = HashMap::new();
        for item in &summary.items {
            let Some(_) = item.global_percent else {
                continue;
            };

            let progress = if previous_percents.contains_key(&item.api_name) {
                self.percent_reveal
                    .get(&item.api_name)
                    .copied()
                    .unwrap_or(1.0)
            } else if animate_reveal {
                0.0
            } else {
                1.0
            };
            next_reveal.insert(item.api_name.clone(), progress);

            if progress < 0.999 {
                let started_at = self
                    .percent_reveal_started_at
                    .get(&item.api_name)
                    .copied()
                    .unwrap_or(now);
                next_started_at.insert(item.api_name.clone(), started_at);
            }
        }

        self.percent_reveal = next_reveal;
        self.percent_reveal_started_at = next_started_at;
    }
}
