use std::collections::HashMap;
use std::path::PathBuf;

use crate::i18n::AppLanguage;

use super::achievement_schema::{load_schema_achievement_bits, SchemaAchInfo};
use super::library::{find_most_recent_steam_id, steamid64_to_accountid};

pub(super) enum BvdfVal {
    Nested(HashMap<String, BvdfVal>),
    Str(String),
    Int32(i32),
    Uint64(u64),
}

fn bvdf_cstr(data: &[u8], pos: &mut usize) -> Option<String> {
    let start = *pos;
    while *pos < data.len() && data[*pos] != 0 {
        *pos += 1;
    }
    if *pos >= data.len() {
        return None;
    }
    let s = String::from_utf8_lossy(&data[start..*pos]).into_owned();
    *pos += 1;
    Some(s)
}

fn bvdf_node(data: &[u8], pos: &mut usize) -> HashMap<String, BvdfVal> {
    let mut map = HashMap::new();
    loop {
        if *pos >= data.len() {
            break;
        }
        let ty = data[*pos];
        *pos += 1;
        if ty == 0x08 {
            break;
        }
        let Some(key) = bvdf_cstr(data, pos) else {
            break;
        };
        match ty {
            0x00 => {
                map.insert(key, BvdfVal::Nested(bvdf_node(data, pos)));
            }
            0x01 => {
                let val = bvdf_cstr(data, pos).unwrap_or_default();
                map.insert(key, BvdfVal::Str(val));
            }
            0x02 => {
                if *pos + 4 > data.len() {
                    break;
                }
                let v = i32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
                *pos += 4;
                map.insert(key, BvdfVal::Int32(v));
            }
            0x07 => {
                if *pos + 8 > data.len() {
                    break;
                }
                let v = u64::from_le_bytes([
                    data[*pos],
                    data[*pos + 1],
                    data[*pos + 2],
                    data[*pos + 3],
                    data[*pos + 4],
                    data[*pos + 5],
                    data[*pos + 6],
                    data[*pos + 7],
                ]);
                *pos += 8;
                map.insert(key, BvdfVal::Uint64(v));
            }
            0x03 | 0x04 | 0x06 => {
                if *pos + 4 > data.len() {
                    break;
                }
                *pos += 4;
            }
            0x05 => {
                while *pos + 1 < data.len() {
                    if data[*pos] == 0 && data[*pos + 1] == 0 {
                        *pos += 2;
                        break;
                    }
                    *pos += 2;
                }
            }
            _ => break,
        }
    }
    map
}

pub(super) fn parse_bvdf_at(data: &[u8], start: usize) -> HashMap<String, BvdfVal> {
    let mut pos = start;
    bvdf_node(data, &mut pos)
}

fn extract_ach_unlocks(m: &HashMap<String, BvdfVal>) -> HashMap<String, (bool, Option<u64>)> {
    let mut out = HashMap::new();
    for (name, val) in m {
        if let BvdfVal::Nested(fields) = val {
            let achieved = fields
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("achieved"))
                .map(|(_, v)| matches!(v, BvdfVal::Int32(n) if *n != 0))
                .unwrap_or(false);
            let time = fields
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("time"))
                .and_then(|(_, v)| match v {
                    BvdfVal::Int32(t) => Some(*t as u64),
                    BvdfVal::Uint64(t) => Some(*t),
                    _ => None,
                });
            out.insert(name.clone(), (achieved, time));
        }
    }
    out
}

fn ach_map_unlocks(root: &HashMap<String, BvdfVal>) -> Option<HashMap<String, (bool, Option<u64>)>> {
    if let Some(BvdfVal::Nested(m)) = root
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("achievements"))
        .map(|(_, v)| v)
    {
        let unlocks = extract_ach_unlocks(m);
        if !unlocks.is_empty() {
            return Some(unlocks);
        }
    }

    for val in root.values() {
        if let BvdfVal::Nested(inner) = val {
            if let Some(BvdfVal::Nested(m)) = inner
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("achievements"))
                .map(|(_, v)| v)
            {
                let unlocks = extract_ach_unlocks(m);
                if !unlocks.is_empty() {
                    return Some(unlocks);
                }
            }
        }
    }

    None
}

