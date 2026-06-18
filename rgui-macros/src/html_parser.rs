//! HTML 语法解析器——将 XML-like HTML 语法解析为 HtmlElement AST。
//!
//! 本模块定义 `html!` 过程宏的解析器。设计源自 D1 §13。

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};

/// HTML 元素（AST 节点）。
#[derive(Debug, Clone)]
pub(crate) struct HtmlElement {
    /// 标签名（如 "Button"、"Column"）。
    pub tag_name: String,
    /// 属性列表。
    pub attributes: Vec<HtmlAttribute>,
    /// 子节点。
    pub children: Vec<HtmlChild>,
}

/// HTML 属性。
#[derive(Debug, Clone)]
pub(crate) struct HtmlAttribute {
    /// 属性名。
    pub name: String,
    /// 属性值。
    pub value: HtmlValue,
}

/// HTML 属性值。
#[derive(Debug, Clone)]
pub(crate) enum HtmlValue {
    /// 字符串字面量（`attr="val"`）。
    Str(String),
    /// Rust 表达式（`attr={expr}`）。
    Expr(TokenStream),
    /// 布尔标志（`flag`，无值属性）。
    Bool,
}

/// HTML 子节点。
#[derive(Debug, Clone)]
pub(crate) enum HtmlChild {
    /// 嵌套元素。
    Element(HtmlElement),
    /// 文本内容（`<Label>Hello</Label>` 中的 "Hello"）。
    Text(String),
}

/// 解析 `html!` 宏的 TokenStream，返回根元素列表。
pub(crate) fn parse_html(input: TokenStream) -> Result<Vec<HtmlElement>, syn::Error> {
    let mut parser = HtmlParser::new(input);
    parser.parse_root()
}

// ============================================================================
// Parser
// ============================================================================

struct HtmlParser {
    tokens: Vec<TokenTree>,
    pos: usize,
}

