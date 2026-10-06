///main
#[macro_use]
mod app_log;
mod app;
mod app_config;
mod app_thumbnail;
mod app_ui;
mod folder_tree;
mod gif_animation;
mod gif_worker;
mod image_cache;
mod image_content;
mod image_operation;
mod image_property;
mod jpeg_loader;
mod plugin;
mod save_image;
mod static_image;
mod sub_window;
mod webp_animation;

#[cfg(target_os = "windows")]
mod windows_shell;

use crate::app::MyApp;


fn main() -> eframe::Result<()> {
    let icon = image::load_from_memory(include_bytes!("../assets/RixIcon.png"))
        .expect("embedded application icon must be a valid image")
        .to_rgba8();
    let icon = image::imageops::resize(&icon, 256, 256, image::imageops::FilterType::Lanczos3);
    let (icon_width, icon_height) = icon.dimensions();
    let icon = egui::IconData {
        rgba: icon.into_raw(),
        width: icon_width as u32,
        height: icon_height as u32,
    };

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 600.0])
            .with_min_inner_size([100.0,100.0])
            .with_title("RixViewer")
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "RixViewer",
        options,
        Box::new(|cc| {
            Ok(Box::new(MyApp::new(cc)))
        }),
    )
}

