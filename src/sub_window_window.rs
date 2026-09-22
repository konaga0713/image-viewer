
use crate::sub_window::SubWindow;

// 上部UI・タイトルバー等のための余裕
const TOP_MARGIN :f32 = 80.0;
const SIDE_MARGIN:f32 = 0.0;
// 小さい画像でも確保する最小表示領域
const MIN_DISPLAY_WIDTH: f32 = 100.0;
const MIN_DISPLAY_HEIGHT: f32 = 100.0;


#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::POINT;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW,
    MonitorFromPoint,
    MONITORINFO,
    MONITOR_DEFAULTTONEAREST,
};

impl SubWindow {
    /// サブウィンドウのサイズの設定
    /// タスクバー等を除いた使用可能領域を超えないようにする。
    pub(crate) fn resize_window_to_image(&self, ctx: &egui::Context) {
        let Some(original_size) = 
            self.original_image_size 
        else {
            return;
        };

        let Some(outer_rect) = 
            ctx.input(|i| i.viewport().outer_rect) 
        else {
            return;
        };

        let Some(work_rect) = 
            Self::get_avaivable_screen_rect(ctx) 
        else {
            return;
        };

        // ウインドウのサイズを取得
        //　タイトルバーや枠を含む
        let inner_size = ctx
            .input(|i| i.viewport().inner_rect)
            .map(|r| r.size())
            .unwrap_or(outer_rect.size()
        );
        let decoration = egui::vec2(
            (outer_rect.width() - inner_size.x).max(0.0),
            (outer_rect.height() - inner_size.y).max(0.0),
        );

        // 現在のサブウィンドウ位置
        let position = outer_rect.min;
        // タスクバーを除いた使用可能領域から
        // 現在位置より右・下に残っている領域を取得
        let available_outer_width = (work_rect.max.x - position.x).max(0.0);
        let available_outer_height = (work_rect.max.y - position.y).max(0.0);

        // 拡大率を適用した目標の画像サイズ
        let scaled_image_size = original_size * self.zoom_scale;

println!(
    "fit detail: original={:?}, zoom={}, scaled={:?}",
    original_size,
    self.zoom_scale,
    scaled_image_size
);

    if self.fit_to_screen {
        // 自動縮小ON: 利用可能な内部描画エリアに合わせて縮小
        let max_image_width = (available_outer_width - decoration.x - (SIDE_MARGIN * 2.0)).max(0.0);
        let max_image_height = (available_outer_height - decoration.y - TOP_MARGIN).max(0.0);
        
        let scale_x = max_image_width / scaled_image_size.x;
        let scale_y = max_image_height / scaled_image_size.y;

        // 小さい画像は拡大しない
        let scale = scale_x.min(scale_y).min(1.0);

println!(
    "fit detail: max={},{} scale_x={} scale_y={} scale={}",
    max_image_width,
    max_image_height,
    scale_x,
    scale_y,
    scale
);            
        // サブウィンドウサイズ
        let window_size = egui::vec2(
            ((scaled_image_size.x * scale) + SIDE_MARGIN * 2.0)
                .max(MIN_DISPLAY_WIDTH),
            ((scaled_image_size.y * scale) + TOP_MARGIN)
                .max(MIN_DISPLAY_HEIGHT),
        );
println!(
    "fit: available={},{} window={},{}",
    available_outer_width,
    available_outer_height,
    window_size.x,
    window_size.y
);
            // ウィンドウサイズ変更
            ctx.send_viewport_cmd(
            egui::ViewportCommand::InnerSize(window_size),  
            );
        } else {
            // 原寸表示（自動縮小OFF）:
            // 目標とするInnerサイズ（画像原寸 + UIマージン）            
            let target_inner = egui::vec2(
                (scaled_image_size.x + SIDE_MARGIN * 2.0)
                    .max(MIN_DISPLAY_WIDTH),
                (scaled_image_size.y + TOP_MARGIN)
                    .max(MIN_DISPLAY_HEIGHT),
            );

            // 目標サイズに枠（decoration）を足したOuterサイズ
            let target_outer = target_inner + decoration;      
            // 現在位置から、タスクバーを除いた領域までの最大Outerサイズ                  
            let max_outer_width = (work_rect.max.x - position.x).max(100.0);
            let max_outer_height = (work_rect.max.y - position.y).max(100.0);

            // ウィンドウサイズを使用可能領域まで制限する。
            let final_outer_width = target_outer.x.min(max_outer_width);
            let final_outer_height = target_outer.y.min(max_outer_height);

            // サブウィンドウサイズ
            let final_inner = egui::vec2(
                (final_outer_width - decoration.x).max(100.0),
                (final_outer_height - decoration.y).max(100.0),
            );

    println!(
        "original: image={},{} available={},{} final={},{}",
        scaled_image_size.x,
        scaled_image_size.y,
        available_outer_width,
        available_outer_height,
        final_inner.x,
        final_inner.y
    );          
            ctx.send_viewport_cmd(
            egui::ViewportCommand::InnerSize(final_inner),  
            );
            
        } 
    }

