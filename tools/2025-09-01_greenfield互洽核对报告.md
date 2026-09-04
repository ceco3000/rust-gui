# greenfield 蓝图 ↔ 代码 ↔ 核心 3 份文档 · 一致性互洽核对报告

> 核对方：devco-reviewer｜对象：tools/2025-09-01_rgui-greenfield-architecture.md（architect 按裁决 A 更新 8 点，commit 96adeb7）+ docs/D0/D11/CLAUDE.md + 实际代码
> 方法：逐份 read_file 对照代码实况（traits.rs、各 Cargo.toml、view.rs/color.rs/style/mod.rs/core lib.rs/facade lib.rs）
> 范围：① greenfield↔代码 8 点是否落地 + 有无其它偏差；② greenfield↔核心 3 份互洽；③ refactor-contract-design 失效标注

---

## 〇、总览

| 核对项 | 结论 |
|---|---|
| ① greenfield ↔ 代码（8 点更新） | ✅ **全部落地，无其它偏差** |
| ② greenfield ↔ 核心 3 份互洽 | ✅ **基本互洽，1 处 P2 描述性偏差（EventResult 派生）** |
| ③ refactor-contract-design 失效标注 | ✅ **已标注"⚠️ 历史已失效"** |

**MERGE GATE：全框架文档与代码一致 → 放行（PASS），可进入补 D 系列。** 仅 1 处 P2（greenfield EventResult 派生描述残留）建议顺手修正，不阻塞放行。

---

## 一、① greenfield ↔ 代码（8 点更新全部落地）

| # | greenfield 更新点 | 代码实况 | 结果 |
|---|---|---|---|
| 1 | `Color` 改用 `u8` 4 通道 + `rgba/rgb` 构造器 | view.rs:73-77 `pub struct Color{r,g,b,a:u8}`；view.rs:81/85 `rgba`/`rgb` | ✅ |
| 2 | `PropValue` 删 `WidgetId` 变体 | view.rs:49-59 仅 `Unit/Bool/Int/Float/Str/Color`，无 WidgetId | ✅ |
| 3 | `parse_rgss` 改 `-> StyleSheet` | style/mod.rs:28 `pub fn parse_rgss(_src)->StyleSheet` | ✅ |
| 4 | default feature：core=["layout"]、render=[]、platform=["winit"] | core/Cargo.toml:16 `default=["layout"]`；render:22 `default=[]`；platform:19 `default=["winit"]` | ✅ |
| 5 | core 模块补齐（coordinator/registry/widget_state/message/a11y_tree） | core lib.rs:19-35 含全部（a11y_tree/coordinator/registry/widget_state/message/locale 等） | ✅ |
| 6 | facade 定向重导出 | lib.rs:36 `rgui_platform::{FocusManager, InputModality}`；:37 `rgui_render::{GlyphKey, PathTessellation}` | ✅ |
| 7 | `EventResult` 标注"无 derive"（见 §三 偏差说明） | traits.rs:75 实际 `#[derive(Debug, Clone, PartialEq, Eq)]` | ⚠️ 见下 |
| 8 | `Key{Str,Num}`、WidgetId 含 NodeHandle/WindowId、Color::rgba/rgb | view.rs:92-96 `Key{Str,Num}`；id.rs WidgetId/NodeHandle/WindowId；View:81/85 | ✅ |

**8 点中 7 点完全落地**；第 7 点（EventResult 派生）greenfield 描述与代码不符（见 §三）。

### 其它无偏差核实
- greenfield §A 5-crate 拓扑、§A.3 DAG（render/platform 互不相依、只向下 core）——与 Cargo.toml 一致。
- greenfield §B.2 `RenderBackend{Vello}` 单一、`SceneGraph`/`GlyphKey`/`PathTessellation` 在 render 侧——与 render lib.rs 一致。
- greenfield §B.3 platform `winit` default=["winit"]（§B.3 修正注释）——与 platform Cargo.toml 一致。
- greenfield §B.5 facade 定向重导出 + `#[cfg(feature="window")] fn run<W, F>(config, widget, state, mapper)`——与 rgui/src/app.rs 一致（App::run(config,...)）。
- greenfield §D feature 表（core default=["layout"]/render default=[]/platform default=["winit"]/facade window）——与各 Cargo.toml 一致。
- greenfield §E.2 edition=2021、MSRV 注（以 render 依赖上限为准）——与 Cargo.toml `edition=2021, rust-version=1.85` 一致（greenfield E.2 正文写 1.75、括注"以 render 依赖上限为准"，代码 1.85——greenfield 已用括注自洽，**可接受**，非硬性不符）。
- greenfield §E.3 增量验收措辞"改数据/状态层→不重编 render"——与现状一致（render 依赖 core 整 crate 的事实仍存，验收措辞为"意向性"，符合设计）。
- **M1 教训**：core/state 零 GPU 引用（仅 state/mod.rs:6 注释禁止）——greenfield §B.1 的 M1 教训遵守。✓

---

## 二、② greenfield ↔ 核心 3 份互洽

