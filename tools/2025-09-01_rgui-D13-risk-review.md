# D13（ViewContext.focused 焦点透传 + 获焦高亮 ▶ 前缀 + demo）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D13 交付（commit 8e59f85）
> 基准：greenfield §B.1、D5 事件系统、既有判据 + **流式编码判据**
> 范围：①ViewContext.focused ②获焦高亮 ▶ ③demo 焦点路由 ④流式编码 ⑤防火墙 ⑥文档一致性
> 方法：只读代码核查（context.rs/components.rs/window_demo.rs/d10_components.rs 逐行）+ 实测 `cargo test --workspace --all-features`

---

## 〇、结论速览

| # | 审查点 | 定级 |
|---|---|---|
| 1 | ViewContext.focused | **PASS（view 层焦点透传，向后兼容）** |
| 2 | 获焦高亮 ▶ 绘制 | **PASS（Str prefix 进 DrawText，未泄漏焦点进 GPU/渲染层）** |
| 3 | demo 焦点路由 | **PASS（DemoMsg::Focus + state.focused）** |
| 4 | **流式编码判据** | **PASS（组件 view 无手写循环/装箱/冗余 collect）** |
| 5 | 防火墙/DAG | **PASS（焦点视觉在组件 view=core 零 GPU；render 只渲染 draw 指令）** |
| 6 | 文档一致性 | **PASS（D5 已同步；1 处观察——UpdateContext 描述，见 §六）** |

**总评：D13 达标——ViewContext.focused 视图层焦点透传、获焦高亮 ▶ 前缀（Accordion/WaBadge）、demo 焦点路由全部正确，流式编码合规，60 测试全绿（实测），P0 清零。建议：放行（PASS）。** 无 P0/P1，1 条 P2 观察（UpdateContext 描述与代码偏差，D5 前瞻描述但代码未实现 focus 字段）。

---

## 一、ViewContext.focused（PASS）

### 1.1 向后兼容扩展
- `ViewContext`（context.rs:8-13）：`pub focused: bool`（context.rs:10-11）+ `_p: PhantomData`，`#[derive(Debug, Default)]`（context.rs:8）→ **默认 `focused=false`，向后兼容**（既有 ViewContext::default() 构造不受影响）。✓
- `Default` 派生保证所有 `ViewContext::default()`/`ViewContext{}` 均含 `focused=false`——**不破坏既有组件**。✓

### 1.2 view 层职责（清晰）
- `ViewContext.focused` 供**组件 view** 读取获焦态（D13），用于绘制高亮。✓
- **职责区分**：`ViewContext.focused`（view 视角，只读获焦态）vs `UpdateContext`（update 视角）。当前 `UpdateContext`（context.rs:16-19）只有 `_p: PhantomData`（无 focus 字段）——**职责边界清晰**（view 用 ViewContext.focused，update 用 UpdateContext，二者分离）。✓
- 组件 view 读 `ctx.focused`（components.rs:94/202）→ 决定是否加 ▶ 前缀——**view 契约一致**。✓

---

## 二、获焦高亮 ▶ 绘制（PASS）

### 2.1 焦点状态未泄漏进渲染层/GPU（关键）
- 焦点高亮通过**组件 view 的 props（Str）** 实现：
  - Accordion：`let focus_marker = if ctx.focused { "▶ " } else { "" }`（components.rs:94）→ `title.props = Str(format!("{focus_marker}{} [{}]", ...))`（components.rs:96）——**焦点前缀拼进 Str props**。
  - WaBadge：`focus_marker`（components.rs:202）→ `label.props = Str(format!("{focus_marker}{}: {}", ...))`（components.rs:204）。
- 该 Str 作为文本经 `SceneGraph::from_view` → `DrawCmd::DrawText` → render（cosmic-text 字形）渲染。**焦点状态是"组件 view 决定是否加 ▶ 前缀"，经纯数据（Str）传到渲染层**——**未把焦点状态直接传进 GPU/渲染 API（wgpu/vello）**。✓
- **验证**：render 只拿到 `DrawCmd::DrawText`（含前缀文本），**不知晓"焦点"概念**——焦点逻辑完全隔离在 core 组件 view 层。符合"core 零 GPU"、"render 只渲染 draw 指令"的边界。✓

### 2.2 测试覆盖（获焦标记）
- `accordion_view_adds_focus_marker_when_focused`（d10_components.rs:103-114）：未获焦无▶ / 获焦有▶（Accordion）。✓
- `badge_view_adds_focus_marker_when_focused`（d10_components.rs:117-126）：未获焦无▶ / 获焦有▶（WaBadge）。✓
- 覆盖：两个组件获焦/未获焦高亮**全面**。✓

---

## 三、demo 焦点路由（PASS）

### 3.1 焦点状态进组合根状态
- `DemoMsg::Focus(Option<WidgetId>)`（window_demo.rs:31-33）——组合根新增焦点消息。
- `DemoRootState.focused: Option<WidgetId>`（window_demo.rs:51-52），`Default` 初始 `Some(WidgetId::new(1))`（首个可获焦，window_demo.rs:60）。
- `update`：`DemoMsg::Focus(fid) => state.focused = fid`（window_demo.rs:115）。

### 3.2 ViewContext 按子获焦态设置
- `DemoRoot::view`：
  - `acc_ctx.focused = state.focused == Some(WidgetId::new(1))`（window_demo.rs:96）→ Accordion.view
  - `badge_ctx.focused = state.focused == Some(WidgetId::new(2))`（window_demo.rs:102）→ WaBadge.view
