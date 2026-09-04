# rgui 全新架构设计（低复杂度绿色重构 · 定稿）

> 工作目录：`/Users/chenchao/Documents/code/rust/RUST-GUI`
> 设计方：devco-architect（方案设计师）
> 阶段：D0 设计先行（**只做设计，不写代码**）
> 依据：`tools/2025-09-01_rgui-complexity-audit.md`（§三/§四）、`tasks.md`（D0-D6 + 硬约束 C1-C4）
> 上游 git：**保留** `origin = https://github.com/ceco3000/rust-gui.git`，`.git` 与 remote 不删，仅清空工作区源码。
> 唯一准则：**一个能力，一条路径，一个归属。凡不被当前阶段需要的能力一律不做。**

---

## 设计原则（对应总监 5 条规避约束 + 6 条硬约束）

| 旧问题（复杂度教训） | 新设计规避手段 |
|---|---|
| ① 12+ crate 纯转发壳，增量收益被接口成本抵消 | crate 收敛到 **5 个**（4 核心 + 1 可选薄 facade），每个有实责；禁止为拆而拆（硬约束 A 区） |
| ② rgui-state 污染依赖 rgui-render（状态层引用 GPU 类型） | **数据/状态层零 GPU、零平台依赖**（Cargo 防火墙 + 单向 DAG，禁反向边）（硬约束 A） |
| ③ app.rs 5017 行 God Object | facade 纯重导出 + 极薄启动协调（≤200 行），职责单拆出，测试桩 cfg 隔离（硬约束 B） |
| ④ 多套机制并行（vello/skia、Tier1/Tier2、Rhai、双热重载） | 每条路径只留一条：单 vello、唯一 Tier 1 WidgetSpec、无脚本、无 devtools（硬约束 B） |
| ⑤ 6-8 层克制到 4-6 | 定稿 **5**，落在 6-8/4-6 区间下限（硬约束） |
| ⑥ proc-macro 必须独立 | `rgui-macros` 独立（**Rust 硬约束，不可合并**）（硬约束 C） |
| ⑦ lint 全 deny 拖慢开发 | **lint 克制**：unsafe_code=deny，clippy 只开 default + 必要 pedantic 子集，todo/expect/unwrap 放宽到 warn（硬约束 D） |
| ⑧ 增量验收措辞 | **"改数据/状态层 → 不重编 render"**，而非"改 core→不重编 render"（硬约束 E） |
| ⑨ 公共 API 悬空 use | **facade 再导出清单与被合并/删除模块一一核对**，防悬空 use（硬约束 B） |

---

## A. 最终 crate 拓扑（4 核心 + 1 可选 facade = 5）

### A.1 总表 + 独立/合并理由

| # | 目标 crate | 纳入/来源 | 独立/合并理由 | 行数预估 |
|---|---|---|---|---|
| 1 | `rgui-core` | 现 core + **state** + **layout** + **style** + **components** + a11y 树 | **必须独立**：唯一逻辑核心，零 GPU/零平台，是"能被单独测试、单独增量编译、被全员依赖"的防火墙基座。合并 state/layout/style/components（纯 Rust 逻辑，均无 GPU/平台/重型运行时依赖，拆分会重演转发壳）。 | ~8-9k |
| 2 | `rgui-render` | 现 render | **必须独立**：重型 GPU 依赖（wgpu/vello/cosmic-text/fontdb/skrifa）必须隔离，否则任一 GPU 依赖崩溃波及全仓库；"改渲染不重编核心"与"改核心不重编渲染"的关键。 | ~8-10k |
| 3 | `rgui-platform` | 现 platform | **必须独立**：winit 重型平台依赖必须隔离；是"平台句柄挡在核心外"的地方。**与 render 互不相依**（见 DAG）。 | ~3-4k |
| 4 | `rgui-macros` | 现 macros | **必须独立（Rust 硬约束）**：proc-macro crate 不可与普通 crate 合并。 | ~1k |
| (5) | `rgui` (facade, 可选) | 现 facade 瘦身 | **可选保留**：对外门面（重新导出 + 启动协调），提供零配置入口。`≤200 行`，不含业务逻辑。若嫌重可裁掉让用户自行组装。**本设计倾向保留**。 | ~200-400 |

### A.2 为什么合并这些（每项"为何不独立"）

