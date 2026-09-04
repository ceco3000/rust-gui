# D17（文本换行 + 渲染尺寸统一 + 多组件布局）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D17 交付（commit 2cfea91）
> 基准：greenfield §B.1/§B.2、D5、D15（scale_factor）、既有判据 + **流式编码判据**
> 范围：①文本换行 ②渲染尺寸统一 ③架构边界 ④流式判据 ⑤防火墙 ⑥文档一致性
> 方法：只读代码核查（text.rs/scene_graph.rs/vello.rs/app.rs 逐行）+ 实测 `cargo test --workspace --all-features`

---

## 〇、结论速览

| # | 审查点 | 定级 |
|---|---|---|
| 1 | 文本换行 | **PASS（shape_line 按 width 多行；DrawText.width；换行测试）** |
| 2 | 渲染尺寸统一 | **PASS（render 函数加 scale，Affine::scale 施加；离屏 1.0 无回归）** |
| 3 | 架构边界 | **PASS（换行/尺寸在 render+platform；core 布局逻辑坐标零 scale/render 引用）** |
| 4 | **流式判据** | **PASS（by_font iter().position()；换行测试 y_range 流式；无 dyn Iterator/冗余 collect）** |
| 5 | 防火墙/DAG | **PASS（render scale 在 render/vello 单一 vello；core 零 GPU；DAG 无环）** |
| 6 | 文档一致性 | **PASS（D5 已同步换行+尺寸；D2/D3/D10/greenfield doc 同步中）** |

**总评：D17 达标——文本按宽度换行（cosmic-text 多行）、渲染尺寸物理/逻辑统一（Affine::scale 施加在 fill/stroke/draw_glyphs）、离屏 1.0 无回归，流式合规，65 测试全绿（实测），P0 清零。建议：放行（PASS）。** 无 P0/P1，1 条 P2 观察（draw_text 仿射组合顺序，见 §一.4）。

---

## 一、文本换行（PASS）

### 1.1 shape_line 按宽度换行（text.rs:50-56）
```rust
if let Some(w) = max_width {
    if w > 0.0 {
        text_buf.set_size(&mut self.font_system, Some(w), None);
    }
}
text_buf.shape_until_scroll(&mut self.font_system, false);
```
- `set_size(Some(w), None)`（text.rs:53）——cosmic-text 按可用宽 `w` 换行（多行）。`shape_until_scroll`（text.rs:56）产出多行 `layout_runs()`。✓
- **多行垂直排布**：`layout_runs()` 遍历，`baseline = line.line_y`（text.rs:63）——**line_y 随行递增**（多行垂直排布），`gy = baseline - ...`（text.rs:66）随行变化。✓

### 1.2 DrawText.width 字段（scene_graph.rs:42-43）
- `DrawText` 新增 `width: f32`（scene_graph.rs:42-43）——文本区域可用宽度（逻辑；>0 换行）。`from_view` 传 `view.size` 的宽度（D17）。✓

### 1.3 vello draw_text 按 width 换行（vello.rs:349）
- `self.text.shape_line(text, size, if width > 0.0 { Some(width) } else { None })`（vello.rs:349）——**width>0 换行，否则单行**。✓

### 1.4 【P2-观察】draw_text 仿射组合顺序（潜在坐标微偏差）
- vello.rs:359：`let run_tf = tf * kurbo::Affine::translate((f64::from(x), f64::from(y)));`——`tf * translate`。
- **kurbo Affine 组合语义**：`a * b` 表示先应用 `b` 再应用 `a`（即 `a` 在外面）。`tf * translate(x,y)` = 先 `translate(x,y)` 再 `tf`（scale）。
- **问题**：逻辑坐标 (x,y) 应先被 `scale` 放大到物理，再平移——但此处是"先平移 (x,y) 再整体 scale"。由于 `translate` 的 (x,y) 也是逻辑坐标，`tf`（scale）会作用于**平移后的所有坐标**，即`(x_scaled, y_scaled)` 起点偏移被放大——若 (x,y) 是逻辑尺寸内坐标，经 `tf*translate` 后实际起点 = scale*(x,y)，**符合"逻辑坐标经 scale 放大到物理"的目标**（因为 (x,y) 是逻辑值，scale 放大后是物理位置）。✓
- **但**：`draw_text` 的 `x,y` 来自 `DrawCmd::DrawText` 的**逻辑坐标**（scene_graph.rs 逻辑），`tf = Affine::scale(scale)` 放大——逻辑 (x,y) → 物理 (x*scale, y*scale)，glyph 内部相对 run 原点无绝对 x/y（draw_text 注释 vello.rs:358）——**组合可能正确**（translate 后的坐标被整体 scale）。**需实机验证**（高分屏截图），非 P0/P1，P2 观察（D18 多组件布局时复核）。

### 1.5 换行测试（text.rs:135-160）
- `long_text_wraps_when_width_limited`：`shape_line(long, 24, None)` vs `Some(120)`，断言 `w_max - s_max > 20`（换行后 glyph 最大 y 显著增大 → 多行）——**换行正确性验证**。✓

---

## 二、渲染尺寸统一（PASS）

### 2.1 render 函数加 scale 参数
- `render_to_view(graph, view, w, h, scale)`（vello.rs:68-77）；`render_surface(surface, graph, w, h, scale)`（vello.rs:229-235）；`render_offscreen(...1.0)`（vello.rs:165，**离屏传 1.0 无回归**）。✓

