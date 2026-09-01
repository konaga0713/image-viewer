use egui::TextureHandle;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::sync::Arc;
use crate::plugin::PluginManager;

pub struct SubWindow {
    pub id: egui::ViewportId,
    pub current_path: PathBuf,
    pub directory_files: Vec<PathBuf>, /// フォルダ内の全画像一覧（前後移動用）
    pub directory_history: Vec<PathBuf>, /// フォルダ移動履歴
    pub current_index: usize,
    pub fit_to_screen: bool,     /// オプション: 自動縮小モード
    pub zoom_scale: f32,         /// 手動拡大縮小用スケール
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
        
        let current_index = directory_files.iter().position(|p| p == &path).unwrap_or(0);

        let mut sub_win = Self {
            id,
            current_path: path,
            directory_files,
            directory_history: Vec::new(), 
            current_index,
            fit_to_screen,
            zoom_scale: 1.0,
            texture: None,
            original_image_size: None,
            loading: false,
            keep_window_reposition: false,
            tx,
            rx,
        };

        sub_win.load_image_async(plugin_mgr);
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

    pub fn ui(&mut self, ctx: &egui::Context, plugin_mgr: &Arc<crate::plugin::PluginManager>)  {
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
        // ============================================================

        // ------------------------------------------------------------
        // ← 前の画像
        // ------------------------------------------------------------
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            if self.current_index > 0 {
                self.current_index -= 1;
                self.current_path = self.directory_files[self.current_index].clone();

                self.texture = None; // 前の画像を破棄してメモリ解放
                self.loading = true;
                // サブウィンドウを画面内へ戻す
                self.keep_window_reposition = true;
                self.keep_window_on_screen(ctx);
                // 新しい画像を非同期で読み込む
                self.load_image_async(plugin_mgr.clone());
            }
        }

        // ------------------------------------------------------------
        // → 次の画像
        // ------------------------------------------------------------        
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            if self.current_index + 1 < self.directory_files.len() {
                self.current_index += 1;
                self.current_path = self.directory_files[self.current_index].clone();
                self.texture = None; // 前の画像を破棄してメモリ解放
                self.loading = true;
                // サブウィンドウを画面内へ戻す
                self.keep_window_reposition = true;
                self.keep_window_on_screen(ctx);
                // 新しい画像を非同期で読み込む
                self.load_image_async(plugin_mgr.clone());
            }
        }

        // ------------------------------------------------------------
        // ↑ 前のフォルダ
        // ------------------------------------------------------------
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            if let Some(prev_dir) = self.directory_history.pop() {
                self.change_directory(prev_dir, ctx, plugin_mgr);
            }
        }

        // ------------------------------------------------------------
        // ↓ 次のフォルダ
        // ------------------------------------------------------------
//DBG
        /*         if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            if let Some(next_dir) = self.next_directory(plugin_mgr) {
                self.change_directory(next_dir, ctx, plugin_mgr);
            }
        }
*/
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            if let Some(next_dir) = self.next_directory(plugin_mgr) {

                // 現在のフォルダを履歴に保存
                if let Some(current_dir) = self.current_path.parent() {
                    self.directory_history.push(current_dir.to_path_buf());
                }
                
                self.change_directory(next_dir, ctx, plugin_mgr);

                println!(
                    "after change_directory current_path = {:?}",
                    self.current_path
                );
            } else {
                println!("next_directory returned None");
            }
        }

