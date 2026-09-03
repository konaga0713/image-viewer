use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::sync::Arc;
use crate::egui::TextureHandle;
use crate::plugin::PluginManager;
use crate::image_operation::Rotation;
pub struct SubWindow {
    pub id: egui::ViewportId,
    pub current_path: PathBuf,
    pub directory_files: Vec<PathBuf>, /// フォルダ内の全画像一覧（前後移動用）
    pub image_index: usize,
    pub folder_history: Vec<PathBuf>, /// フォルダ移動履歴（前後移動用）
    pub fit_to_screen: bool,     /// オプション: 自動縮小モード
    pub zoom_scale: f32,         /// 手動拡大縮小用スケール
    pub rotation: Rotation,     /// 画像回転情報
    
    texture: Option<TextureHandle>,
    original_image_size: Option<egui::Vec2>,
    loading: bool,
    keep_window_reposition: bool,
    // スレッド間通信用チャンネル
    tx: Sender<(PathBuf, Result<image::DynamicImage, String>)>,
    rx: Receiver<(PathBuf, Result<image::DynamicImage, String>)>,
}

impl SubWindow {
    pub fn new(id: egui::ViewportId, path: PathBuf, fit_to_screen: bool, plugin_mgr: Arc<PluginManager>) -> Self {
        let (tx, rx) = channel();

        // 同一ディレクトリ内のファイル一覧を取得（矢印キー移動用）
        let  directory_files = 
            if let Some(parent) = path.parent() {
                Self::get_image_files(parent, &plugin_mgr)
            } else {
                Vec::new()
            };
        
        let image_index = directory_files.iter().position(|p| p == &path).unwrap_or(0);

        let mut sub_win = Self {
            id,
            current_path: path.clone(),
            directory_files,
            image_index,
            folder_history: Vec::new(),
            fit_to_screen,
            zoom_scale: 1.0,
            texture: None,
            original_image_size: None,
            rotation: Rotation::None,
            loading: false,
            keep_window_reposition: false,
            tx,
            rx,
        };

        sub_win.load_image_async(plugin_mgr.clone());
        sub_win.change_directory(path, &egui::Context::default(), &plugin_mgr);
        sub_win 
    }

    /// 画像のデコード処理を別スレッドで実行する
    fn load_image_async(&mut self, plugin_mgr: Arc<crate::plugin::PluginManager>) {
        self.loading = true;
        let path = self.current_path.clone();
        let tx = self.tx.clone();

        thread::spawn(move || {
            // バックグラウンドでプラグインデコードを実行
            let res = plugin_mgr.try_decode(&path)
                .map(|img| {
                    // WSLg保護のための安全サイズ縮小
                    let max_dim = 3840;
                    if img.width() > max_dim || img.height() > max_dim {
                        img.thumbnail(max_dim, max_dim)
                    } else {
                        img
                    }
                })
                .map_err(|e| e.to_string());

            let _ = tx.send((path, res));
        });
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
            if let Ok(img) = result {
                let size = [img.width() as _, img.height() as _];
                let image_buffer = img.to_rgba8();
                let pixels = image_buffer.as_flat_samples();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    size,
                    pixels.as_slice(),
                );
                
                self.texture = Some(ctx.load_texture(
                    self.current_path.to_string_lossy(),
                    color_image,
                    Default::default(),
                ));
            } else {
                if let Err(err_msg) = result {
                    eprintln!("Failed to load image: {}", err_msg);
                }
                self.texture = None;
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

        // ------------------------------------------------------------
        // ← 前の画像
        // ------------------------------------------------------------
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            if self.image_index > 0 {
                self.image_index -= 1;
                self.current_path = self.directory_files[self.image_index].clone();

                self.texture = None; // 前の画像を破棄してメモリ解放
                self.loading = true;
                // サブウィンドウを画面内へ戻す
                self.keep_window_reposition = true;
                self.keep_window_on_screen(&ctx);
                // 新しい画像を非同期で読み込む
                self.load_image_async(plugin_mgr.clone());
            }
        }

        // ------------------------------------------------------------
        // → 次の画像
        // ------------------------------------------------------------        
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            if self.image_index + 1 < self.directory_files.len() {
                self.image_index += 1;
                self.current_path = self.directory_files[self.image_index].clone();
                self.texture = None; // 前の画像を破棄してメモリ解放
                self.loading = true;
                // サブウィンドウを画面内へ戻す
                self.keep_window_reposition = true;
                self.keep_window_on_screen(&ctx);
                // 新しい画像を非同期で読み込む
                self.load_image_async(plugin_mgr.clone());
            }
        }

        // ------------------------------------------------------------
        // ↑ 前のフォルダ
        // ------------------------------------------------------------
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            println!("===== ArrowUp pressed =====");
            println!("current_path = {:?}", self.current_path);
            println!("history = {:?}", self.folder_history);
            
            if let Some(new_dir) = self.previous_directory(plugin_mgr) {
                println!("PREV = {:?}", new_dir);

                self.change_directory(new_dir, &ctx, plugin_mgr);
            } else {
                println!("PREV = None");
            }        
        }

        // ------------------------------------------------------------
        // ↓ 次のフォルダ
        // ------------------------------------------------------------
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            println!("===== ArrowDown pressed =====");
            println!("current_path = {:?}", self.current_path);

            if let Some(new_dir) = self.next_directory(plugin_mgr) {
                println!("NEXT = {:?}", new_dir);
                self.change_directory(new_dir, &ctx, plugin_mgr);
            } else {
                println!("NEXT = None");
            }
        }
        // ------------------------------------------------------------
        // ctrl + ← 左回転
        // ------------------------------------------------------------
