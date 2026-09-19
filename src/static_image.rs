///static_image
use std::path::Path;
use image::DynamicImage;

use crate::image_content::ImageContent;
use crate::image_operation::Rotation;

pub struct StaticImage {
    image: DynamicImage,
    rotation: Rotation,
}

impl StaticImage {
    pub fn new(image: DynamicImage) -> Self {
        Self {
            image,
            rotation: Rotation::None,
        }
    }

    fn rotated_image(&self) -> DynamicImage {
        match self.rotation {
            Rotation::None => self.image.clone(),
            Rotation::Right => self.image.rotate90(),
            Rotation::Rotate180 => self.image.rotate180(),
            Rotation::Left => self.image.rotate270(),
        }
    } 
}

impl ImageContent for StaticImage {
    fn current_image(&self) ->image::RgbaImage {
        self.rotated_image().to_rgba8()
    }

    fn size(&self) -> egui::Vec2 {
        let (width, height) = 
            match self.rotation {
                Rotation::None | Rotation::Rotate180 => {
                    (self.image.width(), self.image.height())
                }
                Rotation::Right | Rotation::Left => {
                    (self.image.height(), self.image.width())
                }
        };

        egui::vec2(
            width as f32,
            height as f32,
        )
    }

    fn rotate_right(&mut self) {
        self.rotation = self.rotation.rotate_right();
    }
    fn rotate_left(&mut self) {
        self.rotation = self.rotation.rotate_left();
    }
    fn rotation(&self) -> Rotation {
        self.rotation
    }

    fn is_modified(&self) -> bool {
        self.rotation != Rotation::None
    }
    
    fn save(&self, path: &Path) -> Result<(), String> {
        self.rotated_image()
            .save(path)
            .map_err(|e| e.to_string())
    }


}