    /// 原寸大表示時に、実際の outer_rect が
    /// Windowsの作業領域を超えていないか確認し、
    /// 超過している場合だけウィンドウサイズを縮小する。
    fn correct_window_size_to_work_area(&mut self, ctx: &egui::Context) {
        let Some(outer_rect) = 
            ctx.input(|i| i.viewport().outer_rect) 
        else {
            return;
        };
        
        let Some(work_rect) = 
            Self::get_avaivable_screen_rect(ctx)  
        else {
            return;
        };

        // 現在の outer_rect が作業領域を超えているか確認        
        let overflow_x = (outer_rect.max.x - work_rect.max.x).max(0.0);
        let overflow_y = (outer_rect.max.y - work_rect.max.y).max(0.0);

        println!(
    "correction check: outer_max={:?}, work_max={:?}, overflow=({:.1},{:.1})",
    outer_rect.max,
    work_rect.max,
    overflow_x,
    overflow_y
);
        // はみ出していなければ補正終了
        if overflow_x <= 0.0 && overflow_y <= 0.0 {
            return;
        }
    
        // 作業領域内に収まる outer サイズを計算
        let allowed_outer_width = (work_rect.max.x - outer_rect.min.x).max(100.0);
        let allowed_outer_height = (work_rect.max.y - outer_rect.min.y).max(100.0);
        let allowed_outer_size  = egui::vec2(
            allowed_outer_width,
            allowed_outer_height,
        );

        // outer と inner の差を取得
        let current_inner_size = 
            ctx.input(|i| i.viewport().inner_rect)
                .map(|r| r.size());

        let Some(current_inner_size) = 
            current_inner_size 
        else {
            return;
        };    

        let decoration = egui::vec2(
            (outer_rect.width() - current_inner_size.x).max(0.0),
            (outer_rect.height() - current_inner_size.y).max(0.0),
        );

        // InnerSize に指定するサイズ
        let corrected_inner = egui::vec2(
            (allowed_outer_size.x - decoration.x).max(100.0),
            (allowed_outer_size.y - decoration.y).max(100.0),
        );

        println!(
        "correction: outer_size={:?}, \
        allowed_outer={:?}, decoration={:?}, \
        corrected_inner={:?}",
        outer_rect.size(),
        allowed_outer_size,
        decoration,
        corrected_inner,
    );

        // 位置は変更しない。
        // サイズだけ変更する。
        ctx.send_viewport_cmd(
            egui::ViewportCommand::InnerSize(corrected_inner)
        );

    }

    /// タスクバー等を除いた、現在のサブウィンドウが存在する
    /// ディスプレイの使用可能領域を取得する。
    ///
    /// Windows: GetMonitorInfoW() の rcWork を使用する。
    ///
    /// Windows以外: egui の monitor_size を使用する。
    fn get_avaivable_screen_rect(ctx: &egui::Context) -> Option<egui::Rect> {
        let (outer_rect, monitor_size, pixcels_per_point) = ctx.input(|i| {
            let viewport = i.viewport();
            (
                viewport.outer_rect,
                viewport.monitor_size,
                viewport.native_pixels_per_point.unwrap_or(1.0),
            )
        });

        let outer_rect = outer_rect?;

        #[cfg(target_os = "windows")]
        {
            let scale = pixcels_per_point;
            
            // egui上のウィンドウ位置をWindowsの物理ピクセルへ変換
            let point = POINT {
                x: (outer_rect.min.x * scale).round() as i32,
                y: (outer_rect.min.y * scale).round() as i32,
            };      
            // 現在のウィンドウ位置にあるディスプレイを取得
            let hmonitor = unsafe {
                MonitorFromPoint(
                    point,
                    MONITOR_DEFAULTTONEAREST,
                )      
            };

            if hmonitor == std::ptr::null_mut() {
                return None;
            }
            // Windowsのモニター情報を取得
            let mut monitor_info: MONITORINFO = 
                unsafe { std::mem::zeroed()};
            monitor_info.cbSize = 
                std::mem::size_of::<MONITORINFO>() as u32;

            let result = unsafe {
                GetMonitorInfoW(
                    hmonitor,
                    &mut monitor_info as *mut MONITORINFO,
                )
            };    
            if result == 0 {
                return None;
            }

            // rcWork:
            // タスクバー等を除いた使用可能領域
            let work = monitor_info.rcWork;
            Some(egui::Rect::from_min_max(
                egui::pos2(
                    work.left as f32 / scale,
                    work.top as f32 / scale,
                ),
                 egui::pos2(
                    work.right as f32 / scale,
                    work.bottom as f32 / scale,
                ),
            ))
        }

        #[cfg(not(target_os = "windows"))]
        {
            let monitor_size = monitor_size?;
            Some(egui::REct::from_min_size(
                egui::Pos2::Zero,
                monitor_size,
            ))
        }
    }

}