///sub_window_input
use std::sync::Arc;

use crate::plugin::PluginManager;
use crate::image_cache::ImageCache;

use super::SubWindow;

impl SubWindow {
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

    pub fn handle_keyboard(&mut self, ctx: &egui::Context, plugin_mgr: &Arc<PluginManager>, image_cache: &mut ImageCache,) {

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
            (false,true,false)=> self.previous_image(&ctx,plugin_mgr, image_cache,),   // ← 前の画像
            (false,false,true)=> self.next_image(&ctx,plugin_mgr, image_cache,), // → 次の画像
            _ => {}
        }

        match(ctrl, key_t, key_b) {
            (true, true, false) => self.move_to_image(0, &ctx, plugin_mgr, image_cache,),          // ctrl + T top画像 
            (true, false, true) => {                    
                // ctrl + B Bottom画像
                if !self.directory_files.is_empty() {
                    let last = self.directory_files.len()-1;
                    self.move_to_image(last, &ctx, plugin_mgr, image_cache,);
                }
            }     
            _ => {}
        }

        match(up,down) {
            (true,false) => self.move_to_previous_directory(&ctx, plugin_mgr, image_cache),  // ↑ 前のフォルダ
            (false,true) => self.move_to_next_directory(&ctx,plugin_mgr, image_cache),   // ↓ 次のフォルダ
            _ => {}
        }

        // ------------------------------------------------------------
        // ctrl + s 画像保存
        // ------------------------------------------------------------
        if ctrl && ctx.input(|i| i.key_pressed(egui::Key::S)) {
            println!("ctrl + s pressed");

            self.request_save();
        }
    }

}