| 旧 crate | 归属 | 为何不独立 |
|---|---|---|
| rgui-state | `rgui-core::state` | 状态/差分/快照/订阅是纯 Rust，无 GPU/平台隔离价值；拆分重演转发壳。 |
| rgui-layout | `rgui-core::layout` | Taffy 纯 Rust 布局，无重依赖；旧仅 609 行，纯壳。 |
| rgui-style | `rgui-core::style` | .rgss 解析是纯文本解析（非重型运行时）；且并入后走单一路径（热重载=P1，见 §G）。 |
| rgui-components | `rgui-core::components` | 组件（Accordion/WaBadge）是纯 WidgetSpec；旧 Tier1/Tier2 双路径已废，统一放核心最直观。 |
| rgui-a11y | 删独立 crate；`AccessibilityTree` 并入 `rgui-core::a11y` | AccessKit 桥（backend.rs，需 feature）是重型跨平台依赖且无消费方，删；纯类型保留并入核心。 |
| rgui-script | **删除** | Rhai 脚本绑定，非核心能力。 |
| rgui-devtools | **删除** | 开发期工具（热重载/双进程/IPC/快速重启/HTML reload），且 `.rgui` 解析随声明式路径整体废弃。 |
| rgui-template | 移出交付物 | 45 行主入口模板，判为示例脚手架。 |

### A.3 依赖方向 DAG（禁止反向边标注）

```
              rgui (facade, 可选)                ◆ 只合并，不派生业务
                    │
      ┌─────────────┼───────────────┐
      │             │               │
  rgui-core      rgui-macros     rgui-render     rgui-platform
  零GPU/零平台    (proc-macro)     (GPU 隔离)      (winit 隔离)
      ▲  ▲  ▲        ▲              ▲  ▲             ▲
      │  │  └────────┘──────────────┘  └─────────────┘
      │         全部只能向下指向 rgui-core
      └───────────────── 唯一被依赖核心（数据层） ──────────┘
```

**依赖规则（硬约束 A）**：
1. **禁反向边**：`rgui-core` **不依赖** `rgui-render`/`rgui-platform`/`rgui-macros`（数据层绝不依赖渲染层/平台层）。
   - 禁止 `rgui-render → rgui-platform` 与 `rgui-platform → rgui-render`（渲染不碰窗口，平台不碰 GPU）。
2. `rgui-render` 与 `rgui-platform` **只向下**依赖 `rgui-core`。
3. `rgui-macros` 只依赖 proc-macro 基础设施，不依赖任何运行时 crate（生成代码给 core 用）。
4. `rgui` (facade) 依赖全部四个，仅重新导出 + 启动协调。
5. 图为**有向无环图（DAG）**，无任何横向依赖或环。

**防火墙验证（契约级承诺）**：`rgui-core/Cargo.toml` 的 `[dependencies]` **不得**出现 `rgui-render`、`rgui-platform`、`rgui-macros`、`winit`、`wgpu`、`vello`、`cosmic-text` 任一。渲染层可依赖数据层（`rgui-render → rgui-core`），数据层绝不依赖渲染层。

---

## B. 各 crate 公共 API 清单（含关键契约签名）

### B.0 公共 API 再导出核对表（硬约束 B：防悬空 use）

**facade `rgui` 的 `pub use` 清单逐一核对其来源模块**（旧 → 新）：

| 旧 facade `pub use` | 来源 | 新归属 | 核对（悬空?） |
|---|---|---|---|
| `pub use rgui_core::*` | rgui-core | `rgui-core` 顶层 | ✅ 保留 |
| `pub use rgui_components::*` | rgui-components | **并入 `rgui-core::components`** | ✅ 改为 `rgui_core::*` 覆盖 |
| `pub use rgui_layout::*` | rgui-layout | **并入 `rgui-core::layout`** | ✅ 改为 `rgui_core::*` 覆盖 |
| `pub use rgui_state::*` | rgui-state | **并入 `rgui-core::state`** | ✅ 改为 `rgui_core::*` 覆盖 |
| `pub use rgui_render::*` | rgui-render | 保留（**改定向**） | ✅ 代码用 `pub use rgui_render::{GlyphKey, PathTessellation}`（定向，防通配裸抛内部类型） |
| `pub use rgui_platform::*` | rgui-platform | 保留（**改定向**） | ✅ 代码用 `pub use rgui_platform::{FocusManager, InputModality}`（定向） |
| `pub use rgui_style::*` | rgui-style | **并入 `rgui-core::style`** | ✅ 改为 `rgui_core::*` 覆盖 |
| `pub use rgui_a11y::*` | rgui-a11y | **删除独立 crate**（树并入 core） | ✅ 改为 `rgui_core::*`（含 a11y 树）；**删除 `rgui_a11y::*` 防悬空** |
| `pub use rgui_macros::{AppMessage,WidgetSpec,html}` | rgui-macros | 保留 | ✅ |
| `pub use app::run_simple_app` | `#[cfg(feature="devtools")]` | **删除**（devtools 删）| ✅ 删除防悬空 |
| `pub use app::{App,AppConfig}` | rgui::app | 保留 | ✅ |

