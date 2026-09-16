use std::path::Path;
use std::time::Duration;
use image::RgbaImage;
use std::io::Cursor;
use image_webp::WebPDecoder;

use crate::image_content::ImageContent;
use crate::image_operation::Rotation;

pub struct WebpAnimation {
    frames: Vec<RgbaImage>,
    delays: Vec<std::time::Duration>,
    current_frame: usize,
    rotation: Rotation,
}

impl WebpAnimation {
    /// Webpファイルを読み込んでWebpAnimationを作成する
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {

        let data = std::fs::read(path)?;
        let mut decoder = 
            WebPDecoder::new(Cursor::new(&data))?;
        
        let (width, height) = decoder.dimensions();    
        println!("WebP dimensions: {} x {}", width, height);

        // WebPがアニメーションか確認
        let is_animated = decoder.is_animated();
        println!("WebP animated: {}", is_animated);

        let has_alpha = decoder.has_alpha();
        println!("WebP has alpha: {}", has_alpha);

        // --------------------------------------------------
        // 静止WebP
        // --------------------------------------------------
        if !is_animated {
            let output_size = 
                decoder
                    .output_buffer_size()
                    .ok_or("WebP output buffer size is too large")?;

            let mut decoded = vec![0u8; output_size];
            decoder.read_image(&mut decoded)?;

            let image = 
                if has_alpha {
                    if decoded.len() != (width * height * 4) as usize {
                        return Err( "Unexpected WebP RGBA buffer size".into());
                    }

                    RgbaImage::from_raw(
                        width, 
                        height,
                        decoded,)
                        .ok_or("WebP画像の作成に失敗しました")?
                } else {
                    if decoded.len() != (width * height * 3) as usize {
                        return Err( "Unexpected WebP RGB buffer size".into());
                    }

                    let mut rgba = 
                        RgbaImage::new(width, height);

                    for (src, dst) in decoded
                        .chunks_exact(3)
                        .zip(rgba.chunks_exact_mut(4)) {
                            dst[0] = src[0];
                            dst[1] = src[1];
                            dst[2] = src[2];
                            dst[3] = 255;
                    }
                    rgba
                };

            return Ok(Self {
                frames: vec![image],
                delays: vec![Duration::from_millis(0)],
                current_frame: 0,
                rotation: Rotation::None,
            });
        }
        // --------------------------------------------------
        // アニメーションWebP
        // --------------------------------------------------

        let frame_count = decoder.num_frames();
        if frame_count == 0 {
            return Err("WebPにフレームがありません".into());
        }

        let output_size = decoder
            .output_buffer_size()
            .ok_or("WebP output buffer size is too large")?;

        let mut images = Vec::with_capacity(frame_count as usize);
        let mut delays = Vec::with_capacity(frame_count as usize);

        for _ in 0..frame_count {
            let mut decoded = vec![0u8; output_size];
            
            let delay_ms = decoder.read_frame(&mut decoded)?;
            let image = RgbaImage::from_raw(
                width, height, decoded,)
                .ok_or("WebPフレーム画像の作成に失敗しました")?;

            images.push(image);
            delays.push(Duration::from_millis(delay_ms as u64));
        } 

println!(
        "WebP loaded: frames={}, delays={:?}",
        images.len(),
        delays
    );

        Ok(Self { frames: images, delays, current_frame: 0, rotation: Rotation::None, })
        
    }

