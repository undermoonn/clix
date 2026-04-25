use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::Utc;
use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use image::codecs::png::PngEncoder;
use image::ImageEncoder;
use resvg::{tiny_skia, usvg};

const ICON_SIZES: [u32; 6] = [16, 32, 48, 64, 128, 256];
const SVG_PATH: &str = "assets/app-icon.svg";
const SETTINGS_SVG_PATH: &str = "src/icons/settings.svg";
const SYSTEM_SVG_PATH: &str = "src/icons/system.svg";
const SCREEN_SVG_PATH: &str = "src/icons/screen.svg";
const APPS_SVG_PATH: &str = "src/icons/apps.svg";
const EXIT_SVG_PATH: &str = "src/icons/exit.svg";
const POWER_SLEEP_SVG_PATH: &str = "src/icons/power-sleep.svg";
const POWER_REBOOT_SVG_PATH: &str = "src/icons/power-reboot.svg";
const POWER_OFF_SVG_PATH: &str = "src/icons/power-off.svg";
const MAIN_CLOCK_FONT_SIZE: f32 = 40.0;
const SETTINGS_ICON_SCALE: f32 = 0.63;
const POWER_TRIGGER_ICON_SCALE: f32 = 1.18;
const UI_ICON_MAX_RENDERED_SIZE: u32 =
    (MAIN_CLOCK_FONT_SIZE * SETTINGS_ICON_SCALE * POWER_TRIGGER_ICON_SCALE).ceil() as u32;
const UI_ICON_RENDER_SIZE: u32 = UI_ICON_MAX_RENDERED_SIZE * 4;
const PNG_FILE_NAME: &str = "app-icon-256.png";
const ICO_FILE_NAME: &str = "app-icon.ico";
const STORE_LOGO_1080_FILE_NAME: &str = "app-store-logo-1080.png";
const STORE_LOGO_2160_FILE_NAME: &str = "app-store-logo-2160.png";
const STORE_POSTER_720_FILE_NAME: &str = "app-store-poster-720x1080.png";
const STORE_POSTER_1440_FILE_NAME: &str = "app-store-poster-1440x2160.png";
const STORE_BACKGROUND_RGBA: [u8; 4] = [0x12, 0x12, 0x12, 0xff];
const STORE_LOGO_INSET_SCALE: f32 = 0.82;
const STORE_POSTER_INSET_SCALE: f32 = 0.84;
const SETTINGS_PNG_FILE_NAME: &str = "settings-icon-ui.png";
const SYSTEM_PNG_FILE_NAME: &str = "system-icon-ui.png";
const SCREEN_PNG_FILE_NAME: &str = "screen-icon-ui.png";
const APPS_PNG_FILE_NAME: &str = "apps-icon-ui.png";
const EXIT_PNG_FILE_NAME: &str = "exit-icon-ui.png";
const POWER_SLEEP_PNG_FILE_NAME: &str = "power-sleep-icon-ui.png";
const POWER_REBOOT_PNG_FILE_NAME: &str = "power-reboot-icon-ui.png";
const POWER_OFF_PNG_FILE_NAME: &str = "power-off-icon-ui.png";
const VIGNETTE_FILE_NAME: &str = "top-right-vignette.png";
const VIGNETTE_WIDTH: u32 = 2048;
const VIGNETTE_HEIGHT: u32 = 1152;

