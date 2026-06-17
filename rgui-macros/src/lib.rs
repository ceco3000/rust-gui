//! rgui 过程宏——ui! 声明式 UI 宏和派生宏。

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Ident, LitBool, LitFloat, LitInt, LitStr, Token, braced, parse::Parse, parse::ParseStream,
    parse_macro_input,
};

// ========== AST 类型 ==========

enum PropValueTokens {
    Str(LitStr),
    Bool(LitBool),
    Int(LitInt),
    Float(LitFloat),
}

struct Prop {
    name: Ident,
    value: PropValueTokens,
}

impl Parse for Prop {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let value = if input.peek(LitStr) {
            PropValueTokens::Str(input.parse()?)
        } else if input.peek(LitBool) {
            PropValueTokens::Bool(input.parse()?)
        } else if input.peek(LitFloat) {
            PropValueTokens::Float(input.parse()?)
        } else if input.peek(LitInt) {
            PropValueTokens::Int(input.parse()?)
        } else {
            return Err(input.error("期望字符串、布尔、整数或浮点数"));
        };
        Ok(Prop { name, value })
    }
}

struct WidgetDef {
    widget_type: String,
    props: Vec<Prop>,
    children: Vec<WidgetDef>,
}

fn peek_child(input: ParseStream) -> bool {
    input.peek(Ident) && input.peek2(syn::token::Brace)
}

impl Parse for WidgetDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let type_ident: Ident = input.parse()?;
        let widget_type = type_ident.to_string();
        let mut props = Vec::new();
        let mut children = Vec::new();
        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            loop {
                if content.is_empty() {
                    break;
                }
                if peek_child(&content) {
                    children.push(content.parse::<WidgetDef>()?);
                } else {
                    props.push(content.parse::<Prop>()?);
                }
                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                } else if content.peek(Token![;]) {
                    content.parse::<Token![;]>()?;
                } else if !content.is_empty() && !peek_child(&content) {
                    return Err(content.error("期望 `,` 或 `;` 分隔符"));
                }
            }
        }
        Ok(WidgetDef {
            widget_type,
            props,
            children,
        })
    }
}

// ========== 代码生成 ==========

