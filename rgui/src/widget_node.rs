//! Widget 运行时节点——连接组件、状态、命中测试、diff 全链路。
//!
//! 集成 D1 §11.3 组件异常隔离：
//! - `view()` panic → 渲染错误占位符
//! - `handle()` panic → 捕获异常记录日志，不传播 panic
//! - `paint()` panic → 由场景构建器的 `catch_paint` 处理

use rgui_core::context::{UpdateContext, ViewContext};
use rgui_core::geometry::Rect;
use rgui_core::id::WidgetId;
use rgui_core::traits::AppMessage;
use rgui_core::view::WidgetView;
use rgui_state::Patch;
use rgui_state::diff::{WidgetIdMap, diff};
use std::fmt;
use std::panic::AssertUnwindSafe;

use crate::error_boundary::catch_view;

/// 事件处理器类型。
#[allow(clippy::type_complexity)]
pub type EventHandler = Box<dyn FnMut(&str, &mut UpdateContext) + Send>;
/// 视图生成器类型。
#[allow(clippy::type_complexity)]
pub type ViewBuilder<M> = Box<dyn Fn(&ViewContext) -> WidgetView<M> + Send>;

/// Widget 运行时节点。
///
/// 每个 WidgetNode 对应一个 UI 组件实例，持有：
/// - 状态（通过闭包读写）
/// - 边界矩形（用于命中测试）
/// - 事件处理器
/// - 视图生成器
///
/// # 异常隔离
///
/// `view()` 和 `handle()` 内部使用 `catch_unwind` 保护：
/// - `view()` panic → 返回错误占位符 `__rgui_error_placeholder__`
/// - `handle()` panic → 捕获并记录到 stderr，不传播
pub struct WidgetNode<M: AppMessage> {
    /// widget 唯一标识。
    pub id: WidgetId,
    /// 在窗口中的边界矩形。
    pub bounds: Rect,
    /// 事件处理器：接收消息名 → 更新状态。
    pub on_event: EventHandler,
    /// 视图生成器：根据当前状态生成 WidgetView。
    pub view_fn: ViewBuilder<M>,
    /// 上一次生成的视图（用于 diff）。
    pub prev_view: Option<WidgetView<M>>,
}

impl<M: AppMessage> WidgetNode<M> {
    /// 创建新的 WidgetNode。
    pub fn new(
        id: WidgetId,
        bounds: Rect,
        on_event: impl FnMut(&str, &mut UpdateContext) + Send + 'static,
        view_fn: impl Fn(&ViewContext) -> WidgetView<M> + Send + 'static,
    ) -> Self {
        Self {
            id,
            bounds,
            on_event: Box::new(on_event),
            view_fn: Box::new(view_fn),
            prev_view: None,
        }
    }

    /// 安全处理事件（调用 on_event 闭包，带异常隔离）。
    ///
    /// 如果 `on_event` 闭包 panic，异常被捕获并记录到 stderr，
    /// 不会导致应用崩溃。调用者负责管理状态回滚（框架不介入状态管理）。
    pub fn handle(&mut self, event: &str, ctx: &mut UpdateContext) {
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            (self.on_event)(event, ctx);
        }));
        if let Err(e) = result {
            let msg = if let Some(s) = e.downcast_ref::<&str>() {
                *s
            } else if let Some(s) = e.downcast_ref::<String>() {
                s.as_str()
            } else {
                "unknown panic"
            };
            eprintln!("[rgui] WidgetNode::handle panic (id={:?}): {msg}", self.id);
        }
    }

    /// 安全生成当前视图（带异常隔离）。
    ///
    /// 如果 `view_fn` 闭包 panic，返回错误占位符 WidgetView，
    /// 不会导致应用崩溃。
    pub fn view(&self, ctx: &ViewContext) -> WidgetView<M> {
        catch_view(AssertUnwindSafe(|| (self.view_fn)(ctx)))
    }

    /// 生成视图并 diff 上一次视图，返回 Patch 列表。
    ///
    /// 如果 `view_fn` panic，使用错误占位符进行 diff。
    pub fn diff_and_update(
        &mut self,
        ctx: &ViewContext,
        id_map: &mut WidgetIdMap,
    ) -> Vec<Patch<M>> {
        let new_view = self.view(ctx);
        let patches = if let Some(ref old) = self.prev_view {
            diff(old, &new_view, self.id, id_map)
        } else {
            Vec::new()
        };
        self.prev_view = Some(new_view);
        patches
    }

    /// 检查当前视图是否为错误占位符。
    #[must_use]
    pub fn is_error_view(&self) -> bool {
        self.prev_view
            .as_ref()
            .is_some_and(|v| v.widget_type == "__rgui_error_placeholder__")
    }
}

