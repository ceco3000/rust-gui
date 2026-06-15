//! CSS 属性映射表——将 .rgss 解析出的属性键值对转换为类型化的样式结构体。
//!
//! 本模块实现 D4 §3 定义的 38 个 CSS 属性的完整映射表，
//! 包括属性名验证、值类型转换和分类输出。

use rgui_core::geometry::{
    AlignItems, AlignSelf, FlexBasis, FlexDirection, FlexWrap, GridTrack, JustifyContent,
    LayoutDisplay, LayoutStyle, TextAlign, TextOverflow, TextStyle, Visibility, VisualStyle,
    WhiteSpace,
};
use rgui_core::view::{Color, PropValue};
use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock};

// ============================================================================
// 属性分类
// ============================================================================

/// CSS 属性分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyCategory {
    /// 布局属性（→ Taffy）
    Layout,
    /// 视觉属性（→ DrawCommand）
    Visual,
    /// 文本属性（→ cosmic-text）
    Text,
    /// 尺寸与间距属性
    SizeSpacing,
}

/// 属性元数据：分类 + 合法值描述（用于错误报告）。
#[derive(Debug, Clone)]
pub struct PropertyMeta {
    /// 属性类别。
    pub category: PropertyCategory,
    /// 人类可读的合法值描述。
    pub valid_values: &'static str,
}

impl PropertyMeta {
    const fn new(category: PropertyCategory, valid_values: &'static str) -> Self {
        Self {
            category,
            valid_values,
        }
    }
}

// ============================================================================
// CSS 属性名常量（D4 §3 定义的 38 个属性）
// ============================================================================

/// CSS `display` 属性名。
pub const PROP_DISPLAY: &str = "display";
/// CSS `flex-direction` 属性名。
pub const PROP_FLEX_DIRECTION: &str = "flex-direction";
/// CSS `flex-wrap` 属性名。
pub const PROP_FLEX_WRAP: &str = "flex-wrap";
/// CSS `justify-content` 属性名。
pub const PROP_JUSTIFY_CONTENT: &str = "justify-content";
/// CSS `align-items` 属性名。
pub const PROP_ALIGN_ITEMS: &str = "align-items";
/// CSS `align-self` 属性名。
pub const PROP_ALIGN_SELF: &str = "align-self";
/// CSS `gap` 属性名。
pub const PROP_GAP: &str = "gap";
/// CSS `flex-grow` 属性名。
pub const PROP_FLEX_GROW: &str = "flex-grow";
/// CSS `flex-shrink` 属性名。
pub const PROP_FLEX_SHRINK: &str = "flex-shrink";
/// CSS `flex-basis` 属性名。
pub const PROP_FLEX_BASIS: &str = "flex-basis";
/// CSS `grid-template-columns` 属性名。
pub const PROP_GRID_TEMPLATE_COLUMNS: &str = "grid-template-columns";
/// CSS `grid-template-rows` 属性名。
pub const PROP_GRID_TEMPLATE_ROWS: &str = "grid-template-rows";

/// CSS `background-color` 属性名。
pub const PROP_BACKGROUND_COLOR: &str = "background-color";
/// CSS `color` 属性名。
pub const PROP_COLOR: &str = "color";
/// CSS `opacity` 属性名。
pub const PROP_OPACITY: &str = "opacity";
/// CSS `border-radius` 属性名。
pub const PROP_BORDER_RADIUS: &str = "border-radius";
/// CSS `border-width` 属性名。
pub const PROP_BORDER_WIDTH: &str = "border-width";
/// CSS `border-color` 属性名。
pub const PROP_BORDER_COLOR: &str = "border-color";
/// CSS `box-shadow` 属性名。
pub const PROP_BOX_SHADOW: &str = "box-shadow";
/// CSS `visibility` 属性名。
pub const PROP_VISIBILITY: &str = "visibility";

/// CSS `font-family` 属性名。
pub const PROP_FONT_FAMILY: &str = "font-family";
/// CSS `font-size` 属性名。
pub const PROP_FONT_SIZE: &str = "font-size";
/// CSS `font-weight` 属性名。
pub const PROP_FONT_WEIGHT: &str = "font-weight";
/// CSS `font-style` 属性名。
pub const PROP_FONT_STYLE: &str = "font-style";
/// CSS `line-height` 属性名。
pub const PROP_LINE_HEIGHT: &str = "line-height";
/// CSS `letter-spacing` 属性名。
pub const PROP_LETTER_SPACING: &str = "letter-spacing";
/// CSS `text-align` 属性名。
pub const PROP_TEXT_ALIGN: &str = "text-align";
/// CSS `text-overflow` 属性名。
pub const PROP_TEXT_OVERFLOW: &str = "text-overflow";
/// CSS `white-space` 属性名。
pub const PROP_WHITE_SPACE: &str = "white-space";

/// CSS `width` 属性名。
pub const PROP_WIDTH: &str = "width";
/// CSS `height` 属性名。
pub const PROP_HEIGHT: &str = "height";
/// CSS `min-width` 属性名。
pub const PROP_MIN_WIDTH: &str = "min-width";
/// CSS `max-width` 属性名。
pub const PROP_MAX_WIDTH: &str = "max-width";
/// CSS `min-height` 属性名。
pub const PROP_MIN_HEIGHT: &str = "min-height";
/// CSS `max-height` 属性名。
pub const PROP_MAX_HEIGHT: &str = "max-height";
/// CSS `padding` 属性名。
pub const PROP_PADDING: &str = "padding";
/// CSS `margin` 属性名。
pub const PROP_MARGIN: &str = "margin";
/// CSS `aspect-ratio` 属性名。
pub const PROP_ASPECT_RATIO: &str = "aspect-ratio";

// ============================================================================
// 属性元数据表
// ============================================================================