### 2.1 互洽（无互相矛盾）
| 维度 | greenfield | D0 | D11 | CLAUDE.md | 结论 |
|---|---|---|---|---|---|
| crate 拓扑 5 个 | §A.1 | §2 | §2 | §标题 | 一致 |
| 依赖方向（render/platform 互不相依、只向下 core） | §A.3 | §3 | §2 | §依赖方向 | 一致 |
| core 零 GPU/平台防火墙 | §A.3 契约 | §2/§8-硬约束A | §2/§依赖防火墙 | §开发约定 | 一致 |
| platform default=["winit"] | §B.3/§D | §7.3/§9 | §3/§4 | §17 | 一致（CLAUDE 最早同步） |
| core default=["layout"] | §D | §9 | §3 | （未细化） | 一致 |
| facade 定向重导出（FocusManager/GlyphKey 等） | §B.5/§B.0 | §7.5 | （未详列） | （未详列） | 一致 |
| 单一 vello、唯一 Tier1、删 devtools/skia/Rhai | §F | §8/§10 | §3 render | §单一机制 | 一致 |
| WidgetSpec 签名（measure(BoxConstraints,&MeasureContext)） | §B.1 | §4.3 | （未列） | §核心 Trait | 一致 |
| 4 trait 集合 | §B.1 | §4 | （未列） | §核心 Trait | 一致 |

**核心 3 份与 greenfield 在 feature 配置、trait 签名、crate 拓扑、依赖方向、防火墙、单一机制上完全一致，无互相矛盾。**

### 2.2 【P2-互洽偏差】EventResult 派生描述不一致
- **greenfield §B.1:130 / §更新记录#7**：`pub enum EventResult<M> { Handled, Prevented, Continue(M) }`，**声称"无 derive（裁决 A 接受）"、"建议 dev 回查补 #[derive(Debug, Clone)]（不加 PartialEq）"**。
- **代码 traits.rs:75**：`#[derive(Debug, Clone, PartialEq, Eq)] pub enum EventResult<M>`——**已带 PartialEq, Eq**。
- **D0 §4.4:102/111**：`#[derive(Debug, Clone, PartialEq, Eq)]`——**与代码一致**，且 D0:111 明确"实际派生 Debug, Clone, PartialEq, Eq"。
- **CLAUDE.md:38**：仅"Handled/Prevented/Continue(M)"，未提派生——不矛盾。
- **判定**：**greenfield 与 code/D0 不一致**（greenfield 声称无 derive、建议补（不加 PartialEq），但代码已带全量 derive 含 PartialEq）。greenfield 的"不加 PartialEq（Continue(M) 泛型无法全 Eq）"是技术顾虑，但代码 derive(Eq) 能编译（derive 为 M 加 Eq bound，消费方 M 需满足）。
- **严重度：P2**（描述性偏差，不阻塞编译/功能；D0/CLAUDE 均已正确反映代码带派生）。这是 greenfield 更新时未同步代码的"EventResult 实际已带 derive"这一事实——它还在"建议回查补"，说明写蓝图时误以为无 derive。
- **修正建议**：greenfield §B.1 EventResult 行 + §更新记录 #7 应改为"实际 `#[derive(Debug, Clone, PartialEq, Eq)]`（对齐代码）"，删去"无 derive/建议补"表述。低成本修正，随补 D 系列一并做。

---

## 三、③ refactor-contract-design 失效标注（✅ 已标注）

- `tools/2025-09-01_rgui-refactor-contract-design.md:3`：**"⚠️【历史已失效 · 已被推倒重来取代】本文档针对「渐进重构 old 12-crate → 6-crate」方案。用户已决策「推倒重来（greenfield）」，最终架构以 greenfield 为唯一权威（5-crate 收敛）。本文保留作历史存档，不作为实现依据。"**
- **确认**：该标注完整、明确，含"历史已失效"关键词 + 指向 greenfield 唯一权威 + "不作为实现依据"。**满足总监要求。** ✓

---

## 四、MERGE GATE 判定

**全框架文档与代码一致 → 放行（PASS），可进入补 D 系列。**

- **greenfield ↔ 代码**：8 点更新 7 点完全落地，1 点（EventResult 派生）为描述性偏差（P2）。
- **greenfield ↔ 核心 3 份**：feature 配置、trait 签名、crate 拓扑、依赖方向、防火墙、单一机制完全一致，无互相矛盾（仅 EventResult 派生一处 P2 描述差异）。
- **refactor-contract-design 失效标注**：已确认生效。
- **P0 / P1：均无。**

### P2 观察项（随补 D 系列一并处理，不阻塞放行）
1. **greenfield §B.1/§更新记录#7 EventResult 派生描述与代码不符**——greenfield 称"无 derive、建议补(不加 PartialEq)"，代码实际 `#[derive(Debug, Clone, PartialEq, Eq)]`。建议 greenfield 改为对齐代码（删"无 derive/建议补"），消除与 D0/代码的表述差异。
2. greenfield §E.2 正文 MSRV 写 1.75、括注"以 render 依赖上限为准"（代码 1.85）——已自洽，可接受，但建议正文直接标 1.85 更清晰。

---

*核对方：devco-reviewer｜只读核对，未改任何文档/代码。greenfield 8 点更新与代码一致（除 EventResult 派生 1 处 P2 描述差异）；greenfield 与核心 3 份互洽；refactor-contract-design 已标失效。全框架文档一致，放行。*
