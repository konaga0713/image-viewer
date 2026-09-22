//image_content
use std::path::Path;
use std::time::Duration;

use crate::image_operation::Rotation;
use crate::static_image::StaticImage;
use crate::gif_animation::GifAnimation;
use crate::webp_animation::WebpAnimation; 

pub enum ImageContentType {
    Static(StaticImage),
    Gif(GifAnimation),
    Webp(WebpAnimation),
}
pub trait ImageContent: Send {
    /// 現在表示すべき画像をRGBA形式で取得
    fn current_image(&self) -> &image::RgbaImage;
    /// 現在の画像サイズ
    fn size(&self) ->  egui::Vec2;

    /// 回転
    fn rotate_right(&mut self);
    fn rotate_left(&mut self);
    fn rotation(&self) -> Rotation;

    /// アニメーション画像
    fn is_animated(&self) -> bool {
        false
    }
    fn next_frame(&mut self) -> Option<Duration> {
        None
    }
    fn current_delay(&self) -> Option<Duration> {
        None
    }
    fn frame_count(&self) -> usize {
        1
    }
    
    fn is_modified(&self) -> bool {
        false
    }
    ///　保存
    fn save(&self, path: &Path) -> Result<(), String>; 

    /// ロード中
    fn process_loading(&mut self) -> bool {
        false
    }

    /// loading状態
    fn is_loading(&self) -> bool {
        false
    }

    fn update_loading(&mut self) -> bool {
        false
    }
    
}

