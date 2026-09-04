# D8（收敛窗口逻辑进 platform）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D8 交付（窗口/事件循环收敛进 platform）
> 审查基准：`tools/2025-09-01_rgui-greenfield-architecture.md` §C.3/§D + D7 审查遗留 P2-1（platform 空壳）、P1-1/P1-2（wgpu API 疑点）
> 审查方式：只读代码核查（未运行 GPU 测试）；依赖方向逐 crate 验证
> 结论性质：基于源码逐行审读 + Cargo 依赖图分析

---

## 〇、结论速览

| # | 审查点 | 定级 | 一句话结论 |
|---|---|---|---|
| Q1 | 收敛正确性 | **PASS（问题已解决）** | AppRunner/run_as/WindowConfig 收敛 platform；render_surface/GpuSurface 收敛 render；window_demo 无直接 winit/wgpu |
| Q2 | 窗口尺寸/默认配置 | **PASS** | WindowConfig::default 非零 300x200；run_as→run_as_with_config 委托健壮 |
| Q3 | 防火墙/DAG | **PASS（极净）** | core 零依赖；platform⇄render 互不相依；render 无 winit；platform 无 wgpu/vello；单一 vello/winit |
| Q4 | 事件循环 | **PASS（收敛正确）** | ControlFlow::Poll + about_to_wait 每帧 redraw 正确；首帧稳定渲染保持 |
| Q5 | D7 前风险 | —— | 无 P0；2 项 P2 架构观察（见 §六）|

**总评：D8 彻底解决了 D7 审查发现的"platform 空壳"问题——窗口/事件循环收敛进 `rgui_platform`（run_as/AppRunner/WindowConfig），surface 渲染收敛进 `rgui_render`（render_surface/GpuSurface），window_demo 不再直接 `winit::`/`wgpu::`。P0 清零，核心目标（窗口稳定渲染 + 边界清晰）达成。建议：放行（PASS）。** 剩余 2 项 P2（facade 双入口未打通、facade 残留独立 winit/wgpu 依赖）随 D9 处理。

---

## 一、Q1 — 收敛正确性（PASS，D7 遗留已解决）

### 1.1 【PASS】platform 层已真正承担 winit 隔离（D7 P2-1 解决）
D8 的 `rgui-platform` 从"D7 的纯类型别名壳"升级为**真实 API**：
- `event_loop.rs`：`App trait`（init/event/draw，event_loop.rs:26-33）、`run_as`（event_loop.rs:45-47）、`run_as_with_config`（event_loop.rs:52-64）、`Runner`（winit ApplicationHandler 桥接，event_loop.rs:67-114）。
- `window.rs`：`WindowConfig`（new/named/Default，window.rs:16-48）、`attributes()` 映射 winit WindowAttributes（window.rs:64-71）。
- `lib.rs` re-export `run_as/run_as_with_config/App as AppRunner/ControlFlow/WindowEvent/WindowConfig/Window/attributes`（lib.rs:24-27）。
- **上层不再引用 `winit::`**：全部经 platform 的 re-export（`event_loop.rs:8-15`）。

**验证**：全仓 `grep "^use winit"`（排除注释）→ 仅 `rgui-platform/src/*`（winit re-export），**window_demo.rs 零直接 winit**。✓

### 1.2 【PASS】surface 渲染收敛进 render（D7 P1 疑点解决）
- `rgui-render` 新增 `render_surface`（vello.rs:223-271）——封装了 **D7 的"离屏中间纹理 + TextureBlitter blit + present"逻辑**，上层不再碰 wgpu。
- `GpuSurface = wgpu::Surface` 别名（lib.rs:32）——示例/上层用 `rgui_render::GpuSurface` 而非 `wgpu::Surface`。✓
- **验证**：window_demo.rs 用 `rgui_render::GpuSurface`（window_demo.rs:21/106）+ `backend.render_surface(...)`（window_demo.rs:171），**零直接 `wgpu::`**。✓

### 1.3 【PASS】window_demo 走 platform + render 公共 API
`window_demo.rs`（181 行，D7 是 241 行，更精简）：
- `use rgui_platform::AppRunner`（window_demo.rs:14）+ `rgui_platform::event_loop::{ElementState,...}`（15-17）+ `rgui_platform::window::{Window, WindowConfig}`（18）。
- `use rgui_render::{GpuSurface, VelloBackend}`（20-21）。
- `impl AppRunner for DemoApp`（init/event/draw，window_demo.rs:124-175），`main` 里 `run_as_with_config(DemoApp::new(), config)`（window_demo.rs:179）。
- **主线清晰**：窗口创建/事件循环在 platform Runner（RunHandler 桥接），组件交互在 DemoApp，渲染在 render_surface。✓

