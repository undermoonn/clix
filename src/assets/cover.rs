use eframe::egui;
use image::ImageEncoder;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::cache;

#[cfg(target_os = "windows")]
use walkdir::WalkDir;

#[cfg(target_os = "windows")]
use winapi::shared::minwindef::UINT;

#[cfg(target_os = "windows")]
use winapi::shared::windef::{HBITMAP, HDC, HGDIOBJ, HICON};

#[cfg(target_os = "windows")]
use winapi::um::shellapi::ExtractIconExW;

#[cfg(target_os = "windows")]
use winapi::um::wingdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, SelectObject, BITMAP,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};

#[cfg(target_os = "windows")]
use winapi::um::winuser::{DestroyIcon, GetIconInfo, ICONINFO};

#[cfg(target_os = "windows")]
extern "system" {
    fn PrivateExtractIconsW(
        szFileName: *const u16,
        nIconIndex: i32,
        cxIcon: i32,
        cyIcon: i32,
        phicon: *mut HICON,
        piconid: *mut UINT,
        nIcons: UINT,
        flags: UINT,
    ) -> UINT;
}

pub fn bytes_to_texture_limited(
    ctx: &egui::Context,
    bytes: &[u8],
    label: String,
    max_size: Option<(usize, usize)>,
) -> Option<egui::TextureHandle> {
    let dyn_img = image::load_from_memory(bytes).ok()?;
    let dyn_img = downscale_to_fit(dyn_img, max_size);
    let rgba = dyn_img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    Some(ctx.load_texture(label, color_image, egui::TextureOptions::LINEAR))
}

pub fn bytes_to_cover_texture(
    ctx: &egui::Context,
    bytes: &[u8],
    label: String,
) -> Option<egui::TextureHandle> {
    bytes_to_texture_limited(ctx, bytes, label, Some((2560, 832)))
}

pub fn bytes_to_logo_texture(
    ctx: &egui::Context,
    bytes: &[u8],
    label: String,
) -> Option<egui::TextureHandle> {
    bytes_to_texture_limited(ctx, bytes, label, Some((1024, 512)))
}

pub fn bytes_to_game_icon_texture(
    ctx: &egui::Context,
    bytes: &[u8],
    label: String,
) -> Option<egui::TextureHandle> {
    bytes_to_texture_limited(ctx, bytes, label, Some((256, 256)))
}

pub fn bytes_to_achievement_icon_texture(
    ctx: &egui::Context,
    bytes: &[u8],
    label: String,
) -> Option<egui::TextureHandle> {
    bytes_to_texture_limited(ctx, bytes, label, Some((128, 128)))
}

fn downscale_to_fit(
    dyn_img: image::DynamicImage,
    max_size: Option<(usize, usize)>,
) -> image::DynamicImage {
    let Some((max_width, max_height)) = max_size else {
        return dyn_img;
    };

    let width = dyn_img.width() as usize;
    let height = dyn_img.height() as usize;
    if width == 0 || height == 0 || (width <= max_width && height <= max_height) {
        return dyn_img;
    }

    let scale = (max_width as f32 / width as f32).min(max_height as f32 / height as f32);
    let resized_width = ((width as f32 * scale).round() as u32).max(1);
    let resized_height = ((height as f32 * scale).round() as u32).max(1);
    dyn_img.resize(
        resized_width,
        resized_height,
        image::imageops::FilterType::Triangle,
    )
}

fn png_bytes_from_rgba(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(rgba, width, height, image::ColorType::Rgba8)
        .ok()?;
    Some(bytes)
}

pub fn hd_cache_dir() -> PathBuf {
    cache::cache_subdir("cover_cache")
}

fn hero_logo_cache_path(app_id: u32) -> PathBuf {
    hd_cache_dir().join(format!("{}_logo.png", app_id))
}

fn is_png_bytes(bytes: &[u8]) -> bool {
    const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    bytes.len() >= PNG_SIGNATURE.len() && &bytes[..PNG_SIGNATURE.len()] == PNG_SIGNATURE
}