//DBG
        // UIの描画
        egui::TopBottomPanel::top("sub_top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.fit_to_screen, "画面に合わせて自動縮小");
                ui.label(format!("拡大率: {:.0}%", self.zoom_scale * 100.0));
                if ui.button("リセット").clicked() {
                    self.zoom_scale = 1.0;
                }
                ui.label(format!(" ( {} / {} )", self.current_index + 1, self.directory_files.len()));

                if self.loading {
                    ui.spinner();
                    ui.label("Loading...");
                }    
            });
        });

        // 画像表示エリア（原寸・自動縮小・左上基準）
        egui::CentralPanel::default().show(ctx, |ui| {
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
//DBG
/*
    fn change_directory(
        &mut self, 
        new_dir: PathBuf, 
        ctx: &egui::Context,    
        plugin_mgr: &Arc<PluginManager>,
    ) {
        let image_files = Self::get_image_files(&new_dir, plugin_mgr);

        // 画像がないフォルダには移動しない
        if image_files.is_empty() {
            return;
        }

        if new_dir.is_dir() {
            self.current_path = image_files[0].clone();
            self.directory_files = image_files;
            self.current_index = 0;
            if !self.directory_files.is_empty() {
                self.texture = None; // 前の画像を破棄してメモリ解放
                self.original_image_size = None;
                self.loading = true;
                // サブウィンドウを画面内へ戻す
                self.keep_window_reposition = true;
                self.keep_window_on_screen(ctx);
                // 新しい画像を非同期で読み込む
                self.load_image_async(plugin_mgr.clone());
            }
        }
    }
 */
fn change_directory(
    &mut self, 
    new_dir: PathBuf, 
    ctx: &egui::Context,    
    plugin_mgr: &Arc<PluginManager>,
) {
    println!("===== change_directory =====");
    println!("new_dir = {:?}", new_dir);

    let image_files = Self::get_image_files(&new_dir, plugin_mgr);

    println!("image_files = {:?}", image_files);

    if image_files.is_empty() {
        println!("画像がないため移動しません");
        return;
    }

    if new_dir.is_dir() {
        self.current_path = image_files[0].clone();

        println!(
            "current_path changed to = {:?}",
            self.current_path
        );

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
//DBG
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

//dbg        
//    Self::debug_print(plugin_mgr, &current_dir)?;
//dbg
        // 現在フォルダにサブフォルダがあれば、最初のサブフォルダへ
        let sub_dirs = Self::get_subdirectries(&current_dir);
        for sub_dir in &sub_dirs {
            if !Self::get_image_files(&sub_dir, plugin_mgr).is_empty() {
                println!("child directory = {:?}", sub_dir);
                return Some(sub_dir.clone().to_path_buf());
            }
        }

        // サブフォルダがなければ、親フォルダの兄弟フォルダを探す
        let mut dir = current_dir;

        loop{
            let parent_dir = dir.parent()?.to_path_buf();
            println!("dir        = {:?}", dir);
            println!("parent_dir = {:?}", parent_dir);
                        let sibling_dirs = Self::get_subdirectries(&parent_dir);
            println!("sibling_dirs:");
            for d in &sibling_dirs {
                println!("  {:?}", d);
            }

            let current_index = sibling_dirs.iter().position(|d| d == &dir)?;
        
            println!("current_index = {}", current_index);

            // 現在フォルダより後ろにある兄弟フォルダを探す
            for target_dir in sibling_dirs.iter().skip(current_index + 1) {
                println!("checking target_dir = {:?}", target_dir);

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

    fn debug_print(
        plugin_mgr: &Arc<PluginManager>,
        current_dir: &std::path::PathBuf,
        
    ) -> Option<(PathBuf, PathBuf)> {
        println!("--------------------------------");
        println!("current_dir = {:?}", current_dir);

        let mut dir = current_dir.clone();

        loop {
            let parent_dir = dir.parent()?.to_path_buf();

            println!("dir        = {:?}", dir);
            println!("parent_dir = {:?}", parent_dir);

            let sibling_dirs = Self::get_subdirectries(&parent_dir);

            println!("sibling_dirs:");
            for d in &sibling_dirs {
                println!("  {:?}", d);
            }

            let current_index =
                sibling_dirs.iter().position(|d| d == &dir)?;

            println!("current_index = {}", current_index);

            for target_dir in sibling_dirs.iter().skip(current_index + 1) {
                println!("checking target_dir = {:?}", target_dir);

                if !Self::get_image_files(target_dir, plugin_mgr).is_empty() {
                    println!("FOUND = {:?}", target_dir);
                    return Some((current_dir.clone(), target_dir.clone()));
                }
            }

            dir = parent_dir;
        }        
    }

}
