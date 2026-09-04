//! 本地化（i18n）占位模块。

/// 本地化标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Locale {
    /// 语言代码（如 "zh-CN"）。
    pub language: &'static str,
}

impl Locale {
    /// 默认系统 locale 占位。
    pub const fn system() -> Self {
        Self { language: "en-US" }
    }
}

impl Default for Locale {
    fn default() -> Self {
        Self::system()
    }
}
