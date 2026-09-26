//gif_animation
use std::path::{Path, PathBuf};
use std::time::Duration;
use image::{Delay, Frame, RgbaImage};
use image::codecs::gif::{GifEncoder, Repeat,};
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::image_content::ImageContent;
use crate::image_operation::Rotation;
use crate::gif_worker::GifLoadMessage;

pub struct GifAnimation {
    frames: Vec<RgbaImage>,
    delays: Vec<std::time::Duration>,
    current_frame: usize,
    rotation: Rotation,
    display_image: RgbaImage,
    loading: bool,
    rx: Option<Receiver<GifLoadMessage>>,
}


impl GifAnimation {
    /// 空のGIFアニメーションを作成
    pub fn new() -> Self {
        let (_tx, rx) = std::sync::mpsc::channel();
        Self {
            frames: Vec::new(),
            delays: Vec::new(),
            current_frame: 0,
            rotation: Rotation::None,
            display_image: RgbaImage::new(1,1),
            loading: true,
            rx: Some(rx),
        }
    }

    pub fn load_async(path: PathBuf, ctx: egui::Context,) -> Self {
println!("[GIF] load_async START: {:?}", path);

        let (tx, rx) = std::sync::mpsc::channel();

        crate::gif_worker::load_gif_worker(path, tx, ctx);
        let gif = Self {
            frames: Vec::new(),
            delays: Vec::new(),
            current_frame: 0,
            rotation: Rotation::None,
            display_image: RgbaImage::new(1,1),
            loading: true,
            rx: Some(rx),
        };
println!(
        "[GIF] load_async END: loading={}, rx={}, frames={}, image_x={}, image_y={} ",
        gif.loading,
        gif.rx.is_some(),
        gif.frames.len(),
        gif.display_image.width(),
        gif.display_image.height(),
    );

        gif
    }

    pub fn add_frame(&mut self, frame: image::RgbaImage, delay: Duration,){
        self.frames.push(frame);
        self.delays.push(delay);
    }

    /// GIFの読み込み完了
    pub fn finish_loading(&mut self) {
        self.loading = false;
    }

    /// 読み込み済みフレーム数
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// 現在のフレーム番号
    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    /// 現在のフレーム画像
    pub fn current_image_raw(&self) -> &image::RgbaImage {
        &self.display_image
    }

    /// 現在のフレームの表示時間
    pub fn current_delay_raw(&self) -> Duration {
        self.delays[self.current_frame]
    }

    /// 次のフレームへ移動
    pub fn set_frame(&mut self, frame: usize) {
        if frame < self.frames.len() {
            self.current_frame = frame;
            self.update_display_image();
        }
    }

    // ============================================================
    // GIF読み込み処理
    // ============================================================
    fn process_loading_internal(&mut self) -> bool {
        if !self.loading {
            return false;
        }
println!(
        "[GIF] process_loading_internal: rx_exists={}",
        self.rx.is_some()
    );

        let Some(rx) = self.rx.take()
        else {
println!("[GIF] RX is NONE");            
            return false;
        };

        let mut changed = false;
        let mut finished = false;

        while let Ok(message) = rx.try_recv() {
println!("[GIF] MESSAGE RECEIVED");            
            match message {
                GifLoadMessage::Frame { image , delay } => {
                    self.frames.push(image);
                    self.delays.push(delay);

                    if self.frames.len() == 1 {
                        self.current_frame = 0;
                        self.update_display_image();
                    }
                    changed = true;
                }
                GifLoadMessage::Finished => {
                    // GIF全フレームの読み込み完了
                    self.loading = false;
                    finished = true;
                    changed = true;
                }
                GifLoadMessage::Error(err) => {
                    eprintln!( "Failed to load GIF: {}", err);
                    self.loading = false;                        
                    finished = true;
                }
            }
        }

println!(
        "[GIF] process result: changed={}, finished={}, loading={}, frames={}",
        changed,
        finished,
        self.loading,
        self.frames.len()
    );

        // まだ読み込み中ならReceiverを戻す
        if !finished {
            self.rx = Some(rx);
        }

        changed
    }

    fn update_display_image(&mut self){
        let frame = &self.frames[self.current_frame];

        self.display_image = match self.rotation{
            Rotation::None => { frame.clone()}
            Rotation::Right => {
                image::DynamicImage::ImageRgba8(frame.clone())
                    .rotate90()
                    .to_rgba8()}
            Rotation::Rotate180 => {
                image::DynamicImage::ImageRgba8(frame.clone())
                    .rotate180()
                    .to_rgba8()}
            Rotation::Left => {
                image::DynamicImage::ImageRgba8(frame.clone())
                    .rotate270()
                    .to_rgba8()}
        };
    }

}    

impl ImageContent for GifAnimation {
    /// 現在表示すべき画像をRGBA形式で取得
    fn current_image(&self) -> &image::RgbaImage {
        &self.display_image
    }

    /// 現在の画像サイズ
    fn size(&self) ->  egui::Vec2 {
        let image = &self.display_image;
        egui::vec2(
            image.width() as f32,
            image.height() as f32, 
        )
    }

    /// 現在もGIFを読み込み中か
    fn is_loading(&self) -> bool {
        self.loading
    }

    fn update_loading(&mut self) -> bool {
        self.process_loading_internal()
    }

    /// 右回転
    fn rotate_right(&mut self) {
        self.rotation = match self.rotation {
            Rotation::None => Rotation::Right,
            Rotation::Right => Rotation::Rotate180,
            Rotation::Rotate180 => Rotation::Left,
            Rotation::Left => Rotation::None,
        };
        self.update_display_image();
    }
    /// 左回転
    fn rotate_left(&mut self) {
        self.rotation = match self.rotation {
            Rotation::None => Rotation::Left,
            Rotation::Right => Rotation::None,
            Rotation::Rotate180 => Rotation::Right,
            Rotation::Left => Rotation::Rotate180,
        };
        self.update_display_image();
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

        // まだGIFを読み込み中で、
        // 最後の読み込み済みフレームに到達した場合は待つ
        if self.loading && self.current_frame + 1 >= self.frames.len() {
            return None;
        }

        self.current_frame = 
            (self.current_frame + 1) % self.frames.len();

        self.update_display_image();
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
        if self.frames.is_empty() {
            return Err("GIFにフレームがありません".to_string());
        }

        let file = std::fs::File::create(path)
            .map_err(|e| e.to_string())?;

        let mut encoder = GifEncoder::new(file);
    
        // アニメーションGIFとして無限ループ
        encoder
            .set_repeat(Repeat::Infinite)
            .map_err(|e| format!("GIFの繰り返し設定に失敗しました: {}", e))?;

        for (index, image) in 
            self.frames.iter().enumerate() {

            // 回転状態を全フレームに適用
            let rotated = match self.rotation {
                Rotation::None => image.clone(),
                Rotation::Right => image::imageops::rotate90(image),
                Rotation::Rotate180 => image::imageops::rotate180(image),
                Rotation::Left => image::imageops::rotate270(image),
            };

            // Duration -> image::Deley
            let delay = self
                .delays
                .get(index)
                .copied()
                .unwrap_or(Duration::ZERO);

            let delay_ms = delay.as_millis() as u32;

            let delay = Delay::from_numer_denom_ms(
                delay_ms,
                1
            );

            let frame = Frame::from_parts(
                rotated,
                0,
                0,
                delay,
            );

            encoder
                .encode_frame(frame)
                .map_err(|e| e.to_string())?;
            
        }

        Ok(())
    }

}
