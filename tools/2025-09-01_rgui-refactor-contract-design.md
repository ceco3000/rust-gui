# rgui 简洁化重构 · 接口契约级细化设计（定稿）

> ⚠️ **【历史已失效 · 已被推倒重来取代】** 本文档针对「渐进重构 old 12-crate → 6-crate」方案。**用户已决策「推倒重来（greenfield）」**，最终架构以 `tools/2025-09-01_rgui-greenfield-architecture.md` 为**唯一权威**（5-crate 收敛：core/render/platform/macros/facade）。本文保留作历史存档，**不作为实现依据**。
>
> 工作目录：`/Users/chenchao/Documents/code/rust/RUST-GUI`
> 设计方：devco-architect（方案设计师）
> 依据：`tools/2025-09-01_rgui-complexity-audit.md`（§三/§四 决策）、`tasks.md`（M0–M7）
> 适用范围：**只做设计，不写 Rust 实现代码**。本文以「接口契约」（trait 签名、模块边界、公共 API 清单、feature 划分、依赖方向）为交付形态。
> 编译基线：`cargo check --workspace` 通过。

---

## 0. 摸底关键事实修正（先行说明）

本设计执行前逐项核对了审计报告的判断，其中 **2 处与现状不符**，直接影响 R1/R2 决策，特此先行修正：

| 审计/任务判断 | 实测结论 | 影响 |
|---|---|---|
| R1: "rgui-a11y 被 app.rs **实际使用**于焦点管理" | **错误**。app.rs 的焦点管理来自 `rgui_platform::focus::{FocusManager, InputModality}`（rgui/src/app.rs:20-21, 197, 1034, 2611, 2805 等）。`rgui_a11y` 在整个仓库**仅被 facade 的 lib.rs 做 `pub use rgui_a11y::*` 重导出**；其 `backend.rs`（AccessKit 桥）需 `feature="accesskit"` 且 `rgui-a11y` 的 `accesskit` feature 未在 facade 默认开启。app.rs 中唯一出现的 `A11yCallback`（116 行）是**本地 type alias**，与 `rgui_a11y` 无运行时关联。 | R1 直接从"删除或降级"变为**无悬念的删除**，不含任何功能牵连。 |
| R2: ".rgui 声明式路径被 app.rs 大量使用" | **属实**。`rgui_parser`（parse_rgui_file / collect_widget_ids / collect_state_bindings / is_state_expr / infer_prop_value），以及 `rgui_hot_reload` / `rhai_hot_reload` / `config::HotReloadConfig`，在 app.rs 共 **30+ 处** 引用（403-417, 727-735, 771-777, 1294-1413, 1522-1530 等）。 | R2 需给"删除 devtools 后 .rgui 路径去向"一个**明确且唯一**的裁决，且该裁决必须能被 M3 收敛后保留的 crate 承接。 |

另核查：`rgui-state/src/store.rs.tmp`（40 字节，日期 6/13）为**残留临时文件**，非活跃代码（未被 mod 声明），属仓库污染清理项。

其余审计判断（crate 转发壳、facade God Object、依赖污染、verify/ 污染、多套平行机制）**全部与现状一致**，作为本设计输入。

---

## 1. 目标 crate 拓扑定稿（6–8 层）

### 1.1 目标拓扑总表

采纳"适度收敛到 6–8 层"，在审计 §四 基础上做**1 处收紧 + 1 处明确**后定稿：

| # | 目标 crate | 来源/纳入 | 依赖隔离 | 状态 |
|---|---|---|---|---|
| 1 | `rgui-core` | 现 core + **state** + **layout** + **components**（纯逻辑）+ a11y 的 `tree.rs`（见 §4 R1） | 纯 Rust，零平台/零 GPU/零 CSS-parse 依赖 | 吸收合并后的"唯一逻辑核心" |
| 2 | `rgui-render` | 现 render | wgpu/vello/cosmic-text/fontdb/skrifa 重型 GPU 隔离 | 保留独立 |
| 3 | `rgui-platform` | 现 platform | winit 重型平台隔离 | 保留独立 |
| 4 | `rgui-style` | 现 style（含 .rgss 解析） | cssparser 隔离 | **保留独立**（理由见 1.2） |
| 5 | `rgui-macros` | 现 macros | **proc-macro 硬约束：必须独立** | 保留独立 |
| 6 | `rgui` (facade) | 现 facade 瘦身 | 纯重导出 + 启动协调 | app.rs 拆分（见 §3） |

**硬性原则确认**（全部采纳，无新增矛盾）：
- `rgui-macros` 因 proc-macro 强制要求独立 crate，不可合并。
- breaking `rgui-state → rgui-render`，GPU 资源类型移入 render 层（§2）。
- 单一渲染后端：只保留 `vello-backend`，删除 `skia-backend`/`offscreen` feature 及其依赖（skia-safe 删除）。
- 组件库回归 `rgui-core`，Tier 2 脚本路径移除。

### 1.2 style 独立 vs 并入 core 的裁决

审计 §四 对 `rgui-style` 留了"若重依赖可并入 core"的口子。**裁决：保留独立 `rgui-style`**，理由：