> **结论**：直接 `use rgui_core::*` 即可覆盖全部并入模块。**逐个删除** `rgui_a11y`/`rgui_components`/`rgui_layout`/`rgui_state`/`rgui_style` 的独立 `pub use`（防悬空），仅保留 `rgui_core`/`rgui_render`/`rgui_platform`/`rgui_macros`。`run_simple_app` 因 devtools 删除而移除。

### B.1 `rgui-core`（零 GPU/平台）

```rust
// ===== 核心 trait =====
pub trait AppMessage: Send + Sync + 'static + std::fmt::Debug + Clone {
    fn message_name(&self) -> &'static str;
}
pub trait PersistState: Send + Sync + 'static {
    fn schema_name() -> &'static str where Self: Sized;
    fn schema_version() -> u32 where Self: Sized;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
pub trait WidgetSpec: Send + Sync + 'static {
    type State: PersistState;
    type Message: AppMessage;
    fn name(&self) -> &'static str;
    fn view(&self, state: &Self::State, ctx: &ViewContext) -> WidgetView<Self::Message>;
    fn update(&self, msg: Self::Message, state: &mut Self::State, ctx: &mut UpdateContext);
    fn measure(&self, state: &Self::State, constraints: BoxConstraints, ctx: &MeasureContext) -> Size;
    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext);
    fn focusable(&self) -> bool { false }   // D12：默认不可获焦，组件可覆盖（Tab 导航/焦点切换）
    fn accessibility(&self, _s: &Self::State, _c: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none()
    }
}
/// 事件传播结果（对齐代码：derive Debug, Clone, PartialEq, Eq——与 D0/实现一致）。
pub enum EventResult<M> { Handled, Prevented, Continue(M) }   // 事件传播结果

// ===== 数据/值类型 =====
// 对齐代码实现（D3-D10 验收通过，方向 A 定稿保留代码形态）：
pub struct WidgetId(u64);   // + NodeHandle/WindowId（见 id.rs，适配多视图）
pub struct NodeHandle(u64);
pub struct WindowId(u64);
pub enum Key { Str(String), Num(u64) }   // 稳定键：字符串/数值（对齐代码，非单 Key(Arc<str>)）
pub struct Color { r: u8, g: u8, b: u8, a: u8 }   // 对齐代码：8bit 通道，sRGB，可 derive Eq
impl Color { pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self; pub const fn rgb(r: u8, g: u8, b: u8) -> Self; }
pub enum PropValue { Bool(bool), Int(i64), Float(f64), Str(String), Color(Color) }
// 说明：无 WidgetId 变体（对齐代码）——视图引用 widget 用 Key/NodeHandle/MessageBinding，更类型安全。
pub struct WidgetView<M: AppMessage> { /* 节点 + props + 回调，唯一视图表示 */ }
pub fn map_message<M2>(self, f: &impl Fn(M) -> M2) -> WidgetView<M2>;   // 递归提升子节点消息类型（D11 组合/容器复用）
pub struct Size { width: f32, height: f32 }
pub struct Point { x: f32, y: f32 }
pub struct Rect { origin: Point, size: Size }
pub struct BoxConstraints { min: Size, max: Size }

// ===== 状态管理（纯 Rust，零 GPU）=====
pub mod state;
pub struct StateStore { /* 每 Widget 一份 InstanceState；订阅表；dirty 集 */ }
pub struct InstanceState { /* 当前状态 + 派生视图缓存 + 订阅 */ }
pub struct Subscription { subscriber: WidgetId, target: WidgetId }
pub enum SubscriptionLifetime { Persistent, Transient }
pub enum Patch { ... }                       // 差分补丁
pub fn diff<M>(a: &WidgetView<M>, b: &WidgetView<M>) -> Vec<Patch>;
pub struct Snapshot { ... }                  // 可序列化全量快照
pub struct Snapshotter { ... }
pub struct StoreBinding { /* Arc<RwLock<StateStore>> 封装 */ }

// ===== Context 体系 =====
pub struct ViewContext { pub focused: bool }   // D13 视图层焦点透传（组件 view 读它绘制获焦高亮）；窗口尺寸/locale 字段现不存在（对齐代码）
pub struct UpdateContext { /* 更新上下文（当前占位；焦点信息经 ViewContext.focused 传给视图，非本字段） */ }
pub struct MeasureContext { /* 只读环境 */ }
pub struct PaintContext { /* 绘制指令收集 */ }
pub struct AccessContext { /* 无障碍上下文 */ }

// ===== 布局（并入 core，Taffy 纯 Rust）=====
pub mod layout;
pub struct LayoutEngine { /* Taffy 封装 */ }
pub struct LayoutNode(u64);
pub struct LayoutResult { size: Size, position: Point }

// ===== 样式（并入 core，.rgss 文本解析）=====
pub mod style;
pub struct StyleSheet { /* 解析后的样式 */ }
// 对齐代码：parse_rgss 返回 StyleSheet（D3 占位 stub `StyleSheet::default()`；
// 实际 cssparser 解析在实现阶段补全，届时若需错误处理可升级返回 Result——新增签名改蓝图代价低）
pub fn parse_rgss(input: &str) -> StyleSheet;
pub struct StyleRule { /* selector 占位 */ }

// ===== 逻辑组件（Tier 1 WidgetSpec）=====
pub mod components;
pub struct Accordion { /* impl WidgetSpec */ }
pub struct WaBadge { /* impl WidgetSpec */ }

// ===== 无障碍薄层 =====
pub mod a11y;
pub struct AccessibilityNode { role: AccessibilityRole, label: String, children: Vec<WidgetId> }
pub enum AccessibilityAction { Activate, Focus, ... }
pub enum AccessibilityRole { Button, Container, ... }

// ===== 命中检测（core 新增，D11 多组件事件路由）=====
// 纯 Rust 几何：给定点击坐标命中第一个包含该点的已布局区域，返回其 id（上层据此路由到组件消息）。
pub mod hit_test;
pub struct HitRegion { pub rect: Rect, pub id: u32 }
impl HitRegion { pub const fn new(rect: Rect, id: u32) -> Self; pub fn contains(&self, x: f32, y: f32) -> bool; }
pub fn hit_test(x: f32, y: f32, regions: &[HitRegion]) -> Option<u32>;   // 流式 iter().find()
```

