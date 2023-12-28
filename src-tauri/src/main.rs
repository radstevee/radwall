// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
mod wallpaper;
mod draw;

struct Settings {
    pub base_wallpaper_path: Mutex<String>,
    pub base_wallpaper_url: Mutex<String>,
    pub base_wallpaper_type: Mutex<BaseWallpaperType>,
    pub wallpaper: Mutex<image::RgbaImage>,
}

#[derive(Serialize, Deserialize)]
pub enum BaseWallpaperType {
    Path,
    Url,
}

fn main() {
    let wallpaper_image = image::RgbaImage::new(100, 100);
    let wallpaper_mutex = Mutex::new(wallpaper_image);

    tauri::Builder::default()
        .manage(Settings { 
            base_wallpaper_path: Mutex::new("".to_string()),
            base_wallpaper_url: Mutex::new("".to_string()),
            base_wallpaper_type: Mutex::new(BaseWallpaperType::Path),
            wallpaper: wallpaper_mutex
        })
        .invoke_handler(tauri::generate_handler![
            wallpaper::set_base_wallpaper_path,
            wallpaper::set_base_wallpaper_url,
            draw::draw_text_to_base_wallpaper
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