1. `.rgss` 语法基于 CSS，依赖 `cssparser`（较重）——若并入 core，会把 CSS-parse 重依赖拖进"零平台依赖"的纯逻辑核心，污染 core 的语义边界。
2. `rgui-style` 已有 6420 行（非几百行转发壳），具备独立价值；且 `docs/D0` 将样式系统/热重载列为子系统，保留独立遵循既有设计分层。
3. 收敛的主要目标（砍转发壳、砍依赖污染、砍平行机制）已通过合并 state/layout/components、删除 script/devtools/a11y/skia、删除 component 双路径达成，不必要再细合并 style。

**代价**：`rgui-style` 依赖 `rgui-core`，且 `rgui-crate` 依赖 `rgui-style` —— 位于 render/state 与 platform 之间，仍属可接受的分层。

### 1.3 模块边界（每个目标 crate 的 `pub mod` 清单）

**A. `rgui-core`（逻辑核心，被收敛为子模块）**

核心 crate 内部按**领域子模块**组织（对应 D0 分层），`lib.rs` 顶层重新导出：

```
rgui-core/
├── src/
│   ├── lib.rs                 ← 收敛后重新组织顶层 pub mod
│   ├── id.rs                  (原状态，保留)
│   ├── geometry.rs            (原状态，保留)
│   ├── color.rs               (原状态，保留)
│   ├── traits.rs              (AppMessage / WidgetSpec trait 相关，保留)
│   ├── context.rs             (View 上下文，保留)
│   ├── widget_state.rs        (保留)
│   ├── registry.rs            (保留)
│   ├── message.rs             (保留)
│   ├── a11y.rs                (保留——组件级无障碍类型/角色定义，见 §4)
│   ├── locale.rs              (保留)
│   ├── view.rs                (WidgetView / Color / Key / PropValue——声明式视图类型保留，见 §4)
│   ├── coordinator.rs         (保留)
│   │
│   ├── layout/                ← 由 rgui-layout 迁入（拆掉独立 crate）
│   │   ├── mod.rs             (CachedLayout / LayoutEngine / LayoutNode / LayoutResult)
│   │   └── mapping.rs         (to_taffy_style)
│   │
│   ├── state/                 ← 由 rgui-state 迁入（拆掉独立 crate，依赖 M1 解耦后）
│   │   ├── mod.rs             (StateStore / InstanceState / StoreBinding / Subscription)
│   │   ├── diff.rs            (Patch / apply_patch / diff / diff_props)
│   │   ├── snapshot.rs        (Snapshot / Snapshotter / SchemaMigration)
│   │   ├── harness.rs         (TestHarness)
│   │   └── store.rs           (StateStore 实现——**不再持有 GPU 类型**)
│   │
│   ├── components/            ← 由 rgui-components 迁入（Tier 1 WidgetSpec）
│   │   ├── mod.rs
│   │   ├── accordion.rs       (WidgetSpec 实现，迁入)
│   │   └── wa_badge.rs        (WidgetSpec 实现，迁入)
│   │
│   └── a11y_tree/             ← 由 rgui-a11y/tree.rs 迁入（见 §4 R1）
│       └── mod.rs             (AccessibilityTree)
```

**B. `rgui-render`（保留，重型 GPU 隔离）**

```
rgui-render/
├── src/
│   ├── lib.rs                 (pub mod + 顶层重新导出)
│   ├── glyph.rs               (GlyphKey / GlyphCacheEntry / GlyphAtlas / RasterizedGlyph)——留在 render
│   ├── path_tessellation.rs   (PathTessellation)——留在 render
│   ├── text.rs                (TextShaper / 字形整形)
│   └── scene_graph.rs + vello.rs (仅 vello 后端；删除 skia.rs)
```

→ **GPU 资源类型（GlyphKey/PathTessellation/GlyphCacheEntry）已原生位于 render crate**，无需新建。本设计的**新增点**：把这些类型**只从 render 导出**，并使 `rgui-core` 不再声明对 `rgui-render` 的依赖（见 §2）。

**C. `rgui-platform`（保留，重型平台隔离）**

```
rgui-platform/
├── src/
│   ├── lib.rs
│   ├── focus.rs               (FocusManager / InputModality——焦点管理原生在此，见 §4 R1)
│   ├── window.rs
│   ├── input.rs
│   └── ime.rs
```

**D. `rgui-style`（保留独立，cssparser 隔离，含 .rgss 解析与热重载）**

```
rgui-style/
├── src/
│   ├── lib.rs
│   ├── parser.rs              (.rgss 解析, cssparser)
│   ├── theme.rs
│   └── hot_reload.rs
```

> 注：审计"删除 devtools watcher + style hot_reload 两套热重载"——本设计**保留 `rgui-style` 的热重载**（它跟 CSS 主题就近，属样式域能力），**删除 `rgui-devtools` 的 watcher/双进程热重载**（开发期框架级工具，不在内核）。热重载最终只走 style 一条路径（见 §5 收敛项）。

**E. `rgui-macros`（保留独立，proc-macro）**

```
rgui-macros/
├── src/
│   ├── lib.rs
│   ├── widget_spec.rs         (#[derive(WidgetSpec)])
│   ├── app_message.rs         (#[derive(AppMessage)])
│   ├── persist_state.rs       (#[derive(PersistState)])
│   └── html.rs                (html! 宏——见 §4 R2 / §5 收敛裁决)
```