fn gen_prop_setter(prop: &Prop) -> proc_macro2::TokenStream {
    let name_str = prop.name.to_string();
    let value_tokens = match &prop.value {
        PropValueTokens::Str(s) => {
            let sv = s.value();
            quote! { ::rgui::PropValue::Str(::std::sync::Arc::from(#sv)) }
        },
        PropValueTokens::Bool(b) => {
            let bv = b.value();
            quote! { ::rgui::PropValue::Bool(#bv) }
        },
        PropValueTokens::Int(i) => {
            let iv: i64 = i.base10_parse().unwrap_or(0);
            quote! { ::rgui::PropValue::Int(#iv) }
        },
        PropValueTokens::Float(f) => {
            let fv: f64 = f.base10_parse().unwrap_or(0.0);
            quote! { ::rgui::PropValue::Float(::ordered_float::OrderedFloat(#fv)) }
        },
    };
    quote! { __view = __view.prop(#name_str, #value_tokens); }
}

fn gen_widget_def(wd: &WidgetDef) -> proc_macro2::TokenStream {
    let wt = &wd.widget_type;
    let ps: Vec<_> = wd.props.iter().map(gen_prop_setter).collect();
    let cs: Vec<_> = wd
        .children
        .iter()
        .map(|c| {
            let ct = gen_widget_def(c);
            quote! { __view = __view.child(#ct); }
        })
        .collect();
    quote! { { let mut __view = ::rgui::WidgetView::new(#wt); #(#ps)* #(#cs)* __view } }
}

#[proc_macro]
pub fn ui(input: TokenStream) -> TokenStream {
    let wd = parse_macro_input!(input as WidgetDef);
    gen_widget_def(&wd).into()
}

// ========== derive(AppMessage) ==========

fn camel_to_snake(s: &str) -> String {
    let mut r = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                r.push('_');
            }
            r.push(c.to_lowercase().next().unwrap());
        } else {
            r.push(c);
        }
    }
    r
}

#[proc_macro_derive(AppMessage)]
pub fn derive_app_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;
    let variants = match &input.data {
        syn::Data::Enum(e) => &e.variants,
        _ => panic!("AppMessage 仅可用于枚举"),
    };
    let arms: Vec<_> = variants
        .iter()
        .map(|v| {
            let vn = &v.ident;
            let mn = camel_to_snake(&vn.to_string());
            let pat = match &v.fields {
                syn::Fields::Unit => quote! { Self::#vn },
                syn::Fields::Unnamed(_) => quote! { Self::#vn(..) },
                syn::Fields::Named(_) => quote! { Self::#vn { .. } },
            };
            quote! { #pat => #mn }
        })
        .collect();
    quote! { impl ::rgui_core::AppMessage for #name { fn message_name(&self) -> &'static str { match self { #(#arms),* } } } }.into()
}

#[proc_macro_derive(PersistState)]
pub fn derive_persist_state(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;
    let sn = name.to_string();
    quote! { impl ::rgui_core::PersistState for #name { fn schema_name() -> &'static str { #sn } fn schema_version() -> u32 { 1 } fn as_any(&self) -> &dyn ::std::any::Any { self } fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any { self } } }.into()
}

// ========== WidgetSpec 属性解析 ==========

/// `#[widget(...)]` 属性中定义的参数。
struct WidgetSpecAttr {
    /// 组件持久状态类型（必需）。
    state_type: syn::Path,
    /// 组件消息类型（必需）。
    msg_type: syn::Path,
    /// 可选的自定义组件名称（默认为类型名）。
    name: Option<String>,
}

impl Parse for WidgetSpecAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut state_type: Option<syn::Path> = None;
        let mut msg_type: Option<syn::Path> = None;
        let mut name: Option<String> = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if key == "state" {
                if state_type.is_some() {
                    return Err(input.error("`state` 属性重复定义"));
                }
                state_type = Some(input.parse()?);
            } else if key == "message" {
                if msg_type.is_some() {
                    return Err(input.error("`message` 属性重复定义"));
                }
                msg_type = Some(input.parse()?);
            } else if key == "name" {
                if name.is_some() {
                    return Err(input.error("`name` 属性重复定义"));
                }
                let lit: LitStr = input.parse()?;
                name = Some(lit.value());
            } else {
                return Err(input.error(format!(
                    "未知属性 `{key}`，期望 `state`、`message` 或 `name`"
                )));
            }
            // 可选逗号分隔
            if !input.is_empty() {
                let _ = input.parse::<Token![,]>();
            }
        }

        let state_type =
            state_type.ok_or_else(|| input.error("缺少必需属性 `state`（组件状态类型）"))?;
        let msg_type =
            msg_type.ok_or_else(|| input.error("缺少必需属性 `message`（组件消息类型）"))?;

        Ok(WidgetSpecAttr {
            state_type,
            msg_type,
            name,
        })
    }
}

// ========== derive(WidgetSpec) ==========
///
/// 自动生成的默认行为：
/// - `name()`: 返回类型名（若未通过 `name` 属性指定自定义名称）
/// - `update()`: 空实现（不做任何事）
/// - `measure()`: 返回 `Size::ZERO`
/// - `accessibility()`: 使用 trait 默认实现（返回 `AccessibilityNode::none()`），派生宏不覆盖
///
/// view() 和 paint() 通过固有方法 `__widget_view()` 和 `__widget_paint()` 委托调用。
/// 用户必须为这两个方法提供实现。
///
/// # 使用示例
///
/// ```ignore
/// #[derive(WidgetSpec)]
/// #[widget(
///     state = CounterState,
///     message = CounterMessage,
///     name = "my_app::Counter"
/// )]
/// struct Counter;
///
/// impl Counter {
///     fn __widget_view(&self, state: &CounterState, ctx: &ViewContext) -> WidgetView<CounterMessage> {
///         // 手动实现
///     }
///     fn __widget_paint(&self, state: &CounterState, bounds: Rect, ctx: &mut PaintContext) {
///         // 手动实现
///     }
/// }
/// ```
#[proc_macro_derive(WidgetSpec, attributes(widget))]
pub fn derive_widget_spec(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;

    // 解析 #[widget(...)] 属性：查找 widget 属性并解析其参数
    let attr = input
        .attrs
        .iter()
        .find(|a| a.path().is_ident("widget"))
        .ok_or_else(|| {
            syn::Error::new(
                name.span(),
                "缺少 #[widget(...)] 属性，必须指定 `state` 和 `message` 类型",
            )
        })
        .and_then(|a| a.parse_args::<WidgetSpecAttr>());

    let attr = match attr {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };

    let state_type = &attr.state_type;
    let msg_type = &attr.msg_type;

    // name: 如果用户指定则使用指定值，否则使用类型名
    let name_str = attr.name.unwrap_or_else(|| name.to_string());

    let expanded = quote! {
        impl ::rgui_core::traits::WidgetSpec for #name {
            type State = #state_type;
            type Message = #msg_type;

            fn name(&self) -> &'static str {
                #name_str
            }

            fn view(
                &self,
                state: &Self::State,
                ctx: &::rgui_core::context::ViewContext,
            ) -> ::rgui_core::view::WidgetView<Self::Message> {
                self.__widget_view(state, ctx)
            }

            fn update(
                &self,
                _msg: Self::Message,
                _state: &mut Self::State,
                _ctx: &mut ::rgui_core::context::UpdateContext,
            ) {
                // 默认空实现
            }

            fn measure(
                &self,
                state: &Self::State,
                constraints: ::rgui_core::geometry::BoxConstraints,
                ctx: &::rgui_core::context::MeasureContext,
            ) -> ::rgui_core::geometry::Size {
                self.default_measure(state, constraints, ctx)
            }

            fn paint(
                &self,
                state: &Self::State,
                bounds: ::rgui_core::geometry::Rect,
                ctx: &mut ::rgui_core::context::PaintContext,
            ) {
                self.__widget_paint(state, bounds, ctx)
            }

            // accessibility() 使用 trait 默认实现，派生宏不覆盖
        }
    };

    expanded.into()
}
