use eframe::egui;
use std::io::Read;
use std::path::PathBuf;

pub fn bytes_to_texture(
    ctx: &egui::Context,
    bytes: &[u8],
    label: String,
) -> Option<egui::TextureHandle> {
    let dyn_img = image::load_from_memory(bytes).ok()?;
    let rgba = dyn_img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    Some(ctx.load_texture(label, color_image, egui::TextureOptions::LINEAR))
}

pub fn hd_cache_dir() -> PathBuf {
    let mut dir = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();
    dir.push("cover_cache");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn achievement_icon_cache_dir() -> PathBuf {
    let mut dir = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();
    dir.push("achievement_icon_cache");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn achievement_icon_cache_path(url: &str) -> PathBuf {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    achievement_icon_cache_dir().join(format!("{:x}.img", hasher.finish()))
}

pub fn load_cached_achievement_icon_bytes(url: &str) -> Option<Vec<u8>> {
    let bytes = std::fs::read(achievement_icon_cache_path(url)).ok()?;
    if bytes.is_empty() {
        return None;
    }
    Some(bytes)
}

pub fn load_achievement_icon_bytes(url: &str) -> Option<Vec<u8>> {
    if let Some(bytes) = load_cached_achievement_icon_bytes(url) {
        return Some(bytes);
    }

    let resp = ureq::get(url).call().ok()?;
    if resp.status() != 200 {
        return None;
    }

    let mut bytes = Vec::new();
    let mut reader = resp.into_reader().take(2 * 1024 * 1024);
    if reader.read_to_end(&mut bytes).is_err() || bytes.is_empty() {
        return None;
    }

    let _ = std::fs::write(achievement_icon_cache_path(url), &bytes);
    Some(bytes)
}

fn download_hd_cover(app_id: u32) -> Option<Vec<u8>> {
    // Download 3840x1240 library_hero from Steam CDN
    let urls = [
        format!(
            "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/{}/library_hero.jpg",
            app_id
        ),
        format!(
            "https://steamcdn-a.akamaihd.net/steam/apps/{}/library_hero.jpg",
            app_id
        ),
    ];
    for url in &urls {
        if let Ok(resp) = ureq::get(url).call() {
            if resp.status() == 200 {
                let mut bytes: Vec<u8> = Vec::new();
                if resp
                    .into_reader()
                    .take(10 * 1024 * 1024)
                    .read_to_end(&mut bytes)
                    .is_ok()
                    && bytes.len() > 1024
                {
                    let cache_path = hd_cache_dir().join(format!("{}_hero.jpg", app_id));
                    let _ = std::fs::write(&cache_path, &bytes);
                    return Some(bytes);
                }
            }
        }
    }
    None
}

pub fn load_cover_bytes(steam_paths: &[PathBuf], app_id: u32) -> Option<Vec<u8>> {
    // 1. Check local cache
    let cache_path = hd_cache_dir().join(format!("{}_hero.jpg", app_id));
    if cache_path.exists() {
        if let Ok(bytes) = std::fs::read(&cache_path) {
            if bytes.len() > 1024 {
                return Some(bytes);
            }
        }
    }

    // 2. Try downloading library_hero from Steam CDN
    if let Some(bytes) = download_hd_cover(app_id) {
        return Some(bytes);
    }

    // 3. Fallback: local Steam library cache
    for steam_root in steam_paths {
        let img_path = steam_root
            .join("appcache")
            .join("librarycache")
            .join(app_id.to_string())
            .join("library_hero.jpg");
        if img_path.exists() {
            if let Ok(bytes) = std::fs::read(&img_path) {
                if bytes.len() > 1024 {
                    return Some(bytes);
                }
            }
        }
    }
    None
}

/// Load game icon bytes from Steam's local library cache.
/// The icon is the small hashed .jpg file (32x32) in librarycache/{appid}/.
pub fn load_icon_bytes(steam_paths: &[PathBuf], app_id: u32) -> Option<Vec<u8>> {
    let known_names = [
        "header.jpg",
        "library_600x900.jpg",
        "library_hero.jpg",
        "library_hero_blur.jpg",
    ];
    for steam_root in steam_paths {
        let dir = steam_root
            .join("appcache")
            .join("librarycache")
            .join(app_id.to_string());
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if !name_str.ends_with(".jpg") {
                    continue;
                }
                if known_names.contains(&name_str.as_ref()) {
                    continue;
                }
                // This should be the hashed icon file
                if let Ok(bytes) = std::fs::read(entry.path()) {
                    if !bytes.is_empty() {
                        return Some(bytes);
                    }
                }
            }
        }
    }
    None
}
