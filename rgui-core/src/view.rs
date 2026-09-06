//! 声明式视图类型：`WidgetView` / `PropValue` / `Callback` / `Key` / 消息绑定。
//!
//! D3 阶段 0：最小占位定义，保证 `WidgetSpec::view` 签名可编译。完整视图/属性系统在实现阶段补全。

use std::marker::PhantomData;

use crate::geometry::Size;

/// 边框绘制样式（D16：获焦描边边框）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    /// 边框颜色。
    pub color: Color,
    /// 边框宽度（像素）。
    pub width: f32,
    /// 描边外扩 pad（D16 P2 参数化：非硬编码 2.0）。
    pub pad: f32,
}

impl Border {
    /// 构造边框（pad 默认 2.0）。
    pub const fn new(color: Color, width: f32) -> Self {
        Self {
            color,
            width,
            pad: 2.0,
        }
    }

    /// 设置描边外扩 pad。
    pub const fn with_pad(mut self, pad: f32) -> Self {
        self.pad = pad;
        self
    }
}

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
    /// 描边边框（D16：获焦高亮外框；None = 无边框）。
    pub border: Option<Border>,
    /// 组件复用 key（D18：key-based reconcile 匹配标识；None = 位置型）。
    pub key: Option<u64>,
    /// 正文/标题字号（D23：Body 13pt 阶梯；None = from_view 默认）。
    pub font_size: Option<f32>,
    /// 语义前景色（文字；D23：非硬编码纯白）。
    pub foreground: Option<Color>,
    /// 容器主轴方向（D23 残留 P1-1：Accordion 内部 Column；None = 默认 Row）。
    pub layout_direction: Option<crate::layout::LayoutDirection>,
    /// 容器四周 padding（D23 残留 P1-2：20pt 内容边距；0 = 无）。
    pub padding: f32,
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

    /// 把视图树的消息类型从 `M` 提升为 `M2`（组合根/容器复用子组件视图）。
    ///
    /// 递归映射子节点消息；props/size/border 不变。流式：`into_iter().map().collect()`。
    #[allow(clippy::only_used_in_recursion)] // self 仅递归子节点使用（保留 self 方法签名 API）
    pub fn map_message<M2>(self, f: &impl Fn(M) -> M2) -> WidgetView<M2> {
        WidgetView {
            children: self
                .children
                .into_iter()
                .map(|c| c.map_message(f))
                .collect(),
            props: self.props,
            size: self.size,
            border: self.border,
            key: self.key,
            font_size: self.font_size,
            foreground: self.foreground,
            layout_direction: self.layout_direction,
            padding: self.padding,
            _marker: PhantomData,
        }
    }
}

impl<M> Default for WidgetView<M> {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            props: PropValue::default(),
            size: None,
            border: None,
            key: None,
            font_size: None,
            foreground: None,
            layout_direction: None,
            padding: 0.0,
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
    fn map_message_promotes_child_message_type_recursively() {
        // 子视图消息类型 u32 → 提升为 String
        let mut child: WidgetView<u32> = WidgetView::empty();
        child.props = PropValue::Int(7);
        child.size = Some(crate::geometry::Size::new(10.0, 20.0));

        let root: WidgetView<u32> = WidgetView {
            children: vec![child],
            props: PropValue::Unit,
            size: None,
            border: None,
            key: None,
            font_size: None,
            foreground: None,
            layout_direction: None,
            padding: 0.0,
            _marker: PhantomData,
        };

        let mapped: WidgetView<String> = root.map_message(&|m| format!("id={m}"));

        assert_eq!(mapped.props, PropValue::Unit);
        assert_eq!(mapped.size, None);
        assert_eq!(mapped.children.len(), 1);
        assert_eq!(mapped.children[0].props, PropValue::Int(7));
        assert_eq!(
            mapped.children[0].size,
            Some(crate::geometry::Size::new(10.0, 20.0))
        );
    }

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
