use eframe::egui;
use std::time::Instant;

use crate::animation;
use crate::game::Game;

use super::{AchievementState, HiddenRevealState};

impl AchievementState {
    pub fn reveal_hidden_description_for_selected(
        &mut self,
        selected_game: Option<&Game>,
        selected_index: usize,
    ) -> bool {
        let Some(steam_app_id) = selected_game.and_then(|game| game.steam_app_id) else {
            return false;
        };
        let Some(api_name) = self
            .detail_cache
            .as_ref()
            .filter(|(detail_app_id, _)| *detail_app_id == steam_app_id)
            .map(|(_, summary)| summary)
            .and_then(|summary| summary.items.get(selected_index))
            .filter(|item| item.is_hidden && item.unlocked != Some(true))
            .map(|item| item.api_name.clone())
        else {
            return false;
        };

        if self
            .revealed_hidden
            .get(&steam_app_id)
            .is_some_and(|state| state.api_name == api_name)
        {
            return false;
        }

        self.revealed_hidden.insert(
            steam_app_id,
            HiddenRevealState {
                api_name,
                progress: 0.0,
                started_at: Instant::now(),
            },
        );
        true
    }

    pub fn clear_revealed_hidden_for_selected(&mut self, selected_game: Option<&Game>) {
        if let Some(steam_app_id) = selected_game.and_then(|game| game.steam_app_id) {
            self.revealed_hidden.remove(&steam_app_id);
        }
    }

    pub fn revealed_hidden_for_selected(&self, selected_game: Option<&Game>) -> Option<&str> {
        selected_game
            .and_then(|game| game.steam_app_id)
            .and_then(|steam_app_id| {
                self.revealed_hidden
                    .get(&steam_app_id)
                    .map(|state| state.api_name.as_str())
            })
    }

    pub fn hidden_reveal_progress_for_selected(&self, selected_game: Option<&Game>) -> f32 {
        selected_game
            .and_then(|game| game.steam_app_id)
            .and_then(|steam_app_id| {
                self.revealed_hidden
                    .get(&steam_app_id)
                    .map(|state| state.progress)
            })
            .unwrap_or(0.0)
    }

    pub fn animate_reveals(&mut self, ctx: &egui::Context, now: Instant) {
        const ACHIEVEMENT_ICON_FADE_IN_SECONDS: f32 = 0.3;
        const ACHIEVEMENT_PERCENT_FADE_IN_SECONDS: f32 = 0.3;
        const HIDDEN_REVEAL_SECONDS: f32 = 0.24;
        const ACHIEVEMENT_TEXT_FADE_IN_SECONDS: f32 = 0.35;
        const ACHIEVEMENT_TEXT_FADE_OUT_SECONDS: f32 = 0.22;

        let mut finished_icon_reveals = Vec::new();
        for (url, progress) in &mut self.icon_reveal {
            let Some(started_at) = self.icon_reveal_started_at.get(url).copied() else {
                finished_icon_reveals.push(url.clone());
                continue;
            };

            *progress = animation::linear_progress(started_at, now, ACHIEVEMENT_ICON_FADE_IN_SECONDS);
            if *progress < 0.999 {
                ctx.request_repaint();
            } else {
                finished_icon_reveals.push(url.clone());
            }
        }
        for url in finished_icon_reveals {
            self.icon_reveal.remove(&url);
            self.icon_reveal_started_at.remove(&url);
        }

        let mut finished_percent_reveals = Vec::new();
        for (api_name, progress) in &mut self.percent_reveal {
            let Some(started_at) = self.percent_reveal_started_at.get(api_name).copied() else {
                finished_percent_reveals.push(api_name.clone());
                continue;
            };

            *progress =
                animation::linear_progress(started_at, now, ACHIEVEMENT_PERCENT_FADE_IN_SECONDS);
            if *progress < 0.999 {
                ctx.request_repaint();
            } else {
                finished_percent_reveals.push(api_name.clone());
            }
        }
        for api_name in finished_percent_reveals {
            self.percent_reveal.remove(&api_name);
            self.percent_reveal_started_at.remove(&api_name);
        }

        for state in self.revealed_hidden.values_mut() {
            state.progress = animation::linear_progress(state.started_at, now, HIDDEN_REVEAL_SECONDS);
            if state.progress < 0.999 {
                ctx.request_repaint();
            }
        }

        let mut finished_text_reveals = Vec::new();
        for (steam_app_id, progress) in &mut self.text_reveal {
            let Some(started_at) = self.text_reveal_started_at.get(steam_app_id).copied() else {
                continue;
            };

            *progress = animation::linear_progress(started_at, now, ACHIEVEMENT_TEXT_FADE_IN_SECONDS);
            if *progress < 0.999 {
                ctx.request_repaint();
            } else {
                *progress = 1.0;
                finished_text_reveals.push(*steam_app_id);
            }
        }
        for steam_app_id in finished_text_reveals {
            self.text_reveal_started_at.remove(&steam_app_id);
        }

        if let Some(started_at) = self.previous_overview_reveal_started_at {
            self.previous_overview_reveal =
                (1.0 - animation::linear_progress(started_at, now, ACHIEVEMENT_TEXT_FADE_OUT_SECONDS))
                    .max(0.0);
            if self.previous_overview_reveal > 0.001 {
                ctx.request_repaint();
            } else {
                self.previous_overview = None;
                self.previous_overview_reveal = 0.0;
                self.previous_overview_reveal_started_at = None;
            }
        }

        let refresh_now = Instant::now();
        self.refresh_indicator_until.retain(|_, until| {
            let keep = *until > refresh_now;
            if keep {
                ctx.request_repaint();
            }
            keep
        });
    }
}