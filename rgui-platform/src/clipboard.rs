//! 剪贴板管理（D5 §8）。
//!
//! 基于 `arboard` crate 提供跨平台剪贴板的文本读写接口。
//! 目前仅支持文本格式，图片/文件等格式在后续阶段扩展。

use thiserror::Error;

// ============================================================================
// ClipboardError
// ============================================================================

/// 剪贴板操作错误。
#[derive(Debug, Error)]
pub enum ClipboardError {
    /// 无法打开系统剪贴板。
    #[error("无法打开系统剪贴板: {0}")]
    OpenFailed(String),

    /// 读取剪贴板内容失败。
    #[error("读取剪贴板失败: {0}")]
    ReadFailed(String),

    /// 写入剪贴板失败。
    #[error("写入剪贴板失败: {0}")]
    WriteFailed(String),
}

// ============================================================================
// Clipboard
// ============================================================================

/// 跨平台剪贴板管理器（D5 §8）。
///
/// 提供文本内容的获取和设置功能。
///
/// # 示例
///
/// ```no_run
/// use rgui_platform::Clipboard;
///
/// let mut cb = Clipboard::new().expect("无法初始化剪贴板");
/// cb.set_text("Hello, World!").expect("写入剪贴板失败");
/// let text = cb.get_text().expect("读取剪贴板失败");
/// assert_eq!(text, "Hello, World!");
/// ```
pub struct Clipboard {
    inner: arboard::Clipboard,
}

impl std::fmt::Debug for Clipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Clipboard").finish()
    }
}

impl Clipboard {
    /// 创建新的剪贴板管理器。
    ///
    /// 在首次调用时获取系统剪贴板句柄。
    pub fn new() -> Result<Self, ClipboardError> {
        let inner =
            arboard::Clipboard::new().map_err(|e| ClipboardError::OpenFailed(e.to_string()))?;
        Ok(Self { inner })
    }

    /// 从系统剪贴板读取文本内容。
    ///
    /// 如果剪贴板中不包含文本数据（例如包含图片），返回 `ClipboardError::ReadFailed`。
    pub fn get_text(&mut self) -> Result<String, ClipboardError> {
        self.inner
            .get_text()
            .map_err(|e| ClipboardError::ReadFailed(e.to_string()))
    }

    /// 将文本内容写入系统剪贴板。
    ///
    /// 覆盖剪贴板中的现有内容。
    pub fn set_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        self.inner
            .set_text(text)
            .map_err(|e| ClipboardError::WriteFailed(e.to_string()))
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试剪贴板初始化。
    ///
    /// 注意：此测试需要系统剪贴板可用（通常在 CI/headless 环境中不可用），
    /// 因此标记为 `ignore`。可通过 `cargo test -- --ignored` 手动运行。
    #[test]
    #[ignore]
    fn clipboard_new_succeeds() {
        let cb = Clipboard::new();
        assert!(cb.is_ok(), "应在有桌面环境的系统中成功创建剪贴板管理器");
    }

    /// 测试设置和获取文本的往返操作。
    #[test]
    #[ignore]
    fn clipboard_set_and_get_text() {
        let mut cb = Clipboard::new().expect("创建剪贴板管理器");
        let test_text = "Hello, rgui clipboard!";
        cb.set_text(test_text).expect("设置剪贴板文本");
        let result = cb.get_text().expect("获取剪贴板文本");
        assert_eq!(result, test_text);
    }

    /// 测试写入空字符串。
    #[test]
    #[ignore]
    fn clipboard_set_empty_text() {
        let mut cb = Clipboard::new().expect("创建剪贴板管理器");
        cb.set_text("").expect("设置空字符串");
        let result = cb.get_text().expect("获取剪贴板文本");
        assert_eq!(result, "");
    }

    /// 测试 Unicode 文本（中文、Emoji）。
    #[test]
    #[ignore]
    fn clipboard_set_unicode_text() {
        let mut cb = Clipboard::new().expect("创建剪贴板管理器");
        let text = "你好，世界！ 🌍 rgui 框架";
        cb.set_text(text).expect("设置 Unicode 文本");
        let result = cb.get_text().expect("获取剪贴板文本");
        assert_eq!(result, text);
    }

    /// 测试多次写入覆盖。
    #[test]
    #[ignore]
    fn clipboard_overwrite_text() {
        let mut cb = Clipboard::new().expect("创建剪贴板管理器");
        cb.set_text("first").expect("第一次写入");
        cb.set_text("second").expect("第二次写入（覆盖）");
        let result = cb.get_text().expect("获取剪贴板文本");
        assert_eq!(result, "second");
    }

    /// 测试 ClipboardError 的 Display 实现。
    #[test]
    fn clipboard_error_display() {
        let err = ClipboardError::OpenFailed("test error".to_string());
        let msg = err.to_string();
        assert!(msg.contains("test error"));
        assert!(msg.contains("无法打开系统剪贴板"));
    }

    /// 测试各方法返回正确的错误变体。
    #[test]
    fn clipboard_error_variants() {
        let open_err = ClipboardError::OpenFailed("open".to_string());
        let read_err = ClipboardError::ReadFailed("read".to_string());
        let write_err = ClipboardError::WriteFailed("write".to_string());

        assert!(matches!(open_err, ClipboardError::OpenFailed(_)));
        assert!(matches!(read_err, ClipboardError::ReadFailed(_)));
        assert!(matches!(write_err, ClipboardError::WriteFailed(_)));
    }
}
