use std::path::{Path, PathBuf};

pub fn app_root_dir() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

pub fn cache_subdir(name: &str) -> PathBuf {
    let dir = app_root_dir().join("big_screen_launcher_cache").join(name);

    let _ = std::fs::create_dir_all(&dir);
    dir
}