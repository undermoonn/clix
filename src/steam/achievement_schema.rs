use std::collections::HashMap;
use std::path::PathBuf;

use crate::i18n::AppLanguage;

use super::achievement_bvdf::{parse_bvdf_at, BvdfVal};
use super::types::AchievementItem;

#[derive(Clone)]
pub(super) struct SchemaAchInfo {
    pub(super) api_name: String,
    pub(super) display_name: Option<String>,
    pub(super) description: Option<String>,
    pub(super) is_hidden: bool,
    pub(super) icon_url: Option<String>,
    pub(super) icon_gray_url: Option<String>,
}

pub(super) fn load_schema_achievement_bits(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> HashMap<String, Vec<(u32, SchemaAchInfo)>> {
    for steam_root in steam_paths {
        let schema_path = steam_root
            .join("appcache")
            .join("stats")
            .join(format!("UserGameStatsSchema_{}.bin", app_id));
        let Ok(data) = std::fs::read(&schema_path) else {
            continue;
        };

        for &start in &[0usize, 2, 4, 8] {
            if start >= data.len() {
                break;
            }
            let root = parse_bvdf_at(&data, start);
            let mut result: HashMap<String, Vec<(u32, SchemaAchInfo)>> = HashMap::new();
            collect_achievement_bits(&root, &mut result, 0, language);
            if !result.is_empty() {
                return result;
            }
        }
    }

    HashMap::new()
}

pub(super) fn load_schema_achievement_metadata(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> HashMap<String, SchemaAchInfo> {
    for steam_root in steam_paths {
        let schema_path = steam_root
            .join("appcache")
            .join("stats")
            .join(format!("UserGameStatsSchema_{}.bin", app_id));
        let Ok(data) = std::fs::read(&schema_path) else {
            continue;
        };

        for &start in &[0usize, 2, 4, 8] {
            if start >= data.len() {
                break;
            }
            let root = parse_bvdf_at(&data, start);
            let mut map: HashMap<String, SchemaAchInfo> = HashMap::new();
            collect_schema_achievement_metadata(&root, &mut map, 0, language);
            if !map.is_empty() {
                return map;
            }
        }
    }

    HashMap::new()
}

pub(super) fn load_schema_display_names(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> HashMap<String, String> {
    let bits = load_schema_achievement_bits(app_id, steam_paths, language);
    let metadata = load_schema_achievement_metadata(app_id, steam_paths, language);
    let mut map = HashMap::new();

    for entries in bits.values() {
        for (_, info) in entries {
            if let Some(display_name) = &info.display_name {
                map.insert(info.api_name.clone(), display_name.clone());
            }
        }
    }

    for (api_name, info) in metadata {
        if let Some(display_name) = info.display_name {
            map.entry(api_name).or_insert(display_name);
        }
    }

    map
}

pub(super) fn load_schema_descriptions(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> HashMap<String, String> {
    let bits = load_schema_achievement_bits(app_id, steam_paths, language);
    let metadata = load_schema_achievement_metadata(app_id, steam_paths, language);
    let mut map = HashMap::new();

    for entries in bits.values() {
        for (_, info) in entries {
            if let Some(description) = &info.description {
                map.insert(info.api_name.clone(), description.clone());
            }
        }
    }

    for (api_name, info) in metadata {
        if let Some(description) = info.description {
            map.entry(api_name).or_insert(description);
        }
    }

    map
}

pub(super) fn load_schema_icon_urls(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> HashMap<String, (String, String)> {
    let bits = load_schema_achievement_bits(app_id, steam_paths, language);
    let metadata = load_schema_achievement_metadata(app_id, steam_paths, language);
    let mut map = HashMap::new();

    for entries in bits.values() {
        for (_, info) in entries {
            let icon = info
                .icon_url
                .as_deref()
                .and_then(|value| normalize_schema_icon_value(app_id, value));
            let icon_gray = info
                .icon_gray_url
                .as_deref()
                .and_then(|value| normalize_schema_icon_value(app_id, value));
            match (icon, icon_gray) {
                (Some(icon), Some(icon_gray)) => {
                    map.insert(info.api_name.clone(), (icon, icon_gray));
                }
                (Some(icon), None) => {
                    map.insert(info.api_name.clone(), (icon.clone(), icon));
                }
                (None, Some(icon_gray)) => {
                    map.insert(info.api_name.clone(), (icon_gray.clone(), icon_gray));
                }
                _ => {}
            }
        }
    }

    for (api_name, info) in metadata {
        let icon = info
            .icon_url
            .as_deref()
            .and_then(|value| normalize_schema_icon_value(app_id, value));
        let icon_gray = info
            .icon_gray_url
            .as_deref()
            .and_then(|value| normalize_schema_icon_value(app_id, value));
        match (icon, icon_gray) {
            (Some(icon), Some(icon_gray)) => {
                map.entry(api_name).or_insert((icon, icon_gray));
            }
            (Some(icon), None) => {
                map.entry(api_name).or_insert((icon.clone(), icon));
            }
            (None, Some(icon_gray)) => {
                map.entry(api_name).or_insert((icon_gray.clone(), icon_gray));
            }
            _ => {}
        }
    }

    map
}

pub(super) fn load_local_schema_achievement_names(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let is_language_key = |s: &str| {
        matches!(
            s.to_ascii_lowercase().as_str(),
            "english"
                | "schinese"
                | "tchinese"
                | "german"
                | "french"
                | "italian"
                | "koreana"
                | "spanish"
                | "russian"
                | "japanese"
                | "portuguese"
                | "brazilian"
                | "latam"
                | "polish"
                | "danish"
                | "dutch"
                | "finnish"
                | "norwegian"
                | "swedish"
                | "turkish"
                | "thai"
                | "arabic"
        )
    };

    for steam_root in steam_paths {
        let schema_path = steam_root
            .join("appcache")
            .join("stats")
            .join(format!("UserGameStatsSchema_{}.bin", app_id));
        let Ok(bytes) = std::fs::read(&schema_path) else {
            continue;
        };

        let text = String::from_utf8_lossy(&bytes);
        let tokens: Vec<String> = text
            .split('\0')
            .map(|token| {
                token
                    .trim_matches(|c: char| c.is_ascii_control() || c.is_whitespace())
                    .to_string()
            })
            .collect();

        for index in 0..tokens.len() {
            let token = tokens[index].trim();
            if !token.starts_with("NEW_ACHIEVEMENT_") || !token.ends_with("_NAME") {
                continue;
            }

            let mut zh_name: Option<String> = None;
            let mut en_name: Option<String> = None;
            let mut internal_name: Option<String> = None;

            let start = index.saturating_sub(24);
            let end = (index + 24).min(tokens.len().saturating_sub(1));

            for scan_index in (start..index).rev() {
                let key = tokens[scan_index].trim();
                if key.eq_ignore_ascii_case("display") {
                    break;
                }

                if key.eq_ignore_ascii_case("schinese") && scan_index + 1 < index {
                    let value = tokens[scan_index + 1].trim();
                    if !value.is_empty() {
                        zh_name = Some(value.to_string());
                    }
                } else if key.eq_ignore_ascii_case("english") && scan_index + 1 < index {
                    let value = tokens[scan_index + 1].trim();
                    if !value.is_empty() {
                        en_name = Some(value.to_string());
                    }
                } else if key.eq_ignore_ascii_case("name") && scan_index + 1 < index {
                    let next = tokens[scan_index + 1].trim();
                    if next.eq_ignore_ascii_case("schinese") && scan_index + 2 < index {
                        let value = tokens[scan_index + 2].trim();
                        if !value.is_empty() && !value.starts_with("NEW_ACHIEVEMENT_") {
                            zh_name = Some(value.to_string());
                        }
                    } else if next.eq_ignore_ascii_case("english") && scan_index + 2 < index {
                        let value = tokens[scan_index + 2].trim();
                        if !value.is_empty() && !value.starts_with("NEW_ACHIEVEMENT_") {
                            en_name = Some(value.to_string());
                        }
                    } else if !next.is_empty()
                        && !next.eq_ignore_ascii_case("display")
                        && !is_language_key(next)
                        && !next.starts_with("NEW_ACHIEVEMENT_")
                    {
                        internal_name = Some(next.to_string());
                    }
                }
            }

            for scan_index in (index + 1)..=end {
                let key = tokens[scan_index].trim();
                if key.eq_ignore_ascii_case("desc")
                    || key.eq_ignore_ascii_case("hidden")
                    || key.eq_ignore_ascii_case("icon")
                    || key.eq_ignore_ascii_case("icon_gray")
                    || key.eq_ignore_ascii_case("bit")
                {
                    break;
                }

                if key.eq_ignore_ascii_case("schinese") && scan_index < end {
                    let value = tokens[scan_index + 1].trim();
                    if !value.is_empty() {
                        zh_name = Some(value.to_string());
                    }
                } else if key.eq_ignore_ascii_case("english") && scan_index < end {
                    let value = tokens[scan_index + 1].trim();
                    if !value.is_empty() {
                        en_name = Some(value.to_string());
                    }
                }
            }

            let localized_name = match language {
                AppLanguage::SimplifiedChinese => zh_name.or(en_name),
                AppLanguage::English => en_name,
            };

            if let Some(name) = localized_name.or(internal_name) {
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }

        if !names.is_empty() {
            return names;
        }
    }

    names
}

pub(super) fn load_local_schema_items(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> HashMap<String, AchievementItem> {
    let bits = load_schema_achievement_bits(app_id, steam_paths, language);
    let metadata = load_schema_achievement_metadata(app_id, steam_paths, language);
    let mut items_map = HashMap::new();

    for entries in bits.values() {
        for (_, info) in entries {
            let item = items_map
                .entry(info.api_name.clone())
                .or_insert_with(|| AchievementItem {
                    api_name: info.api_name.clone(),
                    display_name: None,
                    description: None,
                    is_hidden: false,
                    unlocked: None,
                    unlock_time: None,
                    global_percent: None,
                    icon_url: None,
                    icon_gray_url: None,
                });
            if item.display_name.is_none() {
                item.display_name = info.display_name.clone();
            }
            if item.description.is_none() {
                item.description = info.description.clone();
            }
            if !item.is_hidden {
                item.is_hidden = info.is_hidden;
            }
            if item.icon_url.is_none() {
                item.icon_url = info
                    .icon_url
                    .as_deref()
                    .and_then(|value| normalize_schema_icon_value(app_id, value));
            }
            if item.icon_gray_url.is_none() {
                item.icon_gray_url = info
                    .icon_gray_url
                    .as_deref()
                    .and_then(|value| normalize_schema_icon_value(app_id, value));
            }
        }
    }

    for (api_name, info) in metadata {
        let item = items_map
            .entry(api_name.clone())
            .or_insert_with(|| AchievementItem {
                api_name,
                display_name: None,
                description: None,
                is_hidden: false,
                unlocked: None,
                unlock_time: None,
                global_percent: None,
                icon_url: None,
                icon_gray_url: None,
            });
        if item.display_name.is_none() {
            item.display_name = info.display_name;
        }
        if item.description.is_none() {
            item.description = info.description;
        }
        if !item.is_hidden {
            item.is_hidden = info.is_hidden;
        }
        if item.icon_url.is_none() {
            item.icon_url = info
                .icon_url
                .as_deref()
                .and_then(|value| normalize_schema_icon_value(app_id, value));
        }
        if item.icon_gray_url.is_none() {
            item.icon_gray_url = info
                .icon_gray_url
                .as_deref()
                .and_then(|value| normalize_schema_icon_value(app_id, value));
        }
    }

    items_map
}

fn collect_schema_achievement_metadata(
    node: &HashMap<String, BvdfVal>,
    out: &mut HashMap<String, SchemaAchInfo>,
    depth: u32,
    language: AppLanguage,
) {
    if depth > 8 {
        return;
    }

    for val in node.values() {
        if let BvdfVal::Nested(inner) = val {
            let api_name = inner
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("name"))
                .and_then(|(_, v)| if let BvdfVal::Str(s) = v { Some(s.clone()) } else { None });

            if let Some(api_name) = api_name {
                let display_name = extract_display_name(inner, language);
                let description = extract_description(inner, language);
                let is_hidden = extract_hidden_flag(inner);
                let (icon_url, icon_gray_url) = extract_icon_urls(inner);
                if display_name.is_some()
                    || description.is_some()
                    || is_hidden
                    || icon_url.is_some()
                    || icon_gray_url.is_some()
                {
                    let entry = out.entry(api_name.clone()).or_insert_with(|| SchemaAchInfo {
                        api_name: api_name.clone(),
                        display_name: None,
                        description: None,
                        is_hidden: false,
                        icon_url: None,
                        icon_gray_url: None,
                    });
                    if entry.display_name.is_none() {
                        entry.display_name = display_name;
                    }
                    if entry.description.is_none() {
                        entry.description = description;
                    }
                    if !entry.is_hidden {
                        entry.is_hidden = is_hidden;
                    }
                    if entry.icon_url.is_none() {
                        entry.icon_url = icon_url;
                    }
                    if entry.icon_gray_url.is_none() {
                        entry.icon_gray_url = icon_gray_url;
                    }
                }
            }

            collect_schema_achievement_metadata(inner, out, depth + 1, language);
        }
    }
}

fn collect_achievement_bits(
    node: &HashMap<String, BvdfVal>,
    result: &mut HashMap<String, Vec<(u32, SchemaAchInfo)>>,
    depth: u32,
    language: AppLanguage,
) {
    if depth > 6 {
        return;
    }

    for (key, val) in node {
        if let BvdfVal::Nested(inner) = val {
            if let Some(BvdfVal::Nested(bits)) = inner
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("bits"))
                .map(|(_, v)| v)
            {
                let mut entries: Vec<(u32, SchemaAchInfo)> = Vec::new();
                for (bit_key, bit_val) in bits {
                    if let (Ok(bit_idx), BvdfVal::Nested(fields)) = (bit_key.parse::<u32>(), bit_val)
                    {
                        if let Some(BvdfVal::Str(api_name)) = fields
                            .iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case("name"))
                            .map(|(_, v)| v)
                        {
                            if !api_name.is_empty() {
                                let display_name = extract_display_name(fields, language);
                                let description = extract_description(fields, language);
                                let is_hidden = extract_hidden_flag(fields);
                                let (icon_url, icon_gray_url) = extract_icon_urls(fields);
                                entries.push((
                                    bit_idx,
                                    SchemaAchInfo {
                                        api_name: api_name.clone(),
                                        display_name,
                                        description,
                                        is_hidden,
                                        icon_url,
                                        icon_gray_url,
                                    },
                                ));
                            }
                        }
                    }
                }
                if !entries.is_empty() {
                    entries.sort_by_key(|(idx, _)| *idx);
                    result.insert(key.clone(), entries);
                }
            }

            collect_achievement_bits(inner, result, depth + 1, language);
        }
    }
}

fn extract_localized_nested_string(
    node: &HashMap<String, BvdfVal>,
    language: AppLanguage,
) -> Option<String> {
    let preferred = language.steam_language_key();

    if let Some(BvdfVal::Str(value)) = node
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(preferred))
        .map(|(_, value)| value)
    {
        if !value.is_empty() {
            return Some(value.clone());
        }
    }

    if !preferred.eq_ignore_ascii_case("english") {
        if let Some(BvdfVal::Str(value)) = node
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case("english"))
            .map(|(_, value)| value)
        {
            if !value.is_empty() {
                return Some(value.clone());
            }
        }
    }

    None
}

fn extract_display_name(fields: &HashMap<String, BvdfVal>, language: AppLanguage) -> Option<String> {
    let display = fields
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("display"))
        .and_then(|(_, v)| if let BvdfVal::Nested(d) = v { Some(d) } else { None })?;
    let name_node = display
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("name"))
        .and_then(|(_, v)| if let BvdfVal::Nested(n) = v { Some(n) } else { None })?;

    extract_localized_nested_string(name_node, language)
}

