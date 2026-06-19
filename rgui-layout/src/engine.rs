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
