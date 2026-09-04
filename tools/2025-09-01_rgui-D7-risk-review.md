# D7（窗口 + 事件循环）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D7 交付（winit 窗口 + wgpu surface 渲染 + 事件->更新->重绘）
> 审查基准：`tools/2025-09-01_rgui-greenfield-architecture.md` §C.3/§D + D6 审查遗留 P2-1 等
> 审查方式：只读代码核查（未运行 GPU 测试）；已用 vision 验证总监截图 `tools/qa/d7_screenshots/d7_director_verified.png`
> 关键结论：截图确认真实窗口渲染出蓝色按钮（非空白），D7 的"窗口能画出来"**实证成立**（与总监实测一致）。

---

## 〇、结论速览

| # | 审查点 | 定级 | 一句话结论 |
|---|---|---|---|
| Q1 | 窗口渲染路径 | **PASS（实证成立）+ P2 观察** | vello 离屏中间纹理 + TextureBlitter blit 稳健；但有 surface configure 每帧、display_handle 疑点 |
| Q2 | 事件->更新->重绘闭环 | **PASS** | MouseInput->update->request_redraw->draw 正确；P2-1 嵌套修复无回归（有 nested_layout 测试）|
| Q3 | winit 0.30 + wgpu 29 稳健性 | **P1（2 处 API 疑点）** | `new_without_display_handle` 建 surface 疑点 + instance/adapter 存储疑点 |
| Q4 | 边界清晰 / 防火墙 | **P2（架构边界缺口）** | core 零 GPU ✓ 单一 vello/winit ✓ 但 platform 层是"类型别名壳"，D7 的窗口逻辑绕过它 |
| Q5 | D7 前风险 | —— | 无 P0；P1 疑点见 Q3；P2 观察项见 §六 |
| Q6 | P2 观察项 | —— | 全部列出（见 §六） |

**总评：D7 的"窗口渲染出组件 + 事件闭环"实证成立（截图真实），P0 清零，"窗口能画出来"这一 D7 核心目标达成。** 但有 2 处需关注的 wgpu 29 API 疑点（Q3，建议 D8 前确认）+ 1 个架构边界缺口（platform 层空壳）。**建议：有条件放行（CONDITIONAL PASS），P1 疑点需在 D8（组件/交互深化）前澄清，避免放大为运行时崩溃。**

---

## 一、Q1 — 窗口渲染路径（PASS 实证成立）

### 1.1 【实证】截图验证窗口真实渲染（非空白）
vision 分析 `d7_director_verified.png`：窗口标题 "rgui window demo" 清晰可见，窗口内渲染出**蓝色按钮组件**（顶部蓝色条 + 中下部灰色区域），**非空白窗口**。这与总监实测"窗口渲染出蓝色按钮组件"一致，**"窗口能画出来"实证成立**。✓

### 1.2 【稳健】vello 离屏中间纹理 + blit 到 surface（dev 关键修复正确）
- dev 对"surface 直写多平台不支持"的修复**正确且必要**：`surface.get_current_texture()` 后直接用 vello `render_to_texture` 会因 surface 格式/能力不匹配触发 Validation Error。改为 `create_offscreen_texture`（vello.rs:114-131，含 STORAGE_BINDING|TEXTURE_BINDING|COPY_SRC）→ `render_to_view` 渲染到离屏 → `TextureBlitter::copy` blit 到 surface（window_demo.rs:226-231）。
- **链路正确**：offscreen 纹理带 `TEXTURE_BINDING`（供 blit 采样）+ `COPY_SRC`；blit 用 `wgpu::util::TextureBlitter::copy`（window_demo.rs:230），标准做法。✓
- **资源管理**：offscreen 纹理局部作用域自动 drop；surface 由 winit 管理；`frame.present()` 正确提交（window_demo.rs:232）。无泄漏。
- **render_to_view 复用**（vello.rs:63-86）：`render_offscreen` 和窗口 surface 共用此核心，代码收敛良好。✓

### 1.3 【P2-观察】两处可优化（非缺陷）
- **`surface.configure` 每帧调用**（window_demo.rs:203）：每次 draw 都重新 configure，浪费。应只在 resize 时 configure。**P2**（性能/D8 优化）。
- **`SurfaceConfiguration` 手写 fallback**（window_demo.rs:193-202）：仅当 `get_default_config` 返回 None 时用，逻辑正确但冗长。**P2**。

