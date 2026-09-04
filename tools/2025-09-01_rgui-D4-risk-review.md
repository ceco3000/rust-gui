# D4（核心循环 + 状态 diff/snapshot + 布局 Taffy）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D4 交付（rgui-core 核心逻辑实现）
> 审查基准：`tools/2025-09-01_rgui-greenfield-architecture.md` §B.1/§C.1 契约 + D3 风险审查遗留项
> 审查方式：只读代码核查 + 隔离复刻验证 diff 收敛性（未修改项目任何源码）
> 声明：以下 P1 缺陷已通过独立 Rust 复刻程序验证（`applied == b ? false`），非推测

---

## 〇、结论速览

| # | 审查点 | 定级 | 一句话结论 |
|---|---|---|---|
| Q1 | 状态 diff/snapshot 正确性 | **P1（1 项真实缺陷 + 1 项设计缺口）** | diff/apply_patch **不收敛**（子节点结构差异丢失）；diff 无 key-based reconcile，纯位置型 |
| Q2 | 布局 Taffy 集成 | **P2** | `to_taffy_style` 死路径（compute 未调用），`grow` 未接入；Taffy 类型未泄漏公共 API ✓ |
| Q3 | 核心循环 | **PASS（无 P0/P1）** | Coordinator 设计稳健、所有权/线程安全纯净，无循环更新问题 |
| Q4 | TDD 证据 | **P1（覆盖盲区）** | 测试真实但 roundtrip 收敛测试写的假设被缺陷推翻；布局/compute 未覆盖 |
| Q5 | 契约一致性 | **PASS（D3 遗留已修复）** | measure/EventResult 已回靠 greenfield B.1；防火墙仍成立（taffy 纯 Rust 非违规） |
| Q6 | unused warning | **P2（占位遗留）** | 非设计缺陷，但 `state` 字段与 `widget_state`/`registry` 是深占位，需 D5+ 清理或说明 |

**总评：核心结构与契约已达 greenfield 标准（尤其 D3 的 P1-C1 契约漂移已修复），具备放行条件。但存在 1 项真实 P1（diff/apply_patch 不收敛）——它直接关系到"状态一致性与状态丢失"这一 D4 核心验收目标，建议**在放行 D4 时带回一个修复工单**（conditional pass），或要求 dev 在 D5 前补齐 diff 收敛性 + 补充 roundtrip 测试。**

---

## 一、Q1 — 状态 diff/snapshot 正确性与稳健性（P1，最核心）

### 1.1 【P1-1】`diff`/`apply_patch` 对子节点结构差异**不收敛**（真实缺陷，已复刻验证）

**代码位置**：`rgui-core/src/state/diff.rs:22-47`（diff）+ `:50-70`（apply_patch）。

**缺陷机理**：
- `diff(a,b)` 在 `a.children.len() != b.children.len()` 时，**只生成 `SetChildCount{len:b.children.len()}`**（diff.rs:31-33），然后 `common = min(...)` 只在**公共索引范围内**比较 props（diff.rs:36-43）。
- 结果是：当子节点**数量变化**时，超出 `common` 的那些子节点（含其整个子树内容）**完全不会被 diff 捕获**——不会生成任何 `SetChildProps` 或递归 patch。
- `apply_patch` 的 `SetChildCount`（diff.rs:56-62）：多则 `push(WidgetView::empty())`，少则 `truncate`。

**复刻验证（独立 Rust 程序，未改项目）**：
```
输入 a = {props:1, children:[{props:10, children:[{props:100}]}, {props:20, children:[{props:200}]}]}
输入 b = {props:1, children:[{props:10, children:[{props:999}]}]}   // 根 props 相同，子数 2→1，index0 深层子从 100→999
diff 输出 = [SetChildCount{len:1}]                                   // 关键：只有这一条 patch
apply(a, patches) = {props:1, children:[{props:10, children:[{props:100}]}]}  // 仍是 100，不是 999
收敛检验：applied == b ? false   ❌   // 不收敛！
```
**后果**：子节点结构差异（数量增删 + 深层内容变更）会导致 `diff→apply_patch` 后状态视图**不等于目标视图**——这直接违反 diff.rs:6 注释自述的"`diff(a,b)` 后 `apply_patch(b, patches)` 收敛"，也否定 demo 声称的"diff 1 patch 闭环正确"（demo 只测了根 props 变化，未覆盖子节点结构场景）。

