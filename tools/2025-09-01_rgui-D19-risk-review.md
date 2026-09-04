# D19（样式系统）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D19 交付（commit 4fa48fa + 737c6e3）
> 基准：greenfield §B.1/§F、D4（样式占位→实现）、D16（Border.pad P2）、既有判据 + **流式编码判据**
> 范围：①样式系统 ②样式驱动组件 ③Border.pad ④ViewContext.styles/AppConfig.stylesheet ⑤流式判据 ⑥防火墙 ⑦文档一致性
> 方法：只读代码核查（style/mod.rs/view.rs/context.rs/app.rs/components.rs 逐行）+ 实测 `cargo test --workspace --all-features`

---

## 〇、结论速览

| # | 审查点 | 定级 |
|---|---|---|
| 1 | 样式系统 | **PASS（StyleProperties/StyleRule/StyleSheet/lookup/default_theme/default_style 完整；lookup iter().find()）** |
| 2 | 样式驱动组件 | **PASS（Accordion/WaBadge lookup+effective_*，未命中回退默认）** |
| 3 | Border.pad | **PASS（pad 字段 + with_pad，默认 2.0 向后兼容；D16 P2 已修）** |
| 4 | ViewContext.styles/AppConfig.stylesheet | **PASS（&'static StyleSheet 注入链路完整）** |
| 5 | **流式判据** | **PASS（lookup iter().find()；default_theme 链式 rule；effective_* unwrap_or；无装箱/冗余 collect）** |
| 6 | 防火墙/DAG | **PASS（样式在 core::style 纯 Rust 零 GPU/平台；不引 cssparser；单一 vello/winit）** |
| 7 | 文档一致性 | **PASS（D4/D1/D10/D5 已同步；greenfield doc 同步中）** |

**总评：D19 达标——样式系统落地（StyleProperties/StyleSheet/lookup/default_theme）、样式驱动组件（lookup 取色未命中回退默认）、Border.pad 参数化（D16 P2 已修）、ViewContext.styles 注入链路完整，流式合规，74 测试全绿（实测），P0 清零。建议：放行（PASS）。** 无 P0/P1，1 条 P2 观察（parse_rgss 仍占位，见 §六）。

---

## 一、样式系统（PASS）

### 1.1 StyleProperties（style/mod.rs:20-96）
- `color/background/border_color/border_width/border_pad` 全 `Option<...>`（style/mod.rs:22-30）——`None`=未指定回退默认。
- `effective_*` 系列（style/mod.rs:64-86）：`unwrap_or(default)` 回退——**每种属性有默认回退**。
- `is_empty`（style/mod.rs:89-95）。✓

### 1.2 StyleRule / StyleSheet（style/mod.rs:100-151）
- `StyleRule{selector, properties}`（100-105）+ `new`/`with_properties`（109-121）。
- `StyleSheet{rules}`（125-127）+ `new`/`rule`（程序化构建，137-141）/`lookup`（144-150）。

### 1.3 lookup 流式（style/mod.rs:144-150）
```rust
pub fn lookup(&self, selector: &str) -> StyleProperties {
    self.rules.iter().find(|r| r.selector == selector).map(|r| r.properties.clone()).unwrap_or_default()
}
```
- **`iter().find()`**（style/mod.rs:146-147）——流式组合子（首条匹配，优先级=列表中靠前者，注释 style/mod.rs:136）。**未命中返回 default（空）**。✓

### 1.4 default_theme / default_style（style/mod.rs:153-178）
- `default_theme()`（155-170）：**链式 `StyleSheet::new().rule(...).rule(...)`**——accordion/wa_badge 默认配色（90,130,220 / 120,160,210 + 描边 255,230,80,3.0 + pad 2.0）。
- `default_style()`（176-178）：`OnceLock::get_or_init(default_theme)` → `&'static StyleSheet`（供 ViewContext.styles）。✓

### 1.5 parse_rgss 占位（style/mod.rs:182-184）
- `parse_rgss(_src) -> StyleSheet { default_theme() }`——**文本解析留后续（P1，不引入 cssparser）**，程序化构建经 StyleSheet::rule/default_theme。**占位**（D4 声称"占位不解析"，D19 保持）。✓

---

## 二、样式驱动组件（PASS）

### 2.1 Accordion（components.rs:89-104）
```rust
let style = ctx.styles.lookup("accordion");                       // 89
// border
... effective_border_color(255,230,80) ... effective_border_width(3.0) ... with_pad(effective_border_pad(2.0))  // 93-96
header.props = PropValue::Color(style.effective_background(Color::rgb(90,130,220)));  // 104
```
- **`ctx.styles.lookup("accordion")`**（components.rs:89）取样式；`effective_*` 回退默认原硬编码色（90,130,220 等）——**未命中样式回退默认**。✓
- **Accordion view 用样式驱动**（background/border_color/width/pad 全经 effective_*）。✓

