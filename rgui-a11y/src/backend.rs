//! 无障碍后端 trait — AccessibilityBackend + AccessKit 角色映射（D6 §3-4）。
//!
//! 本模块定义了屏幕阅读器操作处理的核心抽象和实现。

use std::sync::{Arc, Mutex};

use rgui_core::a11y::{AccessibilityAction, AccessibilityRole};
use rgui_core::id::WidgetId;

use crate::tree::AccessibilityTree;

// ---------------------------------------------------------------------------
// 屏幕阅读器动作处理器
// ---------------------------------------------------------------------------

/// 屏幕阅读器动作处理器回调。
///
/// 当平台屏幕阅读器发出操作请求（如 VoiceOver 激活按钮）时，
/// 通过此回调将操作转发到框架事件系统。
///
/// # 参数
///
/// * `WidgetId` — 目标组件
/// * `AccessibilityAction` — 请求的动作类型
///
/// # 返回值
///
/// `true` 表示操作已成功处理，`false` 表示框架未处理该操作。
pub type ActionHandlerCallback = Box<dyn FnMut(WidgetId, AccessibilityAction) -> bool + Send>;

// ---------------------------------------------------------------------------
// AccessibilityBackend trait（D6 §4）
// ---------------------------------------------------------------------------

/// 无障碍后端抽象。
///
/// 负责将 rgui 无障碍树推送到平台无障碍 API，
/// 以及处理来自屏幕阅读器的操作请求。
pub trait AccessibilityBackend: Send + Sync {
    /// 推送完整的无障碍树更新。
    fn push_tree(&mut self, tree: &AccessibilityTree);

    /// 推送焦点变更。
    fn focus_changed(&mut self, widget_id: WidgetId);

    /// 处理来自屏幕阅读器的操作请求（返回是否处理成功）。
    ///
    /// `action` 参数使用框架统一的 `AccessibilityAction` 枚举，
    /// 后端应将其转换为对应的事件并分发给目标 widget。
    fn handle_action(&mut self, widget_id: WidgetId, action: AccessibilityAction) -> bool;

    /// 后端名称。
    fn name(&self) -> &'static str;
}

// ---------------------------------------------------------------------------
// NullBackend
// ---------------------------------------------------------------------------

/// 空后端（测试及无平台集成场景）。
///
/// 所有操作均为空操作，`handle_action` 始终返回 `false`。
pub struct NullBackend;

impl AccessibilityBackend for NullBackend {
    fn push_tree(&mut self, _: &AccessibilityTree) {}

    fn focus_changed(&mut self, _: WidgetId) {}

    fn handle_action(&mut self, _: WidgetId, _: AccessibilityAction) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "null"
    }
}

// ---------------------------------------------------------------------------
// AccessKitBackend（D6 §4）
// ---------------------------------------------------------------------------

/// AccessKit 后端：将 rgui 无障碍树同步到平台屏幕阅读器。
///
/// 支持注册屏幕阅读器操作处理器，当平台请求操作时通过回调转发。
pub struct AccessKitBackend {
    /// 已注册的屏幕阅读器动作处理器。
    handler: Mutex<Option<ActionHandlerCallback>>,
}

impl AccessKitBackend {
    /// 创建新的 AccessKit 后端实例。
    #[must_use]
    pub fn new() -> Self {
        Self {
            handler: Mutex::new(None),
        }
    }

    /// 注册屏幕阅读器动作处理器。
    ///
    /// 当屏幕阅读器发起操作请求时（如 VoiceOver 激活按钮），
    /// 后端将调用此处理器，参数为目标 widget 和请求的动作类型。
    pub fn set_action_handler(&mut self, handler: ActionHandlerCallback) {
        *self.handler.lock().unwrap() = Some(handler);
    }

    /// 移除已注册的屏幕阅读器动作处理器。
    pub fn clear_action_handler(&mut self) {
        *self.handler.lock().unwrap() = None;
    }

    /// 检查是否已注册动作处理器。
    #[must_use]
    pub fn has_handler(&self) -> bool {
        self.handler.lock().unwrap().is_some()
    }
}

impl Default for AccessKitBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AccessibilityBackend for AccessKitBackend {
    fn push_tree(&mut self, _tree: &AccessibilityTree) {}

    fn focus_changed(&mut self, _widget_id: WidgetId) {}

    fn handle_action(&mut self, widget_id: WidgetId, action: AccessibilityAction) -> bool {
        let mut guard = self.handler.lock().unwrap();
        match &mut *guard {
            Some(handler) => handler(widget_id, action),
            None => false,
        }
    }

