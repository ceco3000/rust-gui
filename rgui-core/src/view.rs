//! 声明式视图类型——Color、Key、Callback 和 PropValue。
//!
//! 本模块定义框架声明式 UI 描述语言的核心值类型。
//! WidgetView 类型见本模块末尾（将在 C05 中完善 builder API）。

use crate::geometry::{Rect, Size};
use crate::id::WidgetId;
use crate::traits::AppMessage;
use ordered_float::OrderedFloat;
use std::any::Any;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

// ============================================================================
// Color
// ============================================================================

/// RGBA 颜色，各通道取值范围 0.0–1.0。
///
/// ## 色彩空间约定
///
/// - **输入**：所有 `Color` 值均以 sRGB 色彩空间存储
/// - **混合/插值**：应在 linear sRGB 空间完成（使用 `lerp_linear()`），
///   避免 gamma 校正带来的插值颜色偏差
/// - **输出**：渲染后端负责将 sRGB 值传递到 `*_Srgb` swapchain，
///   由 GPU/平台完成最终 sRGB 编码
///
/// 此约定与 D0 设计文档和 WCAG 2.1 AA 对比度公式使用的 sRGB 相对亮度一致。
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

    // ------------------------------------------------------------------
    // 色彩空间转换（sRGB ↔ linear sRGB）
    // ------------------------------------------------------------------

    /// 将当前 sRGB 值转换为 linear sRGB 色彩空间。
    ///
    /// 使用标准 sRGB 传输函数（IEC 61966-2-1）。
    /// alpha 通道保持不变。
    #[must_use]
    pub fn to_linear(self) -> Self {
        fn srgb_channel(c: f64) -> f64 {
            debug_assert!(
                (0.0..=1.0).contains(&c),
                "sRGB channel value out of range: {c}"
            );
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        Self {
            r: srgb_channel(self.r),
            g: srgb_channel(self.g),
            b: srgb_channel(self.b),
            a: self.a,
        }
    }

    /// 将当前 linear sRGB 值转换回 sRGB 色彩空间。
    ///
    /// 使用标准 sRGB 传输函数的逆函数。
    /// alpha 通道保持不变。
    #[must_use]
    pub fn from_linear(self) -> Self {
        fn linear_channel(c: f64) -> f64 {
            debug_assert!(
                (0.0..=1.0).contains(&c),
                "linear channel value out of range: {c}"
            );
            if c <= 0.0031308 {
                c * 12.92
            } else {
                1.055 * c.powf(1.0 / 2.4) - 0.055
            }
        }
        Self {
            r: linear_channel(self.r),
            g: linear_channel(self.g),
            b: linear_channel(self.b),
            a: self.a,
        }
    }

    /// 在 linear sRGB 空间中线性插值。
    ///
    /// 将两个 sRGB 颜色分别转换到 linear 空间做线性插值，
    /// 再将结果转回 sRGB，避免直接在 sRGB 空间插值导致的
    /// gamma 暗化偏差。
    ///
    /// `t` 取值范围 [0.0, 1.0]：
    /// - `t = 0.0` 返回 `self`
    /// - `t = 1.0` 返回 `other`
    #[must_use]
    pub fn lerp_linear(self, other: Self, t: f64) -> Self {
        let t = t.clamp(0.0, 1.0);
        // 当 t 为端点值时直接返回，避免 powf 精度损失
        if t <= 0.0 {
            return self;
        }
        if t >= 1.0 {
            return other;
        }
        let a = self.to_linear();
        let b = other.to_linear();
        Color {
            r: a.r + (b.r - a.r) * t,
            g: a.g + (b.g - a.g) * t,
            b: a.b + (b.b - a.b) * t,
            a: a.a + (b.a - a.a) * t,
        }
        .from_linear()
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

impl Callback {
    /// 创建一个无操作的回调。
    #[must_use]
    pub fn noop() -> Self {
        Self(Arc::new(|| {}))
    }

    /// 从闭包创建回调。
    #[must_use]
    pub fn new<F>(f: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self(Arc::new(f))
    }

    /// 调用回调。
    pub fn call(&self) {
        (self.0)();
    }
}

impl<F> From<F> for Callback
where
    F: Fn() + Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        Self::new(f)
    }
}

impl fmt::Debug for Callback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Callback").field(&"<closure>").finish()
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