/// D4 §3 定义的 38 个 CSS 属性的元数据查找表。
///
/// 使用 [`LazyLock`] 确保仅在首次访问时构造一次，后续查询直接返回缓存引用。
static META_TABLE: LazyLock<BTreeMap<&'static str, PropertyMeta>> = LazyLock::new(|| {
    use PropertyCategory::*;

    let mut table = BTreeMap::new();

    // --- 布局属性（12 个）---
    table.insert(
        PROP_DISPLAY,
        PropertyMeta::new(Layout, "flex | grid | block | none"),
    );
    table.insert(
        PROP_FLEX_DIRECTION,
        PropertyMeta::new(Layout, "row | column | row-reverse | column-reverse"),
    );
    table.insert(
        PROP_FLEX_WRAP,
        PropertyMeta::new(Layout, "nowrap | wrap | wrap-reverse"),
    );
    table.insert(
        PROP_JUSTIFY_CONTENT,
        PropertyMeta::new(
            Layout,
            "start | end | center | space-between | space-around | space-evenly",
        ),
    );
    table.insert(
        PROP_ALIGN_ITEMS,
        PropertyMeta::new(Layout, "start | end | center | stretch | baseline"),
    );
    table.insert(
        PROP_ALIGN_SELF,
        PropertyMeta::new(Layout, "auto | start | end | center | stretch | baseline"),
    );
    table.insert(PROP_GAP, PropertyMeta::new(Layout, "长度值（如 12px）"));
    table.insert(
        PROP_FLEX_GROW,
        PropertyMeta::new(Layout, "非负数值（如 1）"),
    );
    table.insert(
        PROP_FLEX_SHRINK,
        PropertyMeta::new(Layout, "非负数值（如 0）"),
    );
    table.insert(
        PROP_FLEX_BASIS,
        PropertyMeta::new(Layout, "auto | 长度值（如 200px）"),
    );
    table.insert(
        PROP_GRID_TEMPLATE_COLUMNS,
        PropertyMeta::new(Layout, "轨道列表（如 100px 1fr 2fr）"),
    );
    table.insert(
        PROP_GRID_TEMPLATE_ROWS,
        PropertyMeta::new(Layout, "轨道列表（如 auto 1fr）"),
    );

    // --- 视觉属性（8 个）---
    table.insert(
        PROP_BACKGROUND_COLOR,
        PropertyMeta::new(Visual, "颜色值（#RRGGBB）"),
    );
    table.insert(PROP_COLOR, PropertyMeta::new(Visual, "颜色值（#RRGGBB）"));
    table.insert(PROP_OPACITY, PropertyMeta::new(Visual, "0.0-1.0 的浮点数"));
    table.insert(
        PROP_BORDER_RADIUS,
        PropertyMeta::new(Visual, "长度值（如 6px）"),
    );
    table.insert(
        PROP_BORDER_WIDTH,
        PropertyMeta::new(Visual, "长度值（如 1px）"),
    );
    table.insert(
        PROP_BORDER_COLOR,
        PropertyMeta::new(Visual, "颜色值（#RRGGBB）"),
    );
    table.insert(PROP_BOX_SHADOW, PropertyMeta::new(Visual, "x y blur color"));
    table.insert(
        PROP_VISIBILITY,
        PropertyMeta::new(Visual, "visible | hidden"),
    );

    // --- 文本属性（9 个）---
    table.insert(
        PROP_FONT_FAMILY,
        PropertyMeta::new(Text, "字体族名称（如 \"Inter\", sans-serif）"),
    );
    table.insert(PROP_FONT_SIZE, PropertyMeta::new(Text, "长度值（如 14px）"));
    table.insert(
        PROP_FONT_WEIGHT,
        PropertyMeta::new(Text, "400 | bold | normal"),
    );
    table.insert(
        PROP_FONT_STYLE,
        PropertyMeta::new(Text, "normal | italic | oblique"),
    );
    table.insert(
        PROP_LINE_HEIGHT,
        PropertyMeta::new(Text, "倍数（如 1.5）或长度（如 24px）"),
    );
    table.insert(
        PROP_LETTER_SPACING,
        PropertyMeta::new(Text, "长度值（如 0.5px）"),
    );
    table.insert(
        PROP_TEXT_ALIGN,
        PropertyMeta::new(Text, "start | center | end | justify"),
    );
    table.insert(
        PROP_TEXT_OVERFLOW,
        PropertyMeta::new(Text, "clip | ellipsis"),
    );
    table.insert(
        PROP_WHITE_SPACE,
        PropertyMeta::new(Text, "normal | nowrap | pre"),
    );

    // --- 尺寸/间距属性（9 个）---
    table.insert(
        PROP_WIDTH,
        PropertyMeta::new(SizeSpacing, "auto | 长度值（如 100px）| 百分比（100%）"),
    );
    table.insert(
        PROP_HEIGHT,
        PropertyMeta::new(SizeSpacing, "auto | 长度值（如 100px）| 百分比（100%）"),
    );
    table.insert(
        PROP_MIN_WIDTH,
        PropertyMeta::new(SizeSpacing, "长度值（如 50px）"),
    );
    table.insert(
        PROP_MAX_WIDTH,
        PropertyMeta::new(SizeSpacing, "长度值（如 1200px）"),
    );
    table.insert(
        PROP_MIN_HEIGHT,
        PropertyMeta::new(SizeSpacing, "长度值（如 50px）"),
    );
    table.insert(
        PROP_MAX_HEIGHT,
        PropertyMeta::new(SizeSpacing, "长度值（如 800px）"),
    );
    table.insert(
        PROP_PADDING,
        PropertyMeta::new(SizeSpacing, "长度值（如 8px 16px）"),
    );
    table.insert(
        PROP_MARGIN,
        PropertyMeta::new(SizeSpacing, "长度值（如 8px 16px）"),
    );
    table.insert(
        PROP_ASPECT_RATIO,
        PropertyMeta::new(SizeSpacing, "比例（如 16/9）"),
    );

    table
});

/// 返回全部 38 个 D4 属性的元数据查找表的引用。
///
/// 通过 [`LazyLock`] 在首次调用时构造并缓存，后续调用直接返回同一静态实例的引用。
#[must_use]
pub fn property_meta_table() -> &'static BTreeMap<&'static str, PropertyMeta> {
    &META_TABLE
}

/// 判断一个 CSS 属性名是否是 D4 定义的有效属性。
#[must_use]
pub fn is_valid_property(name: &str) -> bool {
    META_TABLE.contains_key(name)
}

/// 获取指定属性的分类，未知属性返回 `None`。
#[must_use]
pub fn category_of(name: &str) -> Option<PropertyCategory> {
    META_TABLE.get(name).map(|m| m.category)
}

// ============================================================================
// PropValue → 类型化值 转换函数
// ============================================================================

/// 从 PropValue 中提取字符串值。
fn value_as_str(v: &PropValue) -> Option<&str> {
    match v {
        PropValue::Str(s) => Some(s.as_ref()),
        PropValue::Enum(s) => Some(s.as_ref()),
        _ => None,
    }
}

/// 从 PropValue 中提取浮点数值。
fn value_as_f64(v: &PropValue) -> Option<f64> {
    match v {
        PropValue::Float(f) => Some(f.0),
        PropValue::Int(i) => Some(*i as f64),
        _ => None,
    }
}

/// 从 PropValue 中提取颜色值。
fn value_as_color(v: &PropValue) -> Option<Color> {
    match v {
        PropValue::Color(c) => Some(*c),
        _ => None,
    }
}

/// 从 PropValue 中提取列表值。
fn value_as_list(v: &PropValue) -> Option<&[PropValue]> {
    match v {
        PropValue::List(items) => Some(items),
        _ => None,
    }
}