//        if i.modifiers.ctrl && i.key_pressed(egui::Key::ArrowLeft) {
//            self.rotate_left();
//        }
        // ------------------------------------------------------------
        // ctrl + → 右回転
        // ------------------------------------------------------------
//        if i.modifiers.ctrl && i.key_pressed(egui::Key::ArrowRight) {
//            self.rotate_right();
//        }

        // ------------------------------------------------------------
        // ctrl + s 画像保存
        // ------------------------------------------------------------
//        if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
//            self.save_image();
//        }

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
                        self.rotate_left();
                        ui.close();
                    }
                    if ui.button("右回転 ctrl + →").clicked() {
                        self.rotate_right();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("画像保存 ctrl + s").clicked() {
                        println!("画像保存");
                        //self.save_image();
                        ui.close();
                    }

                    if ui.button("プロパティ").clicked() {
                        println!("プロパティ");
                        ui.close();
                    }

                });
            })
        });

        // 画像表示エリア（原寸・自動縮小・左上基準）
        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(texture) = &self.texture {
                let available_size = ui.available_size();
                let texture_size = texture.size_vec2();

                // 拡大縮小計算
                let base_size = texture_size * self.zoom_scale;
                // 自動縮小オプション適用 (画面に入りきらない場合のみ左上基準で縮小)
                let display_size = if self.fit_to_screen{
                    let scale_x = available_size.x / texture_size.x;
                    let scale_y = available_size.y / texture_size.y;
                    let fit_scale = scale_x.min(scale_y).min(1.0);
                    base_size * fit_scale
                } else {
                    base_size
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
        });
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

        let mut pos = outer_rect.min;
        let size = outer_rect.size();
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

    /// 指定フォルダにある画像ファイルを取得
    fn get_image_files(
        dir: &std::path::Path,
        plugin_mgr: &Arc<PluginManager>,
    ) -> Vec<PathBuf> {
        let mut files = Vec::new();
     
        if let Ok(entries) = std::fs::read_dir(dir) {
            files = entries
                .filter_map(|entry| 
                    entry.ok().map(|entry| entry.path()))
                .filter(|pb| {
                    pb.is_file() && plugin_mgr.can_decode(pb)
                })
                .collect();
            files.sort();
        }
        files
    }

    fn get_subdirectries(dir: &std::path::Path) -> Vec<PathBuf> {
        let mut subdirs = Vec::new();
     
        if let Ok(entries) = std::fs::read_dir(dir) {
            subdirs = entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|pb| pb.is_dir())
                .collect();
            subdirs.sort();
        }
        subdirs
    }
    fn change_directory(
        &mut self, 
        new_dir: PathBuf, 
        ctx: &egui::Context,    
        plugin_mgr: &Arc<PluginManager>,
    ) {

        // 1. 親ディレクトリを取得
        if let Some(parent) = new_dir.parent() {
            // 2. 親ディレクトリ内のフォルダ一覧を取得
            let mut folders = Vec::new();
            if let Ok(entries) = std::fs::read_dir(parent) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        folders.push(path);
                    }
                }
            }

        }

        // 5. 画像一覧を取得        
        self.current_path = new_dir.clone();
