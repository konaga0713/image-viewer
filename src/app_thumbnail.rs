//app_thumbnail

use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

// サムネイル表示枠
pub const FRAME_SIZE: egui::Vec2 = egui::vec2(160.0, 120.0);
// サムネイルの作成結果
pub struct ThumbnailResult {
    pub path: PathBuf,
    pub image: Result<image::RgbaImage, String>,
} 

pub struct ThumbnailState {
    pub textures: HashMap<PathBuf, egui::TextureHandle>,
    pub tx: Sender<PathBuf>,
    pub rx: Receiver<ThumbnailResult>,
    pub loading: HashSet<PathBuf>,
    pub failed:  HashSet<PathBuf>,
}

impl ThumbnailState {
    pub fn new(ctx: egui::Context) -> Self {
        // サムネイル用チャンネル
        let (tx, request_rx) =
            mpsc::channel::<PathBuf>();
        let (result_tx, rx) =
            mpsc::channel::<ThumbnailResult>();

        // サムネイルワーカースレッド
        std::thread::spawn(move || {
            while let Ok(path) = request_rx.recv() {
                let result = image::open(&path)
                    .map(|image| {
                        image.thumbnail(160, 120).to_rgba8() 
                    })
                    .map_err(|e| e.to_string());
                let _ = result_tx.send(
                    ThumbnailResult {
                        path,
                        image: result,
                    }
                );
                // 1枚完成するたびにGUIを再描画
                ctx.request_repaint();              
            }
        });

        Self {
            textures: HashMap::new(),
            tx,
            rx,
            loading: HashSet::new(),
            failed: HashSet::new(),
        }
    }

    pub fn update_results(&mut self, ctx: &egui::Context) {
        while let Ok(result) = self.rx.try_recv() {
            self.loading.remove(&result.path);

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
                        self.textures
                            .insert(result.path, texture);        

                }
                Err(e) => {
                    eprintln!("サムネイル読み込み失敗: {}: {}", result.path.display(),e);
                    self.failed.insert(result.path);
                }
            }
        }
    }
}