//! 无障碍基础类型——AccessibilityNode、AccessibilityRole、AccessibilityAction、AccessibilityState。
//!
//! 定义源自 D0 §4.1 和 D6 无障碍系统设计。
//! `rgui-core` 仅定义类型，平台桥接由 `rgui-a11y` 负责。

use crate::geometry::Rect;
use crate::id::WidgetId;
use std::fmt;
use std::sync::Arc;

// ============================================================================
// AccessibilityRole
// ============================================================================

/// 无障碍角色，描述 UI 元素的语义类型。
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum AccessibilityRole {
    None,
    Button,
    Link,
    TextInput,
    CheckBox,
    RadioButton,
    Toggle,
    ComboBox,
    List,
    ListItem,
    Table,
    TableRow,
    TableCell,
    ProgressBar,
    Slider,
    TabList,
    Tab,
    TabPanel,
    Image,
    Heading,
    Group,
    Dialog,
    Alert,
    Menu,
    MenuItem,
    ScrollBar,
    Separator,
    Tooltip,
    Tree,
    TreeItem,
    SearchBox,
    Custom(&'static str),
}

// ============================================================================
// AccessibilityAction
// ============================================================================

/// 无障碍动作，表示用户可对元素执行的操作。
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum AccessibilityAction {
    Click,
    Toggle,
    Expand,
    Collapse,
    SetValue(Arc<str>),
    ScrollIntoView,
    Focus,
    Blur,
}

// ============================================================================
// AccessibilityState
// ============================================================================

/// 无障碍状态集合，描述元素的当前交互状态。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct AccessibilityState {
    pub disabled: bool,
    pub selected: bool,
    pub expanded: bool,
    pub collapsed: bool,
    pub modal: bool,
    pub multiline: bool,
    pub required: bool,
    pub read_only: bool,
    pub password: bool,
    pub checked: bool,
}

impl AccessibilityState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            disabled: false,
            selected: false,
            expanded: false,
            collapsed: false,
            modal: false,
            multiline: false,
            required: false,
            read_only: false,
            password: false,
            checked: false,
        }
    }

    #[must_use]
    pub fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }
}

// ============================================================================
// AccessibilityNode
// ============================================================================

/// 无障碍节点，表示 widget 树中一个元素的无障碍信息。
///
/// 框架在布局后调用 `WidgetSpec::accessibility()`，
/// 将返回的节点推入 `AccessibilityTree`。
#[derive(Clone, PartialEq, Debug)]
pub struct AccessibilityNode {
    pub widget_id: WidgetId,
    pub role: AccessibilityRole,
    pub label: Option<Arc<str>>,
    pub value: Option<Arc<str>>,
    pub state: AccessibilityState,
    pub bounds: Rect,
    pub children: Vec<WidgetId>,
    pub actions: Vec<AccessibilityAction>,
    pub heading_level: Option<u8>,
}

impl AccessibilityNode {
    /// 创建表示"无无障碍信息"的占位节点。
    #[must_use]
    pub fn none() -> Self {
        Self {
            widget_id: WidgetId::from_u64(0),
            role: AccessibilityRole::None,
            label: None,
            value: None,
            state: AccessibilityState::new(),
            bounds: Rect::ZERO,
            children: Vec::new(),
            actions: Vec::new(),
            heading_level: None,
        }
    }

    #[must_use]
    pub fn new(widget_id: WidgetId, role: AccessibilityRole, bounds: Rect) -> Self {
        Self {
            widget_id,
            role,
            label: None,
            value: None,
            state: AccessibilityState::new(),
            bounds,
            children: Vec::new(),
            actions: Vec::new(),
            heading_level: None,
        }
    }

    #[must_use]
    pub fn label(mut self, text: impl Into<Arc<str>>) -> Self {
        self.label = Some(text.into());
        self
    }

    #[must_use]
    pub fn value(mut self, text: impl Into<Arc<str>>) -> Self {
        self.value = Some(text.into());
        self
    }

    #[must_use]
    pub fn child(mut self, id: WidgetId) -> Self {
        self.children.push(id);
        self
    }

    #[must_use]
    pub fn action(mut self, action: AccessibilityAction) -> Self {
        self.actions.push(action);
        self
    }

    #[must_use]
    pub fn heading_level(mut self, level: u8) -> Self {
        self.heading_level = Some(level);
        self
    }
}

impl fmt::Display for AccessibilityNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AccessibilityNode({:?}, label={})",
            self.role,
            self.label.as_deref().unwrap_or("(无)")
        )
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_none_has_none_role() {
        let node = AccessibilityNode::none();
        assert_eq!(node.role, AccessibilityRole::None);
    }

    #[test]
    fn node_builder_constructs_correctly() {
        let id = WidgetId::from_u64(1);
        let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
        let node = AccessibilityNode::new(id, AccessibilityRole::Button, bounds)
            .label("提交")
            .action(AccessibilityAction::Click)
            .child(WidgetId::from_u64(2));
        assert_eq!(node.role, AccessibilityRole::Button);
        assert_eq!(node.label, Some(Arc::from("提交")));
        assert_eq!(node.actions.len(), 1);
        assert_eq!(node.children.len(), 1);
    }

    #[test]
    fn state_default_all_false() {
        let state = AccessibilityState::new();
        assert!(!state.disabled);
        assert!(!state.selected);
        assert!(!state.checked);
    }

    #[test]
    fn state_disabled_builder() {
        let state = AccessibilityState::new().disabled(true);
        assert!(state.disabled);
    }
}