// ------------------------------------------------------------------
// 布局属性解析
// ------------------------------------------------------------------

fn resolve_display(v: &PropValue) -> Option<LayoutDisplay> {
    let s = value_as_str(v)?;
    match s {
        "flex" => Some(LayoutDisplay::Flex),
        "grid" => Some(LayoutDisplay::Grid),
        "block" => Some(LayoutDisplay::Block),
        "none" => Some(LayoutDisplay::None),
        _ => None,
    }
}

fn resolve_flex_direction(v: &PropValue) -> Option<FlexDirection> {
    let s = value_as_str(v)?;
    match s {
        "row" => Some(FlexDirection::Row),
        "row-reverse" => Some(FlexDirection::RowReverse),
        "column" => Some(FlexDirection::Column),
        "column-reverse" => Some(FlexDirection::ColumnReverse),
        _ => None,
    }
}

fn resolve_flex_wrap(v: &PropValue) -> Option<FlexWrap> {
    let s = value_as_str(v)?;
    match s {
        "nowrap" => Some(FlexWrap::NoWrap),
        "wrap" => Some(FlexWrap::Wrap),
        "wrap-reverse" => Some(FlexWrap::WrapReverse),
        _ => None,
    }
}

fn resolve_justify_content(v: &PropValue) -> Option<JustifyContent> {
    let s = value_as_str(v)?;
    match s {
        "start" | "flex-start" => Some(JustifyContent::Start),
        "end" | "flex-end" => Some(JustifyContent::End),
        "center" => Some(JustifyContent::Center),
        "space-between" => Some(JustifyContent::SpaceBetween),
        "space-around" => Some(JustifyContent::SpaceAround),
        "space-evenly" => Some(JustifyContent::SpaceEvenly),
        _ => None,
    }
}

fn resolve_align_items(v: &PropValue) -> Option<AlignItems> {
    let s = value_as_str(v)?;
    match s {
        "start" | "flex-start" => Some(AlignItems::Start),
        "end" | "flex-end" => Some(AlignItems::End),
        "center" => Some(AlignItems::Center),
        "baseline" => Some(AlignItems::Baseline),
        "stretch" => Some(AlignItems::Stretch),
        _ => None,
    }
}

fn resolve_align_self(v: &PropValue) -> Option<AlignSelf> {
    let s = value_as_str(v)?;
    match s {
        "auto" => Some(AlignSelf::Auto),
        "start" | "flex-start" => Some(AlignSelf::Start),
        "end" | "flex-end" => Some(AlignSelf::End),
        "center" => Some(AlignSelf::Center),
        "baseline" => Some(AlignSelf::Baseline),
        "stretch" => Some(AlignSelf::Stretch),
        _ => None,
    }
}

fn resolve_flex_basis(v: &PropValue) -> Option<FlexBasis> {
    if let Some(s) = value_as_str(v) {
        if s == "auto" {
            return Some(FlexBasis::Auto);
        }
    }
    if let Some(n) = value_as_f64(v) {
        return Some(FlexBasis::Length(n));
    }
    None
}

fn resolve_grid_tracks(v: &PropValue) -> Option<Vec<GridTrack>> {
    let items = value_as_list(v)?;
    let tracks: Vec<GridTrack> = items
        .iter()
        .filter_map(|item| {
            if let Some(s) = value_as_str(item) {
                if s == "auto" {
                    return Some(GridTrack::Auto);
                }
                if let Some(stripped) = s.strip_suffix("fr") {
                    return stripped.trim().parse::<f64>().ok().map(GridTrack::Fr);
                }
                if let Some(stripped) = s.strip_suffix("px") {
                    return stripped.trim().parse::<f64>().ok().map(GridTrack::Px);
                }
                if let Some(stripped) = s.strip_suffix('%') {
                    return stripped.trim().parse::<f64>().ok().map(GridTrack::Percent);
                }
            }
            value_as_f64(item).map(GridTrack::Px)
        })
        .collect();
    if tracks.is_empty() {
        None
    } else {
        Some(tracks)
    }
}

// ------------------------------------------------------------------
// 视觉属性解析
// ------------------------------------------------------------------

fn resolve_visibility(v: &PropValue) -> Option<Visibility> {
    let s = value_as_str(v)?;
    match s {
        "visible" => Some(Visibility::Visible),
        "hidden" => Some(Visibility::Hidden),
        _ => None,
    }
}

fn resolve_box_shadow(v: &PropValue) -> Option<(f64, f64, f64, Color)> {
    let items = value_as_list(v)?;
    if items.len() >= 4 {
        let x = value_as_f64(&items[0])?;
        let y = value_as_f64(&items[1])?;
        let blur = value_as_f64(&items[2])?;
        let color = value_as_color(&items[3])?;
        Some((x, y, blur, color))
    } else {
        None
    }
}

// ------------------------------------------------------------------
// 文本属性解析
// ------------------------------------------------------------------

fn resolve_font_weight(v: &PropValue) -> Option<rgui_core::geometry::FontWeight> {
    use rgui_core::geometry::FontWeight;
    match v {
        PropValue::Str(s) => match s.as_ref() {
            "normal" => Some(FontWeight::Normal),
            "bold" => Some(FontWeight::Bold),
            "thin" => Some(FontWeight::Thin),
            "light" => Some(FontWeight::Light),
            "medium" => Some(FontWeight::Medium),
            _ => s.parse::<u16>().ok().map(FontWeight::Number),
        },
        PropValue::Int(i) => Some(FontWeight::Number((*i).clamp(1, 1000) as u16)),
        _ => None,
    }
}

fn resolve_font_style(v: &PropValue) -> Option<rgui_core::geometry::FontStyle> {
    use rgui_core::geometry::FontStyle;
    let s = value_as_str(v)?;
    match s {
        "normal" => Some(FontStyle::Normal),
        "italic" => Some(FontStyle::Italic),
        "oblique" => Some(FontStyle::Oblique),
        _ => None,
    }
}

fn resolve_text_align(v: &PropValue) -> Option<TextAlign> {
    let s = value_as_str(v)?;
    match s {
        "start" | "left" => Some(TextAlign::Start),
        "center" => Some(TextAlign::Center),
        "end" | "right" => Some(TextAlign::End),
        "justify" => Some(TextAlign::Justify),
        _ => None,
    }
}

fn resolve_text_overflow(v: &PropValue) -> Option<TextOverflow> {
    let s = value_as_str(v)?;
    match s {
        "clip" => Some(TextOverflow::Clip),
        "ellipsis" => Some(TextOverflow::Ellipsis),
        _ => None,
    }
}

fn resolve_white_space(v: &PropValue) -> Option<WhiteSpace> {
    let s = value_as_str(v)?;
    match s {
        "normal" => Some(WhiteSpace::Normal),
        "nowrap" => Some(WhiteSpace::NoWrap),
        "pre" => Some(WhiteSpace::Pre),
        _ => None,
    }
}

