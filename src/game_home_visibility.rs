use std::collections::HashSet;
use std::path::PathBuf;

const HOME_HIDDEN_FILE_NAME: &str = "game_home_hidden.json";

fn storage_dir() -> PathBuf {
    crate::assets::cache::config_dir()
}

fn storage_path() -> PathBuf {
    storage_dir().join(HOME_HIDDEN_FILE_NAME)
}

pub fn load_hidden_keys() -> HashSet<String> {
    let Ok(bytes) = std::fs::read(storage_path()) else {
        return HashSet::new();
    };

    serde_json::from_slice::<Vec<String>>(&bytes)
        .unwrap_or_default()
        .into_iter()
        .collect()
}

pub fn store_hidden_keys(keys: &HashSet<String>) {
    let mut sorted = keys.iter().cloned().collect::<Vec<_>>();
    sorted.sort();
    let Ok(bytes) = serde_json::to_vec(&sorted) else {
        return;
    };

    let _ = std::fs::write(storage_path(), bytes);
}
