//! `html!` 宏集成测试。
//!
//! 验证 HTML 语法 → WidgetView 展开的正确性。
//! H01 验收标准：`html! { <Button label="Hi" /> }` 编译通过，展开为正确的 WidgetView。

use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage, html};

#[derive(Debug, Clone, PartialEq, AppMessage)]
#[allow(dead_code)]
enum TestMessage {
    Click,
}

/// H01 验收标准：`<Button label="Hi" />` → WidgetView
#[test]
fn h01_basic_self_closing() {
    let view: WidgetView<TestMessage> = html! { <Button label="Hi" /> };

    assert_eq!(view.widget_type, "Button");
    assert!(view.id.is_none());
    assert_eq!(view.props.len(), 1);
    match view.props.get("label") {
        Some(PropValue::Str(s)) => assert_eq!(s.as_ref(), "Hi"),
        other => panic!("期望 PropValue::Str(\"Hi\")，实际: {other:?}"),
    }
    assert!(view.children.is_empty());
}

/// 嵌套子元素。
#[test]
fn h01_nested_children() {
    let view: WidgetView<TestMessage> = html! {
        <Column gap="8.0">
            <Label text="Hello" />
        </Column>
    };

    assert_eq!(view.widget_type, "Column");
    assert_eq!(view.props.len(), 1);
    assert_eq!(view.children.len(), 1);
    assert_eq!(view.children[0].widget_type, "Label");
    assert_eq!(view.children[0].props.len(), 1);
}

/// 多层嵌套。
#[test]
fn h01_deeply_nested() {
    let view: WidgetView<TestMessage> = html! {
        <Column>
            <Row gap="4">
                <Label text="A" />
                <Label text="B" />
            </Row>
        </Column>
    };

    assert_eq!(view.widget_type, "Column");
    assert_eq!(view.children.len(), 1);
    let row = &view.children[0];
    assert_eq!(row.widget_type, "Row");
    assert_eq!(row.children.len(), 2);
    assert_eq!(row.children[0].widget_type, "Label");
    assert_eq!(row.children[1].widget_type, "Label");
}

/// H08: 文本内容语法糖——`<Label>Hello</Label>` → `WidgetView::new("Label").child(WidgetView::new("Text").prop("text", "Hello"))`
#[test]
fn h08_text_content() {
    let view: WidgetView<TestMessage> = html! {
        <Label>Hello World</Label>
    };

    assert_eq!(view.widget_type, "Label");
    assert_eq!(view.children.len(), 1);
    let text_child = &view.children[0];
    assert_eq!(text_child.widget_type, "Text");
    match text_child.props.get("text") {
        Some(PropValue::Str(s)) => assert_eq!(s.as_ref(), "Hello World"),
        other => panic!("期望 PropValue::Str(\"Hello World\")，实际: {other:?}"),
    }
}

/// 布尔属性（无值属性）。
#[test]
fn h01_boolean_attribute() {
    let view: WidgetView<TestMessage> = html! { <Button disabled /> };

    assert_eq!(view.widget_type, "Button");
    match view.props.get("disabled") {
        Some(PropValue::Bool(true)) => {},
        other => panic!("期望 PropValue::Bool(true)，实际: {other:?}"),
    }
}

/// 多个属性。
#[test]
fn h01_multiple_attributes() {
    let view: WidgetView<TestMessage> = html! {
        <Button label="确认" class="primary" disabled />
    };

    assert_eq!(view.widget_type, "Button");
    assert_eq!(view.props.len(), 3);
    assert!(view.props.contains_key("label"));
    assert!(view.props.contains_key("class"));
    assert!(view.props.contains_key("disabled"));
}

/// 表达式属性（`attr={expr}`）。
#[test]
fn h01_expression_attribute() {
    let disabled = true;
    let view: WidgetView<TestMessage> = html! {
        <Button disabled={disabled} />
    };

    assert_eq!(view.widget_type, "Button");
    match view.props.get("disabled") {
        Some(PropValue::Bool(true)) => {},
        other => panic!("期望 PropValue::Bool(true)，实际: {other:?}"),
    }
}

/// 混合文本内容和子元素。
#[test]
fn h01_mixed_text_and_elements() {
    let view: WidgetView<TestMessage> = html! {
        <Card>
            Header Text
            <Label text="Body" />
            Footer Text
        </Card>
    };

    assert_eq!(view.widget_type, "Card");
    assert_eq!(view.children.len(), 3);
    // 第一个子节点: Text("Header Text")
    assert_eq!(view.children[0].widget_type, "Text");
    // 第二个子节点: Label
    assert_eq!(view.children[1].widget_type, "Label");
    // 第三个子节点: Text("Footer Text")
    assert_eq!(view.children[2].widget_type, "Text");
}

