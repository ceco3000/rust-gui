//! Widget 运行时节点——连接组件、状态、命中测试、diff 全链路。

use rgui_core::context::{UpdateContext, ViewContext};
use rgui_core::geometry::Rect;
use rgui_core::id::WidgetId;
use rgui_core::traits::AppMessage;
use rgui_core::view::WidgetView;
use rgui_state::Patch;
use rgui_state::diff::{WidgetIdMap, diff};
use std::fmt;

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

    /// 处理事件（调用 on_event 闭包）。
    pub fn handle(&mut self, event: &str, ctx: &mut UpdateContext) {
        (self.on_event)(event, ctx);
    }

    /// 生成当前视图。
    pub fn view(&self, ctx: &ViewContext) -> WidgetView<M> {
        (self.view_fn)(ctx)
    }

    /// 生成视图并 diff 上一次视图，返回 Patch 列表。
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
}

impl<M: AppMessage> fmt::Debug for WidgetNode<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WidgetNode")
            .field("id", &self.id)
            .field("bounds", &self.bounds)
            .finish()
    }
}