/// 将包含 `"16/9"` 格式字符串的 PropValue 解析为 f64 宽高比。
fn resolve_aspect_ratio_value(v: &PropValue) -> Option<f64> {
    if let Some(s) = value_as_str(v) {
        if let Some((w_str, h_str)) = s.split_once('/') {
            let w: f64 = w_str.trim().parse().ok()?;
            let h: f64 = h_str.trim().parse().ok()?;
            if h != 0.0 {
                return Some(w / h);
            }
        }
    }
    value_as_f64(v)
}

// ============================================================================
// 主解析函数：BTreeMap<Arc<str>, PropValue> → (LayoutStyle, VisualStyle, TextStyle)
// ============================================================================

/// 属性解析结果。
#[derive(Debug, Clone, Default)]
pub struct ResolvedStyles {
    /// 解析后的布局样式。
    pub layout: LayoutStyle,
    /// 解析后的视觉样式。
    pub visual: VisualStyle,
    /// 解析后的文本样式。
    pub text: TextStyle,
    /// 未能识别的属性名列表（用于诊断）。
    pub unknown_properties: Vec<String>,
}

impl ResolvedStyles {
    /// 创建空的解析结果。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 判断是否所有样式均为空且无未知属性。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layout.is_empty()
            && self.visual.is_empty()
            && self.text.is_empty()
            && self.unknown_properties.is_empty()
    }
}

/// 将 `.rgss` 解析器输出的属性键值对 `BTreeMap<Arc<str>, PropValue>` 转换为
/// 类型化的 `LayoutStyle`、`VisualStyle` 和 `TextStyle` 三元组。
///
/// 未识别的属性名收集到 `ResolvedStyles::unknown_properties` 中（不报错误——
/// 调用方可根据 `unknown_properties` 决定是否发出警告）。
///
/// # 示例
///
/// ```rust
/// use rgui_core::view::PropValue;
/// use rgui_core::geometry::LayoutDisplay;
/// use rgui_style::property_map::resolve_properties;
/// use std::collections::BTreeMap;
/// use std::sync::Arc;
///
/// let mut props: BTreeMap<Arc<str>, PropValue> = BTreeMap::new();
/// props.insert(Arc::from("display"), PropValue::str("flex"));
/// props.insert(Arc::from("width"), PropValue::Float(ordered_float::OrderedFloat(200.0)));
///
/// let resolved = resolve_properties(&props);
/// assert_eq!(resolved.layout.display, Some(LayoutDisplay::Flex));
/// assert_eq!(resolved.layout.width, Some(200.0));
/// ```
#[must_use]
pub fn resolve_properties(props: &BTreeMap<Arc<str>, PropValue>) -> ResolvedStyles {
    let mut result = ResolvedStyles::new();

    for (key, value) in props {
        apply_property(&mut result, key, value);
    }

    result
}