**F. `rgui`（facade，瘦身）**

```
rgui/
├── src/
│   ├── lib.rs                 (纯重导出收敛)
│   ├── app.rs                 (AppConfig + App 启动协调——拆分后 ≤ ~800 行)
│   ├── event_loop.rs          (新，自 app.rs 拆分)
│   ├── render_coord.rs        (新，自 app.rs 拆分)
│   ├── interaction.rs         (新，自 app.rs 拆分——InteractionRegion/ResolvedHitTest)
│   ├── automation.rs          (新，自 app.rs 拆分——InteractionAutomationHarness 测试桩)
│   ├── props_sync.rs          (新，自 app.rs 拆分——state/属性注入/递归同步)
│   ├── accordion.rs           (Tier 1 组件实现——与 rgui-core::components 协调归属)
│   ├── widget_node.rs         (保留)
│   ├── paint_factory.rs       (保留)
│   ├── render.rs              (保留)
│   ├── interactive.rs         (保留)
│   ├── logging.rs             (保留)
│   └── error_boundary.rs      (保留)
```

### 1.4 公共 API 清单（每个目标 crate 的 `pub use` / 顶层导出）

**`rgui-core`（converged）顶层导出（lib.rs）**：

```
pub mod id, geometry, color, traits, context, widget_state, registry,
          message, a11y, locale, view, coordinator,
          layout, state, components, a11y_tree;

// 顶层重导出（保持 facade 原有 `use rgui_core::*` 兼容性）
pub use id::*;
pub use geometry::*;
pub use color::*;
pub use traits::*;           // AppMessage, WidgetSpec trait
pub use context::*;
pub use widget_state::*;
pub use registry::*;
pub use message::*;
pub use a11y::*;             // AccessibilityAction, AccessibilityRole
pub use locale::*;
pub use view::*;             // WidgetView, Color, Key, PropValue
pub use coordinator::*;
pub use layout::*;           // CachedLayout, LayoutEngine, LayoutNode, LayoutResult  ← 由 rgui-layout 迁入
pub use state::*;            // StateStore, InstanceState, Subscription, Patch, Snapshot  ← 由 rgui-state 迁入
pub use components::*;       // Accordion, WaBadge 等 WidgetSpec 组件  ← 由 rgui-components 迁入
pub use a11y_tree::*;        // AccessibilityTree  ← 由 rgui-a11y 迁入

// 关键类型再导出（显式列出，作为契约）
pub type StateStore<M> = state::StateStore<M>;
pub struct GlyphKey;          // ❌ 不在 core——见 §2，此处不导出
```

**`rgui-render`（收敛后）顶层导出（lib.rs）**：

```
pub mod glyph, path_tessellation, text, scene_graph, vello;

pub use glyph::{GlyphKey, GlyphCacheEntry, GlyphAtlas, RasterizedGlyph};
pub use path_tessellation::PathTessellation;
pub use text::TextShaper;
pub use scene_graph::SceneGraph;
// vello 后端：pub use vello::VelloBackend;
// 删除 skia:: 与 skia-backend / offscreen feature 相关导出
```

**`rgui-platform`（保留）顶层导出（lib.rs）**：

```
pub mod focus, window, input, ime;
pub use focus::{FocusManager, InputModality};
// ...其余保留
```

**`rgui-style`（保留）顶层导出（lib.rs）**：

```
pub mod parser, theme, hot_reload;
pub use parser::*;      // StyleSheet / parse_rgss
pub use theme::*;
pub use hot_reload::*;
```

**`rgui-macros`（保留）顶层导出（lib.rs）**：

```
pub use widget_spec::WidgetSpec;   // derive
pub use app_message::AppMessage;   // derive
pub use persist_state::PersistState; // derive
pub use html::html;                 // 视 §4 R2 裁决保留或移除
```

**`rgui`（facade，瘦身）顶层导出（lib.rs）**：

```
pub mod app, event_loop, render_coord, interaction, automation,
          props_sync, widget_node, paint_factory, render, interactive,
          logging, error_boundary, accordion;

pub use app::{App, AppConfig};
// 删除 [cfg(feature="devtools")] pub use app::run_simple_app;  ← devtools 已删
// 删除 pub use rgui_a11y::*;  ← a11y 已删（见 §4）
// 删除 pub use rgui_components::*;  ← components 已并入 core
// 删除 pub use rgui_layout::*;  ← layout 已并入 core
// 删除 pub use rgui_screen::*;  ← （如有）script 已删
pub use rgui_core::*;
pub use rgui_render::*;
pub use rgui_platform::*;
pub use rgui_style::*;
pub use rgui_macros::{AppMessage, WidgetSpec, html, PersistState};
pub use graph::GraphWidget;  // ← 若有，从 render 或 core 重导出，确保使用方（cargo check -p rgui）不破
```

### 1.5 feature 划分（目标 crate 的 `[features]`）

**关键 feature 收敛原则**：删除所有"开发期增强"/未启动的后端 feature；只保留"按重型依赖隔离"所需的 feature。

