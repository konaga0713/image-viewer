//app_thumbnail

use eframe::egui;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::sync::mpsc::{self, Receiver, Sender};

use crate::jpeg_loader;
use crate::image_cache::ImageCache;

const THUMBNAIL_WIDTH: u32 = 160;
const THUMBNAIL_HEIGHT: u32 = 120;
const MAX_RESULTS_PER_FRAME: usize = 32;

// サムネイルの要求世代管理
pub struct ThumbnailRequest {
    pub generation: u64,
    pub path: PathBuf,
}

// キャッシュ操作
const DEFAULT_CAPACITY: usize = 1000;

// サムネイルの作成結果
pub struct ThumbnailResult {
    pub generation: u64,
    pub path: PathBuf,
    pub image: Result<image::RgbaImage, String>,
} 

pub struct ThumbnailState {
    pub textures: HashMap<PathBuf, egui::TextureHandle>,
    pub tx: Sender<ThumbnailRequest>,
    pub rx: Receiver<ThumbnailResult>,
    pub loading: HashSet<PathBuf>,
    pub failed:  HashSet<PathBuf>,
    pub generation: u64,
    pub current_generation: Arc<AtomicU64>,
    pub cache: ImageCache,
    pub scroll_to_top: bool,
}

impl ThumbnailState {
    pub fn new(ctx: egui::Context) -> Self {
        // サムネイル用チャンネル
        let (tx, request_rx) =
            mpsc::channel::<ThumbnailRequest>();
        let (result_tx, rx) =
            mpsc::channel::<ThumbnailResult>();

        let current_generation = Arc::new(AtomicU64::new(0));
        let worker_generation = Arc::clone(&current_generation);

            // サムネイルワーカースレッド
        std::thread::spawn(move || {
            while let Ok(request) = request_rx.recv() {
                // すでに古い要求ならデコードしない
                if request.generation != worker_generation.load(Ordering::Relaxed){
                    continue;
                }

                let result = 
                    if jpeg_loader::is_jpeg(&request.path) {
                        jpeg_loader::load(&request.path)
                            .map(|image| {
                                create_thumbnail(&image,)
                            })
                    } else {
                        image::open(&request.path)
                            .map(|image| {
                                let thumbnail = image.to_rgba8(); 
                                create_thumbnail(&thumbnail,)})
                            .map_err(|e| e.to_string())
                    };

                // デコード中にフォルダが変更された
                if request.generation != worker_generation.load(Ordering::Relaxed){
                    continue;
                }

                let _ = result_tx.send(
                    ThumbnailResult {
                        generation: request.generation,
                        path: request.path,
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
            generation: 0,
            current_generation,
            cache: ImageCache::new(DEFAULT_CAPACITY),
            scroll_to_top: true,
        }
    }

    pub fn update_results(&mut self, ctx: &egui::Context) {

        for _ in 0..MAX_RESULTS_PER_FRAME {
            let Ok(result) = self.rx.try_recv() else {
                break;
            };
            // 古い世代の結果は破棄
            if result.generation != self.generation {
                continue;
            }

            self.loading.remove(&result.path);

            match result.image {
                Ok(rgba)=> {
                    self.cache.insert(
                        result.path.clone(),
                        rgba.clone(),
                    );

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


fn create_thumbnail(image: &image::RgbaImage) -> image::RgbaImage {
    let width = image.width();
    let height = image.height();

    let scale = (THUMBNAIL_WIDTH as f32 / width as f32)
        .min(THUMBNAIL_HEIGHT as f32 / height as f32)
        .min(1.0);

    let new_width = (width as f32 * scale).round() as u32;
    let new_height = (height as f32 * scale).round() as u32;

    image::imageops::resize(
        image, 
        new_width,
        new_height,
        image::imageops::FilterType::Triangle,
    )
}


