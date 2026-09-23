///main
mod app;
mod app_thumbnail;
mod app_ui;
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
mod webp_decoder;

use crate::app::MyApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 600.0])
            .with_min_inner_size([100.0,100.0])
            .with_title("Image Explorer App"),
        ..Default::default()
    };

    eframe::run_native(
        "RIX: Image Explorer App",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
}

