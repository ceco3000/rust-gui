//! 运行时 `.html` 文件解析器——将 HTML 字符串/文件解析为 WidgetView 树。
//!
//! ## 设计
//!
//! 本模块实现编译期 `html!` 宏的运行时对应物。与宏生成 Rust 代码不同，
//! 本模块直接构造 `WidgetView<M>` 实例，用于开发时热重载。
//!
//! ### 与 `html!` 宏的差异
//!
//! | 特性 | `html!` 宏（编译期） | 运行时解析器 |
//! |------|---------------------|-------------|
//! | 属性类型推断 | 生成 PropValue 代码 | 运行时构造函数 |
//! | `on:event` 绑定 | 生成 MessageHandler 闭包 | 静默跳过（运行时无闭包） |
//! | `class` 属性 | 写入 props | 写入 props（复用 rgss 选择器引擎） |
//! | 错误报告 | 编译错误 | `HtmlParseError` |
//!
//! ### 使用示例
//!
//! ```ignore
//! use rgui_devtools::html_reload::parse_html_str;
//!
//! let html = r#"<Column gap="8"><Label text="Hello" /></Column>"#;
//! let view = parse_html_str::<MyMessage>(html)?;
//! ```
//!
//! 设计源自 D1 §13.7、D8 H05。

use ordered_float::OrderedFloat;
use rgui_core::AppMessage;
use rgui_core::view::{Color, PropValue, WidgetView};
use std::fmt;
use std::path::Path;
use std::sync::Arc;

// ============================================================================
// Error Types
// ============================================================================

/// HTML 解析错误。
#[derive(Debug)]
pub enum HtmlParseError {
    /// 语法/结构错误，携带位置信息。
    ParseError {
        /// 行号（1-based）。
        line: u64,
        /// 列号（1-based）。
        col: u64,
        /// 错误描述。
        message: String,
    },
    /// I/O 读取错误。
    IoError(std::io::Error),
    /// 不支持的事件绑定（运行时无法创建闭包）。
    UnsupportedEventBinding {
        /// 事件属性名（如 `on:click`）。
        name: String,
    },
}

impl fmt::Display for HtmlParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError { line, col, message } => {
                write!(f, "HTML 解析错误 (行 {line}, 列 {col}): {message}")
            },
            Self::IoError(e) => write!(f, "HTML 文件读取失败: {e}"),
            Self::UnsupportedEventBinding { name } => {
                write!(
                    f,
                    "运行时 HTML 不支持事件绑定 `{name}`——请在 `html!` 宏中使用"
                )
            },
        }
    }
}

impl std::error::Error for HtmlParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for HtmlParseError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

// ============================================================================
// Public API
// ============================================================================

/// 从 HTML 字符串解析为 WidgetView 树。
///
/// # 类型参数
///
/// * `M` - 应用消息类型（仅用于类型标注；运行时解析器不处理消息绑定）。
///
/// # 错误
///
/// * `HtmlParseError::ParseError` - HTML 语法错误
/// * `HtmlParseError::UnsupportedEventBinding` - 遇到 `on:event` 属性
///
/// # 示例
///
/// ```ignore
/// let view = parse_html_str::<MyMsg>(r#"<Button label="Hi" />"#)?;
/// assert_eq!(view.widget_type, "Button");
/// ```
pub fn parse_html_str<M: AppMessage>(html: &str) -> Result<WidgetView<M>, HtmlParseError> {
    let elements = parse_html_to_ast(html)?;
    if elements.is_empty() {
        return Err(HtmlParseError::ParseError {
            line: 1,
            col: 1,
            message: "HTML 内容为空".into(),
        });
    }
    Ok(ast_to_widget_view(&elements[0]))
}

/// 从 `.html` 文件解析为 WidgetView 树。
///
/// 读取文件内容后调用 [`parse_html_str`]。
pub fn parse_html_file<M: AppMessage>(path: &Path) -> Result<WidgetView<M>, HtmlParseError> {
    let content = std::fs::read_to_string(path)?;
    parse_html_str(&content)
}

