use std::collections::HashMap;
use std::path::PathBuf;

use crate::i18n::AppLanguage;

use super::achievement_bvdf::load_local_unlock_status;
use super::achievement_cache::load_global_achievement_percentages;
use super::achievement_schema::{
    load_local_schema_achievement_names, load_local_schema_items, load_schema_metadata_maps,
};
use super::types::{AchievementItem, AchievementSummary};

fn compare_achievement_api_names(left: &str, right: &str) -> std::cmp::Ordering {
    match (left.parse::<u32>(), right.parse::<u32>()) {
        (Ok(left_number), Ok(right_number)) => left_number.cmp(&right_number),
        _ => left.cmp(right),
    }
}

fn compare_optional_group_keys(left: Option<&str>, right: Option<&str>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => compare_achievement_api_names(left, right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn compare_optional_bit_indices(left: Option<u32>, right: Option<u32>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

pub fn sort_achievement_items(items: &mut [AchievementItem]) {
    items.sort_by(|a, b| {
        compare_optional_group_keys(a.group_key.as_deref(), b.group_key.as_deref())
            .then_with(|| compare_optional_bit_indices(a.bit_index, b.bit_index))
            .then_with(|| compare_achievement_api_names(&a.api_name, &b.api_name))
    });
}

pub fn load_achievement_summary(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
    allow_global_percentage_refresh: bool,
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

    merge_schema_metadata(
        app_id,
        steam_paths,
        language,
        &mut items_map,
        allow_global_percentage_refresh,
    );

    let mut items: Vec<AchievementItem> = items_map.into_values().collect();
    sort_achievement_items(&mut items);

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
    allow_global_percentage_refresh: bool,
) {
    let schema_metadata = load_schema_metadata_maps(app_id, steam_paths, language);
    let global_percentages =
        load_global_achievement_percentages(app_id, allow_global_percentage_refresh);

    for item in items_map.values_mut() {
        if item.display_name.is_none() {
            if let Some(display_name) = schema_metadata.display_names.get(&item.api_name) {
                item.display_name = Some(display_name.clone());
            }
        }
        if item.description.is_none() {
            if let Some(description) = schema_metadata.descriptions.get(&item.api_name) {
                item.description = Some(description.clone());
            }
        }
        if !item.is_hidden {
            if let Some(info) = schema_metadata.hidden_flags.get(&item.api_name) {
                item.is_hidden = info.is_hidden;
            }
        }
        if item.icon_url.is_none() || item.icon_gray_url.is_none() {
            if let Some((icon, gray)) = schema_metadata.icon_urls.get(&item.api_name) {
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
        group_key: None,
        bit_index: None,
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
    use super::sort_achievement_items;
    use crate::steam::types::AchievementItem;

    #[test]
    fn achievement_sort_orders_by_group_then_bit() {
        let mut items = vec![
            AchievementItem {
                api_name: "33".to_string(),
                group_key: Some("2".to_string()),
                bit_index: Some(0),
                ..AchievementItem::default()
            },
            AchievementItem {
                api_name: "2".to_string(),
                group_key: Some("1".to_string()),
                bit_index: Some(1),
                ..AchievementItem::default()
            },
            AchievementItem {
                api_name: "1".to_string(),
                group_key: Some("1".to_string()),
                bit_index: Some(0),
                ..AchievementItem::default()
            },
            AchievementItem {
                api_name: "34".to_string(),
                group_key: Some("2".to_string()),
                bit_index: Some(1),
                ..AchievementItem::default()
            },
        ];

        sort_achievement_items(&mut items);

        let names: Vec<_> = items.iter().map(|item| item.api_name.as_str()).collect();
        assert_eq!(names, vec!["1", "2", "33", "34"]);
    }

    #[test]
    fn achievement_sort_uses_api_name_as_final_tiebreaker() {
        let mut items = vec![
            AchievementItem {
                api_name: "10".to_string(),
                group_key: Some("1".to_string()),
                bit_index: Some(3),
                ..AchievementItem::default()
            },
            AchievementItem {
                api_name: "2".to_string(),
                group_key: Some("1".to_string()),
                bit_index: Some(3),
                ..AchievementItem::default()
            },
            AchievementItem {
                api_name: "1".to_string(),
                group_key: Some("1".to_string()),
                bit_index: Some(3),
                ..AchievementItem::default()
            },
        ];

        sort_achievement_items(&mut items);

        let names: Vec<_> = items.into_iter().map(|item| item.api_name).collect();
        assert_eq!(names, vec!["1", "2", "10"]);
    }

    #[test]
    fn achievement_sort_places_missing_group_after_grouped_items() {
        let mut items = vec![
            AchievementItem {
                api_name: "ungrouped".to_string(),
                ..AchievementItem::default()
            },
            AchievementItem {
                api_name: "1".to_string(),
                group_key: Some("1".to_string()),
                bit_index: Some(0),
                ..AchievementItem::default()
            },
        ];

        sort_achievement_items(&mut items);

        let names: Vec<_> = items.into_iter().map(|item| item.api_name).collect();
        assert_eq!(names, vec!["1", "ungrouped"]);
    }
}