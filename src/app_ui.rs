//app_ui

use std::path::PathBuf;
use eframe::egui;

use crate::app::MyApp;
// サムネイル表示枠
const THUMBNAIL_FRAME_SIZE: egui::Vec2 = egui::vec2(160.0, 120.0);
// 1個のサムネイルに必要な幅
const THUMBNAIL_ITEM_WIDTH: f32 = 170.0;

impl eframe::App for MyApp {
    // --- サブウィンドウの描画・管理 ---
    // メイン画面が閉じられると、アプリケーション全体が終了しすべてのサブ画面も自動消去されます
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {

        let ctx = ui.ctx().clone();
        // バックグラウンドで完成したサムネイルを受け取る
        self.thumbnail.update_results(&ctx);
        // サブウィンドウ
        self.show_sub_windows(&ctx);
        // 上部パネル
        self.show_top_panel(ui);
        // 左ペイン
        self.show_left_panel(ui);
        // 画像一覧
        self.show_image_panel(ui, &ctx);
    }
}

impl MyApp {

    fn show_sub_windows(&mut self, ctx: &egui::Context) {

        self.sub_windows.retain_mut(|sub_win| {
            let mut keep_open = true;

            // タイトルバーの文字化けを防ぐためファイル名のみ取得
            let filename = sub_win
                .current_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();    

            ctx.show_viewport_immediate(
                sub_win.id,
                egui::ViewportBuilder::default()
                    .with_title(format!("Sub Window - {}", filename))
                    .with_inner_size([800.0, 600.0]),
                |sub_ui, class| {
                    if class == egui::ViewportClass::EmbeddedWindow {
                        return;
                    }

                    sub_win.ui(sub_ui, &self.plugin_mgr);
                    if sub_ui.ctx().input(|i| i.viewport().close_requested()) {
                        keep_open = false;
                    }
                },
            );
            keep_open
        });
    }

    // --- メイン画面 GUI ---
    fn show_top_panel(&mut self, ui: &mut egui::Ui) {
        
        egui::Panel::top("top_panel").show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("フォルダを開く").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.current_dir = path;
                        self.thumbnail.generation += 1;
                        self.thumbnail.loading.clear();
                        self.refresh_files();
                    }
                }
                ui.label(format!("現在地: {}", self.current_dir.display()));
                ui.separator();
                ui.checkbox(&mut self.auto_fit_option, "新規サブ画面の自動縮小をデフォルトにする");
            });
        });
    }

    // 左右ペイン構成
    fn show_left_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("left_panel")
            .resizable(true)
            .default_size(250.0)
            .show(ui, |ui| {
                ui.heading("フォルダ");

                let current_dir = self.current_dir.clone();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.show_folder_tree(
                        ui,
                        &current_dir,
                    );
                });
            });
    }

    fn show_image_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context,) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("画像");

            let available_width = ui.available_width();
            // 横に何個並べられるか
            let columns = ((available_width / THUMBNAIL_ITEM_WIDTH).floor() as usize).max(1);

            let image_files: Vec<PathBuf> = self.get_display_image_files();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for row in image_files.chunks(columns) {
                    ui.horizontal(|ui | {
                        for path in row {
                            self.show_thumbnail(ui, path, ctx,);
                        }
                    });
                }
            });
        });
    }

    fn show_thumbnail(&mut self, ui:&mut egui::Ui, path: &PathBuf, ctx: &egui::Context) {
        let selected = self.selected_file.as_ref() == Some(path);

        ui.vertical(|ui| {
            if let Some(texture) = self.thumbnail.textures.get(path) {
                // サムネイル枠
                let image_size = texture.size_vec2();
                let scale = (THUMBNAIL_FRAME_SIZE.x / image_size.x)
                    .min(THUMBNAIL_FRAME_SIZE.y / image_size.y)
                    .min(1.0);
                let display_size = image_size * scale; 
                
                let image_response = 
                    ui.allocate_ui_with_layout(
                        THUMBNAIL_FRAME_SIZE,
                        egui::Layout::centered_and_justified(
                            egui::Direction::LeftToRight,),
                            |ui| {
                                ui.add(
                                    egui::Image::new(texture)
                                        .fit_to_exact_size(display_size)
                                        .sense(egui::Sense::click())
                                )
                            },    
                    ).inner;

                // ファイル名
                Self::show_thumbnail_filename(ui, path);

                // シングルクリック
                if image_response.clicked() {
                    self.selected_file = Some(path.clone());
                }

                // ダブルクリック
                if image_response.double_clicked() {
                    self.selected_file = Some(path.clone());
                    self.open_sub_window(
                        path.clone(),
                        &ctx,
                    );    
                }
            } else if self.thumbnail.loading.contains(path) {
                // 読み込み中
                ui.allocate_ui(
                    THUMBNAIL_FRAME_SIZE,
                    |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.spinner();
                        });
                    },
                );
            } else {
                // 読み込み失敗
                ui.allocate_ui(
                    THUMBNAIL_FRAME_SIZE,
                    |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label("読込失敗");
                        });
                    },
                );
                Self::show_thumbnail_filename(ui, path);
            }
        });
    }   

    fn get_display_image_files(&self) -> Vec<PathBuf> {
        self.files
            .iter()
            .filter(|path| {
                path.is_file() && self.plugin_mgr.can_decode(path)
            })
            .cloned()
            .collect()
    }

    fn show_thumbnail_filename(ui: &mut egui::Ui, path: &PathBuf) {
        ui.add_sized(
            [THUMBNAIL_FRAME_SIZE.x, 20.0],
            egui::Label::new(
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("")
            )
            .truncate(),
        );
    }
}

