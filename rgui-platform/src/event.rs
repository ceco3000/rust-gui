//! 事件类型体系——Event、Key、Modifiers、MouseButton。
//!
//! 定义源自 D5 §2。

use rgui_core::geometry::Point;
use rgui_core::id::WidgetId;
use rgui_style::theme::ColorScheme;

// ============================================================================
// Event
// ============================================================================

/// 框架统一事件类型（D5 §2.1）。
#[derive(Debug, Clone)]
pub enum Event {
    MouseDown {
        position: Point,
        button: MouseButton,
        modifiers: Modifiers,
    },
    MouseUp {
        position: Point,
        button: MouseButton,
        modifiers: Modifiers,
    },
    MouseMove {
        position: Point,
        delta: Point,
        modifiers: Modifiers,
    },
    MouseWheel {
        position: Point,
        delta_x: f64,
        delta_y: f64,
        modifiers: Modifiers,
    },
    MouseEnter {
        widget_id: WidgetId,
    },
    MouseLeave {
        widget_id: WidgetId,
    },

    KeyDown {
        key: Key,
        modifiers: Modifiers,
        repeat: bool,
    },
    KeyUp {
        key: Key,
        modifiers: Modifiers,
    },

    ImePreedit {
        text: String,
        cursor_offset: usize,
    },
    ImeCommit {
        text: String,
    },
    ImeEnabled,
    ImeDisabled,

    FocusIn {
        widget_id: WidgetId,
        source: FocusSource,
    },
    FocusOut {
        widget_id: WidgetId,
    },

    DragEnter {
        position: Point,
    },
    DragOver {
        position: Point,
    },
    DragLeave,
    Drop {
        position: Point,
    },

    WindowResized {
        width: f64,
        height: f64,
    },
    WindowFocused,
    WindowUnfocused,
    ScaleFactorChanged {
        scale_factor: f64,
    },
    ThemeChanged {
        color_scheme: ColorScheme,
    },
    CloseRequested,
}

// ============================================================================
// FocusSource
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusSource {
    Keyboard,
    Mouse,
    Programmatic,
}

// ============================================================================
// Modifiers
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl Modifiers {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: false,
            meta: false,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.shift && !self.ctrl && !self.alt && !self.meta
    }
}

// ============================================================================
// MouseButton
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u8),
}

// ============================================================================
// Key
// ============================================================================

/// 键码枚举（D5 §2.2，核心键码）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    PageUp,
    PageDown,
    Enter,
    Backspace,
    Delete,
    Insert,
    Tab,
    Escape,
    Space,
    Shift,
    Ctrl,
    Alt,
    Meta,
}

impl Key {
    /// 判断是否为修饰键。
    #[must_use]
    pub fn is_modifier(&self) -> bool {
        matches!(self, Self::Shift | Self::Ctrl | Self::Alt | Self::Meta)
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_default_empty() {
        let m = Modifiers::default();
        assert!(m.is_empty());
    }

    #[test]
    fn modifiers_ctrl() {
        let m = Modifiers {
            ctrl: true,
            ..Modifiers::new()
        };
        assert!(!m.is_empty());
        assert!(m.ctrl);
    }

    #[test]
    fn key_is_modifier() {
        assert!(Key::Shift.is_modifier());
        assert!(!Key::Enter.is_modifier());
    }

    #[test]
    fn event_clone() {
        let evt = Event::KeyDown {
            key: Key::Enter,
            modifiers: Modifiers::new(),
            repeat: false,
        };
        let _clone = evt.clone();
    }
}
