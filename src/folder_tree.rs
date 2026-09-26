/// app_tree
// ↑ ↓  : 同じ階層のフォルダを移動
// →    : 1回目＝展開、2回目＝最初の子フォルダへ
// ←    : 親フォルダへ

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
                .unwrap_or_default();
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
    parent_path: Option<PathBuf>,
    name: String,
    depth: usize,
    is_expanded: bool,
}

pub struct FolderTree {
    root: FolderNode,
    selected_path: PathBuf,             //現在選択されているフォルダ
    visible_items: Vec<VisibleItem>,    // 表示用の平坦化されたリスト
    last_notified_path: PathBuf,        // 前回MyAppへ通知した選択フォルダ
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
        };

        // selected_pathまでの経路を展開
        tree.expand_path_to(&tree.selected_path.clone());
        tree.rebuild_visible_items();
        tree

    }

    /// Tree UIを描画する。
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        self.handle_keyboard(ui.ctx());
        self.rebuild_visible_items();

        let mut clicked_path = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for item in &self.visible_items {
                let selected = self.selected_path == item.path;

                let path = item.path.clone();

                ui.horizontal(|ui| {
                    ui.add_space(item.depth as f32 * 18.0);
                    let icon = if item.is_expanded {
                        "▼"
                    } else {
                        "▶"
                    };

                    let label = format!("{} {}", icon, item.name);
                    let response = ui.selectable_label(selected, label);

                    if response.clicked() {
                        clicked_path = Some(path.clone());
                    }
                    if response.double_clicked() {
                        clicked_path = Some(path.clone());
                    }
                });
            }
        });

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
                self.move_sibling(-1);
            }
            if input.key_pressed(egui::Key::ArrowDown) {
                self.move_sibling(1);
            }
            if input.key_pressed(egui::Key::ArrowRight) {
                self.move_child();
            }
            if input.key_pressed(egui::Key::ArrowLeft) {
                self.move_parent();
            }
        });
    }

    fn move_sibling(&mut self, direction: i32) {
        let Some(current) = self.find_visible(&self.selected_path) else {
            return;
        };

        let Some(parent_path) = current.parent_path.clone() else {
            return;
        };

        let siblings: Vec<PathBuf> =self
            .visible_items
            .iter()
            .filter(|item| item.parent_path.as_ref() == Some(&parent_path))
            .map(|item| item.path.clone())
            .collect();

        let Some(current_index) = siblings
            .iter()
            .position(|path| path == &self.selected_path)
        else {
            return;
        };

        let new_index = current_index as i32 + direction;
        if new_index < 0 || new_index >= siblings.len() as i32 {
            return;
        }

            self.selected_path = siblings[new_index as usize].clone();
    }

    /// 1回目:
    ///     未展開なら展開する
    ///
    /// 2回目:
    ///     展開済みなら最初の子フォルダへ移動する
    fn move_child(&mut self) {
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
            return;
        }

        // 展開済みなら最初の子フォルダへ
        self.selected_path = node.children[0].path.clone();
    }

    /// 親フォルダへ移動する。
    fn move_parent(&mut self) {
        let Some(current) = self.find_visible(&self.selected_path) else {
            return;
        };

        let Some(parent_path) = current.parent_path.clone() else {
            return;
        };    

        self.selected_path = parent_path;
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
            None,
            0,
            &mut self.visible_items,
        );
    }

    fn collect_visible(
        node: &FolderNode,
        parent_path: Option<PathBuf>,
        depth: usize,
        output: &mut Vec<VisibleItem>,
    ) {
        output.push(VisibleItem { 
            path: node.path.clone(),
            parent_path: parent_path.clone(),
            name: node.name.clone(),
            depth,
            is_expanded: node.is_expanded,
        });

        if node.is_expanded {
            for child in &node.children {
                Self::collect_visible(
                    child,
                    Some(node.path.clone()),
                    depth + 1,
                    output,
                );
            }

        }
    }

    fn find_visible(&self, path: &Path) -> Option<VisibleItem> {
        self.visible_items
            .iter()
            .find(|item| item.path == path)
            .cloned()
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

        for component in relative.components() {
            current.push(component);

            let Some(node) = self.root.find_mut(&current) else {
                return;
            };
            node.load_children();
            node.is_expanded = true;
        }
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
    } 
}