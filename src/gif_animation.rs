use std::path::Path;
use std::time::Duration;
use image::{AnimationDecoder, Delay, Frame,};
use image::codecs::gif::{GifDecoder, GifEncoder, Repeat,};

use crate::image_content::ImageContent;
use crate::image_operation::Rotation;


pub struct GifAnimation {
    frames: Vec<image::RgbaImage>,
    delays: Vec<std::time::Duration>,
    current_frame: usize,
    rotation: Rotation,
}

impl GifAnimation {
    /// GIFファイルを読み込んでGifAnimationを作成する
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let decoder = GifDecoder::new(reader)?;
        let frame_iterator = decoder.into_frames();
        //正常に読み込めたフレームだけ保存する
        let mut frames = Vec::new();
        for result in frame_iterator {
            match result {
                Ok(frame) => {
                    frames.push(frame);
                }
                Err(err) => {
                    eprintln!("GIF frame decode waring: {}", err);
                    break;   
                }
            }
        }

        if frames.is_empty() {
            return Err("GIFにフレームがありません".into());
        }

        let mut images = Vec::with_capacity(frames.len());
        let mut delays = Vec::with_capacity(frames.len());

        for frame in frames {
            images.push(frame.buffer().clone());
            let delay = frame.delay();

            let (numerator, denominator) = delay.numer_denom_ms();
            let millis = if denominator == 0 {
                0
            } else {
                numerator / denominator
            };
            delays.push(Duration::from_millis(millis as u64));
        }

        Ok(Self { frames: images, delays, current_frame: 0, rotation: Rotation::None, })
    }

    /// フレーム数を取得
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// フレーム番号を取得
    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    /// フレーム画像を取得
    pub fn current_image(&self) -> &image::RgbaImage {
        &self.frames[self.current_frame]
    }

    /// フレーム表示時間を取得
    pub fn current_delay(&self) -> Duration  {
        self.delays[self.current_frame]
    }

    /// 次のフレームへ移動
    ///
    /// 最終フレームの場合は先頭フレームへ戻る
    pub fn next_frame(&mut self) {
        self.current_frame += 1;

        if self.current_frame >= self.frames.len() {
            self.current_frame = 0;
        }
    }

    /// 指定したフレームへ移動
    pub fn set_frame(&mut self, frame:usize) {
        if frame < self.frames.len() {
            self.current_frame = frame;
        }
    }

}    

impl ImageContent for GifAnimation {
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
        let file = std::fs::File::create(path)
            .map_err(|e| format!("GIFファイルを作成できません: {}", e))?;

        let writer = std::io::BufWriter::new(file);
        let mut encoder = GifEncoder::new(writer);
    
        // アニメーションGIFとして無限ループ
        encoder
            .set_repeat(Repeat::Infinite)
            .map_err(|e| format!("GIFの繰り返し設定に失敗しました: {}", e))?;

        for (frame, delay) in 
            self.frames.iter().zip(self.delays.iter()) {
            // 回転状態を全フレームに適用
            let rotated = 
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
            // Duration -> image::Deley
            let delay = Delay::from_saturating_duration(*delay);
            let gif_frame = Frame::from_parts(
                rotated,
                0,
                0,
                delay,
            );

            encoder
                .encode_frame(gif_frame)
                .map_err(|e| format!("GIFフレームの保存に失敗しました: {}", e))?;
            
        }

        Ok(())
    }

}