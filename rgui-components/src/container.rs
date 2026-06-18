//! Container 组件——通用布局容器。
//!
//! 无视觉外观；在 `view()` 中将 State 中的布局属性映射为
//! `WidgetView.props` 中的 CSS 属性键值对。布局由 `rgui-layout`/Taffy 驱动。

use rgui_core::a11y::{AccessibilityNode, AccessibilityRole};
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage, PersistState, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

/// Container 业务状态。
///
/// 所有字段均为 `Option`——`None` 表示该属性未被设置，
/// 布局引擎将使用默认值。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct ContainerState {
    /// CSS display（flex / grid / block / none）。默认 flex。
    pub display: Option<String>,
    /// 宽度（逻辑像素）。
    pub width: Option<f64>,
    /// 高度（逻辑像素）。
    pub height: Option<f64>,
    /// Flex 主轴方向（row / row-reverse / column / column-reverse）。
    pub flex_direction: Option<String>,
    /// 主轴对齐方式（flex-start / center / flex-end / space-between / space-around / space-evenly）。
    pub justify_content: Option<String>,
    /// 交叉轴对齐方式（flex-start / center / flex-end / baseline / stretch）。
    pub align_items: Option<String>,
    /// 交叉轴内容分布（flex-start / center / flex-end / space-between / space-around / space-evenly / stretch）。
    pub align_content: Option<String>,
    /// 子元素间距（逻辑像素）。
    pub gap: Option<f64>,
    /// 内边距（四边相同，逻辑像素）。
    pub padding: Option<f64>,
    /// 外边距（四边相同，逻辑像素）。
    pub margin: Option<f64>,
    /// Flex 换行模式（nowrap / wrap / wrap-reverse）。
    pub flex_wrap: Option<String>,
}

/// Container 消息类型（占位）。
///
/// Container 本身不产生交互消息，提供此枚举以满足 `WidgetSpec` 关联类型约束。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum ContainerMessage {
    NoOp,
}

/// Container 组件（unit struct）。
///
/// 无视觉外观的通用布局容器。实现 [`WidgetSpec`] trait。
pub struct Container;

impl WidgetSpec for Container {
    type State = ContainerState;
    type Message = ContainerMessage;

