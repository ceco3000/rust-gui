//! PaintFn 工厂——根据 WidgetView.props 分发到对应组件的 paint() 方法。
//!
//! 组件通过 `wa-translate` 技能从 Web Awesome (MIT) 手工翻译加入
//! `rgui-components`。翻译后在此添加对应的 match 分支。
//!
//! 布局容器（Container/Row/Column 等）自身不绘制，
//! 子节点的 paint 结果由 walk_view_tree 递归收集。

use rgui_core::context::PaintOp;
use rgui_core::geometry::Rect;
use rgui_core::traits::AppMessage;
use rgui_render::PaintFn;

/// 创建默认的 PaintFn。
///
/// 当前无已翻译组件，所有类型返回空 Vec。
/// WA 翻译组件后，在此添加 `"WaButton" => WaButton::paint(...)` 等分支。
#[must_use]
pub fn default_paint_fn<M: AppMessage>() -> PaintFn<M> {
    Box::new(
        move |view: &rgui_core::view::WidgetView<M>, _bounds: Rect| -> Vec<PaintOp> {
            let _ = view; // 占位——翻译组件后使用
            Vec::new()
        },
    )
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rgui_core::geometry::Rect;
    use rgui_core::view::{PropValue, WidgetView};

    #[derive(Debug, Clone, PartialEq)]
    enum TestMsg {
        Dummy,
    }

    impl AppMessage for TestMsg {
        fn message_name(&self) -> &'static str {
            "dummy"
        }
    }

    #[test]
    fn default_paint_fn_creates_valid_paint_fn() {
        let _paint_fn = default_paint_fn::<TestMsg>();
    }

    #[test]
    fn unknown_type_returns_empty() {
        let paint_fn = default_paint_fn::<TestMsg>();
        let view = WidgetView::<TestMsg>::new("UnknownWidget");
        let ops = paint_fn(&view, Rect::new(0.0, 0.0, 100.0, 40.0));
        assert!(ops.is_empty());
    }
}
