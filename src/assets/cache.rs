use std::path::{Path, PathBuf};

const APP_DATA_DIR_NAME: &str = "Big Screen Launcher";

pub fn app_root_dir() -> PathBuf {
    let dir = dirs::data_local_dir()
        .map(|path| path.join(APP_DATA_DIR_NAME))
        .unwrap_or_else(legacy_app_root_dir);

    ensure_dir(&dir)
}

pub fn config_dir() -> PathBuf {
    data_subdir("config")
}

pub fn cache_subdir(name: &str) -> PathBuf {
    let dir = data_subdir("caches").join(name);

    ensure_dir(&dir)
}

pub fn logs_dir() -> PathBuf {
    data_subdir("logs")
}

fn data_subdir(name: &str) -> PathBuf {
    let dir = app_root_dir().join(name);

    ensure_dir(&dir)
}

fn ensure_dir(dir: &Path) -> PathBuf {
    let _ = std::fs::create_dir_all(dir);
    dir.to_path_buf()
}

fn legacy_app_root_dir() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}
