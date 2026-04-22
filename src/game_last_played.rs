use std::collections::HashMap;
use std::path::PathBuf;

const LAST_PLAYED_FILE_NAME: &str = "game_last_played.json";

fn storage_dir() -> PathBuf {
    let dir = crate::assets::cache::app_root_dir().join("config");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn storage_path() -> PathBuf {
    storage_dir().join(LAST_PLAYED_FILE_NAME)
}

fn load_entries() -> HashMap<String, u64> {
    let Ok(bytes) = std::fs::read(storage_path()) else {
        return HashMap::new();
    };

    serde_json::from_slice::<HashMap<String, u64>>(&bytes).unwrap_or_default()
}

fn store_entries(entries: &HashMap<String, u64>) {
    let Ok(bytes) = serde_json::to_vec(entries) else {
        return;
    };

    let _ = std::fs::write(storage_path(), bytes);
}

pub fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn merge_into_games(games: &mut [crate::game::Game]) {
    let entries = load_entries();
    if entries.is_empty() {
        return;
    }

    let now = now_unix_secs();
    for game in games {
        let Some(&stored) = entries.get(&game.persistent_key()) else {
            continue;
        };
        game.last_played = select_closest_to_now(now, game.last_played, stored);
    }
}

pub fn record_for_game(game_key: &str, timestamp: u64) {
    let mut entries = load_entries();
    entries.insert(game_key.to_owned(), timestamp);
    store_entries(&entries);
}

fn select_closest_to_now(now: u64, scanned: u64, stored: u64) -> u64 {
    match (scanned, stored) {
        (0, 0) => 0,
        (0, value) | (value, 0) => value,
        (left, right) => {
            let left_distance = now.abs_diff(left);
            let right_distance = now.abs_diff(right);
            match left_distance.cmp(&right_distance) {
                std::cmp::Ordering::Less => left,
                std::cmp::Ordering::Greater => right,
                std::cmp::Ordering::Equal => left.max(right),
            }
        }
    }
}