> **M1 教训制度化**：`StateStore`/`Patch`/`Snapshot` 的**所有字段类型全部来自 `rgui-core` 自身**（WidgetId/PropValue/Size），**绝不**含 `GlyphKey`/`PathTessellation`/`LayoutResult`。由 Cargo 依赖防火墙强保证。

### B.2 `rgui-render`（GPU 隔离）

```rust
pub enum RenderBackend { Vello(VelloBackend) }   // 单一 vello，无 Skia 变体
pub struct GlyphKey { /* font_id, text, size, ... */ }
pub struct GlyphCacheEntry { /* atlas UV, 尺寸 */ }
pub struct GlyphAtlas { /* GPU 纹理 */ }
pub struct RasterizedGlyph { /* 位图 */ }
pub struct PathTessellation;
pub struct SceneGraph { /* 绘制指令列表：DrawCmd{FillRect, DrawText(width 换行, D17), StrokeRect(描边, D16)}；render_to_view/render_surface 传 scale(逻辑→物理 Affine::scale, D17，render_offscreen=1.0) */ }
pub struct TextShaper;
```

**契约**：`rgui-render` 是唯一可能依赖 GPU/cosmic-text/fontdb 的 crate。`rgui-core` 通过"只存语义数据（Color/Size/PropValue）"与 render 彻底隔离。

### B.3 `rgui-platform`（winit 隔离，`winit` feature 默认启用）

> **feature 修复（对齐代码实况）**：`winit` 是 platform 核心依赖，`default = ["winit"]`（非可选）；上层/qa 默认构建即可编译，无需显式开启。

