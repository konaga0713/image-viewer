use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::sync::Arc;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::POINT;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW,
    MonitorFromPoint,
    MONITORINFO,
    MONITOR_DEFAULTTONEAREST,
};

use crate::egui::TextureHandle;
use crate::plugin::PluginManager;
use crate::image_content::ImageContent;
use crate::image_property::ImageProperty;
use crate::gif_animation::GifAnimation;
use crate::static_image::StaticImage; 

// 上部UI・タイトルバー等のための余裕
const TOP_MARGIN :f32 = 80.0;
const SIDE_MARGIN:f32 = 0.0;

pub struct SubWindow {
    pub id: egui::ViewportId,
    pub current_path: PathBuf,
    pub directory_files: Vec<PathBuf>, /// フォルダ内の全画像一覧（前後移動用）
    pub image_index: usize,
    pub folder_history: Vec<PathBuf>, /// フォルダ移動履歴（前後移動用）
    pub fit_to_screen: bool,     /// オプション: 自動縮小モード
    pub zoom_scale: f32,         /// 手動拡大縮小用スケール
    pub show_property: bool,     /// プロパティ表示フラグ
    
    texture: Option<TextureHandle>,
    image: Option<Box<dyn ImageContent>>,
    original_image_size: Option<egui::Vec2>,
    loading: bool,
    keep_window_reposition: bool,
    show_save_confirm: bool,          /// 保存確認ダイアログ
    // スレッド間通信用チャンネル
    tx: Sender<(PathBuf, Result<Box<dyn ImageContent>, String>)>,
    rx: Receiver<(PathBuf, Result<Box<dyn ImageContent>, String>)>,
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
            tx,
            rx,
        };

        sub_window.load_async(plugin_mgr.clone(), ctx.clone(),);

        sub_window 
    }

    /// 画像のデコード処理を別スレッドで実行する
    fn load_async(&mut self, plugin_mgr: Arc<crate::plugin::PluginManager>, ctx: egui::Context,) {
        self.loading = true;
        let path = self.current_path.clone();
        let tx = self.tx.clone();

        thread::spawn(move || {
            // バックグラウンドでプラグインデコードを実行
            let result = 
                if path 
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("gif"))
                    .unwrap_or(false)
                {
                    GifAnimation::load(&path)
                        .map(|gif| {
                            Box::new(gif) as Box<dyn ImageContent>
                        })
                        .map_err(|e| e.to_string())
                } else {
                    plugin_mgr
                        .try_decode(&path)
                        .map(|img | {
                            // WSLg保護のための安全サイズ縮小
                            let max_dim = 3840;
                            let img =
                                if img.width() > max_dim || img.height() > max_dim {
                                    img.thumbnail(max_dim, max_dim)
                                } else {
                                    img
                                };
                            Box::new(
                                StaticImage::new(img)
                            ) as Box<dyn ImageContent>
                        })    
                        .map_err(|e| e.to_string())
                };

            let _ = tx.send((path, result));
            // 読み込み完了後にGUIを再描画
            ctx.request_repaint();            
        });
    }

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

        match(ctrl, left, right) {
            (true, true, false) => self.do_rotate_left(&ctx),          // ctrl + ← 左回転
            (true, false, true) => self.do_rotate_right(&ctx),     // ctrl + → 右回転
            (false,true,false)=> self.previous_image(&ctx,plugin_mgr),   // ← 前の画像
            (false,false,true)=> self.next_image(&ctx,plugin_mgr), // → 次の画像
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
                    if let Some(delay) = image.next_frame() {
                        ctx.request_repaint_after(delay);
                    } 
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

    /// 原寸大表示時に、実際の outer_rect が
    /// Windowsの作業領域を超えていないか確認し、
    /// 超過している場合だけウィンドウサイズを縮小する。
    fn correct_window_size_to_work_area(&mut self, ctx: &egui::Context) {
        let Some(outer_rect) = 
            ctx.input(|i| i.viewport().outer_rect) 
        else {
            return;
        };
        
        let Some(work_rect) = 
            Self::get_avaivable_screen_rect(ctx)  
        else {
            return;
        };

        // 現在の outer_rect が作業領域を超えているか確認        
        let overflow_x = (outer_rect.max.x - work_rect.max.x).max(0.0);
        let overflow_y = (outer_rect.max.y - work_rect.max.y).max(0.0);

        println!(
    "correction check: outer_max={:?}, work_max={:?}, overflow=({:.1},{:.1})",
    outer_rect.max,
    work_rect.max,
    overflow_x,
    overflow_y
);
        // はみ出していなければ補正終了
        if overflow_x <= 0.0 && overflow_y <= 0.0 {
            return;
        }
    
        // 作業領域内に収まる outer サイズを計算
        let allowed_outer_width = (work_rect.max.x - outer_rect.min.x).max(100.0);
        let allowed_outer_height = (work_rect.max.y - outer_rect.min.y).max(100.0);
        let allowed_outer_size  = egui::vec2(
            allowed_outer_width,
            allowed_outer_height,
        );

        // outer と inner の差を取得
        let current_inner_size = 
            ctx.input(|i| i.viewport().inner_rect)
                .map(|r| r.size());

        let Some(current_inner_size) = 
            current_inner_size 
        else {
            return;
        };    

        let decoration = egui::vec2(
            (outer_rect.width() - current_inner_size.x).max(0.0),
            (outer_rect.height() - current_inner_size.y).max(0.0),
        );

        // InnerSize に指定するサイズ
        let corrected_inner = egui::vec2(
            (allowed_outer_size.x - decoration.x).max(100.0),
            (allowed_outer_size.y - decoration.y).max(100.0),
        );

        println!(
        "correction: outer_size={:?}, \
        allowed_outer={:?}, decoration={:?}, \
        corrected_inner={:?}",
        outer_rect.size(),
        allowed_outer_size,
        decoration,
        corrected_inner,
    );

        // 位置は変更しない。
        // サイズだけ変更する。
        ctx.send_viewport_cmd(
            egui::ViewportCommand::InnerSize(corrected_inner)
        );

    }

    // ------------------------------------------------------------
    // ← 前の画像
    // ------------------------------------------------------------
    fn previous_image(&mut self, ctx: &egui::Context, plugin_mgr: &Arc<PluginManager>) {            
        if self.image_index > 0 {
            self.image_index -= 1;
            self.current_path = self.directory_files[self.image_index].clone();

            self.texture = None; // 前の画像を破棄してメモリ解放
            self.image = None;
            self.original_image_size = None;
            self.loading = true;
            // 新しい画像を非同期で読み込む
            self.load_async(plugin_mgr.clone(), ctx.clone(),);
        }
    }

    // ------------------------------------------------------------
    // → 次の画像
    // ------------------------------------------------------------  

    fn next_image(&mut self, ctx: &egui::Context, plugin_mgr: &Arc<PluginManager>) {
        if self.image_index + 1 < self.directory_files.len() {
            self.image_index += 1;
            self.current_path = self.directory_files[self.image_index].clone();

            self.texture = None; // 前の画像を破棄してメモリ解放
            self.image = None;
            self.original_image_size = None;
            self.loading = true;
            // 新しい画像を非同期で読み込む
            self.load_async(plugin_mgr.clone(), ctx.clone(),);
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

        println!("new_dir = {:?}", new_dir);    
        // 画像一覧を取得        
        self.current_path = new_dir.clone();
        let image_files = Self::get_image_files(&new_dir, plugin_mgr);
        if image_files.is_empty() {
            return;
        }

        self.current_path = image_files[0].clone();
        self.directory_files = image_files;
        self.image_index = 0;

        self.texture = None;
        self.image = None;
        self.original_image_size = None;
        self.loading = true;
        self.load_async(plugin_mgr.clone(), ctx.clone(),);

    }


    /// 下矢印で移動するフォルダを取得
    ///
    /// 優先順位:
    /// 1. サブフォルダ
    /// 2. 次の兄弟フォルダ
    fn get_next_directory(
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

    /// サブウィンドウのサイズの設定
    /// タスクバー等を除いた使用可能領域を超えないようにする。
    fn resize_window_to_image(&self, ctx: &egui::Context) {
        let Some(original_size) = 
            self.original_image_size 
        else {
            return;
        };

        let Some(outer_rect) = 
            ctx.input(|i| i.viewport().outer_rect) 
        else {
            return;
        };

        let Some(work_rect) = 
            Self::get_avaivable_screen_rect(ctx) 
        else {
            return;
        };

        // ウインドウのサイズを取得
        //　タイトルバーや枠を含む
        let inner_size = ctx
            .input(|i| i.viewport().inner_rect)
            .map(|r| r.size())
            .unwrap_or(outer_rect.size()
        );
        let decoration = egui::vec2(
            (outer_rect.width() - inner_size.x).max(0.0),
            (outer_rect.height() - inner_size.y).max(0.0),
        );

        // 現在のサブウィンドウ位置
        let position = outer_rect.min;
        // タスクバーを除いた使用可能領域から
        // 現在位置より右・下に残っている領域を取得
        let available_outer_width = (work_rect.max.x - position.x).max(100.0);
        let available_outer_height = (work_rect.max.y - position.y).max(100.0);

        // 拡大率を適用した目標の画像サイズ
        let scaled_image_size = original_size * self.zoom_scale;

        if self.fit_to_screen {
            // 自動縮小ON: 利用可能な内部描画エリアに合わせて縮小
            let max_image_width = (available_outer_width - decoration.x - (SIDE_MARGIN * 2.0)).max(100.0);
            let max_image_height = (available_outer_height - decoration.y - TOP_MARGIN).max(100.0);
            
            let scale_x = max_image_width / scaled_image_size.x;
            let scale_y = max_image_height / scaled_image_size.y;

            // 小さい画像は拡大しない
            let scale = scale_x.min(scale_y).min(1.0);

            // サブウィンドウサイズ
            let window_size = egui::vec2(
                (scaled_image_size.x * scale) + SIDE_MARGIN * 2.0,
                (scaled_image_size.y * scale) + TOP_MARGIN,
            );
        println!(
            "fit: available={},{} window={},{}",
            available_outer_width,
            available_outer_height,
            window_size.x,
            window_size.y
        );
            // ウィンドウサイズ変更
            ctx.send_viewport_cmd(
            egui::ViewportCommand::InnerSize(window_size),  
            );
        } else {
            // 原寸表示（自動縮小OFF）:
            // 目標とするInnerサイズ（画像原寸 + UIマージン）            
            let target_inner = egui::vec2(
                scaled_image_size.x + SIDE_MARGIN * 2.0,
                scaled_image_size.y + TOP_MARGIN,
            );

            // 目標サイズに枠（decoration）を足したOuterサイズ
            let target_outer = target_inner + decoration;      
            // 現在位置から、タスクバーを除いた領域までの最大Outerサイズ                  
            let max_outer_width = (work_rect.max.x - position.x).max(100.0);
            let max_outer_height = (work_rect.max.y - position.y).max(100.0);

            // ウィンドウサイズを使用可能領域まで制限する。
            let final_outer_width = target_outer.x.min(max_outer_width);
            let final_outer_height = target_outer.y.min(max_outer_height);

            // サブウィンドウサイズ
            let final_inner = egui::vec2(
                (final_outer_width - decoration.x).max(100.0),
                (final_outer_height - decoration.y).max(100.0),
            );

    println!(
        "original: image={},{} available={},{} final={},{}",
        scaled_image_size.x,
        scaled_image_size.y,
        available_outer_width,
        available_outer_height,
        final_inner.x,
        final_inner.y
    );          
            ctx.send_viewport_cmd(
            egui::ViewportCommand::InnerSize(final_inner),  
            );
            
        } 
    }

    /// タスクバー等を除いた、現在のサブウィンドウが存在する
    /// ディスプレイの使用可能領域を取得する。
    ///
    /// Windows: GetMonitorInfoW() の rcWork を使用する。
    ///
    /// Windows以外: egui の monitor_size を使用する。
    fn get_avaivable_screen_rect(ctx: &egui::Context) -> Option<egui::Rect> {
        let (outer_rect, monitor_size, pixcels_per_point) = ctx.input(|i| {
            let viewport = i.viewport();
            (
                viewport.outer_rect,
                viewport.monitor_size,
                viewport.native_pixels_per_point.unwrap_or(1.0),
            )
        });

        let outer_rect = outer_rect?;

        #[cfg(target_os = "windows")]
        {
            let scale = pixcels_per_point;
            
            // egui上のウィンドウ位置をWindowsの物理ピクセルへ変換
            let point = POINT {
                x: (outer_rect.min.x * scale).round() as i32,
                y: (outer_rect.min.y * scale).round() as i32,
            };      
            // 現在のウィンドウ位置にあるディスプレイを取得
            let hmonitor = unsafe {
                MonitorFromPoint(
                    point,
                    MONITOR_DEFAULTTONEAREST,
                )      
            };

            if hmonitor == std::ptr::null_mut() {
                return None;
            }
            // Windowsのモニター情報を取得
            let mut monitor_info: MONITORINFO = 
                unsafe { std::mem::zeroed()};
            monitor_info.cbSize = 
                std::mem::size_of::<MONITORINFO>() as u32;

            let result = unsafe {
                GetMonitorInfoW(
                    hmonitor,
                    &mut monitor_info as *mut MONITORINFO,
                )
            };    
            if result == 0 {
                return None;
            }

            // rcWork:
            // タスクバー等を除いた使用可能領域
            let work = monitor_info.rcWork;
            Some(egui::Rect::from_min_max(
                egui::pos2(
                    work.left as f32 / scale,
                    work.top as f32 / scale,
                ),
                 egui::pos2(
                    work.right as f32 / scale,
                    work.bottom as f32 / scale,
                ),
            ))
        }

        #[cfg(not(target_os = "windows"))]
        {
            let monitor_size = monitor_size?;
            Some(egui::REct::from_min_size(
                egui::Pos2::Zero,
                monitor_size,
            ))
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
