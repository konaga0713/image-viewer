use std::path::Path;

// Mirrors the host plugin contracts in src/plugin.rs and src/image_content.rs.
// Keep these trait definitions and method order in sync with the application.
pub trait ImageDecoderPlugin: Send + Sync {
    fn supported_extensions(&self) -> Vec<&'static str>;
    fn decode(&self, path: &Path) -> Result<Box<dyn ImageContent>, String>;
}

pub trait ImageContent: Send {
    fn current_image(&self) -> &image::RgbaImage;
    fn size(&self) -> egui::Vec2;
    fn rotate_right(&mut self);
    fn rotate_left(&mut self);
    fn rotation(&self) -> Rotation;
    fn is_animated(&self) -> bool { false }
    fn next_frame(&mut self) -> Option<std::time::Duration> { None }
    fn current_delay(&self) -> Option<std::time::Duration> { None }
    fn frame_count(&self) -> usize { 1 }
    fn is_modified(&self) -> bool { false }
    fn save(&self, path: &Path) -> Result<(), String>;
    fn process_loading(&mut self) -> bool { false }
    fn is_loading(&self) -> bool { false }
    fn update_loading(&mut self) -> bool { false }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Rotation { None, Right, Rotate180, Left }

impl Rotation {
    fn right(self) -> Self {
        match self { Self::None => Self::Right, Self::Right => Self::Rotate180,
            Self::Rotate180 => Self::Left, Self::Left => Self::None }
    }
    fn left(self) -> Self {
        match self { Self::None => Self::Left, Self::Right => Self::None,
            Self::Rotate180 => Self::Right, Self::Left => Self::Rotate180 }
    }
}

struct AvifPlugin;

impl ImageDecoderPlugin for AvifPlugin {
    fn supported_extensions(&self) -> Vec<&'static str> { vec!["avif"] }

    fn decode(&self, path: &Path) -> Result<Box<dyn ImageContent>, String> {
        let image = image::open(path).map_err(|e| format!("AVIF decode failed: {e}"))?;
        Ok(Box::new(AvifImage::new(image)))
    }
}

struct AvifImage {
    original: image::DynamicImage,
    displayed: image::RgbaImage,
    rotation: Rotation,
}

impl AvifImage {
    fn new(image: image::DynamicImage) -> Self {
        let displayed = image.to_rgba8();
        Self { original: image, displayed, rotation: Rotation::None }
    }

    fn refresh(&mut self) {
        self.displayed = match self.rotation {
            Rotation::None => self.original.to_rgba8(),
            Rotation::Right => self.original.rotate90().to_rgba8(),
            Rotation::Rotate180 => self.original.rotate180().to_rgba8(),
            Rotation::Left => self.original.rotate270().to_rgba8(),
        };
    }
}

impl ImageContent for AvifImage {
    fn current_image(&self) -> &image::RgbaImage { &self.displayed }
    fn size(&self) -> egui::Vec2 {
        egui::vec2(self.displayed.width() as f32, self.displayed.height() as f32)
    }
    fn rotate_right(&mut self) { self.rotation = self.rotation.right(); self.refresh(); }
    fn rotate_left(&mut self) { self.rotation = self.rotation.left(); self.refresh(); }
    fn rotation(&self) -> Rotation { self.rotation }
    fn is_modified(&self) -> bool { self.rotation != Rotation::None }
    fn save(&self, path: &Path) -> Result<(), String> {
        self.displayed.save(path).map_err(|e| e.to_string())
    }
}

#[unsafe(no_mangle)]
pub fn create_plugin() -> Box<dyn ImageDecoderPlugin> { Box::new(AvifPlugin) }