| Crate | 目标 feature | 说明 |
|---|---|---|
| `rgui-core` | `default = ["std"]`；`std`（可 no_std 预留，但阶段 0 不做） | 零平台依赖，无大型可选 feature |
| `rgui-render` | `default = ["vello-backend"]`；`vello-backend`（含 wgpu/vello/cosmic-text/fontdb/skrifa 等）| **删除 `skia-backend`、`offscreen`**（skia-safe、skia 相关依赖均删）|
| `rgui-platform` | `default = []`；按平台可选 feature（winit 依赖选择性开启） | 保留 |
| `rgui-style` | `default = ["cssparser"]`；`hot-reload` | 保留热重载 feature；**移除 `devtools` 相关热重载依赖**（删 notify?——style 热重载如依赖 notify 需确认，见 §5）|
| `rgui-macros` | `default = []` | 无大型可选 feature |
| `rgui` (facade) | `default = []`；可选 `test-harness`（含自动化桩） | **删除 `devtools`/`script`/`a11y` feature 及其可选依赖**；`InteractionAutomationHarness` 挪到 `#[cfg(feature="test-harness")]` 或 `#[cfg(test)]`（见 §3.2）|

**workspace 根 `Cargo.toml` 收敛**：
- `members` 收敛为：`rgui-core`、`rgui-render`、`rgui-platform`、`rgui-style`、`rgui-macros`、`rgui`（+ 可选保留的 examples/`one-accordion` 或迁移为 Rust 示例）。
- **移除**：`rgui-state`、`rgui-layout`、`rgui-components`、`rgui-script`、`rgui-devtools`、`rgui-a11y`、`rgui-template`（若判为示例）及 verify/ 全部验证期 crate。
- **`workspace.dependencies` 清理**：删除 `rgui-state`/`rgui-layout`/`rgui-components`/`rgui-script`/`rgui-devtools`/`rgui-a11y` 的 path 依赖条目；删除 `skia-safe`（及其 version 冲突注记）。

---

## 2. `rgui-state → rgui-render` 解耦方案（M1 核心）

### 2.1 问题本质

`rgui-core`（现阶段）不直接依赖 render，但 `rgui-state/src/store.rs` 的 `RenderLayoutCache`（63-75 行）持有：

```rust
// 现状（state/src/store.rs）
pub(crate) struct RenderLayoutCache {
    pub layout: Option<rgui_layout::LayoutResult>,              // ← 状态层引用布局类型
    pub glyph_cache: FxHashMap<rgui_render::GlyphKey, rgui_render::GlyphCacheEntry>,  // ← 状态层引用 GPU 类型
    pub path_tessellation: Option<rgui_render::PathTessellation>, // ← 状态层引用 GPU 类型
    pub last_paint_color: Option<rgui_core::Color>,
}
```

这导致 `rgui-state → rgui-render` + `rgui-state → rgui-layout`。**架构污染核心**：状态管理（纯快照/差分）不应知道 GPU 字形缓存与布局结果。

### 2.2 解耦目标

让 `rgui-state`（及其"迁入"版的 `rgui-core::state` 子模块）满足：
- **零依赖 GPU 类型**：不再出现 `GlyphKey` / `PathTessellation` / `GlyphCacheEntry` / `LayoutResult`。
- **零依赖 `rgui-layout`**：不出现 `rgui_layout::` / `LayoutResult`。
- **"改状态逻辑不重编渲染引擎"**：改 `core::state` 的 diff/snapshot 时，`rgui-render` 不触发重编译。

### 2.3 处理决策（两选一，本设计裁决）

审计的方向 2 是"把 GPU 资源类型移入 render 层，state 只保留纯逻辑"。但实测发现：`GlyphKey`/`PathTessellation`/`GlyphCacheEntry` **本来就在 rgui-render/src/glyph.rs / path_tessellation.rs，且已被 render 导出**。真正的**污染源不在类型定义位置，而在"state 层持有这些类型的缓存实例"**这一架构决定。

因此给出**两条可行路径**并裁决：

- **方案 A（推荐·纯拆缓存）**：把 `RenderLayoutCache` 从 `rgui-state` 整体**移出**，重定义为渲染层专用组件。状态层不再持有任何字形/路径/布局缓存；这些缓存改由渲染/布局子系统（`rgui-render` 或 `rgui-core::layout`）持有。
- **方案 B（收窄类型）**：保留缓存结构，但把 `layout`/`glyph_cache`/`path_tessellation` 字段换成**纯数据描述**（如 `Option<Size>`、`Vec<(GlyphId, u32)>` 等平台无关类型），GPU 类型全部收敛到 render 侧映射。

**裁决：方案 A**。理由：
1. 方案 A 让状态层回到"D2 定义的纯状态/差分/快照"本源，语义最干净，且**彻底**消除依赖（不只是类型收窄）。
2. `RenderLayoutCache` 的生命周期是"组件挂载→卸载"（store.rs:60 注释），本质上是 **渲染缓存**，归渲染子系统管理更符合单一职责（SRP）。
3. 方案 B 需要定义一套"平台无关的字形映射类型"，增加一套需要长期维护的平行类型——违背"砍平行机制"收敛原则。

### 2.4 具体契约（解耦后的类型归属与替换）

