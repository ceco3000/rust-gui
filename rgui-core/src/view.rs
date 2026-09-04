//! 声明式视图类型：`WidgetView` / `PropValue` / `Callback` / `Key` / 消息绑定。
//!
//! D3 阶段 0：最小占位定义，保证 `WidgetSpec::view` 签名可编译。完整视图/属性系统在实现阶段补全。

use std::marker::PhantomData;

use crate::geometry::Size;

/// 声明式视图树节点（泛型消息）。
///
/// 手动实现 `Default`（不要求 `M: Default`，因 `PhantomData<M>` 不占数据）。
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetView<M = ()> {
    /// 子节点。
    pub children: Vec<WidgetView<M>>,
    /// 属性表。
    pub props: PropValue,
    /// 布局建议尺寸（供 LayoutEngine 计算真实 bounds；None = 由布局系统决定）。
    pub size: Option<Size>,
    _marker: PhantomData<M>,
}

impl<M> WidgetView<M> {
    /// 构造空视图。
    pub fn empty() -> Self {
        Self::default()
    }

    /// 设置布局建议尺寸。
    pub fn with_size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }
}

impl<M> Default for WidgetView<M> {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            props: PropValue::default(),
            size: None,
            _marker: PhantomData,
        }
    }
}

/// 属性值（最小枚举，完整类型在实现阶段补全）。
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PropValue {
    /// 空。
    #[default]
    Unit,
    /// 布尔。
    Bool(bool),
    /// 整数。
    Int(i64),
    /// 浮点。
    Float(f64),
    /// 字符串。
    Str(String),
    /// 颜色。
    Color(Color),
}

impl From<bool> for PropValue {
    fn from(v: bool) -> Self {
        PropValue::Bool(v)
    }
}

/// 颜色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }
}

/// 稳定键。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    /// 字符串键。
    Str(String),
    /// 数值键。
    Num(u64),
}

impl From<&str> for Key {
    fn from(s: &str) -> Self {
        Key::Str(s.to_string())
    }
}

/// 无参回调。
#[derive(Debug, Clone)]
pub struct Callback<M> {
    _marker: PhantomData<M>,
}

impl<M> Default for Callback<M> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

/// 消息绑定。
#[derive(Debug, Clone)]
pub struct MessageBinding<M> {
    _marker: PhantomData<M>,
}

impl<M> Default for MessageBinding<M> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

/// 消息处理器。
#[derive(Debug, Clone)]
pub struct MessageHandler<M> {
    _marker: PhantomData<M>,
}

impl<M> MessageHandler<M> {
    /// 占位构造。
    pub fn new() -> Self {
        Self::default()
    }
}

impl<M> Default for MessageHandler<M> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_rgb_default_alpha() {
        let c = Color::rgb(1, 2, 3);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn propvalue_from_bool() {
        assert_eq!(PropValue::from(true), PropValue::Bool(true));
    }
}