// ============================================================================
// Internal AST Types
// ============================================================================

/// HTML 元素（运行时 AST 节点）。
#[derive(Debug, Clone)]
struct HtmlElement {
    tag_name: String,
    attributes: Vec<HtmlAttribute>,
    children: Vec<HtmlChild>,
}

/// HTML 属性。
#[derive(Debug, Clone)]
struct HtmlAttribute {
    name: String,
    value: HtmlValue,
}

/// HTML 属性值。
#[derive(Debug, Clone)]
enum HtmlValue {
    /// 字符串字面量（`attr="val"`）。
    Str(String),
    /// 布尔标志（`flag`，无值属性）。
    Bool,
}

/// HTML 子节点。
#[derive(Debug, Clone)]
enum HtmlChild {
    Element(HtmlElement),
    Text(String),
}

// ============================================================================
// quick-xml based parser
// ============================================================================

use quick_xml::Reader;
use quick_xml::events::Event;

/// 将 HTML 字符串解析为 `HtmlElement` AST。
fn parse_html_to_ast(html: &str) -> Result<Vec<HtmlElement>, HtmlParseError> {
    let mut reader = Reader::from_str(html);
    reader.config_mut().trim_text(true);

    let mut elements = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let el = parse_element_with_children(&mut reader, e)?;
                elements.push(el);
                buf.clear();
            },
            Ok(Event::Empty(e)) => {
                let el = parse_self_closing(e)?;
                elements.push(el);
                buf.clear();
            },
            Ok(Event::Eof) => break,
            Ok(Event::Text(t)) => {
                let text = t.unescape().map_err(|e| parse_error(&reader, &e))?;
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    return Err(parse_error_msg(
                        &reader,
                        format!("意外的文本内容 '{trimmed}'——所有内容必须位于标签内"),
                    ));
                }
                buf.clear();
            },
            Ok(_) => {
                buf.clear();
            },
            Err(e) => {
                return Err(parse_error(&reader, &e));
            },
        }
    }

    Ok(elements)
}

/// 解析开始标签及其子节点和闭合标签。
fn parse_element_with_children(
    reader: &mut Reader<&[u8]>,
    start: quick_xml::events::BytesStart<'_>,
) -> Result<HtmlElement, HtmlParseError> {
    let tag_name = String::from_utf8_lossy(start.name().as_ref()).to_string();
    let attributes = parse_attributes(&start);

    let mut children = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let child = parse_element_with_children(reader, e)?;
                children.push(HtmlChild::Element(child));
                buf.clear();
            },
            Ok(Event::Empty(e)) => {
                let child = parse_self_closing(e)?;
                children.push(HtmlChild::Element(child));
                buf.clear();
            },
            Ok(Event::End(e)) => {
                let name_ref = e.name();
                let end_tag = String::from_utf8_lossy(name_ref.as_ref());
                if end_tag.as_ref() != tag_name {
                    return Err(parse_error_msg(
                        reader,
                        format!("闭合标签不匹配：期望 `</{tag_name}>`，但找到 `</{end_tag}>`"),
                    ));
                }
                break;
            },
            Ok(Event::Text(t)) => {
                let text = t.unescape().map_err(|e| parse_error(reader, &e))?;
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    children.push(HtmlChild::Text(trimmed.to_string()));
                }
                buf.clear();
            },
            Ok(Event::Eof) => {
                return Err(parse_error_msg(
                    reader,
                    format!("未闭合的 `<{tag_name}>` 标签"),
                ));
            },
            Ok(_) => {
                buf.clear();
            },
            Err(e) => return Err(parse_error(reader, &e)),
        }
    }

    Ok(HtmlElement {
        tag_name,
        attributes,
        children,
    })
}