```rust
pub mod window; pub mod input; pub mod ime; pub mod focus;
pub struct Window { /* winit Window 封装 */ }
// D15 坐标层（物理→逻辑）：平台用 window.scale_factor() 更新；hit-test/布局用逻辑坐标，避高分屏/多显示器 DPI 偏移
pub fn set_platform_scale(scale: f64);                      // 事件处理时更新（thread_local PLATFORM_SCALE）
pub fn platform_scale() -> f64;                             // 默认 1.0
pub fn to_logical(physical: (f64, f64), scale: f64) -> (f32, f32);  // 物理/scale，scale>0 否则用 1.0
pub fn window_scale(window: &Window) -> f64;                // winit scale_factor
pub struct EventLoop { /* winit 事件循环封装 */ }
pub enum InputEvent { MouseMove, MouseDown, MouseUp, KeyDown, KeyUp, Char, Scroll }
pub struct FocusManager { focused: Option<WidgetId>, focusable: Vec<WidgetId> }   // 焦点管理原生在此（D12 增强）
impl FocusManager {
    pub fn set_focusable(&mut self, ids: Vec<WidgetId>);            // 注册可获焦有序列表（Tab 循环）
    pub fn set_focus(&mut self, id: WidgetId) -> bool;              // 仅可获焦才成功
    pub fn focus(&self) -> Option<WidgetId>;                        // 当前焦点
    pub fn is_focused(&self, id: WidgetId) -> bool;                 // 获焦查询
    pub fn focus_next(&mut self) -> Option<WidgetId>;               // Tab：move_focus(1)
    pub fn focus_prev(&mut self) -> Option<WidgetId>;               // Shift+Tab：move_focus(-1)
    // move_focus(dir)：iter().position + rem_euclid 循环回绕（私有）
}
pub enum InputModality { Pointer, Keyboard, Touch }   // 对齐代码（D5 已确认含 Touch）
```

### B.4 `rgui-macros`（proc-macro）

```rust
#[derive(WidgetSpec)]     // 生成 accessibility() 空实现等
#[derive(AppMessage)]     // 生成 message_name()
#[derive(PersistState)]   // 生成 schema_name/schema_version/as_any
#[macro_export] macro_rules! html { ... }   // 只生成 WidgetSpec/WidgetView 的 Rust 树（Tier 1 侧）
```

### B.5 `rgui` (facade, 可选)

```rust
// 对齐代码：定向重导出（防悬空 use / 防公共 API 污染），非通配
pub use rgui_core::*;
pub use rgui_platform::{FocusManager, InputModality};   // 定向：只暴露真公共项
pub use rgui_render::{GlyphKey, PathTessellation};       // 定向：只暴露真公共项
pub use rgui_macros::{WidgetSpec, AppMessage, PersistState, html};
pub struct App;              // 极薄启动协调
#[cfg(feature = "window")]    // App::run 仅 window feature 门控
pub fn run<W: WidgetSpec, F: FnMut(&WindowEvent) -> Option<W::Message> + 'static>(
    config: AppConfig, widget: W, state: W::State, mapper: F,
) -> Result<(), Box<dyn std::error::Error>>;
// 组装 core/render/platform：Coordinator + run_as_with_config + surface 渲染
```

---

## C. 模块边界划分（每 crate 内部 + 依赖方向，无环）

### C.1 `rgui-core` 内部

```
rgui-core/src/
├── lib.rs          (顶层 pub mod + 重导出，见 §B.1)
├── id.rs           WidgetId + NodeHandle + WindowId
├── geometry.rs     Size/Point/Rect/BoxConstraints
├── color.rs        Color (u8 通道)
├── traits.rs       AppMessage/PersistState/WidgetSpec/EventResult
├── context.rs      View/Update/Measure/Paint/AccessContext
├── view.rs         WidgetView/PropValue/Key/Callback/MessageBinding/MessageHandler
├── message.rs      消息与事件 (NoopMsg)
├── locale.rs       Locale
├── coordinator.rs  视图协调 (Coordinator)
├── registry.rs     注册表 (Registry)
├── widget_state.rs 组件状态 (WidgetState)
├── state/
│   ├── mod.rs      StateStore/InstanceState/Subscription/SubscriptionLifetime/StoreBinding
│   ├── diff.rs     Patch/diff/apply_patch
│   └── snapshot.rs Snapshot/Snapshotter/SchemaMigration
├── layout/
│   ├── mod.rs      LayoutEngine/LayoutNode/LayoutResult/LayoutStyle
│   └── mapping.rs  to_taffy_style (封装 Taffy，不暴露 Taffy 类型到公共 API)
├── style/
│   ├── mod.rs      StyleSheet/StyleRule/parse_rgss
│   └── theme.rs    (可选，最小必要)
├── components/
│   ├── mod.rs
│   ├── accordion.rs
│   └── wa_badge.rs
├── a11y/
│   ├── mod.rs      AccessibilityNode/Action/Role/State
│   └── (无 AccessKit 后端)
├── a11y_tree/
│   └── mod.rs      AccessibilityTree (由 rgui-a11y/tree.rs 迁入)
└── hit_test.rs     HitRegion/hit_test (D11 多组件事件路由，纯几何)
```