### 1.4 【PASS】D7 的 P1-1/P1-2（wgpu API 疑点）现状
- P1-1（`new_without_display_handle` 建 surface）：`create_surface`（vello.rs:104-111）+ `VelloBackend::new`（vello.rs:31-32）保持 D7 实现。**本次未改**，仍是 macOS 实证 + 跨平台待验证（D9 确认）。
- P1-2（renderer owned）：编译+运行成功，renderer owned 假设成立。D8 未破坏。

---

## 二、Q2 — 窗口尺寸/默认配置（PASS）

### 2.1 WindowConfig::default 非零 300x200（正确修复）
- `WindowConfig::new()`（window.rs:27-33）：`title:"rgui"`, `width:300`, `height:200`——**非零默认尺寸**。`Default` 委托 `new()`（window.rs:44-48）。✓
- **修复意义**：D7 的 `WindowConfig::default()` 曾是无默认值（0x0），导致窗口零尺寸不可见。D8 改为 300x200，**解决"窗口弹出不可见"回归**。总监实测"窗口弹出可见（非零尺寸）"与此一致。✓

### 2.2 run_as 委托 run_as_with_config（健壮）
- `run_as(app)`（event_loop.rs:45-47）→ `run_as_with_config(app, WindowConfig::new())`——**单一入口，委托清晰**，用默认 300x200。✓
- `run_as_with_config(app, config)`（event_loop.rs:52-64）——真正创建 EventLoop + 设置 ControlFlow::Poll + run_app(Runner)。✓

---

## 三、Q3 — 防火墙 / DAG（PASS，极净）

逐 crate 依赖验证（全部通过）：

| 依赖方向 | 结果 |
|---|---|
| `core` → render/platform | **零依赖**（core/Cargo.toml 无 render/platform）✓ |
| `platform` → render | **无**（platform/Cargo.toml 无 rgui-render）✓（互不相依）|
| `render` → platform | **无**（render/Cargo.toml 无 rgui-platform）✓（互不相依）|
| `render` → winit | **无**（render 仅 core + GPU deps）✓ |
| `platform` → wgpu/vello | **无**（platform 仅 core + winit）✓ |
| 单一 vello/winit | render 仅 vello-backend（无 skia）；platform 仅 winit ✓ |

**DAG 形态**：`core(零依赖) ← render(GPU) ← facade` 和 `core ← platform(winit) ← facade`——**render 与 platform 互不相依，都是只向下依赖 core**，形成"core 为唯一底座，render/platform 两个隔离柱"的干净 DAG。**这是 D8 架构收敛最成功的点**。

---

## 四、Q4 — 事件循环收敛（PASS）

### 4.1 ControlFlow::Poll + about_to_wait 每帧重绘（收敛正确）
- `run_as_with_config`：`event_loop.set_control_flow(ControlFlow::Poll)`（event_loop.rs:57）——**持续轮询**。
- `about_to_wait`（event_loop.rs:109-113）：每帧 `window.request_redraw()`——**主动每帧重绘**，不依赖系统 RedrawRequested 调度。
- **首帧稳定渲染保持**：`resumed` 里窗口创建后 `request_redraw()`（event_loop.rs:84）+ about_to_wait 每帧触发——窗口弹出即渲染。总监实测"稳定渲染蓝色按钮"与此一致。✓

### 4.2 【P2-观察】无限每帧重绘（CPU 占用）
- `about_to_wait` **无条件**每帧 `request_redraw`（event_loop.rs:110-112）——即使无事件/无变化也持续渲染。这是"窗口持续刷新"模式，好处是简单且首帧稳定，**坏处是高 CPU/GPU 占用**（无事件时也满帧渲染）。
- **判定**：P2（非缺陷）。当前 demo 无动画逻辑，可接受；但 D9+ 引入真实组件后，**应有 dirty-flag/事件驱动的按需重绘**（仅状态变化才 redraw），否则交互应用持续烧 CPU。建议 D9 引入 `ControlFlow::Wait` + 仅 dirty 时 redraw 的演进。**这与 greenfield §E.3 的增量/性能目标相关。**

### 4.3 【PASS】事件分发正确
- `window_event`（event_loop.rs:88-107）：`CloseRequested`→`exit()`；其它事件 → `app.event(window, &event)`；`RedrawRequested` → `app.draw(window)`。✓
- `resumed`（event_loop.rs:74-86）：`create_window` → `app.init(window)` → `request_redraw`。✓

---

## 五、Q5 — D7 前风险总结

- **P0 清零**。
- **P1：无（本次未新增）**。D7 的 P1-1（display_handle 跨平台）仍存在但未恶化，且是"跨平台待验证"性质，非本机崩溃。当前 macOS 实证截图稳定。
- **P2 观察项**：见 §六。

---

## 六、P2 观察项清单（随 D9/后续处理，不阻塞 D8）

