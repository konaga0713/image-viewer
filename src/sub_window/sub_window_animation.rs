///sub_window_animation

use super::SubWindow;

impl SubWindow {
    // ------------------------------------------------------------
    //animation 更新処理
    // ------------------------------------------------------------
    pub fn update_animation(&mut self, ctx: &egui::Context,) {
        let Some(image) = &mut self.image 
        else {
            return;
        };

        if !image.is_animated() {
            return;
        }

        // アニメーション開始時に次フレームの表示時刻を設定
        if self.animation_next_frame_time.is_none() {
            if let Some(delay) = image.current_delay(){
                self.animation_next_frame_time = 
                    Some(std::time::Instant::now() + delay);
            }
        }

        let now = std::time::Instant::now();
        // 次フレームの時刻になったらフレームを進める
        if let Some(next_time) = self.animation_next_frame_time {
            if now >= next_time {
                if let Some(delay) = image.next_frame() {
                    self.animation_next_frame_time = Some(now + delay);
                } else {
                    self.animation_next_frame_time = None;
                }
            }
        }

        // 次フレームの時刻まで再描画を待つ
        if let Some(next_time) =self.animation_next_frame_time{
            ctx.request_repaint_after(
                next_time.saturating_duration_since(now));
        }

        // 現在フレームをテクスチャへ反映
        let rgba = image.current_image();
        let size =[
            rgba.width() as usize,
            rgba.height() as usize, 
        ];
        let color_image =
            egui::ColorImage::from_rgba_unmultiplied(
                size,
                rgba.as_raw(),
            );

        self.texture = Some(
            ctx.load_texture(
                self.current_path.to_string_lossy(),
                color_image,
                Default::default(),
            )
        );                   
    }
}