impl HtmlParser {
    fn new(input: TokenStream) -> Self {
        Self {
            tokens: input.into_iter().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<&TokenTree> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn expect_punct(&mut self, c: char) -> Result<(), syn::Error> {
        match self.tokens.get(self.pos) {
            Some(TokenTree::Punct(p)) if p.as_char() == c => {
                self.advance();
                Ok(())
            },
            Some(other) => Err(syn::Error::new(
                other.span(),
                format!("期望 `{c}`，但找到 `{other}`"),
            )),
            None => Err(syn::Error::new(
                Span::call_site(),
                format!("期望 `{c}`，但到达输入末尾"),
            )),
        }
    }

    fn expect_ident(&mut self) -> Result<String, syn::Error> {
        match self.tokens.get(self.pos) {
            Some(TokenTree::Ident(ident)) => {
                let name = ident.to_string();
                self.advance();
                Ok(name)
            },
            Some(other) => Err(syn::Error::new(
                other.span(),
                format!("期望标识符，但找到 `{other}`"),
            )),
            None => Err(syn::Error::new(
                Span::call_site(),
                "期望标识符，但到达输入末尾",
            )),
        }
    }

    fn expect_literal_str(&mut self) -> Result<String, syn::Error> {
        match self.tokens.get(self.pos) {
            Some(TokenTree::Literal(lit)) => {
                let s = lit.to_string();
                // 去除引号
                let inner = if s.starts_with('"') && s.ends_with('"') {
                    s[1..s.len() - 1].to_string()
                } else {
                    s
                };
                self.advance();
                Ok(inner)
            },
            Some(other) => Err(syn::Error::new(
                other.span(),
                format!("期望字符串字面量，但找到 `{other}`"),
            )),
            None => Err(syn::Error::new(
                Span::call_site(),
                "期望字符串字面量，但到达输入末尾",
            )),
        }
    }

    fn expect_brace_group(&mut self) -> Result<TokenStream, syn::Error> {
        match self.tokens.get(self.pos) {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
                let stream = g.stream();
                self.advance();
                Ok(stream)
            },
            Some(other) => Err(syn::Error::new(
                other.span(),
                format!("期望 `{{ }}` 表达式，但找到 `{other}`"),
            )),
            None => Err(syn::Error::new(
                Span::call_site(),
                "期望 `{ }` 表达式，但到达输入末尾",
            )),
        }
    }

    fn parse_root(&mut self) -> Result<Vec<HtmlElement>, syn::Error> {
        let mut elements = Vec::new();
        loop {
            // 跳过空白 token（实际上 token 流中不会出现空白，跳过 Group 的空白内容）
            if self.pos >= self.tokens.len() {
                break;
            }
            // 查找 `<` 开始标签
            if let Some(TokenTree::Punct(p)) = self.peek() {
                if p.as_char() == '<' {
                    let is_closing = self
                        .tokens
                        .get(self.pos + 1)
                        .map(|tt| matches!(tt, TokenTree::Punct(p) if p.as_char() == '/'))
                        .unwrap_or(false);
                    if is_closing {
                        return Err(syn::Error::new(
                            p.span(),
                            "意外的闭合标签，缺少对应的开始标签",
                        ));
                    }
                    elements.push(self.parse_element()?);
                    continue;
                }
            }
            return Err(syn::Error::new(
                self.tokens[self.pos].span(),
                "期望 `<` 开始标签",
            ));
        }
        Ok(elements)
    }

    fn parse_element(&mut self) -> Result<HtmlElement, syn::Error> {
        // `<`
        self.expect_punct('<')?;

        // 标签名
        let tag_name = self.expect_ident()?;

        // 属性列表
        let mut attributes = Vec::new();
        loop {
            if self.pos >= self.tokens.len() {
                return Err(syn::Error::new(
                    Span::call_site(),
                    format!("未闭合的 `<{tag_name}>` 标签"),
                ));
            }

            // 检查是否标签结束
            match self.tokens[self.pos] {
                TokenTree::Punct(ref p) if p.as_char() == '/' => {
                    // `/>` 自闭合
                    self.advance(); // `/`
                    self.expect_punct('>')?; // `>`
                    return Ok(HtmlElement {
                        tag_name,
                        attributes,
                        children: Vec::new(),
                    });
                },
                TokenTree::Punct(ref p) if p.as_char() == '>' => {
                    // `>` 开始标签结束
                    self.advance();
                    break;
                },
                _ => {},
            }

            // 解析属性
            attributes.push(self.parse_attribute()?);
        }

        // 解析子节点
        let children = self.parse_children(&tag_name)?;

        Ok(HtmlElement {
            tag_name,
            attributes,
            children,
        })
    }

    fn parse_attribute(&mut self) -> Result<HtmlAttribute, syn::Error> {
        let mut name = self.expect_ident()?;

        // 处理 `on:event` 语法：将 `:ident` 追加到属性名
        // 支持 XML 命名空间风格的属性名（如 `on:click`、`on:input`）
        loop {
            match self.peek() {
                Some(TokenTree::Punct(p)) if p.as_char() == ':' => {
                    self.advance(); // `:`
                    let suffix = self.expect_ident()?;
                    name.push(':');
                    name.push_str(&suffix);
                },
                _ => break,
            }
        }

        // 检查是否有 `=`
        match self.peek() {
            Some(TokenTree::Punct(p)) if p.as_char() == '=' => {
                self.advance(); // `=`

                // 值：字符串字面量或表达式
                let value = match self.peek() {
                    Some(TokenTree::Literal(_)) => HtmlValue::Str(self.expect_literal_str()?),
                    Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
                        HtmlValue::Expr(self.expect_brace_group()?)
                    },
                    _ => {
                        return Err(syn::Error::new(
                            self.peek()
                                .map(|tt| tt.span())
                                .unwrap_or_else(Span::call_site),
                            "期望字符串字面量或 `{ }` 表达式作为属性值",
                        ));
                    },
                };
                Ok(HtmlAttribute { name, value })
            },
            _ => {
                // 无值属性（如 `disabled`）
                Ok(HtmlAttribute {
                    name,
                    value: HtmlValue::Bool,
                })
            },
        }
    }

