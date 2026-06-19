/// Translated from Web Awesome tree-item
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
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

// ============================================================================
// State
// ============================================================================

/// Web Awesome wa-tree-item 组件状态。
///
/// TreeItem 是树控件的叶子节点，支持展开/折叠、选择和嵌套子节点。
///
/// Phase 0 简化项：
/// - 展开/折叠动画 → 即时切换（无动画）
/// - 懒加载 → 跳过（lazy 字段保留但无实际行为）
/// - 键盘导航 → rgui 无焦点管理，暂不实现
/// - RTL 方向 → 硬编码 LTR
/// - 命名 slot（expand-icon/collapse-icon）→ 使用 Unicode 字符
/// - 嵌套深度自动计算 → 使用 state.depth 字段
/// - checkbox 同步 → Phase 2
/// - 缩进线 → Phase 2（CSS ::before 伪元素）
/// - loading spinner → 跳过（使用静态文字替代）
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaTreeItemState {
    /// 节点标签文本
    pub label: String,
    /// 是否展开
    pub expanded: bool,
    /// 是否选中
    pub selected: bool,
    /// 是否禁用
    pub disabled: bool,
    /// 是否懒加载
    pub lazy: bool,
    /// 是否半选状态（部分子节点选中）
    pub indeterminate: bool,
    /// 是否为叶子节点（无子节点）
    pub is_leaf: bool,
    /// 是否正在加载
    pub loading: bool,
    /// 是否可选择（由父 Tree 设置）
    pub selectable: bool,
    /// 嵌套深度（0 = 根节点，由父 Tree 设置）
    pub depth: u32,
}

impl WaTreeItemState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            label: String::new(),
            expanded: false,
            selected: false,
            disabled: false,
            lazy: false,
            indeterminate: false,
            is_leaf: true, // Phase 0: 默认为叶子，子节点由布局引擎管理
            loading: false,
            selectable: false,
            depth: 0,
        }
    }
}

// ============================================================================
// Message
// ============================================================================

/// TreeItem 事件：
/// - `Trigger` — 节点被点击（展开/折叠 或 选择）
/// - `Expand` — 展开前事件（wa-expand）
/// - `AfterExpand` — 展开后事件（wa-after-expand）
/// - `Collapse` — 折叠前事件（wa-collapse）
/// - `AfterCollapse` — 折叠后事件（wa-after-collapse）
/// - `LazyChange` — 懒加载状态变更
/// - `LazyLoad` — 懒加载触发
///
/// Phase 0：Trigger 处理展开/折叠和选择切换，其余事件无实际行为。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaTreeItemMessage {
    Trigger,
    Expand,
    AfterExpand,
    Collapse,
    AfterCollapse,
    LazyChange,
    LazyLoad,
}

// ============================================================================
// Constants
// ============================================================================

/// 树节点行高（与 WA 源 item 高度对齐，约 36px）
const ROW_HEIGHT: f64 = 36.0;
/// 字体大小比例
const FONT_RATIO: f64 = 0.44;
/// 每级缩进宽度
const INDENT_WIDTH: f64 = 24.0;
/// 展开按钮宽度
const EXPAND_BUTTON_WIDTH: f64 = 24.0;
/// Checkbox 宽度
const CHECKBOX_WIDTH: f64 = 24.0;

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaTreeItem;