    fn save_animation(&self, path: &Path) -> Result<(), String> {
        use webp_animation::Encoder;

        if self.frames.is_empty() {
            return Err("WebPにフレームがありません".to_string());
        }

        // 1フレーム目のサイズを取得
        let (width, height) = 
            match self.rotation {
                Rotation::None | Rotation::Rotate180 => {(
                    self.frames[0].width(),
                    self.frames[0].height(),
                )}
                Rotation::Left | Rotation::Right => {(
                    self.frames[0].height(),
                    self.frames[0].width(),
                )}
            };

        let mut encoder = Encoder::new((width, height))
            .map_err(|e| format!("WebP Encoder作成失敗: {}", e))?;

        // 各フレームの開始時刻
        let mut timestamp_ms: i32 = 0;
        for (frame, delay) in 
            self.frames.iter().zip(self.delays.iter()) {
            
            // 回転を適用
            let rotated_frame = 
                match self.rotation {
                    Rotation::None => frame.clone(),
                    Rotation::Right => {
                        image::DynamicImage::ImageRgba8(frame.clone())
                        .rotate90()
                        .to_rgba8()
                    }
                    Rotation::Rotate180 => {
                        image::DynamicImage::ImageRgba8(frame.clone())
                        .rotate180()
                        .to_rgba8()
                    }
                    Rotation::Left => {
                        image::DynamicImage::ImageRgba8(frame.clone())
                        .rotate270()
                        .to_rgba8()
                    }
                };
            
            // RGBAデータを追加   
            encoder
                .add_frame(
                    rotated_frame.as_raw(),
                    timestamp_ms,)
                .map_err(|e| format!("WebPフレーム追加失敗: {}", e))?;

            // 次フレームの開始時刻へ
            let delay_ms = delay.as_millis() as i32;
            timestamp_ms = timestamp_ms
                    .checked_add(delay_ms)
                    .ok_or_else(|| "WebPアニメーション時間が長すぎます".to_string())?;
        }

        // 最後のフレームが表示される時間を含めた終了時刻
        let final_timestamp_ms = timestamp_ms;
        let webp_data = encoder
            .finalize(final_timestamp_ms)
            .map_err(|e| format!("WebPアニメーション生成失敗: {}", e))?;
        std::fs::write(path, &*webp_data)
            .map_err(|e| format!("WebPファイル保存失敗: {}", e))?;
        


        Ok(())
    }

}


impl ImageContent for WebpAnimation {
    /// 現在表示すべき画像をRGBA形式で取得
    fn current_image(&self) -> image::RgbaImage {
        let frame = &self.frames[self.current_frame];

        match self.rotation {
            Rotation::None => frame.clone(),
            Rotation::Right => {
                image::DynamicImage::ImageRgba8(frame.clone())
                    .rotate90()
                    .to_rgba8()
            }
            Rotation::Rotate180 => {
                image::DynamicImage::ImageRgba8(frame.clone())
                    .rotate180()
                    .to_rgba8()
            } 
            Rotation::Left => {
                image::DynamicImage::ImageRgba8(frame.clone())
                    .rotate270()
                    .to_rgba8()
            } 
        }

    }

    /// 現在の画像サイズ
    fn size(&self) ->  egui::Vec2 {
        let frame = &self.frames[self.current_frame];
        match self.rotation {
            Rotation::None | Rotation::Rotate180 => {
                egui::vec2(
                    frame.width() as f32,
                    frame.height() as f32,
                )
            }
            Rotation::Right | Rotation::Left => {
                egui::vec2(
                    frame.height() as f32,
                    frame.width() as f32,
                )
            }

        }
    }

    /// 右回転
    fn rotate_right(&mut self) {
        self.rotation = self.rotation.rotate_right();
    }
    /// 左回転
    fn rotate_left(&mut self) {
        self.rotation = self.rotation.rotate_left();
    }
    /// 回転状態
    fn rotation(&self) -> Rotation {
        self.rotation
    }
    /// 保存判定
    fn is_modified(&self) -> bool {
        self.rotation != Rotation::None
    }
    /// アニメーション画像か
    fn is_animated(&self) -> bool {
        self.frames.len() > 1
    }
    /// 次のフレームへ進む
    /// アニメーションでない場合は None
    fn next_frame(&mut self) -> Option<Duration> {
        if self.frames.is_empty() {
            return None;
        }

        self.current_frame = 
            (self.current_frame + 1) % self.frames.len();
        self.delays
            .get(self.current_frame)
            .copied()    
    }
      /// 現在フレームの表示時間
    fn current_delay(&self) -> Option<std::time::Duration> {
        self.delays
            .get(self.current_frame)
            .copied()
    }
    /// フレーム数
    fn frame_count(&self) -> usize {
        self.frames.len()
    }
    
    ///　保存
    fn save(&self, path: &Path) -> Result<(), String> {
        self.save_animation(path)
    }

}