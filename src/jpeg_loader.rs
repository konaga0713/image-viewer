//jpeg_loader

use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub enum LoadMode {
    Thumbnail {max_width:u32, max_height: u32},
    Full,
}

pub fn load(path: &Path, mode: LoadMode,) -> Result<image::RgbaImage, String> {
    let data = 
        std::fs::read(path)
            .map_err(|e| format!("JPEG読み込み失敗: {}", e))?;

    match mode {
        LoadMode::Thumbnail { max_width, max_height } => {
            load_thumbnail(&data, max_width, max_height)
        }
        LoadMode::Full => {
            load_full(&data)
        }
    }        
}

// サムネイル読み込み
fn load_thumbnail(data: &[u8], max_width: u32, max_height: u32
) -> Result<image::RgbaImage, String> {

    // TurboJPEGの縮小デコード
    let mut decompressor = turbojpeg::Decompressor::new()
        .map_err(|e| e.to_string())?;

    let header = decompressor
        .read_header(data)
        .map_err(|e| e.to_string())?;

    let scale = (max_width as f32 / header.width as f32)
        .min(max_height as f32 / header.height as f32);

    let scaling_factor = 
        if scale <= 0.125 {
            turbojpeg::ScalingFactor::ONE_EIGHTH
        } else if scale <= 0.25 {
            turbojpeg::ScalingFactor::ONE_QUARTER
        } else if scale <= 0.5 {
            turbojpeg::ScalingFactor::ONE_HALF
        } else {
            turbojpeg::ScalingFactor::ONE
        };

    decompressor
        .set_scaling_factor(scaling_factor)
        .map_err(|e| e.to_string())?;

    let scaled_header = header.scaled(scaling_factor);

    let width = scaled_header.width;
    let height = scaled_header.height;

    let mut pixels = vec![0u8; width * height * 4];
    let image_buf = turbojpeg::Image {
        pixels: &mut pixels[..],
        width,
        pitch: width * 4,
        height,
        format: turbojpeg::PixelFormat::RGBA,
    };

    let res = decompressor.decompress(data, image_buf);
    if let Err(ref err) = res {
        let is_empty = pixels.iter().all(|&p| p == 0);
        if is_empty {
            return Err(err.to_string());
        }
    }

    let image = image::RgbaImage::from_raw(
        width as u32, 
        height as u32, 
        pixels,
        )
        .ok_or_else(|| {"バッファからRgbaImageの生成に失敗しました".to_string()
    })?;

    Ok(resize_thumbnail(
        &image,
        max_width,
        max_height,
    ))

}

// 通常画像読み込み
fn load_full(data: &[u8]) -> Result<image::RgbaImage, String> {

    if let Ok(rgba_image) = decompress_turbojpeg_tolerant(&data) {
        return Ok(rgba_image);
    }

    image::load_from_memory(&data) 
            .map(|image| image.to_rgba8())
            .map_err(|e| {format!("画像復号失敗（turbojpeg / image 共に失敗）: {}", e)
            })

}

fn resize_thumbnail(
    image: &image::RgbaImage,
    max_width: u32,
    max_height: u32,
) -> image::RgbaImage {
    let width = image.width();
    let height = image.height();

    let scale = (max_width as f32 / width as f32)
        .min(max_height as f32 / height as f32)
        .min(1.0);

    let new_width = (width as f32 * scale).round() as u32;
    let new_height = (height as f32 * scale).round() as u32;

    image::imageops::resize(
        image, 
        new_width,
        new_height,
        image::imageops::FilterType::Triangle,
    )
}

/// turbojpeg を使用してエラー（破損）直前までの復号を試みる関数
fn decompress_turbojpeg_tolerant(data: &[u8]) -> Result<image::RgbaImage, String> {
    let mut decompressor = turbojpeg::Decompressor::new()
        .map_err(|e| e.to_string())?;

    // ヘッダ情報（画像サイズ）を取得
    let header = decompressor.read_header(data)
        .map_err(|e| e.to_string())?;

    let width = header.width;
    let height = header.height;
    // RGBA 用バッファ
    let mut pixels = vec![0u8; width * height * 4]; 

    // 出力バッファの割り当て（RGBA フォーマット）
    let image_buf = turbojpeg::Image {
        pixels: &mut pixels[..],
        width,
        pitch: width * 4,
        height,
        format: turbojpeg::PixelFormat::RGBA,
    };

    // 復号の実行（フラグで不完全なストリームの受容を許可）
    let res = decompressor.decompress(data, image_buf);

    if let Err(ref err) = res {
        let is_empty = pixels.iter().all(|&p| p == 0);
        if is_empty {
            return Err(err.to_string());
        }
    }

    image::RgbaImage::from_raw(width as u32, height as u32, pixels)
        .ok_or_else(|| "バッファからの RgbaImage 生成に失敗しました".to_string())

} 

pub fn is_jpeg(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref(),
        Some("jpg") | Some("jpeg")
    )
}