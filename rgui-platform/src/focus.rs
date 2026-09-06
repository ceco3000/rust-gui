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
    /// 可获焦组件（有序，供 Tab 循环切换）。模态打开时 = 模态内集合（焦点隔离）。
    focusable: Vec<WidgetId>,
    /// 模态打开前的可获焦列表（关闭模态时恢复）。
    base_focusable: Vec<WidgetId>,
    /// 模态打开前的焦点（关闭模态时恢复）。
    base_focused: Option<WidgetId>,
    /// 是否处于模态层（打开时焦点隔离在模态集合内）。
    modal: bool,
}

impl FocusManager {
    /// 构造空焦点管理器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置输入模态。D12：占位（输入模态类型后续）；模态层级见 `open_modal`/`close_modal`。
    pub fn set_modality(&mut self, _modality: InputModality) {
        // 输入模态（modality）留后续；焦点模态层级经 open_modal/close_modal 提供。
    }

    /// 打开模态层（D20）：把当前可获焦列表暂存为 base，焦点限定在 `modal_focusable` 集合内（隔离）。
    /// 若当前焦点不在模态集合，自动移到第一个模态可获焦项。
    pub fn open_modal(&mut self, modal_focusable: Vec<WidgetId>) {
        if self.modal {
            return; // 单层，忽略嵌套（模态不叠加）
        }
        self.base_focusable = std::mem::take(&mut self.focusable);
        self.base_focused = self.focused; // 保存打开前焦点供关闭恢复
        self.focusable = modal_focusable;
        self.modal = true;
        // 焦点隔离：当前焦点不在模态集合 → 移到第一个模态可获焦项
        if let Some(f) = self.focused {
            if !self.focusable.contains(&f) {
                self.focused = self.focusable.first().copied();
            }
        }
    }

    /// 关闭模态层（D20）：恢复基座可获焦列表与打开前焦点；若不在基座则清空。
    pub fn close_modal(&mut self) {
        if !self.modal {
            return;
        }
        self.focusable = std::mem::take(&mut self.base_focusable);
        self.modal = false;
        // 恢复打开模态前的焦点（若仍在基座列表），否则清空
        match self.base_focused.take() {
            Some(f) if self.focusable.contains(&f) => self.focused = Some(f),
            _ => self.focused = None,
        }
    }

    /// 是否处于模态层（焦点隔离中）。
    pub fn is_modal_open(&self) -> bool {
        self.modal
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
        tracing::debug!(target: "rgui_platform", "focus_changed {:?}", next);
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
        assert_eq!(
            fm.focus_next(),
            Some(WidgetId::new(1)),
            "无焦点应获焦第一个"
        );
    }

    #[test]
    fn set_focus_rejects_non_focusable() {
        let mut fm = FocusManager::new();
        fm.set_focusable(ids());
        assert!(
            !fm.set_focus(WidgetId::new(99)),
            "不可获焦组件的 set_focus 应失败"
        );
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

    #[test]
    fn modal_opened_isolates_focus_within_modal_set() {
        let mut fm = FocusManager::new();
        fm.set_focusable(ids()); // base = [1,2,3]
        assert!(fm.set_focus(WidgetId::new(1)));
        // 打开模态（可获焦 = [10,11]）；焦点从 1 隔离到模态内
        fm.open_modal(vec![WidgetId::new(10), WidgetId::new(11)]);
        assert!(fm.is_modal_open());
        // 焦点不在模态集合 → 移到第一个模态项
        assert_eq!(fm.focus(), Some(WidgetId::new(10)));
        // Tab 只在模态内循环，不返回 base 的 2/3
        assert_eq!(fm.focus_next(), Some(WidgetId::new(11)));
        assert_eq!(fm.focus_next(), Some(WidgetId::new(10)), "模态内循环");
        // base 不可获焦
        assert!(
            !fm.set_focus(WidgetId::new(2)),
            "模态打开时 base 组件不可获焦"
        );
    }

    #[test]
    fn modal_closed_restores_base_focusable_and_focus() {
        let mut fm = FocusManager::new();
        fm.set_focusable(ids());
        assert!(fm.set_focus(WidgetId::new(2)));
        fm.open_modal(vec![WidgetId::new(10)]);
        fm.close_modal();
        assert!(!fm.is_modal_open());
        // base 恢复，2 仍在其中 → 焦点保留
        assert_eq!(
            fm.focusable(),
            &[WidgetId::new(1), WidgetId::new(2), WidgetId::new(3)][..]
        );
        assert_eq!(fm.focus(), Some(WidgetId::new(2)), "恢复后焦点保留");
        assert!(fm.set_focus(WidgetId::new(3)), "base 恢复后可获焦");
    }

    #[test]
    fn modal_close_clears_focus_when_base_focused_is_none() {
        let mut fm = FocusManager::new();
        fm.set_focusable(vec![WidgetId::new(1)]); // 打开前无焦点
        fm.open_modal(vec![WidgetId::new(10)]);
        assert!(fm.set_focus(WidgetId::new(10)));
        fm.close_modal();
        // 打开前无焦点（base_focused=None）→ 关闭后仍无焦点（不残留模态焦点）
        assert_eq!(fm.focus(), None, "打开前无焦点，关闭模态后应保持无焦点");
    }
}
