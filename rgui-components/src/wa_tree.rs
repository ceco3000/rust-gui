/// Translated from Web Awesome tree
/// Original license: MIT
/// Copyright (c) Font Awesome
use rgui_core::WidgetId;
use rgui_core::a11y::{AccessibilityNode, AccessibilityRole};
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::WidgetSpec;
// 以下 traits 由派生宏实现，test 中需要 in scope
#[allow(unused_imports)]
use rgui_core::traits::{AppMessage, PersistState};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

// ============================================================================
// State
// ============================================================================

/// Web Awesome wa-tree 组件状态。
///
/// Tree 是树形控件容器，管理多个 `<wa-tree-item>` 子节点。
/// 通过 `<slot>` 渲染子节点，自身无直接视觉绘制（缩进线在 TreeItem 子组件中渲染）。
///
/// Phase 0 简化项：
/// - 选择协调逻辑（selectItem）→ 仅存 selection 字段，不做强制协调
/// - 键盘导航（ArrowDown/Up/Right/Left/Home/End/Enter/Space）→ rgui 无焦点路由
/// - 焦点管理（lastFocusedItem/tabIndex）→ 跳过
/// - Checkbox 同步（syncCheckboxes）→ 跳过
/// - MutationObserver（handleTreeChanged）→ rgui 无 DOM 变更事件
/// - 点击目标跟踪（clickTarget/mouseDown）→ 跳过
/// - expand-icon/collapse-icon 命名 slot 克隆 → 跳过
/// - initTreeItem（设置子节点 selectable + 图标）→ paint_factory 中透传
/// - focusIn/focusOut 事件处理 → 跳过
/// - 懒加载 slot change → 跳过
/// - RTL 方向处理 → 硬编码 LTR
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaTreeState {
    /// 选择模式：single | multiple | leaf | leaf-multiple
    pub selection: String,
}

impl WaTreeState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            selection: "single".into(),
        }
    }
}

// ============================================================================
// Message
// ============================================================================

/// Tree 事件：
/// - `SelectionChange` — 选择变更事件（wa-selection-change）
///
/// Phase 0：事件无实际行为，保留占位供未来实现。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaTreeMessage {
    SelectionChange,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaTree;

impl WidgetSpec for WaTree {
    type State = WaTreeState;
    type Message = WaTreeMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaTree"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaTree")
            .prop("selection", PropValue::str(state.selection.as_str()))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaTreeMessage::SelectionChange => {
                // Phase 0: no coordination logic between items yet
            },
        }
    }

    /// Tree 是容器，尺寸由 Taffy 根据子节点和约束计算。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, _state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        // Tree 是纯容器，无自身视觉绘制。
        // 子节点（WaTreeItem）各自渲染自己的行内容（缩进、展开按钮、checkbox、标签）。
        // Phase 2 可添加容器级边框/背景。
        let _ = (bounds, ctx);
    }

    fn accessibility(&self, _state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::new(
            WidgetId::from_u64(0),
            AccessibilityRole::Custom("tree"),
            Rect::ZERO,
        )
        .label("tree")
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;

    fn make_ctx() -> ViewContext {
        ViewContext::new(Size::new(800.0, 600.0))
    }

    #[test]
    fn name() {
        assert_eq!(WaTree.name(), "rgui_components::WaTree");
    }

    #[test]
    fn default_state() {
        let s = WaTreeState::new();
        assert_eq!(s.selection, "single");
    }

    #[test]
    fn state_multiple() {
        let s = WaTreeState {
            selection: "multiple".into(),
        };
        assert_eq!(s.selection, "multiple");
    }

    #[test]
    fn state_leaf() {
        let s = WaTreeState {
            selection: "leaf".into(),
        };
        assert_eq!(s.selection, "leaf");
    }

    #[test]
    fn state_leaf_multiple() {
        let s = WaTreeState {
            selection: "leaf-multiple".into(),
        };
        assert_eq!(s.selection, "leaf-multiple");
    }

    #[test]
    fn update_selection_change_noop() {
        let mut s = WaTreeState::new();
        WaTree.update(
            WaTreeMessage::SelectionChange,
            &mut s,
            &mut UpdateContext::default(),
        );
        // 不应 panic，状态不应改变
        assert_eq!(s.selection, "single");
    }

    #[test]
    fn measure_returns_zero() {
        let s = WaTreeState::new();
        let size = WaTree.measure(
            &s,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO, "Tree 容器委托 Taffy 计算尺寸");
    }

    #[test]
    fn paint_produces_no_ops() {
        let s = WaTreeState::new();
        let bounds = Rect::new(0.0, 0.0, 400.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaTree.paint(&s, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "Tree 无自身视觉绘制");
    }

    #[test]
    fn view_contains_selection_prop() {
        let s = WaTreeState::new();
        let v = WaTree.view(&s, &make_ctx());
        assert!(v.props.contains_key("selection"));
    }

    #[test]
    fn view_selection_prop_value() {
        let s = WaTreeState {
            selection: "multiple".into(),
        };
        let v = WaTree.view(&s, &make_ctx());
        let val = v.props.get("selection").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "multiple"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn derive_msg() {
        assert_eq!(
            WaTreeMessage::SelectionChange.message_name(),
            "selection_change"
        );
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaTreeState::schema_name(), "WaTreeState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaTreeState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaTreeState>());
    }

    #[test]
    fn accessibility_label() {
        let s = WaTreeState::new();
        let node = WaTree.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("tree"));
    }
}
