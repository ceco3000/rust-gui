//! .rgss 解析器——将 CSS-like 字符串解析为 StyleRule。
//!
//! 支持标准规则和 `@media` 块（D4 §8）。
//! `@media` 块内的每条规则携带对应的媒体查询条件。

use crate::selector::{MediaCondition, Selector, StyleRule};
use crate::theme::ColorScheme;
use rgui_core::view::PropValue;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// .rgss 解析错误。
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("语法错误: {0}")]
    Syntax(String),
    #[error("未知属性: {0}")]
    UnknownProperty(String),
    #[error("无效值: {0}")]
    InvalidValue(String),
}

/// 解析 .rgss 字符串，返回 StyleRule 列表。
///
/// 支持的语法（简化版）：
/// - `WidgetType { prop: value; }`
/// - `.class-name { prop: value; }`
/// - `WidgetType:pseudo { prop: value; }`
/// - `@media (max-width: 768px) { ... }`
/// - `@media (min-width: 640px) and (max-width: 1024px) { ... }`
/// - `@media (prefers-color-scheme: dark) { ... }`
/// - 属性值支持：`14px`, `#FF0000`, `true/false`, `8`, `"text"`
pub fn parse_rgss(source: &str) -> Result<Vec<StyleRule>, ParseError> {
    let mut rules = Vec::new();
    let mut pos = 0;
    let chars: Vec<char> = source.chars().collect();

    while pos < chars.len() {
        // 跳过空白和注释
        skip_whitespace_and_comments(&chars, &mut pos);
        if pos >= chars.len() {
            break;
        }

        // 检测 @media 关键字
        if pos < chars.len() && chars[pos] == '@' {
            let media_condition = parse_media_query(&chars, &mut pos)?;

            // 跳过空白
            skip_whitespace(&chars, &mut pos);

            // 解析 { ... } 块内的所有规则
            let block_rules = parse_rules_in_block(&chars, &mut pos)?;

            // 为块内的每条规则附加媒体查询条件
            for rule in block_rules {
                rules.push(StyleRule::with_media_and_important(
                    rule.selector,
                    rule.declarations,
                    media_condition.clone(),
                    rule.important_declarations,
                ));
            }
            continue;
        }

        // 解析选择器（类型/类/ID）
        let selector = parse_selector(&chars, &mut pos)?;

        // 跳过空白
        skip_whitespace(&chars, &mut pos);

        // 解析 { ... }
        if pos >= chars.len() || chars[pos] != '{' {
            return Err(ParseError::Syntax("期望 `{`".into()));
        }
        pos += 1;

        let (declarations, important) = parse_declarations_with_important(&chars, &mut pos)?;

        rules.push(StyleRule::with_important(selector, declarations, important));
    }

    Ok(rules)
}

/// 解析 `@media` 查询条件。
///
/// 支持的格式：
/// - `@media (max-width: 768px)`
/// - `@media (min-width: 640px) and (max-width: 1024px)`
/// - `@media (prefers-color-scheme: dark)`
///
/// 调用前 `chars[pos]` 指向 `@`，调用后指向块左花括号或之后（由调用方处理）。
fn parse_media_query(chars: &[char], pos: &mut usize) -> Result<MediaCondition, ParseError> {
    // 消耗 `@`
    if *pos >= chars.len() || chars[*pos] != '@' {
        return Err(ParseError::Syntax("期望 `@media`".into()));
    }
    *pos += 1;

    // 解析 `media` 关键字
    let keyword = parse_identifier(chars, pos);
    if keyword != "media" {
        return Err(ParseError::Syntax(format!(
            "期望 `@media`，得到 `@{keyword}`"
        )));
    }

    skip_whitespace(chars, pos);

    // 解析至少一个条件
    let first = parse_single_media_condition(chars, pos)?;

    // 检查后续是否有 `and` 连接
    let mut conditions = vec![first];

    loop {
        skip_whitespace(chars, pos);
        // 检查 `and`
        if *pos + 2 < chars.len()
            && chars[*pos..*pos + 3]
                .iter()
                .collect::<String>()
                .eq_ignore_ascii_case("and")
        {
            *pos += 3;
            let next = parse_single_media_condition(chars, pos)?;
            conditions.push(next);
        } else {
            break;
        }
    }

    if conditions.len() == 1 {
        Ok(conditions
            .into_iter()
            .next()
            .expect("已确认 conditions.len() == 1"))
    } else {
        Ok(MediaCondition::And(conditions))
    }
}

