//! 样式系统子模块（由 `rgui-style` 并入 `rgui-core`，greenfield 裁决 §F）。
//!
//! `.rgss` 解析是纯文本解析（非重型运行时）；并入 core 后走单一路径（热重载 = P1，§G）。
//!
//! ## D19 实现
//! - `StyleProperties`：样式属性（前景色/背景色/描边色/描边宽/描边 pad）。
//! - `StyleRule`：规则（selector + properties）。
//! - `StyleSheet`：样式表（规则列表 + `lookup` 命中 + 程序化 `rule` 构建）。
//! - `default_theme`：默认主题（当前各组件默认配色的权威来源；未命中样式时组件回退默认）。
//! - `parse_rgss`：**文本解析留后续（P1，不引入 cssparser）**，D19 保持占位（程序化构建经 `StyleSheet::rule` 提供）。
//!
//! `#![allow(dead_code)]` 保留（部分 API 供组件/示例/后续主题使用）。

#![allow(dead_code)]

use crate::view::Color;

/// 样式属性集合（组件从样式表取用；`None` = 未指定，组件回退默认）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleProperties {
    /// 前景/强调色（如 Accordion header、WaBadge 背景）。
    pub color: Option<Color>,
    /// 背景色。
    pub background: Option<Color>,
    /// 描边颜色（焦点/边框）。
    pub border_color: Option<Color>,
    /// 描边宽度。
    pub border_width: Option<f32>,
    /// 描边外扩 pad（D16 P2：非硬编码 2.0）。
    pub border_pad: Option<f32>,
    /// 正文/标题字号（D23：Body 13pt 阶梯语义；None = 默认正文字号）。
    pub font_size: Option<f32>,
    /// 语义前景色（文字；D23：非硬编码纯白，浅字/深字视背景）。
    pub foreground: Option<Color>,
}

impl StyleProperties {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置前景/强调色。
    pub fn color(mut self, c: Color) -> Self {
        self.color = Some(c);
        self
    }

    /// 设置背景色。
    pub fn background(mut self, c: Color) -> Self {
        self.background = Some(c);
        self
    }

    /// 设置描边（颜色 + 宽度）。
    pub fn border(mut self, color: Color, width: f32) -> Self {
        self.border_color = Some(color);
        self.border_width = Some(width);
        self
    }

    /// 设置描边 pad（外扩，非硬编码）。
    pub fn border_pad(mut self, pad: f32) -> Self {
        self.border_pad = Some(pad);
        self
    }

    /// 设置正文/标题字号（D23 正文阶梯）。
    pub fn font_size(mut self, s: f32) -> Self {
        self.font_size = Some(s);
        self
    }

    /// 设置语义前景色（文字）。
    pub fn foreground(mut self, c: Color) -> Self {
        self.foreground = Some(c);
        self
    }

    /// 取前景色（未指定回退默认）。
    pub fn effective_color(&self, default: Color) -> Color {
        self.color.unwrap_or(default)
    }

    /// 取背景色（未指定回退默认）。
    pub fn effective_background(&self, default: Color) -> Color {
        self.background.unwrap_or(default)
    }

    /// 取描边色（未指定回退默认）。
    pub fn effective_border_color(&self, default: Color) -> Color {
        self.border_color.unwrap_or(default)
    }

    /// 取描边宽（未指定回退默认）。
    pub fn effective_border_width(&self, default: f32) -> f32 {
        self.border_width.unwrap_or(default)
    }

    /// 取描边 pad（未指定回退默认）。
    pub fn effective_border_pad(&self, default: f32) -> f32 {
        self.border_pad.unwrap_or(default)
    }

    /// 取正文/标题字号（未指定回退默认 13pt Body）。
    pub fn effective_font_size(&self, default: f32) -> f32 {
        self.font_size.unwrap_or(default)
    }

    /// 取语义前景色（未指定回退默认）。
    pub fn effective_foreground(&self, default: Color) -> Color {
        self.foreground.unwrap_or(default)
    }

    /// 是否显式指定任何属性。
    pub fn is_empty(&self) -> bool {
        self.color.is_none()
            && self.background.is_none()
            && self.border_color.is_none()
            && self.border_width.is_none()
            && self.border_pad.is_none()
            && self.font_size.is_none()
            && self.foreground.is_none()
    }
}

/// 样式规则（selector + properties）。
#[derive(Debug, Clone, Default)]
pub struct StyleRule {
    /// 选择器串（组件/主题标识，如 `"rgui_accordion"`）。
    pub selector: String,
    /// 样式属性。
    pub properties: StyleProperties,
}

impl StyleRule {
    /// 构造规则。
    pub fn new(selector: impl Into<String>) -> Self {
        Self {
            selector: selector.into(),
            properties: StyleProperties::default(),
        }
    }

    /// 设置属性。
    pub fn with_properties(mut self, p: StyleProperties) -> Self {
        self.properties = p;
        self
    }
}

/// 样式表：规则集合 + 按 selector 命中。
#[derive(Debug, Clone, Default)]
pub struct StyleSheet {
    /// 规则列表。
    pub rules: Vec<StyleRule>,
}

