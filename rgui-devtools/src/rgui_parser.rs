//! 运行时 `.rgui` 文件解析器——将 .rgui 文本/文件解析为 WidgetView 树。
//!
//! ## 设计
//!
//! 本模块实现 RG01 `.rgui` 格式规范的解析器，复用 H05 `html_reload` 的
//! quick-xml 架构。与 HTML 解析器不同，`.rgui` 额外支持：
//!
//! - HTML 风格事件（`onclick="fn()"`，而非 `on:event`）
//! - `${id.prop}` 表达式绑定
//! - `slot` 属性
//! - `data-*` 自定义数据属性
//! - 标签名大小写不敏感（统一转为小写）
//!
//! ### 与 `html!` 宏的差异
//!
//! | 特性 | `html!` 宏（编译期） | 运行时 .rgui 解析器 |
//! |------|---------------------|---------------------|
//! | 事件语法 | `on:event={Msg::Variant}` | `onclick="fn()"`（字符串） |
//! | 表达式 | `{rust_expr}` | `${id.prop}` 属性路径 |
//! | 动态表达式 | ✅ 完整 Rust | ❌ 仅属性路径 |
//! | 热重载 | ❌ | ✅ |
//!
//! ### 使用示例
//!
//! ```ignore
//! use rgui_devtools::rgui_parser::parse_rgui_str;
//!
//! let rgui = r#"<Button label="Save"/>"#;
//! let view = parse_rgui_str::<MyMessage>(rgui)?;
//! ```
//!
//! 设计源自 D8 RG02、RG01 .rgui 格式规范。

use ordered_float::OrderedFloat;
use rgui_core::AppMessage;
use rgui_core::view::{Color, PropValue, WidgetView};
use std::fmt;
use std::path::Path;
use std::sync::Arc;

// ============================================================================
// Error Types
// ============================================================================

/// .rgui 解析错误。
#[derive(Debug)]
pub enum RguiParseError {
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
    /// 验证错误（语义检查）。
    ValidationError {
        /// 错误描述。
        message: String,
    },
}

impl fmt::Display for RguiParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError { line, col, message } => {
                write!(f, ".rgui 解析错误 (行 {line}, 列 {col}): {message}")
            },
            Self::IoError(e) => write!(f, ".rgui 文件读取失败: {e}"),
            Self::ValidationError { message } => {
                write!(f, ".rgui 验证错误: {message}")
            },
        }
    }
}

impl std::error::Error for RguiParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for RguiParseError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

// ============================================================================
// Public API
// ============================================================================

/// 从 .rgui 字符串解析为 WidgetView 树。
///
/// # 类型参数
///
/// * `M` - 应用消息类型（仅用于类型标注；运行时解析器不处理消息绑定）。
///
/// # 错误
///
/// * `RguiParseError::ParseError` - .rgui 语法错误
/// * `RguiParseError::ValidationError` - 语义验证错误
///
/// # 示例
///
/// ```ignore
/// let view = parse_rgui_str::<MyMsg>(r#"<Button label="Save"/>"#)?;
/// assert_eq!(view.widget_type, "button");
/// ```
pub fn parse_rgui_str<M: AppMessage>(rgui: &str) -> Result<WidgetView<M>, RguiParseError> {
    let elements = parse_rgui_to_ast(rgui)?;
    if elements.is_empty() {
        return Err(RguiParseError::ParseError {
            line: 1,
            col: 1,
            message: ".rgui 内容为空".into(),
        });
    }
    if elements.len() > 1 {
        return Err(RguiParseError::ValidationError {
            message: format!(
                ".rgui 文件必须有且仅有一个根元素（找到 {} 个）",
                elements.len()
            ),
        });
    }
    let view = ast_to_widget_view(&elements[0]);
    validate_view(&view)?;
    Ok(view)
}

/// 从 `.rgui` 文件解析为 WidgetView 树。
///
/// 读取文件内容后调用 [`parse_rgui_str`]。
pub fn parse_rgui_file<M: AppMessage>(path: &Path) -> Result<WidgetView<M>, RguiParseError> {
    let content = std::fs::read_to_string(path)?;
    parse_rgui_str(&content)
}

// ============================================================================
// Internal AST Types
// ============================================================================

/// .rgui 元素（运行时 AST 节点）。
#[derive(Debug, Clone)]
struct RguiElement {
    tag_name: String,
    attributes: Vec<RguiAttribute>,
    children: Vec<RguiChild>,
}

