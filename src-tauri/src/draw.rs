use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use image::io::Reader as ImageReader;
use image::ImageError;
use imageproc::drawing::draw_text_mut;
use reqwest::blocking::get;
use rusttype::{Font, Scale};
use std::{ffi::OsStr, io, path::Path, time::Instant};
use tauri::State;
use tokio::{fs::File, io::AsyncWriteExt};

use crate::{wallpaper, Settings};

pub async fn open_image(path: String) -> Result<image::RgbaImage, ImageError> {
    let start = Instant::now();
    let image: Result<_, tokio::task::JoinError> = tokio::task::spawn_blocking(move || {
        let img = ImageReader::open(&path)
            .unwrap()
            .decode()
            .unwrap()
            .to_rgba8();
        println!("open image {}ms", start.elapsed().as_millis());
        img
    })
    .await;

    Ok(image.unwrap())
}

pub async fn open_image_url(url: String) -> Result<image::RgbaImage, ImageError> {
    // Download the image
    let response = get(url.clone()).map_err(|e| {
        ImageError::IoError(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to download image: {}", e),
        ))
    })?;
    let bytes = response.bytes().map_err(|e| {
        ImageError::IoError(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to read image bytes: {}", e),
        ))
    })?;

    // Extract the file extension from the URL
    let path = Path::new(&url);
    let extension = path.extension().and_then(OsStr::to_str).unwrap_or("jpg");

    // Create the temporary file path
    let file_path = format!("/tmp/wallpaper.{}.{}", rand::random::<u64>(), extension);

    // Save the image to a temporary file
    let mut file = File::create(&file_path).await.map_err(|e| {
        ImageError::IoError(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to create file: {}", e),
        ))
    })?;
    file.write_all(&bytes).await.map_err(|e| {
        ImageError::IoError(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to write to file: {}", e),
        ))
    })?;
    file.flush().await.map_err(|e| {
        ImageError::IoError(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to flush file: {}", e),
        ))
    })?;

    let image: Result<_, tokio::task::JoinError> = tokio::task::spawn_blocking(move || {
        let img = ImageReader::open(&file_path)
            .unwrap()
            .decode()
            .unwrap()
            .to_rgba8();
        img
    })
    .await;

    Ok(image.map_err(|_| {
        ImageError::IoError(io::Error::new(io::ErrorKind::Other, "Failed to load image"))
    })?)
}

fn load_system_font() -> Result<rusttype::Font<'static>, Box<dyn std::error::Error>> {
    let font_kit_font = SystemSource::new()
        .select_best_match(
            &[FamilyName::Title("Geist".to_string())],
            &Properties::new(),
        )
        .unwrap()
        .load()
        .unwrap();

    let font_data = (*font_kit_font.copy_font_data().unwrap()).clone();
    let rusttype_font =
        Font::try_from_vec(font_data).unwrap_or_else(|| panic!("error constructing a Font"));

    Ok(rusttype_font)
}

pub fn draw_text(
    image: &mut image::RgbaImage,
    text: &str,
    x: i32,
    y: i32,
    scale: Scale,
    color: image::Rgba<u8>,
    font: &Font,
) {
    draw_text_mut(image, color, x, y, scale, font, text);
}

#[tauri::command]
pub async fn draw_text_to_base_wallpaper(
    text: String,
    settings: State<'_, Settings>,
) -> Result<(), String> {
    let base_wallpaper_path = {
        let settings = settings.inner();
        settings.base_wallpaper_path.lock().unwrap().clone()
    };

    let mut wallpaper_guard = settings.wallpaper.lock().unwrap();
    let wallpaper = &mut *wallpaper_guard;

    let font = load_system_font().unwrap();
    draw_text(
        wallpaper,
        &text,
        10,
        10,
        Scale::uniform(50.0),
        image::Rgba([255, 255, 255, 255]),
        &font,
    );


    match wallpaper::save_wallpaper(wallpaper, &base_wallpaper_path) {
        Ok(_) => {}
        Err(e) => {
            println!("Failed to save wallpaper: {}", e);
            return Err(e);
        }
    }
    wallpaper::change_wallpaper(&base_wallpaper_path).unwrap();

    Ok(())
}
