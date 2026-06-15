//! CSS 属性 → Taffy Style 映射。

use rgui_core::geometry::{LayoutDisplay, LayoutStyle};
use taffy::prelude::*;

/// CSS display 值 → Taffy Display。
#[must_use]
pub fn to_taffy_display(css_display: &str) -> Display {
    match css_display {
        "flex" => Display::Flex,
        "grid" => Display::Grid,
        "none" => Display::None,
        _ => Display::Block,
    }
}

/// CSS flex-direction → Taffy FlexDirection。
#[must_use]
pub fn to_taffy_flex_direction(css_value: &str) -> FlexDirection {
    match css_value {
        "row" => FlexDirection::Row,
        "row-reverse" => FlexDirection::RowReverse,
        "column" => FlexDirection::Column,
        "column-reverse" => FlexDirection::ColumnReverse,
        _ => FlexDirection::Row,
    }
}

/// CSS justify-content → Taffy JustifyContent。
#[must_use]
pub fn to_taffy_justify_content(css_value: &str) -> JustifyContent {
    match css_value {
        "flex-start" | "start" => JustifyContent::Start,
        "flex-end" | "end" => JustifyContent::End,
        "center" => JustifyContent::Center,
        "space-between" => JustifyContent::SpaceBetween,
        "space-around" => JustifyContent::SpaceAround,
        "space-evenly" => JustifyContent::SpaceEvenly,
        _ => JustifyContent::Start,
    }
}

/// CSS align-items → Taffy AlignItems。
#[must_use]
pub fn to_taffy_align_items(css_value: &str) -> AlignItems {
    match css_value {
        "flex-start" | "start" => AlignItems::Start,
        "flex-end" | "end" => AlignItems::End,
        "center" => AlignItems::Center,
        "baseline" => AlignItems::Baseline,
        "stretch" => AlignItems::Stretch,
        _ => AlignItems::Start,
    }
}

/// 简化的 CSS 样式列表 → Taffy Style 构建器。
///
/// 返回一个 `Style` 对象，可直接传递给 `LayoutEngine::create_node()`。
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn to_taffy_style(
    display: Option<&str>,
    width: Option<f32>,
    height: Option<f32>,
    flex_direction: Option<&str>,
    justify_content: Option<&str>,
    align_items: Option<&str>,
    gap: Option<f32>,
    padding: Option<f32>,
    margin: Option<f32>,
) -> Style {
    let mut style = Style::default();

    if let Some(d) = display {
        style.display = to_taffy_display(d);
    }

    style.size = taffy::geometry::Size {
        width: width.map_or(Dimension::Auto, Dimension::Length),
        height: height.map_or(Dimension::Auto, Dimension::Length),
    };

    if let Some(fd) = flex_direction {
        style.flex_direction = to_taffy_flex_direction(fd);
    }

    if let Some(jc) = justify_content {
        // 使用 AlignContent 作为 justify_content 的类型
        style.justify_content = Some(to_taffy_justify_content(jc));
    }

    if let Some(ai) = align_items {
        style.align_items = Some(to_taffy_align_items(ai));
    }

    if let Some(g) = gap {
        style.gap = taffy::geometry::Size {
            width: LengthPercentage::Length(g),
            height: LengthPercentage::Length(g),
        };
    }

    if let Some(p) = padding {
        let lp = LengthPercentage::Length(p);
        style.padding = taffy::geometry::Rect {
            left: lp,
            right: lp,
            top: lp,
            bottom: lp,
        };
    }

    if let Some(m) = margin {
        let lpa = LengthPercentageAuto::Length(m);
        style.margin = taffy::geometry::Rect {
            left: lpa,
            right: lpa,
            top: lpa,
            bottom: lpa,
        };
    }

    style
}

