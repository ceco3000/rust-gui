# D6（真实 from_view 转换）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D6 交付（WidgetView→SceneGraph 真实转换）
> 审查基准：`tools/2025-09-01_rgui-greenfield-architecture.md` §B.2/§C.2 + D5 审查遗留 P1-1/P1-2
> 审查方式：只读代码核查 + 坐标叠加逻辑隔离复刻验证（未运行 GPU 测试）
> 结论性质：基于源码逐行审读 + taffy 布局语义分析

---

## 〇、结论速览

| # | 审查点 | 定级 | 一句话结论 |
|---|---|---|---|
| Q1 | from_view 转换正确性 | **PASS（基础达标）+ P2（文本仍近似）** | Color→FillRect、Str→DrawText、递归遍历均实现；但文本仍矩形近似 |
| Q2 | 布局应用 | **P2（嵌套坐标叠加缺口）** | compute_children 真实 bounds、单层正确；**嵌套时父偏移未累加** |
| Q3 | Props 映射完整性 | **P2（注释与实现不符）** | Color/Str 已处理；Unit/Bool/Int/Float 静默忽略合理；但 `WidgetId` 注释误写（PropValue 无此变体） |
| Q4 | 文本路径 | **P2** | Str→DrawText 是**矩形近似块**（非真实 cosmic-text 字形），e2e2 测试无法区分 |
| Q5 | 防火墙/单一 vello | **PASS** | core 零 GPU；render 单一 vello 无 skia 残留 |
| Q6 | D7 前需处理 | —— | 见 §六：嵌套坐标叠加、文本字形是 D7 观察项（非 P0/P1） |

**总评：D5 的 P1-1/P1-2（from_view 占位、离屏绕过转换）已由真实 e2e 链路闭合——PASS。** from_view 现在真实转换（布局 bounds + Color/Str 映射 + 递归），端到端像素测试替换手工 rect（e2e1-e2e4）真实成立。**P0 清零，P1 清零，具备放行 D7 的条件。** 剩余均为 P2 观察项（文本字形、嵌套布局坐标、增量正向未验证）。**建议：放行（PASS），P2 观察项随 D7 处理。**

---

## 一、Q1 — from_view 转换正确性（PASS 基础达标）

### 1.1 转换链路完整（D6 真实实现）
`SceneGraph::from_view`（scene_graph.rs:68-75）：
1. 取容器尺寸：`view.size.unwrap_or(DEFAULT_CONTAINER=200x200)`（scene_graph.rs:70）——**有布局空间兜底**（D5 的漏洞已补，避免 0 尺寸导致无图元）。
2. 根 slot：`LayoutResult::new(container, (0,0))`（scene_graph.rs:71）——根在原点。
3. `emit_node(view, slot)` 递归处理（scene_graph.rs:73）。

`emit_node`（scene_graph.rs:78-120）——D6 核心改进：
- **布局 bounds**：用 `LayoutEngine::compute_children(slot.size, &child_sizes)`（scene_graph.rs:88）计算每个子节点的真实位置/尺寸——**布局真正作用于渲染**（D5 的固定 100x40 已被替换）。✓
- **本节点图元映射**（scene_graph.rs:91-114）：
  - `PropValue::Color(c)` → `FillRect{x:slot.position.x, y:slot.position.y, width:size, height:size, color}`（scene_graph.rs:92-101）——用**布局位置 + 尺寸**绘制。✓
  - `PropValue::Str(text)` → `DrawText{x, y, text, size:size.height, color:灰}`（scene_graph.rs:102-112）——Str 不再静默忽略，转 DrawText。✓（D5 的 P1-2"Str 静默忽略"已修复）
  - 其它 → `_ => {}`（scene_graph.rs:113）。✓

