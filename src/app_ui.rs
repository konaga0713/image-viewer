//app_ui

use std::path::PathBuf;
use eframe::egui;

use crate::app::MyApp;
use crate::image_property::ImageProperty;

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
        // プロパティ表示
        self.show_property(&ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.config.save();
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

            let viewport_builder = egui::ViewportBuilder::default()
                .with_title(format!("Sub Window - {}", filename));
            
            ctx.show_viewport_immediate(
                sub_win.id,
                viewport_builder,
                |sub_ui, class| {
                    if class == egui::ViewportClass::EmbeddedWindow {
                        return;
                    }

                    sub_win.ui(sub_ui, &self.plugin_mgr, &mut self.image_cache);
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
                        self.thumbnail.generation += 1;
                        self.thumbnail.loading.clear();
                        self.change_current_dir(path);
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
            .default_size(self.config.tree_width)
            .show(ui, |ui| {
                ui.heading("フォルダ");

                let current_dir = self.current_dir.clone();
                let tree_root = Self::get_tree_root(&current_dir);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    if let Some(path) = self.folder_tree.ui(ui) {
                        self.change_current_dir(path);
                    }
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
            let scroll_to_top = self.thumbnail.scroll_to_top;

            egui::ScrollArea::vertical().show(ui, |ui| {
                if scroll_to_top {
                    ui.scroll_to_cursor(Some(egui::Align::TOP));
                }
                for row in image_files.chunks(columns) {
                    ui.horizontal(|ui | {
                        for path in row {
                            self.show_thumbnail(ui, path, ctx,);
                        }
                    });
                }
            });

            if scroll_to_top {
                self.thumbnail.scroll_to_top = false;
            }
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

    // println!(
    //     "[DOUBLE_CLICK] path={:?}, rect={:?}",
    //     path,
    //     image_response.rect
    // );
    
                    self.open_image(
                        &path,
                        &ctx,
                    );    
                }

                // ====================================================
                // 右クリックメニュー
                // ====================================================
                image_response.context_menu(|ui| {
                    if ui.button("開く").clicked() {
                        self.open_image(&path, ctx);
                        ui.close();
                    }
                    if ui.button("プログラムから開く").clicked() {
                        println!(
                            "[OPEN_WITH] path={:?}",
                            path
                        );

                        if let Err(err) = crate::windows_shell::open_with(path) {
                            eprintln!("[OPEN_WITH] error: {}", err);
                        }
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("プロパティ").clicked() {
                        println!(
                            "[OPEN_WITH] path={:?}",
                            path
                        );

                        self.property_path = Some(path.clone());
                        self.show_property = true;
                        ui.close();
                    }
                });
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

    fn open_image(&mut self, path: &PathBuf, ctx: &egui::Context,) {
        self.selected_file = Some(path.clone());
        self.open_sub_window(path.to_path_buf(), ctx);
    }

        fn show_property(&mut self, ctx: &egui::Context,) {
        if self.show_property {
            if let Some(path) = &self.property_path {
                ImageProperty::show(
                    ctx,
                    path,
                    &mut self.show_property,
                );
            }
        }
    }

    fn get_tree_root(path: &PathBuf) -> PathBuf {
        let mut components = path.components();
        let Some(prefix) = components.next() else {
            return path.clone();
        };
        let Some(root) = components.next() else {
            return path.clone();
        };

        PathBuf::from(prefix.as_os_str()).join(root.as_os_str())
    }

    fn change_current_dir(&mut self, path: PathBuf) {
        if self.current_dir == path {
            return;
        }
        self.current_dir = path.clone();
        self.config.last_folder = Some(path.clone());

        let tree_root = Self::get_tree_root(&path);

        self.folder_tree.set_current_path(tree_root, path);

        self.selected_file = None;
        self.refresh_files();
    }
}

