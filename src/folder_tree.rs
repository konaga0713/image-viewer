/// app_tree
// ↑ ↓ : 見えているフォルダを移動
// → : 自フォルダを展開
// ← : 自フォルダを格納

use std::path::{Path, PathBuf};
use eframe::egui;

#[derive(Debug, Clone)]
pub struct FolderNode {
    pub path: PathBuf,
    pub name: String,
    pub is_expanded: bool,          //展開済み
    pub children_loaded: bool,      //子フォルダ読込済み
    pub children: Vec<FolderNode>,  //子フォルダ
}

impl FolderNode {
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        Self {
            path,
            name,
            is_expanded: false,
            children_loaded: false,
            children: Vec::new(), 
        }
    }

    pub fn load_children(&mut self) {
        if self.children_loaded {
            return;
        }

        self.children.clear();
        let Ok(entries) = std::fs::read_dir(&self.path) else {
            self.children_loaded = true;
            return;
        };

        let mut dirs = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                dirs.push(path);
            }
        }

        dirs.sort_by_key(|a| {
            a.file_name()
                .map(|v| v.to_string_lossy().to_lowercase())
                .unwrap_or_default()
        });

        self.children = dirs
            .into_iter()
            .map(FolderNode::new)
            .collect();

        self.children_loaded = true;

    }

    pub fn find_mut(&mut self, path: &Path) -> Option<&mut FolderNode> {
        if self.path == path {
            return Some(self);
        }

        if !self.children_loaded {
            return None;
        }

        for child in &mut self.children {
            if let Some(node) =child.find_mut(path) {
                return Some(node);
            }
        }

        None
    }

    pub fn find(&self, path: &Path) -> Option<&FolderNode> {
        if self.path == path {
            return Some(self)
        }

        for child in &self.children {
            if let Some(node) = child.find(path) {
                return Some(node)
            }
        }

        None
    }
}

#[derive(Debug, Clone)]
struct VisibleItem {
    path: PathBuf,
    name: String,
    depth: usize,
    is_expanded: bool,
}

pub struct FolderTree {
    root: FolderNode,
    selected_path: PathBuf,             //現在選択されているフォルダ
    visible_items: Vec<VisibleItem>,    // 表示用の平坦化されたリスト
    last_notified_path: PathBuf,        // 前回MyAppへ通知した選択フォルダ
    visible_dirty: bool,
    scroll_to_selected: bool,
}

impl FolderTree {
    pub fn new(root_path: PathBuf, selected_path: PathBuf) -> Self {
        let mut root = FolderNode::new(root_path);

        // ルートを展開
        root.load_children();
        root.is_expanded = true;

        let mut tree = Self {
            root,
            selected_path: selected_path.clone(),
            visible_items: Vec::new(),
            last_notified_path: selected_path,
            visible_dirty: true,
            scroll_to_selected: true,
        };

        // selected_pathまでの経路を展開
        tree.expand_path_to(&tree.selected_path.clone());
        tree.rebuild_visible_items();
        tree

    }

    /// Tree UIを描画する。
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        self.handle_keyboard(ui.ctx());

        if self.visible_dirty {
            self.rebuild_visible_items();
        }