### 1.2 【PASS】D5 的 P1-1（离屏绕过转换）已闭合
- `e2e1`（d6_acceptance_e2e.rs:26-37）：`view_with(Color(0,0,255))` → `from_view` → `render_offscreen` → 中心像素 `B>200/R<60/G<60` ——**真实 WidgetView→from_view→像素**端到端。✓
- `e2e3`（d6_acceptance_e2e.rs:53-69）：`Unit` 根 + 红/绿 2 子节点 → 像素中出现红色和绿色。✓
- `e2e4`（d6_acceptance_e2e.rs:72-79）：空视图 → 渲染无像素崩溃。✓
- `e2e_from_view.rs`（from_view_color_rect/from_view_text_props）：断言 Color→FillRect、Str→DrawText 指令生成。✓

### 1.3 【P2】文本仍用矩形近似
`vello.rs:187-198` 的 `DrawText` 编码**仍是用 `scene.fill` 画一个矩形块**（宽=charcount*size*0.6，高=size），**不是真实 cosmic-text 字形**。虽然 `DrawCmd::DrawText` 保留了 `text/size/color` 字段（scene_graph.rs:31-42），但**编码路径没把 text 转成字形**。详见 §四。

---

## 二、Q2 — 布局应用（P2，嵌套坐标叠加缺口）

### 2.1 【PASS 基本】compute_children 真实 bounds（单层正确）
- `LayoutEngine::compute_children`（layout/mod.rs:117-171）**是 D6 新增**：为每个子节点返回 `LayoutResult{size, position}`，position 取 taffy `l.location.x/y`（mod.rs:167）。D4 只有 `compute`（整树布局），D6 补了逐子节点 bounds。✓
- `from_view` 用它对子节点定位（scene_graph.rs:88）——**布局从"固定尺寸"升级为"真实位置"**。✓
- **固定尺寸残留检查**：子节点建议尺寸用 `c.size.unwrap_or_else(|| Size::new(100.0, 40.0))`（scene_graph.rs:83）——**仍有 100x40 兜底**，但仅当子节点未显式给 size 时用（可接受，P2；D7 组件会给 size）。根节点尺寸用 `view.size.unwrap_or(slot.size)`（scene_graph.rs:93）。**无硬编码固定 100x40 画图**（仅作子节点建议尺寸兜底）。✓

### 2.2 【P2-嵌套坐标叠加缺口】emit_node 未累加父偏移
- `emit_node` 中 `child_slots = compute_children(slot.size, ...)`，返回的子节点位置是**相对当前容器**（mod.rs:167 `l.location` 是相对此 tree 的根）。
- 递归时：`for (child, child_slot) in zip` → `emit_node(child, *child_slot)`（scene_graph.rs:117-118）。子节点绘制用 `slot.position.x`（scene_graph.rs:95/96/105/106），**但该 position 未叠加父节点 slot 的偏移**。
- **后果**：单层（父 slot=(0,0)）时子节点绝对位置 = compute_children 结果，正确。**但嵌套结构**（D7 组件含子组件的子组件）：孙节点的 slot.position 是相对中间父容器的，**未累加祖先偏移** → 嵌套组件会在错误坐标绘制。
- 我用隔离复刻验证了逻辑：`compute_children` 每次新建 TaffyTree，`l.location` 是相对该 tree 根（即相对当前容器），`emit_node` 无 `+ parent_slot.position` 累加 → **嵌套时坐标偏移丢失**。
- **定级：P2**（非 P0/P1）。当前 e2e3 只测**扁平**2 子节点（父 slot=(0,0)），未覆盖嵌套，故未暴露。**D7 组件（Accordion 含 AccordionItem 子组件）会踩中**——需在 D7 的 from_view 加"父偏移累加"（`child_slot.position + slot.position`）。

---

## 三、Q3 — Props 映射完整性（P2，注释与实现不符）

### 3.1 【PASS 已处理】Color→FillRect、Str→DrawText
`emit_node`（scene_graph.rs:91-114）处理了 Color 与 Str 两个可变图元前缀。✓