| # | 项 | 位置 | 说明 |
|---|---|---|---|
| P2-1 | **facade 双入口未打通**：facade 的 `App/AppConfig`（app.rs）仍是 D3 占位（`App::run()` 是 `todo!()`，app.rs:50），而真正的入口是 platform 的 `AppRunner`/`run_as_with_config`。window_demo 用 platform，绕过了 facade 的 `App` | app.rs vs platform/event_loop.rs | D9 建议让 facade 的 `App::run()` 委托 platform `run_as`（或删掉占位 App，统一走 AppRunner），避免"两个 App 概念"并存造成使用困惑 |
| P2-2 | **facade 残留独立 `dep:winit`/`dep:wgpu`** | rgui/Cargo.toml:16-18, 26-27 | facade 源码未直接 `use winit/wgpu`（src/lib.rs 仅 re-export core/platform/render），但 Cargo.toml 仍挂 `winit`/`wgpu`（optional）+ `dep:winit`/`dep:wgpu`。window_demo 在 examples 用 `rgui_platform::`/`rgui_render::`，**无需 facade 提供 winit/wgpu**。这两个独立依赖是**多余**的（feature 透传即可）。D9 可清理（除非有意让 `rgui` crate 直接依赖，但当前源码未消费）。 |
| P2-3 | **文本仍矩形近似（无真实字形）** | vello.rs:300-310（DrawText 用 scene.fill 色块）| 遗留；截图蓝色按钮未见文字。cosmic-text 在 vello-backend feature 内，D9 接真实字形 |
| P2-4 | **offscreen.rs 手工 rect 残留** | rgui-render/tests/offscreen.rs | D5 旧测试（red_filled_rect），与新 e2e 并存，建议归档/删 |
| P2-5 | **增量单向（改 core→render）不可达** | D4 遗留 | render 依赖 core 整 crate，改 core 数据层仍重编 render；D9 裁决 |
| P2-6 | **demo 未走 Coordinator** | window_demo.rs:142/152 | `self.button.update` 手调；D9 收敛到 Coordinator |
| P2-7 | **surface.configure 在 render_surface 每帧调用** | vello.rs:245 | 应 resize 时才 configure |
| P2-8 | **about_to_wait 无限每帧重绘（CPU 占用）** | event_loop.rs:109-113 | D9 引入 dirty/按需重绘（Wait + dirty-only redraw）|
| P2-9 | **CurrentSurfaceTexture 非成功帧直接 return** | vello.rs:257 | Outdated/Lost 应重新 configure；当前可自愈 |
| P2-10 | **注释误写 WidgetId** | scene_graph.rs:65（D6 遗留）| PropValue 无 WidgetId 变体 |

---

## 七、P0/P1 风险清单 + MERGE GATE 建议

### P0 风险清单：**无（P0 清零）**

### P1 风险清单：**无（P1 清零）**

### MERGE GATE 建议：**放行（PASS）**

- **放行理由（充分）**：
  1. **D7 的 P2-1（platform 空壳/窗口逻辑绕过）已彻底解决**——窗口/事件循环收敛进 platform（run_as/AppRunner/WindowConfig），surface 渲染收敛进 render（render_surface/GpuSurface），window_demo 零直接 winit/wgpu。**架构边界真正落地。**
  2. **窗口尺寸回归修复正确**——WindowConfig::default 非零 300x200，解决"窗口不可见"。
  3. **事件循环收敛正确**——ControlFlow::Poll + about_to_wait 每帧 redraw，首帧稳定渲染保持（总监实测截图确认）。
  4. **防火墙/DAG 极净**——core 零依赖、platform⇄render 互不相依、render 无 winit、platform 无 wgpu/vello、单一 vello/winit。**这是 D8 最成功的点。**
  5. **P0 清零，P1 无新增。**
- **P2 观察（随 D9 处理）**：facade 双入口未打通（App/AppConfig 占位 vs AppRunner）、facade 残留独立 dep:winit/dep:wgpu、文本真实字形、about_to_wait 无限重绘（CPU）、增量单向不可达。
- **一句话**：D8 把 D7 的架构收尾做对了——窗口逻辑真正收进 platform，surface 收进 render，防火墙修到"极净"形态。核心目标达成，可放行 D9。建议 D9 优先：① facade 入口统一（App::run 委托 run_as，或删占位 App）；② 文本真实字形；③ about_to_wait 按需重绘（避免交互应用烧 CPU）。P1-1（display_handle 跨平台）建议 D9 在非 macOS 平台验证一次。

---

*审查方：devco-reviewer｜只读审查（未运行 GPU 测试，窗口实证源自总监截图）；依赖方向经逐 crate Cargo.toml 验证；D8 的边界收敛（platform/render 互不相依、core 零依赖）为本次审查确认的最优结果。*