**触发条件**：任何真实 GUI 中**增删子组件 / 树结构调整**（如折叠 + 插入列表项）都会踩中。

**定级**：P1。这是 D4 验收目标"状态 diff/snapshot 正确性"的直接反例。**修复方向**（供 dev，不代改）：diff 需对子节点做**递归**处理——数量不同时，多出的子节点应生成"插入整棵子树"的 patch（把 `b.children[i]` 整节点作为 `SetChildProps`/新增 patch 携带），而非仅 `SetChildCount` + 依赖 apply 用空节点补齐。同时增删子节点需 key-based reconcile（见 1.2）。

### 1.2 【P1-2】diff 无 key-based reconcile，纯位置型（设计缺口，与 1.1 协同）

- `diff` 按 `index` 逐个比较 props（diff.rs:37-44），**没有 key（`Key::Str/Num`）匹配机制**。greenfield 定义了 `Key`（view.rs:80），但 diff 完全未使用。
- **后果**：子节点**顺序交换**（如列表重排）会被误判为每个 index 的 props 都变了，产生大量错误 patch；且无法做最小化移动。这是"diff 正确性"的另一个维度。
- **定级**：P1（与 1.1 同属 diff 正确性家族）。修复需引入 key-based diff（类似 React/egui 的 reconcile），这可能在 D5+ 才需要——**建议至少将"position-based diff 的局限"写入文档/契约，明确当前仅支持"props 变更"这一最小用例**，避免 D4 验收时被误认为"完整 diff"。

### 1.3 【P1（同族）- snapshot 序列化是占位】schema 稳定性"名义成立，实质未实现"

- `Snapshot`（snapshot.rs:14-21）字段是 `schema_name: String` + `schema_version: u32` + `instances: BTreeMap<u64, PropValue>`。
- 但 `snapshot.rs:3` 注释明确"序列化使得在 D6/D7"，当前 `Snapshot` **不是真的可序列化快照**，只是内存结构。qa 声称的"snapshot schema=demo_counter"验证的是 `schema_name` 字符串，**没有真正验证序列化往返**。
- **定级**：P2（占位，符合 D4 阶段规划）。但需确保 qa 的"snapshot 验收"措辞不误报为"已实现序列化"。
- **sub-note**：`instances: BTreeMap<u64, PropValue>` 用 `u64` 而非 `WidgetId`（snapshot.rs:20），虽可编译但绕过了 `WidgetId(u64)` 类型安全——`WidgetId` 是 newtype（greenfield B.1），这里用裸 `u64` 会丢失"不可混用 id"的类型保护。**P2**，建议改 `BTreeMap<WidgetId, PropValue>`（需要 WidgetId: Ord/Hash，D5 对齐）。

### 1.4 snapshot 测试盲区
- 测试只有 `sn4_snapshot_empty_no_panic`、`sn1_snapshot_schema_stable`（d4_acceptance_state.rs:74-82），**无插入/读取/键冲突/迁移测试**。`Snapshot::insert_state/get_state`（snapshot.rs:34-41）逻辑正确但无测试覆盖。**P2**（建议 D5 补）。

---

## 二、Q2 — 布局 Taffy 集成（P2）

### 2.1 依赖防火墙：taffy 是**合规依赖**，非违规（总监第 5 问确认）
- `core/Cargo.toml`：`taffy = { version="0.7", optional=true }`，`default=["layout"]`，`layout = ["dep:taffy"]`。
- `cargo tree -p rgui-core`：`taffy→{arrayvec, grid, slotmap}`——**纯 Rust，无平台/GPU**（taffy/grid/slotmap 均非 GPU crates）。
- greenfield §A.2 明确"layout 并入 core"、§D"仅重型依赖隔离所需 feature"。taffy 是纯 Rust 布局库，**符合** greenfield，**不违反**"core 零 GPU/零平台"（绿色架构的"零平台"指 winit/wgpu/accesskit 这类，非纯 Rust 的计算库）。
- **唯一小疑虑（P2）**：`default = ["layout"]` 使 **core 默认构建就拉入 taffy**，这与 greenfield 的"keep core 轻量，仅重型依赖 feature 门控"略有张力——但 taffy 很轻（纯 Rust），且 layout 是 core 内建能力，**可接受**。若想让 core 默认零依赖，可改 `default=[]`，D5 权衡。

