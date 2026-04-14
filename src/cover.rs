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

fn download_hd_cover(app_id: u32) -> Option<Vec<u8>> {
    let url = format!(
        "https://steamcdn-a.akamaihd.net/steam/apps/{}/library_hero.jpg",
        app_id
    );
    let resp = ureq::get(&url).call().ok()?;
    if resp.status() != 200 {
        return None;
    }
    let mut bytes: Vec<u8> = Vec::new();
    resp.into_reader()
        .take(10 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() < 1024 {
        return None;
    }
    let cache_path = hd_cache_dir().join(format!("{}_hero_hd.jpg", app_id));
    let _ = std::fs::write(&cache_path, &bytes);
    Some(bytes)
}

pub fn load_cover_bytes(steam_paths: &[PathBuf], app_id: u32) -> Option<Vec<u8>> {
    // 1. Check our own HD cache first
    let hd_path = hd_cache_dir().join(format!("{}_hero_hd.jpg", app_id));
    if hd_path.exists() {
        if let Ok(bytes) = std::fs::read(&hd_path) {
            if bytes.len() > 1024 {
                return Some(bytes);
            }
        }
    }

    // 2. Try downloading HD version from Steam CDN
    if let Some(bytes) = download_hd_cover(app_id) {
        return Some(bytes);
    }

    // 3. Fallback: local Steam library cache
    let candidates = [
        "library_hero.jpg",
        "library_hero.png",
        "header.jpg",
        "library_600x900.jpg",
    ];

    for steam_root in steam_paths {
        let app_cache_dir = steam_root
            .join("appcache")
            .join("librarycache")
            .join(app_id.to_string());
        if !app_cache_dir.exists() {
            continue;
        }
        for name in &candidates {
            let img_path = app_cache_dir.join(name);
            if img_path.exists() {
                if let Ok(bytes) = std::fs::read(&img_path) {
                    if bytes.len() > 1024 {
                        return Some(bytes);
                    }
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
