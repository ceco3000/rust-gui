# D16（StrokeRect 描边边框 + WidgetView.border 获焦描边）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D16 交付（commit b3ab6eb）
> 基准：greenfield §B.1/§B.2、D5、既有判据 + **流式编码判据**
> 范围：①StrokeRect 图元 ②WidgetView.border ③架构边界 ④流式判据 ⑤防火墙 ⑥文档一致性
> 方法：只读代码核查（scene_graph.rs/view.rs/components.rs/vello.rs 逐行）+ 实测 `cargo test --workspace --all-features`

---

## 〇、结论速览

| # | 审查点 | 定级 |
|---|---|---|
| 1 | StrokeRect 图元 | **PASS（新枚举向后兼容；from_view 识别 border；vello stroke 正确）** |
| 2 | WidgetView.border | **PASS（Border + Default None；map_message 透传）** |
| 3 | 架构边界 | **PASS（描边 core view 的 border → StrokeRect → render stroke；焦点隔离 core）** |
| 4 | **流式判据** | **PASS（border 测试 iter().any()；from_view border if-let；无 dyn Iterator/冗余 collect）** |
| 5 | 防火墙/DAG | **PASS（StrokeRect 在 render/vello；WidgetView.border 在 core 纯类型；core 零 GPU）** |
| 6 | 文档一致性 | **PASS（D5 已同步真描边；D3/D10/greenfield doc 同步中）** |

**总评：D16 达标——StrokeRect 描边边框（用户 D14 遗留"真描边"已实现）、WidgetView.border 获焦描边高亮、from_view 识 border→vello stroke 绘制全链路正确，流式合规，64 测试全绿（实测），P0 清零。建议：放行（PASS）。** 无 P0/P1，2 条 P2 观察（见 §六）。

---

## 一、StrokeRect 图元（PASS）

### 1.1 新枚举（scene_graph.rs:44-57，向后兼容）
- `DrawCmd` 新增 `StrokeRect { x, y, width, height, color, stroke_width }`（scene_graph.rs:44-57）——**追加新变体**，`FillRect`/`DrawText` 未变，`#[derive(Debug, Clone, PartialEq)]`（scene_graph.rs:15）保持。**向后兼容**（既有消费方仍可 match）。✓

### 1.2 from_view 识别 border（scene_graph.rs:137-148）
```rust
if let Some(b) = &view.border {
    let pad = 2.0;
    let size = view.size.unwrap_or(slot.size);
    self.cmds.push(DrawCmd::StrokeRect {
        x: slot.position.x as f32 - pad,
        y: slot.position.y as f32 - pad,
        width: size.width + 2.0 * pad,
        height: size.height + 2.0 * pad,
        color: b.color,
        stroke_width: b.width,
    });
}
```
- `if let Some(b) = &view.border`——**流式 `if-let`**，仅在有 border 时 push StrokeRect。✓
- **外扩 2px pad**（scene_graph.rs:138）：边框外扩 2px，边缘不被内容覆盖（scene_graph.rs:136 注释）——绘制正确。✓

### 1.3 vello stroke 绘制（vello.rs:305-322）
```rust
let rect = kurbo::Rect::new(x, y, x+width, y+height);
let stroke = kurbo::Stroke::new(f64::from(*stroke_width));
let brush = Brush::Solid(to_peniko_color(*color));
scene.stroke(&stroke, kurbo::Affine::IDENTITY, brush, None, &rect);
```
- **`scene.stroke(...)`**（vello.rs:322）——vello **真实描边**（非 fill 模拟），`kurbo::Stroke::new(stroke_width)`（vello.rs:320）设置描边宽度。✓

---

## 二、WidgetView.border（PASS）

### 2.1 Border 类型（view.rs:9-23）
- `Border { color: Color, width: f32 }`（view.rs:11-16），`Border::new`（view.rs:20-22）——纯值类型。✓

### 2.2 WidgetView.border 字段（view.rs:37，向后兼容）
- `pub border: Option<Border>`（view.rs:37），`Default` 派生 `border: None`（view.rs:77）——**默认无边框，向后兼容**（既有 WidgetView 构造不破坏）。✓

### 2.3 map_message 透传（view.rs:65）
- `map_message`（view.rs:56-68）children 递归 + `props/size/border` 透传（view.rs:63-65）——**border 随 map_message 保持不变**（组合根提升消息类型时边框保留）。✓

### 2.4 获焦描边高亮（components.rs）
- Accordion：`root.border = if ctx.focused { Some(Border::new(Color::rgb(255,230,80), 3.0)) }`（components.rs:89-90）——**获焦描边**（亮黄 3px 外框）。
- WaBadge：`root.border = if ctx.focused { Some(Border::new(...)) }`（components.rs:208-209）。
- **D14 背景高亮 + D16 描边边框**：获焦组件现为"背景变亮 + 描边外框"双重视觉。✓

