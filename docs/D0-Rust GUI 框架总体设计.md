# Rust GUI 框架总体设计

> **文档定位：** 本设计文档将《Rust GUI 框架技术路线书》中的架构决策转化为具体设计——定义 crate 拆分、trait 体系、模块边界和跨子系统契约。本文档是 D1-D7 子系统详细设计的约束边界和索引入口。

> **前置阅读：** [Rust GUI 框架技术路线书](./Rust%20GUI%20框架技术路线书.md)（§4 总架构设计、§5 关键子系统设计、§6 完整数据流）
>
> **验证基础：** 技术路线验证已全部通过（V1-V10），关键 API 使用模式见各验证项代码。本文档中标注 `[V{N}]` 的设计元素直接引用对应验证项的已验证模式。

> **状态：** 初版——D1-D7 子系统设计过程中可能对本文档做小幅调整。

---

## 目录

1. [设计目标与范围](#1-设计目标与范围)
2. [Crate 结构](#2-crate-结构)
3. [核心 Trait 体系](#3-核心-trait-体系)
4. [模块职责与公共 API 边界](#4-模块职责与公共-api-边界)
5. [关键数据结构](#5-关键数据结构)
6. [数据流总览](#6-数据流总览)
7. [跨子系统不变式](#7-跨子系统不变式)
8. [命名与代码规范](#8-命名与代码规范)
9. [Rust/Rhai 逻辑分层边界](#9-rustrhai-逻辑分层边界)
10. [与子系统设计文档的接口](#10-与子系统设计文档的接口)

---

## 1. 设计目标与范围

### 1.1 本文档的目标

定义框架的顶层结构，回答以下问题：

1. 框架由哪些 crate 组成？各自承担什么职责？
2. crate 之间的依赖方向是什么？（哪些是核心，哪些是平台适配层）
3. 核心 trait 的完整签名长什么样？（组件、状态、渲染后端的抽象边界）
4. 跨子系统的不变式是什么？（哪些规则在所有子系统中不可违反）
5. 应用代码和第三方组件如何与框架交互？（public API 的形状）

### 1.2 不在本文档范围内

- 子系统内部的详细算法（见 D1-D7）
- `.rgss` 样式语言的完整语法（见 D4）
- 具体组件的实现（Button、DataGrid 等）
- 构建配置（Cargo features、feature flags）

---

## 2. Crate 结构

### 2.1 拆分原则

1. **核心零依赖**：`rgui-core` 不依赖任何平台/GPU/窗口 crate，只依赖 `std` 和基础序列化库
2. **单向依赖**：从核心到外围，严禁循环依赖
3. **按职责拆分**：一个 crate 只解决一个问题（渲染、布局、样式、平台……）
4. **公共 API 集中暴露**：顶层 `rgui` facade crate 重新导出所有面向用户的 API

### 2.2 Crate 拓扑

```
                          ┌─────────────────────┐
                          │        rgui          │
                          │   （Facade Crate）    │
                          │ 重新导出全部公共 API   │
                          └──────────┬──────────┘
                                     │ 依赖全部
              ┌──────────────────────┼──────────────────────┐
              │                      │                      │
    ┌─────────▼────────┐  ┌─────────▼────────┐  ┌─────────▼────────┐
    │   rgui-render    │  │   rgui-style     │  │   rgui-devtools  │
    │   渲染引擎        │  │   样式系统        │  │   开发工具        │
    └────────┬─────────┘  └─────┬─────┬──────┘  └──┬──────┬───────┘
             │                  │     └─────────────┘      │
             │                  │            │              │
    ┌────────▼────────┐  ┌──────▼──────┐    │              │
    │   rgui-layout   │  │ rgui-platform│    │              │
    │   布局引擎        │  │  平台抽象     │    │              │
    └────────┬─────────┘  └──────┬───────┘   │              │
             │                   │           │              │
    ┌────────▼────────┐  ┌───────▼───────┐   │   ┌──────────▼────────┐
    │    rgui-state   │  │    rgui-a11y   │   │   │                  │
    │    状态管理       │  │    无障碍系统   │   │   │                  │
    └────────┬─────────┘  └───────┬───────┘   │   │                  │
             │                    │           │   │                  │
             └──────────┬─────────┘           │   │                  │
                        │           ┌─────────┘   │                  │
                        │           │  ┌───────────┘                  │
              ┌─────────▼───────────▼──▼──────────────────────────────┘
              │
    ┌─────────▼────────┐
    │    rgui-core      │
    │    核心类型与 trait │
    │   （零平台依赖）    │
    └───────────────────┘
```

依赖方向：所有 crate 依赖 `rgui-core`，平台/渲染 crate 之间无横向依赖。

`rgui-devtools` 除依赖 `rgui-core` 外，还依赖：
- `rgui-state`（快速重启的状态快照协议）
- `rgui-style`（监控 `.rgss` 文件变更以触发热重载）
- `rgui-script`（Rhai 引擎集成，脚本热重载）

### 2.3 各 Crate 职责

| Crate | 职责 | 核心依赖（除 rgui-core 外） | 是否平台相关 |
|-------|------|---------------------------|------------|
| **rgui-core** | WidgetView、WidgetId、PropValue、WidgetSpec trait、AppMessage trait、基础几何类型、无障碍基础类型（AccessibilityNode/Role/Action/State） | `std`、`serde`（可选 feature） | ❌ 纯 Rust |
| **rgui-state** | StateStore、PersistState trait、diff 算法、快照与迁移协议、订阅模型 | `serde`、`postcard` | ❌ 纯 Rust |
| **rgui-render** | RenderBackend trait、Scene 抽象、Vello 后端实现、Skia 后端（预留）、字形 Atlas 管理 | `wgpu`、`vello`、`cosmic-text` | ✅ GPU 相关 |
| **rgui-layout** | Taffy 封装、布局缓存。从 `LayoutStyle` 提取布局属性 → Taffy Style | `taffy` | ❌ 纯 Rust |
| **rgui-style** | `.rgss` 解析器、主题系统、选择器引擎、样式热重载、CSS 属性定义。产出 `BTreeMap<String, PropValue>` 样式属性集 | `cssparser`（或自研解析器） | ❌ 纯 Rust |
| **rgui-platform** | 窗口管理、输入事件、IME、剪贴板、拖放 | `winit` | ✅ 平台相关 |
| **rgui-a11y** | AccessibilityBackend trait、AccessKit 集成、无障碍树生成、焦点管理 | `accesskit`、`accesskit_winit` | ✅ 平台相关 |
| **rgui-devtools** | 资源热重载 watcher、Rust 快速重启协议、双进程通信、状态恢复序列化 | `notify`、`serde_json`；内部依赖 `rgui-state`（快照协议）、`rgui-style`（.rgss 监控） | ❌ 纯 Rust |
| **rgui-macros** | `html!` 声明式宏、`#[derive(WidgetSpec)]`、`#[derive(AppMessage)]`、`#[derive(PersistState)]` | `syn`、`quote`、`proc-macro2` | ❌ 纯 Rust |
| **rgui-components** | 内置组件库（Button、TextField、DataGrid、Form 等） | `rgui-core` | ❌ 纯 Rust |
| **rgui** | Facade crate：重新导出所有公共 API、`html!` 宏、`App` 启动器 | 所有上述 crate | ❌ 纯 Rust |
| **rgui-script**（✅ 已实现） | Rhai 绑定、脚本热重载、命令处理器注册 | `rhai` | ❌ 纯 Rust |

> **layout-style 交互契约**：`rgui-style` 解析 `.rgss` 产生样式属性键值对（`BTreeMap<String, PropValue>`）；`rgui-layout` 从其中提取布局相关键（`display`、`flex-direction`、`gap`、`grid-template-columns` 等）→ 映射为 Taffy `Style`。两者通过 `LayoutStyle` 结构体交互（定义于 `rgui-core` 的 geometry 模块）。

> **测试分层原则**：各 crate 的测试方式由 D9（测试策略）统一定义。顶层原则：`rgui-core`/`rgui-state`/`rgui-style`/`rgui-layout` 走纯单元测试；`rgui-render` 增加离屏渲染截图回归；`rgui-platform`/`rgui-a11y` 增加平台 mock 集成测试；`rgui` facade 走 E2E 测试。

### 2.4 Crate 命名

所有 crate 以 `rgui-` 为前缀。顶层 facade crate 命名为 `rgui`。命名原则：

- 使用者通过 `rgui` 依赖所有能力
- 需要精细控制的用户可以直接依赖子 crate（如只用 `rgui-render`）
- 第三方组件依赖 `rgui-core` 即可实现 `WidgetSpec`

### 2.5 Rust 最低支持版本（MSRV）

框架统一要求 **Rust 1.85+**（stable）。各子 crate 在 `Cargo.toml` 中统一设置 `rust-version = "1.85"`。CI 中同时测试 MSRV 和 latest stable。

选择 1.85 的理由：该版本包含框架依赖项（`wgpu 24`、`vello 0.8`、`taffy`）所需的稳定特性。当关键依赖升级导致 MSRV 需要提升时，minor version 的 changelog 中单独标注。

### 2.6 版本兼容性策略（0.x 阶段）

在 1.0 之前适用以下原则：

- **Minor 版本（0.x → 0.y）** 可能包含 breaking change。变更内容在 changelog 中完整列出
- **Patch 版本（0.x.y → 0.x.z）** 仅包含 bug 修复和安全更新，不包含 breaking change
- **Deprecation** 至少提前一个 minor version 在 changelog 中通知，标注替代方案
- **1.0 之后** 遵循标准 semver（主版本号变更 = breaking change）
- 完整的兼容性政策在 1.0 发布前独立成文

---

## 3. 核心 Trait 体系

### 3.1 Trait 层次

```
PersistState           AppMessage            WidgetSpec
  (可序列化)            (消息类型)             (组件规范)
     │                     │                     │
     ▼                     ▼                     ▼
┌─────────┐         ┌──────────┐        ┌─────────────┐
│ 状态快照  │         │ 事件分发  │        │ 组件注册表   │
│ 时间旅行  │         │ 消息路由  │        │ 视图生成     │
└─────────┘         └──────────┘        └─────────────┘
                                                  │
                                          ┌───────▼────────┐
                                          │ RenderBackend  │
                                          │  (渲染后端抽象)  │
                                          └────────────────┘
```

### 3.2 WidgetSpec —— 组件规范

> 完整契约、调用时序和 Context 类型详细定义见 D1 §2-§6。

```rust
pub trait WidgetSpec: Send + Sync + 'static {
    /// 组件持有的业务状态类型
    type State: PersistState;

    /// 组件产生的消息类型
    type Message: AppMessage;

    /// 组件的唯一名称（用于调试、注册、序列化）
    fn name(&self) -> &'static str;

    /// 从持久状态派生声明式视图。应为纯函数。
    /// - `state`: 只读业务状态
    /// - `ctx`: 视图上下文（主题、区域设置等）
    fn view(&self, state: &Self::State, ctx: &ViewContext) -> WidgetView<Self::Message>;

    /// 处理来自 UI 的消息。只能修改自身的持久状态。
    /// - `msg`: 消息
    /// - `state`: 可变的自身业务状态
    /// - `ctx`: 更新上下文（可访问其他 widget 状态、发起事件）
    fn update(
        &self,
        msg: Self::Message,
        state: &mut Self::State,
        ctx: &mut UpdateContext,
    );

    /// 纯测量：根据约束计算组件期望尺寸。不写状态。
    fn measure(
        &self,
        state: &Self::State,
        constraints: BoxConstraints,
        ctx: &MeasureContext,
    ) -> Size;

    /// 绘制：将当前状态转换为绘制指令。可访问渲染缓存（由框架持有）。
    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext);

    /// 生成无障碍节点。框架在布局后调用，结果推入 AccessibilityTree。
    fn accessibility(
        &self,
        state: &Self::State,
        ctx: &AccessContext,
    ) -> AccessibilityNode;
}
```

> **派生宏：** 框架将提供 `#[derive(WidgetSpec)]` 派生宏，自动为 `update()` 生成空实现、为 `accessibility()` 生成返回 `AccessibilityNode::none()` 的默认实现，减轻简单组件的样板代码负担。详细设计见 D1。

**与验证代码的关系**：`view()` 返回的 `WidgetView` 类型由 V5 验证了 diff 性能（1000 节点 < 1ms）。`WidgetSpec` 的 `paint()` 签名与 V1 的 Scene 构建模式一一对应。

### 3.3 PersistState —— 持久状态

```rust
/// 可持久化的业务状态。完整定义见 D1 §2.4。
pub trait PersistState: erased_serde::Serialize + Send + Sync + 'static {
    fn schema_name() -> &'static str;       // 关联函数
    fn schema_version() -> u32;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
```

**设计约束**：`PersistState` 不允许持有 GPU 资源句柄（纹理 ID、Buffer 引用）、平台句柄（窗口 ID）、文件描述符。这些属于实例态和缓存态，由框架统一持有。

### 3.4 AppMessage —— 消息类型

```rust
/// 组件产生的消息。
///
/// 约束：必须是 'static、可跨线程传递、可调试。
/// 推荐使用 derive 宏自动生成。
pub trait AppMessage: Send + Sync + 'static + std::fmt::Debug + Clone {
    /// 消息名称（用于调试和日志）
    fn message_name(&self) -> &'static str;
}
```

### 3.5 RenderBackend —— 渲染后端抽象

```rust
/// 渲染后端抽象。
///
/// 主实现：VelloBackend（基于 Vello + wgpu）
/// 预留实现：SkiaBackend（基于 skia-safe）
///
/// 此 trait 由框架内部使用，不对第三方组件暴露。
pub trait RenderBackend: Send + Sync {
    /// 提交场景图并渲染到表面
    fn render(
        &mut self,
        scene: &SceneGraph,
        surface: &RenderSurface,
        params: &RenderParams,
    ) -> Result<(), RenderError>;

    /// 注册纹理数据（用于字形 Atlas）。
    /// 各后端自行处理数据到 GPU 纹理的转换。
    fn register_texture(
        &mut self,
        data: &TextureData,
        format: TextureFormat,
    ) -> TextureId;

    /// 释放纹理
    fn unregister_texture(&mut self, id: TextureId);

    /// 更新纹理内容（用于字形 Atlas 动态增长）。
    fn update_texture(
        &mut self,
        id: TextureId,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: &[u8],
    );

    /// 当前后端名称（用于调试和 fallback 决策）
    fn backend_name(&self) -> &'static str;
}
```

**设计理由**：路线书 §9 已将 Skia 列为 Vello 的 fallback。RenderBackend trait 使渲染后端可切换，而不影响框架的其余部分。`[V1]` 验证了整个 Vello 渲染链路，`[AV2]` 验证了 Skia 基本可行性。

---

## 4. 模块职责与公共 API 边界

### 4.1 rgui-core 的公共 API

```rust
// rgui-core 重新导出的公共类型（所有上游 crate 可见）

// 标识
pub use id::{WidgetId, WindowId};

// 视图
pub use view::{WidgetView, PropValue, Key};

// 几何
pub use geometry::{Rect, Size, Point, BoxConstraints};

// 核心 trait
pub use traits::{WidgetSpec, PersistState, AppMessage};

// 上下文
pub use context::{ViewContext, UpdateContext, MeasureContext, PaintContext, AccessContext};

// 无障碍类型
pub use a11y::{AccessibilityNode, AccessibilityRole, AccessibilityAction, AccessibilityState};
```

`rgui-core` 的 API 稳定承诺：1.0 之前，trait 方法只增不减，类型字段只增不减。

### 4.2 rgui-state 的公共 API

```rust
// 状态存储
pub use store::{StateStore, StoreAccess, StoreAccessMut, WidgetState};

// diff
pub use diff::{diff, Patch};

// 快照
pub use snapshot::{Snapshot, Snapshotter, SchemaMigration};

// 订阅
pub use subscription::{Subscription, Dependency};
```

`rgui-state` 仅依赖 `rgui-core`。它不依赖任何渲染、平台、样式相关 crate。

### 4.3 rgui-render 的公共 API

```rust
// 渲染后端
pub use backend::RenderBackend;

// Vello 后端（默认）
pub use vello_backend::VelloBackend;

// 场景图（详细类型见 D3 §3）
pub use scene::{SceneGraph, SceneLayer, DrawCommand, PathData, Paint};

// 渲染参数
pub use params::RenderParams;

// 表面
pub use surface::RenderSurface;
```

`rgui-render` 依赖 `rgui-core`（几何类型）、`wgpu`、`vello`、`cosmic-text`。

### 4.4 rgui-layout 的公共 API

```rust
// 布局引擎
pub use engine::{LayoutEngine, LayoutTree, LayoutNode};

// CSS 属性映射
pub use style_mapping::{to_taffy_style, from_css_display, from_css_size};

// 布局缓存
pub use cache::{LayoutCache, CachedLayout};
```

`rgui-layout` 依赖 `rgui-core`（几何类型）、`taffy`。

### 4.5 rgui-platform 的公共 API

```rust
// 窗口
pub use window::{Window, WindowConfig, WindowEvent};

// 输入
pub use input::{KeyEvent, MouseEvent, TouchEvent, Modifiers};

// IME
pub use ime::{ImeEvent, ImePosition, CandidateWindow};

// 剪贴板
pub use clipboard::Clipboard;
```

`rgui-platform` 依赖 `rgui-core`、`winit`。

### 4.6 rgui-a11y 的公共 API

> AccessibilityNode / AccessibilityRole / AccessibilityAction / AccessibilityState 等基础无障碍类型由 `rgui-core` 定义，`rgui-a11y` 重新导出并在此基础上增加 AccessKit 平台桥接。

```rust
// 无障碍后端
pub use backend::AccessibilityBackend;

// AccessKit 后端
pub use accesskit_backend::AccessKitBackend;

// 树结构
pub use tree::{AccessibilityTree, AccessibilityNode, TreeUpdate};

// 焦点管理
pub use focus::FocusManager;
```

### 4.7 rgui-style 的公共 API

```rust
// 样式表
pub use stylesheet::{StyleSheet, StyleRule, Selector};

// 主题
pub use theme::{Theme, ThemeVariables, ColorScheme};

// 解析
pub use parser::parse_rgss;

// 热重载
pub use hot_reload::StyleHotReload;
```

### 4.8 rgui-devtools 的公共 API

```rust
// 热重载 watcher
pub use hot_reload::{FileWatcher, ChangeSet};

// 快速重启
pub use fast_restart::{RestartProtocol, StateTransfer};

// 双进程通信
pub use ipc::{DisplayProcess, AppProcess, IpcMessage};
```

### 4.9 rgui（Facade）的公共 API

```rust
// rgui 重新导出所有子 crate 的公共 API
pub use rgui_core::*;
pub use rgui_state::*;
pub use rgui_render::*;
pub use rgui_layout::*;
pub use rgui_platform::*;
pub use rgui_a11y::*;
pub use rgui_style::*;

// html! 宏
pub use rgui_macros::html;

// 应用启动器
pub use app::{App, AppConfig};
```

---

## 5. 关键数据结构

### 5.1 WidgetView —— 声明式视图

```rust
/// WidgetView 是轻量值类型，描述 UI 结构。
/// 由 view() 返回，框架负责 diff 并应用到 retained tree。
///
/// 设计约束：
/// - 不持有状态引用（所有数据是值或 Arc）
/// - 不包含闭包（消息通过 Message 类型传递）
/// - Clone 开销可控（props 和 children 数量通常在 100 以内）
#[derive(Clone, PartialEq, Debug)]
pub struct WidgetView<M: AppMessage> {
    /// widget 类型名（用于查找 WidgetSpec 注册项）
    pub widget_type: &'static str,

    /// 可选的稳定 ID（用于跨 diff 追踪同一逻辑节点）
    pub id: Option<WidgetId>,

    /// 列表 key（用于列表 reconciliation，类比 React key）
    pub key: Option<Key>,

    /// 属性映射
    pub props: BTreeMap<&'static str, PropValue>,

    /// 子视图
    pub children: Vec<WidgetView<M>>,

    /// 消息绑定（子组件触发消息 → 父组件的处理方式）
    pub message_bindings: Vec<MessageBinding<M>>,
}

/// 列表 diff 的稳定标识（类比 React key）。
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Key(pub Arc<str>);

/// 属性值类型
#[derive(Clone, PartialEq, Debug)]
pub enum PropValue {
    Str(Arc<str>),
    Bool(bool),
    Int(i64),
    /// f64 不实现 Eq/Ord，使用 OrderedFloat 包裹以支持 PropValue 的 PartialEq 派生
    Float(OrderedFloat<f64>),
    Color(Color),
    Size(Size),
    Rect(Rect),
    /// 列表属性（如 grid-template-columns）
    List(Vec<PropValue>),
    /// 嵌套映射（如自定义数据）
    Map(BTreeMap<Arc<str>, PropValue>),
    /// 枚举值（如 variant: Primary）
    Enum(Arc<str>),
    /// 回调值（用于 html! 宏事件绑定，见 D5）
    Callback(Callback),
    /// 预计算的绘制操作列表（Tier 2 脚本 paint 产出，每帧热路径直接消费）
    PaintOps(Vec<PaintOp>),
}
```

```rust
/// 消息绑定：子组件的消息 → 父组件的处理方式。
/// 详细设计见 D1 §3.5 和 D5 事件系统。
pub struct MessageBinding<M: AppMessage> {
    pub source: WidgetId,
    pub message_name: Option<&'static str>,  // 消息名称过滤
    pub handler: MessageHandler<M>,
}
```

**与验证代码的关系**：`[V5]` 验证了不含 `message_bindings` 和 `Key` 的简化版 WidgetView 在 1000 节点树上的 diff 性能。完整版增加 `message_bindings` 和 `Key` 字段，其 diff 开销在独立 benchmark 中验证（见 D2）。

### 5.2 StateStore —— 状态存储

```rust
pub struct StateStore {
    /// 持久业务状态（可快照、可迁移）
    ///
    /// 使用 FxHashMap 而非标准 HashMap：WidgetId 是 u64，
    /// FxHash 对整数键的哈希速度远优于 SipHash。
    /// StateStore 存取在每帧热路径上，哈希性能直接影响帧率。
    persistent: FxHashMap<WidgetId, Box<dyn PersistState>>,

    /// 实例态（焦点、悬浮、命中测试、节点句柄）
    /// 由运行时持有，不对应用暴露
    instance: FxHashMap<WidgetId, InstanceState>,

    /// 渲染与布局缓存（字形缓存、布局结果、GPU 纹理 ID）
    /// 由运行时持有，不参与快照
    caches: FxHashMap<WidgetId, RenderLayoutCache>,

    /// 脏标记集合
    dirty: FxHashSet<WidgetId>,

    /// 订阅关系（widget A 读取 widget B 的状态时自动建立）
    subscriptions: FxHashMap<WidgetId, Vec<Subscription>>,
}
```

```rust
/// 可变状态访问句柄。持有此句柄的代码可修改自身业务状态、
/// 读取其他 widget 状态（自动建立订阅）。详细 API 见 D2。
pub struct StoreAccessMut<'a> { /* D2 定义 */ }

/// 只读状态访问句柄。详细 API 见 D2。
pub struct StoreAccess<'a> { /* D2 定义 */ }
```

### 5.3 SceneGraph —— 场景图

```rust
/// 场景图：Widget 树 → 绘制指令的中间表示。
/// 仅 dirty widget 重新生成。
pub struct SceneGraph {
    /// 绘制层（按 z-order 排列）
    pub layers: Vec<SceneLayer>,

    /// 本帧发生变化的层索引（供渲染后端做增量提交优化）
    pub dirty_layers: Vec<usize>,

    /// 裁剪区域（用于虚拟滚动等场景）
    pub clip_regions: Vec<ClipRegion>,

    /// 纹理引用（字形 Atlas 等）
    pub texture_refs: Vec<TextureRef>,
}

pub struct SceneLayer {
    pub z_index: i32,
    pub bounds: Rect,
    pub commands: Vec<DrawCommand>,
}

pub enum DrawCommand {
    FillRect { rect: Rect, color: Color, radius: f32 },
    /// PathData / Paint 定义于 rgui-render，详见 D3 渲染管线
    FillPath { path: PathData, paint: Paint },
    /// GlyphData 定义于 rgui-render，详见 D3 渲染管线
    DrawGlyphs { glyphs: Vec<GlyphData>, font_size: f32, color: Color },
    DrawImage { texture_id: TextureId, src: Rect, dst: Rect },
}
```

### 5.4 渲染相关基础类型

```rust
/// 纹理像素数据。各渲染后端自行处理到 GPU 纹理的转换。
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// 纹理颜色格式。
pub enum TextureFormat {
    Rgba8Unorm,
    Bgra8Unorm,
    // 详细变体见 D3 渲染管线
}

/// 纹理句柄（渲染后端注册纹理后的标识）。
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct TextureId(pub u64);
```

### 5.5 Context 类型（骨架）

以下 Context 类型在 `WidgetSpec` trait 方法中使用。本节给出职责说明和关键字段清单，完整定义见对应子系统文档。

#### ViewContext

```rust
/// view() 的上下文：提供只读的环境信息。
pub struct ViewContext {
    /// 当前主题引用（定义于 rgui-style，详见 D4）
    pub theme: &'static Theme,
    /// 区域设置（定义于 rgui-style，详见 D4）
    pub locale: &'static Locale,
    /// 窗口逻辑尺寸
    pub window_size: Size,
}
```
> 详细定义见 D1 组件模型。

#### UpdateContext

```rust
/// 向父组件发送事件。详细 API 见 D5 事件系统。
pub struct EventSender { /* D5 定义 */ }

/// update() 的上下文：提供状态读写和事件发送能力。
pub struct UpdateContext<'a> {
    /// 访问持久状态存储
    pub store: StoreAccessMut<'a>,
    /// 向父组件发送事件
    pub event_sender: EventSender,
    /// 当前焦点 widget ID
    pub focus: Option<WidgetId>,
}
```
> 详细定义见 D2 状态管理。

#### MeasureContext

```rust
/// measure() 的上下文：提供字体度量和 DPI 信息。
pub struct MeasureContext {
    /// 字体度量缓存（定义于 rgui-render，详见 D3）
    pub font_metrics: &'static FontMetricsCache,
    /// 当前 DPI 缩放比例
    pub scale_factor: f64,
}
```
> 详细定义见 D3 渲染管线（布局阶段）。

#### PaintContext

```rust
/// paint() 的上下文：提供渲染缓存句柄。
pub struct PaintContext<'a> {
    /// 字形 Atlas 句柄（定义于 rgui-render，详见 D3）
    pub glyph_atlas: &'a mut GlyphAtlas,
    /// 当前裁剪区域
    pub clip_rect: Rect,
    /// 渲染缓存（定义于 rgui-render，详见 D3）
    pub render_cache: &'a mut RenderLayoutCache,
}
```
> 详细定义见 D3 渲染管线（绘制阶段）。

#### AccessContext

```rust
/// accessibility() 的上下文：提供可见区域和焦点路径信息。
pub struct AccessContext {
    /// 当前可见区域（滚动容器内的视口）
    pub visible_bounds: Rect,
    /// 从根节点到当前 widget 的焦点路径
    pub focus_path: Vec<WidgetId>,
}
```
> 详细定义见 D6 无障碍系统。

---

## 6. 数据流总览

### 6.1 帧循环

```
每帧固定执行顺序（来自路线书 §6，经 V1/V6 验证）：

fn tick(&mut self) {
    // 1. 事件分发（平台事件 → WidgetSpec::update()）
    //    可能修改持久状态 → 标记 dirty
    self.dispatch_events();

    // 2. 布局计算（仅 dirty widget 子树重新布局）
    //    Taffy compute_layout → 缓存结果
    self.layout_dirty();

    // 3. 无障碍同步（遍历 widget 树 → 生成 AccessibilityNode → 推送 accesskit）
    self.update_accessibility();

    // 4. 场景图生成（仅 dirty widget 重新 paint → SceneGraph）
    let scene = self.generate_scene();

    // 5. GPU 提交（SceneGraph → Vello/Skia → wgpu → 屏幕）
    self.render(scene);
}
```

每帧预算 ≤ 17ms（目标 120fps）。

> **未来扩展（异步帧提交）：** 当前帧循环为同步执行。Vello 的 compute shader 编码（CPU）与 GPU 执行可重叠——D3 可讨论将步骤 4-5 拆分为「CPU 编码当前帧 + GPU 执行上一帧」的流水线模式，进一步提升帧率天花板。

### 6.2 完整事件处理链

```
1. 平台事件捕获  →  winit → WindowEvent::MouseDown / KeyEvent / Ime(...)
                              [rgui-platform]
2. 命中测试       →  根据坐标查找最深层 WidgetId
3. 事件分发       →  WidgetSpec::update(msg, &mut state, &mut ctx)
                     - 可能修改自身持久状态
                     - 可能读取其他 widget 状态（自动建立订阅）
                     - 可能发送事件到父组件
                     [rgui-state]
3a.脚本执行       →  （若启用，阶段 2）Rhai 脚本处理消息 → 调用 StoreAccessMut
   （可选）           [rgui-script]
4. 脏标记传播     →  修改者 dirty + 所有订阅者 dirty
                     [rgui-state]
5. 视图重建       →  仅 dirty widget 执行 view() → 新 WidgetView → diff → patch
                     [rgui-state]
6. 布局重算       →  仅受影响子树用 Taffy 重算
                     [rgui-layout]
7. 无障碍更新     →  widget.accessibility() → AccessibilityTree → AccessKit
                     [rgui-a11y]
8. 场景图生成     →  widget.paint() → SceneGraph（仅 dirty 节点）
                     [rgui-render]
9. GPU 渲染       →  RenderBackend::render(scene, surface, params)
                     [rgui-render]
```

---

## 7. 跨子系统不变式

以下不变式在所有子系统中不可违反。各子系统设计文档（D1-D7）必须在「约束」章节明确列出其遵守的不变式。

### 不变式 1：核心零平台依赖

`rgui-core` 的 Cargo.toml 中 `[dependencies]` 不得出现 `wgpu`、`winit`、`vello`、`accesskit` 或任何平台相关 crate。

**验证方式**：CI 中 `cargo tree -p rgui-core --no-dev-deps` 断言无平台依赖。

### 不变式 2：业务状态不持有 GPU 资源

实现 `PersistState` 的类型不得包含 `wgpu::Texture`、`wgpu::Buffer`、`vello::ImageData` 或任何 GPU 资源句柄。

**验证方式**：代码审查 + `PersistState` derive 宏在编译期检查字段类型（见 D2）。

### 不变式 3：WidgetView 是纯值类型

`view()` 方法不得有副作用（不写文件、不发网络请求、不修改全局状态）。

**验证方式**：`view()` 接收 `&Self::State` 和 `&ViewContext`（不可变引用）。Tier 2 paint 脚本的 `fill_rect`/`draw_text` 调用仅构建 `Vec<PaintOp>` 数据结构存入 `WidgetView.props`，不写文件、不修改全局状态，同样满足纯值约束。

### 不变式 4：渲染热点路径避免动态分发

`paint()` 产生的 `DrawCommand` 列表应是具体枚举，不使用 `Box<dyn DrawCommand>`。`SceneGraph` 提交到 `RenderBackend` 时，批量处理而非逐条分发。

### 不变式 5：实例态和缓存态不对应用暴露

`StateStore` 的 `instance` 和 `caches` 字段为 `pub(crate)`。应用代码只能通过 `StoreAccess / StoreAccessMut` 访问持久状态。

### 不变式 6：WidgetId 全局唯一

每个 widget 实例在运行时拥有唯一的 `WidgetId`。`WidgetId` 不可复用。

**验证方式**：`StateStore::insert()` 断言 ID 不冲突。

### 不变式 7：默认字体保证跨平台一致

框架内置 Noto Sans CJK + Inter + Noto Color Emoji，通过 `cosmic-text` 的 `fontdb::Source::Binary` 嵌入。不使用系统字体作为默认字体。

---

## 8. 命名与代码规范

### 8.1 Rust 命名约定

遵循 ECC Rust 编码规范：

| 类别 | 约定 | 示例 |
|------|------|------|
| crate | kebab-case, `rgui-` 前缀 | `rgui-core`、`rgui-render` |
| module | snake_case | `widget_spec`、`state_store` |
| trait | PascalCase | `WidgetSpec`、`PersistState` |
| struct / enum | PascalCase | `WidgetView`、`PropValue`、`DrawCommand` |
| function / method | snake_case | `dispatch_events()`、`compute_layout()` |
| constant | SCREAMING_SNAKE_CASE | `MAX_WIDGET_DEPTH`、`DEFAULT_FONT_SIZE` |
| macro | snake_case（小写） | `html!`、`widget!` |
| type alias | PascalCase | `WidgetId`、`TextureId` |

### 8.2 注释语言

所有注释、文档字符串、错误信息使用**简体中文**（符合 ECC 语言规则）。技术术语保留英文原文。

### 8.3 错误处理

- **框架内部（rgui-* crate）**：使用 `thiserror` 定义结构化错误类型
- **应用开发 API（rgui facade）**：提供中文错误信息
- **渲染/平台层**：错误类型包含平台原始错误（透明传递，不丢失信息）
- **任何地方**：不使用 `unwrap()` 和 `expect()` 处理可恢复错误

```rust
// 框架内部错误定义示例
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("GPU 设备不可用")]
    DeviceLost(#[source] wgpu::RequestDeviceError),

    #[error("表面创建失败：{0}")]
    SurfaceCreationFailed(String),

    #[error("着色器编译失败：{0}")]
    ShaderCompilationFailed(String),
}
```

### 8.4 日志

使用 `log` crate（框架内部）：
- `trace!`：每帧每个 widget 的内部状态变更
- `debug!`：每帧 dirty 节点数量、diff patch 数量
- `info!`：应用启动、窗口创建、后端选择
- `warn!`：降级行为（如 Skia fallback 被激活）
- `error!`：渲染失败、事件循环异常

### 8.5 代码组织

```
rgui-core/
├── src/
│   ├── lib.rs            # 公共 API 重新导出
│   ├── id.rs             # WidgetId, WindowId
│   ├── view.rs           # WidgetView, PropValue, Key
│   ├── geometry.rs       # Rect, Size, Point, BoxConstraints
│   ├── traits.rs         # WidgetSpec, PersistState, AppMessage
│   ├── context.rs        # ViewContext, UpdateContext, MeasureContext 等
│   └── a11y.rs           # AccessibilityNode, Role, Action, State
```

每个文件目标 200-400 行，最大 800 行。

---

## 9. Rust/Rhai 逻辑分层边界

> **设计来源：** Qt/QML 社区的成熟分层共识——声明式 UI（QML）负责 UI 交互逻辑和组件外观定义，原生层（C++/Scene Graph）负责渲染引擎。本框架的 Rust ≈ C++、.rgui+.rhai ≈ QML+JavaScript，分层原则对齐 Qt。

### 9.1 分层模型

```
┌─ .rgui + .rhai 层 ──────────────────────────────┐
│                                                  │
│  .rgui：声明式 UI 结构                             │
│    - 组件树定义（标签、属性、子节点）                  │
│    - 布局容器（Container/Row/Column/Center 等）     │
│                                                  │
│  .rhai：行为和视觉定义                              │
│    - 事件处理与 prop 读写（onclick → handle_click）  │
│    - 表单验证、联动逻辑、导航/路由                    │
│    - paint 脚本（fill_rect / draw_text 描述外观）   │
│                                                  │
│  约束：                                           │
│    - 脚本仅在加载/热重载时执行，生成数据结构            │
│    - 每帧渲染时不执行脚本（热路径零开销）               │
│    - 单次执行 < 1ms                                │
│    - 无 I/O（文件/网络/数据库）                       │
│    - 修改后热重载，秒级生效，Rust 进程不重启            │
│                                                  │
└────────────────────┬─────────────────────────────┘
                     │ 加载时：
                     │   .rgui → parse → WidgetView<M>
                     │   .rhai paint → 执行 → Vec<PaintOp>
                     │   .rhai event → 注册 → CommandRegistry
                     │
                     │ 运行时（每帧）：
                     │   Rust walk_view_tree → 取 PaintOp → Vello GPU
                     │   脚本不参与热路径
┌─ Rust 原生层 ───────────────────────────────────┐
│                                                  │
│  职责：                                           │
│    - 绘制原语实现（fill_rect / draw_text / GPU 提交）│
│    - 布局引擎（Taffy）                              │
│    - 渲染管线（Vello/wgpu/GPU）                      │
│    - 平台窗口（winit）                               │
│    - 字符排版与文字渲染                               │
│                                                  │
│    - 网络请求、文件 I/O                               │
│    - 数据库操作                                      │
│    - 加密/安全                                       │
│    - 重计算                                          │
│    - 跨页面共享的核心业务规则                           │
│                                                  │
│  约束：                                           │
│    - 编译期类型安全                                   │
│    - 可编写单元测试                                   │
│    - 修改需要 cargo build + 重启进程                  │
│                                                  │
└──────────────────────────────────────────────────┘
```

#### 9.1.1 关键机制：脚本 paint 只执行一次

```
时间线
──────
加载/热重载时（冷路径）：
  .rhai paint 脚本
       │
       ▼  Rhai 引擎执行（一次性）
  调用原生函数 fill_rect / draw_text
       │
       ▼
  Vec<PaintOp> ──→ 存入 WidgetView.props["paint_ops"]

每帧渲染时（热路径，60fps）：
  walk_view_tree
       │
       ▼
  从 props 取出 PaintOp    ← 纯 Rust 数据结构
       │
       ▼
  Vello 提交 GPU            ← 零脚本参与
```

> **设计理由：** 完全对齐 Qt Quick Scene Graph 的架构——QML 声明式代码在加载时编译为场景图节点（`QSGNode` 树），渲染线程每帧直交遍历 C++ 节点提交 GPU，不再执行 JavaScript。rgui 同理——Rhai paint 脚本生成 `Vec<PaintOp>`，渲染线帧直接消费 Rust 数据结构。（实现细节见 [D1 §9.4.1](./D1-组件模型与WidgetSpec设计.md#941-架构加载时生成-paintop每帧纯-rust-渲染)）

### 9.2 组件定义双轨模型

框架支持两种组件定义路径，应用开发者选择其一：

| | Tier 1：Rust WidgetSpec | Tier 2：.rgui + .rhai paint 脚本 |
|------|------|------|
| **谁定义** | 框架开发者 | 应用开发者 |
| **方式** | `impl WidgetSpec for WaAccordion { ... }` | `.rgui` 标签 + `.rhai` paint 函数 |
| **性能** | 编译期优化（内联、死代码消除） | 热路径相同；冷路径（脚本执行一次）略慢 |
| **热重载** | ❌ 需要 cargo build + 重启 | ✅ 改脚本秒级生效 |
| **适用场景** | 框架内置组件库 | 应用层定制组件、业务 UI |
| **典型例子** | WaAccordion、WaButton | 自定义卡片、数据表格行模板 |

> **与 Qt/QML 对照**：Tier 1 对应 C++ 注册的 QQuickItem，Tier 2 对应纯 QML 文件定义的组件。Qt 的实践表明——大量应用层组件走 Tier 2，只有性能关键或需要调用 C++ 底层 API 的才走 Tier 1。

### 9.3 逻辑放置判据

| 判据 | 放 `.rhai` 脚本 | 放 Rust |
|------|:---:|:---:|
| 组件树结构声明 | ✅（.rgui） | — |
| 事件处理 + prop 读写 | ✅ | — |
| 组件视觉外观（paint） | ✅（加载时生成 PaintOp） | ✅（编译期 WidgetSpec） |
| 执行时间 < 1ms | ✅ | — |
| 不需要外部 I/O | ✅ | — |
| 逻辑随 UI 频繁变化 | ✅ | — |
| 需要网络 / 文件 / 数据库 | — | ✅ |
| 大量计算 / 循环（hot path） | — | ✅ |
| 跨页面共享的核心规则 | — | ✅ |
| 新的渲染原语（非已有 fill_rect） | — | ✅ |

### 9.4 与 Qt/QML 的对照

| | Qt/QML | rgui |
|------|------|------|
| **原生语言** | C++ | Rust |
| **渲染引擎** | Qt Quick Scene Graph (C++) | Vello / wgpu (Rust) |
| **脚本语言** | JavaScript (QML 内嵌) | Rhai (.rhai) |
| **声明式 UI** | QML 标记 | .rgui (XML) |
| **组件定义方式** | QML 文件或 C++ 注册 | .rgui + .rhai paint 脚本，或 Rust WidgetSpec |
| **脚本 paint 机制** | 加载时生成 QSGNode → 每帧 GPU 直接渲染 | 加载时生成 PaintOp → 每帧 Vello 直接渲染 |
| **脚本在热路径** | 不参与 | 不参与 |

> Qt/QML 社区经验：**80% 的 UI 交互逻辑可放在脚本层，20% 的核心业务规则留在原生层。** 不把整个业务层放脚本里。

### 9.5 Rhai 侧需要的原生函数绑定

| 类别 | 函数 | 优先级 |
|------|------|:---:|
| **绘制原语** 🔑 | `fill_rect` / `draw_text` | 🔴 P0 |
| Prop 读写 | `get_prop` / `set_prop` | ✅ 已实现 |
| 类型转换 | `to_int` / `to_float` / `to_string` | 🔴 P1 |
| 数组操作 | `push` / `pop` / `len` / `contains` | 🔴 P1 |
| 字符串 | `to_upper` / `to_lower` / `contains` / `starts_with` | 🔴 P1 |
| 颜色 | `rgb(r, g, b)` / `rgba(r, g, b, a)` | 🔴 P1 |
| 几何 | `rect(x, y, w, h)` | 🔴 P1 |
| 子组件递归 | `paint_children(children)` | 🔴 P1 |
| 时间 | `now_timestamp` / `format_date` | 🟡 P2 |
| 日志 | `log_info` / `log_warn` | 🟡 P2 |

> **P0 绘制原语是 Tier 2 组件定义的先决条件**——没有它们，.rhai 无法描述组件外观，只能实例化已有 Rust 组件。

### 9.6 热重载路径

```
修改 .rgui / .rhai
       │
       ▼
notify 检测文件变更（rgui-devtools 已有）
       │
       ├── 新增 .rgui/+.rhai 文件：
       │     → notify 检测文件创建（需要 watcher 监听目录，非仅文件）
       │     → 扫描同目录 .rgui/.rhai 文件对，自动注册为 Tier 2 组件
       │     → 已渲染的父组件若引用新标签名，下一帧走 Tier 2 渲染路径
       │
       ├── .rgui 变更：
       │     → parse_rgui_file() → WidgetView<M> 新树
       │     → 遍历新树，执行关联的 .rhai paint 脚本
       │     → PaintOp 写入新树 props["paint_ops"]
       │
       ├── .rhai 变更：
       │     → Rhai 引擎重新编译 AST
       │     → CommandRegistry 替换事件处理器
       │     → 标记受影响的 WidgetView 节点为 dirty
       │     → 下一帧 walk_view_tree 重执行 paint 脚本 → PaintOp 更新
       │
       ▼
渲染管线取新 WidgetView + 新 PaintOp + 新事件路由        ← Rust 进程不重启
```

> **dirty 标记机制**：当 `.rhai` paint 脚本被热重载时，框架遍历 WidgetView 树，将 `widget_type` 匹配该组件的所有节点标记为 dirty。下一帧 `walk_view_tree` 对 dirty 节点重新调用 Rhai paint 脚本生成新 `PaintOp`，非 dirty 节点继续使用缓存的 `PaintOp`。这与 D2 的增量 diff 机制一致——只重建受影响节点的绘制数据。

---

## 10. 与子系统设计文档的接口

### 10.1 各子系统文档对 D0 的引用关系

| 子系统文档 | 依赖 D0 中的 | 可修改 D0 中的 |
|-----------|------------|--------------|
| D1 组件模型 | WidgetSpec trait 签名、WidgetView 类型、ViewContext | WidgetSpec 方法（增加非破坏性方法） |
| D2 状态管理 | StateStore 结构、PersistState trait、StoreAccess API | StateStore 字段（增加缓存策略字段） |
| D3 渲染管线 | RenderBackend trait、SceneGraph 结构、帧循环 | DrawCommand 枚举（增加变体） |
| D4 样式系统 | PropValue 类型、rgui-style 的公共 API | PropValue（增加 CSS 特定值类型） |
| D5 事件系统 | 事件分发链、UpdateContext、AppMessage | UpdateContext（增加方法） |
| D6 无障碍 | AccessibilityBackend trait、AccessibilityNode | AccessibilityNode（增加字段） |
| D7 开发反馈 | rgui-devtools 的公共 API、StateStore 快照协议 | — |
| rgui-script（✅ 已实现） | AppMessage trait、事件分发链（§6.2 步骤 3a） | — |

### 10.2 修改 D0 的流程

1. 子系统设计发现冲突 → 在子系统设计文档中记录
2. 评估影响范围（是否影响其他子系统）
3. 更新 D0 相应章节
4. 检查不变式是否仍成立
5. 通知受影响的其他子系统

---

> **下一步：** 本设计经评审确认后，按 [待编写设计文档列表](./待编写设计文档列表.md) 的顺序进入 D1-D7 子系统详细设计。D1-D7 在 D0 完成后可并行推进。
