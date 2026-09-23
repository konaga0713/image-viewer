///sub_window_ui
use std::sync::Arc;

use crate::plugin::PluginManager;
use crate::image_property::ImageProperty;
use crate::image_cache::ImageCache;

use super::SubWindow;

impl SubWindow {

    pub fn ui(
        &mut self, 
        ui: &mut egui::Ui,
        plugin_mgr: &Arc<crate::plugin::PluginManager>,
        image_cache: &mut ImageCache,     
    )  {
        let ctx = ui.ctx().clone();

        // 画像読込結果
        self.process_image_loading(&ctx, image_cache);

        if self.resize_pending {
println!("RESIZE");            
            self.resize_pending = false;
            self.resize_window_to_image(&ctx);
        }

        // キーボード操作
        self.handle_keyboard(&ctx, plugin_mgr, image_cache);

        // 上部メニュー
        self.show_top_panel(ui, &ctx, plugin_mgr, image_cache);

        // 画像表示
        self.show_image_area(ui, &ctx);

        // プロパティ
        self.show_property(&ctx);

        // 保存確認
        self.show_save_confirm(&ctx);
        
    }

    // ------------------------------------------------------------
    // TOP画面
    // ------------------------------------------------------------
    fn show_top_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, plugin_mgr: &Arc<PluginManager>, image_cache: &mut ImageCache) {
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
            self.show_image_menu(ui, ctx, plugin_mgr, image_cache);
        });
    }

    // ------------------------------------------------------------
    // メニュー画面
    // ------------------------------------------------------------
    fn show_image_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, plugin_mgr: &Arc<PluginManager>, image_cache: &mut ImageCache) {
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
                    self.move_to_image(0, &ctx, plugin_mgr, image_cache,);
                    ui.close();
                }
                if ui.button("Bottom画像 ctrl + b").clicked() {
                    if !self.directory_files.is_empty() {
                        let last = self.directory_files.len()-1;
                        self.move_to_image(last, &ctx, plugin_mgr, image_cache,);
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
        });
    }

    // ------------------------------------------------------------
    // 画像表示エリア（原寸・自動縮小・左上基準）
    // ------------------------------------------------------------
    fn show_image_area(&mut self, ui: &mut egui::Ui, ctx: &egui::Context,) {
/*
println!(
    "[SUBWINDOW] id={:?}, path={:?}, ptr={:p}, available={:?}",
    self.id,
    self.current_path,
    self,
    ui.available_size()
);
 */
        egui::CentralPanel::default().show(ui, |ui| {
            self.show_texture(ui);
            self.update_animation(ctx);
        });
    }

    // ------------------------------------------------------------
    // プロパティ表示
    // ------------------------------------------------------------
    fn show_property(&mut self, ctx: &egui::Context){
        if self.show_property {
            ImageProperty::show(
                &ctx,
                &self.current_path,
                &mut self.show_property,
            );
        }
    }

}