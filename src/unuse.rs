    /// 上矢印で移動するフォルダを取得
    fn previous_directory(
        &self,
    ) -> Option<PathBuf> {
        let current_dir = self.current_path.parent()?.to_path_buf();
        let parent_dir = current_dir.parent()?.to_path_buf();
        let sibling_dirs = Self::get_subdirectries(&parent_dir);
        let current_index = sibling_dirs.iter().position(|d| d == &current_dir)?;

        if current_index > 0 {
            Some(sibling_dirs[current_index - 1].clone())
        } else {
            Some(parent_dir.to_path_buf())
        }
    }

    /// フォルダを階層順に取得する
    fn collect_image_directories(
        dir: &std::path::Path,
        plugin_mgr: &Arc<PluginManager>,
        result: &mut Vec<PathBuf>,
    ) {
        let subdirs = Self::get_subdirectries(dir);
        for subdir in subdirs {
            if !Self::get_image_files(&subdir, plugin_mgr).is_empty() {
                result.push(subdir.clone());
            }
            // サブフォルダ内を再帰的に収集
            Self::collect_image_directories(&subdir, plugin_mgr, result);
        }
    }

        fn change_directory(
        &mut self, 
        new_dir: PathBuf, 
        ctx: &egui::Context,    
        plugin_mgr: &Arc<PluginManager>,
    ) {
    //    println!("===== change_directory =====");
    //    println!("new_dir = {:?}", new_dir);

        let image_files = Self::get_image_files(&new_dir, plugin_mgr);

    //    println!("image_files = {:?}", image_files);

        if image_files.is_empty() {
    //    println!("画像がないため移動しません");
            return;
        }

        if new_dir.is_dir() {
            self.current_path = image_files[0].clone();

    //    println!(
    //            "current_path changed to = {:?}",
    //            self.current_path
    //        );

            self.directory_files = image_files;
            self.current_index = 0;

            self.texture = None;
            self.original_image_size = None;
            self.loading = true;

            self.keep_window_reposition = true;
            self.keep_window_on_screen(ctx);

            self.load_image_async(plugin_mgr.clone());
        }
    }
