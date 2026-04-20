use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use dotenvy::from_filename_override;
use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use image::codecs::png::PngEncoder;
use image::ImageEncoder;
use resvg::{tiny_skia, usvg};

const ICON_SIZES: [u32; 6] = [16, 32, 48, 64, 128, 256];
const SVG_PATH: &str = "assets/app-icon.svg";
const PNG_FILE_NAME: &str = "app-icon-256.png";
const ICO_FILE_NAME: &str = "app-icon.ico";
const APP_DISPLAY_NAME: &str = "Big Screen Launcher";
const APP_BINARY_NAME: &str = "big-screen-launcher.exe";
const GA4_MEASUREMENT_ID_ENV: &str = "GA4_MEASUREMENT_ID";
const GA4_API_SECRET_ENV: &str = "GA4_API_SECRET";
const GA4_ENABLED_ENV: &str = "GA4_ENABLED";

fn main() {
    println!("cargo:rerun-if-changed={}", SVG_PATH);
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.env");
    println!("cargo:rerun-if-env-changed={}", GA4_MEASUREMENT_ID_ENV);
    println!("cargo:rerun-if-env-changed={}", GA4_API_SECRET_ENV);
    println!("cargo:rerun-if-env-changed={}", GA4_ENABLED_ENV);

    load_local_dotenv();
    emit_compile_time_env(GA4_MEASUREMENT_ID_ENV);
    emit_compile_time_env(GA4_API_SECRET_ENV);
    emit_compile_time_env(GA4_ENABLED_ENV);

    if let Err(error) = build_icon_assets() {
        panic!("failed to generate app icon assets: {error}");
    }
}

fn load_local_dotenv() {
    if env::var_os(GA4_MEASUREMENT_ID_ENV).is_some()
        || env::var_os(GA4_API_SECRET_ENV).is_some()
        || env::var_os(GA4_ENABLED_ENV).is_some()
    {
        return;
    }

    let _ = from_filename_override(".env");
}

fn emit_compile_time_env(key: &str) {
    if let Some(value) = env::var_os(key) {
        println!("cargo:rustc-env={}={}", key, value.to_string_lossy());
    }
}

fn build_icon_assets() -> Result<(), Box<dyn Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let svg = fs::read_to_string(SVG_PATH)?;
    let png_path = out_dir.join(PNG_FILE_NAME);
    let ico_path = out_dir.join(ICO_FILE_NAME);

    write_png(&png_path, &svg, 256)?;
    write_ico(&ico_path, &svg)?;
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

#[cfg(target_os = "windows")]
fn compile_windows_resource(icon_path: &Path) -> Result<(), Box<dyn Error>> {
    let mut resource = winres::WindowsResource::new();
    resource.set("FileDescription", APP_DISPLAY_NAME);
    resource.set("ProductName", APP_DISPLAY_NAME);
    resource.set("InternalName", APP_BINARY_NAME);
    resource.set("OriginalFilename", APP_BINARY_NAME);
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