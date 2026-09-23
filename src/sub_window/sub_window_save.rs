///sub_window_save

use super::SubWindow;

impl SubWindow {
    /// 画像保存
    fn save_current_image(&self) {    
        let Some(image) = 
            &self.image 
        else {
            return;
        };    
        if let Err(err) = image.save(&self.current_path) {
                eprintln!("{}",err);
        }
    }

    /// 回転などによる変更がある場合は、そのまま保存する。
    /// 変更がない場合は保存確認ダイアログを表示する。
    pub fn request_save(&mut self) {
        
        let Some(image) = &self.image 
        else {
            return;
        };   

        if image.is_modified() {
            self.save_current_image();
        } else {
            self.show_save_confirm = true;
        }    
    } 

    // ------------------------------------------------------------
    // 保存確認
    // ------------------------------------------------------------
    pub fn show_save_confirm(&mut self, ctx: &egui::Context){
        if !self.show_save_confirm { 
            return;
        }

        egui::Window::new("画像保存の確認")
            .collapsible(false)
            .resizable(false)
            .show(&ctx, |ui| {
                ui.label("この画像を保存しますか？");
                ui.horizontal(|ui| {
                    if ui.button("保存").clicked() {
                        self.save_current_image();
                        self.show_save_confirm = false;
                    }
                    if ui.button("キャンセル").clicked() {
                        self.show_save_confirm =false;
                    }
                });
            });
    }

}