/// 解析媒体条件中的数值（带可选小数点和 `px` 后缀）。
fn parse_media_numeric_value(chars: &[char], pos: &mut usize) -> String {
    let mut s = String::new();
    while *pos < chars.len()
        && (chars[*pos].is_ascii_digit() || chars[*pos] == '.' || chars[*pos] == '-')
    {
        s.push(chars[*pos]);
        *pos += 1;
    }
    // 跳过可选的 `px` 单位
    if *pos + 1 < chars.len() && chars[*pos] == 'p' && chars[*pos + 1] == 'x' {
        *pos += 2;
    }
    s
}

/// 解析单个媒体条件，格式 `(type: value)`。
fn parse_single_media_condition(
    chars: &[char],
    pos: &mut usize,
) -> Result<MediaCondition, ParseError> {
    skip_whitespace(chars, pos);

    // 期望 (
    if *pos >= chars.len() || chars[*pos] != '(' {
        return Err(ParseError::Syntax("期望 `(` 开始媒体条件".into()));
    }
    *pos += 1;

    // 解析条件名
    skip_whitespace(chars, pos);
    let name = parse_identifier(chars, pos);

    skip_whitespace(chars, pos);

    // 期望 :
    if *pos >= chars.len() || chars[*pos] != ':' {
        return Err(ParseError::Syntax(format!(
            "期望 `:` 在媒体条件 `{name}` 中"
        )));
    }
    *pos += 1;

    skip_whitespace(chars, pos);

    // 根据条件名判断值类型
    match name.as_str() {
        "max-width" | "min-width" => {
            let raw = parse_media_numeric_value(chars, pos);
            let px = raw
                .parse::<f64>()
                .map_err(|_| ParseError::InvalidValue(format!("{name} 值: {raw}")))?;

            skip_whitespace(chars, pos);

            // 期望 )
            if *pos >= chars.len() || chars[*pos] != ')' {
                return Err(ParseError::Syntax(format!(
                    "期望 `)` 在媒体条件 {name}: {raw} 后"
                )));
            }
            *pos += 1;

            match name.as_str() {
                "max-width" => Ok(MediaCondition::MaxWidth(px)),
                _ => Ok(MediaCondition::MinWidth(px)),
            }
        },
        "prefers-color-scheme" => {
            let value = parse_identifier(chars, pos);

            skip_whitespace(chars, pos);

            // 期望 )
            if *pos >= chars.len() || chars[*pos] != ')' {
                return Err(ParseError::Syntax(format!(
                    "期望 `)` 在媒体条件 {name}: {value} 后"
                )));
            }
            *pos += 1;

            match value.as_str() {
                "dark" => Ok(MediaCondition::PrefersColorScheme(ColorScheme::Dark)),
                "light" => Ok(MediaCondition::PrefersColorScheme(ColorScheme::Light)),
                _ => Err(ParseError::InvalidValue(format!(
                    "prefers-color-scheme 值: {value}"
                ))),
            }
        },
        _ => {
            let value = parse_identifier(chars, pos);
            skip_whitespace(chars, pos);

            // 期望 )
            if *pos >= chars.len() || chars[*pos] != ')' {
                return Err(ParseError::Syntax(format!(
                    "期望 `)` 在媒体条件 {name}: {value} 后"
                )));
            }
            *pos += 1;

            Err(ParseError::Syntax(format!("未知媒体条件: {name}")))
        },
    }
}

/// 解析 `{ ... }` 块内的样式规则列表。
///
/// 调用前假设 `chars[pos]` 指向 `{`。
/// 调用后 `pos` 指向匹配的 `}` 之后。
fn parse_rules_in_block(chars: &[char], pos: &mut usize) -> Result<Vec<StyleRule>, ParseError> {
    if *pos >= chars.len() || chars[*pos] != '{' {
        return Err(ParseError::Syntax("期望 `{` 开始规则块".into()));
    }
    *pos += 1;

    let mut rules = Vec::new();

    loop {
        skip_whitespace_and_comments(chars, pos);
        if *pos >= chars.len() {
            break;
        }
        // 遇到块结束 `}` 则退出
        if chars[*pos] == '}' {
            *pos += 1;
            break;
        }

        // 块内的规则不支持嵌套 @media（简化处理）
        let selector = parse_selector(chars, pos)?;
        skip_whitespace(chars, pos);

        if *pos >= chars.len() || chars[*pos] != '{' {
            return Err(ParseError::Syntax("期望 `{` 在 @media 块规则中".into()));
        }
        *pos += 1;

        let (declarations, important) = parse_declarations_with_important(chars, pos)?;
        rules.push(StyleRule::with_important(selector, declarations, important));
    }

    Ok(rules)
}