### 3.2 【P2-注释误写】`WidgetId` 在 PropValue 中不存在
- `scene_graph.rs:65` 注释写"`Unit/Bool/Int/Float/WidgetId → 无图元`"，**但 `PropValue`（view.rs:49-62）没有 `WidgetId` 变体**——只剩 6 个：`Unit/Bool(i64)/Float/Str/Color`。
- **这是注释与实现（类型定义）不一致**。`WidgetId` 在 view.rs 中未定义（`PropValue` 无此变体），只出现在 scene_graph.rs:65 注释和 lib.rs 的 `id` 模块导出。不影响编译/运行（注释仅文档），但**误导**——让读者以为 `PropValue` 支持 `WidgetId`。**P2**：修正注释（删 WidgetId 或改为实际存在的变体）。
- **PASS 部分**：`_ => {}`（scene_graph.rs:113）对 `Unit/Bool/Int/Float` **静默忽略**——这是合理的（这些不是绘制语义），且 `d6_acceptance_from_view.rs:35-38` 的 `fv0b` 已验证空视图无 cmd。✓

---

## 四、Q4 — 文本路径（P2，仍矩形近似）

### 4.1 现状：Str→DrawText 是"矩形近似块"
- `from_view` 把 `Str` 转成 `DrawCmd::DrawText{text, size, color}`（scene_graph.rs:102-112），字段真实。
- **但 `vello.rs:187-198` 编码 DrawText 时**：
  ```rust
  let width = text.chars().count() as f32 * *size * 0.6;
  let rect = kurbo::Rect::new(x, y, x+width, y+*size);
  scene.fill(Fill::NonZero, ..., &rect);  // 画的是一个矩形块，不是字形
  ```
  **`.fill` 矩形近似文本，未做 cosmic-text 字形整形**。`size` 字段（scene_graph.rs:108 传的是 `size.height`）在本路径中当"高度"用，`text` 字符数 × 0.6 估算宽——**这不是真实文本渲染**。

### 4.2 【重点】e2e2 文本测试无法区分真实字形 vs 矩形近似
- `e2e2`（d6_acceptance_e2e.rs:40-50）：`view_with(Str("Hello"))` → 断言 `has_non_bg`（任一像素 >20）。**矩形近似色块同样产生非背景像素**，所以该测试**既不能证明真实字形，也不能证明它是矩形近似**——它只证明"文本区域有非背景内容"。
- **定级：P2**（资源真实性/观感）。这与 `from_view` 层无关（from_view 正确产出 DrawText 指令），是**渲染编码层**（vello.rs）的文本未实现真实字形。**结论：** 文本路径"Str→DrawText 指令"已实现，但"DrawText→真实字形"未实现（仍矩形近似）。**够 D7 用吗？** —— 若 D7 只需"文本是可有像素的内容"（占位），够；若 D7 需"真实可读文本"，不够。**建议 D7 明确 text.rs/glyph.rs 的真实字形实现为依赖项**（cosmic-text 已在 render/Cargo.toml:16 optional，`vello-backend` feature 已含）。

---

## 五、Q5 — 防火墙 / 单一 vello（PASS）

- **core 零 GPU**：`rgui-core/Cargo.toml` grep `wgpu/vello/cosmic/fontdb/skrifa` → **无**（仅 taffy optional，纯 Rust）。core 源码 `grep wgpu/vello` → 仅注释（lib.rs:8）。✓
- **render 单一 vello**：`rgui-render/Cargo.toml` feature `vello-backend` = `{vello,wgpu,cosmic-text,fontdb,skrifa,pollster}`（Cargo.toml:24-31），**无 `skia` feature/`skia-safe`/`offscreen` 变体**；`RenderBackend`（vello.rs:15-18）仅 `Vello(VelloBackend)` 一态。✓
- **DAG 无环**：render 仅依赖 core（Cargo.toml:11），无反向。✓

**达标，无 P0/P1。**

---

## 六、Q6 — D7（窗口+事件循环）前需处理 + P2 观察项清单

### 6.1 无 P0/P1 阻塞 D7（P0 清零，P1 清零）
D5 的 P1-1/P1-2 已闭合（真实 e2e 链路 + from_view 真实转换）。**D7 放行无阻塞。**

