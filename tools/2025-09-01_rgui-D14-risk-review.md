# D14（获焦高亮升级为背景变亮）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D14 交付（commit 629fdeb）
> 基准：greenfield §B.1、D5、既有判据 + **流式编码判据**
> 范围：①获焦背景高亮 ②架构边界 ③流式判据 ④防火墙 ⑤文档一致性
> 方法：只读代码核查（components.rs/d10_components.rs 逐行）+ 实测 `cargo test --workspace --all-features`

---

## 〇、结论速览

| # | 审查点 | 定级 |
|---|---|---|
| 1 | 获焦背景高亮 | **PASS（ViewContext.focused → Color 条件，替代 D13 ▶ 前缀）** |
| 2 | 架构边界 | **PASS（焦点高亮是 core 组件 view 的 Color 指令，render 只渲染颜色）** |
| 3 | **流式判据** | **PASS（Color 用 if/else 表达式，非迭代循环；无装箱/冗余 collect）** |
| 4 | 防火墙/DAG | **PASS（焦点高亮在 core 组件 view 零 GPU；render 只渲染）** |
| 5 | 文档一致性 | **PASS（D5 已同步背景高亮 + 描边留后续）** |

**总评：D14 达标——获焦高亮从 D13 的 ▶ 文本前缀升级为**背景变亮高亮**（用户已接受），ViewContext.focused → 组件 root/header Color 条件，render 只渲染颜色。流式合规，60 测试全绿（实测），P0 清零。建议：放行（PASS）。** 无 P0/P1，1 条 P2（描边边框 StrokeRect 为 D5 未实现项，留后续）。

---

## 一、获焦背景高亮（PASS）

### 1.1 Accordion（components.rs:90-96）
```rust
let header_color = if ctx.focused {
    Color::rgb(140, 185, 255)   // 获焦：高亮（亮蓝）
} else {
    Color::rgb(90, 130, 220)    // 未获焦：普通深蓝
};
let mut header = WidgetView::empty();
header.props = PropValue::Color(header_color);
```
- **ViewContext.focused → header Color 条件**：获焦时 header 背景变亮（140,185,255 vs 90,130,220），**背景色即获焦指示**。✓
- **替代 D13 ▶ 前缀**（D13 用 Str "▶ " 前缀，D14 改为背景色差）——**用户已接受的背景变亮方案**。✓

### 1.2 WaBadge（components.rs:205-209）
```rust
root.props = PropValue::Color(if ctx.focused {
    Color::rgb(170, 210, 255)   // 获焦：高亮
} else {
    Color::rgb(120, 160, 210)   // 未获焦：普通
});
```
- 同样背景变亮高亮。✓

### 1.3 测试覆盖（背景色精确断言）
- `accordion_view_background_highlights_when_focused`（d10_components.rs:113-123）：未获焦断 `Color::rgb(90,130,220)`（深蓝）+ 非 `rgb(140,185,255)`；获焦断 `rgb(140,185,255)`（高亮）。✓
- `badge_view_background_highlights_when_focused`（d10_components.rs:126-134）：未获焦 `rgb(120,160,210)` / 获焦 `rgb(170,210,255)`。✓
- `contains_color`/`contains_color_badge`（d10_components.rs:19-26）辅助断言 props==Color——**精确比较背景色**。✓
- **覆盖**：两个组件获焦/未获焦背景色**精确断言**，非模糊比较。✓

---

## 二、架构边界（PASS）

### 2.1 焦点高亮是 core 组件 view 的 Color 指令（render 只渲染颜色）
- 焦点高亮通过 `PropValue::Color(if ctx.focused {...} else {...})`（components.rs:96/205-209）——高亮是**组件的 Color props**。
- 该 Color 经 `SceneGraph::from_view` → `DrawCmd::FillRect(color)` → render（vello）渲染。**焦点逻辑完全在 core 组件 view 层（读 ctx.focused 决定颜色），render 只收到一个 Color 值，不知晓"焦点/高亮"概念**。✓
- **验证**：render（vello.rs）的 `DrawCmd::FillRect` 只拿 color，无任何焦点状态——焦点完全隔离在 core。符合"core 零 GPU、render 只渲染 draw 指令"。✓

