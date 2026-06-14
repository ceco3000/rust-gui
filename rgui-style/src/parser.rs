//! .rgss 解析器——将 CSS-like 字符串解析为 StyleRule。

use crate::selector::{Selector, StyleRule};
use rgui_core::view::PropValue;
use std::collections::BTreeMap;
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

        // 解析选择器
        let selector = parse_selector(&chars, &mut pos)?;

        // 跳过空白
        skip_whitespace(&chars, &mut pos);

        // 解析 `{ ... }`
        if pos >= chars.len() || chars[pos] != '{' {
            return Err(ParseError::Syntax("期望 `{`".into()));
        }
        pos += 1;

        let mut declarations = BTreeMap::new();
        loop {
            skip_whitespace(&chars, &mut pos);
            if pos >= chars.len() {
                break;
            }
            if chars[pos] == '}' {
                pos += 1;
                break;
            }

            let (prop, value) = parse_declaration(&chars, &mut pos)?;
            declarations.insert(Arc::from(prop), value);

            // 跳过分号
            skip_whitespace(&chars, &mut pos);
            if pos < chars.len() && chars[pos] == ';' {
                pos += 1;
            }
        }

        rules.push(StyleRule::new(selector, declarations));
    }

    Ok(rules)
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

fn parse_declaration(chars: &[char], pos: &mut usize) -> Result<(String, PropValue), ParseError> {
    skip_whitespace(chars, pos);
    let prop = parse_identifier(chars, pos);

    skip_whitespace(chars, pos);
    if *pos >= chars.len() || chars[*pos] != ':' {
        return Err(ParseError::Syntax(format!("期望 `:`，属性: {prop}")));
    }
    *pos += 1;
    skip_whitespace(chars, pos);

    let value = parse_value(chars, pos)?;
    Ok((prop, value))
}

fn parse_value(chars: &[char], pos: &mut usize) -> Result<PropValue, ParseError> {
    if *pos >= chars.len() {
        return Err(ParseError::InvalidValue("空值".into()));
    }

    // 字符串 `"..."`
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

    // 关键词（true/false/none/auto 等）
    let keyword = parse_identifier(chars, pos);
    match keyword.as_str() {
        "true" => Ok(PropValue::Bool(true)),
        "false" => Ok(PropValue::Bool(false)),
        _ => Ok(PropValue::Str(Arc::from(keyword))),
    }
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
}
