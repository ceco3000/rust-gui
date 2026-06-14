//! 声明式视图类型——Color、Key、Callback 和 PropValue。
//!
//! 本模块定义框架声明式 UI 描述语言的核心值类型。
//! WidgetView 类型见本模块末尾（将在 C05 中完善 builder API）。

use crate::geometry::{Rect, Size};
use crate::id::WidgetId;
use crate::traits::AppMessage;
use ordered_float::OrderedFloat;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

// ============================================================================
// Color
// ============================================================================

/// RGBA 颜色，各通道取值范围 0.0–1.0（sRGB 色彩空间）。
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    pub const RED: Self = Self::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Self = Self::new(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    #[must_use]
    pub const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    #[must_use]
    pub const fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    #[must_use]
    pub fn to_u8_array(self) -> [u8; 4] {
        let r = (self.r.clamp(0.0, 1.0) * 255.0).round() as u8;
        let g = (self.g.clamp(0.0, 1.0) * 255.0).round() as u8;
        let b = (self.b.clamp(0.0, 1.0) * 255.0).round() as u8;
        let a = (self.a.clamp(0.0, 1.0) * 255.0).round() as u8;
        [r, g, b, a]
    }

    #[must_use]
    pub fn with_alpha(self, a: f64) -> Self {
        Self { a, ..self }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rgba({:.2}, {:.2}, {:.2}, {:.2})",
            self.r, self.g, self.b, self.a
        )
    }
}

// ============================================================================
// Key
// ============================================================================

/// 列表 diff 的稳定标识（类比 React key）。
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Key(pub Arc<str>);

impl Key {
    #[must_use]
    pub fn new(s: impl Into<Arc<str>>) -> Self {
        Self(s.into())
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "key:{}", self.0)
    }
}

impl From<&str> for Key {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Key {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

// ============================================================================
// Callback
// ============================================================================

/// 回调值类型（占位——完整定义见 D5 事件系统）。
#[derive(Clone)]
pub struct Callback(Arc<dyn Fn() + Send + Sync + 'static>);

impl fmt::Debug for Callback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Callback").field(&"<closure>").finish()
    }
}

impl Callback {
    #[must_use]
    pub fn noop() -> Self {
        Self(Arc::new(|| {}))
    }
}

impl PartialEq for Callback {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Default for Callback {
    fn default() -> Self {
        Self::noop()
    }
}

// ============================================================================
// PropValue
// ============================================================================

/// 属性值类型，定义源自 D0 §5.1。
#[derive(Clone, PartialEq, Debug)]
pub enum PropValue {
    Str(Arc<str>),
    Bool(bool),
    Int(i64),
    Float(OrderedFloat<f64>),
    Color(Color),
    Size(Size),
    Rect(Rect),
    List(Vec<PropValue>),
    Map(BTreeMap<Arc<str>, PropValue>),
    Enum(Arc<str>),
    Callback(Callback),
}

impl PropValue {
    #[must_use]
    pub fn str(s: impl Into<Arc<str>>) -> Self {
        Self::Str(s.into())
    }

    #[must_use]
    pub fn bool(b: bool) -> Self {
        Self::Bool(b)
    }

    #[must_use]
    pub fn int(i: i64) -> Self {
        Self::Int(i)
    }
}

impl From<&str> for PropValue {
    fn from(s: &str) -> Self {
        Self::Str(Arc::from(s))
    }
}

impl From<String> for PropValue {
    fn from(s: String) -> Self {
        Self::Str(Arc::from(s))
    }
}

impl From<bool> for PropValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<i64> for PropValue {
    fn from(i: i64) -> Self {
        Self::Int(i)
    }
}

impl From<f64> for PropValue {
    fn from(f: f64) -> Self {
        Self::Float(OrderedFloat(f))
    }
}

impl From<Color> for PropValue {
    fn from(c: Color) -> Self {
        Self::Color(c)
    }
}

impl From<Size> for PropValue {
    fn from(s: Size) -> Self {
        Self::Size(s)
    }
}

impl From<Rect> for PropValue {
    fn from(r: Rect) -> Self {
        Self::Rect(r)
    }
}

impl fmt::Display for PropValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Str(s) => write!(f, "\"{s}\""),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Int(i) => write!(f, "{i}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::Color(c) => write!(f, "{c}"),
            Self::Size(s) => write!(f, "{s}"),
            Self::Rect(r) => write!(f, "{r}"),
            Self::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            },
            Self::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            },
            Self::Enum(v) => write!(f, ".{v}"),
            Self::Callback(_) => write!(f, "Callback(…)"),
        }
    }
}

