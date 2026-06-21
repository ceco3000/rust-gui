//! 布局引擎——Taffy 封装。

use rgui_core::geometry::{Point, Size};
use rgui_core::id::WidgetId;
use rustc_hash::FxHashMap;
use std::fmt;
use taffy::TaffyTree;
use taffy::prelude::*;

/// 布局节点句柄。
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct LayoutNode(taffy::NodeId);

impl fmt::Debug for LayoutNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LayoutNode({:?})", self.0)
    }
}

/// 布局结果。
#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub size: Size,
    pub position: Point,
}

/// 布局缓存条目。
#[derive(Debug, Clone)]
pub struct CachedLayout {
    pub result: LayoutResult,
    pub children: Vec<LayoutResult>,
    /// 创建节点时传入的 Taffy Style（含 props 映射后的值）。
    pub style: Style,
}

/// 布局引擎——核心 Taffy 封装。
pub struct LayoutEngine {
    tree: TaffyTree,
    widget_to_node: FxHashMap<WidgetId, LayoutNode>,
    node_to_widget: FxHashMap<LayoutNode, WidgetId>,
    cache: FxHashMap<WidgetId, CachedLayout>,
}

impl LayoutEngine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
            widget_to_node: FxHashMap::default(),
            node_to_widget: FxHashMap::default(),
            cache: FxHashMap::default(),
        }
    }

    pub fn create_node(
        &mut self,
        widget_id: WidgetId,
        style: Style,
        children: &[LayoutNode],
    ) -> LayoutNode {
        let node = LayoutNode(self.tree.new_leaf(style).unwrap());

        for &child in children {
            if let Err(e) = self.tree.add_child(node.0, child.0) {
                eprintln!(
                    "[rgui] LayoutEngine::add_node: add_child({:?}, {:?}) 失败: {e:?}",
                    node, child
                );
            }
        }

        self.widget_to_node.insert(widget_id, node);
        self.node_to_widget.insert(node, widget_id);
        node
    }

    pub fn set_style(&mut self, node: LayoutNode, style: Style) {
        if let Err(e) = self.tree.set_style(node.0, style) {
            eprintln!("[rgui] LayoutEngine::set_style({node:?}) 失败: {e:?}");
        }
    }

    pub fn remove(&mut self, widget_id: WidgetId) {
        if let Some(node) = self.widget_to_node.remove(&widget_id) {
            self.node_to_widget.remove(&node);
            if let Err(e) = self.tree.remove(node.0) {
                eprintln!(
                    "[rgui] LayoutEngine::remove({widget_id:?}) → node({node:?}) 失败: {e:?}"
                );
            }
            self.cache.remove(&widget_id);
        }
    }

    pub fn compute_layout(&mut self, root: LayoutNode, available: Size) {
        if let Err(e) = self.tree.compute_layout(
            root.0,
            taffy::geometry::Size {
                width: AvailableSpace::Definite(available.width as f32),
                height: AvailableSpace::Definite(available.height as f32),
            },
        ) {
            eprintln!("[rgui] LayoutEngine::compute_layout({root:?}, {available:?}) 失败: {e:?}");
        }

        for (&node, &widget_id) in &self.node_to_widget {
            match self.tree.layout(node.0) {
                Ok(layout) => {
                    let result = LayoutResult {
                        size: Size::new(layout.size.width as f64, layout.size.height as f64),
                        position: Point::new(layout.location.x as f64, layout.location.y as f64),
                    };
                    let style = self.tree.style(node.0).unwrap_or(&Style::default()).clone();
                    self.cache.insert(
                        widget_id,
                        CachedLayout {
                            result,
                            children: Vec::new(),
                            style,
                        },
                    );
                },
                Err(e) => {
                    eprintln!(
                        "[rgui] LayoutEngine::compute_layout: tree.layout({node:?}) (widget={widget_id:?}) 失败: {e:?}"
                    );
                },
            }
        }
    }

    #[must_use]
    pub fn get_layout(&self, widget_id: WidgetId) -> Option<&CachedLayout> {
        self.cache.get(&widget_id)
    }

    /// 获取 widget 在窗口中的绝对坐标（累加所有祖先节点偏移）。
    ///
    /// `get_layout().position` 返回的是相对父节点的本地坐标。
    /// 此方法从 Taffy 树沿父链累加，得到窗口绝对坐标，供 hit_test 使用。
    #[must_use]
    pub fn absolute_position(&self, widget_id: WidgetId) -> Option<Point> {
        let node = self.widget_to_node.get(&widget_id)?;
        let layout = self.tree.layout(node.0).ok()?;
        let mut x = layout.location.x as f64;
        let mut y = layout.location.y as f64;
        let mut parent = self.tree.parent(node.0);
        while let Some(p) = parent {
            if let Ok(pl) = self.tree.layout(p) {
                x += pl.location.x as f64;
                y += pl.location.y as f64;
            }
            parent = self.tree.parent(p);
        }
        Some(Point::new(x, y))
    }

    pub fn clear(&mut self) {
        self.tree = TaffyTree::new();
        self.widget_to_node.clear();
        self.node_to_widget.clear();
        self.cache.clear();
    }

    #[must_use]
    pub fn widget_node(&self, widget_id: WidgetId) -> Option<LayoutNode> {
        self.widget_to_node.get(&widget_id).copied()
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for LayoutEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LayoutEngine")
            .field("nodes", &self.widget_to_node.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_new() {
        let engine = LayoutEngine::new();
        assert!(engine.get_layout(WidgetId::new()).is_none());
    }

    #[test]
    fn create_node_and_layout() {
        let mut engine = LayoutEngine::new();
        let widget_id = WidgetId::new();

        let style = Style {
            size: taffy::geometry::Size {
                width: Dimension::Length(100.0),
                height: Dimension::Length(50.0),
            },
            ..Default::default()
        };

        let node = engine.create_node(widget_id, style, &[]);
        engine.compute_layout(node, Size::new(800.0, 600.0));

        let cached = engine.get_layout(widget_id).unwrap();
        assert!((cached.result.size.width - 100.0).abs() < 0.1);
    }

    #[test]
    fn remove_node() {
        let mut engine = LayoutEngine::new();
        let widget_id = WidgetId::new();
        let _node = engine.create_node(widget_id, Style::default(), &[]);
        engine.remove(widget_id);
        assert!(engine.get_layout(widget_id).is_none());
    }
}
