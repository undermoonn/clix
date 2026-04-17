use std::path::{Path, PathBuf};

pub fn app_root_dir() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

pub fn cache_subdir(name: &str) -> PathBuf {
    let root = app_root_dir();
    let cache_root = root.join("cache");
    let dir = cache_root.join(name);

    migrate_legacy_cache_dir(&root.join(name), &dir);

    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn migrate_legacy_cache_dir(old_dir: &Path, new_dir: &Path) {
    if !old_dir.exists() || old_dir == new_dir {
        return;
    }

    if let Some(parent) = new_dir.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if !new_dir.exists() && std::fs::rename(old_dir, new_dir).is_ok() {
        return;
    }

    let Ok(entries) = std::fs::read_dir(old_dir) else {
        return;
    };

    let _ = std::fs::create_dir_all(new_dir);

    for entry in entries.flatten() {
        let source_path = entry.path();
        let target_path = new_dir.join(entry.file_name());

        if source_path.is_dir() {
            migrate_legacy_cache_dir(&source_path, &target_path);
            let _ = std::fs::remove_dir(&source_path);
            continue;
        }

        if target_path.exists() {
            let _ = std::fs::remove_file(&source_path);
            continue;
        }

        if std::fs::rename(&source_path, &target_path).is_err() {
            if std::fs::copy(&source_path, &target_path).is_ok() {
                let _ = std::fs::remove_file(&source_path);
            }
        }
    }

    let _ = std::fs::remove_dir(old_dir);
}