---

## 二、Q2 — 事件->更新->重绘闭环（PASS）

### 2.1 闭环正确性（逐行审读）
`AppHandler::window_event`（window_demo.rs:131-163）：
- `RedrawRequested` → `self.draw()`（window_demo.rs:136-138）——**重绘驱动正确**。
- `MouseInput{Left, Pressed}` → `button.update(ClickMsg::Increment, &mut state, &mut ctx)` → `request_redraw()`（window_demo.rs:139-149）——**事件→update 状态变化→request redraw→draw** 闭环完整。✓
- `KeyboardInput(KeyR, Pressed)` → `update(Reset)` → `request_redraw()`（window_demo.rs:150-160）——键盘交互也走闭环。✓
- `CloseRequested` → `event_loop.exit()`（window_demo.rs:133-135）——窗口关闭正确处理。✓
- `resumed()` 里窗口创建后 `request_redraw()`（window_demo.rs:128）——**dev 修复"窗口空白"的关键**：主动触发首帧。✓

### 2.2 【PASS】P2-1 嵌套布局修复无回归
- scene_graph.rs:117-126 **已累加父偏移**：`accumulated.position = slot.position + child_slot.position`（scene_graph.rs:121-122）——正是 D6 审查指出的"嵌套坐标未累加父偏移"，已修复。
- 有专门测试 `nested_layout.rs`：断言"目标子绿 x >= 100（占位子之后）"（nested_layout.rs:55-59）+ "孙红 x >= 子绿 x（累加父偏移）"（nested_layout.rs:65-70）。**修复有 TDD 证据闭环**。✓

### 2.3 【P2-观察】update 与 view 分离（window_demo 里 update 直接手调）
- `draw()` 里 `button.view(&state, &ctx)`（window_demo.rs:211）+ `SceneGraph::from_view(view_tree)`（window_demo.rs:212）——view 每次都重新构建。这是**朴素但对**的做法。但**没有走 `Coordinator`**（D4 的 `Coordinator<W>` 封装了 update/view 闭环），demo 直接 `button.update` + `button.view` 手动拼。**P2**：D8 组件深化时应收敛到 Coordinator，避免 demo 与框架 API 脱节（window_demo.rs:145/156 直接 `self.button.update`，未用 Coordinator）。

---

## 三、Q3 — winit 0.30 ApplicationHandler + wgpu 29 API 稳健性（P1，2 处疑点）

### 3.1 【P1-疑点A】`VelloBackend::new()` 用 `new_without_display_handle()` 却要 `create_surface`
- vello.rs:32：`Instance::new(InstanceDescriptor::new_without_display_handle())`——**无 display handle 的 instance**。
- window_demo.rs:121-122：`backend.create_surface(window.clone())`——`instance.create_surface(window)`（vello.rs:109）。
- **矛盾**：`new_without_display_handle()` 创建的 instance 在多数平台**无法创建基于窗口的 surface**（需要 display handle 才能绑定到 OS 窗口/raw-window-handle）。当前截图成功，说明 macOS 上 wgpu 29 可能通过后台方式解析了 handle（或 wgpu 29 语义变化）。**但这依赖平台行为，跨平台（Linux/Wayland/Windows）不一定成立。**
- **定级：P1（需 D8 前确认）**。建议：`VelloBackend::new()` 区分"离屏模式"（`new_without_display_handle`）与"窗口模式"（用 `Instance::new(InstanceDescriptor::default())` 或传入 display_handle），或提供 `new_for_window()` 构造。当前"一个 new() 通吃"在窗口场景下依赖平台巧合。**截图成功不代表跨平台稳健**。

### 3.2 【P1-疑点B】`VelloBackend` 存储 `instance`+`adapter`+`device`+`queue`（生命周期）
- vello.rs:22-26：`VelloBackend{ instance, adapter, device, queue, renderer }`。`renderer: Renderer` 是 vello 渲染器（持有 `device` 的引用吗？）。若 Renderer 内部 borrow device，则 `&self.device`/`&self.queue` 暴露（vello.rs:89-101）可能与 renderer 的借用冲突。
- **但**从 window_demo.rs 编译通过 + 运行成功看，当前 wgpu 29 + vello 0.9 的 Renderer 是**owned（不 borrow device）**，所以无冲突。**属猜测，需确认**——D8 前建议跑一次 `cargo build --features window` 确认 Renderer 是 owned。
- **定级：P2**（低风险，能编译即安全）。

