use std::collections::HashMap;
use std::path::PathBuf;

use crate::i18n::AppLanguage;

use super::achievement_bvdf::load_local_unlock_status;
use super::achievement_cache::load_global_achievement_percentages;
use super::achievement_schema::{
    load_local_schema_achievement_names, load_local_schema_items,
    load_schema_achievement_metadata, load_schema_descriptions,
    load_schema_display_names, load_schema_icon_urls,
};
use super::types::{AchievementItem, AchievementSummary};

fn achievement_unlock_sort_rank(unlocked: Option<bool>) -> u8 {
    match unlocked {
        Some(true) => 0,
        Some(false) => 1,
        None => 2,
    }
}

fn achievement_percent_sort_value(global_percent: Option<f32>) -> Option<f32> {
    match global_percent {
        Some(value) if value.is_finite() => Some(value),
        _ => None,
    }
}

pub fn sort_achievement_items(items: &mut [AchievementItem], descending: bool) {
    items.sort_by(|a, b| {
        let a_percent = achievement_percent_sort_value(a.global_percent);
        let b_percent = achievement_percent_sort_value(b.global_percent);

        a_percent
            .is_none()
            .cmp(&b_percent.is_none())
            .then_with(|| match (a_percent, b_percent) {
                (Some(a_percent), Some(b_percent)) if descending => b_percent.total_cmp(&a_percent),
                (Some(a_percent), Some(b_percent)) => a_percent.total_cmp(&b_percent),
                _ => std::cmp::Ordering::Equal,
            })
            .then_with(|| {
                achievement_unlock_sort_rank(a.unlocked)
                    .cmp(&achievement_unlock_sort_rank(b.unlocked))
            })
            .then_with(|| {
                a.display_name
                    .as_deref()
                    .unwrap_or(&a.api_name)
                    .cmp(b.display_name.as_deref().unwrap_or(&b.api_name))
            })
            .then_with(|| a.api_name.cmp(&b.api_name))
    });
}

pub fn load_achievement_summary(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> Option<AchievementSummary> {
    let local_unlocks = load_local_unlock_status(app_id, steam_paths, language);
    let mut items_map = load_local_schema_items(app_id, steam_paths, language);

    let mut unlocked_count: Option<u32> = None;
    if !local_unlocks.is_empty() {
        let mut count = 0u32;
        merge_unlocks_into_items(&local_unlocks, &mut items_map, &mut count);
        unlocked_count = Some(count);
    }

    if items_map.is_empty() {
        backfill_schema_names(app_id, steam_paths, language, &mut items_map);
    }

    if items_map.is_empty() {
        return None;
    }

    merge_schema_metadata(app_id, steam_paths, language, &mut items_map);

    let items: Vec<AchievementItem> = items_map.into_values().collect();

    Some(AchievementSummary {
        unlocked: unlocked_count,
        total: items.len() as u32,
        items,
    })
}

fn merge_unlocks_into_items(
    local_unlocks: &HashMap<String, (bool, Option<u64>)>,
    items_map: &mut HashMap<String, AchievementItem>,
    unlocked_count: &mut u32,
) {
    for (api_name, (achieved, time)) in local_unlocks {
        let item = items_map
            .entry(api_name.clone())
            .or_insert_with(|| empty_achievement_item(api_name.clone()));
        item.unlocked = Some(*achieved);
        item.unlock_time = *time;
        if *achieved {
            *unlocked_count += 1;
        }
    }
}

fn backfill_schema_names(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
    items_map: &mut HashMap<String, AchievementItem>,
) {
    let names = load_local_schema_achievement_names(app_id, steam_paths, language);
    for name in names {
        items_map.insert(name.clone(), empty_achievement_item(name));
    }
}

fn merge_schema_metadata(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
    items_map: &mut HashMap<String, AchievementItem>,
) {
    let display_names = load_schema_display_names(app_id, steam_paths, language);
    let descriptions = load_schema_descriptions(app_id, steam_paths, language);
    let hidden_flags = load_schema_achievement_metadata(app_id, steam_paths, language);
    let schema_icons = load_schema_icon_urls(app_id, steam_paths, language);
    let global_percentages = load_global_achievement_percentages(app_id);

    for item in items_map.values_mut() {
        if item.display_name.is_none() {
            if let Some(display_name) = display_names.get(&item.api_name) {
                item.display_name = Some(display_name.clone());
            }
        }
        if item.description.is_none() {
            if let Some(description) = descriptions.get(&item.api_name) {
                item.description = Some(description.clone());
            }
        }
        if !item.is_hidden {
            if let Some(info) = hidden_flags.get(&item.api_name) {
                item.is_hidden = info.is_hidden;
            }
        }
        if item.icon_url.is_none() || item.icon_gray_url.is_none() {
            if let Some((icon, gray)) = schema_icons.get(&item.api_name) {
                if item.icon_url.is_none() {
                    item.icon_url = Some(icon.clone());
                }
                if item.icon_gray_url.is_none() {
                    item.icon_gray_url = Some(gray.clone());
                }
            }
        }
        if let Some(percent) = global_percentages.get(&item.api_name) {
            item.global_percent = Some(*percent);
        }
    }
}

fn empty_achievement_item(api_name: String) -> AchievementItem {
    AchievementItem {
        api_name,
        display_name: None,
        description: None,
        is_hidden: false,
        unlocked: None,
        unlock_time: None,
        global_percent: None,
        icon_url: None,
        icon_gray_url: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{achievement_percent_sort_value, sort_achievement_items};
    use crate::steam::types::AchievementItem;

    #[test]
    fn achievement_sort_defaults_to_highest_unlock_rate_first() {
        let mut items = vec![
            AchievementItem {
                api_name: "unknown".to_string(),
                unlocked: None,
                global_percent: Some(2.0),
                ..AchievementItem::default()
            },
            AchievementItem {
                api_name: "unlocked".to_string(),
                unlocked: Some(true),
                global_percent: Some(1.0),
                ..AchievementItem::default()
            },
            AchievementItem {
                api_name: "locked".to_string(),
                unlocked: Some(false),
                global_percent: Some(f32::NAN),
                ..AchievementItem::default()
            },
        ];

        sort_achievement_items(&mut items, true);

        let names: Vec<_> = items.iter().map(|item| item.api_name.as_str()).collect();
        assert_eq!(names, vec!["unknown", "unlocked", "locked"]);
    }

    #[test]
    fn achievement_sort_can_switch_to_lowest_unlock_rate_first() {
        let mut items = vec![
            AchievementItem {
                api_name: "rare".to_string(),
                global_percent: Some(1.5),
                ..AchievementItem::default()
            },
            AchievementItem {
                api_name: "common".to_string(),
                global_percent: Some(62.5),
                ..AchievementItem::default()
            },
            AchievementItem {
                api_name: "unknown".to_string(),
                global_percent: None,
                ..AchievementItem::default()
            },
        ];

        sort_achievement_items(&mut items, false);

        let names: Vec<_> = items.into_iter().map(|item| item.api_name).collect();
        assert_eq!(names, vec!["rare", "common", "unknown"]);
    }

    #[test]
    fn achievement_percent_sort_value_filters_invalid_values() {
        assert_eq!(achievement_percent_sort_value(Some(18.2)), Some(18.2));
        assert_eq!(achievement_percent_sort_value(Some(f32::NAN)), None);
        assert_eq!(achievement_percent_sort_value(None), None);
    }
}