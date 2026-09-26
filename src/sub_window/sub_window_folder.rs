/// sub_window_folder
use std::path::PathBuf;
use std::sync::Arc;
use eframe::egui;

use crate::image_cache::ImageCache;
use crate::plugin::PluginManager;

use super::SubWindow;

impl SubWindow {
    // 画像フォルダの切り替え
    pub(crate) fn change_directory(
        &mut self, 
        new_dir: PathBuf, 
        ctx: &egui::Context,    
        plugin_mgr: &Arc<PluginManager>,
        image_cache: &mut ImageCache,
    ) {
        let image_files = Self::get_image_files(&new_dir, plugin_mgr);
        if image_files.is_empty() {
            return;
        }

        println!("new_dir = {:?}", new_dir);    
        // 画像一覧を取得        
        self.directory_files = image_files;
        self.current_path = self.directory_files[0].clone();
        self.image_index = 0;

        self.texture = None;
        self.image = None;
        self.original_image_size = None;
        self.loading = true;
        self.animation_next_frame_time = None;
        self.load_async(plugin_mgr.clone(), image_cache, ctx.clone(),);

    }

    pub(crate) fn get_subdirectories(dir: &std::path::Path) -> Vec<PathBuf> {
        let mut subdirs = Vec::new();
     
        if let Ok(entries) = std::fs::read_dir(dir) {
            subdirs = entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|pb| pb.is_dir())
                .collect();

            subdirs.sort_by_key(|p| {
                p.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase()
            });
        }

        subdirs
    }

    /// 指定フォルダにある画像ファイルを取得
    pub(crate) fn get_image_files(
        dir: &std::path::Path,
        plugin_mgr: &Arc<PluginManager>,
    ) -> Vec<PathBuf> {
        let mut files = Vec::new();
     
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let pb = entry.path();
                if pb.is_file() && plugin_mgr.can_decode(&pb) {
                    files.push(pb);
                }
            }
        }
        files.sort();
        files
    }

    /// 下矢印で移動するフォルダを取得
    ///
    /// 優先順位:
    /// 1. サブフォルダ
    /// 2. 次の兄弟フォルダ
    pub(crate) fn get_next_directory(
        &self,
        plugin_mgr: &Arc<PluginManager>,
    ) -> Option<PathBuf> {
        // 現在表示している画像のフォルダ
        let current_dir = self.current_path.parent()?.to_path_buf();

        let mut dir = current_dir;

        loop{
            let next = Self::next_dfs_directory(&dir)?;
            // println!("DFS NEXT CHECK = {:?}", next);

            if !Self::get_image_files(&next, plugin_mgr).is_empty() {
                // println!("DFS NEXT FOUND = {:?}", next);
                return Some(next);
            }
            // 次の兄弟がなければ、さらに親へ
            dir = next;
        }

    }

    /// 上矢印で移動するフォルダを取得
    ///
    /// 優先順位:
    /// 1. サブフォルダ
    /// 2. 次の兄弟フォルダ
    fn previous_directory(&self, plugin_mgr: &Arc<PluginManager>,) -> Option<PathBuf> {
        let current_dir = self.current_path.parent()?.to_path_buf();

        let mut dir = current_dir;
        loop {
            let previous = Self::previous_dfs_directory(&dir)?;
            // println!("DFS PREV CHECK = {:?}", previous);

            if !Self::get_image_files(&previous, plugin_mgr).is_empty() {
                println!("DFS PREV FOUND = {:?}", previous);
                return Some(previous);
            }
            dir = previous
        }
    }

    // ------------------------------------------------------------
    // ↑ 前のフォルダ
    // ------------------------------------------------------------
    pub fn move_to_previous_directory (&mut self, ctx: &egui::Context, plugin_mgr: &Arc<PluginManager>, image_cache: &mut ImageCache) {
        println!("===== ArrowUp pressed =====");
        println!("current_path = {:?}", self.current_path);

        if let Some(new_dir) = self.previous_directory(plugin_mgr) {
            println!("TREE PREV = {:?}", new_dir);
            self.change_directory(new_dir, &ctx, plugin_mgr, image_cache);
        } else {
            println!("PREV = None");
        }
    }

    // ------------------------------------------------------------
    // ↓ 次のフォルダ
    // ------------------------------------------------------------
    pub fn move_to_next_directory (&mut self, ctx: &egui::Context, plugin_mgr: &Arc<PluginManager>, image_cache: &mut ImageCache) {
        println!("===== ArrowDown pressed =====");
        println!("current_path = {:?}", self.current_path);

        if let Some(new_dir) = self.get_next_directory(plugin_mgr) {
            println!("NEXT = {:?}", new_dir);

            self.change_directory(new_dir, &ctx, plugin_mgr, image_cache);
        } else {
            println!("NEXT = None");
        }
    }

    fn next_dfs_directory(dir: &PathBuf,) -> Option<PathBuf> {
         // 子フォルダがあれば最初の子へ    
         let sub_dirs = Self::get_subdirectories(dir);

         if let Some(first) = sub_dirs.first() {
                return Some(first.clone());
         }

        // 子がなければ兄弟を探す         
        let mut current = dir.clone();

        loop {
            let parent = current.parent()?.to_path_buf();
            let siblings = Self::get_subdirectories(&parent);
            let index = siblings
                .iter()
                .position(|d| d == &current)?;

            // 次の兄弟
            if let Some(next) = siblings.get(index + 1) {
                return Some(next.clone());
            }            

            // 兄弟がなければ親へ戻る
            current = parent;
        }
    }

    fn previous_dfs_directory(dir: &PathBuf,) -> Option<PathBuf> {
        let parent = dir.parent()?.to_path_buf();
        let siblings = Self::get_subdirectories(&parent);

        let index = siblings
            .iter()
            .position(|d| d ==dir)?;

        // 前の兄弟がある場合            
        if index > 0 {
            let previous_sibling = &siblings[index -1];
            return Some(Self::last_dfs_directory(previous_sibling));
        }
        // 兄弟がなければ親へ戻る
        Some(parent)
    }

    /// 指定フォルダ以下で、深さ優先順の最後に位置する
    /// 「画像を持つフォルダ」を取得する。
    fn last_dfs_directory(dir: &PathBuf,) -> PathBuf {
        let sub_dirs = Self::get_subdirectories(dir);

        if let Some(last) = sub_dirs.last() {
            return Self::last_dfs_directory(last);
        }

        dir.clone()
    }
}
