use crate::Settings;
use reqwest::blocking::get;
use std::env;
use std::ffi::OsStr;

#[cfg(target_os = "linux")]
use std::io::Write;
use std::path::Path;

use jpeg_encoder as jpeg;
use png::{ColorType, Encoder};
use std::fs::File;
use std::io::BufWriter;
use std::process::Command;
use tauri::State;

#[cfg(target_os = "windows")]
#[cfg(target_os = "macos")]
use wallpaper::set_from_path;

#[cfg(target_os = "windows")]
#[cfg(target_os = "macos")]
pub fn change_wallpaper(image_path: &str) {
    wallpaper::set_from_path(image_path)
}

enum LinuxWallpaperCommand {
    GSettings,
    XfConf,
    DbusPlasma,
    SwayBG,
    Nitrogen,
    Feh,
}

fn escape_path(path: &str) -> String {
    let special_chars = [
        ' ', '$', '`', '\\', '"', '\'', '&', '|', '*', '?', ';', '<', '>', '(', ')', '[', ']', '{',
        '}', '^', '#', '~',
    ];
    let mut escaped_path = String::new();

    for c in path.chars() {
        if special_chars.contains(&c) {
            escaped_path.push('\\');
        }
        escaped_path.push(c);
    }

    escaped_path
}

impl LinuxWallpaperCommand {
    fn command(&self, image_path: String) -> String {
        // Format it for proper use with command syntax
        let formatted_path = escape_path(image_path.as_str());

        match self {
            LinuxWallpaperCommand::GSettings => format!(
                "gsettings set org.gnome.desktop.background picture-uri \"file://{}\"",
                image_path.clone()
            ),
            LinuxWallpaperCommand::XfConf => format!(
                "xfconf-query -c xfce4-desktop -p /backdrop/screen0/monitor0/image-path -s {}",
                formatted_path
            ),
            LinuxWallpaperCommand::DbusPlasma => format!(
                "qdbus org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.evaluateScript 'string:\
                var Desktops = desktops(); \
                for (i=0;i<Desktops.length;i++) \
                {{ \
                    d = Desktops[i]; \
                    d.wallpaperPlugin = \"org.kde.image\"; \
                    d.currentConfigGroup = Array(\"Wallpaper\", \"org.kde.image\", \"General\"); \
                    d.writeConfig(\"Image\", \"file://{}\") \
                }}'", image_path
            ),
            LinuxWallpaperCommand::SwayBG => {
                format!("swaybg -i {}", formatted_path)
            }
            LinuxWallpaperCommand::Nitrogen => {
                format!("nitrogen --set-zoom-fill {}", formatted_path)
            }
            LinuxWallpaperCommand::Feh => format!("feh --bg-fill {}", formatted_path),
        }
    }

    fn execute(&self, command: String) -> Result<(), String> {
        let mut parts = command.trim().split_whitespace();
        let command = parts.next().ok_or("Command was empty")?;
        let args = parts.collect::<Vec<&str>>();

        Command::new(command)
            .args(args)
            .spawn()
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        Ok(())
    }
}

#[cfg(target_os = "linux")]
pub fn change_wallpaper(image_path: &str) -> Result<(), String> {
    println!("Changing wallpaper to {}", image_path);
    use core::panic;

    let environment = env::var("XDG_CURRENT_DESKTOP").unwrap();
    let command = match environment.as_str() {
        "GNOME" => LinuxWallpaperCommand::GSettings,
        "ubuntu:GNOME" => LinuxWallpaperCommand::GSettings,
        "XFCE" => LinuxWallpaperCommand::XfConf,
        "KDE" => LinuxWallpaperCommand::DbusPlasma,
        "sway" => LinuxWallpaperCommand::SwayBG,
        "Hyprland" => LinuxWallpaperCommand::SwayBG,
        _ => {
            // Check if nitrogen is installed
            if Command::new("nitrogen").arg("--version").output().is_ok() {
                LinuxWallpaperCommand::Nitrogen
            } else if Command::new("feh").arg("--version").output().is_ok() {
                LinuxWallpaperCommand::Feh
            } else {
                panic!("No supported command found!");
            }
        } // Default to Feh or Nitrogen
    };

    let full_command = command.command(image_path.to_string());
    let result = command.execute(full_command);
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to change wallpaper: {}", e).to_string()),
    }
}

#[cfg(target_os = "windows")]
#[cfg(target_os = "macos")]
pub fn change_wallpaper_url(url: &str) -> Result<(), String> {
    wallpaper::set_from_url(url).unwrap();
    wallpaper::set_mode(wallpaper::Mode::Fit).unwrap();

    Ok(())
}

