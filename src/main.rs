mod plugin;
mod sub_window;

use eframe::egui;
use plugin::PluginManager;
use std::path::PathBuf;
use sub_window::SubWindow;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 600.0])
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
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_japanese_font(&cc.egui_ctx);  // 日本語フォントの設定
        cc.egui_ctx.set_visuals(eframe::egui::Visuals::light()); // ライトモードの設定
        let mut plugin_mgr = PluginManager::new();
        plugin_mgr.load_plugins(std::path::Path::new("./plugins"));

        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut app = Self {
            current_dir,
            files: Vec::new(),
            selected_file: None,
            sub_windows: Vec::new(),
            plugin_mgr: Arc::new(plugin_mgr),
            auto_fit_option: true,
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
        }
    }

    fn open_sub_window(&mut self, path: PathBuf) {
        // メイン画面で画像を選択するたびに、新しいサブ画面を作成
        let id = egui::ViewportId::from_hash_of((path.clone(), self.sub_windows.len(), std::time::Instant::now()));
        self.sub_windows.push(SubWindow::new(id, path, self.auto_fit_option, self.plugin_mgr.clone()));
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- サブウィンドウの描画・管理 ---
        // メイン画面が閉じられると、アプリケーション全体が終了しすべてのサブ画面も自動消去されます
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
                |sub_ctx, class| {
                    if class == egui::ViewportClass::Embedded {
                        return;
                    }
                    sub_win.ui(sub_ctx, &self.plugin_mgr);
                    if sub_ctx.input(|i| i.viewport().close_requested()) {
                        keep_open = false;
                    }
                },
            );
            keep_open
        });

        // --- メイン画面 GUI ---
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
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
        egui::SidePanel::left("left_pane")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("フォルダツリー / ファイル一覧");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for path in &self.files {
                        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                        if path.is_dir() {
                            if ui.selectable_label(false, format!("📁 {}", file_name)).clicked() {
                                self.current_dir = path.clone();
                                self.refresh_files();
                                break;
                            }
                        } else {
                            let is_selected = self.selected_file.as_ref() == Some(path);
                            if ui.selectable_label(is_selected, format!("🖼 {}", file_name)).clicked() {
                                self.selected_file = Some(path.clone());
                            }
                        }
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("プレビュー・アクション（右ペイン）");
            if let Some(selected_path) = &self.selected_file {
                ui.label(format!("選択中: {}", selected_path.display()));
                
                // 右ペインからボタン（またはダブルクリック）でサブ画面起動
                if ui.button("新規サブ画面で表示").clicked() {
                    self.open_sub_window(selected_path.clone());
                }
            } else {
                ui.label("左ペインから画像ファイルを選択してください。");
            }
        });
    }
}

/// OSごとの日本語フォントを自動検索してロードする関数
fn setup_japanese_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // .ttc ではなく .ttf ファイルを優先的に指定
    let font_paths = [
// --- WSL環境用（Windows側のフォントを参照） ---
// 軽量な単一 .ttf フォントを最優先にする
    "/mnt/c/Windows/Fonts/yugothr.ttf",       // 游ゴシック (約4MB)
    "/mnt/c/Windows/Fonts/yugothb.ttf",       // 游ゴシック Bold
    "/usr/share/fonts/truetype/fonts-japanese-gothic.ttf", // WSL標準
    
    // .ttc は末尾に配置
    "/mnt/c/Windows/Fonts/meiryo.ttc",    
    
    // OS標準の日本語フォントパスの候補
    "C:\\Windows\\Fonts\\msyh.ttc",       // Windows (メイリオ / YaHei)
    "C:\\Windows\\Fonts\\yuantic.ttf",    // Windows (游ゴシック)
    "C:\\Windows\\Fonts\\msgothic.ttc",   // Windows (ＭＳ ゴシック)
    "/System/Library/Fonts/Hiragino Sans GB.ttc", // macOS
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc", // Linux
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
            egui::FontData::from_owned(data),
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