    fn name(&self) -> &'static str {
        "rgui_components::Container"
    }

    fn view(&self, s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let mut view = WidgetView::new("rgui_components::Container");

        if let Some(ref d) = s.display {
            view = view.prop("display", PropValue::str(d.as_str()));
        }
        if let Some(w) = s.width {
            view = view.prop("width", w);
        }
        if let Some(h) = s.height {
            view = view.prop("height", h);
        }
        if let Some(ref fd) = s.flex_direction {
            view = view.prop("flex-direction", PropValue::str(fd.as_str()));
        }
        if let Some(ref jc) = s.justify_content {
            view = view.prop("justify-content", PropValue::str(jc.as_str()));
        }
        if let Some(ref ai) = s.align_items {
            view = view.prop("align-items", PropValue::str(ai.as_str()));
        }
        if let Some(ref ac) = s.align_content {
            view = view.prop("align-content", PropValue::str(ac.as_str()));
        }
        if let Some(g) = s.gap {
            view = view.prop("gap", g);
        }
        if let Some(p) = s.padding {
            view = view.prop("padding", p);
        }
        if let Some(m) = s.margin {
            view = view.prop("margin", m);
        }
        if let Some(ref fw) = s.flex_wrap {
            view = view.prop("flex-wrap", PropValue::str(fw.as_str()));
        }

        view
    }

    fn update(&self, _: Self::Message, _: &mut Self::State, _: &mut UpdateContext) {}

    fn paint(&self, _: &Self::State, _: Rect, _: &mut PaintContext) {}

    fn accessibility(&self, _: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name() {
        assert_eq!(Container.name(), "rgui_components::Container");
    }

    #[test]
    fn view_empty_state_has_no_props() {
        let state = ContainerState::default();
        let view = Container.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(view.props.is_empty());
        assert_eq!(view.widget_type, "rgui_components::Container");
    }

    #[test]
    fn view_sets_display() {
        let state = ContainerState {
            display: Some("flex".into()),
            ..Default::default()
        };
        let view = Container.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("display"), Some(&PropValue::str("flex")));
    }

    #[test]
    fn view_sets_flex_direction_column() {
        let state = ContainerState {
            flex_direction: Some("column".into()),
            ..Default::default()
        };
        let view = Container.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            view.props.get("flex-direction"),
            Some(&PropValue::str("column"))
        );
    }

    #[test]
    fn view_sets_gap() {
        let state = ContainerState {
            gap: Some(8.0),
            ..Default::default()
        };
        let view = Container.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("gap"), Some(&PropValue::from(8.0)));
    }

    #[test]
    fn view_sets_padding() {
        let state = ContainerState {
            padding: Some(16.0),
            ..Default::default()
        };
        let view = Container.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("padding"), Some(&PropValue::from(16.0)));
    }

    #[test]
    fn view_sets_justify_content_center() {
        let state = ContainerState {
            justify_content: Some("center".into()),
            ..Default::default()
        };
        let view = Container.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            view.props.get("justify-content"),
            Some(&PropValue::str("center"))
        );
    }

    #[test]
    fn view_sets_align_items_stretch() {
        let state = ContainerState {
            align_items: Some("stretch".into()),
            ..Default::default()
        };
        let view = Container.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            view.props.get("align-items"),
            Some(&PropValue::str("stretch"))
        );
    }

    #[test]
    fn view_sets_width_and_height() {
        let state = ContainerState {
            width: Some(300.0),
            height: Some(200.0),
            ..Default::default()
        };
        let view = Container.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("width"), Some(&PropValue::from(300.0)));
        assert_eq!(view.props.get("height"), Some(&PropValue::from(200.0)));
    }

    #[test]
    fn view_sets_margin() {
        let state = ContainerState {
            margin: Some(4.0),
            ..Default::default()
        };
        let view = Container.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("margin"), Some(&PropValue::from(4.0)));
    }

    #[test]
    fn view_sets_flex_wrap() {
        let state = ContainerState {
            flex_wrap: Some("wrap".into()),
            ..Default::default()
        };
        let view = Container.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("flex-wrap"), Some(&PropValue::str("wrap")));
    }

    #[test]
    fn view_sets_align_content() {
        let state = ContainerState {
            align_content: Some("center".into()),
            ..Default::default()
        };
        let view = Container.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            view.props.get("align-content"),
            Some(&PropValue::str("center"))
        );
    }

    #[test]
    fn view_full_flex_row_config() {
        let state = ContainerState {
            display: Some("flex".into()),
            flex_direction: Some("row".into()),
            gap: Some(8.0),
            justify_content: Some("flex-start".into()),
            align_items: Some("center".into()),
            ..Default::default()
        };
        let view = Container.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("display"), Some(&PropValue::str("flex")));
        assert_eq!(
            view.props.get("flex-direction"),
            Some(&PropValue::str("row"))
        );
        assert_eq!(view.props.get("gap"), Some(&PropValue::from(8.0)));
        assert_eq!(
            view.props.get("justify-content"),
            Some(&PropValue::str("flex-start"))
        );
        assert_eq!(
            view.props.get("align-items"),
            Some(&PropValue::str("center"))
        );
    }

    #[test]
    fn update_is_noop() {
        let mut state = ContainerState::default();
        let mut ctx = UpdateContext::default();
        Container.update(ContainerMessage::NoOp, &mut state, &mut ctx);
        // State should remain unchanged
        assert!(state.display.is_none());
    }

    #[test]
    fn paint_is_empty() {
        let state = ContainerState::default();
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        Container.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0);
    }

    #[test]
    fn measure_returns_zero() {
        let state = ContainerState::default();
        let ctx = MeasureContext::default();
        let size = Container.measure(&state, BoxConstraints::UNCONSTRAINED, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn derive_state_schema_name() {
        assert_eq!(ContainerState::schema_name(), "ContainerState");
    }

    #[test]
    fn derive_message_name() {
        assert_eq!(ContainerMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn accessibility_returns_none() {
        let state = ContainerState::default();
        let ctx = AccessContext::new(Rect::new(0.0, 0.0, 800.0, 600.0));
        let node = Container.accessibility(&state, &ctx);
        assert_eq!(node.role, AccessibilityRole::None);
    }
}
