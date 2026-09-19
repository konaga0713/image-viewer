mod gif_animation;
mod image_content;
mod image_operation;
mod image_property;
mod plugin;
mod save_image;
mod static_image;
mod sub_window;
mod webp_animation;
mod webp_decoder;
mod sub_window_folder;
mod sub_window_window;

use eframe::egui;
use eframe::wgpu::BindingResource::ExternalTexture;
use plugin::PluginManager;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use crate::sub_window::SubWindow;

// サムネイル表示枠
const THUMBNAIL_FRAME_SIZE: egui::Vec2 = egui::vec2(160.0, 120.0);
// サムネイルの作成結果
struct ThumbnailResult {
    path: PathBuf,
    image: Result<image::RgbaImage, String>,
} 

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 600.0])
            .with_min_inner_size([100.0,100.0])
            .with_title("Image Explorer App"),
        ..Default::default()
    };

    eframe::run_native(
        "Image Explorer App",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
}

struct MyApp {
    current_dir: PathBuf,
    files: Vec<PathBuf>,
    selected_file: Option<PathBuf>,
    sub_windows: Vec<SubWindow>,
    plugin_mgr: Arc<PluginManager>,
    auto_fit_option: bool,
    //サムネイルのキャッシュ,処理要求チャンネル,受取チャンネル,デコード中,デコード失敗
    thumbnail_textures: HashMap<PathBuf, egui::TextureHandle>,
    thumbnail_tx: Sender<PathBuf>,
    thumbnail_rx: Receiver<ThumbnailResult>,
    thumbnail_loading: HashSet<PathBuf>,
    thumbnail_failed:  HashSet<PathBuf>,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_japanese_font(&cc.egui_ctx);  // 日本語フォントの設定
        cc.egui_ctx.set_visuals(eframe::egui::Visuals::light()); // ライトモードの設定
        let mut plugin_mgr = PluginManager::new();
        plugin_mgr.load_plugins(std::path::Path::new("./plugins"));

        // ---------------------------------
        // サムネイル用チャンネル
        // ---------------------------------
        let (thumbnail_tx, thumbnail_request_rx) =
            mpsc::channel::<PathBuf>();
        let (thumbnail_result_tx, thumbnail_rx) =
            mpsc::channel::<ThumbnailResult>();
        let thumbnail_ctx = cc.egui_ctx.clone();

        // ---------------------------------
        // サムネイルワーカースレッド
        // ---------------------------------
        std::thread::spawn(move || {
            while let Ok(path) = thumbnail_request_rx.recv() {
                let result = image::open(&path)
                    .map(|image| {
                        image.thumbnail(160, 120).to_rgba8() 
                    })
                    .map_err(|e| e.to_string());
                let _ = thumbnail_result_tx.send(
                    ThumbnailResult {
                        path,
                        image: result,
                    }
                );
                // 1枚完成するたびにGUIを再描画
                thumbnail_ctx.request_repaint();              
            }
        });

        // ---------------------------------
        // メイン画面の描画
        // ---------------------------------
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut app = Self {
            current_dir,
            files: Vec::new(),
            selected_file: None,
            sub_windows: Vec::new(),
            plugin_mgr: Arc::new(plugin_mgr),
            auto_fit_option: true,
            thumbnail_textures: HashMap::new(),
            thumbnail_tx,
            thumbnail_rx,
            thumbnail_loading: HashSet::new(),
            thumbnail_failed: HashSet::new(),
        };

