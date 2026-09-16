use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::{fs, thread};
use std::sync::Arc;

use crate::egui::TextureHandle;
use crate::plugin::PluginManager;
use crate::image_content::ImageContent;
use crate::image_property::ImageProperty;

pub struct SubWindow {
    pub id: egui::ViewportId,
    pub current_path: PathBuf,
    pub directory_files: Vec<PathBuf>, /// フォルダ内の全画像一覧（前後移動用）
    pub image_index: usize,
    pub folder_history: Vec<PathBuf>, /// フォルダ移動履歴（前後移動用）
    pub fit_to_screen: bool,     /// オプション: 自動縮小モード
    pub zoom_scale: f32,         /// 手動拡大縮小用スケール
    pub show_property: bool,     /// プロパティ表示フラグ
    
    pub texture: Option<TextureHandle>,
    pub image: Option<Box<dyn ImageContent>>,
    pub original_image_size: Option<egui::Vec2>,
    pub loading: bool,
    pub keep_window_reposition: bool,
    pub show_save_confirm: bool,          /// 保存確認ダイアログ
    pub animation_next_frame_time: Option<std::time::Instant>,
    // スレッド間通信用チャンネル
    pub tx: Sender<(PathBuf, Result<Box<dyn ImageContent>, String>)>,
    pub rx: Receiver<(PathBuf, Result<Box<dyn ImageContent>, String>)>,
}

impl SubWindow {
    pub fn new(
        id: egui::ViewportId,
        path: PathBuf,
        fit_to_screen: bool, 
        plugin_mgr: Arc<PluginManager>,
        ctx: &egui::Context,
    ) -> Self {
        let (tx, rx) = channel();

        // 同一ディレクトリ内のファイル一覧を取得（矢印キー移動用）
        let  directory_files = 
            if let Some(parent) = path.parent() {
                Self::get_image_files(parent, &plugin_mgr)
            } else {
                Vec::new()
            };
        
        let image_index = directory_files.iter().position(|p| p == &path).unwrap_or(0);

        let mut sub_window = Self {
            id,
            current_path: path.clone(),
            directory_files,
            image_index,
            folder_history: Vec::new(),
            fit_to_screen,
            zoom_scale: 1.0,
            texture: None,
            image: None,
            original_image_size: None,
            show_property: false,
            loading: false,
            keep_window_reposition: false,
            show_save_confirm: false,
            animation_next_frame_time: None,
            tx,
            rx,
        };

        sub_window.load_async(plugin_mgr.clone(), ctx.clone(),);

        sub_window 
    }

    /// 画像のデコード処理を別スレッドで実行する
    pub(crate) fn load_async(&mut self, plugin_mgr: Arc<crate::plugin::PluginManager>, ctx: egui::Context,) {
        self.loading = true;
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

    //fn resize_for_display(&mut self, max_dim: u32);

    fn is_gif(&self) -> bool {
        self.current_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("gif"))
            .unwrap_or(false)
    }