fn extract_description(fields: &HashMap<String, BvdfVal>, language: AppLanguage) -> Option<String> {
    let display = fields
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("display"))
        .and_then(|(_, v)| if let BvdfVal::Nested(d) = v { Some(d) } else { None })?;

    let desc_node = display
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("desc") || k.eq_ignore_ascii_case("description"))
        .and_then(|(_, v)| if let BvdfVal::Nested(n) = v { Some(n) } else { None })?;

    extract_localized_nested_string(desc_node, language)
}

fn extract_hidden_flag(fields: &HashMap<String, BvdfVal>) -> bool {
    fn parse_hidden_value(value: &BvdfVal) -> bool {
        match value {
            BvdfVal::Int32(number) => *number != 0,
            BvdfVal::Uint64(number) => *number != 0,
            BvdfVal::Str(text) => matches!(
                text.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            ),
            BvdfVal::Nested(_) => false,
        }
    }

    if let Some(value) = fields
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("hidden"))
        .map(|(_, value)| value)
    {
        return parse_hidden_value(value);
    }

    fields
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("display"))
        .and_then(|(_, value)| match value {
            BvdfVal::Nested(display) => display
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("hidden"))
                .map(|(_, value)| parse_hidden_value(value)),
            _ => None,
        })
        .unwrap_or(false)
}