**类型最终归属（定稿）**：

| 类型 | 原位置 | **契约位置（新）** | 说明 |
|---|---|---|---|
| `GlyphKey` | rgui-render/src/glyph.rs | **`rgui-render` 根治 + 导出** | 无需移动，仅确保完整导出 |
| `GlyphCacheEntry` | rgui-render/src/glyph.rs | **`rgui-render` 导出** | 同上 |
| `GlyphAtlas` | rgui-render/src/glyph.rs | **`rgui-render` 导出** | 同上 |
| `RasterizedGlyph` | rgui-render/src/text.rs | **`rgui-render` 导出** | 同上 |
| `PathTessellation` | rgui-render/src/path_tessellation.rs | **`rgui-render` 导出** | 同上 |
| `LayoutResult` | rgui-layout/src/engine.rs | **`rgui-core::layout`（由 rgui-layout 并入）** | 因为 layout 是纯逻辑，并入 core；但 **state 不持有它** |
| `LayoutEngine`/`LayoutNode`/`CachedLayout` | rgui-layout/src/engine.rs | **`rgui-core::layout`** | 逻辑层 |
| `to_taffy_style` | rgui-layout/src/mapping.rs | **`rgui-core::layout`** | 逻辑层（依赖 taffy——见下） |

**⚠️ taffy 依赖归属裁决**：`rgui-layout` 依赖 `taffy`。布局并入 core 后，`rgui-core` 会依赖 `taffy`。taffy 是纯 Rust 布局引擎（无平台/GPU），**可接受**作为 core 的依赖——不与"零平台依赖"冲突。因此 layout 并入 core 是干净的。（若坚持 core 零第三方依赖，则 layout 保留独立或并入 render；本设计裁决**并入核心**，因 taffy 是纯逻辑、且能显著减少转发壳。）

**`RenderLayoutCache` 的去向（定稿）**：

方案 A 下，`RenderLayoutCache` **从 `rgui-state` 移到 `rgui-render`**（它是渲染缓存）：

```rust
// 新位置：rgui-render/src/cache.rs
// 渲染专用缓存——每挂载组件一份，生命周期=组件挂载→卸载
#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct RenderLayoutCache {
    pub layout: Option<rgui_core::layout::LayoutResult>,          // 引用 core::layout（纯逻辑）
    pub glyph_cache: FxHashMap<GlyphKey, GlyphCacheEntry>,        // 引用 render 自身类型——从 state 剥离后此依赖已合法
    pub path_tessellation: Option<PathTessellation>,              // 引用 render 自身类型
    pub last_paint_color: Option<rgui_core::Color>,
}
```

> 移动后 `RenderLayoutCache` 同时引用 `rgui-core::layout::LayoutResult` 与 render 自身类型。因它在 render crate 内，对 render 类型无"横向依赖"问题，对 core 是向下依赖（合法）。**状态层彻底不再持有它。**

**状态层原引用点的替换（逐点）**：

| 引用点（state/src/store.rs） | 原代码 | 处理（契约级） |
|---|---|---|
| L65 `pub layout: Option<rgui_layout::LayoutResult>` | 引用 layout | **从 store.rs 移除**；该字段/缓存整体迁至 render/src/cache.rs |
| L68 `glyph_cache: FxHashMap<rgui_render::GlyphKey, ...>` | 引用 render GPU | **移除**；迁至 render/src/cache.rs |
| L71 `path_tessellation: Option<rgui_render::PathTessellation>` | 引用 render GPU | **移除**；迁至 render/src/cache.rs |
| L1156-1166 测试 `render_layout_cache_path_tessellation_field` | 构造 PathTessellation | **随缓存迁移**至 render/src/cache.rs（或删除，因缓存类型已不属 state） |
| L1168-1182 测试 `render_layout_cache_layout_field` | 构造 LayoutResult | **随缓存迁移**至 render/src/cache.rs（或删除） |
| L1184+ 测试 `render_layout_cache_last_paint_color_field` | 构造 Color | **随缓存迁移**或保留（依赖 core 类型，合法） |
| L65 所在结构体整体 | `RenderLayoutCache` | 从 `state` 中**删除** |

**`rgui-state/Cargo.toml` 收敛**（或在并入 core 后，`rgui-core` 的 `state` 子模块 Cargo 层面不声明 render/layout）：

```
# rgui-state → 并入 core 前，先移除：
rgui-render.workspace = false   # 删除依赖
rgui-layout.workspace = false   # 删除依赖
# 保留：
rgui-core.workspace = true
```

**若走独立 crate 保留（备选）**：`rgui-state` 若因迁移保守而保留独立，则其 Cargo.toml 移除 `rgui-render`/`rgui-layout`，且 `store.rs` 中删除 `RenderLayoutCache` 全部字段与测试。

### 2.5 增量编译收益验证点（对接 M7）

解耦后必须验证（M7 最后门禁）：
- 改 `rgui-core`（含 `state`/`layout` 子模块）内任一函数 → `cargo check -p rgui-render` **不触发重编译**。这是本解耦的核心验收点。
- `cargo test -p rgui-core`（原 state 测试）全绿。

---

## 3. facade 瘦身 & `app.rs` 拆分（M4 核心）

