//! 事件类型体系——Event、Key、Modifiers、MouseButton、EventSender。
//!
//! 定义源自 D5 §2。

use rgui_core::geometry::{Point, Size};
use rgui_core::id::WidgetId;
use rgui_style::theme::ColorScheme;

// ============================================================================
// Coordinate Semantics
// ============================================================================

/// 平台原始窗口坐标归一化策略。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoordinateNormalization {
    /// 平台已经以窗口逻辑坐标报告输入，无需再次除以 `scale_factor`。
    PlatformNativeLogical,
    /// 原始平台坐标需要在平台边界除以 `scale_factor` 才能进入高层逻辑坐标。
    DivideByScaleFactor { scale_factor: f64 },
}

/// 鼠标输入来源。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseInputOrigin {
    /// 真实平台窗口事件。
    PlatformWindowEvent {
        raw_window_position: Point,
        normalization: CoordinateNormalization,
    },
    /// 自动化物理注入。
    PhysicalInjection {
        raw_window_position: Point,
        normalization: CoordinateNormalization,
    },
    /// 自动化逻辑注入。
    LogicalInjection,
}

/// 鼠标事件坐标集合。
///
/// - `window_logical`：窗口逻辑坐标，是高层事件、命中测试和日志的统一坐标
/// - `local_logical`：接收者局部坐标；未命中具体接收者时为 `None`
/// - `origin`：保留平台原始输入或自动化注入来源，便于调试和回归验证
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseEventCoords {
    pub window_logical: Point,
    pub local_logical: Option<Point>,
    pub origin: MouseInputOrigin,
}

impl MouseEventCoords {
    #[must_use]
    pub fn new(window_logical: Point, origin: MouseInputOrigin) -> Self {
        Self {
            window_logical,
            local_logical: None,
            origin,
        }
    }

    #[must_use]
    pub fn with_local(mut self, local_logical: Point) -> Self {
        self.local_logical = Some(local_logical);
        self
    }
}

/// 平台窗口坐标归一化结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalizedWindowPoint {
    pub window_logical: Point,
    pub normalization: CoordinateNormalization,
}

/// 平台边界唯一允许的“原始窗口坐标 -> 窗口逻辑坐标”转换入口。
#[must_use]
pub fn normalize_platform_window_point(
    raw_window_position: Point,
    scale_factor: f64,
) -> NormalizedWindowPoint {
    #[cfg(target_os = "macos")]
    let normalization = {
        let _ = scale_factor;
        CoordinateNormalization::PlatformNativeLogical
    };
    #[cfg(not(target_os = "macos"))]
    let normalization = CoordinateNormalization::DivideByScaleFactor { scale_factor };

    let window_logical = match normalization {
        CoordinateNormalization::PlatformNativeLogical => raw_window_position,
        CoordinateNormalization::DivideByScaleFactor { scale_factor } => {
            let scale_factor = scale_factor.max(f64::EPSILON);
            Point::new(
                raw_window_position.x / scale_factor,
                raw_window_position.y / scale_factor,
            )
        },
    };

    NormalizedWindowPoint {
        window_logical,
        normalization,
    }
}

/// 物理窗口尺寸转换为高层逻辑窗口尺寸。
#[must_use]
pub fn logical_window_size_from_physical_size(
    physical_width: u32,
    physical_height: u32,
    scale_factor: f64,
) -> Size {
    let scale_factor = scale_factor.max(f64::EPSILON);
    Size::new(
        physical_width as f64 / scale_factor,
        physical_height as f64 / scale_factor,
    )
}

// ============================================================================
// Event
// ============================================================================

/// 框架统一事件类型（D5 §2.1）。
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// 鼠标按下。
    MouseDown {
        coords: MouseEventCoords,
        button: MouseButton,
        modifiers: Modifiers,
    },
    /// 鼠标抬起。
    MouseUp {
        coords: MouseEventCoords,
        button: MouseButton,
        modifiers: Modifiers,
    },
    /// 鼠标移动。
    MouseMove {
        coords: MouseEventCoords,
        delta_window_logical: Point,
        modifiers: Modifiers,
    },
    MouseWheel {
        coords: MouseEventCoords,
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
        coords: MouseEventCoords,
    },
    DragOver {
        coords: MouseEventCoords,
    },
    DragLeave,
    Drop {
        coords: MouseEventCoords,
    },
    /// 窗口逻辑尺寸变更。
    ///
    /// `width`/`height` 面向组件层消费，单位统一为逻辑像素。
    WindowResized {
        width: f64,
        height: f64,
    },
    WindowFocused,
    WindowUnfocused,
    /// 窗口 DPI 缩放因子变更。
    ///
    /// 事件仅暴露新的缩放比例；相关逻辑尺寸应通过 `WindowResized` 的逻辑像素值读取。
    ScaleFactorChanged {
        scale_factor: f64,
    },
    ThemeChanged {
        color_scheme: ColorScheme,
    },
    CloseRequested,

    /// 关闭弹层组件（WTI03：点击外部关闭）。
    ///
    /// 当命中测试未命中任何 widget，且存在弹层组件时，
    /// 框架发送此事件通知弹层执行关闭逻辑。
    Close {
        /// 目标弹层 widget ID（None 表示关闭全部弹层）。
        widget_id: Option<WidgetId>,
    },
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

    #[test]
    fn normalize_platform_window_point_returns_native_logical_on_macos() {
        let normalized = normalize_platform_window_point(Point::new(120.0, 80.0), 2.0);
        #[cfg(target_os = "macos")]
        {
            assert_eq!(
                normalized.normalization,
                CoordinateNormalization::PlatformNativeLogical
            );
            assert_eq!(normalized.window_logical, Point::new(120.0, 80.0));
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(
                normalized.normalization,
                CoordinateNormalization::DivideByScaleFactor { scale_factor: 2.0 }
            );
            assert_eq!(normalized.window_logical, Point::new(60.0, 40.0));
        }
    }

    #[test]
    fn logical_window_size_from_physical_size_divides_by_scale_factor() {
        let logical = logical_window_size_from_physical_size(800, 600, 2.0);
        assert_eq!(logical, Size::new(400.0, 300.0));
    }

    #[test]
    fn mouse_event_coords_can_attach_local_position() {
        let coords =
            MouseEventCoords::new(Point::new(20.0, 30.0), MouseInputOrigin::LogicalInjection)
                .with_local(Point::new(5.0, 6.0));
        assert_eq!(coords.window_logical, Point::new(20.0, 30.0));
        assert_eq!(coords.local_logical, Some(Point::new(5.0, 6.0)));
    }
}
