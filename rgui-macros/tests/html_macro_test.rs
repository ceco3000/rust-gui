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
