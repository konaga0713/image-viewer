///plugin

use libloading::{Library, Symbol};
use std::path::Path;

use crate::image_content::ImageContent;
use crate::jpeg_loader::{self, LoadMode};
use crate::static_image::StaticImage;
use crate::webp_animation::WebpAnimation;

// プラグインが実装すべきトレイト定義
pub trait ImageDecoderPlugin: Send + Sync {
    fn supported_extensions(&self) -> Vec<&'static str>;
    fn decode(&self, path: &Path) -> Result<Box<dyn ImageContent>, String>;
}

// プラグイン関数型 (動的ライブラリ側で `#[no_mangle]` して公開する関数)
type CreatePluginFn = unsafe fn() -> Box<dyn ImageDecoderPlugin>;

pub struct PluginManager {
    plugins: Vec<Box<dyn ImageDecoderPlugin>>,
    _libs: Vec<Library>, // メモリ上から解放されないよう保持
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            _libs: Vec::new(),
        }
    }

    /// plugins ディレクトリから動的ライブラリ (.dll / .so) を動的にロード
    pub fn load_plugins(&mut self, plugin_dir: &Path) {
        eprintln!("[plugin] searching directory: {}", plugin_dir.display());
        if !plugin_dir.exists() {
            return;
        }

        let entries = match std::fs::read_dir(plugin_dir) {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!("[plugin] failed to read directory: {}", e);
                return;
            }
        };

        for entry in entries{
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    eprintln!("[plugin] failed to read entry: {}", e);
                    continue;
                }
            };
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            eprintln!("[plugin] trying library: {}", path.display());
            unsafe {
                let lib = match Library::new(&path) {
                    Ok(lib) => lib,
                    Err(e) => {
                        eprintln!("[plugin] failed to load library {}: {}", path.display(), e);
                        continue;
                    }
                };
                let func: Symbol<CreatePluginFn> = match lib.get(b"create_plugin") {
                    Ok(func) => func,
                    Err(e) => {
                        eprintln!("[plugin] failed to find symbol in {}: {}", path.display(), e);
                        continue;
                    }
                };

                let plugin = func();
                eprintln!("[plugin] loaded plugin: {} (supports: {:?})", path.display(), plugin.supported_extensions());
                self.plugins.push(plugin);
                self._libs.push(lib); // ライブラリを保持して解放されない   
            }
            eprintln!("[plugin] {} plugin(s) loaded", self.plugins.len());
        }
        
    }    

    /// 拡張子に応じたデコードを試行
    pub fn can_decode(&self, path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        self.plugins.iter().any(|plugin| {
            plugin.supported_extensions().contains(&ext.as_str())
        }) || matches!(
            ext.as_str(),
            "avif" | "bmp" | "dds" | "exr" | "farbfeld" | "gif" | "hdr"
                | "ico" | "jpeg" | "jpg" | "png" | "pnm" | "qoi" | "tga"
                | "tif" | "tiff" | "webp"
        )
    }

    /// 画像を ImageContent としてデコード
    pub fn try_decode(&self, path: &Path) -> Result<Box<dyn ImageContent>, String> {
        // 1. jpegは jpeg_loader でデコード
        if jpeg_loader::is_jpeg(&path) {
            return jpeg_loader::load(&path, LoadMode::Full)
                .map(|image| {
                    Box::new(StaticImage::new(image::DynamicImage::ImageRgba8(image))) 
                        as Box<dyn ImageContent>})
                .map_err(|e| e.to_string());
        }

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        // 2. WebPは image-webp でデコード
        if ext == "webp" {
            return WebpAnimation::load(path)
                .map(|image| Box::new(image) as Box<dyn ImageContent>)
                .map_err(|e| e.to_string());
        }

        // 3. gif
        // 処理方式変更により不要
        /*
        if path.extension() == Some(OsStr::new("gif")) {
           return GifAnimation::load(path)
            .map(|image| Box::new(image) as Box<dyn ImageContent>)
            .map_err(|e| e.to_string());
        }
        */
        
        // 4. プラグインから検索
        for plugin in &self.plugins {
            if plugin.supported_extensions().contains(&ext.as_str()) {
                return plugin.decode(path).map_err(|err| {
                    eprintln!("[plugin] failed to decode {}: {}", path.display(), err);
                    err
                }).map(|image| {
                    eprintln!("[plugin] decode succeeded for {}", path.display());
                    image
                });
            }
        }

        if ext == "avif" {
            eprintln!("[plugin] no plugin handles .avif; trying built-in image decoder");
        }
        
        // 5. プラグインになければ標準の image クレートで試行 (JPG, PNG など)
        let image = image::open(path)
            .map_err(|e| e.to_string())?;

        Ok(Box::new(StaticImage::new(image)))
    }
}