/// 解析自闭合标签（`<tag attr="val" />`）。
fn parse_self_closing(e: quick_xml::events::BytesStart<'_>) -> Result<HtmlElement, HtmlParseError> {
    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
    let attributes = parse_attributes(&e);

    Ok(HtmlElement {
        tag_name,
        attributes,
        children: Vec::new(),
    })
}

/// 解析属性列表。
fn parse_attributes(e: &quick_xml::events::BytesStart<'_>) -> Vec<HtmlAttribute> {
    let mut attrs = Vec::new();

    // quick-xml 对布尔属性（`disabled` 无值）返回 ExpectedEq 错误。
    // 我们从 raw bytes 提取属性名，将此类错误视为布尔属性。
    let raw_bytes = e.as_ref();

    for attr_result in e.attributes() {
        match attr_result {
            Ok(attr) => {
                let name = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                let raw_value = attr.value.as_ref();
                let value = if raw_value.is_empty() {
                    HtmlValue::Bool
                } else {
                    let decoded = attr
                        .unescape_value()
                        .map(|cow| cow.to_string())
                        .unwrap_or_else(|_| String::from_utf8_lossy(raw_value).to_string());
                    HtmlValue::Str(decoded)
                };
                attrs.push(HtmlAttribute { name, value });
            },
            Err(err) => {
                // ExpectedEq 错误：从 raw bytes 中提取布尔属性名
                if let Some(pos) = extract_err_position(&err) {
                    if pos > 0 && pos <= raw_bytes.len() {
                        // 回溯到前一个空白字符，提取属性名
                        let attr_name = extract_attr_name_at(raw_bytes, pos);
                        if !attr_name.is_empty() {
                            attrs.push(HtmlAttribute {
                                name: attr_name,
                                value: HtmlValue::Bool,
                            });
                        }
                    }
                }
            },
        }
    }

    attrs
}

/// 从 quick-xml 属性错误中提取字节位置。
fn extract_err_position(err: &dyn fmt::Display) -> Option<usize> {
    let err_str = format!("{err}");
    // 错误格式: "expected `=` at position N" 或类似格式
    if let Some(pos_start) = err_str.find("position ") {
        let num_part = &err_str[pos_start + 9..];
        let num_str: String = num_part
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(pos) = num_str.parse::<usize>() {
            return Some(pos);
        }
    }
    // 也尝试 ExpectedEq(N) 格式
    if let Some(start) = err_str.find('(') {
        if let Some(end) = err_str[start..].find(')') {
            let num_str = &err_str[start + 1..start + end];
            if let Ok(pos) = num_str.parse::<usize>() {
                return Some(pos);
            }
        }
    }
    None
}

/// 从 raw bytes 中提取位于 position 处的属性名。
///
/// position 是 `=` 期望出现的位置，属性名在其前面（可能紧接空白）。
fn extract_attr_name_at(raw: &[u8], pos: usize) -> String {
    let end = pos.min(raw.len());
    if end == 0 {
        return String::new();
    }
    // 跳过末尾空白，找到属性名的结束位置
    let attr_end = raw[..end]
        .iter()
        .rposition(|&b| !b.is_ascii_whitespace())
        .map(|p| p + 1)
        .unwrap_or(0);
    if attr_end == 0 {
        return String::new();
    }
    // 向前扫描到空白，找到属性名的起始位置
    let attr_start = raw[..attr_end]
        .iter()
        .rposition(|&b| b.is_ascii_whitespace())
        .map(|p| p + 1)
        .unwrap_or(0);
    String::from_utf8_lossy(&raw[attr_start..attr_end]).to_string()
}

// ============================================================================
// AST → WidgetView 转换
// ============================================================================

