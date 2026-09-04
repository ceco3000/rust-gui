# D3（5-crate 最小可编译骨架）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D3 交付（5-crate workspace 骨架）
> 审查基准：`tools/2025-09-01_rgui-greenfield-architecture.md`（接口契约级文档）
> 结论性质：只读代码核查 + `cargo tree` 依赖验证（未运行 cargo check/test，qa 已有 PASS 记录）
> 审查范围：总监五问

---

## 〇、结论速览

| # | 审查点 | 定级 | 一句话结论 |
|---|---|---|---|
| Q1 | 5-crate 架构符合性 | **PASS（无 P0/P1）** | 收敛克制，无转发壳/过度拆分 |
| Q2 | 依赖防火墙 | **PASS（无 P0/P1）** | 契约级成立：空 deps + 源码零引用 + cargo tree 仅自身 |
| Q3 | 核心 trait 契约签名 | **P1（2 处契约漂移）** | `measure` 与 `EventResult` 签名与 greenfield B.1 不符，D4+ 有返工风险 |
| Q4 | 增量编译收益 | **P1（验收表述逻辑漏洞）** | 验收对象（diff/snapshot）未实现 + Cargo 增量粒度是 crate 级 |
| Q5 | 其它 P0/P1 | —— | 见 §五（无新 P0） |

**总评：D3 骨架的基础是扎实的（架构收敛、防火墙、无重型依赖、facade 瘦身均达标），P0 清零，具备放行 D4 的条件。** 但存在 2 项 P1 需在 D4 开工前澄清，否则会在 D4/深实现时引发签名级返工。**建议：有条件放行（CONDITIONAL PASS）**，P1 处理项随 D4 工单并行交付，不阻塞 MERGE GATE 但需显式跟踪。

---

## 一、Q1 — 5-crate 是否符合 greenfield 架构（PASS）

### 实测
workspace 仅 5 个 crate（Cargo.toml L5-10），与 greenfield §A.1 拓扑一字不差：
- `rgui-core`（1021 行）：已并入 state/layout/style/components/a11y_tree，模块精简（最大文件 traits.rs 193 行，其余大部分 <100 行）。
- `rgui-render`：收纳 `RenderLayoutCache`（lib.rs:37,41）、`GlyphKey`/`GlyphCacheEntry`/`GlyphAtlas`（lib.rs:22）、`PathTessellation`、`TextShaper`、`SceneGraph`——GPU 类型归属正确。
- `rgui-platform`：window/input/ime/focus 模块齐备。
- `rgui-macros`：proc-macro=true（macros/Cargo.toml `[lib]`），app_message/html/persist_state/widget_spec。
- `rgui`（facade）：**总计 334 行**（对比旧 app.rs 的 5017 行），lib.rs 纯重导出，无 God Object。

### 判定
- **无转发壳迹象**：每个 crate 都有实责（render 持 GPU 类型、platform 持 winit 抽象、core 持逻辑契约、macros 持派生宏、facade 持启动协调）。
- **无过度拆分**：5 层落在 greenfield 定稿的"4-6 层"区间内，且各模块边界清晰。
- **唯一观察项**（非缺陷）：`rgui-core` 现在同时含 `state`（数据层）+ `components`（UI 层）+ `style`（文本解析）——greenfield §E.3 要求"数据层与 UI 层严格分开"以保证增量收益。当前这些是**模块级**分开（`state.rs` vs `components.rs`），但物理上同属一个 crate。这直接关系到 Q4 的增量验收能否实现（见 §四）。

**结论：Q1 达标，无 P0/P1。**

---

## 二、Q2 — 依赖防火墙是否真正成立（PASS）

### 双重验证（每层都过）
1. **Cargo 层**：`rgui-core/Cargo.toml [dependencies]` **完全为空**（仅 `[lints] workspace=true`，L10-13），无任何依赖条目。
2. **工程层**：`cargo tree -p rgui-core` 输出仅 `rgui-core v0.1.0` 自身，**无任何传递依赖**——比 greenfield §A.3 承诺的"不依赖 render/platform/macros/winit/wgpu/vello/cosmic-text"更严格（连 serde/thiserror 也在 D3 阶段 0 被排除）。
3. **源码层**：grep `rgui_render/rgui_platform/rgui_macros` 在 `rgui-core/src` 返回**空**；core 仅 `use std::*/crate::*`，零外部 crate 引用。