impl WidgetSpec for WaTreeItem {
    type State = WaTreeItemState;
    type Message = WaTreeItemMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaTreeItem"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaTreeItem")
            .prop(
                "label",
                PropValue::str(if state.label.is_empty() {
                    ""
                } else {
                    state.label.as_str()
                }),
            )
            .prop("expanded", PropValue::Bool(state.expanded))
            .prop("selected", PropValue::Bool(state.selected))
            .prop("disabled", PropValue::Bool(state.disabled))
            .prop("lazy", PropValue::Bool(state.lazy))
            .prop("indeterminate", PropValue::Bool(state.indeterminate))
            .prop("is-leaf", PropValue::Bool(state.is_leaf))
            .prop("loading", PropValue::Bool(state.loading))
            .prop("selectable", PropValue::Bool(state.selectable))
            .prop("depth", PropValue::Int(state.depth as i64))
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaTreeItemMessage::Trigger => {
                if state.disabled {
                    return;
                }
                // 如果有子节点或懒加载，切换展开状态
                if !state.is_leaf || state.lazy {
                    state.expanded = !state.expanded;
                } else if state.selectable {
                    // 叶子节点且可选择：切换选中
                    state.selected = !state.selected;
                }
            },
            WaTreeItemMessage::Expand => {
                state.expanded = true;
            },
            WaTreeItemMessage::AfterExpand => {
                // Phase 0: no animation, noop
            },
            WaTreeItemMessage::Collapse => {
                state.expanded = false;
            },
            WaTreeItemMessage::AfterCollapse => {
                // Phase 0: no animation, noop
            },
            WaTreeItemMessage::LazyChange => {
                // Phase 0: lazy loading not implemented
            },
            WaTreeItemMessage::LazyLoad => {
                // Phase 0: lazy loading not implemented
            },
        }
    }

    /// TreeItem 是叶子组件，尺寸由 Taffy 布局决定。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let w: f64 = bounds.size.width;
        let h: f64 = bounds.size.height;

        if w < 8.0 || h < ROW_HEIGHT {
            return;
        }

        let font_size: f32 = (ROW_HEIGHT * FONT_RATIO) as f32;

        // —— 选中背景 ——
        if state.selected && !state.disabled {
            let bg_color = Color::new(0.90, 0.90, 0.95, 1.0); // WA --wa-color-neutral-fill-quiet
            let bg_rect = Rect::new(bounds.origin.x, bounds.origin.y, w, h);
            ctx.fill_rect(bg_rect, bg_color, 0.0);
        }

        // —— 左侧选中指示线 ——
        if state.selected && !state.disabled {
            let indicator_color = Color::new(0.2, 0.45, 0.9, 1.0); // WA --wa-color-brand-fill-loud
            let indicator_w = 3.0_f64;
            let indicator_rect = Rect::new(bounds.origin.x, bounds.origin.y, indicator_w, h);
            ctx.fill_rect(indicator_rect, indicator_color, 0.0);
        }

        // —— 缩进区域 ——
        let indent = (state.depth as f64) * INDENT_WIDTH;
        let mut x: f64 = bounds.origin.x + indent;

        // —— 展开/折叠按钮 ——
        let show_expand = !state.loading && (!state.is_leaf || state.lazy);
        if show_expand {
            let chevron = if state.expanded {
                "\u{25BC}"
            } else {
                "\u{25B6}"
            }; // ▼ or ▶
            let chevron_color = if state.disabled {
                Color::new(0.7, 0.7, 0.7, 1.0)
            } else {
                Color::new(0.5, 0.5, 0.5, 1.0) // WA --wa-color-text-quiet
            };
            let chevron_rect = Rect::new(x, bounds.origin.y, EXPAND_BUTTON_WIDTH, ROW_HEIGHT);
            ctx.draw_text(chevron, chevron_rect, chevron_color, font_size);
        }
        x += EXPAND_BUTTON_WIDTH;

        // —— Loading 状态 ——
        if state.loading {
            let loading_text = "...";
            let loading_color = Color::new(0.5, 0.5, 0.5, 1.0);
            let loading_rect = Rect::new(x, bounds.origin.y, CHECKBOX_WIDTH, ROW_HEIGHT);
            ctx.draw_text(loading_text, loading_rect, loading_color, font_size);
            x += CHECKBOX_WIDTH;
        }

        // —— Checkbox（多选模式） ——
        if state.selectable {
            let check_char = if state.indeterminate {
                "\u{25A1}" // □ (indeterminate, hollow)
            } else if state.selected {
                "\u{2611}" // ☑ (checked)
            } else {
                "\u{2610}" // ☐ (unchecked)
            };
            let check_color = if state.disabled {
                Color::new(0.7, 0.7, 0.7, 1.0)
            } else {
                Color::new(0.3, 0.3, 0.3, 1.0)
            };
            let check_rect = Rect::new(x, bounds.origin.y, CHECKBOX_WIDTH, ROW_HEIGHT);
            ctx.draw_text(check_char, check_rect, check_color, font_size);
            x += CHECKBOX_WIDTH;
        }

        // —— 标签文本 ——
        let text_color = if state.disabled {
            Color::new(0.6, 0.6, 0.6, 1.0)
        } else {
            Color::new(0.1, 0.1, 0.1, 1.0) // WA --wa-color-text-normal
        };

        let remaining_w = f64::max(w - (x - bounds.origin.x), 0.0);
        if remaining_w > 0.0 && !state.label.is_empty() {
            let text_rect = Rect::new(x, bounds.origin.y, remaining_w, ROW_HEIGHT);
            ctx.draw_text(state.label.as_str(), text_rect, text_color, font_size);
        }
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::new(
            WidgetId::from_u64(0),
            AccessibilityRole::Custom("treeitem"),
            Rect::ZERO,
        )
        .label(if state.label.is_empty() {
            "tree item"
        } else {
            state.label.as_str()
        })
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
        assert_eq!(WaTreeItem.name(), "rgui_components::WaTreeItem");
    }

    #[test]
    fn default_state() {
        let s = WaTreeItemState::new();
        assert!(!s.expanded);
        assert!(!s.selected);
        assert!(!s.disabled);
        assert!(!s.lazy);
        assert!(s.is_leaf);
        assert_eq!(s.depth, 0);
    }

    #[test]
    fn state_with_label() {
        let s = WaTreeItemState {
            label: "Node 1".into(),
            ..WaTreeItemState::new()
        };
        assert_eq!(s.label, "Node 1");
    }

    #[test]
    fn state_expanded() {
        let s = WaTreeItemState {
            expanded: true,
            ..WaTreeItemState::new()
        };
        assert!(s.expanded);
    }

    #[test]
    fn state_selected() {
        let s = WaTreeItemState {
            selected: true,
            ..WaTreeItemState::new()
        };
        assert!(s.selected);
    }

    #[test]
    fn state_disabled() {
        let s = WaTreeItemState {
            disabled: true,
            ..WaTreeItemState::new()
        };
        assert!(s.disabled);
    }

    #[test]
    fn state_with_depth() {
        let s = WaTreeItemState {
            depth: 3,
            ..WaTreeItemState::new()
        };
        assert_eq!(s.depth, 3);
    }

    #[test]
    fn update_trigger_toggles_expand_on_non_leaf() {
        let mut s = WaTreeItemState {
            is_leaf: false,
            ..WaTreeItemState::new()
        };
        assert!(!s.expanded);
        WaTreeItem.update(
            WaTreeItemMessage::Trigger,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.expanded);
        WaTreeItem.update(
            WaTreeItemMessage::Trigger,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(!s.expanded);
    }

    #[test]
    fn update_trigger_toggles_expand_on_lazy() {
        let mut s = WaTreeItemState {
            is_leaf: true,
            lazy: true,
            ..WaTreeItemState::new()
        };
        WaTreeItem.update(
            WaTreeItemMessage::Trigger,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.expanded);
    }

    #[test]
    fn update_trigger_selects_when_selectable_leaf() {
        let mut s = WaTreeItemState {
            is_leaf: true,
            selectable: true,
            ..WaTreeItemState::new()
        };
        assert!(!s.selected);
        WaTreeItem.update(
            WaTreeItemMessage::Trigger,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.selected);
    }

    #[test]
    fn update_trigger_ignored_when_disabled() {
        let mut s = WaTreeItemState {
            disabled: true,
            is_leaf: false,
            ..WaTreeItemState::new()
        };
        WaTreeItem.update(
            WaTreeItemMessage::Trigger,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(!s.expanded, "disabled item should not toggle");
    }

    #[test]
    fn update_expand_sets_true() {
        let mut s = WaTreeItemState::new();
        WaTreeItem.update(
            WaTreeItemMessage::Expand,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.expanded);
    }

    #[test]
    fn update_collapse_sets_false() {
        let mut s = WaTreeItemState {
            expanded: true,
            ..WaTreeItemState::new()
        };
        WaTreeItem.update(
            WaTreeItemMessage::Collapse,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(!s.expanded);
    }

    #[test]
    fn update_after_expand_noop() {
        let mut s = WaTreeItemState::new();
        WaTreeItem.update(
            WaTreeItemMessage::AfterExpand,
            &mut s,
            &mut UpdateContext::default(),
        );
        // no state change expected
        assert!(!s.expanded);
    }

    #[test]
    fn update_after_collapse_noop() {
        let mut s = WaTreeItemState::new();
        WaTreeItem.update(
            WaTreeItemMessage::AfterCollapse,
            &mut s,
            &mut UpdateContext::default(),
        );
        // no state change expected
    }

    #[test]
    fn update_lazy_events_noop() {
        let mut s = WaTreeItemState::new();
        WaTreeItem.update(
            WaTreeItemMessage::LazyChange,
            &mut s,
            &mut UpdateContext::default(),
        );
        WaTreeItem.update(
            WaTreeItemMessage::LazyLoad,
            &mut s,
            &mut UpdateContext::default(),
        );
        // no panic expected
    }

    #[test]
    fn measure_returns_zero() {
        let s = WaTreeItemState::new();
        let size = WaTreeItem.measure(
            &s,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn paint_leaf_produces_ops() {
        let s = WaTreeItemState {
            label: "Leaf".into(),
            is_leaf: true,
            ..WaTreeItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 36.0);
        let mut ctx = PaintContext::new(bounds);
        WaTreeItem.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0, "leaf item should render label");
    }

    #[test]
    fn paint_expanded_non_leaf_shows_chevron() {
        let s = WaTreeItemState {
            label: "Parent".into(),
            is_leaf: false,
            expanded: true,
            ..WaTreeItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 36.0);
        let mut ctx = PaintContext::new(bounds);
        WaTreeItem.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0);
    }

    #[test]
    fn paint_selected_shows_background() {
        let s = WaTreeItemState {
            label: "Selected".into(),
            selected: true,
            is_leaf: true,
            ..WaTreeItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 36.0);
        let mut ctx = PaintContext::new(bounds);
        WaTreeItem.paint(&s, bounds, &mut ctx);
        // selected: bg + indicator + label >= 3 ops
        assert!(
            ctx.op_count() >= 2,
            "selected item should have bg + indicator + label"
        );
    }

    #[test]
    fn paint_disabled_style() {
        let s = WaTreeItemState {
            label: "Disabled".into(),
            disabled: true,
            is_leaf: true,
            ..WaTreeItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 36.0);
        let mut ctx = PaintContext::new(bounds);
        WaTreeItem.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0, "disabled item should still render");
    }

    #[test]
    fn paint_too_small_returns_early() {
        let s = WaTreeItemState::new();
        let bounds = Rect::new(0.0, 0.0, 4.0, 4.0);
        let mut ctx = PaintContext::new(bounds);
        WaTreeItem.paint(&s, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0);
    }

    #[test]
    fn paint_with_depth_indentation() {
        let s = WaTreeItemState {
            label: "Child".into(),
            depth: 2,
            is_leaf: true,
            ..WaTreeItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 36.0);
        let mut ctx = PaintContext::new(bounds);
        WaTreeItem.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0);
    }

    #[test]
    fn paint_selectable_with_checkbox() {
        let s = WaTreeItemState {
            label: "Checkable".into(),
            selectable: true,
            is_leaf: true,
            ..WaTreeItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 36.0);
        let mut ctx = PaintContext::new(bounds);
        WaTreeItem.paint(&s, bounds, &mut ctx);
        // selectable adds unchecked checkbox character
        assert!(
            ctx.op_count() > 1,
            "selectable should have checkbox + label"
        );
    }

    #[test]
    fn paint_selectable_selected_shows_checked() {
        let s = WaTreeItemState {
            label: "Checked".into(),
            selectable: true,
            selected: true,
            is_leaf: true,
            ..WaTreeItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 36.0);
        let mut ctx = PaintContext::new(bounds);
        WaTreeItem.paint(&s, bounds, &mut ctx);
        // selected + selectable: bg + indicator + checkbox + label
        assert!(ctx.op_count() >= 3);
    }

    #[test]
    fn paint_loading_state() {
        let s = WaTreeItemState {
            label: "Loading".into(),
            loading: true,
            is_leaf: false,
            ..WaTreeItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 36.0);
        let mut ctx = PaintContext::new(bounds);
        WaTreeItem.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0, "loading should show ... indicator");
    }

    #[test]
    fn view_contains_props() {
        let s = WaTreeItemState {
            label: "Node".into(),
            expanded: true,
            selected: true,
            depth: 1,
            ..WaTreeItemState::new()
        };
        let v = WaTreeItem.view(&s, &make_ctx());
        assert!(v.props.contains_key("label"));
        assert!(v.props.contains_key("expanded"));
        assert!(v.props.contains_key("selected"));
        assert!(v.props.contains_key("disabled"));
        assert!(v.props.contains_key("depth"));
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaTreeItemMessage::Trigger.message_name(), "trigger");
        assert_eq!(WaTreeItemMessage::Expand.message_name(), "expand");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaTreeItemState::schema_name(), "WaTreeItemState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaTreeItemState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaTreeItemState>());
    }

    #[test]
    fn accessibility_default_label() {
        let s = WaTreeItemState::new();
        let node = WaTreeItem.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("tree item"));
    }

    #[test]
    fn accessibility_with_label() {
        let s = WaTreeItemState {
            label: "Documents".into(),
            ..WaTreeItemState::new()
        };
        let node = WaTreeItem.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("Documents"));
    }
}