fn achievement_icon_cache_dir() -> PathBuf {
    cache::cache_subdir("achievement_icon_cache")
}

fn achievement_icon_cache_path(url: &str) -> PathBuf {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    achievement_icon_cache_dir().join(format!("{:x}.png", hasher.finish()))
}

fn encode_achievement_icon_cache_bytes(bytes: &[u8]) -> Option<Vec<u8>> {
    let dyn_img = image::load_from_memory(bytes).ok()?;
    let rgba = dyn_img.to_rgba8();
    png_bytes_from_rgba(rgba.width(), rgba.height(), rgba.as_raw())
}

fn achievement_icon_cache_bytes_are_valid(bytes: &[u8]) -> bool {
    is_png_bytes(bytes) && image::load_from_memory(bytes).is_ok()
}

pub fn clear_cached_achievement_icon(url: &str) {
    let _ = std::fs::remove_file(achievement_icon_cache_path(url));
}

pub fn load_cached_achievement_icon_bytes(url: &str) -> Option<Vec<u8>> {
    let cache_path = achievement_icon_cache_path(url);
    let bytes = std::fs::read(&cache_path).ok()?;
    if !achievement_icon_cache_bytes_are_valid(&bytes) {
        let _ = std::fs::remove_file(cache_path);
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
    let (_, body) = resp.into_parts();
    let mut reader = body.into_reader().take(2 * 1024 * 1024);
    if reader.read_to_end(&mut bytes).is_err() {
        return None;
    }

    let cache_bytes = encode_achievement_icon_cache_bytes(&bytes)?;
    let _ = std::fs::write(achievement_icon_cache_path(url), &cache_bytes);
    Some(cache_bytes)
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
                let (_, body) = resp.into_parts();
                if body
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

fn download_logo_bytes(app_id: u32) -> Option<Vec<u8>> {
    let urls = [
        format!(
            "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/{}/library_logo.png",
            app_id
        ),
        format!(
            "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/{}/logo.png",
            app_id
        ),
        format!(
            "https://steamcdn-a.akamaihd.net/steam/apps/{}/library_logo.png",
            app_id
        ),
        format!(
            "https://steamcdn-a.akamaihd.net/steam/apps/{}/logo.png",
            app_id
        ),
    ];

    for url in &urls {
        if let Ok(resp) = ureq::get(url).call() {
            if resp.status() == 200 {
                let mut bytes: Vec<u8> = Vec::new();
                let (_, body) = resp.into_parts();
                if body
                    .into_reader()
                    .take(4 * 1024 * 1024)
                    .read_to_end(&mut bytes)
                    .is_ok()
                    && bytes.len() > 512
                    && is_png_bytes(&bytes)
                {
                    let cache_path = hero_logo_cache_path(app_id);
                    let _ = std::fs::write(&cache_path, &bytes);
                    return Some(bytes);
                }
            }
        }
    }

    None
}

fn librarycache_candidate_dirs(steam_paths: &[PathBuf], app_id: u32) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    for steam_root in steam_paths {
        let app_dir = steam_root
            .join("appcache")
            .join("librarycache")
            .join(app_id.to_string());
        if !app_dir.exists() {
            continue;
        }

        dirs.push(app_dir.clone());

        if let Ok(entries) = std::fs::read_dir(&app_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                }
            }
        }
    }

    dirs
}

fn load_named_librarycache_asset_bytes(
    steam_paths: &[PathBuf],
    app_id: u32,
    preferred_names: &[&str],
    min_size: usize,
    require_png: bool,
) -> Option<Vec<u8>> {
    for dir in librarycache_candidate_dirs(steam_paths, app_id) {
        for name in preferred_names {
            let candidate = dir.join(name);
            if !candidate.exists() {
                continue;
            }

            if let Ok(bytes) = std::fs::read(&candidate) {
                if bytes.len() < min_size {
                    continue;
                }
                if require_png && !is_png_bytes(&bytes) {
                    continue;
                }
                return Some(bytes);
            }
        }
    }

    None
}