### 反向依赖方向（DAG 无环，已核实）
- `rgui-render/Cargo.toml:11` → `rgui-core`（只向下）
- `rgui-platform/Cargo.toml:11` → `rgui-core`（只向下）
- `rgui/Cargo.toml:11-14` → core/render/platform/macros（facade 依赖全部）
- render 与 platform **互不相依**（各自仅依赖 core）——greenfield §A.3 禁横向依赖达成。

### 隐藏反向依赖排查（总监问"有无隐藏"）
- 无变体：core 未通过 feature 反向引入、无 `dep:` 隐式依赖、无私下 `path` 引用。
- `state.rs:3` 明确注释"RenderLayoutCache 已移至 rgui-render，零依赖 rgui-render/rgui-layout 类型"——GPU 类型已被物理迁出 core，防传播。

**结论：防火墙契约级成立，无隐藏反向依赖，无 P0/P1。**

---

## 三、Q3 — 核心 trait 契约签名合理性（P1，2 处契约漂移）

总监第 3 问直指要害。实际 `traits.rs` 与 greenfield B.1 的**接口契约级签名**存在 2 处实质偏差，另有 1 处差异需澄清。

### 漂移 #1：`WidgetSpec::measure`（P1，高返工风险）
| | greenfield B.1 | 实际 traits.rs |
|---|---|---|
| 签名 | `measure(&self, &State, constraints: BoxConstraints, ctx: &MeasureContext) -> Size` | `measure(&self, &State, _available: Rect, ctx: &mut UpdateContext) -> Size` |
| 参数 | `BoxConstraints`（布局约束：min/max） | `Rect`（几何区域） |
| ctx 类型 | `&MeasureContext` | `&mut UpdateContext` |

**风险**：
- **语义偏差**：`BoxConstraints` 是布局引擎的关键输入（Taffy/min-max），选 `Rect` 会丢失约束信息，D4+ 布局系统接入时 `measure` 必须改签名回 `BoxConstraints`。
- **Context 选型偏差**：`measure` 理论上应是只读测量（`&MeasureContext`），实际用了 `&mut UpdateContext`——可变更新上下文用于测量，违背"测量不修改状态"的设计意图，D4+ 会改回。
- **连带**：greenfield B.1 定义了 `MeasureContext`（只读环境），但实际 core/src/context.rs 里 `MeasureContext` **根本没有被定义**（context.rs 只有 View/Update/Paint/Access 四种）。这是契约缺失。
- **void**：目前 `measure` 无反调用方（grep 空），所以此漂移未产生编译错误，但它是**埋着的签名级地雷**——D4 实现 WidgetSpec 组件（accordion/wa_badge）时必然踩中。

### 漂移 #2：`EventResult<M>`（P1，语义漂移 + 悬空契约）
| | greenfield B.1 | 实际 traits.rs:95-104 |
|---|---|---|
| 变体 | `{ Handled, Prevented, Continue(M) }` | `{ Continue, Consumed, Emit(M), Stop }` |
| 语义 | 事件传播三态（处理/阻止/继续） | 四态（继续/消费/派发消息/停止） |

**风险**：
- **语义漂移**：`Handled/Prevented` 与 `Continue/Consumed` 语义接近但不等同；新增 `Emit(M)` 与 `Stop`。设计上更完善，但**与定稿契约不一致**。
- **悬空**：全仓 `grep EventResult` 仅 `traits.rs:95` 定义 + `lib.rs:48` 导出，**无任何消费方**。它是"无实现方、无调用方"的占位契约。
- **影响**：D4+ 事件系统（interaction/hit-test）会实现 EventResult 的传播逻辑。若按现有 `Emit(M)/Stop` 设计，则与 greenfield B.1 脱节；若按 greenfield 改回 `Handled/Prevented/Continue(M)`，则现在定义白写。**需在 D4 前定稿 EventResult 语义**，避免事件逻辑返工。

### 差异 #3：`WidgetSpec` 额外方法（低风险，P2）
实际 traits.rs 多了 `default_measure()` 默认实现（L71-78），greenfield B.1 未列。属合理补充，不构成风险，但 D4+ 实现组件时注意它覆写 `measure` 而非 `default_measure`。

### 与 greenfield 可能冲突的项（P2 观察）
- `AppMessage`/`PersistState`/`WidgetSpec` 的超级绑定（`Send + Sync + 'static`）与 greenfield B.1 一致 ✓。
- `PersistState` 无 `erased_serde::Serialize` 绑定——greenfield B.1 本身也未列，traits.rs L20-22 注明"实现阶段（D6/D7 引入序列化）按契约补全"，正确。

