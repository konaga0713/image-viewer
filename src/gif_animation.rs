use std::path::Path;
use std::time::Duration;
use image::AnimationDecoder;

pub struct GifAnimation {
    frames: Vec<image::RgbaImage>,
    delays: Vec<std::time::Duration>,
    current_frame: usize,
}

impl GifAnimation {
    /// GIFファイルを読み込んでGifAnimationを作成する
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let decoder = image::codecs::gif::GifDecoder::new(reader)?;
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

        Ok(Self { frames: images, delays, current_frame: 0, })
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
