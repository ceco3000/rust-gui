//! 焦点管理模块（greenfield §B.3：焦点管理原生在 platform）。
//!
//! D12：增强 FocusManager——维护可获焦组件列表，`Tab`/`Shift+Tab` 循环切换焦点，
//! 提供获焦/失焦查询（`is_focused` / `focus()`），组件可声明可聚焦（`WidgetSpec::focusable`）。

use crate::input::InputModality;
use rgui_core::id::WidgetId;

/// 焦点管理器。
#[derive(Debug, Default)]
pub struct FocusManager {
    /// 当前焦点组件 id。
    focused: Option<WidgetId>,
    /// 可获焦组件（有序，供 Tab 循环切换）。
    focusable: Vec<WidgetId>,
}

impl FocusManager {
    /// 构造空焦点管理器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置输入模态。D12：保持占位（模态层级留后续）。
    pub fn set_modality(&mut self, _modality: InputModality) {
        // 模态层级（modal layer）留后续；当前仅维护焦点。
    }

    /// 注册可获焦组件（有序列表，供 Tab 循环）。若当前焦点被移除，清空焦点（D18 focused 残留边界）。
    pub fn set_focusable(&mut self, ids: Vec<WidgetId>) {
        if let Some(f) = self.focused {
            if !ids.contains(&f) {
                self.focused = None;
            }
        }
        self.focusable = ids;
    }

    /// 设置焦点到指定组件（仅当其可获焦时成功），返回是否设置成功。
    /// 调用方对比 `focus()` 前后值可判定获焦/失焦。
    pub fn set_focus(&mut self, widget_id: WidgetId) -> bool {
        if self.focusable.contains(&widget_id) {
            self.focused = Some(widget_id);
            true
        } else {
            false
        }
    }

    /// 当前焦点组件 id。
    pub fn focus(&self) -> Option<WidgetId> {
        self.focused
    }

    /// 判断某组件是否为当前焦点（获焦查询，供保留视觉/行为）。
    pub fn is_focused(&self, widget_id: WidgetId) -> bool {
        self.focused == Some(widget_id)
    }

    /// 可获焦列表。
    pub fn focusable(&self) -> &[WidgetId] {
        &self.focusable
    }

    /// `Tab`：焦点循环移动到下一个可获焦组件，返回新焦点（无焦点则获焦第一个；无可获焦则 `None`）。
    pub fn focus_next(&mut self) -> Option<WidgetId> {
        self.move_focus(1)
    }

    /// `Shift+Tab`：焦点循环移动到上一个可获焦组件。
    pub fn focus_prev(&mut self) -> Option<WidgetId> {
        self.move_focus(-1)
    }

    /// 移动焦点 `dir` 步（正=下一个，负=上一个），循环回绕。返回新焦点。流式：`iter().position`。
    fn move_focus(&mut self, dir: i32) -> Option<WidgetId> {
        if self.focusable.is_empty() {
            return None;
        }
        let n = self.focusable.len() as i32;
        let next = match self.focused {
            Some(c) => self
                .focusable
                .iter()
                .position(|&x| x == c)
                .map(|i| self.focusable[((i as i32 + dir).rem_euclid(n)) as usize]),
            None => Some(if dir > 0 {
                self.focusable[0]
            } else {
                self.focusable[self.focusable.len() - 1]
            }),
        }?;
        self.focused = Some(next);
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> Vec<WidgetId> {
        vec![WidgetId::new(1), WidgetId::new(2), WidgetId::new(3)]
    }

    #[test]
    fn focus_next_cycles_forward() {
        let mut fm = FocusManager::new();
        fm.set_focusable(ids());
        assert!(fm.set_focus(WidgetId::new(1)));
        assert_eq!(fm.focus_next(), Some(WidgetId::new(2)));
        assert!(fm.is_focused(WidgetId::new(2)));
        assert_eq!(fm.focus_next(), Some(WidgetId::new(3)));
        assert_eq!(fm.focus_next(), Some(WidgetId::new(1)), "末位应回绕到首位");
    }

    #[test]
    fn focus_prev_cycles_backward() {
        let mut fm = FocusManager::new();
        fm.set_focusable(ids());
        assert!(fm.set_focus(WidgetId::new(2)));
        assert_eq!(fm.focus_prev(), Some(WidgetId::new(1)));
        assert_eq!(fm.focus_prev(), Some(WidgetId::new(3)), "首位应回绕到末位");
    }

    #[test]
    fn focus_next_with_no_focus_takes_first() {
        let mut fm = FocusManager::new();
        fm.set_focusable(ids());
        assert_eq!(fm.focus_next(), Some(WidgetId::new(1)), "无焦点应获焦第一个");
    }

    #[test]
    fn set_focus_rejects_non_focusable() {
        let mut fm = FocusManager::new();
        fm.set_focusable(ids());
        assert!(!fm.set_focus(WidgetId::new(99)), "不可获焦组件的 set_focus 应失败");
        assert_eq!(fm.focus(), None);
    }

    #[test]
    fn focus_next_on_empty_focusable_returns_none() {
        let mut fm = FocusManager::new();
        assert_eq!(fm.focus_next(), None);
    }

    #[test]
    fn set_focusable_clears_removed_focus_and_keeps_existing() {
        // focused=2 被移除 → 清空焦点
        let mut fm = FocusManager::new();
        fm.set_focusable(vec![WidgetId::new(1), WidgetId::new(2)]);
        assert!(fm.set_focus(WidgetId::new(2)));
        fm.set_focusable(vec![WidgetId::new(1), WidgetId::new(3)]);
        assert_eq!(fm.focus(), None, "被移除的焦点应清空（不残留）");

        // focused=1 仍在新列表 → 保留
        let mut fm2 = FocusManager::new();
        fm2.set_focusable(vec![WidgetId::new(1), WidgetId::new(2)]);
        assert!(fm2.set_focus(WidgetId::new(1)));
        fm2.set_focusable(vec![WidgetId::new(1), WidgetId::new(3)]);
        assert_eq!(fm2.focus(), Some(WidgetId::new(1)), "仍存在的焦点应保留");
    }
}
