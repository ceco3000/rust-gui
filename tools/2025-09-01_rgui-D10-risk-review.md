# D10（Accordion/WaBadge 组件 + App::run 暴露 config）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D10 交付
> 审查基准：`tools/2025-09-01_rgui-greenfield-architecture.md` §B.1/§C.1 + D9 审查遗留 P2-1（AppConfig 死代码）
> 审查方式：只读代码核查 + vision 分析截图 + 依赖/边界逐 crate 验证
> 结论性质：基于源码逐行审读 + 组件测试逻辑审读

---

## 〇、结论速览

| # | 审查点 | 定级 | 一句话结论 |
|---|---|---|---|
| Q1 | Accordion/WaBadge 组件正确性 | **PASS** | 完整 WidgetSpec 生命周期；展开/收起逻辑正确；mapper 稳健 |
| Q2 | App::run 暴露 config | **PASS（D9 死代码已修复）** | AppConfig 接收 config，映射平台 WindowConfig；无死代码残留 |
| Q3 | 交互链路 | **PASS（+P2 命中检测简化）** | 点击/Space→Toggle→update flipped→view 更新→dirty→重绘正确；但 mapper 无 hit-test |
| Q4 | 组件架构 | **PASS** | Accordion/WaBadge 归属 core::components 正确；零 GPU/平台泄漏 |
| Q5 | 防火墙/DAG | **PASS** | core 零 GPU/平台；组件无违规依赖；单一 vello/winit |
| Q6 | D9 前风险 | —— | 无 P0；P2 观察项见 §六 |

**总评：D10 达成——Accordion（可折叠/展开交互组件）+ WaBadge（徽章）作为 Tier 1 WidgetSpec 落地，App::run 暴露 config（D9 的 AppConfig 死代码已修复），截图实证窗口显示 Accordion "Settings [+]" 收起态（清晰渲染非空白）。P0 清零。组件生命周期、交互链路、边界/防火墙全部可靠。建议：放行（PASS）。** 有 2 项 P2 观察（mapper 无 hit-test 属架构现状、Accordion 交互为全局事件）随后续处理。

---

## 一、Q1 — Accordion/WaBadge 组件正确性（PASS）

### 1.1 Accordion 完整生命周期（WidgetSpec 五项全实现，components.rs:76-122）
- **state**：`AccordionState{title, subtitle, expanded}`（components.rs:30-38），`Default` 收起态（expanded:false）。`PersistState` 实现（schema_name="accordion_state", schema_version=0, as_any*，components.rs:50-63）。✓
- **message**：`AccordionMsg::Toggle`（components.rs:15-19），`AppMessage`（message_name="accordion.toggle"）。✓
- **view**（components.rs:84-108）：根 size 随 expanded 变高（44 vs 130，components.rs:86-87）；header（Color 蓝底 340x36，components.rs:91-92）+ 标题行显示 `"{} [{}]"` 带 `+`/`-` 标记（components.rs:93-96）；**展开时** push 内容（components.rs:101-106），**收起时不显示**。✓
- **update**（components.rs:110-114）：`Toggle → expanded = !expanded`——**翻转正确**。
- **measure**（components.rs:116-119）：高度随 expanded 变（44 vs 130）。**paint**（components.rs:121）：空实现（可视渲染靠 from_view 的 Color/Str 映射，符合 D6 的 SceneGraph 路径）。✓

### 1.2 WaBadge（components.rs:180-206）
- `WaBadgeState{label, count}`，`WaBadgeMsg`（空枚举，无交互），view 显示 `"{}: {}"` label+count（components.rs:193），Color 底。完整生命周期（update 空、measure 160x40、paint 空）。✓

### 1.3 【PASS】展开/收起状态逻辑（测试证据）
`d10_components.rs`（TDD）：
- `accordion_initially_collapsed`（d10_components.rs:29-33）：初始未展开。✓
- `accordion_toggle_expands_then_collapses`（d10_components.rs:36-46）：Toggle→expanded，再 Toggle→collapsed——**翻转正确**。✓
- `accordion_view_shows_content_when_expanded`（d10_components.rs:49-70）：收起不含 "details"，展开含 "details"——**view 按 expanded 正确显示/隐藏内容**。✓
- `badge_view_shows_label_count`（d10_components.rs:73-83）：WaBadge view 含 count。✓

### 1.4 【PASS】事件→消息映射闭包稳健
`window_demo.rs:17-32`：`mapper = |event| match event { MouseInput(Left)→Toggle, KeyR(Space)→Toggle, _→None }`——闭包干净，`FnMut(&WindowEvent)->Option<AccordionMsg>`（app.rs:92 约束），无借用逃逸。✓

