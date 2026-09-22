///static_image
use std::path::Path;
use image::DynamicImage;

use crate::image_content::ImageContent;
use crate::image_operation::Rotation;

pub struct StaticImage {
    image: DynamicImage,
    display_image: image::RgbaImage,
    rotation: Rotation,
}

impl StaticImage {
    pub fn new(image: DynamicImage) -> Self {
        let display_image = image.to_rgba8();
        Self {
            image,
            display_image,
            rotation: Rotation::None,
        }
    }

    fn update_display_image(&mut self) {
        self.display_image = match self.rotation {
            Rotation::None => self.image.to_rgba8(),
            Rotation::Right => self.image.rotate90().to_rgba8(),
            Rotation::Rotate180 => self.image.rotate180().to_rgba8(),
            Rotation::Left => self.image.rotate270().to_rgba8(),
        };
    }

    fn update_loading(&mut self) -> bool {
        false
    }
}

impl ImageContent for StaticImage {
    fn current_image(&self) -> &image::RgbaImage {
        &self.display_image
    }

    fn size(&self) -> egui::Vec2 {
        egui::vec2(
            self.display_image.width() as f32,
            self.display_image.height() as f32,
        )
    }

    fn rotate_right(&mut self) {
        self.rotation = self.rotation.rotate_right();
        self.update_display_image();
    }
    fn rotate_left(&mut self) {
        self.rotation = self.rotation.rotate_left();
        self.update_display_image();
    }
    fn rotation(&self) -> Rotation {
        self.rotation
    }

    fn is_modified(&self) -> bool {
        self.rotation != Rotation::None
    }
    
    fn save(&self, path: &Path) -> Result<(), String> {
        self.display_image
            .save(path)
            .map_err(|e| e.to_string())
    }


}