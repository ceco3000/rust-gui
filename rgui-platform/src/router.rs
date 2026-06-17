//! 事件路由器——捕获/目标/冒泡三阶段路由（D5 §3）。

use crate::event::{Event, EventSender};
use crate::widget_tree::WidgetTree;
use rgui_core::id::WidgetId;

/// 事件路由阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutePhase {
    /// 捕获阶段：根 → 目标（自上而下）。
    Capture,
    /// 目标阶段：在目标 widget 上。
    Target,
    /// 冒泡阶段：目标 → 根（自下而上）。
    Bubble,
}

/// 路由结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteOutcome {
    /// 事件未被任何处理器处理。
    Unhandled,
    /// 事件已被处理，继续传播。
    Handled,
    /// 事件已被消费，停止传播。
    Consumed,
}

/// 事件路由回调：`(widget_id, phase, event, sender) -> RouteOutcome`。
///
/// `sender` 提供事件传播控制：处理器可通过 `sender.consume()` 停止传播，
/// 或通过 `sender.prevent_default()` 阻止默认行为。
#[allow(clippy::type_complexity)]
pub type RouteCallback<'a> =
    Box<dyn FnMut(WidgetId, RoutePhase, &Event, &mut EventSender) -> RouteOutcome + 'a>;

/// 事件路由器（D5 §3.1）。
///
/// 管理 widget 树结构，实现捕获→目标→冒泡三阶段事件路由。
pub struct EventRouter {
    /// widget 层级树。
    tree: WidgetTree,
}

impl EventRouter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tree: WidgetTree::new(),
        }
    }

    /// 获取对内部 WidgetTree 的不可变引用。
    #[must_use]
    pub fn tree(&self) -> &WidgetTree {
        &self.tree
    }

    /// 添加父子关系。
    pub fn add_child(&mut self, parent: WidgetId, child: WidgetId) {
        self.tree.add_child(parent, child);
    }

    /// 移除 widget 及其后代。
    pub fn remove(&mut self, widget_id: WidgetId) {
        self.tree.remove(widget_id);
    }

    /// 获取从根到目标的祖先链（根在前）。
    #[must_use]
    pub fn ancestors(&self, widget_id: WidgetId) -> Vec<WidgetId> {
        self.tree.path_to_root(widget_id)
    }

    /// 判断 widget 是否在路由器中。
    #[must_use]
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.tree.contains(widget_id)
    }

    /// 三阶段事件路由：捕获 → 目标 → 冒泡。
    ///
    /// 每个阶段调用 `handler(widget_id, phase, event, sender)`，
    /// 返回值或 `sender.consumed` 控制传播：
    /// - 返回 `Consumed` 或 `sender.consumed` 为 `true`：立即停止
    /// - 其他情况：继续
    pub fn route(
        &self,
        target: WidgetId,
        event: &Event,
        handler: &mut RouteCallback,
    ) -> RouteOutcome {
        let ancestors = self.tree.path_to_root(target);
        let mut sender = EventSender::new();

        // 阶段 1：捕获（根 → 目标之前，不含目标）
        for &id in ancestors.iter().take(ancestors.len().saturating_sub(1)) {
            if handler(id, RoutePhase::Capture, event, &mut sender) == RouteOutcome::Consumed
                || sender.consumed
            {
                return RouteOutcome::Consumed;
            }
        }

        // 阶段 2：目标
        if handler(target, RoutePhase::Target, event, &mut sender) == RouteOutcome::Consumed
            || sender.consumed
        {
            return RouteOutcome::Consumed;
        }

        // 阶段 3：冒泡（目标 → 根，不含目标）
        for &id in ancestors.iter().rev().skip(1) {
            if handler(id, RoutePhase::Bubble, event, &mut sender) == RouteOutcome::Consumed
                || sender.consumed
            {
                return RouteOutcome::Consumed;
            }
        }

        RouteOutcome::Unhandled
    }
}