---

## 二、Q2 — App::run 暴露 config（PASS，D9 死代码已修复）

### 2.1 【PASS】AppConfig 被真正使用（D9 P2-1 闭合）
- `App::run<W,F>(config: AppConfig, widget, state, mapper)`（app.rs:84-89）——**config 是第一个参数**，用户显式传 AppConfig。
- 内部：`WindowConfig{title: config.window_title.clone(), width: config.width, height: config.height}`（app.rs:95-99）——**AppConfig 映射到平台 WindowConfig，D9 的硬编码 620x220 已消除**。✓
- `AppConfig::new()` 默认（app_name/window_title="rgui", width=300, height=200，app.rs:33-40）+ `with_title`/`with_size`（app.rs:43-53）——**无死代码，全部被用**。✓
- window_demo 用 `AppConfig::new().with_title("rgui accordion demo").with_size(480, 320)`（window_demo.rs:12-14）——**用户可自定义标题/尺寸**。✓

### 2.2 【PASS】与 platform WindowConfig 整合干净
- AppConfig（rgui）→ WindowConfig（platform）的映射在 `App::run` 内一次性完成（app.rs:95-99），`title/width/height` 三字段一一对应。**无重复配置概念**（AppConfig 是 facade 层用户接口，WindowConfig 是 platform 实现细节，边界清晰）。✓
- AppConfig 含 `app_name` 字段（app.rs:22）但未用于 WindowConfig（只有窗口标题/尺寸）——`app_name` 是**预留字段**，当前未消费。**P2 观察**（无功能影响，D11 可明确用途或删）。

---

## 三、Q3 — 交互链路（PASS + P2 命中检测简化）

### 3.1 【PASS】点击/Space→Toggle→update→view→重绘，链路正确
完整链路（复用 D9 的 event_loop + Coordinator）：
1. 窗口事件 `MouseInput(Left)` 或 `KeyboardInput(Space)` → mapper 返回 `Some(Toggle)`（window_demo.rs:19-29）。
2. facade `AppRunnerImpl::event`（app.rs:135-143）：`mapper(event)` 得 Some → `coordinator.dispatch(Toggle, &mut ctx)` → 返回 `true`（dirty）。
3. platform `window_event`（event_loop.rs:114-118）：dirty=true → `pending=true` + `request_redraw()`。
4. `RedrawRequested`（event_loop.rs:119-122）→ `app.draw(window)` → `coordinator.current_view()`（app.rs:150）+ `SceneGraph::from_view` + `render_surface`。
5. `about_to_wait`（event_loop.rs:126-134）：has_drawn/pending 条件控制——不空转。
- **状态更新→视图更新→重绘闭环正确**，无漏重绘（dirty 触发到位）、无过度重绘（Wait 空闲跳过）。✓

### 3.2 【P2-观察】mapper 无 hit-test（命中检测）
- `window_demo.rs:19-23`：**任意 `MouseInput(Left)` 都映射为 Toggle**——无论点击位置（窗口空白、标题区、内容区全触发）。**没有 cursor 位置/坐标命中检测**。
- **现状评估**：demo 单组件（全窗口就 Accordion），无多组件冲突，**可接受**。但暴露架构现状——**事件→消息映射是"全局的/全窗口的"，尚未做"把事件路由到具体宿主/组件（hit-test）"**。
- **P2**：D11+ 多组件/复杂布局时，需在 `Coordinator`/交互层加入 hit-test（把 `MouseInput` 的 cursor 坐标映射到某组件才 dispatch 其消息），否则多组件会"点击 A 触发 B"。当前是"全局分发"简化。

---

## 四、Q4 — 组件架构（PASS）

### 4.1 【PASS】Accordion/WaBadge 归属 rgui-core::components 正确
- `components.rs` 在 `rgui-core`（lib.rs:22 `pub mod components`，lib.rs:10 注释"吸收 state/layout/components/a11y_tree"）。✓
- **契约 greenfield §B.1/§C.1**：components 属于 core（"`rgui-core::components`：Accordion/WaBadge 是纯 WidgetSpec"），归属正确。✓

### 4.2 【PASS】无 UI 逻辑泄漏进 core 的 GPU/平台边界
- components.rs **零引用** wgpu/vello/winit/fontdb/cosmic/rgui_render/rgui_platform（grep 空）。✓
- `_color_marker`（components.rs:209-210）是 `#[allow(dead_code)]` 的占位——**P2 观察**（无功能影响，可删）。
- components.rs 只用 `crate::context/geometry/traits/view`——**纯 core 内部类型**，无外部泄漏。✓

