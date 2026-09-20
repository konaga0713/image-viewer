/// sub_window_window
use std::path::PathBuf;
use std::sync::Arc;
use eframe::egui;

use crate::plugin::PluginManager;
use crate::sub_window::SubWindow;

impl SubWindow {
    // 画像フォルダの切り替え
    pub(crate) fn change_directory(
        &mut self, 
        new_dir: PathBuf, 
        ctx: &egui::Context,    
        plugin_mgr: &Arc<PluginManager>,
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
        self.load_async(plugin_mgr.clone(), ctx.clone(),);

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

        // 現在フォルダにサブフォルダがあれば、最初のサブフォルダへ
        let sub_dirs = Self::get_subdirectories(&current_dir);
        for sub_dir in &sub_dirs {
            if !Self::get_image_files(&sub_dir, plugin_mgr).is_empty() {
                return Some(sub_dir.clone().to_path_buf());
            }
        }

        // サブフォルダがなければ、親フォルダの兄弟フォルダを探す
        let mut dir = current_dir;

        loop{
            let parent_dir = dir.parent()?.to_path_buf();
            let sibling_dirs = Self::get_subdirectories(&parent_dir);
            let image_index = sibling_dirs.iter().position(|d| d == &dir)?;

            // 現在フォルダより後ろにある兄弟フォルダを探す
            for target_dir in sibling_dirs.iter().skip(image_index + 1) {

                if !Self::get_image_files(target_dir, plugin_mgr).is_empty() {
                    println!("FOUND = {:?}", target_dir);
                    return Some(target_dir.clone().to_path_buf());
                }
            }
            // 次の兄弟がなければ、さらに親へ
            dir = parent_dir;
        }

    }

    /// 上矢印で移動するフォルダを取得
    ///
    /// 優先順位:
    /// 1. サブフォルダ
    /// 2. 次の兄弟フォルダ
    pub(crate) fn previous_directory(
        &self,
        plugin_mgr: &Arc<PluginManager>,
    ) -> Option<PathBuf> {
        // 現在表示している画像のフォルダ
        let current_dir = self.current_path.parent()?.to_path_buf();

        // ルートまでたどる
        let mut dir = current_dir.clone();

        loop {
            let parent_dir = dir.parent()?.to_path_buf();
            let sibling_dirs = Self::get_subdirectories(&parent_dir);
            let index = sibling_dirs.iter().position(|d| d == &dir)?;

            // 現在フォルダより前にある兄弟フォルダを探す
            for target_dir in sibling_dirs.iter().take(index).rev() {
                if !Self::has_image_directory(target_dir, plugin_mgr) {
                    continue;
                } else {
                    return Some(Self::last_directory(target_dir, plugin_mgr));
                }
            }

            // 親フォルダに戻る
            if !Self::get_image_files(&parent_dir, plugin_mgr).is_empty() {
                return Some(parent_dir);
            }
            dir = parent_dir;
        }
    }

    /// 指定フォルダ以下で、深さ優先順の最後に位置する
    /// 「画像を持つフォルダ」を取得する。
    pub(crate) fn last_directory(
        dir: &PathBuf,
        plugin_mgr: &Arc<PluginManager>,
    ) -> PathBuf {
        let sub_dirs = Self::get_subdirectories(dir);
        // 最後の子孫から検索
        for sub_dir in sub_dirs.iter().rev() {
            if let Some(last) = Self::last_image_directory(sub_dir, plugin_mgr,) {
                return last;
            }
        }

        if !Self::get_image_files(dir, plugin_mgr).is_empty() {
            return dir.clone();
        }
        dir.clone()
    }
    
    pub(crate) fn last_image_directory(
        dir: &PathBuf,
        plugin_mgr: &Arc<PluginManager>,
    ) -> Option<PathBuf> {

        let sub_dirs = Self::get_subdirectories(dir);

        // 最後の子孫から検索
        for sub_dir in sub_dirs.iter().rev() {

            if let Some(last) = Self::last_image_directory(
                sub_dir,
                plugin_mgr,
            ) {
                return Some(last);
            }
        }

        // 自分自身が画像フォルダなら採用
        if !Self::get_image_files(dir, plugin_mgr).is_empty() {
            return Some(dir.clone());
        }

        None
    }    

    fn has_image_directory(dir: &PathBuf, plugin_mgr: &Arc<PluginManager>,
    ) -> bool {
        if !Self::get_image_files(dir, plugin_mgr).is_empty() {
            return true;
        }

        for sub in Self::get_subdirectories(dir) {
            if Self::has_image_directory(&sub, plugin_mgr) {
                return true;
            }
        }

        false
    }


}