    fn parse_children(&mut self, parent_tag: &str) -> Result<Vec<HtmlChild>, syn::Error> {
        let mut children = Vec::new();

        loop {
            if self.pos >= self.tokens.len() {
                return Err(syn::Error::new(
                    Span::call_site(),
                    format!("未闭合的 `<{parent_tag}>` 标签，缺少 `</{parent_tag}>`"),
                ));
            }

            // 检查 `</`
            if let Some(TokenTree::Punct(p)) = self.peek() {
                if p.as_char() == '<' {
                    let is_closing = self
                        .tokens
                        .get(self.pos + 1)
                        .map(|tt| matches!(tt, TokenTree::Punct(punct) if punct.as_char() == '/'))
                        .unwrap_or(false);

                    if is_closing {
                        // 解析闭合标签
                        self.advance(); // `<`
                        self.advance(); // `/`
                        let close_tag = self.expect_ident()?;
                        self.expect_punct('>')?;

                        if close_tag != parent_tag {
                            return Err(syn::Error::new(
                                Span::call_site(),
                                format!(
                                    "闭合标签不匹配：期望 `</{parent_tag}>`，但找到 `</{close_tag}>`"
                                ),
                            ));
                        }
                        return Ok(children);
                    }

                    // 子元素
                    children.push(HtmlChild::Element(self.parse_element()?));
                    continue;
                }
            }

            // 文本内容
            let text = self.parse_text_content()?;
            if !text.is_empty() {
                children.push(HtmlChild::Text(text));
            } else {
                // 如果文本为空且没找到更多内容，break
                if self.pos >= self.tokens.len() {
                    return Ok(children);
                }
            }
        }
    }

    fn parse_text_content(&mut self) -> Result<String, syn::Error> {
        let mut text = String::new();
        let mut last_was_text_like = false;

        loop {
            if self.pos >= self.tokens.len() {
                break;
            }

            match &self.tokens[self.pos] {
                TokenTree::Punct(p) if p.as_char() == '<' => {
                    // 检查是否是元素开始
                    let next_is_slash = self
                        .tokens
                        .get(self.pos + 1)
                        .map(|tt| matches!(tt, TokenTree::Punct(punct) if punct.as_char() == '/'))
                        .unwrap_or(false);
                    if next_is_slash {
                        // `</` 闭合标签，停止文本收集
                        break;
                    }
                    // `<` 后跟标识符 → 子元素开始
                    if self
                        .tokens
                        .get(self.pos + 1)
                        .map(|tt| matches!(tt, TokenTree::Ident(_)))
                        .unwrap_or(false)
                    {
                        break;
                    }
                    // 否则 `<` 是文本内容的一部分（不太可能但保持健壮）
                    text.push('<');
                    last_was_text_like = false;
                    self.advance();
                },
                TokenTree::Literal(lit) => {
                    if last_was_text_like && !text.ends_with(' ') {
                        text.push(' ');
                    }
                    let s = lit.to_string();
                    let inner = if s.starts_with('"') && s.ends_with('"') {
                        &s[1..s.len() - 1]
                    } else {
                        &s
                    };
                    text.push_str(inner);
                    last_was_text_like = true;
                    self.advance();
                },
                TokenTree::Ident(ident) => {
                    // proc_macro tokenizer 不保留空白——相邻标识符之间手动插入空格
                    if last_was_text_like {
                        text.push(' ');
                    }
                    text.push_str(&ident.to_string());
                    last_was_text_like = true;
                    self.advance();
                },
                TokenTree::Punct(p) => {
                    let ch = p.as_char();
                    match ch {
                        ' ' | '\t' | '\n' | '\r' => {
                            // 空白字符：仅在前一 token 也是文本时添加空格
                            if last_was_text_like && !text.ends_with(' ') {
                                text.push(' ');
                            }
                            last_was_text_like = false;
                        },
                        _ => {
                            if last_was_text_like && !text.ends_with(' ') {
                                text.push(' ');
                            }
                            text.push(ch);
                            last_was_text_like = false;
                        },
                    }
                    self.advance();
                },
                TokenTree::Group(_) => {
                    break;
                },
            }
        }

        let trimmed = text.trim();
        Ok(if trimmed.is_empty() {
            String::new()
        } else {
            trimmed.to_string()
        })
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    fn html_tokens(tokens: proc_macro2::TokenStream) -> Vec<HtmlElement> {
        parse_html(tokens).expect("解析失败")
    }

    #[test]
    fn parse_self_closing_element() {
        let tokens = quote! { <Button label="Hi" /> };
        let elements = html_tokens(tokens);
        assert_eq!(elements.len(), 1);
        let el = &elements[0];
        assert_eq!(el.tag_name, "Button");
        assert_eq!(el.attributes.len(), 1);
        assert_eq!(el.attributes[0].name, "label");
        match &el.attributes[0].value {
            HtmlValue::Str(s) => assert_eq!(s, "Hi"),
            _ => panic!("期望字符串值"),
        }
        assert!(el.children.is_empty());
    }

    #[test]
    fn parse_element_with_children() {
        let tokens = quote! { <Column gap="8.0"> <Label text="Hello" /> </Column> };
        let elements = html_tokens(tokens);
        assert_eq!(elements.len(), 1);
        let col = &elements[0];
        assert_eq!(col.tag_name, "Column");
        assert_eq!(col.attributes.len(), 1);
        assert_eq!(col.children.len(), 1);
        match &col.children[0] {
            HtmlChild::Element(label) => {
                assert_eq!(label.tag_name, "Label");
            },
            _ => panic!("期望子元素"),
        }
    }

    #[test]
    fn parse_boolean_attribute() {
        let tokens = quote! { <Button disabled /> };
        let elements = html_tokens(tokens);
        let el = &elements[0];
        assert_eq!(el.attributes.len(), 1);
        assert_eq!(el.attributes[0].name, "disabled");
        match &el.attributes[0].value {
            HtmlValue::Bool => {},
            _ => panic!("期望 Bool 值"),
        }
    }

    #[test]
    fn parse_expression_attribute() {
        // expr={1 + 1}
        let tokens = quote! { <Button count={1 + 1} /> };
        let elements = html_tokens(tokens);
        let el = &elements[0];
        assert_eq!(el.attributes.len(), 1);
        assert_eq!(el.attributes[0].name, "count");
        match &el.attributes[0].value {
            HtmlValue::Expr(_) => {},
            _ => panic!("期望表达式值"),
        }
    }

    #[test]
    fn parse_nested_elements() {
        let tokens = quote! {
            <Column>
                <Row gap="4">
                    <Label text="A" />
                    <Label text="B" />
                </Row>
            </Column>
        };
        let elements = html_tokens(tokens);
        assert_eq!(elements.len(), 1);
        let col = &elements[0];
        assert_eq!(col.tag_name, "Column");
        assert_eq!(col.children.len(), 1);
        match &col.children[0] {
            HtmlChild::Element(row) => {
                assert_eq!(row.tag_name, "Row");
                assert_eq!(row.children.len(), 2);
            },
            _ => panic!("期望 Row 子元素"),
        }
    }

    #[test]
    fn parse_text_content() {
        let tokens = quote! { <Label> Hello World </Label> };
        let elements = html_tokens(tokens);
        let el = &elements[0];
        assert_eq!(el.children.len(), 1);
        match &el.children[0] {
            HtmlChild::Text(s) => assert_eq!(s, "Hello World"),
            _ => panic!("期望文本子节点"),
        }
    }

    #[test]
    fn parse_mismatched_closing_tag_error() {
        let tokens = quote! { <Column> </Row> };
        let result = parse_html(tokens);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("闭合标签不匹配"),
            "期望闭合标签不匹配错误，实际: {err}"
        );
    }

