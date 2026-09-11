use std::path::Path;
use image::{DynamicImage, ImageError};
use crate::image_operation::Rotation;

pub fn save_image( 
    path: &Path,
    rotation: Rotation,
) -> Result<(), ImageError> {
    let mut img = image::open(path)?;

    img = match rotation {
        Rotation::None => img,
        Rotation::Right => img.rotate90(),
        Rotation::Rotate180 => img.rotate180(),
        Rotation::Left => img.rotate270(),
    };

    img.save(path)?;
    Ok(())

}