///sub_window_navigation
use std::sync::Arc;

use crate::plugin::PluginManager;
use crate::image_cache::ImageCache;

use super::SubWindow;

impl SubWindow {
    // ------------------------------------------------------------
    // 画像の移動
    // ------------------------------------------------------------
    pub fn move_to_image(&mut self, index: usize, ctx: &egui::Context, plugin_mgr: &Arc<PluginManager>, image_cache: &mut ImageCache) {
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
            self.load_async(plugin_mgr.clone(), image_cache, ctx.clone(),);
    }

    // ------------------------------------------------------------
    // ← 前の画像
    // ------------------------------------------------------------
    pub fn previous_image(&mut self, ctx: &egui::Context, plugin_mgr: &Arc<PluginManager>, image_cache: &mut ImageCache,) {            
        if self.image_index > 0 {
            self.move_to_image(
                self.image_index - 1,
                ctx,
                plugin_mgr,  
                image_cache,
            );
        }
    }

    // ------------------------------------------------------------
    // → 次の画像
    // ------------------------------------------------------------  
    pub fn next_image(&mut self, ctx: &egui::Context, plugin_mgr: &Arc<PluginManager>, image_cache: &mut ImageCache) {
        if self.image_index + 1 < self.directory_files.len() {
            self.move_to_image(
                self.image_index + 1,
                ctx,
                plugin_mgr,
                image_cache,    
            );
        }
    }
}
