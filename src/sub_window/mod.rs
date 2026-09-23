///sub_window
pub mod sub_window_ui;
mod sub_window_loading;
mod sub_window_input;
mod sub_window_save;
mod sub_window_image;
mod sub_window_navigation;
mod sub_window_folder;
mod sub_window_window;
mod sub_window_animation;

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use egui::TextureHandle;

use crate::image_cache::ImageCache;
use crate::image_content::ImageContent;
use crate::plugin::PluginManager;

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
    pub resize_pending: bool,
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
        image_cache: &mut ImageCache,
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
            resize_pending: false,
            tx,
            rx,
        };

        sub_window.load_async(plugin_mgr.clone(), image_cache, ctx.clone(),);

        sub_window 
    }
}