pub fn load_cover_bytes(steam_paths: &[PathBuf], app_id: u32) -> Option<Vec<u8>> {
    let cache_path = hd_cache_dir().join(format!("{}_hero.jpg", app_id));

    // 1. Prefer Steam's local librarycache so hashed subdirectories override stale cache.
    if let Some(bytes) = load_named_librarycache_asset_bytes(
        steam_paths,
        app_id,
        &["library_hero.jpg"],
        1024,
        false,
    ) {
        let _ = std::fs::write(&cache_path, &bytes);
        return Some(bytes);
    }

    // 2. Check local app cache
    if cache_path.exists() {
        if let Ok(bytes) = std::fs::read(&cache_path) {
            if bytes.len() > 1024 {
                return Some(bytes);
            }
        }
    }

    // 3. Try downloading library_hero from Steam CDN
    if let Some(bytes) = download_hd_cover(app_id) {
        return Some(bytes);
    }

    None
}

pub fn load_logo_bytes(steam_paths: &[PathBuf], app_id: u32) -> Option<Vec<u8>> {
    let cache_path = hero_logo_cache_path(app_id);

    if let Some(bytes) = load_named_librarycache_asset_bytes(
        steam_paths,
        app_id,
        &["library_logo.png", "logo.png"],
        512,
        true,
    ) {
        let _ = std::fs::write(&cache_path, &bytes);
        return Some(bytes);
    }

    if cache_path.exists() {
        if let Ok(bytes) = std::fs::read(&cache_path) {
            if bytes.len() > 512 && is_png_bytes(&bytes) {
                return Some(bytes);
            }
        }
    }

    let preferred_names = ["library_logo.png", "logo.png"];

    for dir in librarycache_candidate_dirs(steam_paths, app_id) {
        for name in preferred_names {
            let candidate = dir.join(name);
            if !candidate.exists() {
                continue;
            }

            if let Ok(bytes) = std::fs::read(&candidate) {
                if bytes.len() > 512 && is_png_bytes(&bytes) {
                    let _ = std::fs::write(&cache_path, &bytes);
                    return Some(bytes);
                }
            }
        }

        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                    continue;
                };
                let lower_name = name.to_ascii_lowercase();
                if !lower_name.contains("logo") || !lower_name.ends_with(".png") {
                    continue;
                }

                if let Ok(bytes) = std::fs::read(&path) {
                    if bytes.len() > 512 && is_png_bytes(&bytes) {
                        let _ = std::fs::write(&cache_path, &bytes);
                        return Some(bytes);
                    }
                }
            }
        }
    }

    download_logo_bytes(app_id)
}

#[cfg(target_os = "windows")]
fn normalize_name_for_match(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(target_os = "windows")]
fn exe_candidate_score(path: &Path, game_name: &str, root: &Path) -> i64 {
    let file_stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let normalized_stem = normalize_name_for_match(file_stem);
    let normalized_game = normalize_name_for_match(game_name);
    let lower_name = file_stem.to_ascii_lowercase();
    let mut score = 0_i64;

    if !normalized_game.is_empty() && normalized_stem == normalized_game {
        score += 10_000;
    } else if !normalized_game.is_empty()
        && (normalized_stem.contains(&normalized_game) || normalized_game.contains(&normalized_stem))
    {
        score += 6_000;
    }

    for token in normalized_game.split_whitespace() {
        if token.len() >= 3 && normalized_stem.contains(token) {
            score += 700;
        }
    }

    if let Ok(relative) = path.strip_prefix(root) {
        let depth = relative.components().count() as i64;
        score += (6 - depth).max(0) * 350;
    }

    if let Ok(metadata) = std::fs::metadata(path) {
        score += (metadata.len() / (1024 * 1024)).min(64) as i64 * 20;
    }

    let negative_markers = [
        "unins",
        "crash",
        "report",
        "launcher",
        "setup",
        "install",
        "uninstall",
        "benchmark",
        "config",
        "updater",
        "redistributable",
        "redist",
        "eac",
        "anticheat",
    ];
    for marker in negative_markers {
        if lower_name.contains(marker) {
            score -= 2_500;
        }
    }

    score
}