/// .rgui 属性。
#[derive(Debug, Clone)]
struct RguiAttribute {
    name: String,
    value: RguiValue,
}

/// .rgui 属性值。
#[derive(Debug, Clone)]
enum RguiValue {
    /// 字符串字面量（`attr="val"`）。
    Str(String),
    /// 布尔标志（`flag`，无值属性）。
    Bool,
    /// `${id.prop}` 表达式。
    Expr(String),
}

/// .rgui 子节点。
#[derive(Debug, Clone)]
enum RguiChild {
    Element(RguiElement),
    Text(String),
}

// ============================================================================
// quick-xml based parser
// ============================================================================

use quick_xml::Reader;
use quick_xml::events::Event;

/// 将 .rgui 字符串解析为 `RguiElement` AST。
fn parse_rgui_to_ast(rgui: &str) -> Result<Vec<RguiElement>, RguiParseError> {
    let mut reader = Reader::from_str(rgui);
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
) -> Result<RguiElement, RguiParseError> {
    let tag_name = String::from_utf8_lossy(start.name().as_ref())
        .to_lowercase()
        .to_string();
    let attributes = parse_attributes(&start);

    let mut children = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let child = parse_element_with_children(reader, e)?;
                children.push(RguiChild::Element(child));
                buf.clear();
            },
            Ok(Event::Empty(e)) => {
                let child = parse_self_closing(e)?;
                children.push(RguiChild::Element(child));
                buf.clear();
            },
            Ok(Event::End(e)) => {
                let name_ref = e.name();
                let end_tag = String::from_utf8_lossy(name_ref.as_ref()).to_lowercase();
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
                    children.push(RguiChild::Text(trimmed.to_string()));
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

    Ok(RguiElement {
        tag_name,
        attributes,
        children,
    })
}

/// 解析自闭合标签（`<tag attr="val" />`）。
fn parse_self_closing(e: quick_xml::events::BytesStart<'_>) -> Result<RguiElement, RguiParseError> {
    let tag_name = String::from_utf8_lossy(e.name().as_ref())
        .to_lowercase()
        .to_string();
    let attributes = parse_attributes(&e);

    Ok(RguiElement {
        tag_name,
        attributes,
        children: Vec::new(),
    })
}

/// 解析属性列表。
fn parse_attributes(e: &quick_xml::events::BytesStart<'_>) -> Vec<RguiAttribute> {
    let mut attrs = Vec::new();
    let raw_bytes = e.as_ref();

    for attr_result in e.attributes() {
        match attr_result {
            Ok(attr) => {
                let name = String::from_utf8_lossy(attr.key.as_ref())
                    .to_lowercase()
                    .to_string();
                let raw_value = attr.value.as_ref();
                let value = if raw_value.is_empty() {
                    RguiValue::Bool
                } else {
                    let decoded = attr
                        .unescape_value()
                        .map(|cow| cow.to_string())
                        .unwrap_or_else(|_| String::from_utf8_lossy(raw_value).to_string());
                    // 检测 ${...} 表达式
                    if is_expr_binding(&decoded) {
                        RguiValue::Expr(extract_expr(&decoded))
                    } else {
                        RguiValue::Str(decoded)
                    }
                };
                attrs.push(RguiAttribute { name, value });
            },
            Err(err) => {
                // ExpectedEq 错误：从 raw bytes 中提取布尔属性名
                if let Some(pos) = extract_err_position(&err) {
                    if pos > 0 && pos <= raw_bytes.len() {
                        let attr_name = extract_attr_name_at(raw_bytes, pos).to_lowercase();
                        if !attr_name.is_empty() {
                            attrs.push(RguiAttribute {
                                name: attr_name,
                                value: RguiValue::Bool,
                            });
                        }
                    }
                }
            },
        }
    }

    attrs
}

/// 检测属性值是否为 `${...}` 表达式绑定。
fn is_expr_binding(value: &str) -> bool {
    value.starts_with("${") && value.ends_with('}')
}

/// 从 `${expr}` 中提取表达式内容。
fn extract_expr(value: &str) -> String {
    let inner = &value[2..value.len() - 1];
    inner.to_string()
}

