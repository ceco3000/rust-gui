# rgui 简洁化重构方案 — 风险分级审查报告（P0/P1/P2）

> 审查方：devco-reviewer｜审查对象：`tools/2025-09-01_rgui-complexity-audit.md` + `tasks.md`
> 结论性质：只读代码核查（未运行构建/测试），基于源码依赖关系、rustc/Cargo 语义与增量编译模型
> 证据行号均引用实际源码路径，非模版占位

---

## 〇、审查结论速览

| 风险 | 定级 | 状态 |
|---|---|---|
| R1（a11y 牵连） | **P1**（归因有误，需降级） | tasks.md 表述与实际代码不符，真实风险被**高估**，但也暴露一个被低估的断裂点 |
| R2（`.rgui` 声明式依赖） | **P0**（若删 devtools 不内联保留） | 确认为真实硬依赖，`app.rs` 直接消费 `rgui_parser` |
| R3（增量收益窄化） | P2（需 M7 实测） | 成立但可控，非阻塞 |
| R4（verify/文档去向） | P2 | 决策问题，非代码风险 |
| **新 N1（core 循环依赖）** | **P0**（若 M1 未严格前置） | **审计报告与 tasks.md 均未点名**的破坏性风险 |
| **新 N2（facade 公共 API 断裂）** | **P1** | `rgui` 使用者会编译失败，依赖 `rgui/src/lib.rs` 仍重导出 |
| **新 N3（proc-macro 合并陷阱）** | P1 | `rgui-macros`+`rgui-components` 与 `core` 的合并次序错误会引入编译环 |

**阻塞项：R2 + N1 共 2 个 P0。** P0 清零前不应放行 M2、M3；M1 为唯一前置安全先手。

---

## 一、R1 — a11y 牵连（tasks.md 定级预期应与代码事实对齐）

### 实测（vs tasks.md 表述）
tasks.md L14 声称"`rgui-a11y` 被 `rgui/src/app.rs` 实际使用（焦点管理、`FocusManager`）"。

**实际核查**：
- `rgui/src/app.rs:20` → `use rgui_platform::focus::FocusManager;`——焦点管理**来自 `rgui-platform`**，与 `rgui-a11y` 无关。
- `rgui-platform/src/focus.rs:37` → `pub struct FocusManager`，确认为 app.rs 唯一焦点来源。
- `rgui-a11y` crate 的**唯一消费方**：`rgui/Cargo.toml:16`（facade 显式依赖）+ `rgui/src/lib.rs:19`、`:33`（`pub use rgui_a11y::*`）。app.rs 本体对 a11y crate 零直接引用。

### 被低估的真实断裂点（N1 家族）
a11y 的**核心类型并不在 `rgui-a11y` crate 里，而在 `rgui-core` 里**：
- `rgui-core/src/a11y.rs:121` → `pub struct AccessibilityNode`
- `rgui-core/src/lib.rs:69` → `pub use a11y::{AccessibilityAction, AccessibilityNode, AccessibilityRole, AccessibilityState};`
- `rgui-core/src/lib.rs:82` → 同样重导出

而 `rgui-components/src/wa_badge.rs:20` → `use rgui_core::a11y::AccessibilityNode;`——**组件层直接用 core 的 a11y 类型**，根本不经 `rgui-a11y` crate。

### 定级与处置
- **R1 本身：P1，判定为"被高估"但需澄清**。删除 `rgui-a11y` crate **不会**丢失 a11y 核心类型（它们在 core 里），也不会破坏 app.rs 的焦点管理（在 platform 里）。tasks.md L14 的归因应改为：`rgui-a11y` 只是 **accesskit 平台适配转发壳**（`backend.rs` 映射 accesskit::Role/Action，`tree.rs` 是 `AccessibilityTree` 容器），删除它只失去"接入 accesskit 操作系统层面"的能力。
- **阻塞点**：删 `rgui-a11y` 会连带破坏 **`rgui/src/lib.rs:19` + `:33` 的 `pub use rgui_a11y::*`** 两处公共重导出。任何 `rgui` 使用方若曾引用 `rgui::AccessibilityTree` 等 a11y 项，会编译失败。→ 详见 N2。
- 处理建议（供设计师定稿）：a11y 能力保留在 core（无需动），`rgui-a11y` crate 只删除 accesskit 适配层，`rgui/src/lib.rs` 两处 `pub use rgui_a11y` 需删除或替换为 core 内嵌导出。无需"内部模块降级保留焦点管理"——焦点管理本就在 platform，迁移到 core 时一并收纳即可。

---

## 二、R2 — `.rgui` 声明式是 app 核心依赖（P0，若内联未做则必然阻断）