### 2.2 【P2-1】`to_taffy_style` 是**死路径**（compute 未调用该映射）
- `mapping.rs:40 to_taffy_style(style: &LayoutStyle) -> taffy::prelude::Style` 定义了 `LayoutStyle→Taffy` 映射（含 `grow→flex_grow`），**但 `LayoutEngine::compute`（layout/mod.rs:67-112）完全没有调用它**——compute 直接 `Style::default()` 手动设 `size`，`grow` 字段（`flex_grow`）**未接入计算结果**。
- **后果**：`LayoutStyle.grow`/`fixed()` 等公共字段**实际无任何效果**（compute 不读它们）。这是"定义与实现脱节"——greenfield §C.1 要求 mapping"封装 Taffy，不暴露到公共 API"，但 mapping 目前是**孤岛**（只被导出，不被业务路径使用）。
- **定级**：P2。当前 `compute` 只做最简单的"固定尺寸容器 + flex 行排布"，功能上能编译测试通过（layout_engine.rs 只断言尺寸 ≤ 容器），但**距离真实布局（grow/约束/对齐）很远**。建议 D5+ 让 compute 真正调用 `to_taffy_style`，或移除 mapping 的死代码（避免"看似有 grow 能力实则无"的误导）。

### 2.3 【已达标】Taffy 类型未泄漏公共 API（总监第 2 问子项）
- `to_taffy_style` 是 `#[cfg(feature)] pub fn`（mapping.rs:40），但**未被 `lib.rs` re-export**（lib.rs:69 只 `pub use layout::{LayoutEngine, LayoutNode, LayoutResult, LayoutStyle}`）。
- `LayoutStyle` 是纯 Rust 公共类型（mapping.rs:8-15，全是 `Option<f32>`/`bool`）——**泄漏到公共 API 的是纯 Rust 类型，非 Taffy 类型**。✓ greenfield §C.1"不把 Taffy 类型泄漏到公共 API"达成。**无违规。**

---

## 三、Q3 — 核心循环（PASS）

### 3.1 Coordinator 设计稳健
- `Coordinator<W: WidgetSpec>`（coordinator.rs:21-25）持有 `spec: W` + `state: W::State`（具体类型，非泛型擦除），`new/dispatch/current_view/name/state` API 干净。
- `dispatch`（coordinator.rs:43-46）：`self.spec.update(msg, &mut self.state, ctx)` 更新状态 → `self.current_view(&ViewContext::default())` 重新渲染视图。**闭环正确**（core_loop.rs 测试 0→1→2 印证，与 qa 的 demo 一致）。
- **所有权/线程安全**：全类型 `Send + Sync + 'static` supertrait（traits.rs:15/24/47），无 `Rc`、无裸指针、无 `unsafe`。`Coordinator` 单线程宿主，`PhantomData` 仅作 marker。**纯净，无数据竞争风险。**
- **事件传播**：`dispatch` 直接调 `update`（同步），**无循环更新风险**（一次 dispatch 只产生一次 update→view；要产生"消息级联"需在 `update` 内再次 dispatch，当前无此机制——**这反而是"无事件循环"的简化，健康**）。

### 3.2 与 greenfield 一致的确认
- `W::view(&self.state, ctx)` / `W::update(msg, &mut self.state, ctx)` 调用 signature 与 traits.rs 的 `WidgetSpec`（view 用 `&ViewContext`、update 用 `&mut UpdateContext`）**完全吻合**（coordinator.rs:39,44）。无签名漂移。✓

### 3.3 小观察（P2）
- `dispatch` 用 `&ViewContext::default()` 重建视图（coordinator.rs:45）——若未来 ViewContext 含窗口尺寸/主题等真实数据，`dispatch` 后渲染会用**默认空 context**而非上次的。当前 ViewContext 是空占位（context.rs:9-11）无影响，但 D5+ 需要传递真实 ctx 时**是在 dispatch 返回时丢掉了原始 ctx**。**P2**（提前标记，D5 处理）。

---

## 四、Q4 — TDD 证据（P1，覆盖盲区与缺陷相互印证）

### 4.1 已知测试 vs 盲区
| 文件 | 覆盖 | 盲区 |
|---|---|---|
| core_loop.rs | Coordinator dispatch 闭环 0→1→2、state 反射、name/state ✓ | — |
| d4_acceptance_core.rs | Counter view/update/measure/paint/a11y、EventResult 变体、PropValue/几何契约 ✓ | — |
| d4_acceptance_state.rs | diff 空/单 prop/删除子节点，apply 空/单 prop roundtrip，snapshot 空/schema ✓ | **无子节点结构变化的 roundtrip 测试**；roundtrip 只测了根 props（`a5_diff_apply_roundtrip_converges`，d4_acceptance_state.rs:62-69）——这只测根 props，**恰好没测到我发现的子节点不收敛缺陷** |
| layout_engine.rs | compute 单/多子节点、尺寸 ≤ 容器 ✓ | **未测 grow/flex、未测 mapping（to_taffy_style 无任何测试）、未测实际布局位置（只测 size）** |
| diff.rs 单元测试 | `diff_identical_empty_is_empty`（diff.rs:77-82） | 极弱，仅空视图 |

