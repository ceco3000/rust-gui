//! 焦点管理——FocusManager（D5 §5）。

use rgui_core::id::WidgetId;
use std::fmt;

/// 焦点管理器。
#[derive(Default)]
pub struct FocusManager {
    current: Option<WidgetId>,
    history: Vec<WidgetId>,
    trap_stack: Vec<Vec<WidgetId>>,
}

impl FocusManager {
    #[must_use]
    pub fn new() -> Self {
        Self { current: None, history: Vec::new(), trap_stack: Vec::new() }
    }

    #[must_use]
    pub fn current(&self) -> Option<WidgetId> {
        self.current
    }

    pub fn focus(&mut self, widget_id: WidgetId) {
        if let Some(trap) = self.trap_stack.last() {
            if !trap.contains(&widget_id) {
                return;
            }
        }

        if let Some(prev) = self.current {
            if prev != widget_id {
                self.history.push(prev);
            }
        }
        self.current = Some(widget_id);
    }

    /// 清除当前焦点（不修改历史栈）。
    pub fn blur(&mut self) {
        self.current = None;
    }

    /// 恢复上一次焦点。
    pub fn restore(&mut self) {
        self.current = self.history.pop();
    }

    pub fn push_trap(&mut self, widgets: Vec<WidgetId>) {
        self.trap_stack.push(widgets);
    }

    pub fn pop_trap(&mut self) {
        self.trap_stack.pop();
    }

    #[must_use]
    pub fn is_trapped(&self, widget_id: WidgetId) -> bool {
        self.trap_stack
            .last()
            .is_none_or(|trap| trap.contains(&widget_id))
    }
}

impl fmt::Debug for FocusManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FocusManager")
            .field("current", &self.current)
            .field("history_depth", &self.history.len())
            .field("trap_depth", &self.trap_stack.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_manager_new_has_no_focus() {
        let fm = FocusManager::new();
        assert_eq!(fm.current(), None);
    }

    #[test]
    fn focus_and_blur() {
        let mut fm = FocusManager::new();
        let id = WidgetId::from_u64(1);
        fm.focus(id);
        assert_eq!(fm.current(), Some(id));
        fm.blur();
        assert_eq!(fm.current(), None);
    }

    #[test]
    fn focus_restore() {
        let mut fm = FocusManager::new();
        fm.focus(WidgetId::from_u64(1));
        fm.focus(WidgetId::from_u64(2));
        // 焦点从 1→2，history 中存有 1
        fm.restore();
        assert_eq!(fm.current(), Some(WidgetId::from_u64(1)));
    }

    #[test]
    fn trap_blocks_unlisted_focus() {
        let mut fm = FocusManager::new();
        let a = WidgetId::from_u64(1);
        let b = WidgetId::from_u64(2);
        fm.push_trap(vec![a]);
        fm.focus(b);
        assert_eq!(fm.current(), None);
        fm.focus(a);
        assert_eq!(fm.current(), Some(a));
    }
}