---

## 三、架构边界（PASS）

### 3.1 焦点隔离 core（关键）
- 描边是 **core 组件 view 的 `WidgetView.border`**（components.rs 读 `ctx.focused` 设 border）——焦点逻辑在 core 组件 view 层。✓
- `border` → `from_view`（render scene_graph.rs）→ `DrawCmd::StrokeRect` → render vello `scene.stroke`——**render 只收到 StrokeRect（颜色/宽度），不知晓"焦点"概念**。焦点完全隔离 core。✓
- **验证**：render（vello.rs:305-322）只消费 StrokeRect 的 color/stroke_width，无焦点状态。✓

### 3.2 无需 GPU/paint 侵入
- 描边通过视图的 `border` 字段（纯数据）→ from_view → StrokeRect 指令——**不需要 paint 层/GPU 状态**。✓

---

## 四、流式编码判据（PASS）

### 4.1 合规项
| 判据 | 检查结果 |
|---|---|
| **border 测试 iterator().any()** | `scene.cmds().iter().any(|c| matches!(c, DrawCmd::StrokeRect{..}))`（glyph_offscreen.rs:165-169）——**流式组合子** ✓ |
| **from_view border if-let** | `if let Some(b) = &view.border`（scene_graph.rs:137）——**if-let 模式匹配**，非迭代循环 ✓ |
| **`dyn Iterator` 装箱** | 无 `dyn Iterator`（scene_graph/view）✓ |
| **冗余中间 collect** | scene_graph.rs:104 `.collect()`（child_sizes，必要）；view.rs:62 `.collect()`（map_message 收集，必要）——均非冗余 ✓ |
| **递归遍历 for-zip** | scene_graph.rs:151 `for (child, child_slot) in zip`（递归遍历子节点）——**DFS 树遍历**，非可替代的迭代器组合（递归天然需 for+递归），边界内 ✓ |

### 4.2 边界（树递归遍历）
- `emit_node` 递归（scene_graph.rs:151-160）：子节点递归是**树遍历**，用 `for ... zip` + 递归是**明确表达**，流式化反而晦涩（嵌套回调/递归组合子不直观）。**符合铁律边界"复杂场景手写循环可接受"**。✓

**结论：流式编码判据 PASS。** border 测试 iter().any()、from_view if-let、无 dyn Iterator/冗余 collect；递归树遍历（for-zip）属边界内。

---

## 五、防火墙 / DAG（PASS）

- **StrokeRect 在 render（vello.rs）**：stroke 是 GPU 绘制（vello 的 scene.stroke）——GPU 层。✓
- **WidgetView.border 在 core（view.rs 纯类型）**：`Border`/`border: Option<Border>` 是 core 纯 Rust 类型，零 GPU/平台。✓
- **core 零 GPU/平台**：view.rs/components.rs 无 wgpu/vello/winit。✓
- **单一 vello/winit**、DAG 无环——保持。✓

---

## 六、文档一致性（PASS + 2 P2 观察）

- **D5 已同步真描边**（总监确认 commit b3ab6eb 含 D5 更新）——StrokeRect 描边边框 + D14 背景高亮 + D16 描边（用户 D14 遗留"真描边 StrokeRect 留后续"已在 D16 实现）。
- **D3/D10/greenfield 由 doc 同步中**——需在 D3 render（DrawCmd 含 StrokeRect）、D10 组件规范（获焦描边）标注。
- **P2 观察**：① DrawCmd 枚举新增 StrokeRect（D3 §2 需补）；② WidgetView 新增 border 字段（greenfield §B.1 / D0 §5 WidgetView 定义需补 border: Option<Border>）——doc 同步时标注。

---

## 七、P0/P1 风险清单

**P0：无。P1：无。**

**MERGE GATE：放行（PASS）。**

### P2 观察项（随后续处理，不阻塞）
1. **D3/D10/greenfield 需补 StrokeRect/border 标注**：DrawCmd + StrokeRect（D3 §2）、WidgetView.border（D0 §5/greenfield §B.1）、获焦描边（D10）——doc 同步时补记。
2. **描边 pad 硬编码 2.0**（scene_graph.rs:138）：外扩 2px 是固定值，后续样式系统（D19）可参数化。

---

*审查方：devco-reviewer｜只读审查（未运行 GPU 窗口测试，64 全量数字经 `cargo test --workspace --all-features` 实测核实 = 64 passed）。流式编码判据逐条核对：border 测试 iter().any()、from_view if-let、无 dyn Iterator/冗余 collect；递归树遍历（for-zip）属边界内。StrokeRect 在 render（vello stroke）、WidgetView.border 在 core（纯类型），防火墙/DAG 达标。*