### 3.1 拆分模块清单（职责 & 依赖）

`rgui/src/app.rs`（现 5017 行、200 顶层项、混杂职责）拆分为以下模块。**每模块单一职责，无 200 项巨型 impl，无循环依赖。**

| # | 拆分模块 | 职责（单一） | 关键类型/项 | 依赖（→ 用于防循环） |
|---|---|---|---|---|
| 1 | `app.rs` | **启动协调 / AppConfig / App 骨架**。仅存配置、生命周期编排、对外接口 `App`/`AppConfig`。| `App`、`AppConfig`、`App::new`/`run`、启动器 | 依赖其他模块的**类型**（不 import 其自由函数），单向向下 |
| 2 | `event_loop.rs` | **winit 事件循环 + 窗口创建/尺寸/关闭**。| `EventLoop` 编排、`window_create`、事件分发骨架 | 依赖 `render_coord`、`interaction`；**不**依赖 `automation`/`props_sync` |
| 3 | `render_coord.rs` | **wgpu 渲染协调**——单帧绘制、scene graph 提交、重绘脏标记。| `RenderCoord`、`paint/scene` 提交逻辑 | 依赖 `rgui-render`、`app::App` 状态 |
| 4 | `interaction.rs` | **交互命中测试**——`InteractionRegion`/`ResolvedHitTest`/`hit_test_*`。| `InteractionRegion`、`hit_test_*`、hover/click 命中 | 依赖 `rgui-core`、`app::App`；**不**依赖 `automation` |
| 5 | `automation.rs` | **测试自动化桩**——`InteractionAutomationHarness`（约 40 个 inject_/replay_ 方法）。| `InteractionAutomationHarness` | `#[cfg(feature="test-harness")]` 或 `#[cfg(test)]` 隔离；依赖 `interaction`、`app` |
| 6 | `props_sync.rs` | **状态注入/属性注入/递归同步**——`inject_state_bindings_*`/`sync_store_to_props_*`/`resolve_single_mode_conflicts_*`/`inject_props_*`。| 状态绑定注入、props 同步、单冲突解决 | 依赖 `rgui-core::state`、`app::App` |

**拆分后职责归属对照**（app.rs 原文 → 目标模块）：

| app.rs 原文（约） | 拆分模块 |
|---|---|
| L125-176 `AppConfig` 定义/默认/builder | `app.rs` |
| L155-263 `AppConfig` builder（`.set_*`） | `app.rs` |
| L264-334 `inject_props_from_registry`/`inject_props_recursive` | `props_sync.rs` |
| L324-512 `inject_state_bindings(_recursive)` | `props_sync.rs` |
| L513-541 `sync_store_to_props` | `props_sync.rs` |
| L542-653 `resolve_single_mode_conflicts(_recursive)` | `props_sync.rs` |
| L654-706 `sync_store_to_props_recursive` | `props_sync.rs` |
| L707-1086 `InteractionAutomationHarness`（40 方法） | `automation.rs`（cfg 隔离） |
| L1087+ `InteractionHost for App` | `interaction.rs` |
| L53-124 `InteractionRegion`/`ResolvedHitTest` | `interaction.rs` |
| 事件循环/窗口/winit 相关 | `event_loop.rs` |
| wgpu 渲染/绘制/提交 | `render_coord.rs` |
| App 骨架/生命周期/对外接口 | `app.rs` |

### 3.2 防循环依赖规则（硬约束）

1. **依赖方向单向**：`app.rs`（编排层）可 import 其他模块类型；但 `event_loop`/`render_coord`/`interaction`/`props_sync` **不得反向 import 编排层的自由函数**，只能依赖 `App`/`AppConfig` 等类型。
2. **`automation.rs` 必须隔离**：`InteractionAutomationHarness`（测试桩）移到 `#[cfg(feature="test-harness")]` 或 `#[cfg(test)]`，**不进入生产 app.rs 公共路径**。审计明确其约 40 个 inject_/replay_ 是测试设施。
3. **每模块单一职责，无 200 项巨型 impl**：拆分后各 impl 块 ≤ 数十方法，聚焦单一领域。
4. **`app.rs` 行数目标**：≤ ~800 行（审计 M4 验收标准）。

### 3.3 依赖方向图（最终分层，无环）

```
              rgui (facade)  ── 编排层
                 │  (单向向下)
     ┌───────────┼───────────────┐
     │           │               │
  app.rs      event_loop     render_coord
     │           │               │
     ├───────────┴───────────────┘
     │
  interaction.rs
     │
  props_sync.rs        automation.rs (cfg 隔离 → 测试)
     │                       │
     └───────────────────────┘
           （全部仅依赖 rgui-core / rgui-render / rgui-platform，
             且 inter-module 依赖不形成环）
```

**验证点**：任一拆分模块 `cargo check` 通过；`cargo clippy -- -D warnings` 无 cycle/未用警告。

---

## 4. 两个关键风险决策（R1 / R2）

### 4.1 R1：rgui-a11y —— 删除还是降级为内部模块？

