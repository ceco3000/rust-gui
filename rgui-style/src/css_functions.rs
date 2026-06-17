//! CSS 函数求值器——calc()/min()/max()/clamp() 表达式解析与单位算术。
//!
//! D4 §2.2-2.3 定义了 `.rgss` 支持的 CSS 函数。本模块负责：
//! 1. 检测并提取 CSS 函数调用（从属性值文本中）
//! 2. 求值表达式（支持 +、-、*、/ 和括号）
//! 3. 返回计算后的 PropValue

use ordered_float::OrderedFloat;
use rgui_core::view::PropValue;

/// CSS 函数求值错误。
#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("语法错误: {0}")]
    Syntax(String),
    #[error("未知函数: {0}")]
    UnknownFunction(String),
    #[error("不支持的单位: {0}")]
    UnsupportedUnit(String),
    #[error("除零错误")]
    DivisionByZero,
    #[error("calc() 表达式求值错误")]
    CalcError(String),
    #[error("min()/max() 参数数量不足")]
    NotEnoughArgs,
}

/// CSS 数值——带可选的单位标记。
#[derive(Clone, Debug, PartialEq)]
pub struct CssNumber {
    pub value: f64,
    pub unit: Option<String>,
}

impl CssNumber {
    pub fn new_unitless(value: f64) -> Self {
        Self { value, unit: None }
    }

    pub fn new(value: f64, unit: Option<String>) -> Self {
        Self { value, unit }
    }

    /// 两数相加。单位必须相同（或至少一方无单位）。
    fn add(&self, other: &Self) -> Result<Self, EvalError> {
        let unit = resolve_unit(&self.unit, &other.unit)?;
        Ok(Self {
            value: self.value + other.value,
            unit,
        })
    }

    /// 两数相减。单位必须相同（或至少一方无单位）。
    fn sub(&self, other: &Self) -> Result<Self, EvalError> {
        let unit = resolve_unit(&self.unit, &other.unit)?;
        Ok(Self {
            value: self.value - other.value,
            unit,
        })
    }

    /// 两数相乘。结果单位遵循 CSS 规则：
    /// - 两边都无单位 → 无单位
    /// - 一边有单位 → 保留该单位
    /// - 两边都有单位 → 错误
    fn mul(&self, other: &Self) -> Result<Self, EvalError> {
        let unit = match (&self.unit, &other.unit) {
            (None, None) => None,
            (Some(u), None) => Some(u.clone()),
            (None, Some(u)) => Some(u.clone()),
            (Some(a), Some(b)) => {
                return Err(EvalError::Syntax(format!(
                    "不能乘以两个带单位的数值：{a} × {b}"
                )));
            },
        };
        Ok(Self {
            value: self.value * other.value,
            unit,
        })
    }

    /// 两数相除。
    fn div(&self, other: &Self) -> Result<Self, EvalError> {
        if other.value == 0.0 {
            return Err(EvalError::DivisionByZero);
        }
        let unit = match (&self.unit, &other.unit) {
            (None, None) => None,
            (Some(u), None) => Some(u.clone()),
            (None, Some(_)) => None,
            (Some(a), Some(b)) if a == b => None,
            (Some(a), Some(b)) => {
                return Err(EvalError::Syntax(format!(
                    "不能除以两个不同单位的数值：{a} / {b}"
                )));
            },
        };
        Ok(Self {
            value: self.value / other.value,
            unit,
        })
    }
}

impl PartialOrd for CssNumber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.unit != other.unit {
            return None;
        }
        self.value.partial_cmp(&other.value)
    }
}

/// 解析单位：两数运算时的单位统一规则。
fn resolve_unit(a: &Option<String>, b: &Option<String>) -> Result<Option<String>, EvalError> {
    match (a, b) {
        (None, None) => Ok(None),
        (Some(u), None) | (None, Some(u)) => Ok(Some(u.clone())),
        (Some(a), Some(b)) if a == b => Ok(Some(a.clone())),
        (Some(a), Some(b)) => Err(EvalError::Syntax(format!("单位不匹配：{a} vs {b}"))),
    }
}

/// Token 类型（用于词法分析）。
#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    Unit(String),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    Comma,
    Function(String), // calc/min/max/clamp
}

/// 词法分析：将 CSS 表达式字符串拆分为 token 迭代器。
struct Lexer {
    chars: Vec<char>,
    pos: usize,
}

