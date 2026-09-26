///sub_window_image

use super::SubWindow;

const MIN_FIT_SCALE: f32 = 0.8;

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
        if self.fit_to_screen {
            self.resize_window_to_image(ctx);
        }
        ctx.request_repaint(); // UIの再描画を要求
    }

    // ------------------------------------------------------------
    // 画像表示
    // ------------------------------------------------------------
    pub fn show_texture(&self, ui: &mut egui::Ui,) {

        let Some(texture) = &self.texture 
        else {
            return;
        };

        let image_size = texture.size_vec2() * self.zoom_scale;

        let display_size = 
            if self.fit_to_screen {
                let available_size = ui.available_size();

                let scale_x = available_size.x / image_size.x;
                let scale_y = available_size.y / image_size.y;
                // 1.0を上限にするので、小さい画像は拡大しない
                let scale = scale_x.min(scale_y).min(1.0);
                
println!(
    "[IMAGE] texture={}x{}, available={}x{}, scale={}",
    texture.size()[0],
    texture.size()[1],
    available_size.x,
    available_size.y,
    scale,
);

                image_size * scale
            } else {
                image_size
            };


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

