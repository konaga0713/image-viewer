/// app_config

use std::path::PathBuf;
use std::fs;

use serde::{Serialize, Deserialize};
use directories::ProjectDirs;

#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    pub last_folder: Option<PathBuf>,
    pub tree_width: f32,
}

impl AppConfig {
    ///設定ファイルパス取得
    fn config_path() -> Option<PathBuf> {
        let dirs = ProjectDirs::from(
            "com",
            "konaga0713",
            "ImageViewer",
        )?;
        Some(
            dirs.config_dir()
                .join("config.toml")
        )
    } 

    pub fn load() -> Self {
        let Some(path) = Self::config_path()
            else {
                return Self::default();
            };

        let Ok(data) = fs::read_to_string(&path)
            else {
                return Self::default();
            };

        println!("config path = {:?}", Self::config_path());
        
        match toml::from_str::<Self>(&data) {
            Ok(config) =>config,
            Err(err) => {
                eprintln!(
                    "設定ファイルの読み込みに失敗しました: {:?}\n{}",
                    path,
                    err
                );                  
                Self::default()
            }
        } 
    }    

    pub fn save(&self){
        println!("config path = {:?}", Self::config_path());        
        let Some(path) = Self::config_path()
            else {
                eprintln!("設定ファイルの保存先を取得できませんでした");
                return;                
            };

        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                eprintln!(
                    "設定ディレクトリの作成に失敗しました: {:?}\n{}",
                    parent,
                    err
                );
                return;                
            }
        }

        // TOMLへ変換
        let data = match toml::to_string_pretty(self) {
            Ok(data) => data,
            Err(err) => {
                eprintln!(
                    "設定データの変換に失敗しました: {}",
                    err
                );
                return;                
            }
        };

        // ファイルへ保存
        if let Err(err) = fs::write(&path, data) {
            eprintln!(
                "設定ファイルの保存に失敗しました: {:?}\n{}",
                path,
                err
            );            
        }  
    } 

}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            last_folder: None,
            tree_width: 250.0,
        }
    }
}

