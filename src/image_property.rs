//image_property
use std::path::Path;
use std::fs;
use chrono::{DateTime, Local};
use num_format::{Locale, ToFormattedString};

pub struct ImageProperty;

impl ImageProperty {
    pub fn show(
        ctx: &egui::Context,
        image_path: &Path,
        open: &mut bool,
    ) {
        //　ファイル情報
        let metadata = fs::metadata(image_path).ok();

        // file name
        let file_name = image_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("不明");

        // file size
        let file_size = metadata
            .as_ref()
            .map(|m| m.len())
            .unwrap_or(0);

        // created date
        let created_date = metadata 
            .as_ref()
            .and_then(|m| m.created().ok())
            .map(|time| {
                let datetime: DateTime<Local> = time.into();
                datetime.format("%Y/%m/%d %H:%M:%S").to_string()
            })
            .unwrap_or_else(|| "不明".to_string());

        // modified date
        let modified_date= metadata 
            .as_ref()
            .and_then(|m| m.modified().ok())
            .map(|time| {
                let datetime: DateTime<Local> = time.into();
                datetime.format("%Y/%m/%d %H:%M:%S").to_string()
            })
            .unwrap_or_else(||  "不明".to_string());

        // image information
        let image_info = image::open(image_path).ok();
        let (width, height, color_depth) = 
            if let Some(ref img) = image_info {
                let width = img.width();
                let height = img.height();
                let color_depth = match img.color() {
                    image::ColorType::L8 => 8,
                    image::ColorType::La8 => 16,
                    image::ColorType::Rgb8 => 24,
                    image::ColorType::Rgba8 => 32,

                    image::ColorType::L16 => 16,
                    image::ColorType::La16 => 32,
                    image::ColorType::Rgb16 => 48,
                    image::ColorType::Rgba16 => 64,

                    _ => 0,
                };
                (width, height, color_depth)
            } else {
                (0,0,0)
            };    

            // image_type
            let image_type =image_path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| match ext.to_lowercase().as_str(){
                    "jpg" | "jpeg" => "JPEG",
                    "png" => "PNG",
                    "gif" => "GIF",
                    "bmp" => "BMP",
                    "webp" => "WebP",
                    "tif" | "tiff" => "TIFF",
                    _ => "不明",
                })
                .unwrap_or("不明");

        egui::Window::new("プロパティ")
            .open(open)
            .show(ctx, |ui| {
                ui.label(format!("ファイル名: {}", 
                    file_name
                ));

                ui.label(format!("ファイルパス: {}", 
                    image_path.display())
                );

                ui.label(format!(
                    "画像の種類     {}",
                    image_type
                ));

                ui.label(format!(
                    "作成日         {}",
                    created_date
                ));

                ui.label(format!(
                    "更新日         {}",
                    modified_date
                ));

                ui.label(format!(
                    "サイズ         {} バイト",
                    file_size.to_formatted_string(&Locale::ja)
                ));

                ui.label(format!(
                    "画像サイズ     {} × {} ドット",
                    width,
                    height
                ));

                ui.label(format!(
                    "色深度         {}ビット",
                    color_depth
                ));
            });  
    }
}