impl Default for EventRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for EventRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventRouter")
            .field("widgets", &self.tree.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_tree() -> (EventRouter, WidgetId, WidgetId, WidgetId) {
        let mut r = EventRouter::new();
        let root = WidgetId::from_u64(1);
        let child = WidgetId::from_u64(2);
        let leaf = WidgetId::from_u64(3);
        r.add_child(root, child);
        r.add_child(child, leaf);
        (r, root, child, leaf)
    }

    #[test]
    fn add_child_works() {
        let mut r = EventRouter::new();
        let p = WidgetId::from_u64(1);
        let c = WidgetId::from_u64(2);
        r.add_child(p, c);
        assert!(r.tree().parent(c).is_some());
    }

    #[test]
    fn ancestors_root_to_leaf() {
        let (r, root, child, leaf) = build_tree();
        assert_eq!(r.ancestors(leaf), vec![root, child, leaf]);
    }

    #[test]
    fn remove_cascades() {
        let mut r = EventRouter::new();
        r.add_child(WidgetId::from_u64(1), WidgetId::from_u64(2));
        r.add_child(WidgetId::from_u64(2), WidgetId::from_u64(3));
        r.remove(WidgetId::from_u64(2));
        assert!(!r.contains(WidgetId::from_u64(2)));
        assert!(!r.contains(WidgetId::from_u64(3)));
    }

    #[test]
    fn route_capture_target_bubble_order() {
        let (r, root, child, leaf) = build_tree();
        let visited = std::cell::RefCell::new(Vec::new());
        let mut handler: RouteCallback = Box::new(|id, phase, _: &Event, _: &mut EventSender| {
            visited.borrow_mut().push((id, phase));
            RouteOutcome::Handled
        });
        let event = Event::WindowFocused;
        r.route(leaf, &event, &mut handler);
        assert_eq!(
            *visited.borrow(),
            vec![
                (root, RoutePhase::Capture),
                (child, RoutePhase::Capture),
                (leaf, RoutePhase::Target),
                (child, RoutePhase::Bubble),
                (root, RoutePhase::Bubble),
            ]
        );
    }

    #[test]
    fn route_consumed_stops_capture() {
        let (r, _root, child, leaf) = build_tree();
        let count = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let count_clone = count.clone();
        let mut handler: RouteCallback =
            Box::new(move |id, phase, _: &Event, _: &mut EventSender| {
                let c = count_clone.get() + 1;
                count_clone.set(c);
                if id == child && phase == RoutePhase::Capture {
                    RouteOutcome::Consumed
                } else {
                    RouteOutcome::Handled
                }
            });
        let event = Event::WindowFocused;
        r.route(leaf, &event, &mut handler);
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn route_consumed_stops_bubble() {
        let (r, _root, child, leaf) = build_tree();
        let count = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let count_clone = count.clone();
        let mut handler: RouteCallback =
            Box::new(move |id, phase, _: &Event, _: &mut EventSender| {
                let c = count_clone.get() + 1;
                count_clone.set(c);
                if id == child && phase == RoutePhase::Bubble {
                    RouteOutcome::Consumed
                } else {
                    RouteOutcome::Handled
                }
            });
        let event = Event::WindowFocused;
        r.route(leaf, &event, &mut handler);
        assert_eq!(count.get(), 4);
    }

    #[test]
    fn route_event_sender_consume_stops_propagation() {
        let (r, root, _child, leaf) = build_tree();
        let count = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let count_clone = count.clone();
        let mut handler: RouteCallback =
            Box::new(move |id, phase, _: &Event, sender: &mut EventSender| {
                let c = count_clone.get() + 1;
                count_clone.set(c);
                // 在捕获阶段的第一个 widget 消费事件
                if id == root && phase == RoutePhase::Capture {
                    sender.consume(); // 通过 EventSender 消费，而非返回值
                    return RouteOutcome::Handled;
                }
                RouteOutcome::Handled
            });
        let event = Event::WindowFocused;
        r.route(leaf, &event, &mut handler);
        // 只应在捕获阶段的 root 被调用一次
        assert_eq!(count.get(), 1);
    }
}