### 实测（确认 is_true）
`rgui_parser` 与 `parse_rgui_file` 的**全部生产消费方**都在 `rgui/src/app.rs` 的 `#[cfg(feature = "devtools")]` 块内：
- `app.rs:354` → `rgui_devtools::rgui_parser::is_state_expr(expr)`
- `app.rs:358` → `rgui_devtools::rgui_parser::infer_prop_value(value)`
- `app.rs:403` → `use rgui_devtools::rgui_parser::parse_rgui_file;`
- `app.rs:414` → `parse_rgui_file(&rgui_path)?`
- `app.rs:436-438` → `app.load_rhai_scripts(&rhai_refs)?`
- `app.rs:139/150/181/182` → `pub rhai_paths: Vec<PathBuf>` 等，是 AppConfig 的**公开字段**

`rgui::run_rgui_app`（app.rs:398-438）是 `.rgui` 声明式应用的**一行启动入口**（`config.rgui_path` + `config.rhai_paths`），这是对使用者暴露的公共 API。

### 定级
- **R2：P0**。理由：`rgui-devtools` 依赖 `rgui-state`+`rgui-style`+`rgui-script`+`rhai`（devtools/Cargo.toml）。若在 **M1 未清干净**时删除 devtools，会：
  1. 移除 `parse_rgui_file` / `is_state_expr` / `infer_prop_value`——`app.rs:354/358/403/414` 编译失败；
  2. 移除 `rgui-script` 后 `app.rs:436` `load_rhai_scripts` 失败；
  3. app.rs 中 30+ 处 `#[cfg(feature = "devtools")]` 分支需逐一清理。
- **但注意**：R2 是**条件性 P0**——若按 M2 L75 提示，**先内联保留/迁移 `rgui_parser` 到 core/facade**，则降为 P1（仅是废弃决策问题）。tasks.md L75 已正确预判这一点，须在 M2 工单里显式执行"内联迁移"，**不能只写删**。
- 附带发现：`.rgui`/`.rhai` 样例与 Tier 2 路径（`rgui-components/src/accordion.rgui`、`accordion.rhai`、`accordionitem.rgui/rhai`，`interactive.rs:86-93` 的 `_rhai_path` 匹配、`accordion.rs:130-139` 的 `_rhai_path` 识别）**纵深耦合**。删 Tier 2 需同步清理 `rgui::interactive.rs`、`rgui::accordion.rs`、`paint_factory.rs`（`execute_rhai_paint_script` 多处 `cfg(feature="devtools")`）——工作量比 tasks.md 描述的"删 devtools"大，**M2/M5 工单须在范围上扩容**。
- 处置决策仍需设计师/用户确认（tasks.md L80-82 悬置），本报告仅确认其**风险实质与规模**。

---

## 三、R3 — 增量收益窄化（P2，非阻塞）

成立。收敛到 6-8 层后，原"按高频变更逻辑细粒度隔离"的收益确实变粗。**但方向反了会放大成本**：
- 把 **`rgui-state`（高变更：diff/snapshot）与 `rgui-layout`（中变更）并入 `rgui-core`** 后，`rgui-core` 成为"什么都进"的大包，任何 core 内改动都会重编所有依赖 core 的 crate（render/platform/facade）→ 高频变更逻辑落入 core，恰恰是增量收益最差的落点。
- 真正该保留隔离的，是**高频变更且被多 crate 依赖的**——`rgui-core` 本就是全 workspace 的最底层依赖（render/platform/state/layout/style/components/macros 全依赖它），**把高频变更逻辑往里塞，等于把它变成新的 batch-recompile 源头**。
- 建议：M3 合并时**对冲策略**——`rgui-state`/`rgui-layout` 是否真的并入 `core`，应做一次"高频变更频率 vs 增量收益"权衡：若 diff/snapshot 改动频繁，或许**保留 `rgui-state` 独立**更符合增量目标，尽管这会让 core 收敛数变成 7 层而非 6 层（仍在 6-8 范围内）。
- **R3 给 M7 的实证目标**：M7 L188 设定的"改 core 一函数 → `cargo check -p rgui-render` 不重编"**本身就是伪命题**——因为 `rgui-render` 天然依赖 `rgui-core`（render/Cargo.toml:14），改 core 必然重编 render。**该验收标准在目标拓扑下不可达成，需改写成"改 state 部分 → render 不重编"（依赖 M1 剥离后才成立）**。这是 tasks.md 一个隐蔽的验收陷阱。

---

## 四、R4 — verify/ 与文档去向（P2，决策非代码风险）