#[cfg(target_os = "linux")]
pub fn change_wallpaper_url(url: &str) -> Result<(), String> {
    // Download the image
    let response = get(url).map_err(|e| format!("Failed to download image: {}", e))?;
    let bytes = response
        .bytes()
        .map_err(|e| format!("Failed to read image bytes: {}", e))?;

    // Extract the file extension from the URL
    let path = Path::new(url);
    let extension = path.extension().and_then(OsStr::to_str).unwrap_or("jpg");

    // Create the temporary file path
    let file_path = format!("/tmp/wallpaper.{}.{}", rand::random::<u64>(), extension);

    // Save the image to a temporary file
    let mut file = File::create(&file_path).map_err(|e| format!("Failed to create file: {}", e))?;
    file.write_all(&bytes)
        .map_err(|e| format!("Failed to write to file: {}", e))?;
    file.flush()
        .map_err(|e| format!("Failed to flush file: {}", e))?;

    // Set the wallpaper to the temporary file
    change_wallpaper(&file_path)
}

#[tauri::command]
pub async fn set_base_wallpaper_path<'a>(
    image_path: &'a str,
    state: State<'a, Settings>,
) -> Result<(), String> {
    {
        let mut base_wallpaper_path = state.base_wallpaper_path.lock().unwrap();
        *base_wallpaper_path = image_path.to_string();

        let mut base_wallpaper_type = state.base_wallpaper_type.lock().unwrap();
        *base_wallpaper_type = crate::BaseWallpaperType::Path;
    }
    println!(
        "Base wallpaper path set to {}",
        state.base_wallpaper_path.lock().unwrap()
    );

    let image_path_clone = image_path.to_string();
    tokio::task::spawn_blocking(move || {
        let _ = change_wallpaper(&image_path_clone);
    })
    .await
    .unwrap();

    let image_path_clone = image_path.to_string();
    tokio::task::spawn_blocking(move || {
        let _ = change_wallpaper(&image_path_clone);
    })
    .await
    .unwrap();

    // Preload the image
    let image_path_clone = image_path.to_string();
    let image = crate::draw::open_image(image_path_clone).await.unwrap();

    {
        let mut wallpaper_state = state.wallpaper.lock().unwrap();
        *wallpaper_state = image;
    }

    Ok(())
}

#[tauri::command]
pub async fn set_base_wallpaper_url<'a>(
    url: &'a str,
    state: State<'a, Settings>,
) -> Result<(), String> {
    {
        let mut base_wallpaper_url = state.base_wallpaper_url.lock().unwrap();
        *base_wallpaper_url = url.to_string();

        let mut base_wallpaper_type = state.base_wallpaper_type.lock().unwrap();
        *base_wallpaper_type = crate::BaseWallpaperType::Url;
    }
    println!(
        "Base wallpaper URL set to {}",
        state.base_wallpaper_url.lock().unwrap()
    );

    let url_clone = url.to_string();
    tokio::task::spawn_blocking(move || {
        let _ = change_wallpaper_url(&url_clone);
    })
    .await
    .unwrap();

    let url_clone = url.to_string();
    tokio::task::spawn_blocking(move || {
        let _ = change_wallpaper(&url_clone);
    })
    .await
    .unwrap();

    // Preload the image
    let url_clone = url.to_string();
    let image = crate::draw::open_image_url(url_clone).await.unwrap();

    {
        let mut wallpaper_state = state.wallpaper.lock().unwrap();
        *wallpaper_state = image;
    }

    Ok(())
}

// Reason we're using a custom method with the proper encoders
// is because `image::RgbaImage::save` is *really* slow (1.5secs vs 0.2secs)
pub fn save_wallpaper(image: &image::RgbaImage, path: &str) -> Result<(), String> {
    if path.ends_with(".png") {
        let file = File::create(path).map_err(|e| format!("Failed to create file: {}", e))?;
        let ref mut w = BufWriter::new(file);

        let mut encoder = Encoder::new(w, image.width(), image.height());
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("Failed to write PNG header: {}", e))?;

        writer
            .write_image_data(&image)
            .map_err(|e| format!("Failed to write image data: {}", e))?;
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        let encoder = jpeg::Encoder::new_file(path, 100).unwrap();
        let data = image.clone().into_raw();
        encoder
            .encode(
                &data,
                image.width() as u16,
                image.height() as u16,
                jpeg::ColorType::Rgba,
            )
            .unwrap();
    } else {
        return Err(format!("Unsupported file extension: {}", path));
    }
    Ok(())
}
