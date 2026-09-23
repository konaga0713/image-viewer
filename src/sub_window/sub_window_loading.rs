///sub_window_loading
use std::thread;
use std::sync::Arc;

use crate::static_image::StaticImage;
use crate::gif_animation::GifAnimation;
use crate::plugin::PluginManager;
use crate::image_cache::ImageCache;

use super::SubWindow;

impl SubWindow {

    /// 画像のデコード処理を別スレッドで実行する
    pub(crate) fn load_async(&mut self, plugin_mgr: Arc<crate::plugin::PluginManager>, image_cache: &mut ImageCache, ctx: egui::Context,) {
        self.loading = true;

        if self.is_gif() {
            let gif = GifAnimation::load_async(
                self.current_path.clone(),
                ctx.clone(),
            );

            self.image = Some(Box::new(gif));

            self.texture = None;
            self.animation_next_frame_time = None;

            return;
        }
        
        // Cache検索
        if let Some(rgba) = image_cache.get(&self.current_path).cloned() {
    
    println!(
        "[CACHE HIT] {}x{}",
        rgba.width(),
        rgba.height()
    );

            let image = StaticImage::new(
                image::DynamicImage::ImageRgba8(rgba)
            );
            self.image = Some(Box::new(image));
            self.original_image_size = self.image.as_ref().map(|i| i.size());

            self.update_texture(&ctx);

            self.resize_pending = true;
            self.loading = false;

            return;
        }

        self.load_normal_async(plugin_mgr, ctx);

    }

    fn load_normal_async(&mut self, plugin_mgr: Arc<PluginManager>,ctx: egui::Context,){
        let path = self.current_path.clone();
        let tx = self.tx.clone();

        thread::spawn(move || {
            // バックグラウンドでプラグインデコードを実行
            let result = plugin_mgr
                .try_decode(&path)
                .map_err(|e| e.to_string());

            let _ = tx.send((path, result));
            // 読み込み完了後にGUIを再描画
            ctx.request_repaint();            
        });
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        let Some(image) = &self.image 
        else {
            return;
        };

        let rgba = image.current_image();
        let size = [
            rgba.width() as usize,
            rgba.height() as usize,
        ];

        let color_image = 
            egui::ColorImage::from_rgba_unmultiplied(
                size, 
                rgba.as_raw(),
            );

println!(
    "[TEXTURE] {}x{}",
    rgba.width(),
    rgba.height()
);


        self.texture = Some(
            ctx.load_texture(
                self.current_path.to_string_lossy(),
                color_image,
                Default::default(),
            )
        );
    }

    fn is_gif(&self) -> bool {
        self.current_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("gif"))
            .unwrap_or(false)
    }

    // ============================================================
    // 画像の読み込み処理
    // ============================================================
    pub fn process_image_loading(&mut self, ctx: &egui::Context, image_cache: &mut ImageCache,) {
        // GIFなど、ImageContent内部の非同期読み込み
        let mut gif_changed = false;

        if let Some(image) = &mut self.image {
            gif_changed = image.update_loading();
            self.loading = image.is_loading();
        }

        if gif_changed {
            println!("[SubWindow] GIF image changed");            
            if let Some(image) = &self.image {
                let rgba = image.current_image();

                let size = [
                    rgba.width() as usize,
                    rgba.height() as usize,
                ];
                let color_image =
                    egui::ColorImage::from_rgba_unmultiplied(
                        size,
                         rgba.as_raw()
                    );
                self.texture = Some(ctx.load_texture(
                    self.current_path.to_string_lossy(),
                    color_image,
                    Default::default(),
                    ));    

                if self.original_image_size.is_none(){
                    let image_size = image.size();                    

                    println!(
                        "[SubWindow] first frame: {}x{}",
                        image_size.x,
                        image_size.y
                    );

                    self.original_image_size = Some(image.size());
                    self.resize_window_to_image(ctx);
                }

                ctx.request_repaint();
            }
        }

        // 通常画像のworker結果
        while let Ok((loaded_path, result)) = self.rx.try_recv() {
            if loaded_path != self.current_path {
                continue;
            }

            self.loading = false;

            match result {
                Ok(image) => {
                    let rgba = image.current_image().clone();
                    
    println!(
        "[SubWindow] loaded image: {}x{}",
        rgba.width(),
        rgba.height()
    );

                    // ImageContentを設定
                    self.original_image_size = Some(image.size());
                    self.image = Some(image);

                    // Static画像をCacheへ保存
                    image_cache.insert(
                        loaded_path.clone(),
                        rgba.clone(),
                    );

                    // Textureを作成
                    self.update_texture(&ctx);

                    //ウィンドウサイズを画面に合わせる
                    self.resize_pending = true;

                    self.loading = false;

                } 

                Err(err_msg) => {
                    eprintln!("Failed to load image: {}", err_msg);
                    self.texture = None;
                    self.image = None; 
                }
            }
        }

    } 

}
