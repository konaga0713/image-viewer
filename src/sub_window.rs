use egui::{Color32, TextureHandle};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::sync::Arc;
use crate::plugin::PluginManager;

pub struct SubWindow {
    pub id: egui::ViewportId,
    pub current_path: PathBuf,
    pub dir_files: Vec<PathBuf>, // フォルダ内の全画像一覧（前後移動用）
    pub current_index: usize,
    pub fit_to_screen: bool,     /// オプション: 自動縮小モード
    pub zoom_scale: f32,         /// 手動拡大縮小用スケール
    texture: Option<TextureHandle>,
    loading: bool,
    // スレッド間通信用チャンネル
    tx: Sender<(PathBuf, Result<image::DynamicImage, String>)>,
    rx: Receiver<(PathBuf, Result<image::DynamicImage, String>)>,
}

impl SubWindow {
    pub fn new(id: egui::ViewportId, path: PathBuf, fit_to_screen: bool, plugin_mgr: Arc<PluginManager>) -> Self {
        let (tx, rx) = channel();

        // 同一ディレクトリ内のファイル一覧を取得（矢印キー移動用）
        let mut dir_files = Vec::new();
        if let Some(parent) = path.parent() {
            if let Ok(entries) = std::fs::read_dir(parent) {
                dir_files = entries
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.is_file() && (p == &path || plugin_mgr.can_decode(p)))
                    .collect();
                dir_files.sort();
            }
        }

        let current_index = dir_files.iter().position(|p| p == &path).unwrap_or(0);

        let mut sub_win = Self {
            id,
            current_path: path,
            dir_files,
            current_index,
            fit_to_screen,
            zoom_scale: 1.0,
            texture: None,
            loading: false,
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

        // キーボード入力による前後の画像移動
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            if self.current_index > 0 {
                self.current_index -= 1;
                self.current_path = self.dir_files[self.current_index].clone();
                self.load_image_async(plugin_mgr.clone());
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            if self.current_index + 1 < self.dir_files.len() {
                self.current_index += 1;
                self.current_path = self.dir_files[self.current_index].clone();
                self.load_image_async(plugin_mgr.clone());
            }
        }

        // UIの描画
        egui::TopBottomPanel::top("sub_top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.fit_to_screen, "画面に合わせて自動縮小");
                ui.label(format!("拡大率: {:.0}%", self.zoom_scale * 100.0));
                if ui.button("リセット").clicked() {
                    self.zoom_scale = 1.0;
                }
                ui.label(format!(" ( {} / {} )", self.current_index + 1, self.dir_files.len()));

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
                let img_size = texture.size_vec2();

                // 拡大縮小計算
                let mut display_size = img_size * self.zoom_scale;

                // 自動縮小オプション適用 (画面に入りきらない場合のみ左上基準で縮小)
                if self.fit_to_screen {
                    if display_size.x > available_size.x || display_size.y > available_size.y {
                        let scale_x = available_size.x / img_size.x;
                        let scale_y = available_size.y / img_size.y;
                        let fit_scale = scale_x.min(scale_y);
                        display_size = img_size * fit_scale;
                    }
                }

                // スクロールエリアを配置し、基準を左上に設定
                egui::ScrollArea::both()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        // 左上基準で描画
                        let response = ui.add(
                            egui::Image::new(texture)
                                .fit_to_exact_size(display_size)
                        );

                        // マウスホイールでのズームイン/アウト
                        if response.hovered() {
                            let scroll_delta = ctx.input(|i| i.raw_scroll_delta.y);
                            if scroll_delta != 0.0 {
                                self.zoom_scale *= if scroll_delta > 0.0 { 1.1 } else { 0.9 };
                                self.zoom_scale = self.zoom_scale.clamp(0.1, 10.0);
                            }
                        }
                    });
            } else {
                ui.colored_label(Color32::RED, "画像の読み込みに失敗しました");
            }
        });
    }
}