// ============================================================================
// H02: HTML 属性类型推断
// ============================================================================

/// 验收标准：`gap="8.0"` → Float(8.0)
#[test]
fn h02_float_type_inference() {
    let view: WidgetView<TestMessage> = html! { <Column gap="8.0" /> };

    match view.props.get("gap") {
        Some(PropValue::Float(f)) => {
            assert!(
                (f.into_inner() - 8.0).abs() < f64::EPSILON,
                "期望 Float(8.0)"
            );
        },
        other => panic!("期望 PropValue::Float(8.0)，实际: {other:?}"),
    }
}

/// 验收标准：`disabled="true"` → Bool(true)
#[test]
fn h02_bool_type_inference() {
    let view: WidgetView<TestMessage> = html! { <Button disabled="true" /> };

    match view.props.get("disabled") {
        Some(PropValue::Bool(true)) => {},
        other => panic!("期望 PropValue::Bool(true)，实际: {other:?}"),
    }
}

/// `disabled="false"` → Bool(false)
#[test]
fn h02_bool_false_type_inference() {
    let view: WidgetView<TestMessage> = html! { <Button disabled="false" /> };

    match view.props.get("disabled") {
        Some(PropValue::Bool(false)) => {},
        other => panic!("期望 PropValue::Bool(false)，实际: {other:?}"),
    }
}

/// 验收标准：`color="#FF0000"` → Color(1.0, 0.0, 0.0, 1.0)
#[test]
fn h02_color_hex6_inference() {
    let view: WidgetView<TestMessage> = html! { <Button color="#FF0000" /> };

    match view.props.get("color") {
        Some(PropValue::Color(c)) => {
            assert!(
                (c.r - 1.0).abs() < f64::EPSILON,
                "r 期望 1.0，实际: {}",
                c.r
            );
            assert!(c.g < f64::EPSILON, "g 期望 0.0，实际: {}", c.g);
            assert!(c.b < f64::EPSILON, "b 期望 0.0，实际: {}", c.b);
            assert!(
                (c.a - 1.0).abs() < f64::EPSILON,
                "a 期望 1.0，实际: {}",
                c.a
            );
        },
        other => panic!("期望 PropValue::Color，实际: {other:?}"),
    }
}

/// `color="#3B82F6"` → Color（蓝色）
#[test]
fn h02_color_hex6_blue_inference() {
    let view: WidgetView<TestMessage> = html! { <Button color="#3B82F6" /> };

    match view.props.get("color") {
        Some(PropValue::Color(c)) => {
            // #3B = 59/255 ≈ 0.231, #82 = 130/255 ≈ 0.510, #F6 = 246/255 ≈ 0.965
            assert!((c.r - 59.0 / 255.0).abs() < 0.01);
            assert!((c.g - 130.0 / 255.0).abs() < 0.01);
            assert!((c.b - 246.0 / 255.0).abs() < 0.01);
        },
        other => panic!("期望 PropValue::Color，实际: {other:?}"),
    }
}

/// `color="#FF000080"` → Color(1.0, 0.0, 0.0, ~0.5)
#[test]
fn h02_color_hex8_inference() {
    let view: WidgetView<TestMessage> = html! { <Button color="#FF000080" /> };

    match view.props.get("color") {
        Some(PropValue::Color(c)) => {
            assert!(
                (c.a - 128.0 / 255.0).abs() < 0.01,
                "a 期望 ~0.5，实际: {}",
                c.a
            );
        },
        other => panic!("期望 PropValue::Color，实际: {other:?}"),
    }
}

/// 验收标准：`label="Hi"` → Str("Hi")
#[test]
fn h02_str_type_inference() {
    let view: WidgetView<TestMessage> = html! { <Button label="Hi" /> };

    match view.props.get("label") {
        Some(PropValue::Str(s)) => assert_eq!(s.as_ref(), "Hi"),
        other => panic!("期望 PropValue::Str(\"Hi\")，实际: {other:?}"),
    }
}

/// 整数属性 → Int
#[test]
fn h02_int_type_inference() {
    let view: WidgetView<TestMessage> = html! { <Button count="42" /> };

    match view.props.get("count") {
        Some(PropValue::Int(42)) => {},
        other => panic!("期望 PropValue::Int(42)，实际: {other:?}"),
    }
}

/// class 属性值应为 Str（不推断为 Int/Float）
#[test]
fn h02_class_attribute_stays_str() {
    let view: WidgetView<TestMessage> = html! { <Button class="primary large" /> };

    match view.props.get("class") {
        Some(PropValue::Str(s)) => assert_eq!(s.as_ref(), "primary large"),
        other => panic!("期望 PropValue::Str(\"primary large\")，实际: {other:?}"),
    }
}
