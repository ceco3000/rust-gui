//! Locale type for internationalization (i18n).
//!
//! Defines the [`Locale`] struct that carries locale identifier, number format,
//! date format, and currency format information. Used by [`ViewContext`]
//! (see [`crate::context::ViewContext`]) to provide localization data to widgets.
//!
//! Design reference: D0 §5.5, D1 §6.1.

/// Locale information for internationalization (i18n).
///
/// Each `Locale` provides the formatting conventions for a specific language-region
/// combination. Widgets use this to render numbers, dates, and currencies correctly.
///
/// # Examples
///
/// ```
/// use rgui_core::locale::Locale;
///
/// let en = Locale::EN_US;
/// assert_eq!(en.id, "en-US");
/// assert_eq!(en.decimal_separator, '.');
/// assert_eq!(en.currency_symbol, "$");
///
/// let zh = Locale::ZH_CN;
/// assert_eq!(zh.id, "zh-CN");
/// assert_eq!(zh.currency_symbol, "¥");
///
/// assert_eq!(Locale::default().id, "en-US");
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Locale {
    /// BCP-47 locale identifier (e.g., `"en-US"`, `"zh-CN"`, `"ja-JP"`).
    pub id: &'static str,
    /// Decimal separator character (e.g., `'.'` for en-US, `','` for fr-FR).
    pub decimal_separator: char,
    /// Thousands grouping separator (e.g., `','` for en-US, `'.'` for de-DE).
    pub thousands_separator: char,
    /// Date format pattern (strftime-style, e.g., `"%m/%d/%Y"` for en-US).
    pub date_format: &'static str,
    /// Currency symbol (e.g., `"$"`, `"¥"`, `"€"`).
    pub currency_symbol: &'static str,
}

impl Locale {
    /// American English (en-US).
    pub const EN_US: &'static Locale = &Locale {
        id: "en-US",
        decimal_separator: '.',
        thousands_separator: ',',
        date_format: "%m/%d/%Y",
        currency_symbol: "$",
    };

    /// Simplified Chinese (zh-CN).
    pub const ZH_CN: &'static Locale = &Locale {
        id: "zh-CN",
        decimal_separator: '.',
        thousands_separator: ',',
        date_format: "%Y-%m-%d",
        currency_symbol: "¥",
    };

    /// Returns the default locale (`EN_US`).
    #[must_use]
    pub const fn default() -> &'static Locale {
        Self::EN_US
    }
}

impl Default for &'static Locale {
    fn default() -> Self {
        Locale::EN_US
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn en_us_locale() {
        let l = Locale::EN_US;
        assert_eq!(l.id, "en-US");
        assert_eq!(l.decimal_separator, '.');
        assert_eq!(l.thousands_separator, ',');
        assert_eq!(l.date_format, "%m/%d/%Y");
        assert_eq!(l.currency_symbol, "$");
    }

    #[test]
    fn zh_cn_locale() {
        let l = Locale::ZH_CN;
        assert_eq!(l.id, "zh-CN");
        assert_eq!(l.decimal_separator, '.');
        assert_eq!(l.thousands_separator, ',');
        assert_eq!(l.date_format, "%Y-%m-%d");
        assert_eq!(l.currency_symbol, "¥");
    }

    #[test]
    fn default_locale_is_en_us() {
        assert_eq!(Locale::default().id, "en-US");
    }

    #[test]
    fn locale_eq() {
        assert_eq!(Locale::EN_US, Locale::EN_US);
        assert_ne!(Locale::EN_US, Locale::ZH_CN);
    }

    #[test]
    fn locale_clone() {
        let l = Locale::EN_US.clone();
        assert_eq!(l, *Locale::EN_US);
    }

    #[test]
    fn locale_debug() {
        let dbg = format!("{:?}", Locale::EN_US);
        assert!(dbg.contains("en-US"));
    }

    #[test]
    fn static_ref_default() {
        let l: &'static Locale = Default::default();
        assert_eq!(l.id, "en-US");
    }
}