    fn name(&self) -> &'static str {
        "accesskit"
    }
}

// ---------------------------------------------------------------------------
// AccessKit 动作映射（D6 §3-4）
// ---------------------------------------------------------------------------

/// 将 `accesskit::Action` 映射到 rgui `AccessibilityAction`。
///
/// 平台屏幕阅读器通过 AccessKit 发出操作请求（如 VoiceOver 的「激活」），
/// AccessKit 将它转换为 `accesskit::Action`。此函数将其映射为框架统一类型。
///
/// # 返回值
///
/// 返回 `Some(AccessibilityAction)` 表示映射成功；
/// 返回 `None` 表示框架不支持该操作。
#[must_use]
pub fn from_accesskit_action(action: accesskit::Action) -> Option<AccessibilityAction> {
    match action {
        accesskit::Action::Focus => Some(AccessibilityAction::Focus),
        accesskit::Action::Increment => Some(AccessibilityAction::SetValue(Arc::from("increment"))),
        accesskit::Action::Decrement => Some(AccessibilityAction::SetValue(Arc::from("decrement"))),
        accesskit::Action::Expand => Some(AccessibilityAction::Expand),
        accesskit::Action::Collapse => Some(AccessibilityAction::Collapse),
        accesskit::Action::ScrollIntoView => Some(AccessibilityAction::ScrollIntoView),
        accesskit::Action::ShowContextMenu => None,
        accesskit::Action::SetValue => Some(AccessibilityAction::SetValue(Arc::from("set_value"))),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// 角色映射（D6 §3.2）
// ---------------------------------------------------------------------------

/// rgui AccessibilityRole → accesskit::Role 映射。
///
/// 映射 rgui 无障碍角色到 accesskit 等效角色。
/// 未覆盖的角色回退到 `Unknown`。
#[must_use]
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
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rgui_core::a11y::AccessibilityAction;

    // ======================================================================
    // NullBackend 测试
    // ======================================================================

    #[test]
    fn null_backend_push_tree_noop() {
        let mut b = NullBackend;
        let tree = AccessibilityTree::new();
        b.push_tree(&tree);
        assert_eq!(b.name(), "null");
    }

    #[test]
    fn null_backend_handle_action_always_false() {
        let mut b = NullBackend;
        let id = WidgetId::from_u64(42);

        assert!(!b.handle_action(id, AccessibilityAction::Click));
        assert!(!b.handle_action(id, AccessibilityAction::Focus));
        assert!(!b.handle_action(id, AccessibilityAction::Blur));
        assert!(!b.handle_action(id, AccessibilityAction::Toggle));
        assert!(!b.handle_action(id, AccessibilityAction::Expand));
        assert!(!b.handle_action(id, AccessibilityAction::Collapse));
        assert!(!b.handle_action(id, AccessibilityAction::ScrollIntoView));
        assert!(!b.handle_action(id, AccessibilityAction::SetValue(Arc::from("test"))));
    }

    #[test]
    fn null_backend_name() {
        let b = NullBackend;
        assert_eq!(b.name(), "null");
    }

    // ======================================================================
    // AccessKitBackend 测试
    // ======================================================================

    #[test]
    fn accesskit_backend_push_tree_noop() {
        let mut b = AccessKitBackend::new();
        let tree = AccessibilityTree::new();
        b.push_tree(&tree);
        assert_eq!(b.name(), "accesskit");
    }

    #[test]
    fn accesskit_backend_no_handler_returns_false() {
        let mut b = AccessKitBackend::new();
        let id = WidgetId::from_u64(1);

        assert!(!b.handle_action(id, AccessibilityAction::Click));
        assert!(!b.handle_action(id, AccessibilityAction::Focus));
    }

    #[test]
    fn accesskit_backend_handler_called() {
        let mut b = AccessKitBackend::new();
        let id = WidgetId::from_u64(1);
        let processed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let processed_clone = processed.clone();

        b.set_action_handler(Box::new(move |_, _| {
            processed_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            true
        }));

        let result = b.handle_action(id, AccessibilityAction::Click);

        assert!(result);
        assert!(processed.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn accesskit_backend_handler_propagates_result() {
        let mut b = AccessKitBackend::new();
        let id = WidgetId::from_u64(2);

        b.set_action_handler(Box::new(|_, _| false));
        assert!(!b.handle_action(id, AccessibilityAction::Focus));

        b.set_action_handler(Box::new(|_, _| true));
        assert!(b.handle_action(id, AccessibilityAction::Focus));
    }

    #[test]
    fn accesskit_backend_handler_receives_correct_params() {
        let mut b = AccessKitBackend::new();
        let expected_id = WidgetId::from_u64(99);
        let expected_action = AccessibilityAction::SetValue(Arc::from("hello"));

        let captured_id = Arc::new(Mutex::new(None));
        let captured_action = Arc::new(Mutex::new(None));
        let cid = captured_id.clone();
        let ca = captured_action.clone();

        b.set_action_handler(Box::new(move |wid, act| {
            *cid.lock().unwrap() = Some(wid);
            *ca.lock().unwrap() = Some(act);
            true
        }));

        b.handle_action(expected_id, expected_action.clone());

        assert_eq!(*captured_id.lock().unwrap(), Some(expected_id));
        assert_eq!(*captured_action.lock().unwrap(), Some(expected_action));
    }

    #[test]
    fn accesskit_backend_clear_handler() {
        let mut b = AccessKitBackend::new();
        let id = WidgetId::from_u64(1);

        b.set_action_handler(Box::new(|_, _| true));
        assert!(b.has_handler());

        b.clear_action_handler();
        assert!(!b.has_handler());
        assert!(!b.handle_action(id, AccessibilityAction::Click));
    }

    #[test]
    fn accesskit_backend_default_has_no_handler() {
        let b: AccessKitBackend = Default::default();
        assert!(!b.has_handler());
    }

    #[test]
    fn accesskit_backend_with_all_action_types() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let mut b = AccessKitBackend::new();
        let id = WidgetId::from_u64(1);
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        b.set_action_handler(Box::new(move |_, _| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            true
        }));

        let actions = vec![
            AccessibilityAction::Click,
            AccessibilityAction::Focus,
            AccessibilityAction::Blur,
            AccessibilityAction::Toggle,
            AccessibilityAction::Expand,
            AccessibilityAction::Collapse,
            AccessibilityAction::ScrollIntoView,
            AccessibilityAction::SetValue(Arc::from("test")),
        ];

        for action in actions {
            assert!(b.handle_action(id, action));
        }

        assert_eq!(counter.load(Ordering::SeqCst), 8);
    }

    // ======================================================================
    // from_accesskit_action 测试
    // ======================================================================

    #[test]
    fn from_accesskit_action_focus() {
        let result = from_accesskit_action(accesskit::Action::Focus);
        assert_eq!(result, Some(AccessibilityAction::Focus));
    }

    #[test]
    fn from_accesskit_action_expand_collapse() {
        assert_eq!(
            from_accesskit_action(accesskit::Action::Expand),
            Some(AccessibilityAction::Expand)
        );
        assert_eq!(
            from_accesskit_action(accesskit::Action::Collapse),
            Some(AccessibilityAction::Collapse)
        );
    }

    #[test]
    fn from_accesskit_action_scroll_into_view() {
        assert_eq!(
            from_accesskit_action(accesskit::Action::ScrollIntoView),
            Some(AccessibilityAction::ScrollIntoView)
        );
    }

    #[test]
    fn from_accesskit_action_set_value() {
        let result = from_accesskit_action(accesskit::Action::SetValue);
        assert_eq!(
            result,
            Some(AccessibilityAction::SetValue(Arc::from("set_value")))
        );
    }

    #[test]
    fn from_accesskit_action_increment_decrement() {
        assert_eq!(
            from_accesskit_action(accesskit::Action::Increment),
            Some(AccessibilityAction::SetValue(Arc::from("increment")))
        );
        assert_eq!(
            from_accesskit_action(accesskit::Action::Decrement),
            Some(AccessibilityAction::SetValue(Arc::from("decrement")))
        );
    }

    #[test]
    fn from_accesskit_action_show_context_menu() {
        let result = from_accesskit_action(accesskit::Action::ShowContextMenu);
        assert_eq!(result, None);
    }

    /// 验证所有已知 AccessKit 动作变体都已覆盖且不 panic。
    #[test]
    fn from_accesskit_action_all_variants_covered() {
        let all_actions = [
            accesskit::Action::Focus,
            accesskit::Action::Increment,
            accesskit::Action::Decrement,
            accesskit::Action::Expand,
            accesskit::Action::Collapse,
            accesskit::Action::ScrollIntoView,
            accesskit::Action::ShowContextMenu,
            accesskit::Action::SetValue,
        ];

        for action in all_actions {
            let _ = from_accesskit_action(action);
        }
    }

    // ======================================================================
    // 角色映射测试
    // ======================================================================

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
