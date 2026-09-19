//jpeg_loader

use std::path::Path;

pub fn load(path: &Path) -> Result<image::RgbaImage, String> {
    let data = 
        std::fs::read(path)
            .map_err(|e| format!("JPEG読み込み失敗: {}", e))?;

    if let Ok(rgba_image) = decompress_turbojpeg_tolerant(&data) {
        return Ok(rgba_image);
    }

    image::load_from_memory(&data) 
            .map(|image| image.to_rgba8())
            .map_err(|e| format!("画像復号失敗（turbojpeg / image 共に失敗）: {}", e))

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