/// 同时追踪 `!important` 标记的声明解析结果。
type DeclarationsResult = (BTreeMap<Arc<str>, PropValue>, BTreeSet<Arc<str>>);

/// 解析 `{ ... }` 内的声明列表，同时追踪 `!important` 标记的属性名。
fn parse_declarations_with_important(
    chars: &[char],
    pos: &mut usize,
) -> Result<DeclarationsResult, ParseError> {
    let mut declarations = BTreeMap::new();
    let mut important_declarations = BTreeSet::new();
    loop {
        skip_whitespace_and_comments(chars, pos);
        if *pos >= chars.len() {
            break;
        }
        if chars[*pos] == '}' {
            *pos += 1;
            break;
        }

        let (prop, value, important) = parse_declaration_with_important(chars, pos)?;
        let prop_arc: Arc<str> = Arc::from(prop);
        if important {
            important_declarations.insert(Arc::clone(&prop_arc));
        }
        declarations.insert(prop_arc, value);

        // 跳过分号
        skip_whitespace(chars, pos);
        if *pos < chars.len() && chars[*pos] == ';' {
            *pos += 1;
        }
    }
    Ok((declarations, important_declarations))
}

fn skip_whitespace(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && chars[*pos].is_ascii_whitespace() {
        *pos += 1;
    }
}

fn skip_whitespace_and_comments(chars: &[char], pos: &mut usize) {
    loop {
        skip_whitespace(chars, pos);
        if *pos + 1 < chars.len() && chars[*pos] == '/' && chars[*pos + 1] == '*' {
            *pos += 2;
            while *pos + 1 < chars.len() && !(chars[*pos] == '*' && chars[*pos + 1] == '/') {
                *pos += 1;
            }
            if *pos + 1 < chars.len() {
                *pos += 2;
            }
        } else {
            break;
        }
    }
}

fn parse_identifier(chars: &[char], pos: &mut usize) -> String {
    let mut s = String::new();
    while *pos < chars.len()
        && (chars[*pos].is_alphanumeric() || chars[*pos] == '_' || chars[*pos] == '-')
    {
        s.push(chars[*pos]);
        *pos += 1;
    }
    s
}

fn parse_selector(chars: &[char], pos: &mut usize) -> Result<Selector, ParseError> {
    if *pos >= chars.len() {
        return Err(ParseError::Syntax("空选择器".into()));
    }

    // 类选择器 `.classname`
    if chars[*pos] == '.' {
        *pos += 1;
        let name = parse_identifier(chars, pos);
        return Ok(Selector::Class(name));
    }

    // ID 选择器 `#id`
    if chars[*pos] == '#' {
        *pos += 1;
        let name = parse_identifier(chars, pos);
        return Ok(Selector::Id(name));
    }

    // 类型选择器（可能带伪类）
    let type_name = parse_identifier(chars, pos);

    // 检查伪类 `:pseudo`
    if *pos < chars.len() && chars[*pos] == ':' {
        *pos += 1;
        let _pseudo = parse_identifier(chars, pos);
        // 简化处理：忽略伪类，返回类型选择器
        return Ok(Selector::Type(type_name));
    }

    Ok(Selector::Type(type_name))
}

/// 解析单条声明 `prop: value [!important]`，返回属性名、值和是否标记 `!important`。
fn parse_declaration_with_important(
    chars: &[char],
    pos: &mut usize,
) -> Result<(String, PropValue, bool), ParseError> {
    skip_whitespace(chars, pos);
    let prop = parse_identifier(chars, pos);

    skip_whitespace(chars, pos);
    if *pos >= chars.len() || chars[*pos] != ':' {
        return Err(ParseError::Syntax(format!("期望 `:`，属性: {prop}")));
    }
    *pos += 1;
    skip_whitespace(chars, pos);

    let value = parse_value(chars, pos)?;

    // 检查 `!important`
    let important = check_important(chars, pos);

    Ok((prop, value, important))
}