/// 将 HtmlElement AST 转换为 WidgetView。
fn ast_to_widget_view<M: AppMessage>(el: &HtmlElement) -> WidgetView<M> {
    let widget_type: &'static str = Box::leak(el.tag_name.clone().into_boxed_str());
    let mut view = WidgetView::new(widget_type);

    for attr in &el.attributes {
        // 跳过事件绑定——运行时无法创建闭包
        if is_event_binding(&attr.name) {
            continue;
        }

        let prop_value = match &attr.value {
            HtmlValue::Str(s) => infer_prop_value(s),
            HtmlValue::Bool => PropValue::Bool(true),
        };

        let name: &'static str = Box::leak(attr.name.clone().into_boxed_str());
        view = view.prop(name, prop_value);
    }

    for child in &el.children {
        match child {
            HtmlChild::Element(child_el) => {
                view = view.child(ast_to_widget_view(child_el));
            },
            HtmlChild::Text(text) => {
                view = view.child(text_node(text));
            },
        }
    }

    view
}

/// 创建文本子节点（`<Label>Hello</Label>` 中的 "Hello" → `<Text text="Hello" />`）。
fn text_node<M: AppMessage>(text: &str) -> WidgetView<M> {
    WidgetView::new("Text").prop("text", PropValue::Str(Arc::from(text)))
}

// ============================================================================
// 属性类型推断（运行时版，与 html! 宏逻辑一致）
// ============================================================================

/// 事件绑定检测。
fn is_event_binding(name: &str) -> bool {
    name.starts_with("on:")
}

/// 从字符串字面量推断 `PropValue` 类型。
///
/// 推断规则（与 `html!` 宏一致，D1 §13.6）：
/// - `"true"` / `"false"` → `PropValue::Bool`
/// - 纯数字整数 → `PropValue::Int`
/// - 含小数点的数字 → `PropValue::Float`
/// - 十六进制颜色 `#RRGGBB` / `#RRGGBBAA` → `PropValue::Color`
/// - 其他 → `PropValue::Str`
fn infer_prop_value(literal: &str) -> PropValue {
    // 布尔值
    if literal == "true" {
        return PropValue::Bool(true);
    }
    if literal == "false" {
        return PropValue::Bool(false);
    }

    // 十六进制颜色：#RRGGBB 或 #RRGGBBAA
    if let Some(color) = try_parse_hex_color(literal) {
        return PropValue::Color(color);
    }

    // 整数（全数字，可能带前导负号）
    if let Ok(i) = literal.parse::<i64>() {
        return PropValue::Int(i);
    }

    // 浮点数（含小数点）
    if literal.contains('.') {
        if let Ok(f) = literal.parse::<f64>() {
            return PropValue::Float(OrderedFloat(f));
        }
    }

    // 默认：字符串
    PropValue::Str(Arc::from(literal))
}

/// 解析十六进制颜色字符串 `#RRGGBB` 或 `#RRGGBBAA`。
fn try_parse_hex_color(s: &str) -> Option<Color> {
    if !s.starts_with('#') {
        return None;
    }
    let hex = &s[1..];
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;
            Some(Color::new(r, g, b, 1.0))
        },
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()? as f64 / 255.0;
            Some(Color::new(r, g, b, a))
        },
        _ => None,
    }
}

// ============================================================================
// Error helpers
// ============================================================================

fn parse_error(reader: &Reader<&[u8]>, e: &dyn fmt::Display) -> HtmlParseError {
    HtmlParseError::ParseError {
        line: 1,
        col: reader.buffer_position(),
        message: e.to_string(),
    }
}