        app.refresh_files();
        app
    }

    fn refresh_files(&mut self) {
        if let Ok(entries) = std::fs::read_dir(&self.current_dir) {
            self.files = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .collect();
            self.files.sort();

            // 現在フォルダに存在しないサムネイルを削除
            self.thumbnail_textures
                .retain(|path, _| path.exists());

            // 現在フォルダに存在しない処理状態を削除
            self.thumbnail_loading
                .retain(|path| path.exists());
            self.thumbnail_failed
                .retain(|path| path.exists());

            // 現在フォルダの画像をサムネイル処理キューへ追加
            for path in &self.files {
                if !path.is_file() {
                    continue;
                }
                if !self.plugin_mgr.can_decode(path) {
                    continue;
                }

                if self.thumbnail_textures.contains_key(path) {
                    continue;
                }
                if self.thumbnail_loading.contains(path) {
                    continue;
                }
                if self.thumbnail_failed.contains(path) {
                    continue;
                }

                if self.thumbnail_tx.send(path.clone()).is_ok() {
                    self.thumbnail_loading.insert(path.clone());
                }
            }

        }
    }

    fn update_thumbnail_results(&mut self, ctx: &egui::Context) {
        while let Ok(result) = self.thumbnail_rx.try_recv() {
            self.thumbnail_loading.remove(&result.path);

            match result.image {
                Ok(rgba)=> {
                        let size = [
                            rgba.width() as usize,
                            rgba.height() as usize,
                        ];

                        let color_image = 
                            egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                        let texture = ctx.load_texture(
                            format!("thumbnail: {}", result.path.display()),
                            color_image,
                            egui::TextureOptions::LINEAR);
                        self.thumbnail_textures
                            .insert(result.path, texture);        

                }
                Err(e) => {
                    eprintln!("サムネイル読み込み失敗: {}: {}", result.path.display(),e);
                    self.thumbnail_failed.insert(result.path);
                }
            }
        }
    }

    fn open_sub_window(&mut self, path: PathBuf, ctx: &egui::Context,) {
        // メイン画面で画像を選択するたびに、新しいサブ画面を作成
        let id = egui::ViewportId::from_hash_of((path.clone(), self.sub_windows.len(), std::time::Instant::now()));
        self.sub_windows.push(SubWindow::new(id, path, self.auto_fit_option, self.plugin_mgr.clone(), &ctx.clone(),));
    }

    fn show_folder_tree(
        &mut self,
        ui: &mut egui::Ui,
        path: &PathBuf,
    ) {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        let mut dirs: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();

        dirs.sort();

        for dir in dirs {
            let name = dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let is_current = self.current_dir == dir;

            egui::CollapsingHeader::new(
                format!("📁 {}",name)
                )
                .id_salt(&dir)
                .show(ui, |ui| {
                    if ui
                        .selectable_label(
                            is_current,
                            "このフォルダを表示",
                        )
                        .clicked()
                    {
                        self.current_dir = dir.clone();
                        self.selected_file = None;
                        self.refresh_files();
                    }
                    self.show_folder_tree(
                        ui,
                        &dir,
                    );    
                });

        };

    }
}


impl eframe::App for MyApp {
    // --- サブウィンドウの描画・管理 ---
    // メイン画面が閉じられると、アプリケーション全体が終了しすべてのサブ画面も自動消去されます
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // バックグラウンドで完成したサムネイルを受け取る
        self.update_thumbnail_results(&ctx);

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

