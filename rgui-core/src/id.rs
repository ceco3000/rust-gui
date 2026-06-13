//! 标识符类型——WidgetId 和 WindowId。
//!
//! ## 设计约束（D0 §7 不变式 6）
//!
//! - `WidgetId` 在运行时全局唯一，不可复用
//! - `WidgetId` 通过原子计数器分配，永不回收
//!
//! ## 设计约束（D0 §5.2）
//!
//! - `WindowId` 是窗口句柄的轻量包装

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// 全局 WidgetId 分配器。
///
/// 使用原子计数器保证线程安全。计数器永不重置，
/// 确保已删除 widget 的 ID 不会被重新分配。
static NEXT_WIDGET_ID: AtomicU64 = AtomicU64::new(1);

/// Widget 的唯一标识符。
///
/// 在运行时全局唯一，不可复用。由 [`WidgetId::new()`]
/// 通过原子计数器分配。
///
/// # 示例
///
/// ```
/// use rgui_core::id::WidgetId;
///
/// let id1 = WidgetId::new();
/// let id2 = WidgetId::new();
/// assert_ne!(id1, id2);
/// ```
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetId(u64);

impl WidgetId {
    /// 分配一个新的唯一 WidgetId。
    #[must_use]
    pub fn new() -> Self {
        // Relaxed 即可：只需保证唯一性，不需要 happens-before 关系
        let id = NEXT_WIDGET_ID.fetch_add(1, Ordering::Relaxed);
        Self(id)
    }

    /// 从已有的 u64 值构造 WidgetId（用于反序列化/快照恢复）。
    ///
    /// 调用者必须保证传入的 id 在运行时内唯一。
    #[must_use]
    pub const fn from_u64(id: u64) -> Self {
        Self(id)
    }

    /// 返回内部的 u64 表示。
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl Default for WidgetId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WidgetId({})", self.0)
    }
}

impl fmt::Display for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "widget#{}", self.0)
    }
}

/// 窗口的唯一标识符。
///
/// 对平台窗口句柄的轻量包装。由 `rgui-platform` 在窗口创建时分配，
/// `rgui-core` 仅定义类型。
///
/// # 示例
///
/// ```
/// use rgui_core::id::WindowId;
///
/// let win1 = WindowId::new();
/// let win2 = WindowId::new();
/// assert_ne!(win1, win2);
/// ```
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(u64);

impl WindowId {
    /// 分配一个新的唯一 WindowId。
    #[must_use]
    pub fn new() -> Self {
        static NEXT_WINDOW_ID: AtomicU64 = AtomicU64::new(1);
        let id = NEXT_WINDOW_ID.fetch_add(1, Ordering::Relaxed);
        Self(id)
    }

    /// 从已有的 u64 值构造 WindowId。
    #[must_use]
    pub const fn from_u64(id: u64) -> Self {
        Self(id)
    }

    /// 返回内部的 u64 表示。
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl Default for WindowId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WindowId({})", self.0)
    }
}

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "window#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn widget_id_new_generates_unique_values() {
        let mut seen = HashSet::new();
        for _ in 0..1000 {
            let id = WidgetId::new();
            assert!(seen.insert(id), "WidgetId 应该全局唯一");
        }
    }

    #[test]
    fn widget_id_supports_hash() {
        let id1 = WidgetId::from_u64(42);
        let id2 = WidgetId::from_u64(42);
        let mut set = HashSet::new();
        set.insert(id1);
        assert!(set.contains(&id2));
    }

    #[test]
    fn widget_id_debug_format() {
        let id = WidgetId::from_u64(7);
        assert_eq!(format!("{id:?}"), "WidgetId(7)");
    }

    #[test]
    fn widget_id_display_format() {
        let id = WidgetId::from_u64(7);
        assert_eq!(format!("{id}"), "widget#7");
    }

    #[test]
    fn widget_id_clone_is_equal() {
        let id = WidgetId::new();
        assert_eq!(id, id.clone());
    }

    #[test]
    fn widget_id_ordering() {
        let id1 = WidgetId::from_u64(1);
        let id2 = WidgetId::from_u64(2);
        assert!(id1 < id2);
    }

    #[test]
    fn window_id_new_generates_unique_values() {
        let w1 = WindowId::new();
        let w2 = WindowId::new();
        assert_ne!(w1, w2);
    }

    #[test]
    fn window_id_from_u64_roundtrip() {
        let w = WindowId::from_u64(100);
        assert_eq!(w.as_u64(), 100);
    }

    #[test]
    fn window_id_display_format() {
        let w = WindowId::from_u64(3);
        assert_eq!(format!("{w}"), "window#3");
    }

    #[test]
    fn widget_id_from_u64_roundtrip() {
        let id = WidgetId::from_u64(42);
        assert_eq!(id.as_u64(), 42);
    }
}
