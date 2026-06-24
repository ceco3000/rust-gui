# D1：组件模型与 WidgetSpec 设计

> **文档定位：** 本文档是《Rust GUI 框架总体设计》（D0）的子系统详细设计，定义组件模型的完整 trait 体系、声明式视图数据类型、组件生命周期、第三方组件协议和 `html!` 宏设计。

> **前置阅读：** [Rust GUI 框架总体设计](./Rust%20GUI%20框架总体设计.md)（D0）第 3 章（核心 Trait 体系）、第 5 章（关键数据结构）。

> **对应路线书章节：** §5.2（组件模型）、§5.3（状态管理）。

> **状态：** 初版。

---

## 目录

1. [设计目标与范围](#1-设计目标与范围)
    1. [术语约定](#14-术语约定)
2. [WidgetSpec trait 完整定义](#2-widgetspec-trait-完整定义)
    2. [关联类型 trait 的完整定义（引用自 D0）](#24-关联类型-trait-的完整定义引用自-d0)
3. [WidgetView 数据类型](#3-widgetview-数据类型)
    3. [WidgetId 类型](#33-widgetid-类型)
4. [组件注册表（WidgetRegistry）](#4-组件注册表widgetregistry)
5. [组件生命周期](#5-组件生命周期)
6. [Context 类型详细定义](#6-context-类型详细定义)
7. [html! 宏设计](#7-html-宏设计)
8. [派生宏设计](#8-派生宏设计)
9. [第三方组件协议](#9-第三方组件协议)
10. [与其他子系统的交互](#10-与其他子系统的交互)
11. [边界情况处理](#11-边界情况处理)
12. [验证标准](#12-验证标准)

---

## 1. 设计目标与范围

### 1.1 本文档解决什么问题

1. 定义 `WidgetSpec` trait 的完整签名、各方法的契约和调用时机
2. 定义 `WidgetView` 数据类型的完整结构及其在 diff 系统中的角色
3. 定义组件的完整生命周期（创建 → 挂载 → 更新 → 卸载）
4. 定义第三方组件如何与框架互操作（注册、发现、消息路由）
5. 定义 `html!` 声明式宏的语法规则和展开逻辑

### 1.2 不在本文档范围内

- diff 算法和 patch 机制的具体实现（见 D2 状态管理）
- 具体组件的实现代码（Button、DataGrid 等属于实现阶段）
- 布局引擎的详细设计（见 D3 渲染管线）
- 样式系统如何解析 `.rgss` 文件（见 D4 样式系统）

### 1.3 设计原则

1. **组件只能修改自身状态**：`update()` 通过 `StoreAccessMut` 只能写自己的 `Self::State`，对其他 widget 的状态只能读
2. **view() 是纯函数**：从 `&Self::State` 派生 `WidgetView`，不得有副作用
3. **框架持有实例态和缓存态**：第三方组件只持有持久业务状态（`PersistState`）
4. **消息通过类型系统传递**：不使用闭包或函数指针作为事件回调，所有交互通过 `Message` 枚举值传递

### 1.4 术语约定

本文档中部分术语有明确的语义区别：

| 术语 | 含义 | 对应代码 |
|------|------|---------|
| **组件（component）** | 实现 `WidgetSpec` trait 的类型，是 UI 功能的可重用单元 | `struct Button; impl WidgetSpec for Button` |
| **widget** | 组件在 widget 树中的运行时实例，拥有 WidgetId 和状态存储 | `Box<dyn WidgetSpec>` + WidgetId |
| **widget 树** | 运行时 widget 实例构成的树结构，由框架维护 | `WidgetTree`（定义见 D2） |
| **视图（view）** | `view()` 返回的声明式 `WidgetView` 值，描述 UI 结构 | `WidgetView<M>` |
| **widget_type** | 组件的唯一名称字符串，用于 WidgetRegistry 查找 | `"rgui_components::Button"` |

> 后文遵循此约定：讨论类型定义和实现时使用"组件"，讨论运行时实例和树结构时使用"widget"。

---

## 2. WidgetSpec trait 完整定义

### 2.1 trait 签名

```rust
/// 组件规范：第三方组件实现的核心 trait。
///
/// # 设计约束
///
/// - 所有方法从框架运行时接收上下文，不能自行访问全局状态
/// - 状态读写通过 `StoreAccess` / `StoreAccessMut` 完成（见 D2）
/// - `view()` 必须是纯函数（无副作用）
/// - `paint()` 可访问渲染缓存，但缓存由框架持有
///
/// # 线程安全
///
/// `Send + Sync + 'static` 约束保证组件可在多线程环境中使用。
/// 框架在后台线程执行 layout 和 paint，组件必须满足此约束。
pub trait WidgetSpec: Send + Sync + 'static {
    /// 组件持有的业务状态类型。
    ///
    /// 此状态进入 StateStore 的 persistent 区，可快照、可迁移。
    /// 约束：不得包含 GPU 资源句柄、平台句柄、文件描述符。
    type State: PersistState;

    /// 组件产生的消息类型。
    ///
    /// 推荐使用枚举定义所有可能的 UI 交互。
    type Message: AppMessage;

    // ---- 标识 ----

    /// 组件的唯一名称。
    ///
    /// 用于调试输出、日志、组件注册表和序列化。
    /// 返回 `&'static str` 而非 `String`，避免堆分配。
    /// 约定格式：`"crate_name::TypeName"`（如 `"rgui_components::Button"`）。
    fn name(&self) -> &'static str;

    // ---- 视图派生 ----

    /// 从持久状态派生声明式视图。
    ///
    /// # 契约
    ///
    /// - **纯函数**：不得写文件、发网络请求、修改全局状态
    /// - **确定性**：相同 state + ctx 输入应产生相同 WidgetView 输出
    /// - **快速**：在每帧的 dirty 节点重建路径上调用，应避免重计算
    ///
    /// # 参数
    ///
    /// - `state`: 只读业务状态引用
    /// - `ctx`: 视图上下文（主题、区域设置、窗口尺寸）
    fn view(&self, state: &Self::State, ctx: &ViewContext) -> WidgetView<Self::Message>;

    // ---- 消息处理 ----

    /// 处理来自 UI 的消息。
    ///
    /// # 契约
    ///
    /// - 只能修改 **自身的** 持久状态（通过 `state: &mut Self::State`）
    /// - 可以读取 **其他 widget** 的状态（通过 `ctx.store`），自动建立订阅
    /// - 可以发送事件到父组件（通过 `ctx.event_sender`）
    /// - 不得阻塞（不应在此方法中执行 I/O 或重计算）
    ///
    /// # 参数
    ///
    /// - `msg`: 消息值
    /// - `state`: 可变自身业务状态
    /// - `ctx`: 更新上下文（状态存储访问、事件发送、焦点信息）
    fn update(
        &self,
        msg: Self::Message,
        state: &mut Self::State,
        ctx: &mut UpdateContext,
    );

    // ---- 测量 ----

    /// 纯测量：根据约束计算组件期望尺寸。
    ///
    /// # 契约
    ///
    /// - 不写状态（接收 `&Self::State`，不是 `&mut`）
    /// - 不产生副作用
    /// - 结果可缓存（相同 state + constraints + ctx 产生相同 Size）
    ///
    /// # 参数
    ///
    /// - `state`: 只读业务状态（某些组件的期望尺寸依赖于内容）
    /// - `constraints`: 来自父组件或布局引擎的尺寸约束
    /// - `ctx`: 测量上下文（字体度量、DPI 缩放）
    fn measure(
        &self,
        state: &Self::State,
        constraints: BoxConstraints,
        ctx: &MeasureContext,
    ) -> Size;

    // ---- 绘制 ----

    /// 绘制：将当前状态转换为绘制指令。
    ///
    /// # 契约
    ///
    /// - 可以访问渲染缓存（`ctx.glyph_atlas`、`ctx.render_cache`）
    /// - 缓存由框架持有，组件不能自行管理缓存生命周期
    /// - 不修改状态
    ///
    /// # 参数
    ///
    /// - `state`: 只读业务状态
    /// - `bounds`: 组件在窗口中的矩形区域（布局结果）
    /// - `ctx`: 绘制上下文（字形 Atlas、裁剪区域、渲染缓存）
    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext);

    // ---- 无障碍 ----

    /// 生成无障碍节点。
    ///
    /// # 调用时机
    ///
    /// 此方法在布局计算（measure）完成后、paint() 之前被调用。
    /// 框架将结果推入 AccessibilityTree（见 D6）。
    ///
    /// # 契约
    ///
    /// - 框架在布局完成后调用此方法
    /// - 结果推入 AccessibilityTree
    /// - 不修改状态
    ///
    /// # 参数
    ///
    /// - `state`: 只读业务状态
    /// - `ctx`: 无障碍上下文（可见区域、焦点路径）
    fn accessibility(
        &self,
        state: &Self::State,
        ctx: &AccessContext,
    ) -> AccessibilityNode;
}
```

### 2.2 各方法的调用时序

```
组件生命周期中的调用顺序：

1. 组件注册   → WidgetRegistry::register::<T>()  （应用启动时，一次性）
2. 初始挂载   → view() → diff → 确定布局 → 挂载
3. 每帧循环   →
   a. 事件分发  → update()          （用户交互触发）
   b. 视图重建  → view()            （dirty 标记后）
   c. 布局重算  → measure()         （尺寸约束变化后）
   d. 无障碍    → accessibility()    （布局完成后）
   e. 场景图    → paint()           （生成绘制指令）
   f. GPU 提交  → （RenderBackend 处理）
4. 卸载       → 框架释放 StateStore 中的状态
```

### 2.3 调用频率约束

| 方法 | 调用频率 | 性能约束 |
|------|---------|---------|
| `view()` | 仅 dirty widget，每帧最多 1 次 | < 100µs（1000 节点 diff 基准） |
| `update()` | 事件驱动，非每帧 | 不得阻塞（禁止 I/O） |
| `measure()` | 仅 dirty 子树，布局阶段 | < 50µs（可缓存） |
| `paint()` | 仅 dirty widget，场景图生成阶段 | < 200µs（生成 DrawCommand 列表） |
| `accessibility()` | 每帧，所有可见 widget | < 50µs（生成单个节点） |

---

### 2.4 关联类型 trait 的完整定义（引用自 D0）

以下 trait 在 D0 第 3 章核心 Trait 体系中已完整定义。此处列出本文档使用的方法摘要，便于独立理解 WidgetSpec 的关联类型约束。

#### AppMessage trait

```rust
/// 组件消息：WidgetSpec::Message 的约束 trait。
///
/// 完整定义见 D0 第 3.2 节。
pub trait AppMessage: 'static + Send + Sync + Debug + Clone {
    /// 返回消息的名称（用于日志、调试和消息路由）。
    fn message_name(&self) -> &'static str;
}
```

#### PersistState trait

```rust
/// 持久状态：WidgetSpec::State 的约束 trait。
///
/// 实现此 trait 的状态可序列化、可快照、可在快速重启后恢复。
/// 完整定义见 D0 第 3.3 节。
pub trait PersistState: 'static + Send + Sync + Debug + Clone + Default + erased_serde::Serialize {
    /// 返回 schema 类型标识（用于序列化/反序列化路由）。
    fn schema_name() -> &'static str;

    /// 返回 schema 版本号（用于数据迁移判断）。
    fn schema_version() -> u32;

    /// 返回 `&dyn Any` 引用，用于类型擦除后的向下转型。
    /// 通常由 `#[derive(PersistState)]` 自动生成。
    fn as_any(&self) -> &dyn std::any::Any;

    /// 返回 `&mut dyn Any` 引用，用于类型擦除后的可变向下转型。
    /// 通常由 `#[derive(PersistState)]` 自动生成。
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
```

> **注意**：`AppMessage` 和 `PersistState` 可通过派生宏自动实现，见第 8.2-8.3 节。

---

## 3. WidgetView 数据类型

### 3.1 完整定义

```rust
use std::collections::BTreeMap;
use std::sync::Arc;

/// 声明式视图：view() 的返回值，描述 UI 结构。
///
/// # 设计约束
///
/// - 轻量值类型：Clone 开销可控（props 和 children 数量通常在 100 以内）
/// - 不持有状态引用：所有数据是值或 Arc
/// - 不包含闭包：消息通过 Message 枚举值传递
/// - 实现 PartialEq：框架使用结构相等性做 diff
#[derive(Clone, PartialEq, Debug)]
pub struct WidgetView<M: AppMessage> {
    /// 组件类型名，对应 widget_type 字段，用于 WidgetRegistry 查找。
    ///
    /// 框架通过此字段查找 WidgetSpec 实现。
    /// 约定格式：`"crate_name::TypeName"`（如 `"rgui_components::Button"`）。
    pub widget_type: &'static str,

    /// 可选的稳定 ID。
    ///
    /// 用于跨 diff 追踪同一逻辑节点。设置后，即使 widget 在树中移动位置，
    /// 框架也能识别为同一节点并保留其状态。
    /// 不设置时，框架以树位置（父节点 + 索引）作为身份标识。
    pub id: Option<WidgetId>,

    /// 列表 key（用于列表 reconciliation）。
    ///
    /// 在动态列表中，key 帮助框架识别哪些项被添加、删除或移动。
    /// 类比 React 的 `key` prop。
    /// 未设置时，框架使用最小编辑距离算法。
    pub key: Option<Key>,

    /// 属性映射。
    ///
    /// 键为属性名（如 `"text"`、`"variant"`、`"on_click"`），
    /// 值为类型化属性值。
    pub props: BTreeMap<&'static str, PropValue>,

    /// 子视图列表。
    ///
    /// 空 Vec 表示叶子节点。子视图顺序决定默认布局顺序。
    pub children: Vec<WidgetView<M>>,

    /// 消息绑定：子组件消息 → 父组件处理。
    ///
    /// 每个绑定描述一个子组件能产生的消息类型，以及父组件如何处理。
    pub message_bindings: Vec<MessageBinding<M>>,
}
```

> **性能说明**：`WidgetView` 的 `PartialEq` 实现（derive）为递归全树比较，复杂度 O(n)（n = 子树节点数）。对于深层嵌套（最大 256 层）或大规模列表（单层 > 1000 节点）场景，框架 diff 引擎（见 D2）将 `WidgetView` 包装为 `DiffNode`，`DiffNode` 在构造时计算并缓存结构哈希。diff 阶段优先比较哈希值，仅当哈希匹配时才进行全树比较。此优化确保 diff 满足第 2.3 节的性能约束（1000 节点 < 1ms）。

### 3.2 Key 类型

```rust
/// 列表 diff 的稳定标识。
///
/// 使用 Arc<str> 而非 String，使 Clone 开销为 O(1)（引用计数增加）。
/// Key 在 diff 过程中频繁比较（Eq + Hash），Arc<str> 的比较性能与 &str 一致。
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Key(pub Arc<str>);

impl Key {
    /// 从字符串切片构造 Key。
    pub fn new(s: impl Into<String>) -> Self {
        Key(Arc::from(s.into().into_boxed_str()))
    }

    /// 返回 key 的字符串表示。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

### 3.3 WidgetId 类型

```rust
use std::num::NonZeroU64;

/// 全局唯一的 widget 标识符。
///
/// WidgetId 在组件创建时由框架分配，应用运行期间全局唯一。
/// 使用 NonZeroU64 以支持 `Option<WidgetId>` 的零空间优化（与指针等大）。
/// 范围 1..=u64::MAX，0 预留为无效标识。
///
/// # 生命周期
///
/// - 创建时分配（见第 5.2 节阶段 2）
/// - 卸载时回收（见第 5.2 节阶段 5），回收后同一 ID 值不会再分配
/// - WidgetId 跨状态存储、事件路由、订阅追踪和调试日志使用
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct WidgetId(NonZeroU64);

impl WidgetId {
    /// 由框架内部分配器返回下一个可用的 WidgetId（仅框架内部使用）。
    /// 分配策略和实现细节见 D2 状态管理。
    pub(crate) fn next() -> Self {
        // 实现细节见 D2 状态管理
        unimplemented!()
    }

    /// 返回 WidgetId 的数值表示（用于日志和调试）。
    pub fn as_u64(self) -> u64 {
        self.0.get()
    }
}
```

### 3.4 PropValue 类型

```rust
use ordered_float::OrderedFloat;

/// 属性值：WidgetView props 的值的类型安全的枚举。
///
/// # 设计决策
///
/// - 使用枚举而非 `Box<dyn Any>`：保持类型安全，支持 PartialEq 派生
/// - f64 使用 `OrderedFloat` 包裹：f64 本身不实现 Eq/Ord，OrderedFloat 提供了
///   确定性的比较语义（NaN == NaN, -0 == +0）
#[derive(Clone, PartialEq, Debug)]
pub enum PropValue {
    /// 字符串值
    Str(Arc<str>),

    /// 布尔值
    Bool(bool),

    /// 整数
    Int(i64),

    /// 浮点数（使用 OrderedFloat 支持 Eq/Ord）
    Float(OrderedFloat<f64>),

    /// RGBA 颜色
    Color(Color),

    /// 尺寸
    Size(Size),

    /// 矩形
    Rect(Rect),

    /// 列表值（如 `grid-template-columns: [100px, 1fr, 2fr]`）
    List(Vec<PropValue>),

    /// 嵌套映射（如自定义数据属性 `data-custom: { key1: "val1", key2: 42 }`）
    Map(BTreeMap<Arc<str>, PropValue>),

    /// 枚举值（如 `variant: Primary`、`align: Center`）
    Enum(Arc<str>),

    /// 回调值：消息构造器（保留用于 html! 宏中的事件绑定）。
    /// Callback 类型详细定义见 D5 事件系统。
    Callback(Callback),
}

/// 颜色值：sRGBA，每个通道 0-255。
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }
}
```

### 3.5 MessageBinding 类型

```rust
/// 消息绑定：描述子组件消息如何映射到父组件的处理。
///
/// 框架在 diff 阶段检查 message_bindings，当子组件产生匹配消息时，
/// 调用绑定的 handler 将其转换为父组件可处理的形式。
///
/// # 相等性语义
///
/// 两个 MessageBinding 相等当且仅当 `source` 和 `message_name` 相等。
/// `handler` 字段不参与相等性比较（闭包无法做值比较）。
/// 框架 diff 引擎通过此相等性判断消息绑定是否变更。
#[derive(Clone, Debug)]
pub struct MessageBinding<M: AppMessage> {
    /// 消息来源 widget 的 ID（或 key，用于列表场景）
    pub source: WidgetId,

    /// 此绑定匹配的消息名称（用于过滤）
    /// 若为 None，则匹配所有来自 source 的消息
    pub message_name: Option<&'static str>,

    /// 消息处理器（将子组件的消息转换为父组件的消息）
    pub handler: MessageHandler<M>,
}

impl<M: AppMessage> PartialEq for MessageBinding<M> {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source && self.message_name == other.message_name
    }
}

/// 消息处理器枚举。
///
/// 详细定义见 D5 事件系统。本文档仅给出与 WidgetView 交互的边界类型。
#[derive(Clone, Debug)]
pub enum MessageHandler<M: AppMessage> {
    /// 直接转发：子消息 → 父消息（一对一映射）
    Forward(Box<dyn Fn(Box<dyn AppMessage>) -> M + Send + Sync>),

    /// 批量处理：子消息 → 父消息列表（一对多映射）
    Batch(Box<dyn Fn(Box<dyn AppMessage>) -> Vec<M> + Send + Sync>),

    /// 带过滤的转发：先过滤，匹配的才转换
    Filtered {
        predicate: Box<dyn Fn(&Box<dyn AppMessage>) -> bool + Send + Sync>,
        map: Box<dyn Fn(Box<dyn AppMessage>) -> M + Send + Sync>,
    },
}

impl<M: AppMessage> PartialEq for MessageHandler<M> {
    fn eq(&self, _other: &Self) -> bool {
        // MessageHandler 包含闭包，无法在值层面比较相等性。
        // MessageBinding 的 PartialEq 仅比较 source + message_name，
        // handler 字段不参与比较。
        false
    }
}
```

> **设计说明：** `MessageHandler` 包含闭包（`Box<dyn Fn(...)>`），Rust 不支持对闭包做值相等性比较。因此 `MessageBinding` 手动实现了 `PartialEq`，仅比较 `source` 和 `message_name` 字段，忽略 `handler`。框架 diff 引擎通过 `WidgetView` 的 `PartialEq`（进而通过 `MessageBinding` 的 `PartialEq`）判断消息绑定是否变更。当应用代码替换 handler 闭包但保持 source + message_name 不变时，框架不会检测到绑定变更——此时应用代码应同时变更 `message_name` 以触发 diff（D5 将提供辅助工具）。

### 3.6 WidgetView 的构造辅助方法

```rust
impl<M: AppMessage> WidgetView<M> {
    /// 创建一个新的 WidgetView（叶子节点，无子节点）。
    pub fn new(widget_type: &'static str) -> Self {
        WidgetView {
            widget_type,
            id: None,
            key: None,
            props: BTreeMap::new(),
            children: Vec::new(),
            message_bindings: Vec::new(),
        }
    }

    /// 设置 id。
    pub fn id(mut self, id: WidgetId) -> Self {
        self.id = Some(id);
        self
    }

    /// 设置 key。
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(Key::new(key));
        self
    }

    /// 添加单个属性。
    pub fn prop(mut self, name: &'static str, value: PropValue) -> Self {
        self.props.insert(name, value);
        self
    }

    /// 批量添加属性。
    pub fn props(mut self, props: impl IntoIterator<Item = (&'static str, PropValue)>) -> Self {
        self.props.extend(props);
        self
    }

    /// 添加子视图。
    pub fn child(mut self, child: WidgetView<M>) -> Self {
        self.children.push(child);
        self
    }

    /// 添加多个子视图。
    pub fn children(mut self, children: impl IntoIterator<Item = WidgetView<M>>) -> Self {
        self.children.extend(children);
        self
    }

    /// 添加消息绑定。
    pub fn on_message(mut self, binding: MessageBinding<M>) -> Self {
        self.message_bindings.push(binding);
        self
    }
}
```

---

## 4. 组件注册表（WidgetRegistry）

### 4.1 设计

```rust
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// 全局组件注册表。
///
/// 框架在启动时构建，应用代码通过 `App::register::<T>()` 添加组件。
/// 运行时通过 widget_type 名称查找对应的 WidgetSpec 实现。
pub struct WidgetRegistry {
    /// 名称 → 注册项
    entries: HashMap<&'static str, RegistryEntry>,

    /// TypeId → 名称（用于类型查找）
    type_to_name: HashMap<TypeId, &'static str>,
}

struct RegistryEntry {
    /// 创建组件实例的工厂函数
    factory: Box<dyn Fn() -> Box<dyn Any> + Send + Sync>,

    /// 组件的 TypeId
    type_id: TypeId,

    /// 是否由框架内置
    builtin: bool,
}

impl WidgetRegistry {
    /// 创建空的注册表。
    pub fn new() -> Self {
        WidgetRegistry {
            entries: HashMap::new(),
            type_to_name: HashMap::new(),
        }
    }

    /// 注册一个组件类型。
    ///
    /// # 泛型约束
    ///
    /// - `T: WidgetSpec + Default`：需要 Default 以支持工厂创建
    /// - 如果同名组件已注册，返回错误
    pub fn register<T: WidgetSpec + Default>(&mut self) -> Result<(), RegistryError> {
        let instance = T::default();
        let name = instance.name();

        if self.entries.contains_key(name) {
            return Err(RegistryError::DuplicateName(name));
        }

        self.entries.insert(name, RegistryEntry {
            factory: Box::new(|| Box::new(T::default())),
            type_id: TypeId::of::<T>(),
            builtin: false,
        });
        self.type_to_name.insert(TypeId::of::<T>(), name);

        Ok(())
    }

    /// 注册框架内置组件（仅框架内部使用）。
    pub(crate) fn register_builtin<T: WidgetSpec + Default>(
        &mut self,
    ) -> Result<(), RegistryError> {
        let instance = T::default();
        let name = instance.name();

        self.entries.insert(name, RegistryEntry {
            factory: Box::new(|| Box::new(T::default())),
            type_id: TypeId::of::<T>(),
            builtin: true,
        });
        self.type_to_name.insert(TypeId::of::<T>(), name);

        Ok(())
    }

    /// 按名称查找组件工厂。
    pub fn get(&self, name: &str) -> Option<&RegistryEntry> {
        self.entries.get(name)
    }

    /// 检查某个类型是否已注册。
    pub fn contains_type<T: 'static>(&self) -> bool {
        self.type_to_name.contains_key(&TypeId::of::<T>())
    }

    /// 返回已注册的组件名称列表。
    pub fn registered_names(&self) -> Vec<&'static str> {
        self.entries.keys().copied().collect()
    }
}

/// 注册错误。
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("组件名称 '{0}' 已被注册")]
    DuplicateName(&'static str),

    #[error("组件类型已注册")]
    DuplicateType,
}
```

### 4.2 注册时机

```rust
// 应用启动时的典型注册流程：
fn main() {
    App::new()
        .register::<rgui_components::Button>()
        .register::<rgui_components::TextField>()
        .register::<rgui_components::DataGrid>()
        .register::<MyCustomWidget>()   // 第三方组件
        .run(my_app);
}
```

---

## 5. 组件生命周期

### 5.1 生命周期状态机

```
                    ┌──────────┐
                    │ 未注册     │
                    └─────┬────┘
                          │ App::register::<T>()
                          ▼
                    ┌──────────┐
                    │ 已注册     │◄─── 工厂仍在注册表中，可重新创建 ───┐
                    └─────┬────┘                                    │
                          │ view() 返回包含此 widget_type 的 WidgetView
                          ▼                                         │
                    ┌──────────┐                                    │
                    │ 已创建     │  ← StateStore 中分配 WidgetId + 初始状态
                    └─────┬────┘                                    │
                          │ 首次布局 + paint                         │
                          ▼                                         │
                    ┌──────────┐                                    │
                    │ 已挂载     │◄──── 正常运行状态 ────┐            │
                    └─────┬────┘                       │            │
                          │                            │            │
                    ┌─────┼──────────┐                 │            │
                    │     │          │                 │            │
                    ▼     ▼          ▼                 │            │
              ┌────────┐ ┌────────┐ ┌────────┐        │            │
              │ update │ │ remeasure│ │ repaint│        │            │
              │ (事件)  │ │ (尺寸变)│ │ (外观变)│        │            │
              └───┬────┘ └───┬────┘ └───┬────┘        │            │
                  │          │          │              │            │
                  └──────────┼──────────┘              │            │
                             │ dirty 标记               │            │
                             ▼                          │            │
                       ┌──────────┐                    │            │
                       │ 重新 view │────────────────────┘            │
                       └──────────┘                                 │
                                                                     │
             父组件 view() 不再包含此 widget                          │
                          │                                         │
                          ▼                                         │
                    ┌──────────┐                                    │
                    │ 已卸载     │  ← StateStore 释放状态，WidgetId 回收
                    └─────┬────┘                                    │
                          │                                         │
                          │ 同一 widget_type 可在后续 view() 中重新创建
                          │ （新实例，新 WidgetId）                     │
                          └─────────────────────────────────────────┘
```

### 5.2 生命周期各阶段详解

#### 阶段 1：注册（Registration）

- **时机**：应用启动时，`main()` 函数中
- **操作**：`App::register::<T>()` 将组件工厂存入 WidgetRegistry
- **约束**：同一名称不能重复注册
- **失败处理**：重复注册返回错误，应用可选择 panic 或警告后继续

#### 阶段 2：创建（Creation）

- **时机**：父组件的 `view()` 返回包含此 widget_type 的 WidgetView，且 diff 判定为新节点
- **操作**：
  1. 框架从 WidgetRegistry 获取工厂
  2. 调用工厂创建 `Box<dyn WidgetSpec>` 实例
  3. 在 StateStore 中分配 WidgetId
  4. 初始化持久状态（调用 `Default::default()`）
  5. 分配实例态（InstanceState）和缓存态（RenderLayoutCache）
- **约束**：WidgetId 全局唯一，不可复用

#### 阶段 3：挂载（Mount）

- **时机**：首次布局和绘制完成后
- **操作**：
  1. 执行 `measure()` 确定尺寸
  2. 执行布局计算（Taffy）
  3. 执行 `accessibility()` 生成无障碍节点
  4. 执行 `paint()` 生成初始绘制指令
  5. 推入 AccessibilityTree
- **约束**：挂载完成后组件可见且可交互

#### 阶段 4：更新（Update）

- **时机**：事件驱动（用户输入、定时器、网络响应等）
- **操作**：
  1. 框架调用 `update(msg, state, ctx)`
  2. 组件修改 `state` 中的字段
  3. 框架自动标记此 widget 为 dirty
  4. 框架传播 dirty 到订阅者
  5. 下一帧：重执行 `view()` → diff → 可能触发 re-measure/re-paint
- **约束**：`update()` 不得阻塞（禁止同步 I/O）

#### 阶段 5：卸载（Unmount）

- **时机**：父组件的新 WidgetView 不再包含此 widget
- **操作**：
  1. 框架从 AccessibilityTree 移除节点
  2. 释放实例态和缓存态
  3. 从 StateStore 移除持久状态
  4. 回收 WidgetId（后续可重新分配新实例使用）
  5. 移除订阅关系
- **约束**：卸载后组件的 `view()` 不再被调用

### 5.3 生命周期回调（预留）

```rust
/// 生命周期回调（预留，阶段 2 实现）。
///
/// 以下方法不在 WidgetSpec trait 中，而是通过单独的 `WidgetLifecycle` trait 提供。
/// 阶段 1 中组件无需实现这些方法。
pub trait WidgetLifecycle: WidgetSpec {
    /// 组件首次挂载到 widget 树时调用。
    /// 可用于订阅外部事件源、启动定时器等。
    fn on_mount(&self, ctx: &mut UpdateContext) {}

    /// 组件从 widget 树卸载前调用。
    /// 可用于取消订阅、释放外部资源等。
    fn on_unmount(&self, ctx: &mut UpdateContext) {}

    /// 组件在 widget 树中的位置发生变化时调用。
    fn on_reparent(&self, old_parent: WidgetId, new_parent: WidgetId) {}
}
```

> **阶段 1 决策：** `WidgetLifecycle` 在阶段 1 不作为 `WidgetSpec` 的一部分。阶段 1 中需要外部资源管理的组件可在 `update()` 中手动处理。阶段 2 将评估是否需要独立的生命周期回调，根据需要添加。

---

## 6. Context 类型详细定义

### 6.1 ViewContext

```rust
/// view() 的上下文：提供只读的环境信息。
///
/// # 设计说明
///
/// ViewContext 不提供状态存储访问（view() 只能访问 &Self::State）。
/// 如果组件需要在 view() 中读取其他 widget 的状态来控制视图结构，
/// 应改为在 update() 中读取并将结果缓存到自身状态中。
pub struct ViewContext {
    /// 当前主题引用。
    ///
    /// Theme 由 rgui-style 定义（见 D4）。包含颜色方案、间距、字体等变量。
    /// 使用 `&'static` 生命周期是因为主题在应用运行期间不变。
    pub theme: &'static Theme,

    /// 区域设置。
    ///
    /// 包含语言、数字格式、日期格式等本地化信息。
    pub locale: &'static Locale,

    /// 窗口逻辑尺寸（CSS 像素，非物理像素）。
    /// 用于响应式布局决策。
    pub window_size: Size,

    /// 当前 DPI 缩放比例。
    /// 1.0 = 标准 DPI，2.0 = Retina/HiDPI。
    pub scale_factor: f64,
}
```

### 6.2 UpdateContext

```rust
/// update() 的上下文：提供状态读写和事件发送能力。
///
/// # 设计说明
///
/// UpdateContext 持有 StoreAccessMut，但 StoreAccessMut 的设计保证
/// 调用者只能修改**自身的**持久状态（见 D2 详细设计）。
pub struct UpdateContext<'a> {
    /// 可变状态存储访问。
    ///
    /// - `store.state_mut::<Self::State>()` → 修改自身状态
    /// - `store.read::<T>(target_id)` → 读取其他 widget 状态（自动建立订阅）
    pub store: StoreAccessMut<'a>,

    /// 向父组件发送事件。
    pub event_sender: EventSender,

    /// 当前焦点 widget ID。
    /// None 表示没有 widget 持有焦点。
    pub focus: Option<WidgetId>,

    /// 当前悬停 widget ID。
    pub hover: Option<WidgetId>,
}

/// 事件发送器（详细设计见 D5）。
pub struct EventSender {
    /// 发送目标（由框架在事件分派时设置）
    target: WidgetId,
    /// 待发送的事件队列（框架内部）
    queue: Vec<Event>,
}

impl EventSender {
    /// 向父组件发送事件。
    pub fn send_to_parent<M: AppMessage>(&mut self, msg: M) {
        // 实现见 D5
    }

    /// 向指定 widget 发送事件。
    pub fn send_to<M: AppMessage>(&mut self, target: WidgetId, msg: M) {
        // 实现见 D5
    }
}
```

### 6.3 MeasureContext

```rust
/// measure() 的上下文：提供字体度量和 DPI 信息。
pub struct MeasureContext {
    /// 字体度量缓存。
    ///
    /// 提供常用字体的 ascent、descent、line_gap、x_height 等度量信息。
    /// 由 rgui-render 的 cosmic-text 集成提供（见 D3）。
    pub font_metrics: &'static FontMetricsCache,

    /// 当前 DPI 缩放比例。
    pub scale_factor: f64,
}

/// 字体度量缓存（骨架，详细定义见 D3）。
pub struct FontMetricsCache {
    /// 默认字体族的度量信息
    pub default_metrics: FontMetrics,
}

/// 单种字体的度量。
#[derive(Copy, Clone, Debug)]
pub struct FontMetrics {
    /// 上升部高度（em 单位）
    pub ascent: f64,
    /// 下降部高度（em 单位，通常为负值）
    pub descent: f64,
    /// 行间距（em 单位）
    pub line_gap: f64,
    /// x-height（em 单位）
    pub x_height: f64,
    /// 大写字母高度（em 单位）
    pub cap_height: f64,
}
```

### 6.4 PaintContext

```rust
/// paint() 的上下文：提供渲染缓存句柄。
pub struct PaintContext<'a> {
    /// 字形 Atlas 句柄。
    ///
    /// 字形 Atlas 是单个 GPU 纹理，缓存已光栅化的字形。
    /// 组件通过 glyph_atlas 获取文本的字形数据和纹理坐标。
    pub glyph_atlas: &'a mut GlyphAtlas,

    /// 当前裁剪区域。
    ///
    /// 超出此区域的绘制指令可被优化跳过。
    pub clip_rect: Rect,

    /// 渲染缓存。
    ///
    /// 组件可将昂贵的绘制结果缓存在此（如路径细分、渐变纹理）。
    /// 缓存由框架持有，随组件卸载而释放。
    pub render_cache: &'a mut RenderLayoutCache,
}
```

### 6.5 AccessContext

```rust
/// accessibility() 的上下文：提供可见区域和焦点路径信息。
pub struct AccessContext {
    /// 当前可见区域（滚动容器内的视口）。
    ///
    /// 组件可据此只生成可见区域内的无障碍节点，优化大数据集场景。
    pub visible_bounds: Rect,

    /// 从根节点到当前 widget 的焦点路径。
    ///
    /// 用于确定当前 widget 是否在焦点链中。
    pub focus_path: Vec<WidgetId>,
}
```

---

## 7. ui! 宏设计

### 7.1 设计目标

`ui!` 宏是一个声明式 DSL，允许开发者在 Rust 代码中以类似 JSX 的语法描述 UI 结构。宏展开为目标为 `WidgetView<M>` 的纯 Rust 代码。

设计原则：
- **零运行时开销**：所有语法在编译期展开为 `WidgetView` 构造代码
- **类型安全**：属性名和值在编译期检查
- **与 Rust 语法互操作**：可以在属性值中嵌入 Rust 表达式
- **IDE 友好**：展开后的代码应能被 rust-analyzer 理解

### 7.2 语法规范

```text
ui! {
    <WidgetType
        prop_name={expression}
        flag_prop                    // 布尔属性，存在即为 true
        on_event={Message:Variant}   // 消息绑定
    >
        "文本内容"                     // 文本子节点
        <ChildWidget ... />           // 自闭合子组件
        {conditional ? <A/> : <B/>}   // 条件渲染
        {items.map(|item| <Item key={item.id} data={item} />)}  // 列表渲染
    </WidgetType>
}
```

### 7.3 编译期展开示例

```rust
// 输入：ui! 宏
fn app(state: &AppState) -> WidgetView<Message> {
    ui! {
        <VBox spacing=12 padding=16 class="page">
            <HBox spacing=8>
                <TextField
                    placeholder="搜索..."
                    value=state.search_text.as_str()
                    on_change=Message::SearchChanged
                />
            </HBox>
            <DataGrid
                columns=&state.columns
                rows=&state.rows
                virtual_scroll=true
                editable=true
                on_sort=Message::SortBy
            />
            <HBox spacing=8 justify=End>
                <Button variant=Primary on_click=Message::Save>
                    "保存"
                </Button>
                <Button variant=Secondary on_click=Message::Cancel>
                    "取消"
                </Button>
            </HBox>
        </VBox>
    }
}

// 展开后（示意）：
fn app(state: &AppState) -> WidgetView<Message> {
    WidgetView::new("rgui_components::VBox")
        .prop("spacing", PropValue::Int(12))
        .prop("padding", PropValue::Int(16))
        .prop("class", PropValue::Str(Arc::from("page")))
        .child(
            WidgetView::new("rgui_components::HBox")
                .prop("spacing", PropValue::Int(8))
                .child(
                    WidgetView::new("rgui_components::TextField")
                        .prop("placeholder", PropValue::Str(Arc::from("搜索...")))
                        .prop("value", PropValue::Str(Arc::from(state.search_text.as_str())))
                        .on_message(MessageBinding {
                            source: /* 由框架在 WidgetView 挂载时根据父组件的 WidgetId 分配 */,
                            message_name: Some("change"),
                            handler: MessageHandler::Forward(Box::new(|_msg| {
                                Message::SearchChanged
                            })),
                        })
                )
        )
        .child(
            WidgetView::new("rgui_components::DataGrid")
                .on_message(MessageBinding {
                    source: /* 由框架在挂载时分配 */,
                    message_name: Some("sort"),
                    handler: MessageHandler::Forward(Box::new(|_msg| Message::SortBy)),
                })
        )
        .child(
            WidgetView::new("rgui_components::HBox")
                .prop("spacing", PropValue::Int(8))
                .prop("justify", PropValue::Enum(Arc::from("End")))
                .child(
                    WidgetView::new("rgui_components::Button")
                        .prop("variant", PropValue::Enum(Arc::from("Primary")))
                        .on_message(MessageBinding {
                            source: /* 由框架在挂载时分配 */,
                            message_name: Some("click"),
                            handler: MessageHandler::Forward(Box::new(|_msg| Message::Save)),
                        })
                        .child(WidgetView::new("rgui_core::Text")
                            .prop("content", PropValue::Str(Arc::from("保存"))))
                )
                .child(
                    WidgetView::new("rgui_components::Button")
                        .prop("variant", PropValue::Enum(Arc::from("Secondary")))
                        .on_message(MessageBinding {
                            source: /* 由框架在挂载时分配 */,
                            message_name: Some("click"),
                            handler: MessageHandler::Forward(Box::new(|_msg| Message::Cancel)),
                        })
                        .child(WidgetView::new("rgui_core::Text")
                            .prop("content", PropValue::Str(Arc::from("取消"))))
                )
        )
}
```

### 7.4 宏的语法规则

#### 属性语法

| 语法 | 展开 | 示例 |
|------|------|------|
| `name=value` | `.prop("name", value.into())` | `spacing=12` |
| `name={expr}` | `.prop("name", PropValue::from(expr))` | `value={state.text.as_str()}` |
| `name=true` | `.prop("name", PropValue::Bool(true))` | `editable=true` |
| `flag` | `.prop("flag", PropValue::Bool(true))` | `disabled` |
| `on_event=Msg::Variant` | `.on_message(binding)` | `on_click=Message::Save` |

#### 子节点语法

| 语法 | 含义 |
|------|------|
| `<Widget .../>` | 自闭合组件 |
| `<Widget ...>children</Widget>` | 带子节点的组件 |
| `"text"` | 文本节点（展开为 Text 组件） |
| `{expr}` | 嵌入 Rust 表达式，结果必须是 WidgetView 或可转为 WidgetView |

### 7.5 宏的错误提示

```text
// 错误示例 1：未注册的组件名
error: 未找到组件 "VBoxx"——你是否想输入 "VBox"？
  ┌─ src/app.rs:5:10
  │
5 │         <VBoxx spacing=12>
  │          ^^^^^ 此组件未在 WidgetRegistry 中注册

// 错误示例 2：属性类型不匹配
error: 属性 "spacing" 期望类型 `i64`，但收到了 `&str`
  ┌─ src/app.rs:5:20
  │
5 │         <VBox spacing="12">
  │                    ^^^^ 类型不匹配
```

### 7.6 宏的实现策略

`ui!` 宏分两个阶段实现：

**阶段 1（MVP）：** 使用 `macro_rules!` 实现基本版本。支持：
- 静态属性（字面量值）
- 嵌套子组件
- 文本子节点
- 消息绑定（`on_*` 属性）

**阶段 2（完善）：** 使用 proc-macro 实现完整版本。增加：
- Rust 表达式嵌入（`{expr}`）
- 条件渲染和列表渲染
- 编译期属性类型检查
- 更好的错误提示
- IDE 自动补全支持

```rust
// 阶段 1：macro_rules! 版本（简化示意）
#[macro_export]
macro_rules! ui {
    // 自闭合标签：<Widget prop=val />
    (< $widget:ident $( $prop:ident = $val:expr ),* $(,)? />) => {
        {
            let mut __view = $crate::WidgetView::new(stringify!($widget));
            $(
                __view = __view.prop(stringify!($prop), $crate::PropValue::from($val));
            )*
            __view
        }
    };
    // 带子节点的标签（递归展开，完整实现见 rgui-macros crate）
    (< $widget:ident $( $prop:ident = $val:expr ),* $(,)? >
        $($child:tt)*
    </ $widget:ident>) => {
        {
            let mut __view = $crate::WidgetView::new(stringify!($widget));
            $(
                __view = __view.prop(stringify!($prop), $crate::PropValue::from($val));
            )*
            // 子节点递归展开
            __view
        }
    };
}
```

> **实现位置**：`ui!` 宏定义在 `rgui-macros` crate 中，通过 `rgui` facade 重新导出。

---

## 8. 派生宏设计

### 8.1 #[derive(WidgetSpec)]

```rust
/// 为简单组件自动生成 WidgetSpec 的样板实现。
///
/// 自动生成的默认行为：
/// - `name()`: 返回 `"{module}::{TypeName}"` 格式的字符串
/// - `update()`: 空实现（不做任何事）
/// - `measure()`: 委托给 `self.default_measure(state, constraints, ctx)`
/// - `accessibility()`: 返回 `AccessibilityNode::none()`
///
/// 组件必须手动实现 `view()` 和 `paint()`。
/// 手动方法写在固有 `impl Counter { ... }` 块中（非 `impl WidgetSpec for Counter`），
/// derive 宏生成的 trait 实现会通过方法名约定自动委托到这些固有方法。
///
/// # 使用示例
///
/// ```ignore
/// #[derive(WidgetSpec)]
/// #[widget(name = "my_app::Counter")]
/// struct Counter;
///
/// // 注意：不要写 `impl WidgetSpec for Counter`（derive 已生成 trait 实现）。
/// // 改为在固有 impl 块中定义 view() 和 paint()——
/// // derive 宏生成的 trait 方法会通过方法名约定委托到这些固有方法。
/// impl Counter {
///     fn view(&self, state: &Self::State, ctx: &ViewContext) -> WidgetView<Self::Message> {
///         // 手动实现
///     }
///     fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
///         // 手动实现
///     }
/// }
/// ```
#[proc_macro_derive(WidgetSpec, attributes(widget))]
pub fn derive_widget_spec(input: TokenStream) -> TokenStream {
    // 实现见 rgui-macros crate
}
```

### 8.2 #[derive(AppMessage)]

```rust
/// 为消息枚举自动生成 AppMessage trait 实现。
///
/// # 使用示例
///
/// ```ignore
/// #[derive(AppMessage)]
/// enum CounterMessage {
///     Increment,
///     Decrement,
///     Reset,
/// }
/// ```
#[proc_macro_derive(AppMessage)]
pub fn derive_app_message(input: TokenStream) -> TokenStream {
    // 实现见 rgui-macros crate
}
```

### 8.3 #[derive(PersistState)]

```rust
/// 为状态结构体自动生成 PersistState trait 实现。
///
/// # 使用示例
///
/// ```ignore
/// #[derive(PersistState)]
/// #[state(schema = "my_app::CounterState", version = 1)]
/// struct CounterState {
///     count: i64,
/// }
/// ```
///
/// `schema` 属性声明持久状态的类型标识（对应 `PersistState::schema_name()` 的返回值），
/// 供序列化/反序列化系统路由使用。`version` 用于数据迁移时判断 schema 版本号
/// （对应 `PersistState::schema_version()`）。
#[proc_macro_derive(PersistState, attributes(state))]
pub fn derive_persist_state(input: TokenStream) -> TokenStream {
    // 实现见 rgui-macros crate
}
```

---

## 9. 第三方组件协议

### 9.1 第三方组件的发布契约

第三方组件以 crate 形式发布。一个符合规范的第三方组件 crate 必须包含：

1. **WidgetSpec 实现**：实现 `WidgetSpec` trait 的结构体
2. **PersistState 实现**：组件持久状态类型，实现 `PersistState`
3. **AppMessage 实现**：组件消息枚举，实现 `AppMessage`
4. **样式元数据**：组件接受的样式属性声明（见 D4）

```rust
// 第三方组件 crate 示例：rgui-chart
//
// Cargo.toml:
// [dependencies]
// rgui-core = "0.1"

use rgui_core::{WidgetSpec, WidgetView, PersistState, AppMessage, PropValue};
use rgui_core::{ViewContext, UpdateContext, MeasureContext, PaintContext, AccessContext};
use rgui_core::{Rect, Size, BoxConstraints, Color, AccessibilityNode, AccessibilityRole};

#[derive(Debug, Clone, Default, PersistState)]
#[state(schema = "rgui_chart::ChartState", version = 1)]
pub struct ChartState {
    pub data: Vec<DataPoint>,
    pub x_axis_label: String,
    pub y_axis_label: String,
}

#[derive(Debug, Clone, AppMessage)]
pub enum ChartMessage {
    PointHovered(usize),
    PointSelected(usize),
}

pub struct Chart;

impl WidgetSpec for Chart {
    type State = ChartState;
    type Message = ChartMessage;

    fn name(&self) -> &'static str {
        "rgui_chart::Chart"
    }

    /// Chart 是叶子组件（无子组件）。view() 返回自身的 WidgetView 表示
    /// "视图内容为 Chart 类型"。框架 diff 阶段通过节点身份（父位置 + id/key）
    /// 判断此为同一节点的更新，不会递归创建新实例。
    fn view(&self, state: &Self::State, ctx: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_chart::Chart")
            .prop("data_points", PropValue::Int(state.data.len() as i64))
    }

    fn update(
        &self,
        msg: Self::Message,
        state: &mut Self::State,
        ctx: &mut UpdateContext,
    ) {
        match msg {
            ChartMessage::PointHovered(idx) => {
                // 处理悬浮
            }
            ChartMessage::PointSelected(idx) => {
                // 处理选中
            }
        }
    }

    fn measure(
        &self,
        state: &Self::State,
        constraints: BoxConstraints,
        ctx: &MeasureContext,
    ) -> Size {
        Size::new(400.0, 300.0)
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        // 使用 ctx 中的渲染缓存绘制图表
    }

    fn accessibility(
        &self,
        state: &Self::State,
        ctx: &AccessContext,
    ) -> AccessibilityNode {
        AccessibilityNode::new()
            .role(AccessibilityRole::Image)
            .name("图表")
            .description(format!("包含 {} 个数据点", state.data.len()))
    }
}
```

### 9.2 第三方组件的兼容性保证

框架向第三方组件保证：

1. **trait 方法只增不减**：1.0 之前，`WidgetSpec` 不会删除方法。新增方法提供默认实现
2. **Context 类型字段只增不减**：已有字段不会移除或改变类型
3. **WidgetView 字段只增不减**：已有字段的含义不变
4. **废弃通知**：任何废弃的 API 至少在 2 个 minor 版本中保持可用，并提供替代方案

第三方组件的分发方式：
- crates.io 上发布
- 通过 `cargo add rgui-{component_name}` 安装
- 应用在 `main()` 中调用 `App::register::<ThirdPartyComponent>()`

### 9.3 组件能力检测

```rust
/// 组件能力标记（预留，阶段 2 实现）。
///
/// 第三方组件可声明支持的可选能力，框架在运行时查询。
pub trait WidgetCapabilities {
    /// 返回组件支持的能力集合。
    fn capabilities(&self) -> Vec<Capability> {
        vec![]
    }
}

/// 组件能力枚举。
pub enum Capability {
    /// 支持虚拟滚动
    VirtualScroll,
    /// 支持拖放
    DragAndDrop,
    /// 支持内联编辑
    InlineEditing,
    /// 支持自定义绘制
    CustomPainting,
}
```

### 9.4 脚本定义组件（Tier 2）

> **设计来源：** Qt/QML——QML 文件可作为独立组件使用，无需 C++ 注册。rgui 同等支持 `.rgui` + `.rhai` paint 脚本定义组件。

#### 9.4.1 架构：加载时生成 PaintOp，每帧纯 Rust 渲染

```
.rgui 标签 + .rhai paint 脚本
       │
       ▼  加载时（一次性）
  Rhai 引擎执行 paint 脚本
  → 调用 fill_rect / draw_text 等原生函数
       │
       ▼
  Vec<PaintOp> 存入 WidgetView.props["paint_ops"]
       │
       ▼  每帧渲染时（60fps）
  walk_view_tree → 从 props 取 PaintOp
       │
       ▼
  Vello → GPU  ← 纯 Rust，零脚本开销
```

> 渲染效率与 Tier 1（Rust `WidgetSpec::paint()`）完全一致——因为每帧热路径消费的都是 `Vec<PaintOp>` 数据结构，不区分来源。（架构原理见 [D0 §9.1.1](./D0-Rust%20GUI%20框架总体设计.md#911-关键机制脚本-paint-只执行一次)）

#### 9.4.1.1 布局交互

Tier 2 组件的 `.rgui` 树与 Tier 1 组件**同等参与 Taffy 布局**，paint 脚本不负责布局计算：

```
.rgui 标签 + 属性（flex-direction、gap、padding 等）
       │
       ▼
extract_taffy_style → Taffy 布局引擎 → 计算 bounds
       │
       ▼
walk_view_tree：
  - 对 Tier 1 节点：调用 WidgetSpec::paint()
  - 对 Tier 2 节点：检查 props["paint_ops"] 是否有预计算 PaintOp
    → 有则直接消费（加载时已由 execute_tier2_paint_scripts 生成）
    → 无则回退到 paint_fn（Tier 1 路径）
  - **当前为降级模式**：bounds 注入（AC02）+ 动态重执行（AC07）完成后，
    walk_view_tree 可传递 bounds/props/children 给 Rhai 脚本实时生成 PaintOp
       │
       ▼
Vello → GPU
```

> Tier 2 组件的 `Container/Row/Column/Text` 等子节点在 `.rgui` 中声明，Taffy 自动计算其布局。**AC02 完成后**，paint 脚本将接收 `bounds`/`props`/`children` 参数——`bounds` 为 Taffy 计算后的绝对坐标，`children` 是子节点列表（递归渲染由 `paint_children` 完成）。

#### 9.4.1.2 节点识别

`walk_view_tree` 通过以下机制区分 Tier 1 和 Tier 2 节点：

```
.rgui 中遇到标签 <my_card>
       │
       ▼  parse_rgui_file 解析时
  查找同目录下 my_card.rgui + my_card.rhai 文件对
       │
       |       ├── 找到 → 标记为 Tier 2 节点
       |       │    → execute_tier2_paint_scripts 在加载时执行 Rhai 脚本
       |       │    → 生成的 PaintOp 存入 view.props["paint_ops"]
       |       │    → 节点 props 中自动注入文件路径引用（_tier/_rhai_path/_rgui_path）
       │
       └── 未找到 → 按 Tier 1 处理
            → widget_type 匹配 WidgetRegistry 中已注册的 Rust 组件
            → 未匹配则走"未注册组件降级"（红底占位符）
```

> **设计理由**：Tier 2 的入口是文件系统的 `.rgui`/`.rhai` 文件对——框架在文件解析阶段自动识别，不需要应用代码显式注册。这与 QML 引擎自动加载 `.qml` 文件的机制一致。

找到文件对后，`parse_rgui_file` 执行子树内联（T207）：

```
parse_rgui_file("ui.rgui")
       │
       ▼
  遍历 WidgetView 树，遇到 Tier 2 标签 <my_card>
       │
       ├── ① 加载 my_card.rgui → 子 WidgetView 树
       ├── ② 将父传入的子节点替换到子树的 <slot /> 位置（T209）
       ├── ③ 将父传入的 props 注入子组件作用域（T208）
       ├── ④ 将外层 props（onclick/variant 等）附加到子树根节点（T210）
       │
       ▼
  子树内联到父树，替换原 <my_card> 节点
```

> 内联后的 WidgetView 树与直接手写等效——布局引擎看到的是一棵完整的树，不区分 Tier 1/Tier 2 来源。

#### 9.4.2 示例：用脚本定义带圆角的卡片组件

> **⚠️ 示例中 `{props.title}` 属性绑定语法和 `props.get().unwrap_or()` 方法链为设计目标语法，P0 交付时可能以等效的 `get_prop(id, key)` 函数调用语法替代。

`.rgui`（声明结构）：

```xml
<!-- my_card.rgui —— 可直接被其他 .rgui 引用 -->
<Container id="root">
  <Container id="header">
    <Text id="title" text="{props.title}" />
  </Container>
  <Container id="body">
    <slot />
  </Container>
</Container>
```

`.rhai`（paint 脚本，加载时执行一次生成 PaintOp）。**注意：当前脚本不接收参数——`bounds`/`props`/`children` 注入需待 AC02 完成后支持。以下为 AC02+AC07 完成后的目标语法：**

```rust
// my_card.rhai —— 描述组件的外观
fn paint_card(bounds, props, children) {
    let bg = props.get("background").unwrap_or(rgb(1.0, 1.0, 1.0));
    let border_color = props.get("border-color").unwrap_or(rgb(0.85, 0.85, 0.85));
    let radius = props.get("radius").unwrap_or(8.0);

    // 背景填充
    fill_rect(bounds.x, bounds.y, bounds.w, bounds.h, bg, radius);

    // 1px 边框
    fill_rect(bounds.x, bounds.y, bounds.w, 1.0, border_color, 0.0);
    fill_rect(bounds.x, bounds.y + bounds.h - 1.0, bounds.w, 1.0, border_color, 0.0);
    fill_rect(bounds.x, bounds.y, 1.0, bounds.h, border_color, 0.0);
    fill_rect(bounds.x + bounds.w - 1.0, bounds.y, 1.0, bounds.h, border_color, 0.0);

    // 递归绘制子节点（header/body 容器内的 Text 等）
    paint_children(children);
}
```

#### 9.4.2.1 Props 作用域注入（T208）

父组件 `<my_card title="Hello">` 中的 `title="Hello"` 经过解析后，注入到子组件的 props 作用域。子 `.rgui` 中的 `{title}` 绑定在展开时解析为对应的值：

```
父 ui.rgui:
  <my_card title="Hello">

子 my_card.rgui:
  <Text text="{title}" />       ← 展开后 text="Hello"

注入规则：
  - {prop_name} → 直接取父传入的 prop_name 值
  - {props.xxx} → 取父传入的 props 对象中的 xxx 属性（等价写法）
  - 父未传入的 prop → 渲染为空字符串，日志 warn
  - 嵌套 3 层以上时，props 逐层向下传递，子组件可覆盖同名 prop
```

#### 9.4.2.2 `<slot />` 替换算法（T209）

子 `.rgui` 中的 `<slot />` 声明了子节点插入位置。展开时，父组件传入的子节点列表被内联到 slot 所在位置：

```
父 ui.rgui:
  <my_card>
    <Text text="Child 1" />
    <Text text="Child 2" />
  </my_card>

子 my_card.rgui:
  <Container id="root">
    <Container id="body">
      <slot />                   ← 展开后被替换为两个 <Text> 子节点
    </Container>
  </Container>

替换规则：
  - 默认 <slot /> 接收父传入的全部子节点
  - <slot name="header" /> 仅接收父传入中带 slot="header" 的子节点
  - 子组件无 <slot /> 时，父传入的子节点被丢弃，日志 warn
```

#### 9.4.2.3 外层 Props 附加（T210）

父组件标签 `<my_card onclick="fn" variant="primary">` 中的外层 prop（onclick/variant 等）在展开时附加到子组件根节点：

```
父 ui.rgui:
  <my_card onclick="handle_click" variant="primary">

子 my_card.rgui:
  <Container id="root">          ← 展开后自动携带 onclick="handle_click" + variant="primary"

附加规则：
  - 外层 props 自动附加到子 .rgui 的根元素
  - 子组件根元素已有同名 prop 时，子组件优先（不覆盖）
  - onclick 等事件绑定同样遵循此规则——附加到根节点后，根节点可响应交互
```

#### 9.4.3 Tier 1 vs Tier 2 对比

| | Tier 1：Rust WidgetSpec | Tier 2：.rgui + .rhai paint |
|------|------|------|
| **定义者** | 框架开发者 | 应用开发者 |
| **方式** | `impl WidgetSpec for Xxx { ... }` | `.rgui` XML + `.rhai` paint 函数 |
| **每帧渲染** | `paint()` → `PaintOp` → Vello | `.rhai` 加载时生成 `PaintOp` → 每帧 Vello 直消费 |
| **热路径性能** | 编译期优化（内联、死代码消除） | 热路径相同；冷路径（脚本执行一次）略慢 |
| **热重载** | ❌ `cargo build` + 重启 | ✅ 秒级生效 |
| **适用场景** | 框架内置组件（WaAccordion 等） | 应用定制 UI、业务组件 |

#### 9.4.4 前置依赖（P0）

Tier 2 需要以下原生函数先注册到 Rhai 引擎（详见 D0 §9.5）：

| 函数 | 说明 |
|------|------|
| `fill_rect(x, y, w, h, color, radius)` | 填充矩形 |
| `draw_text(text, x, y, w, h, color, font_size)` | 绘制文本 |
| `rgb(r, g, b)` / `rgba(r, g, b, a)` | 构造颜色 |
| `paint_children(children)` | 递归绘制子节点 |

#### 9.4.5 错误处理

Tier 2 paint 脚本在加载时执行一次（非热路径），仍需处理异常场景：

| 故障 | 处理策略 |
|------|---------|
| paint 脚本语法错误 | Rhai 编译失败 → 保留上一版 PaintOp + 日志警告；渲染不中断 |
| paint 脚本运行时 panic | `catch_unwind` 捕获 → 回退到上一版 PaintOp + 日志错误 |
| `fill_rect` / `draw_text` 参数越界 | 原生函数内部 clamp 到合法范围，不 panic |
| `paint_children` 传入无效 child ID | 跳过该子节点 + 日志警告 |
| 热重载时 paint 脚本与 .rgui 结构不匹配 | dirty 标记后重执行，PaintOp 结果按 bounds 裁剪；多余的子节点绘制操作被截断 |

> **设计原则**：paint 脚本的错误绝不让渲染管线崩溃——最坏情况是组件显示上一帧的缓存内容或空白区域，不影响其他组件。

---

## 10. 与其他子系统的交互

### 10.1 与 D2（状态管理）的交互

- `WidgetSpec::view()` 从 `&Self::State` 读取状态生成 `WidgetView`
- `WidgetSpec::update()` 通过 `StoreAccessMut` 修改自身状态
- `view()` 返回的 `WidgetView` 被 D2 的 diff 引擎消费
- diff 产生 Patch，按需调用 `measure()` 和 `paint()`

### 10.2 与 D3（渲染管线）的交互

- `WidgetSpec::paint()` 生成绘制指令（通过 `PaintContext` 访问字形 Atlas）
- `WidgetSpec::measure()` 的结果被 D3 的布局引擎消费
- 渲染后端（Vello/Skia）消费 SceneGraph（由 paint() 产出构建）

### 10.3 与 D4（样式系统）的交互

- `ViewContext::theme` 提供当前主题信息
- `WidgetView::props` 中的样式属性由 D4 的样式引擎解析和合并
- 组件的 `name()` 用于样式选择器匹配

### 10.4 与 D5（事件系统）的交互

- `WidgetSpec::update()` 是事件处理的目标
- `MessageBinding` 定义了消息在组件树中的路由
- `EventSender` 允许组件向父组件发送事件

### 10.5 与 D6（无障碍）的交互

- `WidgetSpec::accessibility()` 返回的 `AccessibilityNode` 被 D6 的无障碍树消费
- `AccessContext::focus_path` 用于判断焦点位置

### 10.6 与 D7（开发反馈）的交互

- 组件注册表在快速重启时保持不变
- 持久状态在重启后恢复，组件自动从新状态重新 view

---

## 11. 边界情况处理

### 11.1 空状态

- `view()` 返回空的 WidgetView（无子节点、无 props）：合法，渲染为空区域
- `children` 为 `Vec::new()`：表示叶子节点，不渲染子内容
- `State` 为 ZST（零大小类型）：`PersistState` 的实现返回空 schema

### 11.2 未注册的组件类型

- `WidgetView::widget_type` 指向未注册的名称时，框架在 diff 阶段检测
- 行为：记录错误日志，渲染占位矩形（红底白叉，200×200），不 panic

### 11.3 组件异常

- `view()` 中 panic：框架捕获（通过 `std::panic::catch_unwind`），渲染错误占位符
- `update()` 中 panic：同上，状态回滚到 panic 前（通过 `std::panic::catch_unwind` + 状态 clone）
- `paint()` 中 panic：跳过此组件的绘制，渲染错误占位符

### 11.4 循环订阅

- Widget A 读 Widget B 的状态，Widget B 读 Widget A 的状态
- 检测：D2 的订阅系统检测循环依赖（BFS 遍历订阅图）
- 处理：记录警告日志，打破循环（使用缓存状态而非实时读取），不 panic

### 11.5 大量子节点

- WidgetView 单层 children > 1000 时，diff 算法自动启用时间切片
- 将 diff 拆分为多帧执行，每帧预算 ≤ 1ms
- 未完成 diff 的部分保持上一帧的渲染结果

### 11.6 组件树深度过大

- 最大嵌套深度：256 层
- 超过限制：框架在 view() 返回时断言（debug build），release build 中截断

### 11.7 消息类型不匹配

- `MessageBinding` 的 handler 期望特定消息类型
- 运行时检测：子组件发出消息后，框架通过 TypeId 检查消息类型是否匹配
- 不匹配时：记录错误日志，丢弃消息

---

## 12. 验证标准

### 12.1 单元测试验证

| 验证项 | 测试方法 | 预期结果 |
|--------|---------|---------|
| WidgetView 构造与相等性 | 构造两个相同属性的 WidgetView，assert_eq | 相等 |
| WidgetView 嵌套 | 构造含 3 层嵌套的 WidgetView，验证 children 深度 | children 正确嵌套 |
| 组件注册 | 注册 Button，按名称查找 | 返回正确工厂 |
| 重复注册 | 注册同名组件两次 | 返回 RegistryError |
| html! 宏展开 | 编译时展开 html! {} 并验证类型 | 展开为 WidgetView\\<M\\> |
| 派生宏 PersistState | #[derive(PersistState)] 编译通过 | schema_name/version 正确 |

### 12.2 集成测试验证

| 验证项 | 测试方法 | 预期结果 |
|--------|---------|---------|
| 组件完整生命周期 | 创建→挂载→更新→卸载→验证 WidgetId 回收 | 各阶段回调正确触发 |
| 第三方组件注册和使用 | 注册外部 crate 的组件，渲染到窗口 | 正常显示和交互 |
| 未注册组件降级 | 使用未注册的 widget_type 创建视图 | 显示错误占位符，不 crash |
| 1000 节点 WidgetView diff | 构造 1000 节点 WidgetView，测量 diff 时间 | < 1ms（V5 验证基准） |

### 12.3 文档质量验证

- [ ] 所有 struct/enum 有中文文档注释
- [ ] WidgetSpec 每个方法的契约有明确的前置/后置条件
- [ ] 第三方组件开发指南（基于本文档）可独立理解
- [ ] `html!` 宏的错误提示覆盖常见错误场景

### 12.4 D0 不变式验证

- [ ] 不变式 1（核心零平台依赖）：WidgetSpec trait 定义在 rgui-core，无平台依赖
- [ ] 不变式 2（业务状态不持有 GPU 资源）：PersistState 约束在本文档中明确声明
- [ ] 不变式 3（WidgetView 是纯值类型）：WidgetView 不含闭包/引用/副作用接口
- [ ] 不变式 6（WidgetId 全局唯一）：WidgetRegistry 分配 WidgetId 时确保唯一性

---

> **下一步：** 本文档经评审确认后，进入 D2（状态管理与差分更新设计）。D2 将定义 diff 算法、Patch 结构、订阅模型和快照协议。