fn normalize_schema_icon_value(app_id: u32, raw: &str) -> Option<String> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }
    if value.starts_with("http://") || value.starts_with("https://") {
        return Some(value.to_string());
    }
    if value.ends_with(".jpg") || value.ends_with(".png") {
        return Some(format!(
            "https://cdn.cloudflare.steamstatic.com/steamcommunity/public/images/apps/{}/{}",
            app_id,
            value
        ));
    }
    Some(format!(
        "https://cdn.cloudflare.steamstatic.com/steamcommunity/public/images/apps/{}/{}.jpg",
        app_id,
        value
    ))
}

fn extract_icon_urls(fields: &HashMap<String, BvdfVal>) -> (Option<String>, Option<String>) {
    let mut icon = fields
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("icon"))
        .and_then(|(_, v)| if let BvdfVal::Str(s) = v { Some(s.clone()) } else { None });

    let mut icon_gray = fields
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("icon_gray") || k.eq_ignore_ascii_case("icongray"))
        .and_then(|(_, v)| if let BvdfVal::Str(s) = v { Some(s.clone()) } else { None });

    if icon.is_none() || icon_gray.is_none() {
        if let Some(BvdfVal::Nested(display)) = fields
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("display"))
            .map(|(_, v)| v)
        {
            if icon.is_none() {
                icon = display
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("icon"))
                    .and_then(|(_, v)| if let BvdfVal::Str(s) = v { Some(s.clone()) } else { None });
            }
            if icon_gray.is_none() {
                icon_gray = display
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("icon_gray") || k.eq_ignore_ascii_case("icongray"))
                    .and_then(|(_, v)| if let BvdfVal::Str(s) = v { Some(s.clone()) } else { None });
            }
        }
    }

    (icon, icon_gray)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        collect_achievement_bits, extract_description, extract_display_name,
        extract_hidden_flag, extract_icon_urls, normalize_schema_icon_value, BvdfVal,
    };
    use crate::i18n::AppLanguage;

    fn string_value(value: &str) -> BvdfVal {
        BvdfVal::Str(value.to_string())
    }

    fn nested(entries: Vec<(&str, BvdfVal)>) -> BvdfVal {
        let mut map = HashMap::new();
        for (key, value) in entries {
            map.insert(key.to_string(), value);
        }
        BvdfVal::Nested(map)
    }

    fn display_fields_with_localized_name() -> HashMap<String, BvdfVal> {
        let mut fields = HashMap::new();
        fields.insert(
            "display".to_string(),
            nested(vec![
                (
                    "name",
                    nested(vec![
                        ("english", string_value("English Name")),
                        ("schinese", string_value("中文名称")),
                    ]),
                ),
                (
                    "desc",
                    nested(vec![("english", string_value("English Desc"))]),
                ),
            ]),
        );
        fields
    }

    #[test]
    fn extract_display_name_prefers_requested_language() {
        let fields = display_fields_with_localized_name();

        assert_eq!(
            extract_display_name(&fields, AppLanguage::SimplifiedChinese),
            Some("中文名称".to_string())
        );
        assert_eq!(
            extract_display_name(&fields, AppLanguage::English),
            Some("English Name".to_string())
        );
    }

    #[test]
    fn extract_description_falls_back_to_english() {
        let fields = display_fields_with_localized_name();

        assert_eq!(
            extract_description(&fields, AppLanguage::SimplifiedChinese),
            Some("English Desc".to_string())
        );
    }

    #[test]
    fn extract_hidden_flag_supports_top_level_and_display_values() {
        let mut top_level_hidden = HashMap::new();
        top_level_hidden.insert("hidden".to_string(), BvdfVal::Int32(1));
        assert!(extract_hidden_flag(&top_level_hidden));

        let mut display_hidden = HashMap::new();
        display_hidden.insert(
            "display".to_string(),
            nested(vec![("hidden", string_value("true"))]),
        );
        assert!(extract_hidden_flag(&display_hidden));
    }

    #[test]
    fn extract_icon_urls_reads_nested_display_icons_and_normalizes_hashes() {
        let mut fields = HashMap::new();
        fields.insert(
            "display".to_string(),
            nested(vec![
                ("icon", string_value("icon_hash")),
                ("icon_gray", string_value("gray_hash")),
            ]),
        );

        let (icon, icon_gray) = extract_icon_urls(&fields);
        assert_eq!(icon.as_deref(), Some("icon_hash"));
        assert_eq!(icon_gray.as_deref(), Some("gray_hash"));

        assert_eq!(
            normalize_schema_icon_value(480, icon.as_deref().unwrap()),
            Some(
                "https://cdn.cloudflare.steamstatic.com/steamcommunity/public/images/apps/480/icon_hash.jpg"
                    .to_string()
            )
        );
        assert_eq!(
            normalize_schema_icon_value(480, "badge.png"),
            Some(
                "https://cdn.cloudflare.steamstatic.com/steamcommunity/public/images/apps/480/badge.png"
                    .to_string()
            )
        );
    }

    #[test]
    fn collect_achievement_bits_extracts_localized_metadata() {
        let mut result = HashMap::new();
        let mut root = HashMap::new();
        root.insert(
            "STAT_BITS".to_string(),
            nested(vec![(
                "bits",
                nested(vec![(
                    "2",
                    nested(vec![
                        ("name", string_value("ACH_TEST")),
                        (
                            "display",
                            nested(vec![
                                (
                                    "name",
                                    nested(vec![
                                        ("english", string_value("English Name")),
                                        ("schinese", string_value("中文名称")),
                                    ]),
                                ),
                                (
                                    "description",
                                    nested(vec![
                                        ("english", string_value("English Description")),
                                        ("schinese", string_value("中文描述")),
                                    ]),
                                ),
                                ("hidden", string_value("1")),
                                ("icon", string_value("icon_hash")),
                                ("icon_gray", string_value("gray_hash")),
                            ]),
                        ),
                    ]),
                )])
            )]),
        );

        collect_achievement_bits(&root, &mut result, 0, AppLanguage::SimplifiedChinese);

        let entries = result.get("STAT_BITS").expect("bit section should exist");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, 2);
        assert_eq!(entries[0].1.api_name, "ACH_TEST");
        assert_eq!(entries[0].1.display_name.as_deref(), Some("中文名称"));
        assert_eq!(entries[0].1.description.as_deref(), Some("中文描述"));
        assert!(entries[0].1.is_hidden);
        assert_eq!(entries[0].1.icon_url.as_deref(), Some("icon_hash"));
        assert_eq!(entries[0].1.icon_gray_url.as_deref(), Some("gray_hash"));
    }
}