impl<M: AppMessage> fmt::Debug for WidgetNode<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WidgetNode")
            .field("id", &self.id)
            .field("bounds", &self.bounds)
            .field("is_error_view", &self.is_error_view())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestMessage;
    impl AppMessage for TestMessage {
        fn message_name(&self) -> &'static str {
            "TestMessage"
        }
    }

    fn make_ctx() -> UpdateContext {
        UpdateContext::new()
    }

    fn make_view_ctx() -> ViewContext {
        ViewContext::new(rgui_core::geometry::Size::new(100.0, 100.0))
    }

    // ========================================================================
    // WidgetNode::view 异常隔离
    // ========================================================================

    #[test]
    fn view_returns_normal_view_when_no_panic() {
        let node = WidgetNode::<TestMessage>::new(
            WidgetId::from_u64(1),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            |_, _| {},
            |_| WidgetView::new("TestWidget"),
        );
        let ctx = make_view_ctx();
        let view = node.view(&ctx);
        assert_eq!(view.widget_type, "TestWidget");
    }

    #[test]
    fn view_returns_error_placeholder_on_panic() {
        let node = WidgetNode::<TestMessage>::new(
            WidgetId::from_u64(1),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            |_, _| {},
            |_| panic!("view 崩溃"),
        );
        let ctx = make_view_ctx();
        let view = node.view(&ctx);
        assert_eq!(view.widget_type, "__rgui_error_placeholder__");
    }

    #[test]
    fn view_propagates_state_changes_outside() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = Arc::clone(&flag);
        let node = WidgetNode::<TestMessage>::new(
            WidgetId::from_u64(1),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            |_, _| {},
            move |_| {
                flag_clone.store(true, Ordering::SeqCst);
                WidgetView::new("TestWidget")
            },
        );
        let ctx = make_view_ctx();
        let _view = node.view(&ctx);
        assert!(flag.load(Ordering::SeqCst));
    }

    // ========================================================================
    // WidgetNode::handle 异常隔离
    // ========================================================================

    #[test]
    fn handle_catches_panic_without_crashing() {
        let mut node = WidgetNode::<TestMessage>::new(
            WidgetId::from_u64(1),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            |_, _| panic!("handle 崩溃"),
            |_| WidgetView::new("TestWidget"),
        );
        let mut ctx = make_ctx();
        // 不应 panic
        node.handle("test", &mut ctx);
    }

    #[test]
    fn handle_succeeds_normally() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = Arc::clone(&called);
        let mut node = WidgetNode::<TestMessage>::new(
            WidgetId::from_u64(1),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            move |_, _| {
                called_clone.store(true, Ordering::SeqCst);
            },
            |_| WidgetView::new("TestWidget"),
        );
        let mut ctx = make_ctx();
        node.handle("test", &mut ctx);
        assert!(called.load(Ordering::SeqCst));
    }

    // ========================================================================
    // WidgetNode::diff_and_update
    // ========================================================================

    #[test]
    fn diff_and_update_works_for_first_call() {
        let mut node = WidgetNode::<TestMessage>::new(
            WidgetId::from_u64(1),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            |_, _| {},
            |_| WidgetView::new("TestWidget"),
        );
        let ctx = make_view_ctx();
        let mut id_map = WidgetIdMap::new();
        let patches = node.diff_and_update(&ctx, &mut id_map);
        assert!(patches.is_empty());
    }

    #[test]
    fn diff_and_update_detects_no_change_for_same_view() {
        let mut node = WidgetNode::<TestMessage>::new(
            WidgetId::from_u64(1),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            |_, _| {},
            |_| WidgetView::new("TestWidget"),
        );
        let ctx = make_view_ctx();
        let mut id_map = WidgetIdMap::new();
        node.diff_and_update(&ctx, &mut id_map);
        let patches = node.diff_and_update(&ctx, &mut id_map);
        // 两次相同视图 → 无 patch
        assert!(patches.is_empty());
    }

    // ========================================================================
    // WidgetNode::is_error_view
    // ========================================================================

    #[test]
    fn is_error_view_returns_false_for_normal_view() {
        let mut node = WidgetNode::<TestMessage>::new(
            WidgetId::from_u64(1),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            |_, _| {},
            |_| WidgetView::new("TestWidget"),
        );
        let ctx = make_view_ctx();
        let mut id_map = WidgetIdMap::new();
        node.diff_and_update(&ctx, &mut id_map);
        assert!(!node.is_error_view());
    }

    #[test]
    fn is_error_view_returns_true_after_view_panic() {
        let mut node = WidgetNode::<TestMessage>::new(
            WidgetId::from_u64(1),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            |_, _| {},
            |_| panic!("view 崩溃"),
        );
        let ctx = make_view_ctx();
        let mut id_map = WidgetIdMap::new();
        node.diff_and_update(&ctx, &mut id_map);
        assert!(node.is_error_view());
    }
}