**判定：Q3 发现 2 项 P1（measure、EventResult 签名漂移）。当前骨架可编译是因为契约都是占位、无调用方，但 D4+ 这是返工源头。必须在 D4 开工前明确"以 greenfield B.1 为准还是以现有 traits.rs 为准"。**

---

## 四、Q4 — 增量编译收益是否真实可达（P1，验收表述逻辑漏洞）

总监问"改 core::state 不重编 render/platform（qa 已验证 SHA 一致）"。我发现了**验收逻辑层面的根本矛盾**，需先澄清再定级：

### 核心矛盾：Cargo 增量粒度是 crate 级，不是模块级
- greenfield §E.3 要求："改 `rgui-core::state` 的 diff/snapshot 日志（纯数据层）→ `cargo check -p rgui-render` 不重编"。**措辞是"核心内 state 模块"。**
- **但** `rgui-render/Cargo.toml:11` → `rgui-core`，**Cargo 的依赖跟踪/增量重编粒度是整个 crate，而非 crate 内的单个模块**。只要 `rgui-core` 这个 crate 的任何源码（含 state.rs）变化，Cargo 都会判定 `rgui-render` 依赖的 `rgui-core` rlib 指纹变化 → **触发 render 重编**。
- **结论**：**"改 rgui-core::state 不重编 render"在当前单 crate 结构下，物理上不可达**。这不是 qa 测错，而是**验收标准本身与工程实现模型不匹配**。
- 这解释了为什么 qa 报"改 core 一函数 → cargo check -p rgui-render 不重编"拿不到——但那应是指**旧审计的 M7 L188 改法**；当前 qa 用的是"改 core::state"验证，两者在同一 crate 内无法区分。

### 第二层问题：验收对象（diff/snapshot）尚未实现
- `grep fn diff / fn snapshot` 在 `rgui-core/src` 返回**空**。state.rs 只有占位 struct（`StateStore<M>`/`InstanceState`/`Patch`/`Snapshot`，全 `PhantomData`），**没有 diff()/snapshot() 函数体**。
- 也就是说 D3 阶段 0 **没有可被"修改以验证增量"的实际逻辑**。qa 的 SHA 一致性验证若针对的是占位 struct，则验证的是空壳，无法证明"真实 diff 逻辑改动不影响 render"。

### 真正可达到的增量验证（建议改写）
**唯一能实现"改数据层不重编 render"的工程路径是把数据层从 core 物理拆出**：
1. **方案 A（推荐，但回退）**：把 `state`（及 id/geometry/color 纯类型）拆成独立 crate（如 `rgui-core-data` 或并入一个更底层的 `rgui-data`），render/platform 依赖它，改动它才不重编 render。但这**增加 crate 数**，与 greenfield"收敛到 5"的初衷相悖——需权衡。
2. **方案 B（务实）**：放弃"改 core 内部模块不重编 render"这一目标，**接受"改 core 任何地方 → render 重编"的现实**（因为 render 依赖整个 core）。把增量收益目标改写为**"改 render/platform/macros → 不重编 core（上行隔离）"**——这个反而真实可达（render 改不改不影响 core 编译，因为 core 不依赖 render）。greenfield §A.2 已隐含此逻辑（"改渲染不重编核心"），§E.3 的措辞则偏向"数据层上行隔离"，两者矛盾。
3. **方案 C**：用 Cargo **feature 门控**，把 state 相关代码藏在 feature 内、切换 feature 时 render 走不同编译单元——过度复杂，非阶段 0 合理开销。

**判定：Q4 的验收标准在当前 5-crate 结构的 Cargo 增量模型下不可真实达成，需重写验收措辞（建议改成 B：验证"改 render/platform/macros 不重编 core"这一真实可达方向，或明确接受"改 core 即重编 render"并取消该项）。这是 P1（验收指标不可达，若按现状交 D4 会卡门/误导），但**不是代码缺陷**，是**验收标准与工程模型的适配问题**。

---

## 五、Q5 — 其它 P0/P1 风险排查

### 未发现新 P0。
### 以下 P1/P2 观察项供 D4 参考：

