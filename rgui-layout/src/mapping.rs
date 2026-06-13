//! CSS 属性 → Taffy Style 映射。

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
}
