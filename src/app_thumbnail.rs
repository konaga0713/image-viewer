//app_thumbnail

use eframe::egui;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use crate::jpeg_loader;

const THUMBNAIL_WIDTH: u32 = 160;
const THUMBNAIL_HEIGHT: u32 = 120;

// サムネイルの要求世代管理
pub struct ThumbnailRequest {
    pub generation: u64,
    pub path: PathBuf,
}

// キャッシュ操作
pub struct ThumbnailCache {
    images: HashMap<PathBuf, image::RgbaImage>,
    order: VecDeque<PathBuf>,
    capacity: usize,
} 
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
    pub cache: ThumbnailCache,
}

impl ThumbnailState {
    pub fn new(ctx: egui::Context) -> Self {
        // サムネイル用チャンネル
        let (tx, request_rx) =
            mpsc::channel::<ThumbnailRequest>();
        let (result_tx, rx) =
            mpsc::channel::<ThumbnailResult>();

        // サムネイルワーカースレッド
        std::thread::spawn(move || {
            while let Ok(request) = request_rx.recv() {
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
            cache: ThumbnailCache::new(DEFAULT_CAPACITY),
        }
    }

    pub fn update_results(&mut self, ctx: &egui::Context) {
        while let Ok(result) = self.rx.try_recv() {
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

impl ThumbnailCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            images: HashMap::new(),
            order: VecDeque::new(),
            capacity,
        }
    }

    pub fn contains(&self, path: &PathBuf) -> bool {
        self.images.contains_key(path)
    }

    pub fn get(&mut self, path: &PathBuf) -> Option<&image::RgbaImage> {
        if !self.images.contains_key(path) {
            return None;
        }

        // 最近使用したものとして末尾へ移動
        self.order.retain(|p| p != path);
        self.order.push_back(path.clone());
        self.images.get(path)
    }

    pub fn insert(&mut self, path: PathBuf, image: image::RgbaImage) {
        // 既存ならアクセス順だけ更新
        if self.images.contains_key(&path) {
            self.order.retain(|p| p != &path);
        }

        self.images.insert(path.clone(), image);
        self.order.push_back(path);

        // 最大枚数を超えたら最古を削除
        while self.order.len() > self.capacity {
            if let Some(old_path) = self.order.pop_front(){
                self.images.remove(&old_path);
            }
        }
    } 

    pub fn remove(&mut self, path: &PathBuf){
        self.images.remove(path);
        self.order.retain(|p| p != path);
    } 

    pub fn retain_existing_files(&mut self){
        self.order.retain(|path| {
            if path.exists() {
                true
            } else {
                self.images.remove(path);
                false
            }     
        });
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


