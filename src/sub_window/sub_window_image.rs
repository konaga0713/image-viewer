///sub_window_image

use super::SubWindow;
use crate::jpeg_loader;

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
        let use_original_texture = 
            self.image.as_ref().is_some_and(|image|
                !image.is_animated());

        // 表示するTextureを決定
        if self.fit_to_screen && use_original_texture {
            if self.texture.is_none() {
                self.update_texture(ctx);
            }
        } else if self.fit_to_screen {
            self.ensure_display_texture(ctx, available_size);
        } else if self.texture.is_none() {
            self.update_texture(ctx);
        };

        // 表示Textureを取得
        let Some(texture) = 
            (if self.fit_to_screen && !use_original_texture {
                self.display_texture.as_ref()
            } else {
                self.texture.as_ref()
            })
        else {
            return;
        };

        let image_size = texture.size_vec2() * self.zoom_scale;

        let display_size = if self.fit_to_screen {
            let fit_scale = (available_size.x / image_size.x)
                .min(available_size.y / image_size.y)
                .min(1.0);

                image_size * fit_scale
            } else {
                image_size
            };

println!(
    "[IMAGE] texture={}x{}, available={}x{}",
    texture.size()[0],
    texture.size()[1],
    available_size.x,
    available_size.y,
);

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

    fn ensure_display_texture(&mut self, ctx: &egui::Context, available_size: egui::Vec2) {

        let Some(image) = &self.image else {
            return;
        };

        let rgba = image.current_image();

        let original_width = rgba.width();
        let original_height = rgba.height();

        if  original_width == 0 || original_height == 0 {
            return;
        }

         // 元画像サイズ
        let image_size = egui::Vec2::new(
            original_width as f32,
            original_height as f32,
        ) * self.zoom_scale;

        //  fit用スケール
        let scale_x = available_size.x / image_size.x;
        let scale_y = available_size.y / image_size.y;

        let scale = scale_x.min(scale_y).min(1.0);

        // 表示用Textureのサイズ
        let target_width = 
            (image_size.x * scale).round().max(1.0) as u32;
        let target_height = 
            (image_size.y * scale).round().max(1.0) as u32;

        let target_size = [
            target_width,
            target_height,            
        ];

        let is_animated = image.is_animated();
        if !is_animated 
            && self.display_texture_size == Some(target_size) 
            && self.display_texture.is_some()
        {
            return;
        }

        // ***********************************************
        // JPEG
        // ***********************************************
        if jpeg_loader::is_jpeg(&self.current_path) 
            && !image.is_modified() {

            let scaled = match jpeg_loader::load_scaled(
                &self.current_path,
                target_width,
                target_height,
                ) {
                    Ok(image) => image,
                    Err(e) => {
                        eprintln!("[JPEG SCALE ERROR] {}", e);
                        return;
                    }
                };

            // println!(
            //     "[JPEG SCALE] texture image={}x{}, target={}x{}",
            //     scaled.width(),
            //     scaled.height(),
            //     target_width,
            //     target_height,
            // );    
            // let start = std::time::Instant::now();

            let color_image =
                egui::ColorImage::from_rgba_unmultiplied(
                    [
                        scaled.width() as usize,
                        scaled.height() as usize,
                        ],
                    scaled.as_raw(),
                );

            // println!("[TIME] JPEG ColorImage: {:?}",start.elapsed());
            // let start = std::time::Instant::now();

            // 表示用Texture作成
            self.display_texture = Some(
                ctx.load_texture(
                    format!(
                        "{}-display-{}x{}",
                        self.current_path.to_string_lossy(),
                        target_width,
                        target_height,
                    ),
                    color_image,
                    Default::default(),
                )
            );

            // println!("[TIME] JPEG load_texture: {:?}",start.elapsed());

            self.display_texture_size = Some([
                target_width,
                target_height,]);
            return;
        }

        // ***********************************************
        // JPEG以外
        // ***********************************************

        // リサイズ
        let resized = image::imageops::resize(
            rgba,
            target_width,
            target_height,
            image::imageops::FilterType::Triangle,
        );

        // println!("[TIME] display resize: {:?}",start.elapsed());
        // let start = std::time::Instant::now();

        let color_image =
            egui::ColorImage::from_rgba_unmultiplied(
                [
                    target_width as usize,
                    target_height as usize,
                    ],
                resized.as_raw(),
            );


        // println!("[TIME] display ColorImage: {:?}",start.elapsed());
        // let start = std::time::Instant::now();

        // 表示用Texture作成
        self.display_texture = Some(
            ctx.load_texture(
                format!(
                    "{}-display-{}x{}",
                    self.current_path.to_string_lossy(),
                    target_width,
                    target_height,
                ),
                color_image,
                Default::default(),
            )
        );
        // println!("[TIME] display load_texture: {:?}",start.elapsed());

        self.display_texture_size = Some(target_size);

    }

 }