### 6.2 P2 观察项（随 D7 处理，不阻塞放行）
| # | 项 | 位置 | 说明 |
|---|---|---|---|
| P2-1 | **嵌套布局坐标未累加父偏移** | scene_graph.rs:88/95/105 | 嵌套组件（D7 Accordion 含子项）会错位——emit_node 需 `child_slot.position + slot.position` |
| P2-2 | **文本仍矩形近似**（非真实字形） | vello.rs:187-198 | `DrawText` 用 `scene.fill` 矩形块；真实 cosmic-text 字形在 text.rs/glyph.rs 待实现；D7 若需真实可读文本必须接 |
| P2-3 | **e2e2 文本测试无法区分字形/近似** | d6_acceptance_e2e.rs:46-49 | `has_non_bg` 太宽，应改判"文本区存在像素"，或待真实字形后精确断言 |
| P2-4 | **offscreen.rs 手工 rect 残留**（旧 D5 测试） | rgui-render/tests/offscreen.rs | D5 的 `red_filled_rect` 测试仍在（`offscreen_renders_red_rect_to_pixels`），与新 e2e 并存——**非缺陷**，但它是"手工构造"路径，容易误导；建议 D6 后归档/标注为"演示用"，或删除（红色矩形演示已由 e2e1 真实转换替代） |
| P2-5 | **增量正向（改 core→render）仍不可达** | D4 遗留 | 改 core 数据层仍重编 render（render 依赖 core 整 crate）；D5/D6 未解决，需总监裁决"接受改 core 即重编 render"或数据层拆 crate |
| P2-6 | **注释误写 WidgetId** | scene_graph.rs:65 | PropValue 无 WidgetId 变体，注释与实现不符，应修正 |
| P2-7 | **`device.poll(Wait)` 阻塞** | vello.rs:140 | D7 接入 winit 事件循环每帧渲染时阻塞 UI 线程，需改 `poll(Maintain::Poll)` 或移出关键路径 |
| P2-8 | **子节点建议尺寸兜底 100x40** | scene_graph.rs:83 | D7 组件需显式给 size，否则一律 100x40（仅建议尺寸，可接受） |

---

## 七、P0/P1 风险清单 + MERGE GATE 建议

### P0 风险清单：**无（P0 清零）**

### P1 风险清单：**无（P1 清零）**

### MERGE GATE 建议：**放行（PASS）**

- **放行理由（充分）**：
  1. D5 的 P1-1（离屏绕过转换）与 P1-2（from_view 占位）**已闭合**——真实 e2e 像素测试（e2e1 蓝像素、e2e2 文本、e2e3 红绿布局、e2e4 空视图）+ from_view 指令断言（Color→FillRect、Str→DrawText）真实覆盖。
  2. **布局真正作用于渲染**：`compute_children` 返回真实 bounds，替换了 D5 的固定 100x40。
  3. **防火墙/单一 vello 保持**：core 零 GPU、render 无 skia 残留。
  4. 无 unsafe（仅老项目一处，新代码零 unsafe 已在 D5 确认，D6 未新增）。
  5. 契约一致性（greenfield B.1）自 D3 修复后保持。
- **P1 清零 → 放行 D7**。

### P2 观察项优先级（给 D7 的建议）
**D7 开工前建议优先**：P2-1（嵌套布局坐标累加）与 P2-2（文本真实字形）——因为 D7 是窗口+事件循环，会把真实"用户组件树"进入渲染，组件含子组件（嵌套）时 P2-1 会错位、P2-2 会使文本仍是色块。**但二者均为 P2，不阻塞 D7 放行**，可在 D7 内一并处理。

---

*审查方：devco-reviewer｜只读审查（未运行 GPU 测试，像素断言以 qa 环境依赖为准）；嵌套坐标叠加经隔离复刻逻辑验证，单层正确、嵌套待补。D6 的 from_view 真实转换达标，P0/P1 清零，建议放行 D7。*