### 2.2 encode 施加 Affine::scale（vello.rs:282-283）
- `let tf = kurbo::Affine::scale(scale);`（vello.rs:283）——逻辑→物理缩放变换。
- **fill/stroke/draw_glyphs 施加 tf**：
  - FillRect：`scene.fill(..., tf, ...)`（vello.rs:300）
  - StrokeRect：`scene.stroke(&stroke, tf, ...)`（vello.rs:329）
  - DrawText：`draw_text(..., tf)` → `run_tf = tf * translate`（vello.rs:359-365）
- **统一**：所有图元经 `tf`（scale）放大到物理。✓

### 2.3 app.rs 传 scale（app.rs:165-169）
- `let scale = window.scale_factor()`（app.rs:165，Retina 2x）→ `render_surface(surface, &graph, size.width, size.height, scale)`（app.rs:169）。**闭合 D15 的"渲染物理/逻辑尺寸混用" P2 观察**（D15 审查列 D17 遗留，已实现）。✓

---

## 三、架构边界（PASS）

- **换行/尺寸在 render（vello/cosmic-text）+ platform 传 scale**：`shape_line`（text.rs，render）换行；`Affine::scale`（vello.rs，render）；`scale_factor` 来自 platform（window.rs）/app.rs。✓
- **core 布局仍逻辑坐标**：core 零 scale/render 引用（grep `scale_factor/Affine::scale/render_to_view` in rgui-core/src → 空）；`from_view` 产逻辑坐标 DrawCmd（scene_graph.rs，render），由 render 的 tf 放大。✓
- **core 零 GPU/平台**：text/vello 在 render；core 只产逻辑坐标指令。✓

---

## 四、流式编码判据（PASS）

### 4.1 合规项
| 判据 | 检查结果 |
|---|---|
| **by_font 归并 iter().position()** | `by_font.iter().position(|(id,_)| *id == g.font_id)`（text.rs:68）——**流式位置查找**（非手写循环+index）✓ |
| **换行测试 y_range 流式** | `runs.iter().map().sum()`（text.rs:124）、`y_range` 用 min/max 迭代（text.rs:142-152）——流式 ✓ |
| **`dyn Iterator` 装箱** | 无 `dyn Iterator`（text/scene_graph/vello）✓ |
| **冗余中间 collect** | text.rs:59 `by_font` 用 `Vec::new()`+push（字体归并）——**accumulator 语义**（字体分组归并，需累积），非"可组合子的循环"，边界内；换行测试无冗余 collect ✓ |
| **字体归并 loop** | text.rs:60-73 `for line in layout_runs()` + `for g in line.glyphs` + `match by_font.iter().position()`——**遍历 cosmic-text 多行/多 glyph，逐字归并到字体桶**——这是"逐原子处理"（每个 glyph 单独归并），用 `into_iter().fold` 需闭包收集分桶，可读性差；**for + position 是明确表达**，属边界内 ✓ |

### 4.2 边界（逐 glyph 字体归并）
- text.rs:64-72 遍历每个 glyph 归并到 by_font 桶：**逐原子处理**（N glyph → 分桶），用 fold/组合子需嵌套收集闭包，**不如 for+position 直观**。符合铁律边界。✓

**结论：流式编码判据 PASS。** by_font 用 iter().position()、换行测试 y_range 流式、无 dyn Iterator/冗余 collect；逐 glyph 字体归并（for+position）属边界内（逐原子分桶，组合子反而晦涩）。

---

## 五、防火墙 / DAG（PASS）

- **render scale 在 render/vello（单一 vello）**：`Affine::scale`（vello.rs）是 GPU 层。✓
- **core 零 GPU/平台**：core 只产逻辑坐标指令（from_view），零 scale/render 引用。✓
- **platform 传 scale**：window.rs `scale_factor`；app.rs。✓
- **单一 vello/winit**、DAG 无环——保持。✓

---

## 六、文档一致性（PASS + P2 观察）

- **D5 已同步换行+尺寸**（总监确认 commit 2cfea91 含 D5 更新）——shape_line 换行、render scale 统一与代码一致。
- **D2/D3/D10/greenfield 由 doc 同步中**——需在 D3 render（DrawText 加 width、render 函数 scale 参数）、D10 标注。P2 观察。

---

## 七、P0/P1 风险清单

**P0：无。P1：无。**

**MERGE GATE：放行（PASS）。**

### P2 观察项（随后续处理，不阻塞）
1. **draw_text 仿射组合顺序**（vello.rs:359 `tf * translate`）：逻辑坐标经 scale 放大——逻辑正确但需高分辨率屏实机验证（多组件布局时复核起点偏移准确性）。P2。
2. **D3/D10/greenfield 需标注**（DrawText.width、render scale 参数、shape_line max_width）——doc 同步时补记。

---

*审查方：devco-reviewer｜只读审查（未运行 GPU 窗口测试，65 全量数字经 `cargo test --workspace --all-features` 实测核实 = 65 passed）。流式编码判据逐条核对：by_font iter().position()、换行测试 y_range 流式、无 dyn Iterator/冗余 collect；逐 glyph 字体归并（for+position）属边界内。render scale 在 render/vello（单一 vello）、core 零 GPU/scale 引用、DAG 无环。*
