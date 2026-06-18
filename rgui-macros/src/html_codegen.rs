//! HTML → WidgetView 代码生成器。
//!
//! 将 HtmlElement AST 展开为 WidgetView builder 代码。设计源自 D1 §13。
//!
//! **路径约定**：生成的代码使用 `rgui_core` 绝对路径（`::rgui_core::view::*`），
//! 因为 `rgui` facade crate 只是重新导出，`rgui_core` 是所有下游 crate 的必选依赖。

use crate::html_parser::{HtmlAttribute, HtmlChild, HtmlElement, HtmlValue};
use proc_macro2::TokenStream;
use quote::quote;

/// 事件名称 → message_name 映射表（D1 §13.5、D5 §13.3）。
///
/// 编译期硬编码，零运行时开销。
const EVENT_TO_MESSAGE_NAME: &[(&str, &str)] = &[
    ("on:click", "click"),
    ("on:input", "text_changed"),
    ("on:change", "value_changed"),
    ("on:focus", "focus_in"),
    ("on:blur", "focus_out"),
    ("on:keydown", "key_down"),
    ("on:scroll", "scroll_changed"),
    ("on:submit", "submit"),
];

/// 检查属性名是否为事件绑定（`on:xxx` 前缀）。
fn is_event_binding(name: &str) -> bool {
    name.starts_with("on:")
}

/// 将事件属性名（如 `on:click`）映射为 message_name（如 `"click"`）。
///
/// 返回 `None` 表示未知事件名（宏在此时产生编译错误）。
fn event_to_message_name(event_attr: &str) -> Option<&'static str> {
    EVENT_TO_MESSAGE_NAME
        .iter()
        .find(|(k, _)| *k == event_attr)
        .map(|(_, v)| *v)
}

/// 将 HTML 元素列表展开为 WidgetView builder 代码。
///
/// 返回类型为 `WidgetView<M>`，`M` 由调用的上下文类型推断决定。
pub(crate) fn generate_widget_views(elements: &[HtmlElement]) -> TokenStream {
    match elements.len() {
        0 => quote! { compile_error!("html! 宏至少需要一个根元素"); },
        1 => generate_element(&elements[0]),
        _ => {
            // 多个根元素：仅返回第一个（未来可扩展为 Fragment）
            let views: Vec<_> = elements.iter().map(generate_element).collect();
            quote! {
                {
                    let __view = #(#views)*;
                    __view
                }
            }
        },
    }
}