/// 从 quick-xml 属性错误中提取字节位置。
fn extract_err_position(err: &dyn fmt::Display) -> Option<usize> {
    let err_str = format!("{err}");
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
fn extract_attr_name_at(raw: &[u8], pos: usize) -> String {
    let end = pos.min(raw.len());
    if end == 0 {
        return String::new();
    }
    let attr_end = raw[..end]
        .iter()
        .rposition(|&b| !b.is_ascii_whitespace())
        .map(|p| p + 1)
        .unwrap_or(0);
    if attr_end == 0 {
        return String::new();
    }
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

/// 将 RguiElement AST 转换为 WidgetView。
fn ast_to_widget_view<M: AppMessage>(el: &RguiElement) -> WidgetView<M> {
    let widget_type: &'static str = Box::leak(el.tag_name.clone().into_boxed_str());
    let mut view = WidgetView::new(widget_type);

    for attr in &el.attributes {
        match &attr.value {
            RguiValue::Str(s) => {
                let prop_value = infer_prop_value(s);
                let name: &'static str = Box::leak(attr.name.clone().into_boxed_str());
                view = view.prop(name, prop_value);
            },
            RguiValue::Bool => {
                let name: &'static str = Box::leak(attr.name.clone().into_boxed_str());
                view = view.prop(name, PropValue::Bool(true));
            },
            RguiValue::Expr(expr) => {
                // 表达式绑定：存储原表达式文本（运行时求值由后续阶段实现）
                let name: &'static str = Box::leak(attr.name.clone().into_boxed_str());
                // 用特殊前缀标记为表达式，后续表达式引擎可识别
                let marked = format!("${{expr:{expr}}}");
                view = view.prop(name, PropValue::Str(Arc::from(marked)));
            },
        }
    }

    // 子节点：slot 属性处理
    for child_el in &el.children {
        match child_el {
            RguiChild::Element(child) => {
                view = view.child(ast_to_widget_view(child));
            },
            RguiChild::Text(text) => {
                view = view.child(text_node(text));
            },
        }
    }

    // 文本内容语法糖：如有文本子节点且当前无 text 属性，添加 text prop
    // （注意：text prop 已在属性循环中设置，这里处理无显式 text 属性的情况）
    if !el.children.is_empty() && !view.props.contains_key("text") {
        // 检查第一个文本子节点
        let text_content: String = el
            .children
            .iter()
            .filter_map(|c| match c {
                RguiChild::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        if !text_content.is_empty() {
            view = view.prop("text", PropValue::Str(Arc::from(text_content)));
        }
    }

    view
}

/// 创建文本子节点（`<Label>Hello</Label>` 中的 "Hello" → `<Text text="Hello" />`）。
fn text_node<M: AppMessage>(text: &str) -> WidgetView<M> {
    WidgetView::new("text").prop("text", PropValue::Str(Arc::from(text)))
}

// ============================================================================
// 属性类型推断（与 html! 宏逻辑一致）
// ============================================================================

/// 从字符串字面量推断 `PropValue` 类型。
///
/// 推断规则（与 `html!` 宏一致）：
/// - `"true"` / `"false"` → `PropValue::Bool`
/// - 纯数字整数 → `PropValue::Int`
/// - 含小数点的数字 → `PropValue::Float`
/// - 十六进制颜色 `#RRGGBB` / `#RRGGBBAA` → `PropValue::Color`
/// - `"WxH"` 格式 → `PropValue::Size`
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

    // "WxH" 尺寸格式（如 "48x48"、"100x200"）
    if let Some(size) = try_parse_size(literal) {
        return PropValue::Size(size);
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

/// 解析 `WxH` 尺寸格式（如 "48x48"、"100x200"）。
fn try_parse_size(s: &str) -> Option<rgui_core::geometry::Size> {
    if let Some(x_pos) = s.find('x') {
        let w_str = &s[..x_pos];
        let h_str = &s[x_pos + 1..];
        if let (Ok(w), Ok(h)) = (w_str.parse::<f64>(), h_str.parse::<f64>()) {
            return Some(rgui_core::geometry::Size::new(w, h));
        }
    }
    None
}

// ============================================================================
// 验证
// ============================================================================

fn validate_view<M: AppMessage>(_view: &WidgetView<M>) -> Result<(), RguiParseError> {
    // V7: 事件名检查保留给后续阶段（RH02 接线时验证）
    Ok(())
}

// ============================================================================
// Helper
// ============================================================================

fn parse_error(reader: &Reader<&[u8]>, e: &dyn fmt::Display) -> RguiParseError {
    RguiParseError::ParseError {
        line: 1,
        col: reader.buffer_position(),
        message: e.to_string(),
    }
}

fn parse_error_msg(reader: &Reader<&[u8]>, message: String) -> RguiParseError {
    RguiParseError::ParseError {
        line: 1,
        col: reader.buffer_position(),
        message,
    }
}

// ============================================================================
// 测试
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
            match self {
                Self::Noop => "noop",
            }
        }
    }

    // --- 解析测试 ---

    #[test]
    fn parse_simple_self_closing() {
        let view = parse_rgui_str::<TestMsg>(r#"<Button label="Save"/>"#).unwrap();
        assert_eq!(view.widget_type, "button");
        assert_eq!(
            view.props.get("label"),
            Some(&PropValue::Str(Arc::from("Save")))
        );
    }

    #[test]
    fn parse_tag_name_case_insensitive() {
        let view1 = parse_rgui_str::<TestMsg>(r#"<BUTTON label="A"/>"#).unwrap();
        let view2 = parse_rgui_str::<TestMsg>(r#"<button label="A"/>"#).unwrap();
        let view3 = parse_rgui_str::<TestMsg>(r#"<Button label="A"/>"#).unwrap();
        assert_eq!(view1.widget_type, "button");
        assert_eq!(view1.widget_type, view2.widget_type);
        assert_eq!(view1.widget_type, view3.widget_type);
    }

    #[test]
    fn parse_nested_elements() {
        let view =
            parse_rgui_str::<TestMsg>(r#"<Column spacing="12"><Label text="Hello"/></Column>"#)
                .unwrap();
        assert_eq!(view.widget_type, "column");
        assert_eq!(view.props.get("spacing"), Some(&PropValue::Int(12)));
        assert_eq!(view.children.len(), 1);
        assert_eq!(view.children[0].widget_type, "label");
    }

    #[test]
    fn parse_boolean_attribute() {
        let view = parse_rgui_str::<TestMsg>(r#"<Button disabled/>"#).unwrap();
        assert_eq!(view.props.get("disabled"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn parse_text_content_sugar() {
        let view = parse_rgui_str::<TestMsg>(r#"<Label>Hello World</Label>"#).unwrap();
        // 文本语法糖：<Label>Hello World</Label> → text="Hello World"
        assert_eq!(
            view.props.get("text"),
            Some(&PropValue::Str(Arc::from("Hello World")))
        );
    }

    #[test]
    fn parse_explicit_text_overrides_content() {
        let view = parse_rgui_str::<TestMsg>(r#"<Label text="Override">Fallback</Label>"#).unwrap();
        assert_eq!(
            view.props.get("text"),
            Some(&PropValue::Str(Arc::from("Override")))
        );
    }

    #[test]
    fn parse_id_attribute() {
        let view =
            parse_rgui_str::<TestMsg>(r#"<TextField id="username" placeholder="Name"/>"#).unwrap();
        // id 作为 prop 存储
        assert_eq!(
            view.props.get("id"),
            Some(&PropValue::Str(Arc::from("username")))
        );
    }

    #[test]
    fn parse_hex_color() {
        let view = parse_rgui_str::<TestMsg>(r##"<Container bg="#3B82F6"/>"##).unwrap();
        let color = view.props.get("bg").unwrap();
        assert!(matches!(color, PropValue::Color(_)));
    }

    #[test]
    fn parse_int_float() {
        let view = parse_rgui_str::<TestMsg>(r#"<Container width="200" height="100.5"/>"#).unwrap();
        assert_eq!(view.props.get("width"), Some(&PropValue::Int(200)));
        assert!(matches!(
            view.props.get("height"),
            Some(PropValue::Float(_))
        ));
    }

    #[test]
    fn parse_size_format() {
        let view = parse_rgui_str::<TestMsg>(r#"<Image size="48x48"/>"#).unwrap();
        assert!(matches!(view.props.get("size"), Some(PropValue::Size(_))));
    }

    #[test]
    fn parse_string_values() {
        // 非特殊格式的字符串保持为 Str
        let view = parse_rgui_str::<TestMsg>(r#"<Button variant="Primary"/>"#).unwrap();
        assert_eq!(
            view.props.get("variant"),
            Some(&PropValue::Str(Arc::from("Primary")))
        );
    }

    #[test]
    fn parse_expr_binding() {
        let view = parse_rgui_str::<TestMsg>(r#"<Label text="${name.value}"/>"#).unwrap();
        let text_val = view.props.get("text").unwrap();
        // 表达式存储为标记格式
        if let PropValue::Str(s) = text_val {
            assert!(
                s.contains("expr:name.value"),
                "expected expr marker, got: {s}"
            );
        } else {
            panic!("expected Str, got {text_val:?}");
        }
    }

    #[test]
    fn parse_html_style_event() {
        // HTML 风格事件保留为属性（后续阶段 RH02 会接线到 Rhai）
        let view = parse_rgui_str::<TestMsg>(r#"<Button onclick="save()"/>"#).unwrap();
        // 事件存储为 prop
        assert!(view.props.contains_key("onclick"));
    }

    #[test]
    fn parse_oninput_event() {
        let view =
            parse_rgui_str::<TestMsg>(r#"<TextField oninput="update()" placeholder="Name"/>"#)
                .unwrap();
        let event_val = view.props.get("oninput").unwrap();
        assert_eq!(event_val, &PropValue::Str(Arc::from("update()")));
    }

    #[test]
    fn parse_slot_attribute() {
        let view = parse_rgui_str::<TestMsg>(r#"<Card><Label slot="header" text="Title"/></Card>"#)
            .unwrap();
        assert_eq!(view.children.len(), 1);
        let child = &view.children[0];
        assert_eq!(
            child.props.get("slot"),
            Some(&PropValue::Str(Arc::from("header")))
        );
    }

    #[test]
    fn parse_data_attributes() {
        let view =
            parse_rgui_str::<TestMsg>(r#"<Button data-action="delete" data-id="42"/>"#).unwrap();
        assert_eq!(
            view.props.get("data-action"),
            Some(&PropValue::Str(Arc::from("delete")))
        );
        assert_eq!(view.props.get("data-id"), Some(&PropValue::Int(42)));
    }

    #[test]
    fn parse_deeply_nested() {
        let view = parse_rgui_str::<TestMsg>(
            r#"<Column><Row><Button label="A"/><Button label="B"/></Row></Column>"#,
        )
        .unwrap();
        assert_eq!(view.widget_type, "column");
        assert_eq!(view.children.len(), 1);
        let row = &view.children[0];
        assert_eq!(row.widget_type, "row");
        assert_eq!(row.children.len(), 2);
    }

    // --- 错误测试 ---

    #[test]
    fn error_empty_input() {
        let result = parse_rgui_str::<TestMsg>("");
        assert!(result.is_err());
    }

    #[test]
    fn error_multiple_roots() {
        let result = parse_rgui_str::<TestMsg>(r#"<Button/><Label/>"#);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            format!("{err}").contains("根元素"),
            "expected root element error, got: {err}"
        );
    }

    #[test]
    fn error_unclosed_tag() {
        let result = parse_rgui_str::<TestMsg>(r#"<Column>"#);
        assert!(result.is_err());
    }

    #[test]
    fn error_mismatched_close_tag() {
        let result = parse_rgui_str::<TestMsg>(r#"<Column></Row>"#);
        assert!(result.is_err());
    }

    // --- 空白文本处理 ---

    #[test]
    fn whitespace_text_trimmed() {
        let view = parse_rgui_str::<TestMsg>("<Label>\n  Hello  \n</Label>").unwrap();
        assert_eq!(
            view.props.get("text"),
            Some(&PropValue::Str(Arc::from("Hello")))
        );
    }

    // --- 注释 ---

    #[test]
    fn parse_with_comments() {
        let view = parse_rgui_str::<TestMsg>("<!-- comment --><Button label=\"OK\"/>").unwrap();
        assert_eq!(view.widget_type, "button");
    }

    // --- login.rgui 完整示例 ---

    #[test]
    fn parse_login_form() {
        let view = parse_rgui_str::<TestMsg>(
            r#"<Column spacing="16" padding="24">
    <Label text="Login" variant="Header"/>
    <TextField id="username" placeholder="Username" oninput="updatePreview()"/>
    <TextField id="password" placeholder="Password" password="true"/>
    <Label text="${username.text}"/>
    <Row spacing="8">
        <Button variant="Primary" onclick="login()">Login</Button>
        <Button onclick="cancel()">Cancel</Button>
    </Row>
</Column>"#,
        )
        .unwrap();
        assert_eq!(view.widget_type, "column");
        assert_eq!(view.children.len(), 5);
        // 确认 id 属性被识别（作为 prop 存储）
        let text_field = &view.children[1];
        assert_eq!(
            text_field.props.get("id"),
            Some(&PropValue::Str(Arc::from("username")))
        );
        // 确认表达式绑定
        let expr_label = &view.children[3];
        assert!(expr_label.props.contains_key("text"));
    }
}