println!("new_dir = {:?}", new_dir);
        let image_files = Self::get_image_files(&new_dir, plugin_mgr);
        if image_files.is_empty() {
            return;
        }

        self.current_path = image_files[0].clone();
        self.directory_files = image_files;
        self.image_index = 0;

        self.texture = None;
        self.original_image_size = None;
        self.loading = true;
        self.keep_window_reposition = true;
        self.keep_window_on_screen(ctx);
        self.load_image_async(plugin_mgr.clone());

    }


    /// 下矢印で移動するフォルダを取得
    ///
    /// 優先順位:
    /// 1. サブフォルダ
    /// 2. 次の兄弟フォルダ
    fn next_directory(
        &self,
        plugin_mgr: &Arc<PluginManager>,
    ) -> Option<PathBuf> {
        // 現在表示している画像のフォルダ
        let current_dir = self.current_path.parent()?.to_path_buf();

        // 現在フォルダにサブフォルダがあれば、最初のサブフォルダへ
        let sub_dirs = Self::get_subdirectries(&current_dir);
        for sub_dir in &sub_dirs {
            if !Self::get_image_files(&sub_dir, plugin_mgr).is_empty() {
//                println!("child directory = {:?}", sub_dir);
                return Some(sub_dir.clone().to_path_buf());
            }
        }

        // サブフォルダがなければ、親フォルダの兄弟フォルダを探す
        let mut dir = current_dir;

        loop{
            let parent_dir = dir.parent()?.to_path_buf();
            let sibling_dirs = Self::get_subdirectries(&parent_dir);
/*          println!("dir        = {:?}", dir);
            println!("parent_dir = {:?}", parent_dir);
                        let sibling_dirs = Self::get_subdirectries(&parent_dir);
            println!("sibling_dirs:");

            for d in &sibling_dirs {
                println!("  {:?}", d);
            }
*/
            let image_index = sibling_dirs.iter().position(|d| d == &dir)?;
        
//            println!("image_index = {}", image_index);

            // 現在フォルダより後ろにある兄弟フォルダを探す
            for target_dir in sibling_dirs.iter().skip(image_index + 1) {
//                println!("checking target_dir = {:?}", target_dir);

                if !Self::get_image_files(target_dir, plugin_mgr).is_empty() {
                    println!("FOUND = {:?}", target_dir);
                    return Some(target_dir.clone().to_path_buf());
                }
            }
            // 次の兄弟がなければ、さらに親へ
            dir = parent_dir;
        }

    }

    /// 上矢印で移動するフォルダを取得
    ///
    /// 優先順位:
    /// 1. サブフォルダ
    /// 2. 次の兄弟フォルダ
    fn previous_directory(
        &self,
        plugin_mgr: &Arc<PluginManager>,
    ) -> Option<PathBuf> {
        // 現在表示している画像のフォルダ
        let current_dir = self.current_path.parent()?.to_path_buf();

        // ルートまでたどる
        let mut dir = current_dir.clone();

        loop {
            let parent_dir = dir.parent()?.to_path_buf();
            let sibling_dirs = Self::get_subdirectries(&parent_dir);
            let index = sibling_dirs.iter().position(|d| d == &dir)?;

            // 現在フォルダより前にある兄弟フォルダを探す
            for target_dir in sibling_dirs.iter().take(index).rev() {
                if Self::get_image_files(target_dir, plugin_mgr).is_empty() {
                    continue;
                } else {
                    return Some(Self::last_directory(target_dir, plugin_mgr));
                }
            }

            // 親フォルダに戻る
            if !Self::get_image_files(&parent_dir, plugin_mgr).is_empty() {
                return Some(parent_dir);
            }
            dir = parent_dir;
        }
    }

    /// 指定フォルダ以下で、深さ優先順の最後に位置する
    /// 「画像を持つフォルダ」を取得する。
    fn last_directory(
        dir: &PathBuf,
        plugin_mgr: &Arc<PluginManager>,
    ) -> PathBuf {
        let sub_dirs = Self::get_subdirectries(dir);
        // 最後の子孫から検索
        for sub_dir in sub_dirs.iter().rev() {
            if let Some(last) = Self::last_image_directory(sub_dir, plugin_mgr,) {
                return last;
            }
        }

        if !Self::get_image_files(dir, plugin_mgr).is_empty() {
            return dir.clone();
        }
        dir.clone()
    }
    
    fn last_image_directory(
        dir: &PathBuf,
        plugin_mgr: &Arc<PluginManager>,
    ) -> Option<PathBuf> {

        let sub_dirs = Self::get_subdirectries(dir);

        // 最後の子孫から検索
        for sub_dir in sub_dirs.iter().rev() {

            if let Some(last) = Self::last_image_directory(
                sub_dir,
                plugin_mgr,
            ) {
                return Some(last);
            }
        }

        // 自分自身が画像フォルダなら採用
        if !Self::get_image_files(dir, plugin_mgr).is_empty() {
            return Some(dir.clone());
        }

        None
    }    

    fn rotate_right(&mut self) {
        self.rotation = self.rotation.rotate_right();
    
        println!("Rotated right. New rotation: {:?}", self.rotation.degrees());

        if self.fit_to_screen {
            self.zoom_scale = 1.0;
            self.keep_window_reposition = true;
        }
        self.texture = None; // 画像を破棄してメモリ解放
    }

    fn rotate_left(&mut self) {
        self.rotation = self.rotation.rotate_left();
    
        println!("Rotated left. New rotation: {:?}", self.rotation.degrees());

        if self.fit_to_screen {
            self.zoom_scale = 1.0;
            self.keep_window_reposition = true;
        }
        self.texture = None; // 画像を破棄してメモリ解放
    }

}