fn main() {
    println!(
        "cargo:rustc-env=BIG_SCREEN_LAUNCHER_BUILD_TIME={}",
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
    if let Some(git_commit) = current_git_commit() {
        println!("cargo:rustc-env=BIG_SCREEN_LAUNCHER_GIT_COMMIT={git_commit}");
    }
    println!("cargo:rerun-if-changed={}", SVG_PATH);
    println!("cargo:rerun-if-changed={}", SETTINGS_SVG_PATH);
    println!("cargo:rerun-if-changed={}", SYSTEM_SVG_PATH);
    println!("cargo:rerun-if-changed={}", SCREEN_SVG_PATH);
    println!("cargo:rerun-if-changed={}", APPS_SVG_PATH);
    println!("cargo:rerun-if-changed={}", EXIT_SVG_PATH);
    println!("cargo:rerun-if-changed={}", POWER_SLEEP_SVG_PATH);
    println!("cargo:rerun-if-changed={}", POWER_REBOOT_SVG_PATH);
    println!("cargo:rerun-if-changed={}", POWER_OFF_SVG_PATH);
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=build.rs");
    emit_git_rerun_hints();

    if let Err(error) = build_icon_assets() {
        panic!("failed to generate app icon assets: {error}");
    }
}

fn current_git_commit() -> Option<String> {
    run_git_command(["rev-parse", "--short=12", "HEAD"])
}

fn emit_git_rerun_hints() {
    let Some(git_dir) = resolve_git_dir() else {
        return;
    };

    let head_path = git_dir.join("HEAD");
    println!("cargo:rerun-if-changed={}", head_path.display());

    if let Ok(head_contents) = fs::read_to_string(&head_path) {
        if let Some(reference) = head_contents.strip_prefix("ref: ") {
            let reference = reference.trim();
            println!(
                "cargo:rerun-if-changed={}",
                git_dir.join(reference).display()
            );
        }
    }

    let packed_refs_path = git_dir.join("packed-refs");
    if packed_refs_path.exists() {
        println!("cargo:rerun-if-changed={}", packed_refs_path.display());
    }
}

fn resolve_git_dir() -> Option<PathBuf> {
    let git_dir = run_git_command(["rev-parse", "--git-dir"])?;
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").ok()?);
    let git_dir_path = PathBuf::from(git_dir);

    if git_dir_path.is_absolute() {
        Some(git_dir_path)
    } else {
        Some(manifest_dir.join(git_dir_path))
    }
}

fn run_git_command<const N: usize>(args: [&str; N]) -> Option<String> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").ok()?;
    let output = Command::new("git")
        .args(args)
        .current_dir(manifest_dir)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn build_icon_assets() -> Result<(), Box<dyn Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let assets_dir = PathBuf::from("assets");
    let svg = fs::read_to_string(SVG_PATH)?;
    let settings_svg = fs::read_to_string(SETTINGS_SVG_PATH)?;
    let system_svg = fs::read_to_string(SYSTEM_SVG_PATH)?;
    let screen_svg = fs::read_to_string(SCREEN_SVG_PATH)?;
    let apps_svg = fs::read_to_string(APPS_SVG_PATH)?;
    let exit_svg = fs::read_to_string(EXIT_SVG_PATH)?;
    let power_sleep_svg = fs::read_to_string(POWER_SLEEP_SVG_PATH)?;
    let power_reboot_svg = fs::read_to_string(POWER_REBOOT_SVG_PATH)?;
    let power_off_svg = fs::read_to_string(POWER_OFF_SVG_PATH)?;
    let png_path = out_dir.join(PNG_FILE_NAME);
    let ico_path = out_dir.join(ICO_FILE_NAME);
    let store_logo_1080_path = assets_dir.join(STORE_LOGO_1080_FILE_NAME);
    let store_logo_2160_path = assets_dir.join(STORE_LOGO_2160_FILE_NAME);
    let store_poster_720_path = assets_dir.join(STORE_POSTER_720_FILE_NAME);
    let store_poster_1440_path = assets_dir.join(STORE_POSTER_1440_FILE_NAME);
    let settings_png_path = out_dir.join(SETTINGS_PNG_FILE_NAME);
    let system_png_path = out_dir.join(SYSTEM_PNG_FILE_NAME);
    let screen_png_path = out_dir.join(SCREEN_PNG_FILE_NAME);
    let apps_png_path = out_dir.join(APPS_PNG_FILE_NAME);
    let exit_png_path = out_dir.join(EXIT_PNG_FILE_NAME);
    let power_sleep_png_path = out_dir.join(POWER_SLEEP_PNG_FILE_NAME);
    let power_reboot_png_path = out_dir.join(POWER_REBOOT_PNG_FILE_NAME);
    let power_off_png_path = out_dir.join(POWER_OFF_PNG_FILE_NAME);
    let vignette_path = out_dir.join(VIGNETTE_FILE_NAME);

    write_png(&png_path, &svg, 256)?;
    write_ico(&ico_path, &svg)?;
    write_store_png(&store_logo_1080_path, &svg, 1080, 1080, STORE_LOGO_INSET_SCALE)?;
    write_store_png(&store_logo_2160_path, &svg, 2160, 2160, STORE_LOGO_INSET_SCALE)?;
    write_store_png(
        &store_poster_720_path,
        &svg,
        720,
        1080,
        STORE_POSTER_INSET_SCALE,
    )?;
    write_store_png(
        &store_poster_1440_path,
        &svg,
        1440,
        2160,
        STORE_POSTER_INSET_SCALE,
    )?;
    write_png(
        &settings_png_path,
        &settings_svg.replace("currentColor", "#ffffff"),
        UI_ICON_RENDER_SIZE,
    )?;
    write_png(
        &system_png_path,
        &system_svg.replace("currentColor", "#ffffff"),
        UI_ICON_RENDER_SIZE,
    )?;
    write_png(
        &screen_png_path,
        &screen_svg.replace("currentColor", "#ffffff"),
        UI_ICON_RENDER_SIZE,
    )?;
    write_png(
        &apps_png_path,
        &apps_svg.replace("currentColor", "#ffffff"),
        UI_ICON_RENDER_SIZE,
    )?;
    write_png(
        &exit_png_path,
        &exit_svg.replace("currentColor", "#ffffff"),
        UI_ICON_RENDER_SIZE,
    )?;
    write_png(
        &power_sleep_png_path,
        &power_sleep_svg.replace("currentColor", "#ffffff"),
        UI_ICON_RENDER_SIZE,
    )?;
    write_png(
        &power_reboot_png_path,
        &power_reboot_svg.replace("currentColor", "#ffffff"),
        UI_ICON_RENDER_SIZE,
    )?;
    write_png(
        &power_off_png_path,
        &power_off_svg.replace("currentColor", "#ffffff"),
        UI_ICON_RENDER_SIZE,
    )?;
    write_top_right_vignette(&vignette_path)?;
    compile_windows_resource(&ico_path)?;

    Ok(())
}