    #[test]
    fn parse_multiple_root_elements() {
        let tokens = quote! { <Label text="A" /> <Label text="B" /> };
        let elements = html_tokens(tokens);
        assert_eq!(elements.len(), 2);
    }

    #[test]
    fn parse_element_with_multiple_attributes() {
        let tokens = quote! { <Button label="确认" class="primary" disabled /> };
        let elements = html_tokens(tokens);
        let el = &elements[0];
        assert_eq!(el.attributes.len(), 3);
        assert_eq!(el.attributes[0].name, "label");
        assert_eq!(el.attributes[1].name, "class");
        assert_eq!(el.attributes[2].name, "disabled");
    }

    #[test]
    fn parse_on_event_attribute() {
        // `on:click={handler}` — 冒号分隔的属性名
        let tokens = quote! { <Button on:click={TestMessage::Confirm} /> };
        let elements = html_tokens(tokens);
        let el = &elements[0];
        assert_eq!(el.attributes.len(), 1);
        assert_eq!(el.attributes[0].name, "on:click");
        match &el.attributes[0].value {
            HtmlValue::Expr(_) => {},
            _ => panic!("期望表达式值"),
        }
    }

    #[test]
    fn parse_on_event_with_props() {
        let tokens = quote! { <Button label="Hi" on:click={TestMessage::Click} /> };
        let elements = html_tokens(tokens);
        let el = &elements[0];
        assert_eq!(el.attributes.len(), 2);
        assert_eq!(el.attributes[0].name, "label");
        assert_eq!(el.attributes[1].name, "on:click");
    }
}
