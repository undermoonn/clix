use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

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
    println!("cargo:rerun-if-changed={}", SVG_PATH);
    println!("cargo:rerun-if-changed={}", SETTINGS_SVG_PATH);
    println!("cargo:rerun-if-changed={}", SYSTEM_SVG_PATH);
    println!("cargo:rerun-if-changed={}", SCREEN_SVG_PATH);
    println!("cargo:rerun-if-changed={}", APPS_SVG_PATH);
    println!("cargo:rerun-if-changed={}", EXIT_SVG_PATH);
    println!("cargo:rerun-if-changed={}", POWER_SLEEP_SVG_PATH);
    println!("cargo:rerun-if-changed={}", POWER_REBOOT_SVG_PATH);
    println!("cargo:rerun-if-changed={}", POWER_OFF_SVG_PATH);
    println!("cargo:rerun-if-changed=build.rs");

    if let Err(error) = build_icon_assets() {
        panic!("failed to generate app icon assets: {error}");
    }
}

fn build_icon_assets() -> Result<(), Box<dyn Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
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
    let rgba = render_svg(svg, size)?;
    let file = fs::File::create(path)?;
    PngEncoder::new(file).write_image(&rgba, size, size, image::ColorType::Rgba8)?;
    Ok(())
}

fn write_ico(path: &Path, svg: &str) -> Result<(), Box<dyn Error>> {
    let mut icon_dir = IconDir::new(ResourceType::Icon);

    for size in ICON_SIZES {
        let rgba = render_svg(svg, size)?;
        let image = IconImage::from_rgba_data(size, size, rgba);
        icon_dir.add_entry(IconDirEntry::encode(&image)?);
    }

    let file = fs::File::create(path)?;
    icon_dir.write(file)?;
    Ok(())
}

fn write_top_right_vignette(path: &Path) -> Result<(), Box<dyn Error>> {
    let rgba = render_top_right_vignette(VIGNETTE_WIDTH, VIGNETTE_HEIGHT);
    let file = fs::File::create(path)?;
    PngEncoder::new(file).write_image(
        &rgba,
        VIGNETTE_WIDTH,
        VIGNETTE_HEIGHT,
        image::ColorType::Rgba8,
    )?;
    Ok(())
}

fn render_svg(svg: &str, size: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &options)?;
    let svg_size = tree.size();
    let scale_x = size as f32 / svg_size.width();
    let scale_y = size as f32 / svg_size.height();
    let mut pixmap = tiny_skia::Pixmap::new(size, size)
        .ok_or_else(|| format!("failed to allocate {size}x{size} icon pixmap"))?;

    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale_x, scale_y),
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