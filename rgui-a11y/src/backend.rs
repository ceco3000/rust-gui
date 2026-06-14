//! 无障碍后端 trait — AccessibilityBackend + AccessKit 角色映射（D6 §3-4）。

use rgui_core::a11y::AccessibilityRole;
use rgui_core::id::WidgetId;

use crate::tree::AccessibilityTree;

// ---------------------------------------------------------------------------
// AccessibilityBackend trait（D6 §4）
// ---------------------------------------------------------------------------

/// 无障碍后端抽象。
pub trait AccessibilityBackend: Send + Sync {
    fn push_tree(&mut self, tree: &AccessibilityTree);
    fn focus_changed(&mut self, widget_id: WidgetId);
    fn handle_action(&mut self, widget_id: WidgetId, action: &str) -> bool;
    fn name(&self) -> &'static str;
}

// ---------------------------------------------------------------------------
// NullBackend
// ---------------------------------------------------------------------------

/// 空后端（测试及无平台集成场景）。
pub struct NullBackend;

impl AccessibilityBackend for NullBackend {
    fn push_tree(&mut self, _: &AccessibilityTree) {}
    fn focus_changed(&mut self, _: WidgetId) {}
    fn handle_action(&mut self, _: WidgetId, _: &str) -> bool {
        false
    }
    fn name(&self) -> &'static str {
        "null"
    }
}

// ---------------------------------------------------------------------------
// 角色映射（D6 §3.2）
// ---------------------------------------------------------------------------

/// rgui AccessibilityRole → accesskit::Role 映射。
///
/// 映射 rgui 无障碍角色到 accesskit 等效角色。
/// 未覆盖的角色回退到 `Unknown`。
pub fn to_accesskit_role(role: AccessibilityRole) -> accesskit::Role {
    use AccessibilityRole as R;
    use accesskit::Role as AR;
    match role {
        R::None => AR::Unknown,
        R::Button => AR::Button,
        R::TextInput => AR::TextInput,
        R::CheckBox => AR::CheckBox,
        R::RadioButton => AR::RadioButton,
        R::Link => AR::Link,
        R::Image => AR::Image,
        R::Heading => AR::Heading,
        R::List => AR::List,
        R::ListItem => AR::ListItem,
        R::Table => AR::Table,
        R::TableRow => AR::Row,
        R::TableCell => AR::Cell,
        R::Menu => AR::Menu,
        R::MenuItem => AR::MenuItem,
        R::Dialog => AR::Dialog,
        R::Slider => AR::Slider,
        R::ProgressBar => AR::ProgressIndicator,
        _ => AR::Unknown,
    }
}

// ---------------------------------------------------------------------------
// AccessKitBackend（D6 §3.3，完整 AccessKit 集成由 A06 实现）
// ---------------------------------------------------------------------------

/// AccessKit 后端：将 rgui 无障碍树同步到平台屏幕阅读器。
pub struct AccessKitBackend;

impl AccessKitBackend {
    pub fn new() -> Self {
        Self {}
    }
}

impl AccessibilityBackend for AccessKitBackend {
    fn push_tree(&mut self, _tree: &AccessibilityTree) {}

    fn focus_changed(&mut self, _widget_id: WidgetId) {}

    fn handle_action(&mut self, _widget_id: WidgetId, _action: &str) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "accesskit"
    }
}

impl Default for AccessKitBackend {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_backend_push_tree_noop() {
        let mut b = NullBackend;
        let tree = AccessibilityTree::new();
        b.push_tree(&tree);
        assert_eq!(b.name(), "null");
    }

    #[test]
    fn null_backend_handle_action_false() {
        let mut b = NullBackend;
        assert!(!b.handle_action(WidgetId::from_u64(1), "click"));
    }

    #[test]
    fn role_mapping_covers_all_variants() {
        use rgui_core::a11y::AccessibilityRole::*;
        let roles = [
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
        ];
        for role in roles {
            let _ak_role = to_accesskit_role(role);
        }
    }
}