### 4.2 【P1】`a5_diff_apply_roundtrip_converges` 断言被缺陷推翻
测试 `a5`（d4_acceptance_state.rs:62-69）：
```rust
let old = view_with("text", PropValue::Str("old"));
let mut new = view_with("text", PropValue::Str("new"));
let target = new.clone();
let patches = diff(&old, &new);
apply_patch(&mut new, &patches);
assert_eq!(new.props, target.props);   // 只断言根 props 相等
```
该测试**label 叫 roundtrip_converges，但只断言 `.props` 相等**——它假设的"收敛"范围**恰好排除了我发现的子节点结构域**。因此测试是**假阳性全绿**：既通过，又掩盖了 1.1 的子节点不收敛缺陷。**这直接印证总监第 4 问"测试未覆盖的盲区"——盲区是"子节点结构 diff 收敛"。** 定为 P1（建议 D5 补一个真正的多子节点 roundtrip 测试，会即刻暴露此缺陷）。

### 4.3 RED→GREEN 流程核实
- core_loop.rs / layout_engine.rs 头注释标注"TDD RED 起点"，测试驱动特征明显（Counter 测试组件、断言具体）。TDD 流程**真实存在**。
- 但 `diff.rs:22` 的实现没有"先写 roundtrip 测试再写 diff"的痕迹——diff 的单元测试只有空视图用例，说明 **diff 是"先写实现，后补最小测试"**，TDD 在 diff 上弱于其它模块。**P2**（流程观察，非功能缺陷）。

---

## 五、Q5 — 契约一致性（PASS，D3 遗留已修复）

### 5.1 【重大利好】D3 的 P1-C1 契约漂移已在 D4 修复
- D3 审查发现 `WidgetSpec::measure`（原 `Rect, &mut UpdateContext`）与 `EventResult`（原 `Continue/Consumed/Emit(M)/Stop`）**偏离 greenfield B.1**。
- D4 现状（traits.rs）：
  - `measure(&self, state: &Self::State, constraints: BoxConstraints, ctx: &MeasureContext) -> Size`（traits.rs:63）✓ **回靠 B.1**
  - `EventResult<M> { Handled, Prevented, Continue(M) }`（traits.rs:76-83）✓ **回靠 B.1**
  - `MeasureContext`（context.rs:28）✓ **已补全定义**
  - `BoxConstraints`（geometry.rs:60）✓ **已补全定义**
- **D3 审查的 P1-C1 判定为已闭环解决**，D4 契约一致性**达标**。

### 5.2 防火墙（总监第 5 问）
- flow：`rgui-core` 依赖仅 taffy（optional/纯 Rust）；`rgui-render`/`rgui-platform` 各自只依赖 `rgui-core`（无重型依赖在 core 内引入）。与 D3 一致，**防火墙仍成立**。
- 未引入 wgpu/winit/vello/cosmic-text/cssparser/accesskit/rhai 任何违规依赖（D4 阶段 0 正确）。✓

### 5.3 契约签名未再改（总监第 5 问）
- `WidgetSpec`/`AppMessage`/`PersistState`/`EventResult` 签名与 greenfield B.1 一致，无新的未定义项。✓

---

## 六、Q6 — unused warning 判断（P2，占位遗留非设计缺陷）

总监问"`StateStore.state` 字段未读等 unused warning 是否仅占位遗留还是设计缺陷"。核查结论：