- **焦点高亮移动**：focus_next/prev 切换 → `DemoMsg::Focus(fid)` → state.focused 变更 → view 重新渲染 → 高亮 ▶ 在获焦组件移动。✓

### 3.3 焦点切换（Tab/Shift+Tab）
- mapper：`ModifiersChanged` 记录 shift（window_demo.rs:152-155）；Tab → shift? focus_prev : focus_next（window_demo.rs:161-166）→ `DemoMsg::Focus(fid)`（window_demo.rs:168）。日志 `[focus] Tab(shift=..) -> ..`（window_demo.rs:167）。✓
- **Tab 焦点导航 + 高亮联动**完整。✓

---

## 四、流式编码判据（PASS）

### 4.1 合规项
| 判据 | 检查结果 |
|---|---|
| **用组合子替代手写循环** | 组件 view（Accordion/WaBadge）`view` 无迭代器循环（仅 `children.push` 向量建树）；focus 逻辑在 window_demo（`if ctx.focused` 标量）。**无"能用组合子却用循环"** ✓ |
| **`dyn Iterator` 装箱** | grep `dyn Iterator`/`Box<dyn Iterator>`（components/context/view）→ **无** ✓ |
| **冗余中间 collect** | 无冗余 collect ✓ |
| **ViewContext.focused 纯值传递** | `focused: bool` 标量字段，`Default` 派生——纯值，无任何迭代器/收集 ✓ |

### 4.2 组件 view push（边界内，接受）
- Accordion/WaBadge view 用 `header.children.push(title)`、`root.children.push(header/child)`（components.rs:98-99, 106, 206）——**声明式视图树向量构造**，非"组合子可替代的循环"（D11/D12 已判为合法，边界内）。接受。✓

**结论：流式编码判据 PASS。** 无 dyn Iterator 装箱、无冗余 collect、无"能组合子却循环"；VieoContext.focused 纯值传递。

---

## 五、防火墙 / DAG（PASS）

- **焦点视觉在 core 组件 view**：Accordion/WaBadge 的 `view`（components.rs）读 `ctx.focused` 加 ▶ 前缀——纯 core 逻辑，**零 GPU/平台**（只用 core context/traits/view 类型）。✓
- **render 只渲染 draw 指令**：render（vello.rs）只消费 `SceneGraph` 的 `DrawCmd`（含 ▶ 前缀文本），**不知晓焦点状态**。✓
- **core 零 GPU**：context.rs/components.rs 无 wgpu/vello/winit；`grep wgpu/vello` in core→空。✓
- **单一 vello/winit**、DAG 无环——保持。✓

---

## 六、文档一致性（PASS，1 条 P2 观察）

- **D5 已同步焦点管理**（总监确认 commit 8e59f85 含 D5 更新）——ViewContext.focused/获焦高亮/demo Focus 路由与代码一致。✓
- **greenfield/D1 由 doc 同步中**——需注意：`ViewContext` 新增 `focused: bool` 字段，greenfield §B.1 / D0 §6 Context 表应标注"ViewContext{focused:bool}"。**P2 观察**（doc 同步中）。
- **P2 观察**：D5 §2 此前声称 `UpdateContext` 含 `focus/hover/cursor_window_position/cursor_local_position` 字段（D5 写实时前瞻描述了 update 上下文），但当前 `UpdateContext`（context.rs:17-19）仍只有 `_p: PhantomData`，**无 focus/cursor 字段**——D5 的描述超前于代码（未实现）。不过这与 D13 的 `ViewContext.focused`（view 层）是**不同层面**，D13 焦点用 ViewContext 而非 UpdateContext——**D13 无问题，但 D5 §2 的 UpdateContext 字段描述需与代码对齐**（要么补充实现，要么 D5 标注"占位待实现"）。**P2**（doc 一致性观察，随 doc 同步处理）。

---

## 七、P0/P1 风险清单

**P0：无。P1：无。**

**MERGE GATE：放行（PASS）。**

### P2 观察项（随后续处理，不阻塞）
1. **D5 §2 UpdateContext 字段描述超前**：D5 声称 UpdateContext{focus/hover/cursor...}，但代码 UpdateContext 仍空（仅 PhantomData）——D5 需标注"占位待实现"或补字段，避免描述与代码不符（与 D13 的 ViewContext.focused 不同层，不影响 D13）。
2. **ViewContext.focused 需 greenfield/D0 标注**：新增字段（D13 扩展），doc 同步时在 greenfield §B.1 / D0 §6 补记。
3. **焦点高亮为文本前缀（▶ Str）**：可读性依赖字形引擎渲染 ▶ 字符——若字体缺 ▶ 字形会显示缺字符（依赖 cosmic-text 系统字体支持）。P2（后续可考虑 draw 层高亮而非文本前缀）。

---

*审查方：devco-reviewer｜只读审查（未运行 GPU 窗口测试，60 全量数字经 `cargo test --workspace --all-features` 实测核实 = 60 passed）。流式编码判据逐条核对：组件 view 无手写循环/装箱/冗余 collect，ViewContext.focused 纯值；焦点高亮经组件 view 的 Str props 进 draw 指令（未泄漏焦点进 GPU/渲染层），防火墙/DAG 达标。*