### 3.3 【P1-疑点C】`wgpu::CurrentSurfaceTexture` 枚举 & `TextureBlitter`
- window_demo.rs:220-222：`SurfaceTexture::Success(t) / Suboptimal(t) / Outdated / Lost / OutOfMemory` 处理——wgpu 29 用**枚举而非 Result**。`Success/Suboptimal` 取纹理，**其它（Outdated/Lost/OutOfMemory）直接 return 不 present**——**逻辑正确**（非成功帧不 present，防错帧提交）。✓ 但 `Outdated/Lost` 时**应重新 configure surface 而非直接丢弃**，当前直接 return 会导致该帧不渲染（下帧 RedrawRequested 会再来，可自愈）。**P2**（D8+ 优化）。
- `wgpu::util::TextureBlitter`：wgpu 29 的 `util::TextureBlitter` 是正确的 blit 工具。`blitter.copy(...)` 用法正确。✓

### 3.4 【PASS】ApplicationHandler 用法
winit 0.30 的 `ApplicationHandler + resumed/window_event`（window_demo.rs:110-163）用法**正确**（winit 0.30 新 trait 模型）。`resumed` 里 `create_window`（必须在事件循环内），`window_event` 分发——符合 winit 0.30 约定。✓

---

## 四、Q4 — 边界 / 防火墙（P2，架构边界缺口）

### 4.1 【PASS】core 零 GPU / 单一 vello / 单一 winit
- core：[dependencies] 无 wgpu/vello/winit；源码零引用。**防火墙保持**。✓
- render：[features] `vello-backend`=《vello,wgpu,cosmic-text,fontdb,skrifa,pollster》单一路径，无 skia。✓
- render 依赖 core（render/Cargo.toml:11）；platform 依赖 core（platform/Cargo.toml）；facade 依赖全部。**DAG 无环**。✓

### 4.2 【P2-边界缺口】platform 层是"类型别名壳"，D7 窗口逻辑绕过它（重点）
- **`rgui-platform` 现状**：window.rs 提供 `WindowConfig`/`to_winit_attributes`/`Window`(类型别名)；event_loop.rs 提供 `EventLoop`(别名)+`build()`。**全是"占位类型别名壳"。**
- **`window_demo.rs` 实际行为**：直接 `use winit::application::ApplicationHandler`、`winit::event_loop::EventLoop`、`winit::window::Window`（window_demo.rs:16-19），**完全没走 `rgui-platform` 的 window.rs/event_loop.rs**。窗口创建（window_demo.rs:118）、事件循环（242）、surface 配置（window_demo.rs:191-203）全在 example 里用原始 winit/wgpu。
- **判断**：greenfield §C.3 要求"`rgui-platform` 是 winit 隔离、平台句柄挡在核心外"，但 D7 的**真实窗口/事件逻辑没进 platform**，platform 的 `window.rs`/`event_loop.rs` 是**未被消费的死代码**（除了 facade 可能 re-export）。**架构边界未落地**——当前"窗口+事件循环"耦合在 example 里，D8+ 若多个 app 都要窗口，会重复这段原始 winit/wgpu 代码，且 winit/wgpu 直接暴露给使用者。
- **定级：P2（架构观察）**，不阻塞 D7（example 能跑），但 D8 组件深化时**建议把 window_demo 的 AppHandler/surface 逻辑收敛进 `rgui-platform` 或 facade**，让 platform 真正承担"winit 隔离"职责，避免绕过后 greenfield §C.3 契约名存实亡。

---

## 五、Q5 — D7 前风险总结

- **P0 清零**。截图实证"窗口渲染出组件"真实成立。
- **P1 疑点 2 项**（Q3-3.1 display_handle 疑点、Q3-3.2 renderer owned 疑点）——非 P0，但建议 D8 前跑一次 `cargo build --features window` + 跨平台确认，避免放大。
- **P2 观察项**：见 §六。

---

## 六、P2 观察项清单（随 D8/后续处理，不阻塞 D7）