**内部依赖方向**：`traits/context/view` 是底层（被组件/状态依赖）；`layout/style` 依赖底层；`components` 依赖 traits/layout/style；`state` 依赖 traits/view 但不依赖 layout/render。全部内部依赖也非环。

### C.2 `rgui-render` 内部

```
rgui-render/src/
├── lib.rs              (重导出 RenderBackend/GlyphKey/SceneGraph/PathTessellation)
├── backend/vello.rs    (单一 vello；无 skia.rs)
├── glyph.rs            (GlyphKey/GlyphCacheEntry/GlyphAtlas/RasterizedGlyph)
├── path_tessellation.rs(PathTessellation)
├── text.rs             (TextShaper)
└── scene_graph.rs      (SceneGraph/绘制指令)
```

### C.3 `rgui-platform` 内部

```
rgui-platform/src/
├── lib.rs
├── window.rs           Window
├── event_loop.rs       EventLoop
├── input.rs            InputEvent
├── ime.rs              IME
└── focus.rs            FocusManager/InputModality
```

### C.4 跨 crate 依赖方向（唯一 DAG，无环）

```
rgui (facade) ──→ rgui-core / rgui-render / rgui-platform / rgui-macros
                      ↑         ↑             ↑             ↑
  rgui-core (零GPU/零平台) ←── rgui-render (GPU) ←── rgui-platform (winit)
                                                    （render/platform 只向下依赖 core，互不相依）
  rgui-macros → 仅 proc-macro 基础设施（不依赖运行时 crate）
```

---

## D. Feature flags 最小集

**原则：只保留"重型依赖隔离"所需 feature，每条路径只留一条，删除一切开发期增强 feature。**

| Crate | feature | 说明 |
|---|---|---|
| `rgui-core` | `default = ["layout"]`（对齐代码）；可选 `std`（预留 no_std，阶段 0 不做） | 零大型可选依赖；layout 默认启用（taffy 纯 Rust） |
| `rgui-render` | `default = []`（对齐代码，克制不默认拉起重型 GPU 编译）；`vello-backend`（wgpu/vello/cosmic-text/fontdb/skrifa） | **仅此一条渲染路径**。删 `skia-backend`/`offscreen`/`skia-safe`；vello 需显式开启 |
| `rgui-platform` | `default = ["winit"]`（对齐代码，winit 核心依赖默认启用）；`winit` feature | 保留；上层/qa 默认构建即可编译 |
| `rgui-macros` | `default = []` | 无大型运行依赖 |
| `rgui` (facade) | `default = []`；`window` = [`rgui-render/vello-backend`, `rgui-platform/winit`]（`App::run` 门控）；可选 `test-harness`（含自动化桩） | **删 `devtools`/`script`/`a11y` feature**；测试桩 `#[cfg(feature="test-harness")]` |
| workspace 根 | 删 `skia-safe`、`rgui-state`/`rgui-layout`/`rgui-components`/`rgui-script`/`rgui-devtools`/`rgui-a11y` 依赖条目 | 清理 |

---

## E. 工程配置（lint / edition / MSRV）——硬约束 D/F

### E.1 lint 克制策略（硬约束 D：避免全 deny 拖慢开发）

**`lints.toml`（或 workspace `[workspace.lints]`）**：