---

## 五、Q5 — 防火墙/DAG（PASS）

### 5.1 core 零 GPU/平台
- core/Cargo.toml：grep wgpu/vello/winit/cosmic/fontdb → **零**。✓
- core 源码：grep `rgui_render/rgui_platform/wgpu/vello/winit` → **零**（仅注释）。✓

### 5.2 组件无违规依赖
- Accordion/WaBadge 只用 core 类型（context/geometry/traits/view），**未引入** GPU/平台/外部 crate 依赖。✓

### 5.3 单一 vello/winit
- render：vello-backend 单一（cosmic/fontdb 门控，无 skia）；platform：winit 单一。DAG 无环（D8/D9 确认）。✓

---

## 六、Q6 — P2 观察项清单（随 D11/后续处理，不阻塞 D10）

| # | 项 | 位置 | 说明 |
|---|---|---|---|
| P2-1 | **mapper 无 hit-test（全局事件分发简化）** | window_demo.rs:19-29 | 任意 Left 点击→Toggle，无坐标命中检测；单组件 demo 可接受，D11 多组件需在交互层加 hit-test |
| P2-2 | **AppConfig.app_name 字段未消费** | app.rs:22 | 预留字段，未用于 WindowConfig；D11 明确用途或删 |
| P2-3 | **Accordion view 中 measure/paint 分离** | components.rs:84-122 | view 里硬编码尺寸（340/36/44/130）而非复用 measure；D11 可统一 |
| P2-4 | **`_color_marker` 占位函数** | components.rs:209-210 | `#[allow(dead_code)]`，无功能，可删 |
| P2-5 | **跨平台 P1-1 未验证** | vello.rs:34 | new_without_display_handle 建 surface，macOS 实证，linux/windows 待验证（D9 遗留） |
| P2-6 | **offscreen 手工 rect 残留** | rgui-render/tests/offscreen.rs | D5 旧测试，建议归档/删 |
| P2-7 | **增量单向不可达** | D4 遗留 | render 依赖 core 整 crate；D11 裁决 |
| P2-8 | **WaBadge 无交互（空消息枚举）** | components.rs:128 | WaBadgeMsg 空枚举，view 显示 label 但不可交互——符合"徽章只读"预期，D11 如需动态可扩展 |

---

## 七、P0/P1 风险清单 + MERGE GATE 建议

### P0 风险清单：**无（P0 清零）**

### P1 风险清单：**无（P1 清零）**

### MERGE GATE 建议：**放行（PASS）**

- **放行理由（充分）**：
  1. **Accordion/WaBadge 作为 Tier 1 WidgetSpec 完整落地**——五项生命周期（view/update/measure/paint/accessibility-default）全实现，展开/收起逻辑正确（d10_components 测试 TDD 证据），view 按 expanded 显示/隐藏内容。
  2. **App::run 暴露 config**——D9 的 AppConfig 死代码已修复，AppConfig 映射平台 WindowConfig，用户可自定义标题/尺寸（window_demo:480x320 "rgui accordion demo"）。
  3. **交互链路正确**——点击/Space→Toggle→update flipped→view 更新→dirty→重绘，闭环无漏/过度重绘。
  4. **组件架构正确**——Accordion/WaBadge 归属 core::components，零 GPU/平台泄漏。
  5. **防火墙/DAG 保持**——core 零 GPU/平台，组件无违规依赖，单一 vello/winit。
  6. **截图实证**——vision 确认窗口 "rgui accordion demo" 显示 Accordion "Settings [+]" 收起态，清晰渲染非空白。
  7. **P0 清零，P1 无新增**。
- **P2 观察（随 D11 处理）**：mapper 无 hit-test（多组件时需路由）、AppConfig.app_name 未消费、跨平台 P1-1、offscreen 手工 rect、增量单向。
- **一句话**：D10 把"真实可交互组件"落地了——Accordion 折叠/展开 + WaBadge 徽章，经 facade App::run(config,...) 启动，截图实证。核心达成，可放行 D11。建议 D11 优先：① 命中检测（hit-test，把事件路由到具体组件）——多组件/复杂 UI 的必经之路，当前是全局分发简化；② 跨平台验证 display_handle；③ 处理 offscreen 手工 rect 与增量单向遗留。

---

*审查方：devco-reviewer｜只读审查（未运行 GPU 测试，交互实证源自截图 + vision 分析 + d10_components 测试审读）；组件生命周期、交互链路、边界/防火墙经逐行审读验证。*
