//! IME 管理器（D5 §7）。

use rgui_core::geometry::Point;

/// IME 组合状态。
#[derive(Debug, Clone, Default)]
pub struct ImeState {
    /// 当前组合文本（预编辑文本）。
    pub composition: Option<String>,
    /// 组合光标位置（字节偏移）。
    pub cursor: usize,
    /// 候选窗口位置。
    pub candidate_pos: Option<Point>,
    /// 是否处于组合中。
    pub composing: bool,
}

/// IME 管理器（D5 §7）。
///
/// 管理文本输入的组合状态和候选窗口位置。
pub struct ImeManager {
    state: ImeState,
    /// 当组合文本变化时的回调。
    on_composition: Box<dyn Fn(&ImeState) + Send>,
    /// 当组合完成（提交）时的回调。
    on_commit: Box<dyn Fn(&str) + Send>,
}

impl ImeManager {
    pub fn new(
        on_composition: impl Fn(&ImeState) + Send + 'static,
        on_commit: impl Fn(&str) + Send + 'static,
    ) -> Self {
        Self {
            state: ImeState::default(),
            on_composition: Box::new(on_composition),
            on_commit: Box::new(on_commit),
        }
    }

    /// 更新预编辑文本。
    pub fn set_composition(&mut self, text: Option<String>, cursor: usize) {
        self.state.composition = text;
        self.state.cursor = cursor;
        self.state.composing = self.state.composition.is_some();
        (self.on_composition)(&self.state);
    }

    /// 提交组合文本。
    pub fn commit(&mut self, text: &str) {
        self.state.composition = None;
        self.state.composing = false;
        (self.on_commit)(text);
    }

    /// 设置候选窗口位置。
    pub fn set_candidate_position(&mut self, pos: Point) {
        self.state.candidate_pos = Some(pos);
    }

    /// 清除 IME 状态。
    pub fn clear(&mut self) {
        self.state = ImeState::default();
    }

    pub fn state(&self) -> &ImeState {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composition_updates_state() {
        let mut ime = ImeManager::new(|_| {}, |_| {});
        ime.set_composition(Some("ni".into()), 2);
        assert!(ime.state().composing);
        assert_eq!(ime.state().composition.as_deref(), Some("ni"));
    }

    #[test]
    fn commit_clears_composition() {
        let mut ime = ImeManager::new(|_| {}, |_| {});
        ime.set_composition(Some("ni".into()), 2);
        ime.commit("你好");
        assert!(!ime.state().composing);
        assert!(ime.state().composition.is_none());
    }
}