/// 检查当前位置是否是 `!important` 关键字，消耗字符并返回 true，否则返回 false。
fn check_important(chars: &[char], pos: &mut usize) -> bool {
    let start = *pos;
    skip_whitespace(chars, pos);

    // 需要至少 "!important" 10 个字符
    if *pos + 10 > chars.len() {
        *pos = start;
        return false;
    }

    let slice: String = chars[*pos..*pos + 10].iter().collect();
    if slice.eq_ignore_ascii_case("!important") {
        *pos += 10;
        true
    } else {
        *pos = start;
        false
    }
}

fn parse_value(chars: &[char], pos: &mut usize) -> Result<PropValue, ParseError> {
    if *pos >= chars.len() {
        return Err(ParseError::InvalidValue("空值".into()));
    }

    // 字符串 `"..."` 或 `'...'`
    if chars[*pos] == '"' || chars[*pos] == '\'' {
        let quote = chars[*pos];
        *pos += 1;
        let mut s = String::new();
        while *pos < chars.len() && chars[*pos] != quote {
            s.push(chars[*pos]);
            *pos += 1;
        }
        if *pos < chars.len() {
            *pos += 1;
        }
        return Ok(PropValue::Str(Arc::from(s)));
    }

    // 颜色 `#RRGGBB` 或 `#RGB`
    if chars[*pos] == '#' {
        *pos += 1;
        let hex = parse_identifier(chars, pos);
        return parse_color(&hex);
    }

    // 数字（可能带单位 px, % 等）
    if chars[*pos].is_ascii_digit() || chars[*pos] == '.' {
        let mut num_str = String::new();
        while *pos < chars.len() && (chars[*pos].is_ascii_digit() || chars[*pos] == '.') {
            num_str.push(chars[*pos]);
            *pos += 1;
        }
        // 跳过单位
        parse_identifier(chars, pos);

        if let Ok(i) = num_str.parse::<i64>() {
            return Ok(PropValue::Int(i));
        }
        if let Ok(f) = num_str.parse::<f64>() {
            return Ok(PropValue::Float(ordered_float::OrderedFloat(f)));
        }
    }

    // 关键词（true/false/none/auto 或 CSS 函数如 calc/min/max/clamp）
    let keyword = parse_identifier(chars, pos);

    // 检测 CSS 函数调用：关键字后跟 `(`
    if *pos < chars.len() && chars[*pos] == '(' {
        let func_text = extract_function_call(&keyword, chars, pos)?;
        match crate::css_functions::evaluate_css_expression(&func_text) {
            Ok(Some(value)) => return Ok(value),
            Ok(None) => {}, // 不是函数调用，继续
            Err(e) => return Err(ParseError::InvalidValue(e.to_string())),
        }
    }

    match keyword.as_str() {
        "true" => Ok(PropValue::Bool(true)),
        "false" => Ok(PropValue::Bool(false)),
        _ => Ok(PropValue::Str(Arc::from(keyword))),
    }
}

/// 从当前位置提取完整的 CSS 函数调用字符串（包括函数名、括号和内部表达式）。
///
/// 调用前 `pos` 指向 `(` 之后的第一个字符。
/// 返回形如 `calc(100px - 20px)` 的完整字符串。
fn extract_function_call(
    func_name: &str,
    chars: &[char],
    pos: &mut usize,
) -> Result<String, ParseError> {
    if chars[*pos] != '(' {
        return Err(ParseError::Syntax("期望 `(` 在函数调用中".into()));
    }

    let mut depth: u32 = 0;
    let mut buf = String::from(func_name);

    loop {
        if *pos >= chars.len() {
            return Err(ParseError::Syntax("未闭合的 `(`".into()));
        }
        buf.push(chars[*pos]);

        match chars[*pos] {
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    return Err(ParseError::Syntax("意外的 `)`".into()));
                }
                depth -= 1;
                if depth == 0 {
                    *pos += 1;
                    break; // 函数调用已完整提取
                }
            },
            _ => {},
        }
        *pos += 1;
    }

    Ok(buf)
}