/// 将 `LayoutStyle` 转换为 Taffy `Style`。
///
/// `LayoutStyle` 中的 `None` 属性不设置对应 Taffy 字段，
/// Taffy 将使用其默认值。
#[must_use]
pub fn to_taffy_style_from_layout(layout: &LayoutStyle) -> Style {
    let mut style = Style::default();

    if let Some(display) = layout.display {
        style.display = match display {
            LayoutDisplay::Flex => Display::Flex,
            LayoutDisplay::Grid => Display::Grid,
            LayoutDisplay::Block => Display::Block,
            LayoutDisplay::None => Display::None,
        };
    }

    style.size = taffy::geometry::Size {
        width: layout.width.map_or(Dimension::Auto, |w| Dimension::Length(w as f32)),
        height: layout.height.map_or(Dimension::Auto, |h| Dimension::Length(h as f32)),
    };

    style.min_size = taffy::geometry::Size {
        width: layout
            .min_width
            .map_or(Dimension::Auto, |w| Dimension::Length(w as f32)),
        height: layout
            .min_height
            .map_or(Dimension::Auto, |h| Dimension::Length(h as f32)),
    };

    style.max_size = taffy::geometry::Size {
        width: layout
            .max_width
            .map_or(Dimension::Auto, |w| Dimension::Length(w as f32)),
        height: layout
            .max_height
            .map_or(Dimension::Auto, |h| Dimension::Length(h as f32)),
    };

    if let Some(fd) = layout.flex_direction {
        style.flex_direction = match fd {
            rgui_core::geometry::FlexDirection::Row => FlexDirection::Row,
            rgui_core::geometry::FlexDirection::RowReverse => FlexDirection::RowReverse,
            rgui_core::geometry::FlexDirection::Column => FlexDirection::Column,
            rgui_core::geometry::FlexDirection::ColumnReverse => FlexDirection::ColumnReverse,
        };
    }

    if let Some(jc) = layout.justify_content {
        style.justify_content = Some(match jc {
            rgui_core::geometry::JustifyContent::Start => JustifyContent::Start,
            rgui_core::geometry::JustifyContent::End => JustifyContent::End,
            rgui_core::geometry::JustifyContent::Center => JustifyContent::Center,
            rgui_core::geometry::JustifyContent::SpaceBetween => JustifyContent::SpaceBetween,
            rgui_core::geometry::JustifyContent::SpaceAround => JustifyContent::SpaceAround,
            rgui_core::geometry::JustifyContent::SpaceEvenly => JustifyContent::SpaceEvenly,
        });
    }

    if let Some(ai) = layout.align_items {
        style.align_items = Some(match ai {
            rgui_core::geometry::AlignItems::Start => AlignItems::Start,
            rgui_core::geometry::AlignItems::End => AlignItems::End,
            rgui_core::geometry::AlignItems::Center => AlignItems::Center,
            rgui_core::geometry::AlignItems::Baseline => AlignItems::Baseline,
            rgui_core::geometry::AlignItems::Stretch => AlignItems::Stretch,
        });
    }

    if let Some(ac) = layout.align_content {
        style.align_content = Some(match ac {
            rgui_core::geometry::AlignContent::Start => AlignContent::Start,
            rgui_core::geometry::AlignContent::End => AlignContent::End,
            rgui_core::geometry::AlignContent::Center => AlignContent::Center,
            rgui_core::geometry::AlignContent::SpaceBetween => AlignContent::SpaceBetween,
            rgui_core::geometry::AlignContent::SpaceAround => AlignContent::SpaceAround,
            rgui_core::geometry::AlignContent::SpaceEvenly => AlignContent::SpaceEvenly,
            rgui_core::geometry::AlignContent::Stretch => AlignContent::Stretch,
        });
    }

    if let Some(g) = layout.gap {
        let gap = LengthPercentage::Length(g as f32);
        style.gap = taffy::geometry::Size {
            width: gap,
            height: gap,
        };
    }

    if let Some(p) = layout.padding {
        let lp = LengthPercentage::Length(p as f32);
        style.padding = taffy::geometry::Rect {
            left: lp,
            right: lp,
            top: lp,
            bottom: lp,
        };
    }

    if let Some(m) = layout.margin {
        let lpa = LengthPercentageAuto::Length(m as f32);
        style.margin = taffy::geometry::Rect {
            left: lpa,
            right: lpa,
            top: lpa,
            bottom: lpa,
        };
    }

    style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_flex() {
        assert_eq!(to_taffy_display("flex"), Display::Flex);
    }

    #[test]
    fn display_default() {
        assert_eq!(to_taffy_display("unknown"), Display::Block);
    }

    #[test]
    fn flex_direction_column() {
        assert_eq!(to_taffy_flex_direction("column"), FlexDirection::Column);
    }

    #[test]
    fn to_taffy_style_flex_column() {
        let style = to_taffy_style(
            Some("flex"),
            Some(300.0),
            Some(200.0),
            Some("column"),
            Some("center"),
            Some("stretch"),
            Some(8.0),
            Some(16.0),
            Some(4.0),
        );
        assert_eq!(style.display, Display::Flex);
        assert_eq!(style.flex_direction, FlexDirection::Column);
    }

    // --- LayoutStyle → Taffy Style 转换 ---

    #[test]
    fn layout_style_empty_uses_taffy_defaults() {
        let layout = LayoutStyle::default();
        let style = to_taffy_style_from_layout(&layout);
        // 空 LayoutStyle 产生默认 Taffy Style
        assert_eq!(style.display, Display::default());
    }

    #[test]
    fn layout_style_flex_column() {
        let layout = LayoutStyle {
            display: Some(LayoutDisplay::Flex),
            width: Some(300.0),
            height: Some(200.0),
            flex_direction: Some(rgui_core::geometry::FlexDirection::Column),
            justify_content: Some(rgui_core::geometry::JustifyContent::Center),
            align_items: Some(rgui_core::geometry::AlignItems::Stretch),
            gap: Some(8.0),
            padding: Some(16.0),
            margin: Some(4.0),
            ..LayoutStyle::default()
        };
        let style = to_taffy_style_from_layout(&layout);
        assert_eq!(style.display, Display::Flex);
        assert_eq!(style.flex_direction, FlexDirection::Column);
        assert_eq!(
            style
                .justify_content
                .expect("justify_content should be set"),
            JustifyContent::Center
        );
        assert_eq!(
            style.align_items.expect("align_items should be set"),
            AlignItems::Stretch
        );
    }

    #[test]
    fn layout_style_grid() {
        let layout = LayoutStyle {
            display: Some(LayoutDisplay::Grid),
            width: Some(400.0),
            height: Some(300.0),
            ..LayoutStyle::default()
        };
        let style = to_taffy_style_from_layout(&layout);
        assert_eq!(style.display, Display::Grid);
    }

    #[test]
    fn layout_style_partial_conversion() {
        // 只设置部分属性，验证其他属性不受影响
        let layout = LayoutStyle {
            display: Some(LayoutDisplay::Flex),
            gap: Some(12.0),
            ..LayoutStyle::default()
        };
        let style = to_taffy_style_from_layout(&layout);
        assert_eq!(style.display, Display::Flex);
        // gap 被设置
        assert_eq!(
            style.gap.width,
            LengthPercentage::Length(12.0_f32)
        );
        // flex_direction 未设置，应保持默认值
        assert_eq!(style.flex_direction, FlexDirection::default());
    }
}