pub(super) fn load_local_unlock_status(
    app_id: u32,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> HashMap<String, (bool, Option<u64>)> {
    let Some(account_id) = find_most_recent_steam_id(steam_paths)
        .as_deref()
        .and_then(steamid64_to_accountid) else {
        return HashMap::new();
    };

    for steam_root in steam_paths {
        let base = steam_root.join("userdata").join(&account_id).join(app_id.to_string());
        let candidates = [
            base.join("stats").join("UserGameStats.bin"),
            base.join("local").join("achievements.bin"),
        ];
        for path in &candidates {
            let Ok(data) = std::fs::read(path) else {
                continue;
            };
            for &start in &[0usize, 2, 4, 8] {
                if start >= data.len() {
                    break;
                }
                let root = parse_bvdf_at(&data, start);
                if let Some(unlocks) = ach_map_unlocks(&root) {
                    return unlocks;
                }
            }
        }
    }

    load_appcache_unlock_status(app_id, &account_id, steam_paths, language)
}

fn load_appcache_unlock_status(
    app_id: u32,
    account_id: &str,
    steam_paths: &[PathBuf],
    language: AppLanguage,
) -> HashMap<String, (bool, Option<u64>)> {
    let bit_mapping = load_schema_achievement_bits(app_id, steam_paths, language);
    if bit_mapping.is_empty() {
        return HashMap::new();
    }

    for steam_root in steam_paths {
        let stats_path = steam_root
            .join("appcache")
            .join("stats")
            .join(format!("UserGameStats_{}_{}.bin", account_id, app_id));
        let Ok(data) = std::fs::read(&stats_path) else {
            continue;
        };

        for &start in &[0usize, 2, 4, 8] {
            if start >= data.len() {
                break;
            }
            let root = parse_bvdf_at(&data, start);
            let mut result = HashMap::new();
            extract_bitmask_unlocks(&root, &bit_mapping, &mut result, 0);
            if !result.is_empty() {
                return result;
            }
        }
    }

    HashMap::new()
}

fn extract_bitmask_unlocks(
    node: &HashMap<String, BvdfVal>,
    bit_mapping: &HashMap<String, Vec<(u32, SchemaAchInfo)>>,
    result: &mut HashMap<String, (bool, Option<u64>)>,
    depth: u32,
) {
    if depth > 6 {
        return;
    }

    for (key, val) in node {
        if let BvdfVal::Nested(inner) = val {
            if let Some(entries) = bit_mapping.get(key) {
                if let Some(BvdfVal::Int32(data)) = inner
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("data"))
                    .map(|(_, v)| v)
                {
                    let bitmask = *data as u32;
                    let timestamps: HashMap<u32, u64> = inner
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case("AchievementTimes"))
                        .and_then(|(_, v)| {
                            if let BvdfVal::Nested(times) = v {
                                Some(times)
                            } else {
                                None
                            }
                        })
                        .map(|times| {
                            times
                                .iter()
                                .filter_map(|(k, v)| {
                                    let idx = k.parse::<u32>().ok()?;
                                    let ts = match v {
                                        BvdfVal::Int32(t) => Some(*t as u64),
                                        BvdfVal::Uint64(t) => Some(*t),
                                        _ => None,
                                    }?;
                                    Some((idx, ts))
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    for (bit_idx, info) in entries {
                        let achieved = (bitmask >> bit_idx) & 1 != 0;
                        let time = if achieved {
                            timestamps.get(bit_idx).copied()
                        } else {
                            None
                        };
                        result.insert(info.api_name.clone(), (achieved, time));
                    }
                }
            }

            extract_bitmask_unlocks(inner, bit_mapping, result, depth + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{ach_map_unlocks, extract_bitmask_unlocks, parse_bvdf_at, BvdfVal};
    use crate::steam::achievement_schema::SchemaAchInfo;

    fn push_cstr(bytes: &mut Vec<u8>, value: &str) {
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(0);
    }

    #[test]
    fn ach_map_unlocks_reads_binary_vdf_achievement_entries() {
        let mut data = Vec::new();

        data.push(0x00);
        push_cstr(&mut data, "achievements");

        data.push(0x00);
        push_cstr(&mut data, "ACH_WIN");
        data.push(0x02);
        push_cstr(&mut data, "achieved");
        data.extend_from_slice(&1_i32.to_le_bytes());
        data.push(0x07);
        push_cstr(&mut data, "time");
        data.extend_from_slice(&123_u64.to_le_bytes());
        data.push(0x08);

        data.push(0x00);
        push_cstr(&mut data, "ACH_LOSE");
        data.push(0x02);
        push_cstr(&mut data, "achieved");
        data.extend_from_slice(&0_i32.to_le_bytes());
        data.push(0x08);

        data.push(0x08);
        data.push(0x08);

        let root = parse_bvdf_at(&data, 0);
        let unlocks = ach_map_unlocks(&root).expect("achievement entries should parse");

        assert_eq!(unlocks.get("ACH_WIN"), Some(&(true, Some(123))));
        assert_eq!(unlocks.get("ACH_LOSE"), Some(&(false, None)));
    }

    #[test]
    fn extract_bitmask_unlocks_maps_bits_and_timestamps() {
        let mut result = HashMap::new();

        let mut timestamps = HashMap::new();
        timestamps.insert("0".to_string(), BvdfVal::Uint64(11));
        timestamps.insert("2".to_string(), BvdfVal::Int32(29));

        let mut stat_fields = HashMap::new();
        stat_fields.insert("data".to_string(), BvdfVal::Int32(0b101));
        stat_fields.insert(
            "AchievementTimes".to_string(),
            BvdfVal::Nested(timestamps),
        );

        let mut root = HashMap::new();
        root.insert("STAT_BITS".to_string(), BvdfVal::Nested(stat_fields));

        let mut mapping = HashMap::new();
        mapping.insert(
            "STAT_BITS".to_string(),
            vec![
                (
                    0,
                    SchemaAchInfo {
                        api_name: "ACH_ZERO".to_string(),
                        display_name: None,
                        description: None,
                        is_hidden: false,
                        icon_url: None,
                        icon_gray_url: None,
                    },
                ),
                (
                    1,
                    SchemaAchInfo {
                        api_name: "ACH_ONE".to_string(),
                        display_name: None,
                        description: None,
                        is_hidden: false,
                        icon_url: None,
                        icon_gray_url: None,
                    },
                ),
                (
                    2,
                    SchemaAchInfo {
                        api_name: "ACH_TWO".to_string(),
                        display_name: None,
                        description: None,
                        is_hidden: false,
                        icon_url: None,
                        icon_gray_url: None,
                    },
                ),
            ],
        );

        extract_bitmask_unlocks(&root, &mapping, &mut result, 0);

        assert_eq!(result.get("ACH_ZERO"), Some(&(true, Some(11))));
        assert_eq!(result.get("ACH_ONE"), Some(&(false, None)));
        assert_eq!(result.get("ACH_TWO"), Some(&(true, Some(29))));
    }
}