impl StyleSheet {
    /// 空样式表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 程序化追加一条规则（命中优先级 = 列表中靠前者）。
    pub fn rule(mut self, selector: impl Into<String>, properties: StyleProperties) -> Self {
        self.rules
            .push(StyleRule::new(selector).with_properties(properties));
        self
    }

    /// 按 `selector` 命中首条匹配规则的属性；未命中返回默认（空）属性。
    pub fn lookup(&self, selector: &str) -> StyleProperties {
        self.rules
            .iter()
            .find(|r| r.selector == selector)
            .map(|r| r.properties.clone())
            .unwrap_or_default()
    }
}

/// 默认主题（D23：macOS 深色观感——控件灰 #3A3A3A 背景 + 浅前景 #E8E8E8（对比度约 9:1≥4.5）+ systemBlue accent + Body 13pt）。
pub fn default_theme() -> StyleSheet {
    StyleSheet::new()
        .rule(
            "accordion",
            StyleProperties::new()
                .background(Color::rgb(58, 58, 58)) // #3A3A3A 控件灰（非饱和亮蓝）
                .foreground(Color::rgb(232, 232, 232)) // #E8E8E8 浅前景
                .border(Color::rgb(0, 122, 255), 3.0) // systemBlue #007AFF accent（仅描边/焦点）
                .border_pad(2.0)
                .font_size(13.0), // Body 13pt
        )
        .rule(
            "wa_badge",
            StyleProperties::new()
                .background(Color::rgb(58, 58, 58))
                .foreground(Color::rgb(232, 232, 232))
                .border(Color::rgb(0, 122, 255), 3.0)
                .border_pad(2.0)
                .font_size(13.0),
        )
        .rule(
            "accent",
            StyleProperties::new()
                .foreground(Color::rgb(0, 122, 255))
                .color(Color::rgb(0, 122, 255)),
        )
}

/// 默认样式表单例（`OnceLock`）。
static DEFAULT_STYLE: std::sync::OnceLock<StyleSheet> = std::sync::OnceLock::new();

/// 取默认样式表（`&'static`，供 `ViewContext.styles`）。
pub fn default_style() -> &'static StyleSheet {
    DEFAULT_STYLE.get_or_init(default_theme)
}

/// 解析 `.rgss` 文本。D19：**文本解析留后续（P1，不引入 cssparser）**——此处保持占位返回默认主题。
/// 程序化构建经 `StyleSheet::rule` / `default_theme`。
pub fn parse_rgss(_src: &str) -> StyleSheet {
    default_theme()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_hits_first_matching_rule_and_misses_default() {
        let sheet = StyleSheet::new()
            .rule(
                "widget_a",
                StyleProperties::new().color(Color::rgb(1, 2, 3)),
            )
            .rule(
                "widget_b",
                StyleProperties::new().border(Color::rgb(9, 9, 9), 4.0),
            );
        // 命中 widget_a 的 color
        assert_eq!(sheet.lookup("widget_a").color, Some(Color::rgb(1, 2, 3)));
        // 未命中 → 默认空（is_empty）
        assert!(sheet.lookup("widget_c").is_empty());
        // widget_b 的 border
        assert_eq!(
            sheet.lookup("widget_b").border_color,
            Some(Color::rgb(9, 9, 9))
        );
        assert_eq!(sheet.lookup("widget_b").border_width, Some(4.0));
    }

    #[test]
    fn default_theme_provides_expected_components() {
        let theme = default_theme();
        let accordion = theme.lookup("accordion");
        assert_eq!(
            accordion.background,
            Some(Color::rgb(58, 58, 58)),
            "D23 控件灰"
        );
        assert_eq!(
            accordion.foreground,
            Some(Color::rgb(232, 232, 232)),
            "D23 浅前景"
        );
        assert_eq!(
            accordion.border_color,
            Some(Color::rgb(0, 122, 255)),
            "D23 systemBlue accent"
        );
        assert_eq!(accordion.border_pad, Some(2.0));
        assert_eq!(accordion.font_size, Some(13.0), "D23 Body 13pt");
        let badge = theme.lookup("wa_badge");
        assert_eq!(badge.background, Some(Color::rgb(58, 58, 58)));
        assert_eq!(badge.font_size, Some(13.0));
    }

    #[test]
    fn effective_fallbacks_to_defaults() {
        let p = StyleProperties::new();
        assert_eq!(p.effective_color(Color::rgb(1, 1, 1)), Color::rgb(1, 1, 1));
        assert_eq!(
            p.effective_background(Color::rgb(2, 2, 2)),
            Color::rgb(2, 2, 2)
        );
        assert_eq!(p.effective_border_pad(2.0), 2.0);
        assert_eq!(p.effective_border_width(3.0), 3.0);

        let q = StyleProperties::new()
            .color(Color::rgb(10, 20, 30))
            .border(Color::rgb(40, 50, 60), 5.0)
            .border_pad(6.0);
        assert_eq!(
            q.effective_color(Color::rgb(1, 1, 1)),
            Color::rgb(10, 20, 30)
        );
        assert_eq!(q.effective_border_width(1.0), 5.0);
        assert_eq!(q.effective_border_pad(1.0), 6.0);
    }
}