        let mut clicked_path = None;
        let visible_items = self.visible_items.clone();
        let selected_path = self.selected_path.clone();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for item in &visible_items {
                let selected = selected_path == item.path;

                let path = item.path.clone();

                ui.horizontal(|ui| {
                    ui.add_space(item.depth as f32 * 18.0);
                    let icon = if item.is_expanded {
                        "[-]"
                    } else {
                        "[+]"
                    };

                    let label = format!("{} {}", icon, item.name);

                    let response = ui.add(
                        egui::Button::selectable(selected, label)
                            .truncate(),
                    );

                    if response.clicked() {
                        self.select_and_toggle(path.clone());
                    }
                    if response.double_clicked() {
                        clicked_path = Some(path.clone());
                    }

                    // 現在選択されているフォルダを画面内へ移動        
                    if selected && self.scroll_to_selected {
                        let visible_rect = ui.clip_rect();

                        if !visible_rect.intersects(response.rect) {
                            ui.scroll_to_rect(
                                response.rect, 
                                Some(egui::Align::TOP));
                        }
                    }

                });
            }
        });

        self.scroll_to_selected = false;

        if let Some(path) = clicked_path {
            self.select_path(path);
        }

        if self.selected_path != self.last_notified_path {
            self.last_notified_path = self.selected_path.clone();
            return Some(self.selected_path.clone());
        }

        None
    }

    // ============================================================
    // キーボード操作
    // ============================================================
    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        ctx.input(|input| {
            if input.key_pressed(egui::Key::ArrowUp) {
                self.move_visible(-1);
            }
            if input.key_pressed(egui::Key::ArrowDown) {
                self.move_visible(1);
            }
            if input.key_pressed(egui::Key::ArrowRight) {
                self.expand_current();
            }
            if input.key_pressed(egui::Key::ArrowLeft) {
                self.collapse_current();
            }
        });
    }

    fn move_visible(&mut self, direction: i32) {
        let Some(current_index) = self
            .visible_items
            .iter()
            .position(|item| item.path == self.selected_path)
        else {
            return;
        };

        let new_index = current_index as i32 + direction;

        if new_index < 0 || new_index >= self.visible_items.len() as i32 {
            return;
        }

        self.selected_path = self.visible_items[new_index as usize].path.clone();

        self.scroll_to_selected = true;
    }


    ///     未展開なら展開する
    fn expand_current(&mut self) {
        let path = self.selected_path.clone();

        let Some(node) = self.root.find_mut(&path) else {
            return;
        };

        node.load_children();

        if node.children.is_empty() {
            return;
        }

        // まだ展開していない
        if !node.is_expanded {
            node.is_expanded = true;
            self.visible_dirty = true;
        }
    }

    /// フォルダの格納
    fn collapse_current(&mut self) {
        let path = self.selected_path.clone();

        let Some(node) = self.root.find_mut(&path) else {
            return;
        };

        // 展開時は格納する
        if node.is_expanded {
            node.is_expanded = false;
            self.visible_dirty = true;
        }
    }

    // 選択
    fn select_path(&mut self, path: PathBuf) {
        if self.selected_path == path {
            return;
        }
        self.selected_path = path;
    }

    // Tree構築
    fn rebuild_visible_items(&mut self) {
        self.visible_items.clear();

        Self::collect_visible(
            &self.root,
            0,
            &mut self.visible_items,
        );

        self.visible_dirty = false;
    }

    fn collect_visible(
        node: &FolderNode,
        depth: usize,
        output: &mut Vec<VisibleItem>,
    ) {
        output.push(VisibleItem { 
            path: node.path.clone(),
            name: node.name.clone(),
            depth,
            is_expanded: node.is_expanded,
        });

        if node.is_expanded {
            for child in &node.children {
                Self::collect_visible(
                    child,
                    depth + 1,
                    output,
                );
            }

        }
    }

    /// selected_pathまでの親をすべて展開する
    fn expand_path_to(&mut self, target: &Path) {

        if self.root.path == target {
            return;
        }

        let Some(relative) = target.strip_prefix(&self.root.path).ok() else {
            return;
        };

        let mut current = self.root.path.clone();
        let mut node = &mut self.root;

        for component in relative.components() {

            node.load_children();
            node.is_expanded = true;

            current.push(component);

            let Some(next) = node.children.iter_mut()
                .find(|c| c.path == current)
            else {
                return;
            };

            node = next;

        }    
        
        node.load_children();
        node.is_expanded = true;
        
        self.visible_dirty = true;
    } 

    pub fn selected_path(&self) -> &Path {
        &self.selected_path
    }

    pub fn set_current_path(&mut self, root_path: PathBuf, selected_path: PathBuf) {
        self.root = FolderNode::new(root_path);

        self.root.load_children();
        self.root.is_expanded = true;
        self.selected_path = selected_path.clone();
        self.expand_path_to(&selected_path);
        self.rebuild_visible_items();
        self.scroll_to_selected = true;
    } 

    fn select_and_toggle(&mut self, path: PathBuf) {
        self.selected_path = path.clone();

        let Some(node) = self.root.find_mut(&path) else {
            return;
        };

        node.load_children();

        if node.children.is_empty() {
            return;
        }

        node.is_expanded = !node.is_expanded;
        self.visible_dirty = true;
    }

}