```toml
[lints.rust]
unsafe_code = "deny"          # 唯一强制 deny——Rust 安全核心
missing_debug_implementations = "warn"   # 降级为 warn，不阻断
unused = "warn"

[lints.clippy]
# 仅 default + 必要 pedantic 子集，其余放宽
# 注意：clippy 不是默认启用（需 cargo clippy），以下为 -D warnings 时的取舍
# 在 CI 用 `cargo clippy --workspace -- -D warnings`，但通过 lint 配置把以下放宽：
todo = "warn"                 # 放宽：允许 todo 不阻断（旧全 deny 拖慢）
expect = "warn"               # TODO in non-Release: 放宽
unwrap_used = "warn"          # 放宽：生产代码允许 Reasonable unwrap，warn 提示
expect_used = "warn"
future_not_send = "deny"      # 保异步安全性
await_holding_lock = "deny"   # 推荐保留（防 UI 死锁）
pedantic = { level = "warn",  # 不开启 pedantic 全量 warn（旧项目 pedantic 全量导致噪音）
             priority = -1 }  # 仅需要时按需开子集
```

**裁决说明**（硬约束 D）：
- `unsafe_code = "deny"` 唯一强制（Rust 内存安全核心）。
- `todo`/`expect`/`unwrap` 从旧的全 deny 放宽到 **warn**（CI 用 `clippy -D warnings` 时这些 warn 会阻断——故需在 CI 命令里**豁免**这些特定 lint，保留 `-D warnings` 但允许 `#[allow]` 或局部 `-A clippy::unwrap_used` 等；或改用 `-D rust-2024` + 非 clippy 的默认 warning 当作门禁，clippy 只做建议。**本设计推荐：门禁 = `cargo test` + `cargo fmt --check` + `clippy` 仅 `-D` 核心安全类 lint，unwrap/todo/expect 用 `-A` 豁免或去重**）。
- 不开启 `pedantic` 全量 warn（避免噪音淹没真实问题）。

**客户端约定**（写进 README/CLAUDE）：
```
// 允许：unwrap 在测试 / 初始化已保证不可变的场景
// 约定：生产库代码用 thiserror/anyhow 结构化错误，避免裸 unwrap
```

### E.2 edition / MSRV

```toml
# workspace 根 Cargo.toml 片段
[workspace.package]
edition = "2021"      # 稳定，工具链兼容性好；不用 2024（部分依赖/宏需适配，克制）
rust-version = "1.85" # 对齐代码实际（reviewer 核对已自洽；主流工具链均已支持 1.85）
resolver = "2"
```

**裁决**：edition 用 `2021`（2024 有 `unsafe` 语法与部分宏适配成本，阶段 0 克制不用）；MSRV 用 `1.85`——**对齐代码实际**（`Cargo.toml` 实测 `rust-version = "1.85"`），与 D 系列一致。若后续 render 依赖（vello/wgpu）拉高 MSRV，在 D3 scaffold 时用 `rust-toolchain.toml` 实测并同步本文档。

### E.3 增量编译验收（硬约束 E：措辞改为"改数据/状态层"）

```rust
// M7/D5 增量编译验证（验收措辞修正版）：
// 修改 rgui-core::state 的 diff/snapshot 逻辑（纯数据层）→
//   cargo check -p rgui-render   # 必须不触发 render 重编译
//   cargo check -p rgui-platform # 必须不触发 platform 重编译
```

> **关键差异**：验收目标是**"改数据/状态层 → 不重编 render"**，而非旧的"改 core → 不重编 render"。因为 `rgui-core` 还含 `components`/`style` 等，这些逻辑改动**可能**触发 render 重编（render 依赖 core）；只有**纯数据层（state）**的改动才必须不触发。据此在核心内**进一步细分**：将 `state`（数据层）与 `components`（UI 层）严格分开，数据层变更才保证不触发 render 重编。**这是本设计对硬约束 E 的落地**：把数据层隔离到 `rgui-core::state` + 底层类型（id/geometry/color/view），使这些模块变更时 `rgui-render` 无需重编。

---

## F. 与旧设计差异对照（旧 12+ → 新 5）

