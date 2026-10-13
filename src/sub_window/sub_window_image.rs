///sub_window_image

use super::SubWindow;

impl SubWindow {
    //回転処理　左
    pub fn do_rotate_left(
        &mut self, 
        ctx: &egui::Context, 
    ) {
        let Some(image) = 
            &mut self.image 
        else {
            return;
        };

        image.rotate_left();

        let rgba = image.current_image();
        let size = [
            rgba.width() as usize,
            rgba.height() as usize,
        ];
        let color_image = 
            egui::ColorImage::from_rgba_unmultiplied(
                size, rgba.as_raw(),
        );
        self.texture = 
            Some(ctx.load_texture(
                self.current_path.to_string_lossy(),
                color_image,
                Default::default(),
            )
        );

        // 回転後の画像サイズを保存
        self.original_image_size = Some(image.size());
        self.display_texture = None; // 表示用Textureをクリアして再生成
        self.display_texture_size = None; // 表示用Textureサイズをクリア
        if self.fit_to_screen {
            self.resize_window_to_image(ctx);
        }
        ctx.request_repaint(); // UIの再描画を要求
     }

    //回転処理　右
    pub fn do_rotate_right(
        &mut self, 
        ctx: &egui::Context, 
    ) {
        let Some(image) = 
            &mut self.image 
        else {
            return;
        };

        image.rotate_right();

        let rgba = image.current_image();
        let size = [
            rgba.width() as usize,
            rgba.height() as usize,
        ];
        let color_image = 
            egui::ColorImage::from_rgba_unmultiplied(
                size, rgba.as_raw(),
        );
        self.texture = 
            Some(ctx.load_texture(
                self.current_path.to_string_lossy(),
                color_image,
                Default::default(),
            )
        );

        // 回転後の画像サイズを保存
        self.original_image_size = Some(image.size());
        self.display_texture = None;
        self.display_texture_size = None;
        if self.fit_to_screen {
            self.resize_window_to_image(ctx);
        }
        ctx.request_repaint(); // UIの再描画を要求
    }

    // ------------------------------------------------------------
    // 画像表示
    // ------------------------------------------------------------
    pub fn show_texture(&mut self, ui: &mut egui::Ui, ctx:&egui::Context,) {
        let available_size = ui.available_size();

        if self.texture.is_none() {
            self.update_texture(ctx);
        }
        
        let Some(texture) = self.texture.as_ref() else {
            return;
        };

        let texture_size = texture.size_vec2();
        
        // 自動縮小は基準サイズにだけ適用し、その後に手動ズームを掛ける。
        // これにより拡大時は表示領域からはみ出し、スクロール可能になる。
        let fit_scale = if self.fit_to_screen
            && texture_size.x > 0.0
            && texture_size.y > 0.0
        {
            (available_size.x / texture_size.x)
                .min(available_size.y / texture_size.y)
                .min(1.0)
        } else {
            1.0
        };
        
        let display_size = texture_size * fit_scale * self.zoom_scale;

        if let Some(original_size) = self.original_image_size {
            if original_size.x > 0.0 {
                self.display_scale = display_size.x / original_size.x;
            }
        }   

        // スクロールエリアを配置し、基準を左上に設定
        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui|{
                ui.add(
                    egui::Image::new(texture)
                        .fit_to_exact_size(display_size)
                );
            }); 
            
    }

}