fn parse_color(hex: &str) -> Result<PropValue, ParseError> {
    let hex = hex.trim();
    let valid = match hex.len() {
        6 => hex.chars().all(|c| c.is_ascii_hexdigit()),
        3 => hex.chars().all(|c| c.is_ascii_hexdigit()),
        _ => false,
    };
    if !valid {
        return Err(ParseError::InvalidValue(format!("颜色: #{hex}")));
    }
    Ok(PropValue::Str(Arc::from(format!("#{hex}"))))
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_rule() {
        let rules = parse_rgss("Button { font-size: 14px; }").unwrap();
        assert_eq!(rules.len(), 1);
        assert!(rules[0].declarations.contains_key("font-size"));
    }

    #[test]
    fn parse_multiple_rules() {
        let rules = parse_rgss("Button { color: red; } Label { font-size: 12; }").unwrap();
        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn parse_class_selector() {
        let rules = parse_rgss(".primary { opacity: 0.5; }").unwrap();
        assert_eq!(rules.len(), 1);
        match &rules[0].selector {
            Selector::Class(c) => assert_eq!(c, "primary"),
            _ => panic!(),
        }
    }

    #[test]
    fn parse_bool_and_int() {
        let rules = parse_rgss("Button { disabled: false; count: 8; }").unwrap();
        let decls = &rules[0].declarations;
        assert_eq!(decls.get("disabled"), Some(&PropValue::Bool(false)));
        assert_eq!(decls.get("count"), Some(&PropValue::Int(8)));
    }

    #[test]
    fn parse_string_value() {
        let rules = parse_rgss(r#"Label { text: "hello world"; }"#).unwrap();
        let v = &rules[0].declarations;
        assert!(v.contains_key("text"));
    }

    #[test]
    fn parse_color() {
        let rules = parse_rgss("Button { bg: #FF0000; }").unwrap();
        assert!(rules[0].declarations.contains_key("bg"));
    }

    #[test]
    fn parse_incomplete_is_ok() {
        let result = parse_rgss("Button { color: red");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_with_comment() {
        let rules = parse_rgss("/* comment */ Button { color: red; }").unwrap();
        assert_eq!(rules.len(), 1);
    }

    #[test]
    fn parse_media_max_width() {
        let rules = parse_rgss(
            "@media (max-width: 768px) { \
                .page { padding: 12px; } \
            }",
        )
        .unwrap();
        assert_eq!(rules.len(), 1);
        assert!(rules[0].media_condition.is_some());
        match &rules[0].selector {
            Selector::Class(c) => assert_eq!(c, "page"),
            _ => panic!("期望 Class 选择器"),
        }
    }

    #[test]
    fn parse_media_min_width() {
        let rules = parse_rgss(
            "@media (min-width: 1024px) { \
                HBox { flex-direction: column; } \
            }",
        )
        .unwrap();
        assert_eq!(rules.len(), 1);
        let cond = rules[0].media_condition.as_ref().unwrap();
        assert!(cond.eval(1024.0, ColorScheme::Light));
        assert!(!cond.eval(800.0, ColorScheme::Light));
    }

    #[test]
    fn parse_media_prefers_color_scheme() {
        let rules = parse_rgss(
            "@media (prefers-color-scheme: dark) { \
                :root { --color-bg: #1A1A2E; } \
            }",
        )
        .unwrap();
        assert_eq!(rules.len(), 1);
        let cond = rules[0].media_condition.as_ref().unwrap();
        assert!(cond.eval(800.0, ColorScheme::Dark));
        assert!(!cond.eval(800.0, ColorScheme::Light));
    }

    #[test]
    fn parse_media_multiple_rules_in_block() {
        let rules = parse_rgss(
            "@media (max-width: 768px) { \
                Button { font-size: 12px; } \
                Label { font-size: 10px; } \
            }",
        )
        .unwrap();
        assert_eq!(rules.len(), 2);
        // 两条规则都应携带媒体条件
        assert!(rules[0].media_condition.is_some());
        assert!(rules[1].media_condition.is_some());
        // 检查选择器类型
        match &rules[0].selector {
            Selector::Type(t) => assert_eq!(t, "Button"),
            _ => panic!("期望 Type 选择器"),
        }
        match &rules[1].selector {
            Selector::Type(t) => assert_eq!(t, "Label"),
            _ => panic!("期望 Type 选择器"),
        }
    }

    #[test]
    fn parse_media_regular_rules_mixed() {
        let source = "\
            Button { color: red; } \
            @media (max-width: 600px) { \
                Button { color: blue; } \
            } \
            Label { color: black; }";
        let rules = parse_rgss(source).unwrap();
        assert_eq!(rules.len(), 3);
        // 第一条无媒体条件
        assert!(rules[0].media_condition.is_none());
        // 第二条有媒体条件
        assert!(rules[1].media_condition.is_some());
        // 第三条无媒体条件
        assert!(rules[2].media_condition.is_none());
    }

    #[test]
    fn parse_media_and_composite() {
        let rules = parse_rgss(
            "@media (min-width: 640px) and (max-width: 1024px) { \
                HBox { spacing: 8px; } \
            }",
        )
        .unwrap();
        assert_eq!(rules.len(), 1);
        let cond = rules[0].media_condition.as_ref().unwrap();
        match cond {
            MediaCondition::And(v) => assert_eq!(v.len(), 2),
            _ => panic!("期望 And 组合条件"),
        }
        // 复合条件在范围内
        assert!(cond.eval(800.0, ColorScheme::Light));
        assert!(!cond.eval(500.0, ColorScheme::Light)); // 小于 min
        assert!(!cond.eval(1200.0, ColorScheme::Light)); // 大于 max
    }

    #[test]
    fn parse_media_with_comments() {
        let rules = parse_rgss(
            "/* 媒体查询块 */ @media (max-width: 768px) { \
                /* 内部规则 */ \
                .page { padding: 12px; } \
            }",
        )
        .unwrap();
        assert_eq!(rules.len(), 1);
        assert!(rules[0].media_condition.is_some());
    }

    #[test]
    fn parse_media_unknown_condition_error() {
        let result = parse_rgss("@media (unknown: 100px) { Button { x: 1; } }");
        assert!(result.is_err());
    }

    #[test]
    fn parse_media_missing_parenthesis_error() {
        let result = parse_rgss("@media (max-width: 768px { Button { x: 1; } }");
        assert!(result.is_err());
    }

    #[test]
    fn parse_media_empty_block() {
        let rules = parse_rgss("@media (max-width: 768px) { }").unwrap();
        assert!(rules.is_empty());
    }

    #[test]
    fn parse_media_decimal_threshold() {
        let rules = parse_rgss(
            "@media (max-width: 768.5px) { \
                Button { font-size: 13px; } \
            }",
        )
        .unwrap();
        assert_eq!(rules.len(), 1);
        let cond = rules[0].media_condition.as_ref().unwrap();
        assert!(cond.eval(768.5, ColorScheme::Light));
        assert!(!cond.eval(769.0, ColorScheme::Light));
    }

    #[test]
    fn parse_media_and_case_insensitive() {
        let rules = parse_rgss(
            "@media (min-width: 640px) AND (max-width: 1024px) { \
                HBox { spacing: 8px; } \
            }",
        )
        .unwrap();
        assert_eq!(rules.len(), 1);
        let cond = rules[0].media_condition.as_ref().unwrap();
        assert!(cond.eval(800.0, ColorScheme::Light));
    }

    #[test]
    fn parse_media_number_without_px_suffix() {
        let rules = parse_rgss(
            "@media (max-width: 768) { \
                Button { color: red; } \
            }",
        )
        .unwrap();
        assert_eq!(rules.len(), 1);
        let cond = rules[0].media_condition.as_ref().unwrap();
        assert!(cond.eval(768.0, ColorScheme::Light));
        assert!(!cond.eval(769.0, ColorScheme::Light));
    }

    // ========================================================================
    // !important 测试（ST07a RED）
    // ========================================================================

    /// 解析 `!important` 声明的属性
    #[test]
    fn parse_important_single_declaration() {
        let rules = parse_rgss("Button { color: red !important; }").unwrap();
        assert_eq!(rules.len(), 1);
        assert!(rules[0].declarations.contains_key("color"));
        assert!(
            rules[0]
                .important_declarations
                .contains(&Arc::from("color"))
        );
    }

    /// 混合 important 和非 important 声明
    #[test]
    fn parse_important_mixed_with_normal() {
        let rules = parse_rgss("Button { color: red !important; font-size: 14px; }").unwrap();
        assert_eq!(rules.len(), 1);
        // color 有 !important
        assert!(
            rules[0]
                .important_declarations
                .contains(&Arc::from("color"))
        );
        // font-size 没有 !important
        assert!(
            !rules[0]
                .important_declarations
                .contains(&Arc::from("font-size"))
        );
    }

    /// 多个 !important 声明
    #[test]
    fn parse_multiple_important() {
        let rules =
            parse_rgss("Button { color: red !important; background-color: blue !important; }")
                .unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].important_declarations.len(), 2);
        assert!(
            rules[0]
                .important_declarations
                .contains(&Arc::from("color"))
        );
        assert!(
            rules[0]
                .important_declarations
                .contains(&Arc::from("background-color"))
        );
    }

    /// 没有 !important 的规则应该有空 important 集合
    #[test]
    fn parse_no_important_empty_set() {
        let rules = parse_rgss("Button { color: red; font-size: 14px; }").unwrap();
        assert_eq!(rules.len(), 1);
        assert!(rules[0].important_declarations.is_empty());
    }

    /// !important 在字符串值后
    #[test]
    fn parse_important_with_string_value() {
        let rules = parse_rgss(r#"Label { text: "hello" !important; }"#).unwrap();
        assert_eq!(rules.len(), 1);
        assert!(rules[0].important_declarations.contains(&Arc::from("text")));
    }

    /// `!important` 在 @media 块内
    #[test]
    fn parse_important_inside_media_block() {
        let rules = parse_rgss(
            "@media (max-width: 768px) { \
                Button { color: red !important; } \
            }",
        )
        .unwrap();
        assert_eq!(rules.len(), 1);
        assert!(
            rules[0]
                .important_declarations
                .contains(&Arc::from("color"))
        );
    }

    // ========================================================================
    // CSS 函数求值测试（ST11 GREEN）
    // ========================================================================

    #[test]
    fn parse_calc_in_property_value() {
        let rules = parse_rgss("Button { width: calc(100px - 20px); }").unwrap();
        assert_eq!(rules.len(), 1);
        let v = rules[0].declarations.get("width").unwrap();
        assert_eq!(v, &PropValue::Int(80));
    }

    #[test]
    fn parse_calc_with_multiplication() {
        let rules = parse_rgss("VBox { padding: calc(10px * 2); }").unwrap();
        let v = rules[0].declarations.get("padding").unwrap();
        assert_eq!(v, &PropValue::Int(20));
    }

    #[test]
    fn parse_min_function() {
        let rules = parse_rgss("Button { font-size: min(14px, 18px); }").unwrap();
        let v = rules[0].declarations.get("font-size").unwrap();
        assert_eq!(v, &PropValue::Int(14));
    }

    #[test]
    fn parse_max_function() {
        let rules = parse_rgss("HBox { spacing: max(8px, 16px); }").unwrap();
        let v = rules[0].declarations.get("spacing").unwrap();
        assert_eq!(v, &PropValue::Int(16));
    }

    #[test]
    fn parse_clamp_function() {
        let rules = parse_rgss("VBox { width: clamp(100px, 200px, 300px); }").unwrap();
        let v = rules[0].declarations.get("width").unwrap();
        assert_eq!(v, &PropValue::Int(200));
    }

    #[test]
    fn parse_calc_with_spaces_in_value() {
        let rules = parse_rgss("Button { margin: calc( 10px  +   5px * 2  ); }").unwrap();
        let v = rules[0].declarations.get("margin").unwrap();
        // 10 + (5 * 2) = 10 + 10 = 20
        assert_eq!(v, &PropValue::Int(20));
    }

    #[test]
    fn parse_calc_in_media_block() {
        let rules = parse_rgss(
            "@media (max-width: 768px) { \
                Button { font-size: calc(12px + 2px); } \
            }",
        )
        .unwrap();
        assert_eq!(rules.len(), 1);
        let v = rules[0].declarations.get("font-size").unwrap();
        assert_eq!(v, &PropValue::Int(14));
    }

    #[test]
    fn parse_keyword_not_function_is_unchanged() {
        // "none" 不是函数（不跟括号），应保持为 Str
        let rules = parse_rgss("Label { display: none; }").unwrap();
        let v = rules[0].declarations.get("display").unwrap();
        assert_eq!(v, &PropValue::Str(Arc::from("none")));
    }
}