属实但为决策项。补充事实：`verify/` 现有 12 个验证子项目（`v1-v10`、`av2-skia`、`baseline-restart` 等），但 workspace members 仅收录 6 个（Cargo.toml L7-13）。`av2-skia` 因 skia-safe 0.75 与 0.82 版本冲突已被注释掉（Cargo.toml:10）——**若 M0 想彻底移出 verify，注意这些被注释成员的编译残留**。文档处置无新增风险，按审计四面处理即可。

---

## 五、遗漏的破坏性风险（审计报告与 tasks.md 均未点名）

### N1 — core 合并引发编译循环依赖（P0，M1 前置的硬理由）
**触发条件**：在 **M1 断链之前**执行 M3 的"state/layout 并入 core"。
- 现状依赖图（已核实）：
  - `rgui-render` → `rgui-core`（render/Cargo.toml:14）
  - `rgui-state` → **`rgui-render`**（state/Cargo.toml，store.rs:68 `GlyphKey`、store.rs:71 `PathTessellation`）
  - `rgui-state` → `rgui-layout`（state/Cargo.toml，store.rs:65 `LayoutResult`）
  - `rgui-components` → `rgui-core`+`rgui-layout`+`rgui-macros`
- **若先合并**：`core ∋ state` → `core → render`；但 `render → core`（既有）→ **`core ⇄ render` 循环依赖，Cargo 直接拒绝编译**。
- **判定**：M1 剥离 `state→render`（把 `GlyphKey`/`PathTessellation`/字形缓存移入 render，store.rs 只留纯快照/差分）是**合并 state 进 core 的不可绕过的先决条件**。tasks.md 已将 M1 标为"最高优先前置"并置顶 M3，**方向正确**；本报告强调：**M3 中"state/layout 并入 core"这一行必须在 M1 完成并通过 `cargo check` 后方可开工**，且 M6 收尾时仍需复核。审计报告 §三方向2 已点到"打破 state→render"，但**未上升到"否则 core 循环依赖"这一编译级后果**，这是本报告最重要的补充。

### N2 — facade 公共 API 断裂（P1，影响所有 `rgui` 使用者）
`rgui/src/lib.rs` 是纯重导出壳（审计报告已确认），删任何被它重导出的 crate 都直接 break 公共 API：
- `lib.rs:19` `pub use rgui_a11y::*;`、`lib.rs:33`(feature 下) → 删 a11y 必炸（见 R1）。
- `lib.rs:15` `#[cfg(feature = "devtools")]` 分支 → 删 devtools 需整块清理。
- `lib.rs:23` `pub use rgui_macros::{AppMessage, WidgetSpec, html};`、`lib.rs:36` → macros 保留（proc-macro 独立），安全。
- **破坏面**：外部使用者 `use rgui::AccessibilityTree` 等会编译失败。**M3 验收标准 L119（"`rgui` 使用方 cargo check -p rgui 通过"）指向正确**，但 M2 阶段（删 devtools/a11y）应**同步**跑一次各示例 `cargo check`，否则 M2 不完整。
- 处理：M2/M3 每次删 crate 后，立即在 `examples/`（`one-accordion`、`one-wa-badge`）和 `rgui/tests/`（`template_test.rs`、`accordion_perf_test.rs`）跑 `cargo check -p rgui`，作为**每步的回归门禁**，而不是只压在 M7。

### N3 — proc-macro 合并陷阱（P1）
- `rgui-macros` 是 `proc-macro = true`（macros/Cargo.toml `[lib] proc-macro = true`），**Rust 硬约束：proc-macro crate 不可与普通 crate 合并**（审计报告已正确标注）。
- **但合并次序陷阱**：`rgui-components/src/wa_badge.rs:39` → `use rgui_macros::{AppMessage, PersistState};`，即 **components 依赖 macros**。目标拓扑 M3 把 `components` 并入 `core` → `core` 将依赖 `macros`。
  - 检查：`rgui-macros` 依赖 `syn/quote/proc-macro2`（无 `rgui-core` 依赖，仅为 dev-deps 在测试里用），**不产生环**。✓
  - 但 `AppMessage`/`PersistState` 这两个 derive 宏被 `WaBadgeState`（components 的**持久状态类型**）使用 → 若 `core` 收纳 components，`core` 的 `[lib]` 将直接使用 proc-macro 派生。**proc-macro crate 必须在依赖图中保持叶子**（不被普通 crate 反向依赖其编译产物）——当前 `core→macros` 方向安全（macros 不依赖 core），但要**明确提醒**：任何试图把 `macros` 的导出再合并进 `core` 的想法都是不可能的，`rgui-macros` 必须始终是独立 crate。**M3 目标拓扑表已把 macros 单独列出，正确，维持即可。**