impl From<Callback> for PropValue {
    fn from(cb: Callback) -> Self {
        Self::Callback(cb)
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

impl<M: AppMessage> MessageHandler<M> {
    /// 从类型擦除的消息盒进行运行时类型检查并分发。
    ///
    /// 使用 `TypeId` 验证动态类型与 `M` 是否一致。
    /// - 类型匹配时调用 handler：`Map` 返回映射后的消息，`Handle` 返回 `None`
    /// - 类型不匹配时记录错误日志并丢弃消息，返回 `None`
    ///
    /// 定义源自 D1 §11.7。
    pub fn dispatch_dynamic(&self, msg: Box<dyn Any + Send + 'static>) -> Option<M> {
        if (*msg).type_id() == std::any::TypeId::of::<M>() {
            let concrete = msg.downcast::<M>().unwrap_or_else(|_| unreachable!());
            match self {
                MessageHandler::Map(f) => Some(f(*concrete)),
                MessageHandler::Handle(f) => {
                    f(*concrete);
                    None
                },
            }
        } else {
            eprintln!("[rgui] 消息类型不匹配: 预期 {}", std::any::type_name::<M>());
            None
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

    // --- 色彩空间转换 ---

    #[test]
    fn color_to_linear_black() {
        // 黑色在两种色彩空间中相同
        let result = Color::BLACK.to_linear();
        assert!((result.r - 0.0).abs() < 1e-10);
        assert!((result.g - 0.0).abs() < 1e-10);
        assert!((result.b - 0.0).abs() < 1e-10);
        assert!((result.a - 1.0).abs() < 1e-10);
    }

    #[test]
    fn color_to_linear_white() {
        // 白色在两种色彩空间中相同
        let result = Color::WHITE.to_linear();
        assert!((result.r - 1.0).abs() < 1e-10);
        assert!((result.g - 1.0).abs() < 1e-10);
        assert!((result.b - 1.0).abs() < 1e-10);
    }

    #[test]
    fn color_srgb_to_linear_known() {
        // sRGB 0.5 → linear ~0.21404 (标准转换)
        let c = Color::new(0.5, 0.5, 0.5, 1.0);
        let linear = c.to_linear();
        let expected: f64 = ((0.5_f64 + 0.055_f64) / 1.055_f64).powf(2.4_f64);
        assert!((linear.r - expected).abs() < 1e-6);
        assert!((linear.g - expected).abs() < 1e-6);
        assert!((linear.b - expected).abs() < 1e-6);
        assert!((linear.a - 1.0).abs() < 1e-10);
    }

    #[test]
    fn color_srgb_low_value_linear() {
        // sRGB 0.01 → linear 0.01 / 12.92 (线性段)
        let c = Color::new(0.01, 0.01, 0.01, 1.0);
        let linear = c.to_linear();
        let expected = 0.01 / 12.92;
        assert!((linear.r - expected).abs() < 1e-10);
        assert!((linear.g - expected).abs() < 1e-10);
        assert!((linear.b - expected).abs() < 1e-10);
    }

    #[test]
    fn color_from_linear_known() {
        // linear 0.5 → sRGB ~0.73536 (标准逆转换)
        let c = Color::new(0.5, 0.5, 0.5, 1.0);
        let srgb = c.from_linear();
        let expected: f64 = 1.055_f64 * 0.5_f64.powf(1.0_f64 / 2.4_f64) - 0.055_f64;
        assert!((srgb.r - expected).abs() < 1e-6);
        assert!((srgb.g - expected).abs() < 1e-6);
        assert!((srgb.b - expected).abs() < 1e-6);
    }

    #[test]
    fn color_from_linear_low_value() {
        // linear 0.001 → sRGB 0.001 * 12.92 (线性段)
        let c = Color::new(0.001, 0.001, 0.001, 1.0);
        let srgb = c.from_linear();
        let expected = 0.001 * 12.92;
        assert!((srgb.r - expected).abs() < 1e-10);
    }

    #[test]
    fn color_srgb_linear_roundtrip() {
        // 完整的 sRGB → linear → sRGB 往返测试
        // 注意：膝关节精确值（0.04045/0.0031308）处于分段函数切换点，
        // 由专用边界测试覆盖此处的浮点精度问题。
        let test_values = [0.0, 0.001, 0.01, 0.04, 0.1, 0.25, 0.5, 0.75, 0.95, 1.0];
        for &r in &test_values {
            for &g in &test_values {
                for &b in &test_values {
                    let original = Color::new(r, g, b, 1.0);
                    let roundtripped = original.to_linear().from_linear();
                    assert!(
                        (original.r - roundtripped.r).abs() < 1e-10,
                        "R roundtrip failed at sRGB={r}"
                    );
                    assert!(
                        (original.g - roundtripped.g).abs() < 1e-10,
                        "G roundtrip failed at sRGB={g}"
                    );
                    assert!(
                        (original.b - roundtripped.b).abs() < 1e-10,
                        "B roundtrip failed at sRGB={b}"
                    );
                }
            }
        }
    }

    #[test]
    fn color_reverse_roundtrip() {
        // linear → sRGB → linear 往返测试
        // 膝关节精确值由专用边界测试覆盖
        let test_values = [0.0, 0.001, 0.01, 0.05, 0.2, 0.5, 0.8, 0.95, 1.0];
        for &r in &test_values {
            let original = Color::new(r, 0.0, 0.0, 0.5);
            let roundtripped = original.from_linear().to_linear();
            assert!(
                (original.r - roundtripped.r).abs() < 1e-10,
                "reverse R roundtrip failed at linear={r}"
            );
        }
    }

    #[test]
    fn alpha_preserved_during_conversion() {
        // alpha 通道在色彩空间转换中保持不变
        let c = Color::new(0.5, 0.3, 0.7, 0.33);
        assert!((c.to_linear().a - 0.33).abs() < 1e-10);
        assert!((c.from_linear().a - 0.33).abs() < 1e-10);
    }

    #[test]
    fn color_srgb_knee_boundary() {
        // sRGB 传输函数膝关节边界值 0.04045
        // 线性段公式：0.04045 / 12.92 ≈ 0.0031308
        let at_knee = Color::new(0.04045, 0.04045, 0.04045, 1.0);
        let linear = at_knee.to_linear();
        let expected = 0.04045_f64 / 12.92_f64;
        assert!((linear.r - expected).abs() < 1e-12);
        assert!((linear.g - expected).abs() < 1e-12);
        assert!((linear.b - expected).abs() < 1e-12);
    }

    #[test]
    fn color_linear_knee_boundary() {
        // linear 传输函数膝关节边界值 0.0031308
        // 线性段公式：0.0031308 * 12.92 ≈ 0.04045
        let at_knee = Color::new(0.0031308, 0.0031308, 0.0031308, 1.0);
        let srgb = at_knee.from_linear();
        let expected = 0.0031308_f64 * 12.92_f64;
        assert!((srgb.r - expected).abs() < 1e-12);
        assert!((srgb.g - expected).abs() < 1e-12);
        assert!((srgb.b - expected).abs() < 1e-12);
    }

    // --- 线性空间插值 ---

    #[test]
    fn lerp_linear_t_zero_returns_self() {
        let a = Color::new(0.2, 0.4, 0.6, 0.8);
        let b = Color::new(0.9, 0.1, 0.3, 0.2);
        let result = a.lerp_linear(b, 0.0);
        assert_eq!(result, a);
    }

    #[test]
    fn lerp_linear_t_one_returns_other() {
        let a = Color::new(0.2, 0.4, 0.6, 0.8);
        let b = Color::new(0.9, 0.1, 0.3, 0.2);
        let result = a.lerp_linear(b, 1.0);
        assert_eq!(result, b);
    }

    #[test]
    fn lerp_linear_midpoint() {
        // 在 linear 空间插值 0.5 结果不等于 sRGB 直接取平均
        let black = Color::BLACK;
        let white = Color::WHITE;
        let mid = black.lerp_linear(white, 0.5);
        // linear 空间中 gray = 0.5, 转回 sRGB = ~0.735
        // 而非 sRGB 空间的 0.5
        let expected_r: f64 = Color::new(0.5, 0.5, 0.5, 1.0).from_linear().r;
        assert!((mid.r - expected_r).abs() < 1e-6);
        assert!((mid.g - expected_r).abs() < 1e-6);
        assert!((mid.b - expected_r).abs() < 1e-6);
        // BLACK 和 WHITE 的 alpha 都是 1.0，插值结果仍为 1.0
        assert!((mid.a - 1.0).abs() < 1e-10);

        // 验证不等于 sRGB 空间的线性插值
        let srgb_mid = Color::new(0.5, 0.5, 0.5, 1.0);
        assert!(
            (mid.r - srgb_mid.r).abs() > 0.1,
            "linear-space midpoint should differ from sRGB midpoint"
        );
    }

    #[test]
    fn lerp_linear_alpha_interpolation() {
        // alpha 通道也在 linear 空间插值（alpha 本来就是线性的）
        let a = Color::new(1.0, 0.0, 0.0, 0.0);
        let b = Color::new(0.0, 0.0, 1.0, 1.0);
        let mid = a.lerp_linear(b, 0.5);
        assert!((mid.a - 0.5).abs() < 1e-10);
    }

    #[test]
    fn lerp_linear_clamps_t() {
        let a = Color::RED;
        let b = Color::BLUE;
        let below = a.lerp_linear(b, -0.5);
        let above = a.lerp_linear(b, 1.5);
        // 端点优化直接返回原值，应完全精确
        assert_eq!(below, a);
        assert_eq!(above, b);
    }

    #[test]
    fn lerp_linear_symmetric() {
        // lerp_linear(a, b, 0.3) 应等于 lerp_linear(b, a, 0.7)
        let a = Color::new(0.1, 0.2, 0.8, 0.3);
        let b = Color::new(0.9, 0.7, 0.1, 0.9);
        let forward = a.lerp_linear(b, 0.3);
        let reverse = b.lerp_linear(a, 0.7);
        assert!(
            (forward.r - reverse.r).abs() < 1e-10,
            "forward r={}, reverse r={}",
            forward.r,
            reverse.r
        );
        assert!(
            (forward.g - reverse.g).abs() < 1e-10,
            "forward g={}, reverse g={}",
            forward.g,
            reverse.g
        );
        assert!(
            (forward.b - reverse.b).abs() < 1e-10,
            "forward b={}, reverse b={}",
            forward.b,
            reverse.b
        );
        assert!(
            (forward.a - reverse.a).abs() < 1e-10,
            "forward a={}, reverse a={}",
            forward.a,
            reverse.a
        );
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
    fn callback_new_from_closure() {
        use std::sync::Arc as StdArc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let called = StdArc::new(AtomicBool::new(false));
        let called_clone = StdArc::clone(&called);
        let cb = Callback::new(move || {
            called_clone.store(true, Ordering::SeqCst);
        });
        cb.call();
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn callback_from_closure() {
        use std::sync::Arc as StdArc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let called = StdArc::new(AtomicBool::new(false));
        let called_clone = StdArc::clone(&called);
        let cb: Callback = (move || {
            called_clone.store(true, Ordering::SeqCst);
        })
        .into();
        cb.call();
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn prop_value_from_callback() {
        let cb = Callback::noop();
        let pv = PropValue::from(cb);
        assert!(matches!(pv, PropValue::Callback(_)));
    }

    #[test]
    fn callback_pointer_eq() {
        let cb_a = Callback::noop();
        let cb_b = Callback::noop();
        let cb_a2 = cb_a.clone();
        // 同一个 Arc 的 clone → pointer equality
        assert_eq!(cb_a, cb_a2);
        // 不同的 Arc → not equal
        assert_ne!(cb_a, cb_b);
    }

    #[test]
    fn prop_value_display() {
        let v = PropValue::str("hello");
        assert!(format!("{v}").contains("hello"));
    }

    // --- MessageHandler::dispatch_dynamic ---

    #[derive(Debug, Clone, PartialEq)]
    enum TestAppMsg {
        Clicked,
        TextChanged(String),
    }

    impl AppMessage for TestAppMsg {
        fn message_name(&self) -> &'static str {
            match self {
                Self::Clicked => "clicked",
                Self::TextChanged(_) => "text_changed",
            }
        }
    }

    #[test]
    fn dispatch_dynamic_map_matching_type() {
        let handler = MessageHandler::Map(Arc::new(|msg: TestAppMsg| match msg {
            TestAppMsg::Clicked => TestAppMsg::TextChanged("mapped".into()),
            other => other,
        }));
        let msg = Box::new(TestAppMsg::Clicked);
        let result = handler.dispatch_dynamic(msg);
        assert_eq!(result, Some(TestAppMsg::TextChanged("mapped".into())));
    }

    #[test]
    fn dispatch_dynamic_handle_matching_type() {
        use std::sync::Arc as StdArc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let called = StdArc::new(AtomicBool::new(false));
        let called_clone = StdArc::clone(&called);
        let handler = MessageHandler::Handle(Arc::new(move |_msg: TestAppMsg| {
            called_clone.store(true, Ordering::SeqCst);
        }));
        let msg = Box::new(TestAppMsg::Clicked);
        let result = handler.dispatch_dynamic(msg);
        assert!(result.is_none());
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn dispatch_dynamic_type_mismatch() {
        // 发送 String（非 TestAppMsg 类型）→ 应安全丢弃
        let handler = MessageHandler::Handle(Arc::new(|_msg: TestAppMsg| {
            panic!("不应被调用");
        }));
        let msg: Box<dyn Any + Send + 'static> = Box::new("not_a_message");
        let result = handler.dispatch_dynamic(msg);
        assert!(result.is_none());
    }

    #[derive(Debug, Clone, PartialEq)]
    enum OtherMsg {
        Something,
    }

    impl AppMessage for OtherMsg {
        fn message_name(&self) -> &'static str {
            "something"
        }
    }

    #[test]
    fn dispatch_dynamic_different_message_types() {
        // 发送 OtherMsg 给期望 TestAppMsg 的 handler → 应安全丢弃
        let handler = MessageHandler::Handle(Arc::new(|_msg: TestAppMsg| {
            panic!("不应被调用");
        }));
        let msg: Box<dyn Any + Send + 'static> = Box::new(OtherMsg::Something);
        let result = handler.dispatch_dynamic(msg);
        assert!(result.is_none());
    }
}