/// 生成单个元素的 WidgetView 代码。
fn generate_element(el: &HtmlElement) -> TokenStream {
    let widget_type = &el.tag_name;

    // 分离普通属性和事件绑定
    let prop_setters: Vec<_> = el
        .attributes
        .iter()
        .filter(|a| !is_event_binding(&a.name))
        .map(generate_prop_setter)
        .collect();

    let event_setters: Vec<_> = el
        .attributes
        .iter()
        .filter(|a| is_event_binding(&a.name))
        .map(generate_event_binding)
        .collect();

    let child_setters: Vec<_> = el.children.iter().map(generate_child_setter).collect();

    quote! {
        {
            let mut __view = ::rgui_core::view::WidgetView::new(#widget_type);
            #( #prop_setters )*
            #( #event_setters )*
            #( #child_setters )*
            __view
        }
    }
}

/// 生成事件绑定代码（`on:event={handler}` → `.on(…, message_name, handler)`）。
///
/// 设计源自 D1 §13.5、D5 §13.2。
fn generate_event_binding(attr: &HtmlAttribute) -> TokenStream {
    let event_name = &attr.name; // e.g. "on:click"
    let message_name = match event_to_message_name(event_name) {
        Some(name) => name,
        None => {
            // 未知事件名在展开时产生编译错误
            return quote! {
                compile_error!(concat!("未知的事件绑定: `", #event_name, "`"));

            };
        },
    };

    match &attr.value {
        HtmlValue::Expr(handler_expr) => {
            // `on:click={Msg::Confirm}` → MessageHandler::Map(|_| Msg::Confirm)
            quote! {
                __view = __view.on(
                    ::rgui_core::WidgetId::from_u64(0),
                    Some(#message_name),
                    ::rgui_core::view::MessageHandler::Map(
                        ::std::sync::Arc::new(|_msg| #handler_expr)
                    ),
                );
            }
        },
        HtmlValue::Str(_) => {
            // 字符串值不适用于事件绑定
            quote! {
                compile_error!(concat!("事件绑定需要表达式 `{ }`，而非字符串字面量: `", #event_name, "=\"...\"`"));
            }
        },
        HtmlValue::Bool => {
            quote! {
                compile_error!(concat!("事件绑定需要表达式 `{ }`，而非布尔标志: `", #event_name, "`"));
            }
        },
    }
}

/// 生成属性设置代码。
fn generate_prop_setter(attr: &HtmlAttribute) -> TokenStream {
    let name = &attr.name;
    match &attr.value {
        HtmlValue::Str(s) => {
            // 类型推断：分析字面量字符串，自动推断 PropValue 类型（D1 §13.6）
            let value_tokens = infer_prop_value_from_literal(s);
            quote! {
                __view = __view.prop(#name, #value_tokens);
            }
        },
        HtmlValue::Expr(expr) => {
            quote! {
                __view = __view.prop(#name, ::rgui_core::view::PropValue::from(#expr));
            }
        },
        HtmlValue::Bool => {
            quote! {
                __view = __view.prop(#name, ::rgui_core::view::PropValue::Bool(true));
            }
        },
    }
}

/// 生成子节点设置代码。
fn generate_child_setter(child: &HtmlChild) -> TokenStream {
    match child {
        HtmlChild::Element(el) => {
            let child_view = generate_element(el);
            quote! {
                __view = __view.child(#child_view);
            }
        },
        HtmlChild::Text(text) => {
            let text_str = text.as_str();
            quote! {
                __view = __view.child(
                    ::rgui_core::view::WidgetView::new("Text")
                        .prop("text", ::rgui_core::view::PropValue::Str(
                            ::std::sync::Arc::from(#text_str)
                        ))
                );
            }
        },
    }
}

// ============================================================================
// 属性字面量类型推断（D1 §13.6）
// ============================================================================

/// 从 HTML 字符串字面量推断 `PropValue` 类型。
///
/// 推断规则（D1 §13.6）：
/// - `"true"` / `"false"` → `PropValue::Bool`
/// - 纯数字整数 → `PropValue::Int`
/// - 含小数点的数字 → `PropValue::Float`
/// - 十六进制颜色 `#RRGGBB` / `#RRGGBBAA` → `PropValue::Color`
/// - 其他 → `PropValue::Str`
fn infer_prop_value_from_literal(literal: &str) -> TokenStream {
    // 布尔值
    if literal == "true" {
        return quote! { ::rgui_core::view::PropValue::Bool(true) };
    }
    if literal == "false" {
        return quote! { ::rgui_core::view::PropValue::Bool(false) };
    }

    // 十六进制颜色：#RRGGBB 或 #RRGGBBAA
    if let Some(color_tokens) = try_parse_hex_color(literal) {
        return color_tokens;
    }

    // 整数（全数字，可能带前导负号）
    if let Ok(i) = literal.parse::<i64>() {
        return quote! { ::rgui_core::view::PropValue::Int(#i) };
    }

    // 浮点数（含小数点）
    if literal.contains('.') {
        if let Ok(f) = literal.parse::<f64>() {
            return quote! { ::rgui_core::view::PropValue::Float(::ordered_float::OrderedFloat(#f)) };
        }
    }

    // 默认：字符串
    quote! { ::rgui_core::view::PropValue::Str(::std::sync::Arc::from(#literal)) }
}

/// 尝试解析十六进制颜色字符串 `#RRGGBB` 或 `#RRGGBBAA`。
///
/// 返回 `Color::new(r, g, b, a)` 的 PropValue 表达式（通道值归一化到 0.0-1.0）。
fn try_parse_hex_color(s: &str) -> Option<TokenStream> {
    if !s.starts_with('#') {
        return None;
    }
    let hex = &s[1..];
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;
            Some(quote! {
                ::rgui_core::view::PropValue::Color(
                    ::rgui_core::view::Color::new(#r, #g, #b, 1.0f64)
                )
            })
        },
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()? as f64 / 255.0;
            Some(quote! {
                ::rgui_core::view::PropValue::Color(
                    ::rgui_core::view::Color::new(#r, #g, #b, #a)
                )
            })
        },
        _ => None,
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_bool_true() {
        let tokens = infer_prop_value_from_literal("true");
        let code = tokens.to_string();
        assert!(code.contains("Bool"), "期望 Bool 变体，实际: {code}");
        assert!(code.contains("true"), "期望 true 值，实际: {code}");
    }

    #[test]
    fn infer_bool_false() {
        let tokens = infer_prop_value_from_literal("false");
        let code = tokens.to_string();
        assert!(code.contains("Bool"), "期望 Bool 变体，实际: {code}");
        assert!(code.contains("false"), "期望 false 值，实际: {code}");
    }

    #[test]
    fn infer_integer() {
        let tokens = infer_prop_value_from_literal("42");
        let code = tokens.to_string();
        assert!(code.contains("Int"), "期望 Int 变体，实际: {code}");
        assert!(code.contains("42"), "期望 42 值，实际: {code}");
    }

    #[test]
    fn infer_float() {
        let tokens = infer_prop_value_from_literal("8.0");
        let code = tokens.to_string();
        assert!(code.contains("Float"), "期望 Float 变体，实际: {code}");
        assert!(code.contains("8"), "期望 8 值，实际: {code}");
    }

    #[test]
    fn infer_color_hex6() {
        let tokens = infer_prop_value_from_literal("#FF0000");
        let code = tokens.to_string();
        assert!(code.contains("Color"), "期望 Color 变体，实际: {code}");
        assert!(code.contains("new"), "期望 Color::new，实际: {code}");
    }

    #[test]
    fn infer_color_hex8() {
        let tokens = infer_prop_value_from_literal("#FF000080");
        let code = tokens.to_string();
        assert!(code.contains("Color"), "期望 Color 变体，实际: {code}");
        assert!(code.contains("new"), "期望 Color::new，实际: {code}");
    }

    #[test]
    fn infer_str_fallback() {
        let tokens = infer_prop_value_from_literal("Hello");
        let code = tokens.to_string();
        assert!(code.contains("Str"), "期望 Str 变体，实际: {code}");
        assert!(code.contains("Hello"), "期望 Hello 值，实际: {code}");
    }

    #[test]
    fn infer_str_not_color() {
        // "# 开头的非十六进制字符串应回退为 Str
        let tokens = infer_prop_value_from_literal("#notacolor");
        let code = tokens.to_string();
        assert!(code.contains("Str"), "期望 Str 回退，实际: {code}");
    }

    #[test]
    fn infer_negative_integer() {
        let tokens = infer_prop_value_from_literal("-10");
        let code = tokens.to_string();
        assert!(code.contains("Int"), "期望 Int 变体，实际: {code}");
        assert!(
            code.contains("-") && code.contains("10"),
            "期望 -10，实际: {code}"
        );
    }

    // --- 事件绑定映射 ---

    #[test]
    fn event_to_message_name_click() {
        assert_eq!(event_to_message_name("on:click"), Some("click"));
    }

    #[test]
    fn event_to_message_name_input() {
        assert_eq!(event_to_message_name("on:input"), Some("text_changed"));
    }

    #[test]
    fn event_to_message_name_change() {
        assert_eq!(event_to_message_name("on:change"), Some("value_changed"));
    }

    #[test]
    fn event_to_message_name_focus() {
        assert_eq!(event_to_message_name("on:focus"), Some("focus_in"));
    }

    #[test]
    fn event_to_message_name_blur() {
        assert_eq!(event_to_message_name("on:blur"), Some("focus_out"));
    }

    #[test]
    fn event_to_message_name_keydown() {
        assert_eq!(event_to_message_name("on:keydown"), Some("key_down"));
    }

    #[test]
    fn event_to_message_name_scroll() {
        assert_eq!(event_to_message_name("on:scroll"), Some("scroll_changed"));
    }

    #[test]
    fn event_to_message_name_submit() {
        assert_eq!(event_to_message_name("on:submit"), Some("submit"));
    }

    #[test]
    fn event_to_message_name_unknown() {
        assert_eq!(event_to_message_name("on:unknown"), None);
    }

    #[test]
    fn is_event_binding_detects_on_prefix() {
        assert!(is_event_binding("on:click"));
        assert!(is_event_binding("on:input"));
        assert!(is_event_binding("on:custom"));
    }

    #[test]
    fn is_event_binding_rejects_regular_attrs() {
        assert!(!is_event_binding("class"));
        assert!(!is_event_binding("label"));
        assert!(!is_event_binding("disabled"));
        assert!(!is_event_binding("on")); // no colon
    }
}