impl Lexer {
    fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, EvalError> {
        let mut tokens = Vec::new();

        while self.pos < self.chars.len() {
            let ch = self.chars[self.pos];

            // 跳过空白
            if ch.is_ascii_whitespace() {
                self.advance();
                continue;
            }

            match ch {
                '+' => {
                    tokens.push(Token::Plus);
                    self.advance();
                },
                '-' => {
                    // 检查是否是负数（前面是运算符或开头，后面是数字）
                    if self.is_negative_number_start() {
                        tokens.push(self.read_number()?);
                    } else {
                        tokens.push(Token::Minus);
                        self.advance();
                    }
                },
                '*' => {
                    tokens.push(Token::Star);
                    self.advance();
                },
                '/' => {
                    tokens.push(Token::Slash);
                    self.advance();
                },
                '(' => {
                    tokens.push(Token::LParen);
                    self.advance();
                },
                ')' => {
                    tokens.push(Token::RParen);
                    self.advance();
                },
                ',' => {
                    tokens.push(Token::Comma);
                    self.advance();
                },
                c if c.is_ascii_digit() || c == '.' => {
                    tokens.push(self.read_number()?);
                },
                c if c.is_alphabetic() || c == '-' || c == '%' => {
                    // 可能是函数名、单位或变量。'%' 是百分比单位。'-' 可能是函数名开头或负数标识符。
                    let word = self.read_word();
                    if self.peek() == Some('(') {
                        tokens.push(Token::Function(word));
                    } else {
                        // 这是一个单位（如 px, em, rem, %）
                        tokens.push(Token::Unit(word));
                    }
                },
                _ => {
                    return Err(EvalError::Syntax(format!("意外字符: {ch}")));
                },
            }
        }

        Ok(tokens)
    }

    /// 判断当前位置的 `-` 是否是负号（而非减法运算符）。
    fn is_negative_number_start(&self) -> bool {
        if self.pos + 1 >= self.chars.len() {
            return false;
        }
        let next = self.chars[self.pos + 1];
        if !next.is_ascii_digit() && next != '.' {
            return false;
        }
        // 检查上下文：如果是开头，或前面是运算符/左括号/逗号
        if self.pos == 0 {
            return true;
        }
        let prev = self.chars[self.pos - 1];
        matches!(prev, '+' | '-' | '*' | '/' | '(' | ',') || prev.is_ascii_whitespace()
    }

    /// 读取一个数字（包括负号前缀和单位后缀），返回 Number token。
    fn read_number(&mut self) -> Result<Token, EvalError> {
        let sign = if self.chars[self.pos] == '-' {
            self.advance(); // 消费负号
            -1.0_f64
        } else {
            1.0
        };

        let num_str = self.read_digits();

        // 检查是否紧跟着单位（如 14px、50%、2em）
        if self.pos < self.chars.len() {
            let ch = self.chars[self.pos];
            if ch.is_alphabetic() || ch == '%' {
                // 注意：负数已经读完了，单位还会在后续以 Unit token 出现
                // 但 read_number 后可能紧跟着单位字符，这里需要延迟到 read_word
            }
        }

        let value = num_str
            .parse::<f64>()
            .map_err(|_| EvalError::Syntax(format!("无效数字: {num_str}")))?;

        Ok(Token::Number(value * sign))
    }