| 维度 | 旧设计（被推翻） | 新设计（克制） | 删除项 |
|---|---|---|---|
| **crate 数** | 12+（7 个转发壳） | **5**（4 核心 + 1 可选 facade） | 合并 state/layout/style/components 入 core |
| **状态层污染** | rgui-state 依赖 rgui-render（GPU 类型进状态缓存） | core 零 GPU/零平台（Cargo 防火墙）；状态只存语义数据 | GlyphKey/PathTessellation 移出状态层 |
| **facade God Object** | app.rs 5017 行（启动+事件循环+渲染+命中+40 测试桩+拖放） | facade 纯重导出 + ≤200 行启动；职责单拆出；测试桩 cfg 隔离 | 业务逻辑与测试桩全部挪出 |
| **渲染后端** | vello + skia 双后端（版本冲突） | **单一 vello** | skia-safe/skia-backend/offscreen |
| **组件路径** | Tier 1 + Tier 2 (.rgui/.rhai) 双路径 | **唯一 Tier 1 WidgetSpec（Rust）** | .rgui/.rhai/rgui_parser/script/Rhai |
| **脚本** | Rhai 引擎 + paint 脚本 | **无脚本** | rgui-script、Rhai 依赖 |
| **热重载** | devtools watcher + style hot_reload 双套 | **P1 再议**（阶段 0 不做）或单一样式热重载 | devtools watcher/双进程 |
| **无障碍** | rgui-a11y (AccessKit 桥) 独立 crate | 删 AccessKit 桥；纯 AccessibilityTree 并入 core::a11y | AccessKit 重型依赖 |
| **开发工具** | rgui-devtools（热重载/双进程/快速重启/IPC/HTML reload） | **整体删除** | devtools 全部模块 |
| **工程文档** | 1.5 万行 D 系列（部分与代码不符） | 保留核心 D0 对齐；其余收敛/标注历史失效 | 过时/漂移文档 |

**一句话对比**：旧「12+ 转发壳 crate + God Object 门面 + 双后端/双组件/双热重载 + 状态层 GPU 污染」→ 新「5 个有实责 crate + 极薄 facade + 单渲染/单组件路径 + 核心零 GPU/平台防火墙 + 克制 lint」。

---

## G. 待总监/用户确认的落点（本设计外决策）

1. **是否保留薄 facade `rgui`**：倾向保留（提供零配置 `App::run`），也可裁掉让用户自行组装。
2. **阶段 0 是否含样式系统 `.rgss`**：建议 P1 增量（并入 core≈文本解析，先聚焦布局+状态+渲染+组件）。
3. **内置组件范围**：建议阶段 0 仅 `Accordion` + `WaBadge` 两个最小示例组件证明 Tier 1 路径。
4. **拖放**：建议阶段 0 不做（P1），聚焦核心渲染+交互主循环。

---

## 更新记录（按裁决 A：改蓝图对齐代码）

> 本次按总监裁决「方向 A（改蓝图向代码）」，将本蓝图与 D3-D10 验收通过的代码对齐，共更新 8 处：
> 1. `Color` 改用 `u8`（4 通道，可 derive Eq；sRGB 存储）——实现优于设计稿，不 revert
> 2. `PropValue` 删除 `WidgetId` 变体（视图引用用 `Key`/`NodeHandle`/`MessageBinding`）
> 3. `parse_rgss` 改为 `-> StyleSheet`（D3 占位；如需错误处理实现阶段升级 Result）
> 4. default feature 对齐代码：`core=["layout"]`、`render=[]`、`platform=["winit"]`
> 5. core 模块补齐 20 项（含 `coordinator`/`registry`/`widget_state`/`message`/`a11y_tree`）
> 6. facade 改**定向重导出**：`rgui_platform::{FocusManager, InputModality}`、`rgui_render::{GlyphKey, PathTessellation}`（防悬空 use / 防公共 API 污染）
> 7. `EventResult` 改为对齐代码：`#[derive(Debug, Clone, PartialEq, Eq)]`（与 D0/实现一致）
> 8. 其它微调同步：`Key` 改枚举 `{Str, Num}`、`WidgetId` 含 `NodeHandle/WindowId`、`Color::rgba/rgb` 构造器

> 判为「实现优于设计稿」的 3 处（原则性说明）：Color 用 u8（紧凑+可 Eq）、facade 定向重导出（防悬空+防污染）、render `default=[]`（克制不默认拉起重型 GPU 编译）。

> **P2 修正（2025-09-01，随 D 系列同步）**：① `EventResult` 改为对齐代码 `#[derive(Debug, Clone, PartialEq, Eq)]`（删"无 derive/建议补"表述；实测 rgui-core/src/traits.rs:75 含 PartialEq,Eq）；② MSRV 正文改为 `1.85`（对齐代码 `rust-version = "1.85"`，删除原 1.75 表述）。

---

## 合规确认
本设计为接口契约级文档，不含 Rust 实现代码。设计全程只读，仅更新 tools/ 本文件（按裁决 A 与代码对齐）；未改动任何现有 Rust 源文件 / Cargo.toml / .git / remote。