### 2.2 无需 paint 层/GPU 侵入
- 焦点高亮**不需要** `paint` 层或 GPU 状态——就是视图的 Color props。组件 `paint` 仍为空（components.rs:127/227）。✓
- 相比 D13 的 ▶ 前缀（要字形渲染 ▶），D14 的**背景色高亮更轻量**（纯 Color，无字形依赖）。✓

---

## 三、流式编码判据（PASS）

### 3.1 合规项
| 判据 | 检查结果 |
|---|---|
| **Color 条件（if/else 而非迭代循环）** | `if ctx.focused { Color::... } else { Color::... }`——**if/else 标量表达式**，非迭代循环，纯值选择 ✓ |
| **`dyn Iterator` 装箱** | 组件/context 无 `dyn Iterator`/`Box<dyn Iterator>` ✓ |
| **冗余中间 collect** | 无冗余 collect ✓ |
| **组件 view 手写循环** | 无迭代器循环（仅 children.push 向量建树，D11 已判边界内）✓ |

### 3.2 组件 view push（边界内，接受）
- Accordion/WaBadge view 用 `root.children.push(...)`（components.rs:103, 110, 213）——**声明式视图树向量构造**（D11/D12/D13 已判为合法，边界内）。接受。✓

**结论：流式编码判据 PASS。** Color 高亮用 if/else 表达式（最直接的标量选择），无迭代循环/装箱/冗余 collect。

---

## 四、防火墙 / DAG（PASS）

- **焦点高亮在 core 组件 view**：Accordion/WaBadge 的 view（components.rs）读 `ctx.focused` 决定 Color——纯 core 逻辑，**零 GPU/平台**（只用 core context/traits/view 类型）。✓
- **render 只渲染**：render（vello.rs）只消费 `DrawCmd::FillRect`（color），不知晓焦点。✓
- **core 零 GPU**：components.rs 无 wgpu/vello/winit。✓
- **单一 vello/winit**、DAG 无环——保持。✓

---

## 五、文档一致性（PASS）

- **D5 已同步**：背景高亮（components.rs 高亮色）+ 描边边框 StrokeRect 留后续（D5 未实现项）——与代码一致（当前为背景高亮，无描边）。
- **D1/D10/greenfield 由 doc 同步中**——需注意：获焦高亮实现方式从 D13 ▶ 前缀改为背景色，D1/D10 组件规范描述应同步（现为"获焦背景变亮"）。
- **P2 观察**：描边边框（StrokeRect）为 D5 未实现项，留后续增强——已在 D5 标注，合理。

---

## 六、P0/P1 风险清单

**P0：无。P1：无。**

**MERGE GATE：放行（PASS）。**

### P2 观察项（随后续处理，不阻塞）
1. **描边边框 StrokeRect 为 D5 未实现项**：当前背景变亮高亮（用户已接受），真描边边框后续增强——已在 D5 标注，合理，非缺陷。
2. **高亮色为硬编码字面**（components.rs:90-93/205-209 的 `Color::rgb(...)`）：无样式系统（D4 样式 P1），当前硬编码可接受；后续接入样式/主题时替换为样式驱动。P2（D 系列样式实现时处理）。

---

*审查方：devco-reviewer｜只读审查（未运行 GPU 窗口测试，60 全量数字经 `cargo test --workspace --all-features` 实测核实 = 60 passed）。流式编码判据逐条核对：Color 用 if/else 标量表达式（非迭代循环）、无 dyn Iterator/冗余 collect；焦点高亮是 core 组件 view 的 Color 指令（render 只渲染颜色），防火墙/DAG 达标。*