    /// 读取数字部分（不含负号、不含单位）。
    fn read_digits(&mut self) -> String {
        let mut s = String::new();
        while self.pos < self.chars.len() {
            let ch = self.chars[self.pos];
            if ch.is_ascii_digit() || ch == '.' {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        if s.is_empty() {
            s.push('0');
        }
        s
    }

    /// 读取一个单词（标识符或单位名）。
    fn read_word(&mut self) -> String {
        let mut s = String::new();
        while self.pos < self.chars.len() {
            let ch = self.chars[self.pos];
            if ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '%' {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        s
    }
}

/// 表达式解析器——将 token 流解析为 CssNumber。
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos);
        self.pos += 1;
        tok
    }

    /// 解析加法/减法表达式（最低优先级）。
    fn parse_add_sub(&mut self) -> Result<CssNumber, EvalError> {
        let mut left = self.parse_mul_div()?;
        loop {
            match self.peek() {
                Some(Token::Plus) => {
                    self.advance();
                    let right = self.parse_mul_div()?;
                    left = left.add(&right)?;
                },
                Some(Token::Minus) => {
                    self.advance();
                    let right = self.parse_mul_div()?;
                    left = left.sub(&right)?;
                },
                _ => break,
            }
        }
        Ok(left)
    }

    /// 解析乘法/除法表达式（中等优先级）。
    fn parse_mul_div(&mut self) -> Result<CssNumber, EvalError> {
        let mut left = self.parse_primary()?;
        loop {
            match self.peek() {
                Some(Token::Star) => {
                    self.advance();
                    let right = self.parse_primary()?;
                    left = left.mul(&right)?;
                },
                Some(Token::Slash) => {
                    self.advance();
                    let right = self.parse_primary()?;
                    left = left.div(&right)?;
                },
                _ => break,
            }
        }
        Ok(left)
    }

    /// 解析原子项：数字（可能带单位）、括号表达式、函数调用、变量引用。
    fn parse_primary(&mut self) -> Result<CssNumber, EvalError> {
        // 先 peek 获取当前 token 类型，避免同时持有 mutable borrow 和后续的 peek/advance
        let current_kind = match self.peek() {
            Some(Token::Number(_)) => "number",
            Some(Token::LParen) => "lparen",
            Some(Token::Function(_)) => "function",
            _ => "other",
        };

        match current_kind {
            "number" => {
                // advance 消费 Number token
                let value = match self.advance() {
                    Some(Token::Number(v)) => *v,
                    _ => unreachable!(),
                };
                // peek 检查是否有单位
                let unit = match self.peek() {
                    Some(Token::Unit(_)) => match self.advance() {
                        Some(Token::Unit(u)) => Some(u.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                Ok(CssNumber::new(value, unit))
            },
            "lparen" => {
                self.advance(); // 消费 `(`
                let result = self.parse_add_sub()?;
                match self.advance() {
                    Some(Token::RParen) => Ok(result),
                    _ => Err(EvalError::Syntax("期望 `)`".into())),
                }
            },
            "function" => {
                // 获取函数名
                let name = match self.advance() {
                    Some(Token::Function(n)) => n.clone(),
                    _ => unreachable!(),
                };
                // 消费 `(`
                match self.advance() {
                    Some(Token::LParen) => {},
                    _ => {
                        return Err(EvalError::Syntax(format!("期望 `(` 在函数 {name} 后")));
                    },
                }

                let result = match name.as_str() {
                    "calc" => {
                        let result = self.parse_add_sub()?;
                        match self.advance() {
                            Some(Token::RParen) => Ok(result),
                            _ => Err(EvalError::Syntax("期望 `)` 结束 calc()".into())),
                        }
                    },
                    "min" => {
                        let args = self.parse_comma_separated_args()?;
                        if args.is_empty() {
                            return Err(EvalError::NotEnoughArgs);
                        }
                        args.into_iter()
                            .min_by(|a, b| {
                                a.value
                                    .partial_cmp(&b.value)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .ok_or(EvalError::NotEnoughArgs)
                    },
                    "max" => {
                        let args = self.parse_comma_separated_args()?;
                        if args.is_empty() {
                            return Err(EvalError::NotEnoughArgs);
                        }
                        args.into_iter()
                            .max_by(|a, b| {
                                a.value
                                    .partial_cmp(&b.value)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .ok_or(EvalError::NotEnoughArgs)
                    },
                    "clamp" => {
                        let args = self.parse_comma_separated_args()?;
                        if args.len() != 3 {
                            return Err(EvalError::Syntax(format!(
                                "clamp() 需要 3 个参数，得到 {} 个",
                                args.len()
                            )));
                        }
                        let min = &args[0];
                        let val = &args[1];
                        let max = &args[2];
                        let result = val.value.clamp(min.value, max.value);
                        // 保留 val 的单位
                        Ok(CssNumber::new(result, val.unit.clone()))
                    },
                    _ => Err(EvalError::UnknownFunction(name)),
                }?;

                Ok(result)
            },
            _ => {
                // advance and report based on actual token
                match self.advance() {
                    Some(token) => Err(EvalError::Syntax(format!("意外 token: {token:?}"))),
                    None => Err(EvalError::Syntax("表达式不完整".into())),
                }
            },
        }
    }

    /// 解析逗号分隔的参数列表，直到遇到 `)`。
    fn parse_comma_separated_args(&mut self) -> Result<Vec<CssNumber>, EvalError> {
        let mut args = Vec::new();
        loop {
            if let Some(Token::RParen) = self.peek() {
                self.advance(); // 消费 `)`
                break;
            }
            let arg = self.parse_add_sub()?;
            args.push(arg);
            match self.peek() {
                Some(Token::Comma) => {
                    self.advance(); // 消费 `,`
                },
                Some(Token::RParen) => {
                    self.advance(); // 消费 `)`
                    break;
                },
                _ => {
                    return Err(EvalError::Syntax("期望 `,` 或 `)` 在参数列表中".into()));
                },
            }
        }
        Ok(args)
    }
}

/// 求值 CSS 表达式，返回 `PropValue`。
///
/// 支持的表达式格式：
/// - `14px` → `PropValue::Float(14.0)`（单位在后续布局时处理）
/// - `calc(100% - 20px)` → 求值结果为 `PropValue::Int` 或 `PropValue::Float`
/// - `min(10px, 20px)` → `PropValue::Float(10.0)`
/// - `max(10px, 20px)` → `PropValue::Float(20.0)`
/// - `clamp(0, val, 100)` → `PropValue::Float(val.clamp(0,100))`
///
/// 如果输入不包含 CSS 函数调用，返回 `None` 表示应由常规 `parse_value` 处理。
pub fn evaluate_css_expression(source: &str) -> Result<Option<PropValue>, EvalError> {
    let source = source.trim();

    // 快速检测：如果没有任何函数调用标记 `(`，不处理
    if !source.contains('(') {
        return Ok(None);
    }

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;

    // 检查是否以函数调用开头
    let starts_with_function = tokens
        .first()
        .is_some_and(|t| matches!(t, Token::Function(_)));

    if !starts_with_function {
        return Ok(None);
    }

    let mut parser = Parser::new(tokens);
    let result = parser.parse_primary()?;

    // 转换为 PropValue
    // 如果是整数值（无小数部分），返回 Int；否则返回 Float
    let prop = if result.value.fract() == 0.0
        && result.value >= i64::MIN as f64
        && result.value <= i64::MAX as f64
    {
        PropValue::Int(result.value as i64)
    } else {
        PropValue::Float(OrderedFloat(result.value))
    };

    Ok(Some(prop))
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // calc() 测试 --------------------------------------------------------

    #[test]
    fn calc_simple_addition() {
        let result = evaluate_css_expression("calc(10px + 20px)")
            .unwrap()
            .unwrap();
        assert_eq!(result, PropValue::Int(30));
    }

    #[test]
    fn calc_simple_subtraction() {
        let result = evaluate_css_expression("calc(100px - 30px)")
            .unwrap()
            .unwrap();
        assert_eq!(result, PropValue::Int(70));
    }

    #[test]
    fn calc_multiplication() {
        let result = evaluate_css_expression("calc(10px * 3)").unwrap().unwrap();
        assert_eq!(result, PropValue::Int(30));
    }

    #[test]
    fn calc_division() {
        let result = evaluate_css_expression("calc(100px / 2)").unwrap().unwrap();
        assert_eq!(result, PropValue::Int(50));
    }

    #[test]
    fn calc_complex_expression() {
        let result = evaluate_css_expression("calc(100px - 20px)")
            .unwrap()
            .unwrap();
        assert_eq!(result, PropValue::Int(80));
    }

    #[test]
    fn calc_float_result() {
        let result = evaluate_css_expression("calc(100px / 3)").unwrap().unwrap();
        match result {
            PropValue::Float(f) => {
                assert!((f.into_inner() - 33.333333333333336).abs() < 0.01);
            },
            _ => panic!("期望 Float"),
        }
    }

    #[test]
    fn calc_with_parentheses() {
        let result = evaluate_css_expression("calc((10px + 5px) * 2)")
            .unwrap()
            .unwrap();
        assert_eq!(result, PropValue::Int(30));
    }

    #[test]
    fn calc_with_negation() {
        let result = evaluate_css_expression("calc(20px - 50px)")
            .unwrap()
            .unwrap();
        assert_eq!(result, PropValue::Int(-30));
    }

    #[test]
    fn calc_with_spaces() {
        let result = evaluate_css_expression("calc( 10px  +   20px )")
            .unwrap()
            .unwrap();
        assert_eq!(result, PropValue::Int(30));
    }

    #[test]
    fn calc_without_units() {
        let result = evaluate_css_expression("calc(5 + 3 * 2)").unwrap().unwrap();
        assert_eq!(result, PropValue::Int(11));
    }

    #[test]
    fn calc_nested_parentheses() {
        // 单位安全的嵌套括号表达式：calc((2 + 3) * (4 - 1))
        // (2+3)*(4-1) = 5*3 = 15
        let result = evaluate_css_expression("calc((2 + 3) * (4 - 1))")
            .unwrap()
            .unwrap();
        assert_eq!(result, PropValue::Int(15));
    }

    #[test]
    fn calc_division_zero_errors() {
        let result = evaluate_css_expression("calc(10px / 0)");
        assert!(result.is_err());
    }

    // min() 测试 ---------------------------------------------------------

    #[test]
    fn min_two_values() {
        let result = evaluate_css_expression("min(10px, 20px)").unwrap().unwrap();
        assert_eq!(result, PropValue::Int(10));
    }

    #[test]
    fn min_three_values() {
        let result = evaluate_css_expression("min(30px, 10px, 20px)")
            .unwrap()
            .unwrap();
        assert_eq!(result, PropValue::Int(10));
    }

    #[test]
    fn min_with_expression() {
        let result = evaluate_css_expression("min(calc(10px + 5px), 20px)")
            .unwrap()
            .unwrap();
        assert_eq!(result, PropValue::Int(15));
    }

    // max() 测试 ---------------------------------------------------------

    #[test]
    fn max_two_values() {
        let result = evaluate_css_expression("max(10px, 20px)").unwrap().unwrap();
        assert_eq!(result, PropValue::Int(20));
    }

    #[test]
    fn max_three_values() {
        let result = evaluate_css_expression("max(10px, 30px, 20px)")
            .unwrap()
            .unwrap();
        assert_eq!(result, PropValue::Int(30));
    }

    #[test]
    fn max_with_expression() {
        let result = evaluate_css_expression("max(calc(10px + 5px), 20px)")
            .unwrap()
            .unwrap();
        assert_eq!(result, PropValue::Int(20));
    }

    // clamp() 测试 -------------------------------------------------------

    #[test]
    fn clamp_above_max() {
        let result = evaluate_css_expression("clamp(10px, 50px, 30px)")
            .unwrap()
            .unwrap();
        // val=50, max=30 → clamped to 30
        assert_eq!(result, PropValue::Int(30));
    }

    #[test]
    fn clamp_below_min() {
        let result = evaluate_css_expression("clamp(10px, 5px, 30px)")
            .unwrap()
            .unwrap();
        // val=5, min=10 → clamped to 10
        assert_eq!(result, PropValue::Int(10));
    }

    #[test]
    fn clamp_within_range() {
        let result = evaluate_css_expression("clamp(10px, 20px, 30px)")
            .unwrap()
            .unwrap();
        assert_eq!(result, PropValue::Int(20));
    }

    #[test]
    fn clamp_exact_boundary() {
        let result = evaluate_css_expression("clamp(10px, 10px, 30px)")
            .unwrap()
            .unwrap();
        assert_eq!(result, PropValue::Int(10));
    }

    // 非函数表达式的正常值不处理 -----------------------------------------

    #[test]
    fn plain_number_returns_none() {
        let result = evaluate_css_expression("14px").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn plain_color_returns_none() {
        let result = evaluate_css_expression("#FF0000").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn plain_string_returns_none() {
        let result = evaluate_css_expression(r#""hello""#).unwrap();
        assert!(result.is_none());
    }

    // 错误测试 -----------------------------------------------------------

    #[test]
    fn unknown_function_error() {
        let result = evaluate_css_expression("unknown(10px)");
        assert!(result.is_err());
    }

    #[test]
    fn calc_malformed_error() {
        let result = evaluate_css_expression("calc(10px +)");
        assert!(result.is_err());
    }

    #[test]
    fn clamp_wrong_arg_count_error() {
        let result = evaluate_css_expression("clamp(10px, 20px)");
        assert!(result.is_err());
    }

    #[test]
    fn empty_min_error() {
        let result = evaluate_css_expression("min()");
        assert!(result.is_err());
    }
}