- 另一个点：`AppMessage` 在 `rgui/src/lib.rs:23` 被 facade 重导出。若把 a11y 的 `AccessibilityNode`/`Action` 和 core 的 `AppMessage` 移来移去，注意 **facade 重导出路径一变，使用方 `rgui::AppMessage` 引用即断**。建议合并时保持 facade 重导出符号表稳定，或一次性在 M3 更新并全仓 grep 引用。

### N4 — `rgui-app` 拆分后的事件循环与 winit 粘合（P2，仅供 M4 参考）
M4 要把 app.rs 拆成 event_loop/render_coord/interaction/automation 多模块。app.rs:20 直接 `use rgui_platform::focus::FocusManager`、且大量引用 winit（`cfg(feature=devtools)` 块内也是）。拆分时这些模块间的**状态共享（`AppState`/`focus`/`rgui_paths`）**需要清晰边界，否则易引入 `&mut` 借用冲突。列为 P2 观察项，非阻塞。

---

## 六、重构顺序建议（M0–M7 依赖图评审）

**总体判断：tasks.md 的 M0→M1→M2→M3→M4 的主链顺序是正确且安全的，M1 的前置定位尤其正确。无需重排主链。** 但有几处需强化/修正：

1. **M1 是唯一无条件先手**——N1 决定它不可跳过、不可与 M3 并行。tasks.md 已置顶，**保持**。
2. **M2 与 M3 需重新明确先后语义**。tasks.md 把 M2（删子系统）排在 M3（合并）之前，理由是"删干净再合并减少面"。但 N1 表明 **M3 的 core 合并依赖 M1 而非 M2**——删 devtools/a11y 与合并不直接耦合。建议：
   - **A 路径（更稳）**：M2 只删 devtools/a11y 的**非 app 依赖部分**，把 `rgui_parser` 内联交付（规避 R2 的 P0），再进 M3 合并。这样 M2、M3 可并行，但 M2 必须先内联 parser。
   - **B 路径（严格串行）**：M1 → M2（含内联 parser）→ M3 → M4。更保守，工期长。
   - 推荐 **B**，理由：R2(P0) 的内联与 N1(P0) 的循环依赖都要求每一步可独立编译验证，串行可把失败面收敛到单步。
3. **R3/M7的验收标准需改写**（见 §三）：M7 L188 的"改 core → render 不重编"在目标拓扑下**不可达成**（render 天然依赖 core）。应改写成依赖 M1 的"改 state 差分区 → render 不重编"。**这是验收层面容易翻车的一处。**
4. **M6 → 提前为 M6 增加"依赖图二次校验"**：合并后（core 变大的新的依赖图）应做一次 `cargo tree`/依赖方向审计，确认无新环（尤其 core 收纳 components 后对 macros 依赖方向）。
5. **M7 门禁放行条件**：P0（R2 的内联交付 + N1 的无环）清零；建议在每次 M 步骤 commit 后即跑 `cargo check -p rgui` + 示例 check，而非只在 M7。

---

## 七、P0 风险清单（放行 Gate 前必须清零）

| # | 风险 | 证据 | 清零动作 |
|---|---|---|---|
| P0-1 | **N1：core 合并前 state→render 未断链 → core⇄render 编译环** | state/Cargo.toml + store.rs:68/71；render/Cargo.toml:14 | M1 剥离 `GlyphKey`/`PathTessellation`/字形缓存到 render；M3 仅在此之后合并 state/layout 进 core |
| P0-2 | **R2：删 devtools 未先内联 `rgui_parser` → app.rs 编译失败** | app.rs:354/358/403/414；devtools/Cargo.toml | M2 工单必须写"内联保留 `parse_rgui_file`/`is_state_expr`/`infer_prop_value`"到 core/facade，而非只删 |

**说明**：M0（仓库清理）、R3（增量收益）、R4（verify/文档）不构成 P0；P0-1 与 P0-2 之间存在**内在关联**——M1（断链）是 M3 的前提，而 M2 的内联又是删 devtools 的必然前置，两者都指向"必须先做干净的逐层重构，再合并"。建议总监在派发 M1 工单时，**一并将 `.rgui`/`.rhai` 声明式路径的去留决策（tasks.md L80-82 悬置项）在 M2 之前定稿**，避免 M2 因决策悬而未决而反复。

---

*审查方：devco-reviewer｜审查基于只读代码核查，未运行 cargo check/test/clippy；建议 M7 前置一次真实 `cargo check --workspace` 以验证 N1 不会实际触发（当前 dev profile 基线 56.49s 为重构前状态）。*
