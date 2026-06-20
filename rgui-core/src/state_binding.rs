//! Rhai state binding trait — bridge between Rhai scripts and StateStore.
//!
//! Allows Rhai scripts to read and write persistent widget state
//! through `store_read(id)` / `store_write(id, value)` functions.

use crate::id::WidgetId;

/// Trait for Rhai↔StateStore state binding (RS05).
///
/// Implementations provide `store_read`/`store_write` that Rhai scripts call
/// to access persistent widget state. The implementing type is responsible for
/// dirty propagation on write.
pub trait StateBinding: Send + Sync {
    /// Read the persistent state for a widget as a string.
    fn store_read(&self, widget_id: WidgetId) -> String;

    /// Write persistent state for a widget. Triggers dirty propagation.
    fn store_write(&self, widget_id: WidgetId, value: &str);
}