**实测结论（§0 修正）**：`rgui-a11y` 未参与 app.rs 焦点管理（焦点管理在 `rgui_platform::focus`），仓库内部对它的唯一引用是 facade 的 `pub use rgui_a11y::*` 重导出（+ `rgui-core/src/a11y.rs` 里一句"由 rgui-a11y 负责平台桥接"的**注释**）。其 `backend.rs`（AccessKit 桥）需要 `feature="accesskit"` 且未默认启用。ap.rs 里的 `A11yCallback` 是本地别名，与 rgui-a11y 无关。

**决策：删除独立 crate `rgui-a11y`，但其 `tree.rs`（`AccessibilityTree`）降级为 `rgui-core` 内部模块（`a11y_tree`），保留焦点管理在 `rgui-platform::focus` 原样不动。**

理由：
1. **无功能牵连**：删除 `rgui-a11y` 不影响 app.rs 焦点管理（焦点管理在 platform）。
2. **`AccessibilityTree` 有保留价值**：`tree.rs` 定义了 `AccessibilityTree`（无障碍树），是 `rgui-core::a11y`（角色/动作类型）的消费方，属于框架"无障碍能力"的薄层。若整体删除会丢失无障碍树类型；降级为内部模块成本低、不引入 AccessKit 重依赖。
3. **弃用 AccessKit 桥**：`backend.rs`（AccessKit 桥，需 feature）**删除**——AccessKit 是重型跨平台无障碍依赖，当前无平台使用方，符合"砍重型平行机制"原则。只保留纯 Rust 的 `AccessibilityTree`。

**契约落点**：
- 删除 `rgui-a11y` crate（目录 + workspace member + facade 依赖 + `pub use rgui_a11y::*`）。
- `rgui-a11y/src/tree.rs` → 迁入 `rgui-core/src/a11y_tree/mod.rs`，顶层导出 `AccessibilityTree`。
- `rgui-platform::focus::{FocusManager, InputModality}` 保留，不在本任务改动。
- app.rs 内 `A11yCallback` 本地别名保留（与仓库无关，属 app 自身 API）。

### 4.2 R2：rgui_parser / .rgui 声明式路径 —— 内联保留还是废弃？

**实测确认**：`rgui_parser`（parse_rgui_file / collect_widget_ids / collect_state_bindings / is_state_expr / infer_prop_value）+ `rgui_hot_reload` + `rhai_hot_reload` + `config::HotReloadConfig` 在 app.rs 有 **30+ 处**引用（403-417、727-735、771-777、1294-1413、1522-1530 等），覆盖：`.rgui` 视图加载、`.rgui`/`.rhai` 声明式应用运行、widget 自动注册交互、hot-reload 监控。

**决策：废弃声明式路径（Tier 2），删除 `rgui_parser`/`.rgui`/`.rhai` 全部链路；统一走 Tier 1 `WidgetSpec`（Rust 原生实现）。**

理由：
1. 审计 §四 决策 3 已明确："统一 Tier 1 WidgetSpec（Rust 实现）；Tier 2 声明式（.rgui/.rhai）**废弃**。"——本设计**完全遵循**已拍板决策，不另开内联保留口子。
2. 两个示例 `examples/one-accordion`、`examples/one-wa-badge` 依赖 `.rgui`/`.rhai`，需**迁移为 Rust WidgetSpec 实现**（或删除并新增 Rust 示例），否则无法践行"唯一路径"。
3. 保留 `rgui_parser` 意味着保留一套"声明式 DSL 解析 + Rhai 脚本 + hot-reload 监控"完整机制——正是审计要砍的 over-engineering。内联保留违背收敛目标。

**契约落点**：
- 删除 `rgui-devtools` 中 `rgui_parser.rs`、`rgui_hot_reload.rs`、`rhai_hot_reload.rs`、`html_hot_reload.rs`、`watcher.rs`、`config.rs`、`ipc.rs`、`fast_restart.rs`、`error.rs`（即 devtools crate 整体删除）。
- **app.rs 去 `.rgui` 依赖**：
  - L134/L173/L176 `AppConfig.rgui_path` → 删除该字段与 builder。
  - L354-358 `is_state_expr`/`infer_prop_value` → 删除相关注入逻辑（.rgui 状态表达式注入）。
  - L380-417 `run_simple_app`/声明式应用运行 → 删除（该函数是 `#[cfg(feature="devtools")]` 导出，devtools 删除后一并移除）。
  - L403/727/1297 `parse_rgui_file`/`collect_widget_ids`/`collect_state_bindings` → 删除相关调用。
  - L1294-1413 `RguiHotReload` 监控 → 删除。
  - L1522-1530 `RhaiHotReload` 监控 → 删除。
- **`Accordion`/`WaBadge` 组件**：迁移为纯 Tier 1 `WidgetSpec` Rust 实现（见 §1.3 `rgui-core::components`）。`accordion.rhai`/`accordion.rgui`/`accordionitem.rhai`/`accordionitem.rgui`/`wa_badge` 相关声明式脚本删除。
- **`html!` 宏**：审计未明确废弃 `html!`（它是 Tier 1 侧的构建宏，与声明式 DSL 不同层级）。**裁决：`html!` 保留**（属 Rust 原生构建 DSL，服务 Tier 1 WidgetSpec），但需在 M3 后确认其不再经 `rgui-devtools` 间接使用。若 `html!` 实际生成 `.rgui` 中间表示，则一并废弃——需 M3 执行时核实 `html!` 产物类别后定，本设计**暂判保留**（Rust 宏构建 WidgetSpec 树，属值得保留的 Tier 1 工具）。

