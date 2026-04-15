use std::path::{Path, PathBuf};

use walkdir::WalkDir;

fn cache_dir() -> PathBuf {
    let mut dir = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    dir.push("dlss_cache");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn cache_path(app_id: u32) -> PathBuf {
    cache_dir().join(format!("{}.txt", app_id))
}

fn load_cached_path(app_id: u32) -> Option<PathBuf> {
    let path = std::fs::read_to_string(cache_path(app_id)).ok()?;
    let dll_path = PathBuf::from(path.trim());
    if !dll_path.is_file() {
        return None;
    }
    Some(dll_path)
}

fn store_cached_path(app_id: u32, dll_path: &Path) {
    let _ = std::fs::write(cache_path(app_id), dll_path.to_string_lossy().as_bytes());
}

#[cfg(target_os = "windows")]
fn format_file_version(version_ms: u32, version_ls: u32) -> String {
    let mut parts = vec![
        (version_ms >> 16) & 0xffff,
        version_ms & 0xffff,
        (version_ls >> 16) & 0xffff,
        version_ls & 0xffff,
    ];

    while parts.len() > 2 && parts.last() == Some(&0) {
        parts.pop();
    }

    parts
        .into_iter()
        .map(|part| part.to_string())
        .collect::<Vec<_>>()
        .join(".")
}

#[cfg(target_os = "windows")]
#[allow(non_snake_case)]
#[repr(C)]
struct FixedFileInfo {
    dwSignature: u32,
    dwStrucVersion: u32,
    dwFileVersionMS: u32,
    dwFileVersionLS: u32,
    dwProductVersionMS: u32,
    dwProductVersionLS: u32,
    dwFileFlagsMask: u32,
    dwFileFlags: u32,
    dwFileOS: u32,
    dwFileType: u32,
    dwFileSubtype: u32,
    dwFileDateMS: u32,
    dwFileDateLS: u32,
}

#[cfg(target_os = "windows")]
fn read_windows_file_version(path: &Path) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use winapi::shared::minwindef::{DWORD, LPVOID, UINT};
    use winapi::um::winver::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let root_block: Vec<u16> = "\\"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut handle: DWORD = 0;
        let size = GetFileVersionInfoSizeW(wide_path.as_ptr(), &mut handle);
        if size == 0 {
            return None;
        }

        let mut version_info = vec![0_u8; size as usize];
        if GetFileVersionInfoW(
            wide_path.as_ptr(),
            0,
            size,
            version_info.as_mut_ptr() as LPVOID,
        ) == 0
        {
            return None;
        }

        let mut version_ptr: LPVOID = std::ptr::null_mut();
        let mut version_len: UINT = 0;
        if VerQueryValueW(
            version_info.as_mut_ptr() as LPVOID,
            root_block.as_ptr(),
            &mut version_ptr,
            &mut version_len,
        ) == 0
            || version_ptr.is_null()
            || version_len == 0
        {
            return None;
        }

        let fixed_info = &*(version_ptr as *const FixedFileInfo);
        if fixed_info.dwSignature != 0xfeef04bd {
            return None;
        }

        Some(format_file_version(
            fixed_info.dwFileVersionMS,
            fixed_info.dwFileVersionLS,
        ))
    }
}

#[cfg(target_os = "windows")]
pub fn detect_version(install_path: &Path, app_id: Option<u32>) -> Option<String> {
    const DLSS_DLL_NAMES: &[&str] = &[
        "nvngx_dlss.dll",
        "nvngx_dlssg.dll",
        "nvngx_dlssd.dll",
        "sl.dlss.dll",
        "sl.dlss_g.dll",
    ];

    if let Some(app_id) = app_id {
        if let Some(cached_path) = load_cached_path(app_id) {
            return Some(read_windows_file_version(&cached_path).unwrap_or_default());
        }
    }

    let search_root = if install_path.is_file() {
        install_path.parent()?
    } else if install_path.is_dir() {
        install_path
    } else {
        return None;
    };

    let mut best_match: Option<(usize, PathBuf)> = None;
    for entry in WalkDir::new(search_root)
        .follow_links(false)
        .max_depth(10)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy();
        if !DLSS_DLL_NAMES
            .iter()
            .any(|candidate| file_name.eq_ignore_ascii_case(candidate))
        {
            continue;
        }

        match &best_match {
            Some((best_depth, _)) if entry.depth() >= *best_depth => {}
            _ => best_match = Some((entry.depth(), entry.path().to_path_buf())),
        }
    }

    let Some(dll_path) = best_match.map(|(_, path)| path) else {
        return None;
    };

    if let Some(app_id) = app_id {
        store_cached_path(app_id, &dll_path);
    }

    let version = read_windows_file_version(&dll_path).unwrap_or_default();
    Some(version)
}

#[cfg(not(target_os = "windows"))]
pub fn detect_version(_install_path: &Path, _app_id: Option<u32>) -> Option<String> {
    None
}