fn parse_error_msg(reader: &Reader<&[u8]>, message: String) -> HtmlParseError {
    HtmlParseError::ParseError {
        line: 1,
        col: reader.buffer_position(),
        message,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // 测试用消息类型
    #[derive(Debug, Clone, PartialEq)]
    enum TestMsg {
        Noop,
    }

    impl AppMessage for TestMsg {
        fn message_name(&self) -> &'static str {
            "test"
        }
    }

    // --- 属性类型推断 ---

    #[test]
    fn infer_bool_true() {
        let v = infer_prop_value("true");
        assert_eq!(v, PropValue::Bool(true));
    }

    #[test]
    fn infer_bool_false() {
        let v = infer_prop_value("false");
        assert_eq!(v, PropValue::Bool(false));
    }

    #[test]
    fn infer_integer() {
        let v = infer_prop_value("42");
        assert_eq!(v, PropValue::Int(42));
    }

    #[test]
    fn infer_negative_integer() {
        let v = infer_prop_value("-10");
        assert_eq!(v, PropValue::Int(-10));
    }

    #[test]
    fn infer_float() {
        let v = infer_prop_value("8.0");
        assert!(matches!(v, PropValue::Float(_)));
        if let PropValue::Float(f) = v {
            assert!((f.into_inner() - 8.0).abs() < 1e-10);
        }
    }

    #[test]
    fn infer_color_hex6() {
        let v = infer_prop_value("#FF0000");
        assert_eq!(v, PropValue::Color(Color::new(1.0, 0.0, 0.0, 1.0)));
    }

    #[test]
    fn infer_color_hex8() {
        let v = infer_prop_value("#FF000080");
        assert_eq!(
            v,
            PropValue::Color(Color::new(1.0, 0.0, 0.0, 0.5019607843137255))
        );
    }

    #[test]
    fn infer_string() {
        let v = infer_prop_value("Hello");
        assert_eq!(v, PropValue::Str(Arc::from("Hello")));
    }

    #[test]
    fn infer_str_not_color() {
        // "# 开头的非十六进制字符串应回退为 Str
        let v = infer_prop_value("#notacolor");
        assert_eq!(v, PropValue::Str(Arc::from("#notacolor")));
    }

    // --- 事件绑定检测 ---

    #[test]
    fn detect_event_bindings() {
        assert!(is_event_binding("on:click"));
        assert!(is_event_binding("on:input"));
        assert!(is_event_binding("on:custom"));
    }

    #[test]
    fn reject_regular_attrs() {
        assert!(!is_event_binding("class"));
        assert!(!is_event_binding("label"));
        assert!(!is_event_binding("disabled"));
        assert!(!is_event_binding("on"));
    }

    // --- HTML 解析 → WidgetView ---

    #[test]
    fn parse_self_closing_element() {
        let view = parse_html_str::<TestMsg>(r#"<Button label="Hi" />"#).unwrap();
        assert_eq!(view.widget_type, "Button");
        assert_eq!(
            view.props.get("label"),
            Some(&PropValue::Str(Arc::from("Hi")))
        );
        assert!(view.children.is_empty());
    }

    #[test]
    fn parse_element_with_children() {
        let view = parse_html_str::<TestMsg>(r#"<Column gap="8"><Label text="Hello" /></Column>"#)
            .unwrap();
        assert_eq!(view.widget_type, "Column");
        assert!(view.props.contains_key("gap"));
        assert_eq!(view.children.len(), 1);
        assert_eq!(view.children[0].widget_type, "Label");
        assert_eq!(
            view.children[0].props.get("text"),
            Some(&PropValue::Str(Arc::from("Hello")))
        );
    }

    #[test]
    fn parse_boolean_attribute() {
        let view = parse_html_str::<TestMsg>(r#"<Button disabled />"#).unwrap();
        assert_eq!(view.widget_type, "Button");
        assert_eq!(view.props.get("disabled"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn parse_nested_structure() {
        let html = r#"<Container>
            <Row gap="4">
                <Button label="OK" />
                <Button label="Cancel" />
            </Row>
        </Container>"#;
        let view = parse_html_str::<TestMsg>(html).unwrap();
        assert_eq!(view.widget_type, "Container");
        assert_eq!(view.children.len(), 1);
        let row = &view.children[0];
        assert_eq!(row.widget_type, "Row");
        assert_eq!(row.children.len(), 2);
        assert_eq!(row.children[0].widget_type, "Button");
        assert_eq!(row.children[1].widget_type, "Button");
    }

    #[test]
    fn parse_color_attribute() {
        let view = parse_html_str::<TestMsg>(r##"<Container bg="#3B82F6" />"##).unwrap();
        let color = view.props.get("bg").unwrap();
        assert!(matches!(color, PropValue::Color(_)));
    }

    #[test]
    fn parse_numeric_attributes() {
        let view =
            parse_html_str::<TestMsg>(r#"<Container width="200" height="100.5" />"#).unwrap();
        assert_eq!(view.props.get("width"), Some(&PropValue::Int(200)));
        if let Some(PropValue::Float(f)) = view.props.get("height") {
            assert!((f.into_inner() - 100.5).abs() < 1e-10);
        } else {
            panic!("expected Float");
        }
    }

    #[test]
    fn parse_skips_event_bindings() {
        // 事件绑定在运行时被静默跳过（无法创建闭包）
        let view =
            parse_html_str::<TestMsg>(r#"<Button label="OK" on:click="ignored" />"#).unwrap();
        assert_eq!(view.widget_type, "Button");
        assert!(view.props.contains_key("label"));
        assert!(!view.props.contains_key("on:click"));
        assert!(view.message_bindings.is_empty());
    }

    #[test]
    fn parse_empty_input_errors() {
        let result = parse_html_str::<TestMsg>("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_text_children() {
        // 文本内容 → Text 子组件
        let view = parse_html_str::<TestMsg>(r#"<Label>Hello World</Label>"#).unwrap();
        assert_eq!(view.widget_type, "Label");
        assert_eq!(view.children.len(), 1);
        let txt = &view.children[0];
        assert_eq!(txt.widget_type, "Text");
        assert_eq!(
            txt.props.get("text"),
            Some(&PropValue::Str(Arc::from("Hello World")))
        );
    }

    #[test]
    fn parse_deeply_nested() {
        let html = r#"<Container>
            <Padding pad="16">
                <Column gap="8">
                    <Label text="Title" />
                    <Row gap="4">
                        <Button label="Yes" />
                        <Button label="No" />
                    </Row>
                </Column>
            </Padding>
        </Container>"#;
        let view = parse_html_str::<TestMsg>(html).unwrap();
        assert_eq!(view.widget_type, "Container");
        let padding = &view.children[0];
        assert_eq!(padding.widget_type, "Padding");
        let column = &padding.children[0];
        assert_eq!(column.widget_type, "Column");
        assert_eq!(column.children.len(), 2);
        let row = &column.children[1];
        assert_eq!(row.children.len(), 2);
    }

    // --- 与 html! 宏结果一致性 ---

    #[test]
    fn consistency_with_macro_simple() {
        let view = parse_html_str::<TestMsg>(r#"<Button label="Click me" disabled />"#).unwrap();
        assert_eq!(view.widget_type, "Button");
        assert_eq!(
            view.props.get("label"),
            Some(&PropValue::Str(Arc::from("Click me")))
        );
        assert_eq!(view.props.get("disabled"), Some(&PropValue::Bool(true)));
        assert!(view.children.is_empty());
        assert!(view.message_bindings.is_empty());
    }

    #[test]
    fn consistency_with_macro_nested() {
        let view =
            parse_html_str::<TestMsg>(r#"<Column gap="8.0"><Label text="Hi" /></Column>"#).unwrap();

        assert_eq!(view.widget_type, "Column");
        if let Some(PropValue::Float(f)) = view.props.get("gap") {
            assert!((f.into_inner() - 8.0).abs() < 1e-10);
        } else {
            panic!("expected Float for gap");
        }
        assert_eq!(view.children.len(), 1);
        let child = &view.children[0];
        assert_eq!(child.widget_type, "Label");
        assert_eq!(
            child.props.get("text"),
            Some(&PropValue::Str(Arc::from("Hi")))
        );
    }

    #[test]
    fn parse_html_file_io_error() {
        let result = parse_html_file::<TestMsg>(Path::new("/nonexistent/file.html"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HtmlParseError::IoError(_)));
    }
}
