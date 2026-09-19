    /// 上矢印で移動するフォルダを取得
    fn previous_directory(
        &self,
    ) -> Option<PathBuf> {
        let current_dir = self.current_path.parent()?.to_path_buf();
        let parent_dir = current_dir.parent()?.to_path_buf();
        let sibling_dirs = Self::get_subdirectries(&parent_dir);
        let current_index = sibling_dirs.iter().position(|d| d == &current_dir)?;

        if current_index > 0 {
            Some(sibling_dirs[current_index - 1].clone())
        } else {
            Some(parent_dir.to_path_buf())
        }
    }

    /// フォルダを階層順に取得する
    fn collect_image_directories(
        dir: &std::path::Path,
        plugin_mgr: &Arc<PluginManager>,
        result: &mut Vec<PathBuf>,
    ) {
        let subdirs = Self::get_subdirectries(dir);
        for subdir in subdirs {
            if !Self::get_image_files(&subdir, plugin_mgr).is_empty() {
                result.push(subdir.clone());
            }
            // サブフォルダ内を再帰的に収集
            Self::collect_image_directories(&subdir, plugin_mgr, result);
        }
    }

        fn change_directory(
        &mut self, 
        new_dir: PathBuf, 
        ctx: &egui::Context,    
        plugin_mgr: &Arc<PluginManager>,
    ) {
    //    println!("===== change_directory =====");
    //    println!("new_dir = {:?}", new_dir);

        let image_files = Self::get_image_files(&new_dir, plugin_mgr);

    //    println!("image_files = {:?}", image_files);

        if image_files.is_empty() {
    //    println!("画像がないため移動しません");
            return;
        }

        if new_dir.is_dir() {
            self.current_path = image_files[0].clone();

    //    println!(
    //            "current_path changed to = {:?}",
    //            self.current_path
    //        );

            self.directory_files = image_files;
            self.current_index = 0;

            self.texture = None;
            self.original_image_size = None;
            self.loading = true;

            self.keep_window_reposition = true;
            self.keep_window_on_screen(ctx);

            self.load_image_async(plugin_mgr.clone());
        }
    }

    fn keep_window_on_screen(&self, ctx: &egui::Context) {
        let (outer_rect, monitor_size) = ctx.input( |i| {
            let viewport = i.viewport();
                (viewport.outer_rect, viewport.monitor_size)
        });

        let Some(outer_rect) = outer_rect else {
            return;
        };
        let Some(monitor_size) = monitor_size else {
            return;
        };

        let size = outer_rect.size();

        // ------------------------------------------------------------
        // サブウィンドウ全体がディスプレイより大きい場合
        //
        // resize_window_to_image() 側で
        //   ・上部UI・タイトルバー等 : 120px
        //   ・左右余白               : 20px
        // を考慮してサイズを再計算する。
        // ------------------------------------------------------------
        if size.x > monitor_size.x || size.y > monitor_size.y {
            self.resize_window_to_image(ctx);
            return;
        }         

        // ------------------------------------------------------------
        // ウィンドウ位置をディスプレイ内に収める
        // ------------------------------------------------------------
        let mut pos = outer_rect.min;
        let mut changed = false;

        // 右にはみ出している
        if pos.x + size.x > monitor_size.x {
            pos.x = monitor_size.x - size.x;
            changed = true;
        }

        // 下にはみ出している
        if pos.y + size.y > monitor_size.y {
            pos.y = monitor_size.y - size.y;
            changed = true;
        }

        // 左にはみ出している
        if pos.x < 0.0 {
            pos.x = 0.0;
            changed = true;
        }

        // 上にはみ出している
        if pos.y < 0.0 {
            pos.y = 0.0;
            changed = true;
        }

        // ウィンドウ位置を修正する
        if changed {
            ctx.send_viewport_cmd(
                egui::ViewportCommand::OuterPosition(pos),
            );
        }
    }

        /// 画像のデコード処理を別スレッドで実行する
    fn load_image_async(&mut self, plugin_mgr: Arc<crate::plugin::PluginManager>) {
        self.loading = true;
        let path = self.current_path.clone();
        let tx = self.tx.clone();

        thread::spawn(move || {
            // バックグラウンドでプラグインデコードを実行
            let res = plugin_mgr
                .try_decode(&path)
                .map(|img | {
                    // WSLg保護のための安全サイズ縮小
                    let max_dim = 3840;
                    if img.width() > max_dim || img.height() > max_dim {
                        img.thumbnail(max_dim, max_dim)
                    } else {
                        img
                    }
                })
                .map(LoadedImage::Static)
                .map_err(|e| e.to_string());

            let _ = tx.send((path, res));
        });
    }

    fn load_gif_async(&mut self) {
        self.loading = true;
        let path = self.current_path.clone();
        let tx = self.tx.clone();

        thread::spawn(move || {
            let result = GifAnimation::load(&path)
                .map(LoadedImage::Gif)
                .map_err(|e| e.to_string());

            let _ = tx.send((path, result));   
        });
    }

    fn do_rotate_left(
        &mut self, 
        ctx: &egui::Context, 
        plugin_mgr: &Arc<PluginManager>, 
    ) {
        let Some(image) = 
            &mut self.image 
        else {
            return;
        };

        image.rotate_left();

        if self.fit_to_screen {
            self.zoom_scale = 1.0;
            self.keep_window_reposition = true;
        }

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
        self.original_image_size = Some(image.size());
        if self.fit_to_screen {
            self.resize_window_to_image(ctx);
        }
        ctx.request_repaint(); // UIの再描画を要求
    }

fn main() {
        use std::path::Path;
    let path = Path::new(r#"D:\Projects\Rust\image-viewer\pics\webp\1.webp"#);

    //let path = Path::new(....);
        match webp_decoder::load_webp(path) {
        Ok(img) => {
            println!("Loaded image: {}x{}", img.width(), img.height());
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}

    fn load_thumbnail(
        &mut self,
        path: &PathBuf,
        ctx: &egui::Context,
    ) -> Option<egui::TextureHandle> {
        if let Some(texture) = self.thumbnail_textures.get(path) {
            return Some(texture.clone());
        }

        // 画像を読み込む
        let image = match image::open(path) {
            Ok(image) => image,
            Err(e) => {
                eprintln!("サムネイル読み込み失敗: {}: {}", path.display(), e);
                return None;
            }
        };

        // サムネイルサイズ
        let thumbnail = image.thumbnail(160,120);
        let rgba = thumbnail.to_rgba8();
        let size = [
            rgba.width() as usize,
            rgba.height() as usize,
        ];
        let color_image = 
            egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
        let texture = ctx.load_texture(
            format!("thumbnail:{}", path.display()),
            color_image,
            egui::TextureOptions::LINEAR,
        );

        self.thumbnail_textures
            .insert(path.clone(), texture.clone());

        Some(texture)
    }


pub fn load(path: &Path) -> Result<image::RgbaImage, String> {
    let data = 
        std::fs::read(path)
            .map_err(|e| format!("JPEG読み込み失敗: {}", e))?;

    match turbojpeg::decompress_image(&data) {
        Ok(image) => 
            Ok(image),
        Err(turbo_err) => {
            match image::load_from_memory(&data) {
                Ok(image) => Ok(image.to_rgba8()),
                Err(image_err) => Err(
                    format!("TurboJPEG: {} image : {}",
                        turbo_err,
                        image_err)
                ),
            }
        }

    }

}