    pub fn ui(
        &mut self, 
        ui: &mut egui::Ui,
        plugin_mgr: &Arc<crate::plugin::PluginManager>,
    )  {
        let ctx = ui.ctx().clone();

        // 画像の読み込み処理
        while let Ok((loaded_path, result)) = self.rx.try_recv() {
            if loaded_path != self.current_path {
                continue;
            }

            self.loading = false;

            match result {
                Ok(image) => {
                    self.original_image_size = Some(image.size());
                    self.resize_window_to_image(&ctx);    //ウィンドウサイズを画面に合わせる

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
                
                    self.texture = 
                        Some(ctx.load_texture(
                            self.current_path.to_string_lossy(),
                            color_image,
                            Default::default(),
                        )
                    );
                    self.image = Some(image);
                } 

                Err(err_msg) => {
                    eprintln!("Failed to load image: {}", err_msg);
                    self.texture = None;
                    self.image = None; 
                }
            }
        }
        // ============================================================
        // キーボード操作
        //
        // ← 前の画像
        // → 次の画像
        // ↑ 前のフォルダ
        // ↓ 次のフォルダ
        // ctrl + ← 左回転
        // ctrl + → 右回転
        // ctrl + s 画像保存
        // ============================================================

        let ctrl = ctx.input(|i| i.modifiers.ctrl);
        let right = ctx.input(|i| i.key_pressed(egui::Key::ArrowRight));
        let left = ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft));
        let up = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));
        let down = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
        let key_t = ctx.input(|i| i.key_pressed(egui::Key::T));
        let key_b = ctx.input(|i| i.key_pressed(egui::Key::B));

        match(ctrl, left, right) {
            (true, true, false) => self.do_rotate_left(&ctx),          // ctrl + ← 左回転
            (true, false, true) => self.do_rotate_right(&ctx),     // ctrl + → 右回転
            (false,true,false)=> self.previous_image(&ctx,plugin_mgr),   // ← 前の画像
            (false,false,true)=> self.next_image(&ctx,plugin_mgr), // → 次の画像
            _ => {}
        }

        match(ctrl, key_t, key_b) {
            (true, true, false) => self.move_to_image(0, &ctx, plugin_mgr),          // ctrl + T top画像 
            (true, false, true) => {                    
                // ctrl + B Bottom画像
                if !self.directory_files.is_empty() {
                    let last = self.directory_files.len()-1;
                    self.move_to_image(last, &ctx, plugin_mgr);
                }
            }     
            _ => {}
        }

        match(up,down) {
            (true,false) => self.move_to_previous_directory(&ctx, plugin_mgr),  // ↑ 前のフォルダ
            (false,true) => self.move_to_next_directory(&ctx,plugin_mgr),   // ↓ 次のフォルダ
            _ => {}
        }

        // ------------------------------------------------------------
        // ctrl + s 画像保存
        // ------------------------------------------------------------
        if ctrl && ctx.input(|i| i.key_pressed(egui::Key::S)) {
            println!("ctrl + s pressed");

            self.request_save();
       }

        // UIの描画
        egui::Panel::top("sub_top_panel").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.fit_to_screen, "画面に合わせて自動縮小");
                ui.label(format!("拡大率: {:.0}%", self.zoom_scale * 100.0));
                if ui.button("リセット").clicked() {
                    self.zoom_scale = 1.0;
                }
                ui.label(format!(" ( {} / {} )", self.image_index + 1, self.directory_files.len()));

                if self.loading {
                    ui.spinner();
                    ui.label("Loading...");
                }    
            });
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("画像", |ui| {
                    if ui.button("左回転 ctrl + ←").clicked() {
                        self.do_rotate_left(&ctx);
                        ui.close();
                    }
                    if ui.button("右回転 ctrl + →").clicked() {
                        self.do_rotate_right(&ctx);
                        ui.close();
                    }
                    if ui.button("Top画像 ctrl + t").clicked() {
                        self.move_to_image(0, &ctx, plugin_mgr);
                        ui.close();
                    }
                    if ui.button("Bottom画像 ctrl + b").clicked() {
                        if !self.directory_files.is_empty() {
                            let last = self.directory_files.len()-1;
                            self.move_to_image(last, &ctx, plugin_mgr);
                        }
                        ui.close();
                    }
                    ui.separator();

                    if ui.button("画像保存 ctrl + s").clicked() {
                        println!("画像保存");

                        self.request_save();
                        ui.close();
                    }

                    if ui.button("プロパティ").clicked() {
                        self.show_property = true;
                        ui.close();
                    }

                });
            })
        });

        // 画像表示エリア（原寸・自動縮小・左上基準）
        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(texture) = &self.texture {
                let image_size = texture.size_vec2() * self.zoom_scale;

                let display_size = if self.fit_to_screen {
                    let available_size = ui.available_size();
                    let scale_x = available_size.x / image_size.x;
                    let scale_y = available_size.y / image_size.y;
                    // 1.0を上限にするので、小さい画像は拡大しない
                    let scale = scale_x.min(scale_y).min(1.9);
                    image_size * scale
                } else {
                    image_size  // 自動縮小OFFの場合は原寸表示
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

            //animation 更新処理
            if let Some(image) = &mut self.image {
                if image.is_animated() {
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
        });

        // プロパティ表示
        if self.show_property {
            ImageProperty::show(
                &ctx,
                &self.current_path,
                &mut self.show_property,
            );
        }

        if self.show_save_confirm { 
            egui::Window::new("画像保存の確認")
                .collapsible(false)
                .resizable(false)
                .show(&ctx, |ui| {
                    ui.label("この画像を保存しますか？");
                    ui.horizontal(|ui| {
                        if ui.button("保存").clicked() {
                            self.save_current_image();
                            self.show_save_confirm = false;
                        }
                        if ui.button("キャンセル").clicked() {
                            self.show_save_confirm =false;
                        }
                    });
                });
        }


    }

    // ------------------------------------------------------------
    // 画像の移動
    // ------------------------------------------------------------
    fn move_to_image(&mut self, index: usize, ctx: &egui::Context, plugin_mgr: &Arc<PluginManager>,) {
        if index >= self.directory_files.len(){
            return;
        }
        if index == self.image_index {
            return;
        }
        self.image_index = index;
        self.current_path = 
            self.directory_files[index].clone();

            self.texture = None; // 前の画像を破棄してメモリ解放
            self.image = None;
            self.original_image_size = None;
            self.loading = true;
            self.animation_next_frame_time = None;
            // 新しい画像を非同期で読み込む
            self.load_async(plugin_mgr.clone(), ctx.clone(),);
    }

    // ------------------------------------------------------------
    // ← 前の画像
    // ------------------------------------------------------------
    fn previous_image(&mut self, ctx: &egui::Context, plugin_mgr: &Arc<PluginManager>) {            
        if self.image_index > 0 {
            self.move_to_image(
                self.image_index - 1,
                ctx,
                plugin_mgr,    
            );
        }
    }

    // ------------------------------------------------------------
    // → 次の画像
    // ------------------------------------------------------------  
    fn next_image(&mut self, ctx: &egui::Context, plugin_mgr: &Arc<PluginManager>) {
        if self.image_index + 1 < self.directory_files.len() {
            self.move_to_image(
                self.image_index + 1,
                ctx,
                plugin_mgr,    
            );
        }
    }

    // ------------------------------------------------------------
    // ↑ 前のフォルダ
    // ------------------------------------------------------------
    fn move_to_previous_directory (&mut self, ctx: &egui::Context, plugin_mgr: &Arc<PluginManager>) {
        println!("===== ArrowUp pressed =====");
        println!("current_path = {:?}", self.current_path);
        println!("history = {:?}", self.folder_history);

        // ------------------------------------------------
        // 履歴があれば、履歴を優先
        // ------------------------------------------------
        if let Some(previous_dir) = self.folder_history.pop() {
            println!("HISTORY = {:?}", previous_dir);
            self.change_directory(previous_dir, &ctx, plugin_mgr);
            return;
        } else if let Some(new_dir) = self.previous_directory(plugin_mgr) {
            println!("TREE PREV = {:?}", new_dir);
            self.change_directory(new_dir, &ctx, plugin_mgr);
        } else {
            println!("PREV = None");
        }
    }

    // ------------------------------------------------------------
    // ↓ 次のフォルダ
    // ------------------------------------------------------------
    fn move_to_next_directory (&mut self, ctx: &egui::Context, plugin_mgr: &Arc<PluginManager>) {
        println!("===== ArrowDown pressed =====");
        println!("current_path = {:?}", self.current_path);

        if let Some(new_dir) = self.get_next_directory(plugin_mgr) {
            println!("NEXT = {:?}", new_dir);
            // 現在のフォルダを履歴に保存
            if let Some(current_dir) = self.current_path.parent() {
                self.folder_history.push(current_dir.to_path_buf());
            }
            println!("history = {:?}", self.folder_history);                

            self.change_directory(new_dir, &ctx, plugin_mgr);
        } else {
            println!("NEXT = None");
        }
    }

    //回転処理　左
    fn do_rotate_left(
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
    fn do_rotate_right(
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

    /// 画像保存
    fn save_current_image(&self) {    
        let Some(image) = 
            &self.image 
        else {
            return;
        };    
        if let Err(err) = image.save(&self.current_path) {
                eprintln!("{}",err);
        }
    }

    /// 回転などによる変更がある場合は、そのまま保存する。
    /// 変更がない場合は保存確認ダイアログを表示する。
    fn request_save(&mut self) {
        
        let Some(image) = &self.image 
        else {
            return;
        };   

        if image.is_modified() {
            self.save_current_image();
        } else {
            self.show_save_confirm = true;
        }    
    } 
}