1. **P2 — `EventResult` 缺 `Clone`/`Debug` 一致性**：traits.rs:94 `#[derive(Debug, Clone, PartialEq, Eq)]`，但 `Emit(M)` 携带 `M: AppMessage`，若 `M` 非 `PartialEq/Eq` 则 derive 失败。D4 实现真实消息类型时需确认 `AppMessage` 消息类满足这些 bound。
2. **P2 — facade 与 greenfield B.0 再导出核对**：facade `lib.rs`：
   - `pub use rgui_core::*` ✓（覆盖 components/layout/state/style/a11y 全部并入内容）
   - `pub use rgui_macros::{html, AppMessage, PersistState, WidgetSpec}` ✓
   - **绿灯**：`pub use rgui_platform::{FocusManager, InputModality}` 与 `pub use rgui_render::{GlyphKey, PathTessellation}` ——已核实这些类型**真实存在**（platform/focus.rs:8、platform/input.rs:6、render/glyph.rs:10、render/path_tessellation.rs:6），**无悬空 use**。✓ 但注意 facade 只显式导出 `FocusManager/InputModality` 与 `GlyphKey/PathTessellation` 两两，**未用 `pub use rgui_platform::*` / `pub use rgui_render::*` 通配**——若 D4+ 有其它 platform/render 公共类型需暴露，facade 需补导出（greenfield B.5 用的是通配，实际这里是点名导出）。**低风险**，D4 对齐即可。
3. **P2 — `a11y` vs `a11y_tree` 双模块**：core 同时有 `a11y.rs`（AccessibilityNode/Action/Role/State）与 `a11y_tree.rs`（AccessibilityTree），greenfield §C.1 只规划了一个 `a11y/` 模块。当前为两个平铺文件，命名易混淆，D4 可考虑合并。非缺陷。
4. **P2 — `coordinator.rs`/`registry.rs`/`widget_state.rs`/`locale.rs`/`message.rs` 均为占位**：需确认这些是"契约占位"还是"遗留"，避免 D4+ 出现未规划模块。当前均为小文件（<35 行），判断为占位，可接受。
5. **P2 — `EventResult`/`FormField`/`WidgetLifecycle` 是 greenfield B.1 未列的额外 trait**：实际比契约多了 3 个 trait（FormField:125、WidgetLifecycle:137）。属合理补充，但 D4+ 需确认其归属与必要性，避免"额外机制"回潮（greenfield 核心原则：一个能力一条路径）。

---

## 六、MERGE GATE 建议

**结论：有条件放行（CONDITIONAL PASS）。**

### P0 风险清单：**无（P0 清零）**

### 放行条件（2 项 P1，须在 D4 开工前处理或随 D4 显式承担）：
1. **P1-C1（Q3）：核心 trait 契约定稿**——明确 `WidgetSpec::measure` 与 `EventResult` 以 greenfield B.1 为准，还是以现有 traits.rs 为准。**若以 greenfield 为准**：D4 前改 `measure(Rect,&mut UpdateContext)→measure(BoxConstraints,&MeasureContext)`、`EventResult` 改回三态，并补 `MeasureContext` 定义。**若以现有为准**：反向回改 greenfield B.1（会让契约文档再次漂移，不推荐，违反"定稿契约"）。**建议以 greenfield B.1 为准**，它是已定稿的接口契约，D3 骨架的占位签名应回靠。
2. **P1-C2（Q4）：增量验收措辞重写**——将"改 core::state 不重编 render"改为现实可达的方向（推荐：改为"改 render/platform/macros → 不重编 core"，或明确接受"改 core 即重编 render"并取消该项），并确保 D4+ 实现 diff/snapshot 后该项有真实验收对象。**勿把当前不可达的验收指标推进到 D5/M7 门禁**，否则会在最终门禁翻车。

### P2 观察建议（不阻塞，D4 参考）：`EventResult` bound 一致性、facade 点名导出 vs 通配导出对齐、`a11y`/`a11y_tree` 合并、额外 trait（FormField/WidgetLifecycle）必要性确认。

**一句话给总监**：D3 骨架的架构、防火墙、克制性（无重型依赖、facade 334 行）都做对了，P0 清零可放行；但**两个 P1（trait 契约漂移 + 增量验收表述）若不处理，会在 D4 实现时产生签名级返工、并在 D5/M7 增量门禁卡壳**。强烈建议 D4 开工前先定稿这两项（尤以 trait 契约为最），避免"骨架对、深实现返工"。

---

*审查方：devco-reviewer｜只读核查，未运行 cargo check/test/clippy。依赖方向与增量模型判断基于 Cargo 语义（crate 级依赖粒度），建议 D4 前置一次真实 `cargo check -p rgui-render` 验证 qa 的 SHA 结论。*
