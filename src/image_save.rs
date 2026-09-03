use std::path::Path;
use image::{DynamicImage, ImageError};

pub fn save_image(
    image: &DynamicImage, 
    path: &Path
) -> Result<(), ImageError> {
    image.save(path)
}