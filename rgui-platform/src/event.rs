//! 事件类型体系——Event、Key、Modifiers、MouseButton、EventSender。
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
// EventSender
// ============================================================================

/// 事件发送器——事件路由期间的事件级状态控制（D5 §3.2）。
///
/// 在事件路由过程中传递给每个事件处理器，允许处理器：
/// - 标记事件已被消费（停止传播）
/// - 阻止默认行为
///
/// `EventRouter::route()` 在每次路由调用时创建新的 `EventSender`，
/// 并在每个阶段检查 `consumed` 以决定是否继续传播。
#[derive(Debug, Clone, Default)]
pub struct EventSender {
    /// 事件是否已被消费（停止进一步传播）。
    pub consumed: bool,
    /// 是否阻止默认行为。
    pub default_prevented: bool,
}

impl EventSender {
    /// 创建默认（未消费、未阻止）的发送器。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 消费事件，停止进一步传播。
    pub fn consume(&mut self) {
        self.consumed = true;
    }

    /// 阻止默认行为。
    pub fn prevent_default(&mut self) {
        self.default_prevented = true;
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

    // --- EventSender ---

    #[test]
    fn event_sender_default_not_consumed() {
        let sender = EventSender::default();
        assert!(!sender.consumed);
        assert!(!sender.default_prevented);
    }

    #[test]
    fn event_sender_consume() {
        let mut sender = EventSender::new();
        sender.consume();
        assert!(sender.consumed);
    }

    #[test]
    fn event_sender_prevent_default() {
        let mut sender = EventSender::new();
        sender.prevent_default();
        assert!(sender.default_prevented);
        assert!(!sender.consumed);
    }

    #[test]
    fn event_sender_clone() {
        let mut original = EventSender::new();
        original.consume();
        let cloned = original.clone();
        assert!(cloned.consumed);
    }
}
