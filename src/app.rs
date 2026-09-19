//app
use eframe::egui;
use std::path::PathBuf;
use std::sync::Arc;

use crate::app_thumbnail::ThumbnailState;
use crate::plugin::PluginManager;
use crate::sub_window::SubWindow;

pub struct MyApp {
    pub current_dir: PathBuf,
    pub files: Vec<PathBuf>,
    pub selected_file: Option<PathBuf>,
    pub sub_windows: Vec<SubWindow>,
    pub plugin_mgr: Arc<PluginManager>,
    pub auto_fit_option: bool,
    //サムネイル情報
    pub thumbnail: ThumbnailState,
}

impl MyApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_japanese_font(&cc.egui_ctx);  // 日本語フォントの設定
        cc.egui_ctx.set_visuals(eframe::egui::Visuals::light()); // ライトモードの設定
        let mut plugin_mgr = PluginManager::new();
        plugin_mgr.load_plugins(std::path::Path::new("./plugins"));

        // サムネイル情報取得
        let thumbnail = ThumbnailState::new(cc.egui_ctx.clone());

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
            thumbnail,
        };

        app.refresh_files();
        app
    }

    pub fn refresh_files(&mut self) {
        // フォルダが変わったので世代を進める
        self.thumbnail.generation += 1;

        if let Ok(entries) = std::fs::read_dir(&self.current_dir) {
            self.files = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .collect();
            self.files.sort();

            // 現在フォルダに存在しないサムネイルを削除
            self.thumbnail.textures
                .retain(|path, _| path.exists());

            // 現在フォルダに存在しない処理状態を削除
            self.thumbnail.loading
                .retain(|path| path.exists());
            self.thumbnail.failed
                .retain(|path| path.exists());

            // 現在フォルダの画像をサムネイル処理キューへ追加
            for path in &self.files {
                if !path.is_file() {
                    continue;
                }
                if !self.plugin_mgr.can_decode(path) {
                    continue;
                }

                if self.thumbnail.textures.contains_key(path) {
                    continue;
                }
                if self.thumbnail.cache.contains(path) {
                    continue;
                }
                if self.thumbnail.loading.contains(path) {
                    continue;
                }
                if self.thumbnail.failed.contains(path) {
                    continue;
                }

                let request = crate::app_thumbnail::ThumbnailRequest {
                    generation: self.thumbnail.generation,
                    path: path.clone(),
                };

                if self.thumbnail.tx.send(request).is_ok() {
                    self.thumbnail.loading.insert(path.clone());
                }
            }

        }
    }

    pub fn open_sub_window(&mut self, path: PathBuf, ctx: &egui::Context,) {
        // メイン画面で画像を選択するたびに、新しいサブ画面を作成
        let id = egui::ViewportId::from_hash_of((path.clone(), self.sub_windows.len(), std::time::Instant::now()));
        self.sub_windows.push(SubWindow::new(id, path, self.auto_fit_option, self.plugin_mgr.clone(), &ctx.clone(),));
    }

    pub fn show_folder_tree(
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