- **`StateStore<S>`（state/mod.rs:36-41）**：字段 `state: InstanceState`（**从未被读取**，仅 `new` 初始化后闲置，mod.rs:47）、`_marker: PhantomData<S>`。**这是深占位**——`StateStore` 泛型 `S` 完全未使用（PhantomData 占位），`state` 字段无 getter/setter。**占位遗留，非缺陷**：D4 的 `diff`/`snapshot` 是**自由函数**（`diff(a,b)`、`apply_patch`），不经过 `StateStore`——所以 `StateStore` 只是个"类型壳"。**P2**：D5 实现真正的状态宿主（或删掉这个误导性字段）。
- **`InstanceState`（mod.rs:83）**：`#[allow(dead_code)] PhantomData<()>`——已显式 allow，是占位。**无问题**。
- **`widget_state.rs` 整文件**：`WidgetState` 空占位 + `_persist_marker` 占位函数（widget_state.rs:20-21，`#[allow(dead_code)]`）。与 `state` 子模块**职责重叠**（widget_state.rs:1-2 自己都注释了"与 state 子模块区分"）。**占位遗留**，D5 决定保留/删除（避免"单组件状态 vs 全局状态存储"两套概念的割裂）。
- **`registry.rs`**：`register` 函数体空（registry.rs:28-30，只注释不执行），`WidgetRegistry.inner` 已定义但 register 未用。**占位**，d4_acceptance_core.rs:171-175 的 `r1_register_duplicate_rejected` 测试**只调用不断言**（实际 register 为空实现，任何注册都"成功"且无逻辑）——测试名与实际行为不符（"rejected"但无拒绝逻辑）。**P2**（测试断言缺失，且 registry 是空壳）。

**综合判定**：全是**深占位 + 待 D5 实现**，**非设计缺陷**。但建议 D5 明确这些占位的走向（实现 or 删除），避免"看似已支持 state/store/registry 实则空壳"误导 D5 集成测试。

---

## 七、P0/P1 风险清单 + MERGE GATE 建议

### P0 风险清单：**无（P0 清零）**

### P1 风险清单（2 项）
| # | 风险 | 位置 | 处置建议 |
|---|---|---|---|
| P1-1 | **diff/apply_patch 子节点结构不收敛**（已复刻验证 `applied == b ? false`） | diff.rs:22-47 / 50-70 | D4 放行时带回**修复工单**：diff 需递归处理子节点（增删子节点生成插入整棵子树的 patch），并补一个**多子节点 roundtrip 收敛测试**（现在 `a5` 测试只测根 props，是假阳性覆盖） |
| P1-2 | **diff 无 key-based reconcile**（位置型，子节点顺序/增删会被误判或丢失） | diff.rs:37-44 | 与 P1-1 协同；至少在契约/文档中明确"当前 diff 仅支持根 props 变更，不支持树结构调整"，避免 D4 验收误读为完整 diff。若 D5 需要列表重排，引入 key-based reconcile |

### MERGE GATE 建议：**有条件放行（CONDITIONAL PASS）**

- **P0 清零**，核心结构（Coordinator 闭环、契约一致性、防火墙、无重型依赖、Taffy 不泄漏）**均达标**，D3 遗留的契约漂移已修复——这些是**放行 D4 的充分理由**。
- **但 P1-1（diff 不收敛）是 D4 验收目标"状态一致性"的直接反例**，属"实现与验收标注不符"。因此：**放行 D4 进入 D5，但 P1-1 须进入下一迭代待办（并回给 dev 修复工单），不得被当作"已完成正确 diff"冻结**。
- 理由：diff 是状态层核心，若带着缺陷进入 D5+ 的真实组件树（Accordion/WaBadge 含子节点），会在树结构调整时产生**状态视图与目标不一致**的 bug。当前 demo 只测根 props（单节点），未暴露——**这是 TDD 覆盖盲区放大了缺陷**，需要在扩散前处理。

### P2 观察清单（随 D5 处理，不阻塞放行）
1. `to_taffy_style` / mapping 是**死路径**（compute 未调用），`LayoutStyle.grow` 无实际效果——让 compute 接入 mapping，或删 mapping 死代码。
2. `Snapshot.instances` 用裸 `u64` 而非 `WidgetId` newtype，丢类型安全。
3. `dispatch` 用 `&ViewContext::default()` 重建视图，D5 有真实 ctx 时会丢上下文。
4. `StateStore`/`widget_state`/`registry` 为深占位，`r1_register_duplicate_rejected` 测试名与行为不符（register 为空实现）；D5 明确走向。
5. core `default=["layout"]` 默认拉入 taffy——与"core 默认零依赖"张力，D5 权衡。

---

*审查方：devco-reviewer｜只读审查，diff 收敛性通过隔离复刻程序验证（未修改项目代码）。建议放行 D4（P0 清零，契约/防火墙/核心循环达标），但 P1-1（diff 不收敛）必须回填修复工单并补真正 roundtrip 测试后方可视为"状态差分正确"。*
