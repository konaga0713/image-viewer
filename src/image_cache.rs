/// image_cache

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

// キャッシュ操作
pub struct ImageCache {
    images: HashMap<PathBuf, image::RgbaImage>,
    order: VecDeque<PathBuf>,
    capacity: usize,
} 

impl ImageCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            images: HashMap::new(),
            order: VecDeque::new(),
            capacity,
        }
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.images.contains_key(path)
    }

    pub fn get(&mut self, path: &Path) -> Option<&image::RgbaImage> {
        if !self.images.contains_key(path) {
            return None;
        }

        self.order.retain(|p| p != path);
        self.order.push_back(path.to_path_buf());

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