/// 将单个属性键值对应用到 `ResolvedStyles`。
fn apply_property(result: &mut ResolvedStyles, key: &str, value: &PropValue) {
    match key {
        // --- 布局属性 ---
        PROP_DISPLAY => {
            result.layout.display = resolve_display(value);
        },
        PROP_FLEX_DIRECTION => {
            result.layout.flex_direction = resolve_flex_direction(value);
        },
        PROP_FLEX_WRAP => {
            result.layout.flex_wrap = resolve_flex_wrap(value);
        },
        PROP_JUSTIFY_CONTENT => {
            result.layout.justify_content = resolve_justify_content(value);
        },
        PROP_ALIGN_ITEMS => {
            result.layout.align_items = resolve_align_items(value);
        },
        PROP_ALIGN_SELF => {
            result.layout.align_self = resolve_align_self(value);
        },
        PROP_GAP => {
            result.layout.gap = value_as_f64(value);
        },
        PROP_FLEX_GROW => {
            result.layout.flex_grow = value_as_f64(value).map(|v| v as f32);
        },
        PROP_FLEX_SHRINK => {
            result.layout.flex_shrink = value_as_f64(value).map(|v| v as f32);
        },
        PROP_FLEX_BASIS => {
            result.layout.flex_basis = resolve_flex_basis(value);
        },
        PROP_GRID_TEMPLATE_COLUMNS => {
            result.layout.grid_template_columns = resolve_grid_tracks(value);
        },
        PROP_GRID_TEMPLATE_ROWS => {
            result.layout.grid_template_rows = resolve_grid_tracks(value);
        },
        // --- 视觉属性 ---
        PROP_BACKGROUND_COLOR => {
            result.visual.background_color = value_as_color(value);
        },
        PROP_COLOR => {
            result.visual.color = value_as_color(value);
        },
        PROP_OPACITY => {
            result.visual.opacity = value_as_f64(value);
        },
        PROP_BORDER_RADIUS => {
            result.visual.border_radius = value_as_f64(value);
        },
        PROP_BORDER_WIDTH => {
            result.visual.border_width = value_as_f64(value);
        },
        PROP_BORDER_COLOR => {
            result.visual.border_color = value_as_color(value);
        },
        PROP_BOX_SHADOW => {
            result.visual.box_shadow = resolve_box_shadow(value);
        },
        PROP_VISIBILITY => {
            result.visual.visibility = resolve_visibility(value);
        },
        // --- 文本属性 ---
        PROP_FONT_FAMILY => {
            if let Some(s) = value_as_str(value) {
                result.text.font_family = Some(s.to_string());
            }
        },
        PROP_FONT_SIZE => {
            result.text.font_size = value_as_f64(value);
        },
        PROP_FONT_WEIGHT => {
            result.text.font_weight = resolve_font_weight(value);
        },
        PROP_FONT_STYLE => {
            result.text.font_style = resolve_font_style(value);
        },
        PROP_LINE_HEIGHT => {
            result.text.line_height = value_as_f64(value);
        },
        PROP_LETTER_SPACING => {
            result.text.letter_spacing = value_as_f64(value);
        },
        PROP_TEXT_ALIGN => {
            result.text.text_align = resolve_text_align(value);
        },
        PROP_TEXT_OVERFLOW => {
            result.text.text_overflow = resolve_text_overflow(value);
        },
        PROP_WHITE_SPACE => {
            result.text.white_space = resolve_white_space(value);
        },
        // --- 尺寸/间距属性 ---
        PROP_WIDTH => {
            result.layout.width = value_as_f64(value);
        },
        PROP_HEIGHT => {
            result.layout.height = value_as_f64(value);
        },
        PROP_MIN_WIDTH => {
            result.layout.min_width = value_as_f64(value);
        },
        PROP_MAX_WIDTH => {
            result.layout.max_width = value_as_f64(value);
        },
        PROP_MIN_HEIGHT => {
            result.layout.min_height = value_as_f64(value);
        },
        PROP_MAX_HEIGHT => {
            result.layout.max_height = value_as_f64(value);
        },
        PROP_PADDING => {
            result.layout.padding = value_as_f64(value);
        },
        PROP_MARGIN => {
            result.layout.margin = value_as_f64(value);
        },
        PROP_ASPECT_RATIO => {
            result.layout.aspect_ratio = resolve_aspect_ratio_value(value);
        },
        // 未知属性
        _ => {
            result.unknown_properties.push(key.to_string());
        },
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ordered_float::OrderedFloat;
    use rgui_core::geometry::{FontStyle, FontWeight, TextAlign, TextOverflow, WhiteSpace};
    use std::collections::BTreeMap;

    /// 构造测试用 PropValue 辅助函数。
    fn prop_str(s: &str) -> PropValue {
        PropValue::str(s)
    }

    fn prop_f64(n: f64) -> PropValue {
        PropValue::Float(OrderedFloat(n))
    }

    fn prop_int(n: i64) -> PropValue {
        PropValue::Int(n)
    }

    fn prop_color(r: f64, g: f64, b: f64, a: f64) -> PropValue {
        PropValue::Color(Color::new(r, g, b, a))
    }

    fn props_builder() -> BTreeMap<Arc<str>, PropValue> {
        BTreeMap::new()
    }

    // ------------------------------------------------------------------
    // 属性元数据表测试
    // ------------------------------------------------------------------

    #[test]
    fn meta_table_has_38_properties() {
        let table = property_meta_table();
        assert_eq!(table.len(), 38, "D4 定义的属性总数应为 38");
    }

    #[test]
    fn all_d4_properties_are_valid() {
        let all_names = vec![
            PROP_DISPLAY,
            PROP_FLEX_DIRECTION,
            PROP_FLEX_WRAP,
            PROP_JUSTIFY_CONTENT,
            PROP_ALIGN_ITEMS,
            PROP_ALIGN_SELF,
            PROP_GAP,
            PROP_FLEX_GROW,
            PROP_FLEX_SHRINK,
            PROP_FLEX_BASIS,
            PROP_GRID_TEMPLATE_COLUMNS,
            PROP_GRID_TEMPLATE_ROWS,
            PROP_BACKGROUND_COLOR,
            PROP_COLOR,
            PROP_OPACITY,
            PROP_BORDER_RADIUS,
            PROP_BORDER_WIDTH,
            PROP_BORDER_COLOR,
            PROP_BOX_SHADOW,
            PROP_VISIBILITY,
            PROP_FONT_FAMILY,
            PROP_FONT_SIZE,
            PROP_FONT_WEIGHT,
            PROP_FONT_STYLE,
            PROP_LINE_HEIGHT,
            PROP_LETTER_SPACING,
            PROP_TEXT_ALIGN,
            PROP_TEXT_OVERFLOW,
            PROP_WHITE_SPACE,
            PROP_WIDTH,
            PROP_HEIGHT,
            PROP_MIN_WIDTH,
            PROP_MAX_WIDTH,
            PROP_MIN_HEIGHT,
            PROP_MAX_HEIGHT,
            PROP_PADDING,
            PROP_MARGIN,
            PROP_ASPECT_RATIO,
        ];
        for name in all_names {
            assert!(is_valid_property(name), "属性 '{name}' 应由 D4 定义为有效");
        }
    }

    #[test]
    fn unknown_property_is_not_valid() {
        assert!(!is_valid_property("unknown-css-prop"));
        assert!(!is_valid_property("webkit-something"));
    }

    #[test]
    fn category_classification() {
        assert_eq!(category_of(PROP_DISPLAY), Some(PropertyCategory::Layout));
        assert_eq!(
            category_of(PROP_BACKGROUND_COLOR),
            Some(PropertyCategory::Visual)
        );
        assert_eq!(category_of(PROP_FONT_SIZE), Some(PropertyCategory::Text));
        assert_eq!(category_of(PROP_WIDTH), Some(PropertyCategory::SizeSpacing));
        assert_eq!(category_of("nonexistent"), None);
    }

    // ------------------------------------------------------------------
    // 布局属性解析测试
    // ------------------------------------------------------------------

    #[test]
    fn resolve_display_all_variants() {
        assert_eq!(
            resolve_display(&prop_str("flex")),
            Some(LayoutDisplay::Flex)
        );
        assert_eq!(
            resolve_display(&prop_str("grid")),
            Some(LayoutDisplay::Grid)
        );
        assert_eq!(
            resolve_display(&prop_str("block")),
            Some(LayoutDisplay::Block)
        );
        assert_eq!(
            resolve_display(&prop_str("none")),
            Some(LayoutDisplay::None)
        );
        assert_eq!(resolve_display(&prop_str("invalid")), None);
    }

    #[test]
    fn resolve_flex_direction_all_variants() {
        assert_eq!(
            resolve_flex_direction(&prop_str("row")),
            Some(FlexDirection::Row)
        );
        assert_eq!(
            resolve_flex_direction(&prop_str("row-reverse")),
            Some(FlexDirection::RowReverse)
        );
        assert_eq!(
            resolve_flex_direction(&prop_str("column")),
            Some(FlexDirection::Column)
        );
        assert_eq!(
            resolve_flex_direction(&prop_str("column-reverse")),
            Some(FlexDirection::ColumnReverse)
        );
    }

    #[test]
    fn resolve_flex_wrap_all_variants() {
        assert_eq!(
            resolve_flex_wrap(&prop_str("nowrap")),
            Some(FlexWrap::NoWrap)
        );
        assert_eq!(resolve_flex_wrap(&prop_str("wrap")), Some(FlexWrap::Wrap));
        assert_eq!(
            resolve_flex_wrap(&prop_str("wrap-reverse")),
            Some(FlexWrap::WrapReverse)
        );
    }

    #[test]
    fn resolve_justify_content_all_variants() {
        assert_eq!(
            resolve_justify_content(&prop_str("start")),
            Some(JustifyContent::Start)
        );
        assert_eq!(
            resolve_justify_content(&prop_str("center")),
            Some(JustifyContent::Center)
        );
        assert_eq!(
            resolve_justify_content(&prop_str("space-between")),
            Some(JustifyContent::SpaceBetween)
        );
        assert_eq!(
            resolve_justify_content(&prop_str("space-around")),
            Some(JustifyContent::SpaceAround)
        );
        assert_eq!(
            resolve_justify_content(&prop_str("space-evenly")),
            Some(JustifyContent::SpaceEvenly)
        );
        // flex-start/end 别名
        assert_eq!(
            resolve_justify_content(&prop_str("flex-start")),
            Some(JustifyContent::Start)
        );
        assert_eq!(
            resolve_justify_content(&prop_str("flex-end")),
            Some(JustifyContent::End)
        );
    }

    #[test]
    fn resolve_align_items_all_variants() {
        assert_eq!(
            resolve_align_items(&prop_str("start")),
            Some(AlignItems::Start)
        );
        assert_eq!(
            resolve_align_items(&prop_str("center")),
            Some(AlignItems::Center)
        );
        assert_eq!(
            resolve_align_items(&prop_str("baseline")),
            Some(AlignItems::Baseline)
        );
        assert_eq!(
            resolve_align_items(&prop_str("stretch")),
            Some(AlignItems::Stretch)
        );
    }

    #[test]
    fn resolve_align_self_all_variants() {
        assert_eq!(resolve_align_self(&prop_str("auto")), Some(AlignSelf::Auto));
        assert_eq!(
            resolve_align_self(&prop_str("start")),
            Some(AlignSelf::Start)
        );
        assert_eq!(
            resolve_align_self(&prop_str("center")),
            Some(AlignSelf::Center)
        );
        assert_eq!(
            resolve_align_self(&prop_str("stretch")),
            Some(AlignSelf::Stretch)
        );
    }

    #[test]
    fn resolve_flex_basis_auto() {
        assert_eq!(resolve_flex_basis(&prop_str("auto")), Some(FlexBasis::Auto));
    }

    #[test]
    fn resolve_flex_basis_length() {
        assert_eq!(
            resolve_flex_basis(&prop_f64(200.0)),
            Some(FlexBasis::Length(200.0))
        );
    }

    #[test]
    fn resolve_grid_tracks_parse() {
        let tracks = PropValue::List(vec![prop_str("1fr"), prop_str("2fr"), prop_str("auto")]);
        let result = resolve_grid_tracks(&tracks);
        assert!(result.is_some());
        let t = result.unwrap();
        assert_eq!(t.len(), 3);
        assert_eq!(t[0], GridTrack::Fr(1.0));
        assert_eq!(t[1], GridTrack::Fr(2.0));
        assert_eq!(t[2], GridTrack::Auto);
    }

    // ------------------------------------------------------------------
    // 视觉属性解析测试
    // ------------------------------------------------------------------

    #[test]
    fn resolve_visibility_variants() {
        assert_eq!(
            resolve_visibility(&prop_str("visible")),
            Some(Visibility::Visible)
        );
        assert_eq!(
            resolve_visibility(&prop_str("hidden")),
            Some(Visibility::Hidden)
        );
        assert_eq!(resolve_visibility(&prop_str("collapse")), None);
    }

    #[test]
    fn resolve_box_shadow_4_elements() {
        let shadow = PropValue::List(vec![
            prop_f64(2.0),
            prop_f64(4.0),
            prop_f64(8.0),
            prop_color(0.0, 0.0, 0.0, 0.5),
        ]);
        let result = resolve_box_shadow(&shadow);
        assert!(result.is_some());
        let (x, y, blur, _color) = result.unwrap();
        assert_eq!(x, 2.0);
        assert_eq!(y, 4.0);
        assert_eq!(blur, 8.0);
    }

    #[test]
    fn resolve_box_shadow_insufficient_elements() {
        let shadow = PropValue::List(vec![prop_f64(2.0), prop_f64(4.0)]);
        assert_eq!(resolve_box_shadow(&shadow), None);
    }

    // ------------------------------------------------------------------
    // 文本属性解析测试
    // ------------------------------------------------------------------

    #[test]
    fn resolve_font_weight_keywords() {
        assert_eq!(
            resolve_font_weight(&prop_str("normal")),
            Some(FontWeight::Normal)
        );
        assert_eq!(
            resolve_font_weight(&prop_str("bold")),
            Some(FontWeight::Bold)
        );
        assert_eq!(
            resolve_font_weight(&prop_str("light")),
            Some(FontWeight::Light)
        );
    }

    #[test]
    fn resolve_font_weight_numeric() {
        assert_eq!(
            resolve_font_weight(&prop_int(500)),
            Some(FontWeight::Number(500))
        );
        assert_eq!(
            resolve_font_weight(&prop_int(1001)),
            Some(FontWeight::Number(1000)) // 截断
        );
    }

    #[test]
    fn resolve_font_style_variants() {
        assert_eq!(
            resolve_font_style(&prop_str("normal")),
            Some(FontStyle::Normal)
        );
        assert_eq!(
            resolve_font_style(&prop_str("italic")),
            Some(FontStyle::Italic)
        );
        assert_eq!(
            resolve_font_style(&prop_str("oblique")),
            Some(FontStyle::Oblique)
        );
    }

    #[test]
    fn resolve_text_align_variants() {
        assert_eq!(
            resolve_text_align(&prop_str("start")),
            Some(TextAlign::Start)
        );
        assert_eq!(
            resolve_text_align(&prop_str("center")),
            Some(TextAlign::Center)
        );
        assert_eq!(resolve_text_align(&prop_str("end")), Some(TextAlign::End));
        assert_eq!(
            resolve_text_align(&prop_str("justify")),
            Some(TextAlign::Justify)
        );
        // left/right 别名
        assert_eq!(
            resolve_text_align(&prop_str("left")),
            Some(TextAlign::Start)
        );
        assert_eq!(resolve_text_align(&prop_str("right")), Some(TextAlign::End));
    }

    #[test]
    fn resolve_text_overflow_variants() {
        assert_eq!(
            resolve_text_overflow(&prop_str("clip")),
            Some(TextOverflow::Clip)
        );
        assert_eq!(
            resolve_text_overflow(&prop_str("ellipsis")),
            Some(TextOverflow::Ellipsis)
        );
    }

    #[test]
    fn resolve_white_space_variants() {
        assert_eq!(
            resolve_white_space(&prop_str("normal")),
            Some(WhiteSpace::Normal)
        );
        assert_eq!(
            resolve_white_space(&prop_str("nowrap")),
            Some(WhiteSpace::NoWrap)
        );
        assert_eq!(resolve_white_space(&prop_str("pre")), Some(WhiteSpace::Pre));
    }

    // ------------------------------------------------------------------
    // 尺寸/间距属性解析测试
    // ------------------------------------------------------------------

    #[test]
    fn resolve_aspect_ratio_slash_format() {
        assert_eq!(
            resolve_aspect_ratio_value(&prop_str("16/9")),
            Some(16.0 / 9.0)
        );
        assert_eq!(
            resolve_aspect_ratio_value(&prop_str("4/3")),
            Some(4.0 / 3.0)
        );
    }

    #[test]
    fn resolve_aspect_ratio_numeric() {
        assert_eq!(resolve_aspect_ratio_value(&prop_f64(1.5)), Some(1.5));
    }

    #[test]
    fn resolve_aspect_ratio_division_by_zero() {
        assert_eq!(resolve_aspect_ratio_value(&prop_str("16/0")), None);
    }

    // ------------------------------------------------------------------
    // resolve_properties 集成测试
    // ------------------------------------------------------------------

    #[test]
    fn resolve_empty_props() {
        let props = props_builder();
        let result = resolve_properties(&props);
        assert!(result.is_empty());
        assert!(result.layout.is_empty());
        assert!(result.visual.is_empty());
        assert!(result.text.is_empty());
    }

    #[test]
    fn resolve_full_layout_properties() {
        let mut props = props_builder();
        props.insert(Arc::from(PROP_DISPLAY), prop_str("flex"));
        props.insert(Arc::from(PROP_FLEX_DIRECTION), prop_str("column"));
        props.insert(Arc::from(PROP_FLEX_WRAP), prop_str("wrap"));
        props.insert(Arc::from(PROP_JUSTIFY_CONTENT), prop_str("center"));
        props.insert(Arc::from(PROP_ALIGN_ITEMS), prop_str("stretch"));
        props.insert(Arc::from(PROP_ALIGN_SELF), prop_str("auto"));
        props.insert(Arc::from(PROP_GAP), prop_f64(12.0));
        props.insert(Arc::from(PROP_FLEX_GROW), prop_int(1));
        props.insert(Arc::from(PROP_FLEX_SHRINK), prop_int(0));
        props.insert(Arc::from(PROP_FLEX_BASIS), prop_str("auto"));
        props.insert(Arc::from(PROP_WIDTH), prop_f64(300.0));
        props.insert(Arc::from(PROP_HEIGHT), prop_f64(200.0));
        props.insert(Arc::from(PROP_MIN_WIDTH), prop_f64(50.0));
        props.insert(Arc::from(PROP_MAX_WIDTH), prop_f64(1200.0));
        props.insert(Arc::from(PROP_MIN_HEIGHT), prop_f64(30.0));
        props.insert(Arc::from(PROP_MAX_HEIGHT), prop_f64(800.0));
        props.insert(Arc::from(PROP_PADDING), prop_f64(16.0));
        props.insert(Arc::from(PROP_MARGIN), prop_f64(8.0));
        props.insert(Arc::from(PROP_ASPECT_RATIO), prop_str("16/9"));

        let result = resolve_properties(&props);

        assert_eq!(result.layout.display, Some(LayoutDisplay::Flex));
        assert_eq!(result.layout.flex_direction, Some(FlexDirection::Column));
        assert_eq!(result.layout.flex_wrap, Some(FlexWrap::Wrap));
        assert_eq!(result.layout.justify_content, Some(JustifyContent::Center));
        assert_eq!(result.layout.align_items, Some(AlignItems::Stretch));
        assert_eq!(result.layout.align_self, Some(AlignSelf::Auto));
        assert_eq!(result.layout.gap, Some(12.0));
        assert_eq!(result.layout.flex_grow, Some(1.0));
        assert_eq!(result.layout.flex_shrink, Some(0.0));
        assert_eq!(result.layout.flex_basis, Some(FlexBasis::Auto));
        assert_eq!(result.layout.width, Some(300.0));
        assert_eq!(result.layout.height, Some(200.0));
        assert_eq!(result.layout.min_width, Some(50.0));
        assert_eq!(result.layout.max_width, Some(1200.0));
        assert_eq!(result.layout.min_height, Some(30.0));
        assert_eq!(result.layout.max_height, Some(800.0));
        assert_eq!(result.layout.padding, Some(16.0));
        assert_eq!(result.layout.margin, Some(8.0));
        assert_eq!(result.layout.aspect_ratio, Some(16.0 / 9.0));
    }

    #[test]
    fn resolve_full_visual_properties() {
        let mut props = props_builder();
        props.insert(
            Arc::from(PROP_BACKGROUND_COLOR),
            prop_color(1.0, 0.5, 0.0, 1.0),
        );
        props.insert(Arc::from(PROP_COLOR), prop_color(0.0, 0.0, 0.0, 1.0));
        props.insert(Arc::from(PROP_OPACITY), prop_f64(0.8));
        props.insert(Arc::from(PROP_BORDER_RADIUS), prop_f64(6.0));
        props.insert(Arc::from(PROP_BORDER_WIDTH), prop_f64(2.0));
        props.insert(Arc::from(PROP_BORDER_COLOR), prop_color(0.5, 0.5, 0.5, 1.0));
        props.insert(Arc::from(PROP_VISIBILITY), prop_str("hidden"));

        let result = resolve_properties(&props);

        assert!(result.visual.background_color.is_some());
        assert!(result.visual.color.is_some());
        assert_eq!(result.visual.opacity, Some(0.8));
        assert_eq!(result.visual.border_radius, Some(6.0));
        assert_eq!(result.visual.border_width, Some(2.0));
        assert!(result.visual.border_color.is_some());
        assert_eq!(result.visual.visibility, Some(Visibility::Hidden));
    }

    #[test]
    fn resolve_full_text_properties() {
        let mut props = props_builder();
        props.insert(Arc::from(PROP_FONT_FAMILY), prop_str("Inter, sans-serif"));
        props.insert(Arc::from(PROP_FONT_SIZE), prop_f64(14.0));
        props.insert(Arc::from(PROP_FONT_WEIGHT), prop_str("bold"));
        props.insert(Arc::from(PROP_FONT_STYLE), prop_str("italic"));
        props.insert(Arc::from(PROP_LINE_HEIGHT), prop_f64(1.5));
        props.insert(Arc::from(PROP_LETTER_SPACING), prop_f64(0.5));
        props.insert(Arc::from(PROP_TEXT_ALIGN), prop_str("center"));
        props.insert(Arc::from(PROP_TEXT_OVERFLOW), prop_str("ellipsis"));
        props.insert(Arc::from(PROP_WHITE_SPACE), prop_str("nowrap"));

        let result = resolve_properties(&props);

        assert_eq!(
            result.text.font_family,
            Some("Inter, sans-serif".to_string())
        );
        assert_eq!(result.text.font_size, Some(14.0));
        assert_eq!(result.text.font_weight, Some(FontWeight::Bold));
        assert_eq!(result.text.font_style, Some(FontStyle::Italic));
        assert_eq!(result.text.line_height, Some(1.5));
        assert_eq!(result.text.letter_spacing, Some(0.5));
        assert_eq!(result.text.text_align, Some(TextAlign::Center));
        assert_eq!(result.text.text_overflow, Some(TextOverflow::Ellipsis));
        assert_eq!(result.text.white_space, Some(WhiteSpace::NoWrap));
    }

    #[test]
    fn resolve_unknown_properties_collected() {
        let mut props = props_builder();
        props.insert(Arc::from(PROP_DISPLAY), prop_str("flex"));
        props.insert(Arc::from("custom-vendor-prop"), prop_str("value"));
        props.insert(Arc::from("another-unknown"), prop_f64(1.0));

        let result = resolve_properties(&props);

        // display 正常解析
        assert_eq!(result.layout.display, Some(LayoutDisplay::Flex));

        // 未知属性被收集
        assert_eq!(result.unknown_properties.len(), 2);
        assert!(
            result
                .unknown_properties
                .contains(&"custom-vendor-prop".to_string())
        );
        assert!(
            result
                .unknown_properties
                .contains(&"another-unknown".to_string())
        );
    }

    #[test]
    fn resolve_partial_valid_and_invalid_values() {
        let mut props = props_builder();
        // display 合法
        props.insert(Arc::from(PROP_DISPLAY), prop_str("flex"));
        // flex-direction 非法值 → 本函数不报错，但值被忽略（None）
        props.insert(Arc::from(PROP_FLEX_DIRECTION), prop_str("diagonal"));
        // 合法数值
        props.insert(Arc::from(PROP_WIDTH), prop_f64(100.0));

        let result = resolve_properties(&props);

        assert_eq!(result.layout.display, Some(LayoutDisplay::Flex));
        // 非法 flex-direction 值 → None
        assert_eq!(result.layout.flex_direction, None);
        assert_eq!(result.layout.width, Some(100.0));
    }

    #[test]
    fn resolve_grid_template_properties() {
        let mut props = props_builder();
        let columns = PropValue::List(vec![prop_str("100px"), prop_str("1fr"), prop_str("2fr")]);
        let rows = PropValue::List(vec![prop_str("auto"), prop_str("1fr")]);
        props.insert(Arc::from(PROP_GRID_TEMPLATE_COLUMNS), columns);
        props.insert(Arc::from(PROP_GRID_TEMPLATE_ROWS), rows);

        let result = resolve_properties(&props);

        assert!(result.layout.grid_template_columns.is_some());
        assert!(result.layout.grid_template_rows.is_some());
        let cols = result.layout.grid_template_columns.unwrap();
        let rows = result.layout.grid_template_rows.unwrap();
        assert_eq!(cols.len(), 3);
        assert_eq!(rows.len(), 2);
    }

    /// 测试 FlexBasis 枚举的默认值。
    #[test]
    fn flex_basis_default_is_auto() {
        assert_eq!(FlexBasis::default(), FlexBasis::Auto);
    }

    /// 测试 AlignSelf 枚举的默认值。
    #[test]
    fn align_self_default_is_auto() {
        assert_eq!(AlignSelf::default(), AlignSelf::Auto);
    }

    /// 测试 FlexWrap 枚举的默认值。
    #[test]
    fn flex_wrap_default_is_nowrap() {
        assert_eq!(FlexWrap::default(), FlexWrap::NoWrap);
    }

    /// 测试 FlexWrap 的 Display trait。
    #[test]
    fn flex_wrap_display() {
        assert_eq!(format!("{}", FlexWrap::NoWrap), "nowrap");
        assert_eq!(format!("{}", FlexWrap::Wrap), "wrap");
        assert_eq!(format!("{}", FlexWrap::WrapReverse), "wrap-reverse");
    }

    /// 测试 AlignSelf 的 Display trait。
    #[test]
    fn align_self_display() {
        assert_eq!(format!("{}", AlignSelf::Auto), "auto");
        assert_eq!(format!("{}", AlignSelf::Start), "start");
        assert_eq!(format!("{}", AlignSelf::End), "end");
        assert_eq!(format!("{}", AlignSelf::Center), "center");
        assert_eq!(format!("{}", AlignSelf::Baseline), "baseline");
        assert_eq!(format!("{}", AlignSelf::Stretch), "stretch");
    }

    /// 测试 LayoutStyle 合并时新字段正确传递。
    #[test]
    fn layout_style_merge_new_fields() {
        let base = LayoutStyle {
            flex_wrap: Some(FlexWrap::NoWrap),
            flex_grow: Some(0.0),
            ..LayoutStyle::default()
        };
        let over = LayoutStyle {
            flex_wrap: Some(FlexWrap::Wrap),
            flex_grow: Some(1.0),
            flex_shrink: Some(1.0),
            ..LayoutStyle::default()
        };
        let merged = base.merge(&over);
        assert_eq!(merged.flex_wrap, Some(FlexWrap::Wrap));
        assert_eq!(merged.flex_grow, Some(1.0));
        assert_eq!(merged.flex_shrink, Some(1.0));
    }

    /// 测试 VisualStyle 的 merge 方法。
    #[test]
    fn visual_style_merge() {
        let base = VisualStyle {
            opacity: Some(0.5),
            border_radius: Some(4.0),
            ..VisualStyle::default()
        };
        let over = VisualStyle {
            opacity: Some(1.0),
            background_color: Some(Color::rgb(1.0, 0.0, 0.0)),
            ..VisualStyle::default()
        };
        let merged = base.merge(&over);
        assert_eq!(merged.opacity, Some(1.0)); // over 覆盖
        assert_eq!(merged.border_radius, Some(4.0)); // base 保留
        assert!(merged.background_color.is_some()); // over 新增
    }

    /// 测试 TextStyle 的 merge 方法。
    #[test]
    fn text_style_merge() {
        let base = TextStyle {
            font_size: Some(14.0),
            font_weight: Some(FontWeight::Normal),
            ..TextStyle::default()
        };
        let over = TextStyle {
            font_size: Some(16.0),
            font_style: Some(FontStyle::Italic),
            ..TextStyle::default()
        };
        let merged = base.merge(&over);
        assert_eq!(merged.font_size, Some(16.0)); // over 覆盖
        assert_eq!(merged.font_weight, Some(FontWeight::Normal)); // base 保留
        assert_eq!(merged.font_style, Some(FontStyle::Italic)); // over 新增
    }

    /// 测试 grid_template_columns/rows 的 clone/merge 行为。
    #[test]
    fn layout_style_merge_grid_tracks() {
        let base = LayoutStyle {
            grid_template_columns: Some(vec![GridTrack::Fr(1.0)]),
            ..LayoutStyle::default()
        };
        let over = LayoutStyle {
            grid_template_rows: Some(vec![GridTrack::Auto]),
            ..LayoutStyle::default()
        };
        let merged = base.merge(&over);
        assert!(merged.grid_template_columns.is_some()); // base 保留
        assert!(merged.grid_template_rows.is_some()); // over 新增
    }

    /// 测试 ResolvedStyles 的构造函数。
    #[test]
    fn resolved_styles_new_is_empty() {
        let resolved = ResolvedStyles::new();
        assert!(resolved.is_empty());
    }
}
