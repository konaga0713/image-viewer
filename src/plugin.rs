use image::DynamicImage;
use libloading::{Library, Symbol};
use std::path::Path;

// プラグインが実装すべきトレイト定義
pub trait ImageDecoderPlugin: Send + Sync {
    fn supported_extensions(&self) -> Vec<&'static str>;
    fn decode(&self, path: &Path) -> Result<DynamicImage, String>;
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
        if !plugin_dir.exists() {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(plugin_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() {
                    unsafe {
                        if let Ok(lib) = Library::new(&path) {
                            if let Ok(func) = lib.get::<Symbol<CreatePluginFn>>(b"create_plugin") {
                                self.plugins.push(func());
                                self._libs.push(lib);
                            }
                        }
                    }
                }
            }
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

    pub fn try_decode(&self, path: &Path) -> Result<DynamicImage, String> {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        // 1. まずはプラグインから検索
        for plugin in &self.plugins {
            if plugin.supported_extensions().contains(&ext.as_str()) {
                return plugin.decode(path);
            }
        }

        // 2. プラグインになければ標準の image クレートで試行 (JPG, PNG など)
        image::open(path).map_err(|e| e.to_string())
    }
}    