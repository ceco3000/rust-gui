//! HTML → WidgetView 代码生成器。
//!
//! 将 HtmlElement AST 展开为 WidgetView builder 代码。设计源自 D1 §13。
//!
//! **路径约定**：生成的代码使用 `rgui_core` 绝对路径（`::rgui_core::view::*`），
//! 因为 `rgui` facade crate 只是重新导出，`rgui_core` 是所有下游 crate 的必选依赖。

use crate::html_parser::{HtmlAttribute, HtmlChild, HtmlElement, HtmlValue};
use proc_macro2::TokenStream;
use quote::quote;

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
    let prop_setters: Vec<_> = el.attributes.iter().map(generate_prop_setter).collect();
    let child_setters: Vec<_> = el.children.iter().map(generate_child_setter).collect();

    quote! {
        {
            let mut __view = ::rgui_core::view::WidgetView::new(#widget_type);
            #( #prop_setters )*
            #( #child_setters )*
            __view
        }
    }
}

/// 生成属性设置代码。
fn generate_prop_setter(attr: &HtmlAttribute) -> TokenStream {
    let name = &attr.name;
    match &attr.value {
        HtmlValue::Str(s) => {
            let value_str = s.as_str();
            quote! {
                __view = __view.prop(#name, ::rgui_core::view::PropValue::Str(
                    ::std::sync::Arc::from(#value_str)
                ));
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