| # | 项 | 位置 | 说明 |
|---|---|---|---|
| P2-1 | **platform 层空壳，窗口逻辑绕过它** | window.rs/event_loop.rs vs window_demo.rs | D8 建议把 AppHandler/surface 逻辑收敛进 platform 或 facade，真正承担 winit 隔离 |
| P2-2 | **文本仍矩形近似（无真实字形）** | vello.rs:246-257（DrawText 用 scene.fill 色块） | 截图看按钮**只有蓝色色块，未见文字**——与 D6 审查一致，文本仍是"块"非"字形"。cosmic-text 在 vello-backend feature 内，D8 接真实字形 |
| P2-3 | **offscreen.rs 手工 rect 残留** | rgui-render/tests/offscreen.rs | D5 的 `red_filled_rect` 测试仍在，与新 e2e 并存；建议归档/删（e2e 真实转换已替代） |
| P2-4 | **增量正向（改 core→render）不可达** | D4 遗留 | render 依赖 core 整 crate，改 core 数据层仍重编 render；D8 需裁决 |
| P2-5 | **demo 未走 Coordinator** | window_demo.rs:145/156 | `button.update` 手调而非 Coordinator 封装；D8 收敛 |
| P2-6 | **surface.configure 每帧** | window_demo.rs:203 | 应 resize 时才 configure |
| P2-7 | **CurrentSurfaceTexture 非成功帧直接 return** | window_demo.rs:220-223 | Outdated/Lost 时应重新 configure 而非丢弃（当前可自愈，D8+ 优化） |
| P2-8 | **注释误写 WidgetId** | scene_graph.rs:65（D6 遗留） | PropValue 无 WidgetId 变体，注释与实现不符 |

---

## 七、P0/P1 风险清单 + MERGE GATE 建议

### P0 风险清单：**无（P0 清零）**

### P1 风险清单（2 项，均为 API 稳健性疑点，非崩溃）
| # | 风险 | 位置 | 处置建议 |
|---|---|---|---|
| P1-1 | **`new_without_display_handle()` 创建 instance 却要 `create_surface`** | vello.rs:32 / vello.rs:109 | macOS 截图成功（wgpu 29 后台解析 handle），但**跨平台（Linux/Wayland/Windows）不一定成立**。D8 前建议：区分离屏/窗口两种构造，或改为 `InstanceDescriptor::default()` + 传入 display_handle。**截图成功是平台巧合证据，非跨平台稳健性** |
| P1-2 | **VelloBackend 同时持 renderer 与 device/queue，假设 renderer owned** | vello.rs:22-26 | 当前编译+运行成功表明 renderer 是 owned（borrow 无冲突），但属依赖 vello 0.9 的行为；D8 前 `cargo build --features window` 确认 |

### MERGE GATE 建议：**有条件放行（CONDITIONAL PASS）**

- **放行理由（充分）**：
  1. **窗口渲染出组件实证成立**（截图 vision 确认蓝色按钮，非空白）——D7 核心目标达成。
  2. **事件->更新->重绘闭环正确**（MouseInput→update→request_redraw→draw 完整；CloseRequested 正确 exit）。
  3. **dev 的 surface 直写→离屏中间纹理+blit 修复正确且必要**（消除 Validation Error，符合 wgpu 29 多平台要求）。
  4. **P2-1 嵌套布局修复有 TDD 证据**（nested_layout.rs 断言父偏移累加），无回归。
  5. **core 零 GPU 防火墙保持、单一 vello/winit**（Q4.1 全过）。
  6. **P0 清零**。
- **条件（2 项 P1 转入 D8 待办）**：
  - P1-1（display_handle 疑点）**D8 前确认跨平台**，或提供 `new_for_window()` ——**截图成功是 macOS 平台证据，不是跨平台稳健性的充分证明**。
  - P1-2（renderer owned 假设）D8 前 `cargo build --features window` 确认。
- **一句话**：D7 把"窗口+事件循环+真实组件渲染"跑通了，截图实证非空窗，事件闭环正确，P2-1 修复干净。核心达成，可放行 D8；但提醒两点：① 窗口渲染路径（display_handle/blit）目前只在 macOS 实证，跨平台需 D8 验证；② platform 层仍是空壳，D8 组件深化建议收敛窗口逻辑进 platform（否则 winit/wgpu 直接暴露给所有例子使用者），文本真实字形（P2-2）也建议 D8 接上。

---

*审查方：devco-reviewer｜只读审查（未运行 GPU 测试，窗口实证源自总监截图 + vision 分析）；跨平台 surface/display_handle 行为未在本机验证，标为 P1 需 D8 确认。*
