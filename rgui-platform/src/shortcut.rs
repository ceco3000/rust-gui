//! 快捷键管理器（D5 §6.2）。

use crate::event::Key;
use rustc_hash::FxHashMap;

/// 组合键：键 + 修饰键。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub key: Key,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

impl KeyChord {
    pub fn new(key: Key) -> Self {
        Self {
            key,
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        }
    }

    pub fn ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }
    pub fn alt(mut self) -> Self {
        self.alt = true;
        self
    }
    pub fn shift(mut self) -> Self {
        self.shift = true;
        self
    }
    pub fn meta(mut self) -> Self {
        self.meta = true;
        self
    }

    pub fn with_key(key: Key, ctrl: bool, alt: bool, shift: bool, meta: bool) -> Self {
        Self {
            key,
            ctrl,
            alt,
            shift,
            meta,
        }
    }
}

/// 快捷键管理器（D5 §6.2）。
///
/// 存储快捷键绑定并在键事件匹配时触发处理。
pub struct ShortcutManager {
    shortcuts: FxHashMap<KeyChord, Box<dyn Fn() + Send + Sync>>,
}

impl ShortcutManager {
    pub fn new() -> Self {
        Self {
            shortcuts: FxHashMap::default(),
        }
    }

    /// 注册快捷键。
    pub fn register(&mut self, chord: KeyChord, handler: impl Fn() + Send + Sync + 'static) {
        self.shortcuts.insert(chord, Box::new(handler));
    }

    /// 尝试匹配并触发快捷键。返回 true 表示已处理。
    pub fn try_handle(&self, key: Key, ctrl: bool, alt: bool, shift: bool, meta: bool) -> bool {
        let chord = KeyChord::with_key(key, ctrl, alt, shift, meta);
        if let Some(handler) = self.shortcuts.get(&chord) {
            handler();
            true
        } else {
            false
        }
    }
}

impl Default for ShortcutManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_trigger() {
        let mut m = ShortcutManager::new();
        m.register(KeyChord::new(Key::S).ctrl(), || {});
        assert!(m.try_handle(Key::S, true, false, false, false));
    }

    #[test]
    fn missing_shortcut_returns_false() {
        let m = ShortcutManager::new();
        assert!(!m.try_handle(Key::Z, true, false, false, false));
    }

    #[test]
    fn modifier_mismatch_returns_false() {
        let mut m = ShortcutManager::new();
        m.register(KeyChord::new(Key::C).ctrl(), || {});
        assert!(!m.try_handle(Key::C, false, false, false, false));
    }
}