#[cfg(target_os = "windows")]
fn find_preferred_executable(install_path: &Path, game_name: &str) -> Option<PathBuf> {
    if install_path.is_file() {
        let is_exe = install_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("exe"))
            .unwrap_or(false);
        return is_exe.then(|| install_path.to_path_buf());
    }

    if !install_path.is_dir() {
        return None;
    }

    let mut best: Option<(i64, PathBuf)> = None;
    for entry in WalkDir::new(install_path)
        .follow_links(false)
        .max_depth(4)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let is_exe = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("exe"))
            .unwrap_or(false);
        if !is_exe {
            continue;
        }

        let score = exe_candidate_score(path, game_name, install_path);
        match &best {
            Some((best_score, _)) if score <= *best_score => {}
            _ => best = Some((score, path.to_path_buf())),
        }
    }

    best.map(|(_, path)| path)
}

#[cfg(target_os = "windows")]
fn read_bitmap_rgba(hdc: HDC, bitmap: HBITMAP, width: i32, height: i32) -> Option<Vec<u8>> {
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [unsafe { std::mem::zeroed() }; 1],
    };

    let mut pixels = vec![0_u8; (width as usize) * (height as usize) * 4];
    let copied = unsafe {
        GetDIBits(
            hdc,
            bitmap,
            0,
            height as UINT,
            pixels.as_mut_ptr() as *mut _,
            &mut info,
            DIB_RGB_COLORS,
        )
    };
    if copied == 0 {
        return None;
    }

    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }

    Some(pixels)
}

#[cfg(target_os = "windows")]
fn merge_icon_mask_alpha(rgba: &mut [u8], mask_rgba: &[u8]) {
    let has_alpha = rgba.chunks_exact(4).any(|pixel| pixel[3] != 0);
    if has_alpha {
        return;
    }

    for (pixel, mask_pixel) in rgba.chunks_exact_mut(4).zip(mask_rgba.chunks_exact(4)) {
        let mask_value = mask_pixel[0].max(mask_pixel[1]).max(mask_pixel[2]);
        pixel[3] = if mask_value > 127 { 0 } else { 255 };
    }
}

#[cfg(target_os = "windows")]
fn icon_handle_to_png_bytes(icon: HICON) -> Option<Vec<u8>> {
    unsafe {
        let mut icon_info: ICONINFO = std::mem::zeroed();
        if GetIconInfo(icon, &mut icon_info) == 0 {
            DestroyIcon(icon);
            return None;
        }

        let mut color_bitmap: BITMAP = std::mem::zeroed();
        let source_bitmap = if !icon_info.hbmColor.is_null() {
            icon_info.hbmColor
        } else {
            icon_info.hbmMask
        };

        if GetObjectW(
            source_bitmap as *mut _,
            std::mem::size_of::<BITMAP>() as i32,
            &mut color_bitmap as *mut _ as *mut _,
        ) == 0
        {
            if !icon_info.hbmColor.is_null() {
                DeleteObject(icon_info.hbmColor as HGDIOBJ);
            }
            if !icon_info.hbmMask.is_null() {
                DeleteObject(icon_info.hbmMask as HGDIOBJ);
            }
            DestroyIcon(icon);
            return None;
        }

        let width = color_bitmap.bmWidth;
        let mut height = color_bitmap.bmHeight;
        if icon_info.hbmColor.is_null() {
            height /= 2;
        }

        let hdc = CreateCompatibleDC(std::ptr::null_mut());
        if hdc.is_null() {
            if !icon_info.hbmColor.is_null() {
                DeleteObject(icon_info.hbmColor as HGDIOBJ);
            }
            if !icon_info.hbmMask.is_null() {
                DeleteObject(icon_info.hbmMask as HGDIOBJ);
            }
            DestroyIcon(icon);
            return None;
        }

        let old_bitmap = SelectObject(hdc, source_bitmap as HGDIOBJ);
        let mut rgba = read_bitmap_rgba(hdc, source_bitmap, width, height);
        if !icon_info.hbmColor.is_null() && !icon_info.hbmMask.is_null() {
            let _ = SelectObject(hdc, icon_info.hbmMask as HGDIOBJ);
            if let (Some(ref mut rgba_pixels), Some(mask_rgba)) = (
                rgba.as_mut(),
                read_bitmap_rgba(hdc, icon_info.hbmMask, width, height),
            ) {
                merge_icon_mask_alpha(rgba_pixels, &mask_rgba);
            }
        }
        SelectObject(hdc, old_bitmap);
        DeleteDC(hdc);

        if !icon_info.hbmColor.is_null() {
            DeleteObject(icon_info.hbmColor as HGDIOBJ);
        }
        if !icon_info.hbmMask.is_null() {
            DeleteObject(icon_info.hbmMask as HGDIOBJ);
        }
        DestroyIcon(icon);

        let rgba = rgba?;
        png_bytes_from_rgba(width as u32, height as u32, &rgba)
    }
}