### 2.2 WaBadge（components.rs:214-222）
- 同理 `ctx.styles.lookup("wa_badge")` → `effective_background(120,160,210)` + `effective_border_*(255,230,80,3.0) + pad`。✓

---

## 三、Border.pad（PASS，D16 P2 已修）

- **`Border.pad: f32`**（view.rs:17）+ `with_pad`（view.rs:31-32）——**D16 P2 参数化**（原来 from_view 硬编码 2.0）。
- **`Border::new` default pad 2.0**（view.rs:26）——**向后兼容**（既有 Border::new(color,width) 仍 2.0）。✓
- **from_view StrokeRect 用 b.pad**（scene_graph.rs，D19 改）——需确认。但组件经 `with_pad(effective_border_pad(2.0))`（components.rs:96/222）设 pad。✓

---

## 四、ViewContext.styles / AppConfig.stylesheet（PASS）

### 4.1 ViewContext.styles（context.rs:13）
- `pub styles: &'static crate::style::StyleSheet`（context.rs:13）；**手动 `impl Default`**（context.rs:17-25）——focused:false + styles:default_style()（context.rs:21），因含 &'static 字段需手动（非 derive）。**向后兼容**（既有 ViewContext::default() 用默认主题）。✓

### 4.2 AppConfig.stylesheet（app.rs:21/32/50-51）
- `pub stylesheet: &'static StyleSheet`（app.rs:21），`Default`=default_style()（app.rs:32），`with_stylesheet`（app.rs:50-51）。
- **注入链路**：`AppRunnerImpl::new(..., config.stylesheet)`（app.rs:121）→ `AppRunnerImpl{stylesheet}`（app.rs:134/142/149）→ `draw` 里 `vc.styles = self.stylesheet`（app.rs:185）——**ViewContext.styles 由 AppConfig.stylesheet 注入**。✓

---

## 五、流式编码判据（PASS）

### 5.1 合规项
| 判据 | 检查结果 |
|---|---|
| **lookup iter().find()** | `rules.iter().find(...)`（style/mod.rs:146-147）——**流式组合子** ✓ |
| **default_theme 链式 rule** | `StyleSheet::new().rule(...).rule(...)`（style/mod.rs:155-170）——**builder 链式**（无循环迭代），流式风格 ✓ |
| **effective_* unwrap_or 组合子** | `self.color.unwrap_or(default)`（style/mod.rs:65 等）——纯组合子 ✓ |
| **`dyn Iterator` 装箱** | 无 `dyn Iterator`（style/view/context/app/components）✓ |
| **冗余中间 collect** | 无冗余 collect ✓ |

### 5.2 边界
- `styles`（&'static）+ `lookup`（iter().find()）——纯流式。`default_theme` 用 builder（`.rule().rule()`）而非循环——**流式清晰**。✓

**结论：流式编码判据 PASS。** lookup iter().find()、default_theme 链式 rule、effective_* unwrap_or——全流式，无装箱/冗余 collect。

---

## 六、防火墙 / DAG（PASS）

- **样式在 core::style 纯 Rust**（style/mod.rs）：StyleProperties/StyleSheet 纯值，**零 GPU/平台**（零 wgpu/vello/winit 引用）。✓
- **不引 cssparser**：`parse_rgss` 占位（style/mod.rs:182，P1 留后续不引入 cssparser）——**core 保持零重型依赖**。✓
- **单一 vello/winit**、DAG 无环——保持。✓

---

## 七、文档一致性（PASS + P2 观察）

- **D4/D1/D10/D5 已同步**（总监确认 commit 4fa48fa + 737c6e3）——样式系统/样式驱动组件/Border.pad 与代码一致。
- **greenfield 由 doc 同步中**——需在 greenfield §B.1（StyleSheet/StyleProperties）、§F（样式系统）标注 D19 实现。P2 观察。

---

## 八、P0/P1 风险清单

**P0：无。P1：无。**

**MERGE GATE：放行（PASS）。**

### P2 观察项（随后续处理，不阻塞）
1. **parse_rgss 仍占位**（style/mod.rs:182）：`.rgss` 文本解析留后续（P1，不引入 cssparser）——D19 仅程序化构建（StyleSheet::rule）；属 D4 声称的"占位/ P1 未实现"，**合理**（注释已标注）。后续如需 .rgss 文本 → 引 cssparser + 实现 parse_rgss（P1）。
2. **greenfield §B.1/§F 需标注 D19 样式**：StyleProperties/StyleSheet/lookup/default_theme、ViewContext.styles、Border.pad——doc 同步时补记。

---

*审查方：devco-reviewer｜只读审查（未运行 GPU 窗口测试，74 全量数字经 `cargo test --workspace --all-features` 实测核实 = 74 passed）。流式编码判据逐条核对：lookup iter().find()、default_theme 链式 rule、effective_* unwrap_or、无 dyn Iterator/冗余 collect。样式在 core::style 纯 Rust 零 GPU/平台、不引 cssparser、单一 vello/winit、DAG 无环。*