fn write_png(path: &Path, svg: &str, size: u32) -> Result<(), Box<dyn Error>> {
    let rgba = render_svg(svg, size, size, 1.0, None)?;
    let png = encode_png(&rgba, size, size)?;
    write_if_changed(path, &png)?;
    Ok(())
}

fn write_store_png(
    path: &Path,
    svg: &str,
    width: u32,
    height: u32,
    inset_scale: f32,
) -> Result<(), Box<dyn Error>> {
    let rgba = render_svg(
        svg,
        width,
        height,
        inset_scale,
        Some(STORE_BACKGROUND_RGBA),
    )?;
    let png = encode_png(&rgba, width, height)?;
    write_if_changed(path, &png)?;
    Ok(())
}

fn write_ico(path: &Path, svg: &str) -> Result<(), Box<dyn Error>> {
    let mut icon_dir = IconDir::new(ResourceType::Icon);

    for size in ICON_SIZES {
        let rgba = render_svg(svg, size, size, 1.0, None)?;
        let image = IconImage::from_rgba_data(size, size, rgba);
        icon_dir.add_entry(IconDirEntry::encode(&image)?);
    }

    let file = fs::File::create(path)?;
    icon_dir.write(file)?;
    Ok(())
}

fn write_top_right_vignette(path: &Path) -> Result<(), Box<dyn Error>> {
    let rgba = render_top_right_vignette(VIGNETTE_WIDTH, VIGNETTE_HEIGHT);
    let png = encode_png(&rgba, VIGNETTE_WIDTH, VIGNETTE_HEIGHT)?;
    write_if_changed(path, &png)?;
    Ok(())
}

fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut png = Vec::new();
    PngEncoder::new(&mut png).write_image(rgba, width, height, image::ColorType::Rgba8)?;
    Ok(png)
}

fn write_if_changed(path: &Path, contents: &[u8]) -> Result<(), Box<dyn Error>> {
    if fs::read(path).ok().as_deref() == Some(contents) {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, contents)?;
    Ok(())
}

fn render_svg(
    svg: &str,
    width: u32,
    height: u32,
    inset_scale: f32,
    background: Option<[u8; 4]>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &options)?;
    let svg_size = tree.size();
    let scale = f32::min(width as f32 / svg_size.width(), height as f32 / svg_size.height())
        * inset_scale;
    let render_width = svg_size.width() * scale;
    let render_height = svg_size.height() * scale;
    let translate_x = (width as f32 - render_width) * 0.5;
    let translate_y = (height as f32 - render_height) * 0.5;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| format!("failed to allocate {width}x{height} icon pixmap"))?;

    if let Some([red, green, blue, alpha]) = background {
        pixmap.fill(tiny_skia::Color::from_rgba8(red, green, blue, alpha));
    }

    resvg::render(
        &tree,
        tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, translate_x, translate_y),
        &mut pixmap.as_mut(),
    );

    Ok(pixmap.data().to_vec())
}

fn render_top_right_vignette(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = vec![0; (width * height * 4) as usize];

    for y in 0..height {
        for x in 0..width {
            let nx = x as f32 / (width.saturating_sub(1)) as f32;
            let ny = y as f32 / (height.saturating_sub(1)) as f32;

            let layer = |cx: f32, cy: f32, rx: f32, ry: f32, strength: f32| -> f32 {
                let dx = (nx - cx) / rx;
                let dy = (ny - cy) / ry;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist >= 1.0 {
                    0.0
                } else {
                    let t = 1.0 - dist;
                    let eased = t * t * (3.0 - 2.0 * t);
                    eased * strength
                }
            };

            let combined = 1.0
                - (1.0 - layer(1.16, -0.12, 0.78, 1.02, 0.24))
                    * (1.0 - layer(1.05, -0.02, 0.54, 0.72, 0.21))
                    * (1.0 - layer(0.97, 0.06, 0.3, 0.42, 0.16));

            let hash = x.wrapping_mul(73856093) ^ y.wrapping_mul(19349663);
            let noise = ((hash & 255) as f32 / 255.0 - 0.5) * 0.018;
            let alpha = (combined + noise).clamp(0.0, 1.0);
            let alpha_u8 = (alpha * 255.0).round() as u8;
            let index = ((y * width + x) * 4) as usize;

            rgba[index] = 6;
            rgba[index + 1] = 8;
            rgba[index + 2] = 12;
            rgba[index + 3] = alpha_u8;
        }
    }

    rgba
}

#[cfg(target_os = "windows")]
fn compile_windows_resource(icon_path: &Path) -> Result<(), Box<dyn Error>> {
    let mut resource = winres::WindowsResource::new();
    resource.set_icon(
        icon_path
            .to_str()
            .ok_or("generated icon path contains invalid unicode")?,
    );
    resource.compile()?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn compile_windows_resource(_icon_path: &Path) -> Result<(), Box<dyn Error>> {
    Ok(())
}