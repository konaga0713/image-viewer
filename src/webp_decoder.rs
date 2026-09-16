use std::fs;
use std::path::Path;
use std::io::Cursor;
use image::RgbaImage;


pub fn load_webp (path: &Path,) -> Result<RgbaImage, Box<dyn std::error::Error>> {

    let data = fs::read(path)?;
    let mut decoder = 
        image_webp::WebPDecoder::new(Cursor::new(&data))?;

    let (width, height) = decoder.dimensions();    
    println!("WebP dimensions: {} x {}", width, height);

    let has_alpha = decoder.has_alpha();
    println!("WebP has alpha: {}",decoder.has_alpha());

    let output_size = decoder
        .output_buffer_size()
        .ok_or("WebP output buffer size is too large")?;
    
    println!(
        "WebP output buffer size: {}",
        output_size
    );

    // WebP自身の色形式でデコード
    let mut decoded = vec![0u8; output_size];
    decoder.read_image(&mut decoded)?;

    // image crate側ではRGBAとして扱いたいので変換する
    let mut rgba = RgbaImage::new(width, height);
    
    if has_alpha {
        // RGBA8
        if decoded.len() != (width as usize * height as usize * 4) {
            return Err( "Unexpected WebP RGBA buffer size".into());
        }
        rgba.as_mut().copy_from_slice(&decoded);
    
    } else {
        // RGB8 -> RGBA8
        if decoded.len() != (width as usize * height as usize * 3) {
            return Err("Unexpected WebP RGB buffer size".into());
        }
        for (src, dst) in decoded
            .chunks_exact(3)
            .zip(rgba.chunks_exact_mut(4)) {
                dst[0] = src[0];
                dst[1] = src[1];
                dst[2] = src[2];
                dst[3] = 255;
            }
    }
    Ok(rgba)
}