// ============================================================================
// MessageHandler
// ============================================================================

/// 消息处理方式，定义源自 D0 §5.1。
#[derive(Clone)]
pub enum MessageHandler<M: AppMessage> {
    /// 将子消息映射为父消息。
    Map(Arc<dyn Fn(M) -> M + Send + Sync>),
    /// 直接处理消息。
    Handle(Arc<dyn Fn(M) + Send + Sync>),
}

impl<M: AppMessage> fmt::Debug for MessageHandler<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Map(_) => f.debug_tuple("Map").field(&"<closure>").finish(),
            Self::Handle(_) => f.debug_tuple("Handle").field(&"<closure>").finish(),
        }
    }
}

impl<M: AppMessage> PartialEq for MessageHandler<M> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Map(a), Self::Map(b)) => Arc::ptr_eq(a, b),
            (Self::Handle(a), Self::Handle(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

// ============================================================================
// MessageBinding
// ============================================================================

/// 消息绑定：子组件的消息 → 父组件的处理方式。
///
/// 定义源自 D0 §5.1。
#[derive(Clone, Debug)]
pub struct MessageBinding<M: AppMessage> {
    /// 消息来源 widget ID。
    pub source: WidgetId,
    /// 消息名称过滤（None 表示匹配所有消息）。
    pub message_name: Option<&'static str>,
    /// 处理方式。
    pub handler: MessageHandler<M>,
}

impl<M: AppMessage> PartialEq for MessageBinding<M> {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
            && self.message_name == other.message_name
            && self.handler == other.handler
    }
}

// ============================================================================
// WidgetView
// ============================================================================

/// WidgetView 是轻量值类型，描述 UI 结构。
///
/// 由 `WidgetSpec::view()` 返回，框架负责 diff 并应用到 retained tree。
///
/// ## 设计约束（D0 §7 不变式 3）
///
/// - 不持有状态引用（所有数据是值或 Arc）
/// - 不包含闭包（消息通过 Message 类型传递）
/// - Clone 开销可控（props 和 children 数量通常在 100 以内）
///
/// 定义源自 D0 §5.1。
#[derive(Clone, PartialEq, Debug)]
pub struct WidgetView<M: AppMessage> {
    /// widget 类型名（用于查找 WidgetSpec 注册项）。
    pub widget_type: &'static str,
    /// 可选的稳定 ID（用于跨 diff 追踪同一逻辑节点）。
    pub id: Option<WidgetId>,
    /// 列表 key（用于列表 reconciliation）。
    pub key: Option<Key>,
    /// 属性映射。
    pub props: BTreeMap<&'static str, PropValue>,
    /// 子视图。
    pub children: Vec<WidgetView<M>>,
    /// 消息绑定。
    pub message_bindings: Vec<MessageBinding<M>>,
}

impl<M: AppMessage> WidgetView<M> {
    /// 创建新的 WidgetView。
    #[must_use]
    pub fn new(widget_type: &'static str) -> Self {
        Self {
            widget_type,
            id: None,
            key: None,
            props: BTreeMap::new(),
            children: Vec::new(),
            message_bindings: Vec::new(),
        }
    }

    /// 设置稳定 ID。
    #[must_use]
    pub fn id(mut self, id: WidgetId) -> Self {
        self.id = Some(id);
        self
    }

    /// 设置列表 key。
    #[must_use]
    pub fn key(mut self, key: impl Into<Key>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// 添加单个属性。
    #[must_use]
    pub fn prop(mut self, name: &'static str, value: impl Into<PropValue>) -> Self {
        self.props.insert(name, value.into());
        self
    }

    /// 批量添加属性。
    #[must_use]
    pub fn props(mut self, props: impl IntoIterator<Item = (&'static str, PropValue)>) -> Self {
        self.props.extend(props);
        self
    }

    /// 添加子视图。
    #[must_use]
    pub fn child(mut self, child: WidgetView<M>) -> Self {
        self.children.push(child);
        self
    }

    /// 批量添加子视图。
    #[must_use]
    pub fn children(mut self, children: impl IntoIterator<Item = WidgetView<M>>) -> Self {
        self.children.extend(children);
        self
    }

    /// 添加消息绑定。
    #[must_use]
    pub fn on(
        mut self,
        source: WidgetId,
        message_name: Option<&'static str>,
        handler: MessageHandler<M>,
    ) -> Self {
        self.message_bindings.push(MessageBinding {
            source,
            message_name,
            handler,
        });
        self
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Color ---

    #[test]
    fn color_default_is_black() {
        assert_eq!(Color::default(), Color::BLACK);
    }

    #[test]
    fn color_to_u8_array() {
        assert_eq!(Color::WHITE.to_u8_array(), [255, 255, 255, 255]);
        assert_eq!(Color::BLACK.to_u8_array(), [0, 0, 0, 255]);
        assert_eq!(Color::TRANSPARENT.to_u8_array(), [0, 0, 0, 0]);
    }

    #[test]
    fn color_with_alpha() {
        let c = Color::RED.with_alpha(0.5);
        assert_eq!(c.a, 0.5);
        assert_eq!(c.r, 1.0);
    }

    #[test]
    fn color_display() {
        assert_eq!(format!("{}", Color::WHITE), "rgba(1.00, 1.00, 1.00, 1.00)");
    }

    // --- Key ---

    #[test]
    fn key_equality() {
        let k1 = Key::new("item-1");
        let k2 = Key::new("item-1");
        let k3 = Key::new("item-2");
        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn key_from_str() {
        let k: Key = "hello".into();
        assert_eq!(k, Key::new("hello"));
    }

    #[test]
    fn key_hash_consistent() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        Key::new("a").hash(&mut h1);
        Key::new("a").hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    // --- PropValue ---

    #[test]
    fn prop_value_str_from_str() {
        let v: PropValue = "hello".into();
        assert_eq!(v, PropValue::Str(Arc::from("hello")));
    }

    #[test]
    fn prop_value_bool_construct() {
        assert_eq!(PropValue::from(true), PropValue::Bool(true));
        assert_eq!(PropValue::from(false), PropValue::Bool(false));
    }

    #[test]
    fn prop_value_float_equality() {
        let a = PropValue::Float(OrderedFloat(2.5));
        let b = PropValue::Float(OrderedFloat(2.5));
        assert_eq!(a, b);
    }

    #[test]
    fn prop_value_color_roundtrip() {
        let c = Color::rgb(0.2, 0.4, 0.6);
        let v = PropValue::from(c);
        assert_eq!(v, PropValue::Color(c));
    }

    #[test]
    fn prop_value_size_roundtrip() {
        let s = Size::new(100.0, 200.0);
        let v = PropValue::from(s);
        assert_eq!(v, PropValue::Size(s));
    }

    #[test]
    fn prop_value_rect_roundtrip() {
        let r = Rect::new(0.0, 0.0, 50.0, 50.0);
        let v = PropValue::from(r);
        assert_eq!(v, PropValue::Rect(r));
    }

    #[test]
    fn prop_value_list_equality() {
        let a = PropValue::List(vec![PropValue::Int(1), PropValue::Int(2)]);
        let b = PropValue::List(vec![PropValue::Int(1), PropValue::Int(2)]);
        assert_eq!(a, b);
    }

    #[test]
    fn prop_value_enum_equality() {
        let a = PropValue::Enum(Arc::from("Primary"));
        let b = PropValue::Enum(Arc::from("Primary"));
        assert_eq!(a, b);
    }

    #[test]
    fn prop_value_map_equality() {
        let mut m1 = BTreeMap::new();
        m1.insert(Arc::from("k"), PropValue::Int(1));
        let mut m2 = BTreeMap::new();
        m2.insert(Arc::from("k"), PropValue::Int(1));
        assert_eq!(PropValue::Map(m1), PropValue::Map(m2));
    }

    #[test]
    fn callback_default_is_noop() {
        let _cb = Callback::default();
    }

    #[test]
    fn prop_value_display() {
        let v = PropValue::str("hello");
        assert!(format!("{v}").contains("hello"));
    }
}