---

## 5. 收敛项汇总（跨 M0-M7 的一致性契约）

本设计除上述三大块（拓扑/解耦/facade/R1/R2）外，同步给出**其余平行机制的收敛契约**（供 M2/M3/M5 执行遵循）：

| 收敛项 | 现状（平行机制） | 收敛裁决 |
|---|---|---|
| 渲染后端 | vello + skia 双后端抽象 | **单一 vello**；删除 `skia-backend` feature、skia-safe 依赖、`verify/av2-skia`（临时移除注释项彻底清） |
| 组件定义 | Tier 1 WidgetSpec + Tier 2 (.rgui/.rhai) | **唯一 Tier 1**；.rgui/.rhai 废弃 |
| 组件实现处 | Accordion 在 rgui crate；WaBadge 在 rgui-components | **统一归 `rgui-core::components`** |
| 宏 | `html!` + derives | 保留 derives；`html!` 保留（见 R2 裁决），M3 核实产物 |
| 脚本 | Rhai 引擎 + paint 脚本两套 | **脚本全删**（rgui-script 删除，Rhai 依赖移除） |
| 热重载 | devtools watcher + style hot_reload 两套 | **仅 style hot_reload**；devtools watcher 删除；style 如依赖 notify 需确认保留（它是样式域能力） |
| a11y | rgui-a11y (AccessKit) 独立 crate | **降级为 core::a11y_tree**，删 AccessKit 桥 |
| 仓库污染 | verify/ 12 crate、{{pkgetc}}/、未跟踪报告、store.rs.tmp | 清理：verify/ 移出 workspace；{{pkgetc}}/ 删除；报告移入 tools/；store.rs.tmp 删除 |
| 焦点管理 | rgui_platform::focus | **保留原样**（不存在审计担心的问题） |

---

## 6. 交付前验收清单（设计侧确认）

按 tasks.md 各里程碑整理出**设计侧可由我确认**的验收点：

- [ ] M3：workspace `members` 仅剩 6 个 crate：core/render/platform/style/macros/facade（+ 可选 examples）。
- [ ] M3：`rgui` facade `cargo check -p rgui` 通过且公共 API 完整（无丢失 `use rgui_core::*` 之类会破坏使用方的项）。
- [ ] M1：`rgui-core`（state/layout 子模块）无 `rgui_render::`/`rgui_layout::` 引用；`RenderLayoutCache` 迁至 render。
- [ ] M1：改 `rgui-core` → `cargo check -p rgui-render` 不触发重编译。
- [ ] M2：`rgui-script`/`rgui-devtools`/`rgui-a11y` 目录删除（或 a11y 已降级为 core 子模块）。
- [ ] M2：app.rs 中 `cfg(feature="devtools")` 分支与 `.rgui`/`.rhai` 引用清零。
- [ ] M4：app.rs ≤ 800 行；`InteractionAutomationHarness` 已 cfg 隔离。
- [ ] M5：Accordion/WaBadge 为纯 WidgetSpec；`.rgui`/`.rhai` 示例迁移或删除。
- [ ] M7：`cargo check --workspace` + `cargo test` + `cargo clippy -- -D warnings` + `cargo fmt -- --check` 全绿。

---

## 7. Design Notes（遗留待执行期核实项）

以下项在**执行阶段**需核实，本设计给出倾向但不阻断：

1. **`html!` 宏产物类别**：若其生成的是 WidgetSpec/Rust 树 → 保留；若生成 `.rgui` 中间表示 → 废弃。执行 M3 时以 `rgui-macros/src/html.rs` 源码为准。
2. **`rgui-style` 热重载是否依赖 notify**：style 的 `hot_reload` 若依赖 `notify`，在删除 devtools 后需确认其依赖来源是否独立（不应经 devtools）。
3. **`rgui` 内 `render.rs`/`paint_factory.rs`/`interactive.rs`**：这些 facade 内模块是否也间接依赖 `.rgui`（`interactive.rs` 注释提到"扫描 .rgui 节点 onclick"）——需在 M3/M5 时核实是否一并清理。
4. **示例去处**：`examples/one-accordion`、`examples/one-wa-badge` 迁移为 Rust WidgetSpec 示例，或新建单一 Rust 示例以证明 Tier 1 路径。

---

## 8. 待总监/用户确认的开放项（本设计落点外的决策）

1. **`rgui-template`（45 行）归属**：审计标注"疑似主入口模板"。建议**初判为示例/脚手架**，不纳入框架交付物（移入 examples/ 或删除）。
2. **`verify/` 12 个验证期 crate 去向**：审计 R4。建议**移出 workspace（不再 member）+ 移入独立 `verify/` workspace 或独立仓库**，不删除（保留验证资产），但不参与主 workspace 编译与发布。@devco-director 决策执行。

---

> 本设计为接口契约级文档，不含 Rust 实现代码。所有文件写操作仅在 `tools/` 下（本设计文档），**未改动任何现有 Rust 源文件 / Cargo.toml**。
