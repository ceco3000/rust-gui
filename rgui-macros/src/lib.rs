//! rgui 过程宏——ui! 声明式 UI 宏。

use proc_macro::TokenStream;
use quote::quote;
use syn::{braced, parse::Parse, parse::ParseStream, parse_macro_input, Ident, LitBool, LitFloat, LitInt, LitStr, Token};

// ========== AST 类型 ==========

enum PropValueTokens { Str(LitStr), Bool(LitBool), Int(LitInt), Float(LitFloat) }

struct Prop { name: Ident, value: PropValueTokens }

impl Parse for Prop {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let value = if input.peek(LitStr) { PropValueTokens::Str(input.parse()?) }
        else if input.peek(LitBool) { PropValueTokens::Bool(input.parse()?) }
        else if input.peek(LitFloat) { PropValueTokens::Float(input.parse()?) }
        else if input.peek(LitInt) { PropValueTokens::Int(input.parse()?) }
        else { return Err(input.error("期望字符串、布尔、整数或浮点数")); };
        Ok(Prop { name, value })
    }
}

struct WidgetDef { widget_type: String, props: Vec<Prop>, children: Vec<WidgetDef> }

fn peek_child(input: ParseStream) -> bool { input.peek(Ident) && input.peek2(syn::token::Brace) }

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
                if content.is_empty() { break; }
                if peek_child(&content) {
                    children.push(content.parse::<WidgetDef>()?);
                } else {
                    props.push(content.parse::<Prop>()?);
                }
                if content.peek(Token![,]) { content.parse::<Token![,]>()?; }
                else if content.peek(Token![;]) { content.parse::<Token![;]>()?; }
                else if !content.is_empty()
                    && !peek_child(&content) {
                        return Err(content.error("期望 `,` 或 `;` 分隔符"));
                    }
            }
        }
        Ok(WidgetDef { widget_type, props, children })
    }
}

// ========== 代码生成 ==========

fn gen_prop_setter(prop: &Prop) -> proc_macro2::TokenStream {
    let name_str = prop.name.to_string();
    let value_tokens = match &prop.value {
        PropValueTokens::Str(s) => { let sv = s.value(); quote! { ::rgui::PropValue::Str(::std::sync::Arc::from(#sv)) } }
        PropValueTokens::Bool(b) => { let bv = b.value(); quote! { ::rgui::PropValue::Bool(#bv) } }
        PropValueTokens::Int(i) => { let iv: i64 = i.base10_parse().unwrap_or(0); quote! { ::rgui::PropValue::Int(#iv) } }
        PropValueTokens::Float(f) => { let fv: f64 = f.base10_parse().unwrap_or(0.0); quote! { ::rgui::PropValue::Float(::ordered_float::OrderedFloat(#fv)) } }
    };
    quote! { __view = __view.prop(#name_str, #value_tokens); }
}

fn gen_widget_def(wd: &WidgetDef) -> proc_macro2::TokenStream {
    let wt = &wd.widget_type;
    let ps: Vec<_> = wd.props.iter().map(gen_prop_setter).collect();
    let cs: Vec<_> = wd.children.iter().map(|c| {
        let ct = gen_widget_def(c);
        quote! { __view = __view.child(#ct); }
    }).collect();
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
        if c.is_uppercase() { if i > 0 { r.push('_'); } r.push(c.to_lowercase().next().unwrap()); }
        else { r.push(c); }
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
    let arms: Vec<_> = variants.iter().map(|v| {
        let vn = &v.ident;
        let mn = camel_to_snake(&vn.to_string());
        let pat = match &v.fields {
            syn::Fields::Unit => quote! { Self::#vn },
            syn::Fields::Unnamed(_) => quote! { Self::#vn(..) },
            syn::Fields::Named(_) => quote! { Self::#vn { .. } },
        };
        quote! { #pat => #mn }
    }).collect();
    quote! { impl ::rgui_core::AppMessage for #name { fn message_name(&self) -> &'static str { match self { #(#arms),* } } } }.into()
}

#[proc_macro_derive(PersistState)]
pub fn derive_persist_state(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;
    let sn = name.to_string();
    quote! { impl ::rgui_core::PersistState for #name { fn schema_name() -> &'static str { #sn } fn schema_version() -> u32 { 1 } fn as_any(&self) -> &dyn ::std::any::Any { self } fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any { self } } }.into()
}