        // --- メイン画面 GUI ---
        egui::Panel::top("top_panel").show(ui, |ui| {
            let _ctx = ui.ctx();
            ui.horizontal(|ui| {
                if ui.button("フォルダを開く").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.current_dir = path;
                        self.refresh_files();
                    }
                }
                ui.label(format!("現在地: {}", self.current_dir.display()));
                ui.separator();
                ui.checkbox(&mut self.auto_fit_option, "新規サブ画面の自動縮小をデフォルトにする");
            });
        });

        // 左右ペイン構成
        egui::Panel::left("left_pane")
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

        egui::CentralPanel::default().show(ui, |ui| {
             ui.heading("画像");

            let available_width = ui.available_width();
            // 1個のサムネイルに必要な幅
            let item_width = 170.0; 

            // 横に何個並べられるか
            let columns = ((available_width / item_width).floor() as usize).max(1);

            egui::ScrollArea::vertical().show(ui, |ui| {
                let image_files: Vec<PathBuf> = self
                    .files
                    .iter()
                    .filter(|path| {
                        path.is_file() && self.plugin_mgr.can_decode(path)
                    })
                    .cloned()
                    .collect();

                for row in image_files.chunks(columns) {
                    ui.horizontal(|ui | {
                        for path in row {
                            let selected = 
                                self.selected_file.as_ref() == Some(path);
                            ui.vertical(|ui| {
                                if let Some(texture) = 
                                    self.thumbnail_textures.get(path) {
                                    let response = ui.vertical(|ui|{
                                        // サムネイル枠
                                        let image_response = ui.allocate_ui_with_layout(
                                            THUMBNAIL_FRAME_SIZE,
                                            egui::Layout::centered_and_justified(
                                                egui::Direction::LeftToRight,),
                                            |ui| {
                                                ui.add(
                                                    egui::Image::new(texture)
                                                        .fit_to_fraction(egui::vec2(1.0,1.0))
                                                        .sense(egui::Sense::click())
                                                )
                                            },    
                                        ).inner;

                                        // ファイル名
                                        ui.add_sized(
                                            [160.0, 20.0],
                                            egui::Label::new(
                                                path.file_name()
                                                    .and_then(|name| name.to_str())
                                                    .unwrap_or("")
                                            )
                                            .truncate(),
                                        );

                                        image_response
                                    }).inner;
                                    
                                    // シングルクリック
                                    if response.clicked() {
                                        self.selected_file = Some(path.clone());
                                    }

                                    // ダブルクリック
                                    if response.double_clicked() {
                                        self.selected_file = Some(path.clone());
                                        self.open_sub_window(
                                            path.clone(),
                                            &ctx,
                                        );    
                                    }
                                } else if self.thumbnail_loading.contains(path) {
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

                                }
                            });
                        }   
                    });
                }
            });
        });
    }

    
}

/// OSごとの日本語フォントを自動検索してロードする関数
fn setup_japanese_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // .ttc ではなく .ttf ファイルを優先的に指定
    let font_paths = [

    // OS標準の日本語フォントパスの候補
    "C:\\Windows\\Fonts\\msyh.ttc",       // Windows (メイリオ / YaHei)
    "C:\\Windows\\Fonts\\yuantic.ttf",    // Windows (游ゴシック)
    "C:\\Windows\\Fonts\\msgothic.ttc",   // Windows (ＭＳ ゴシック)
    "/System/Library/Fonts/Hiragino Sans GB.ttc", // macOS
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc", // Linux

    // --- WSL環境用（Windows側のフォントを参照） ---
// 軽量な単一 .ttf フォントを最優先にする
    "/mnt/c/Windows/Fonts/yugothr.ttf",       // 游ゴシック (約4MB)
    "/mnt/c/Windows/Fonts/yugothb.ttf",       // 游ゴシック Bold
    "/usr/share/fonts/truetype/fonts-japanese-gothic.ttf", // WSL標準
    
    // .ttc は末尾に配置
    "/mnt/c/Windows/Fonts/meiryo.ttc",    
    
    ];

    let mut font_data = None;
    for path in font_paths {
        if let Ok(bytes) = std::fs::read(path) {
            font_data = Some(bytes);
            // println!("フォント読み込み成功: {}", path); // デバッグログ確認用
            break;
        }
    }

    if let Some(data) = font_data {
        fonts.font_data.insert(
            "jp_font".to_owned(),
            egui::FontData::from_owned(data).into(),
        );

        // デフォルトフォント群の先頭に優先設定
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "jp_font".to_owned());

        fonts
            .families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap()
            .push("jp_font".to_owned());

        ctx.set_fonts(fonts);
    } else {
        eprintln!("エラー: 日本語フォントファイルが見つかりませんでした。");
    }
}