#[cfg(target_os = "windows")]
fn extract_private_icon_bytes(executable_path: &Path, size: i32) -> Option<Vec<u8>> {
    use std::os::windows::ffi::OsStrExt;

    let wide_path = executable_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();

    unsafe {
        let mut icon: HICON = std::ptr::null_mut();
        let mut icon_id: UINT = 0;
        if PrivateExtractIconsW(
            wide_path.as_ptr(),
            0,
            size,
            size,
            &mut icon,
            &mut icon_id,
            1,
            0,
        ) == 0
            || icon.is_null()
        {
            return None;
        }

        icon_handle_to_png_bytes(icon)
    }
}

#[cfg(target_os = "windows")]
fn extract_icon_bytes_from_executable(executable_path: &Path) -> Option<Vec<u8>> {
    use std::os::windows::ffi::OsStrExt;

    if let Some(bytes) = extract_private_icon_bytes(executable_path, 256) {
        return Some(bytes);
    }

    let wide_path = executable_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();

    unsafe {
        let mut large_icon: HICON = std::ptr::null_mut();
        if ExtractIconExW(wide_path.as_ptr(), 0, &mut large_icon, std::ptr::null_mut(), 1) == 0
            || large_icon.is_null()
        {
            return None;
        }

        icon_handle_to_png_bytes(large_icon)
    }
}

#[cfg(target_os = "windows")]
fn load_executable_icon_bytes(game: &crate::steam::Game) -> Option<Vec<u8>> {
    let executable = find_preferred_executable(&game.path, &game.name)?;
    extract_icon_bytes_from_executable(&executable)
}

pub fn load_game_icon_bytes(steam_paths: &[PathBuf], game: &crate::steam::Game) -> Option<Vec<u8>> {
    #[cfg(target_os = "windows")]
    if let Some(bytes) = load_executable_icon_bytes(game) {
        return Some(bytes);
    }

    game.app_id.and_then(|app_id| load_icon_bytes(steam_paths, app_id))
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

#[cfg(test)]
mod tests {
    use super::load_named_librarycache_asset_bytes;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("clix_{}_{}", label, unique))
    }

    #[test]
    fn load_named_librarycache_asset_bytes_finds_hashed_subdirectory_assets() {
        let root = unique_temp_dir("librarycache");
        let nested_dir = root
            .join("appcache")
            .join("librarycache")
            .join("3764200")
            .join("90d7401b621e98bd61b2f66616ffafcb58d75fd7");
        std::fs::create_dir_all(&nested_dir).unwrap();

        let expected = vec![7_u8; 2048];
        std::fs::write(nested_dir.join("library_hero.jpg"), &expected).unwrap();

        let bytes = load_named_librarycache_asset_bytes(
            &[root.clone()],
            3764200,
            &["library_hero.jpg"],
            1024,
            false,
        );

        assert_eq!(bytes, Some(expected));

        let _ = std::fs::remove_dir_all(root);
    }
}
