# rgui 全新构建 · 任务分解与验收标准（tasks.md）

> 工作目录：`/Users/chenchao/Documents/code/rust/RUST-GUI`
> 主管线：devco-director（总监）
> 阶段：**D10 完成 ✅ 且代码+文档已入库。文档同步铁律落地：核心 3 份（D0/D11/CLAUDE.md）+ greenfield 已与代码一致（reviewer 复审 PASS）。待 doc 补 D 系列（D1-D10）后推进 D11。**

> 已提交：fa742c3(核心3份+greenfield B3B5) / 2347a4d(D0回填) / 96adeb7(greenfield 8点对齐+refactor失效标注) / 9b4017b(dev feature修复：platform winit默认启用+app.rs门控)。代码与远程同步，剩余未提交仅 tasks.md + reviewer 核对报告（文档）。

---

## 🔒 铁律：文档同步（单次任务，强制，每次完成必须执行）

**文档是开发根基（不可省略，不是冗余）。重构的目标是代码简洁，文档绝不随之精简。文档必须与代码完全一致（single source of truth）。每一次任务/里程碑完成后，必须同步更新相关文档，且必须被检查。绝不允许"代码改了、文档不同步"或"为降代码复杂度而省略文档"。**

> 用户明确：文档是开发的根基，必须与代码一致。文档全集（D 系列 + 核心文档）不能省略；重构只简化代码，不简化文档。

### 规则
1. **每个任务/里程碑完成的验收标准，必须包含「文档同步」检查项**（不是可选项）。
2. **「文档」定义**（本项目的权威文档体系，必须与代码一致）：
   - `docs/D0-总体设计.md` —— 顶层设计权威（crate 拓扑/接口契约/模块边界）。**代码实现若与它偏离，必须先更新它或说明理由。**
   - `docs/D11-Cargo结构与发布策略.md` —— 5-crate 结构 + feature + 发布。
   - `docs/D1-D10 等 D 系列` —— 各子系统设计（D1 WidgetSpec/D2 状态/D3 渲染/D4 样式/D5 事件/D6 无障碍/D7 开发反馈/D8 任务分解/D9 测试策略/D10 组件规范）。**全套存在且与代码一致。**
   - `tools/2025-09-01_rgui-greenfield-architecture.md` —— 架构唯一权威（greenfield 全集蓝图）。
   - `CLAUDE.md` —— 项目入口说明（crate 列表/常用命令/约定）。
   - `tasks.md` —— 任务分解 + 验收标准 + 阶段状态。
   - 各阶段 `tools/*-risk-review.md` —— 审查记录。
3. **分工**：dev 改代码后**必须在同一任务里指出哪些文档受影响**；doc 负责实际更新文档；reviewer/qa 验收时**必须核对「代码 ↔ 文档」一致性**，不一致则判不通过。

### execute 流程（每次任务）
- dev 完成编码 → 在交付报告里列「受影响文档清单」
- doc 据清单同步更新文档（D0/D11/D 系列/CLAUDE.md/greenfield 等）
- reviewer/qa 验收：**核对代码与文档一致**（crate 拓扑/API 签名/命令），不一致 → 拒绝放行
- 总监：核对文档同步已执行 + 一致性检查通过，才放行进入下一阶段

### 检查点（验收必须包含）
- [ ] 代码与 `greenfield-architecture.md` / `docs/D0`（crate 拓扑/接口契约）一致
- [ ] 受影响 D 系列文档已同步（D1-D10 全文案与代码一致）
- [ ] `tasks.md` 阶段状态已更新
- [ ] `CLAUDE.md`（crate 列表/命令）与当前结构一致
- [ ] `docs/D11` Cargo 结构一致
- [ ] reviewer 已核对代码↔文档一致性

---

## 存量文档同步（D3-D10 欠账）
> 用户确认：按 greenfield 全集重建 D1-D10 + 核心 3 份（docs/D0 + docs/D11 + CLAUDE.md）。文档非冗余，必须与代码一致。
- doc 先建核心 3 份（D0/D11/CLAUDE.md），再按 greenfield 全集补 D 系列
- 完成后 reviewer 核对代码↔文档一致 → 总监验收

## 状态
- 旧代码已删除并 push（commit `ae456fe`，历史经 tag `legacy-6631706` 完整保留）
- D3-D10 均通过（四层确认）
- **D10 四层确认**：Accordion（AccordionState{expanded}+Toggle）+ WaBadge 完整 WidgetSpec；App::run(config,...) 暴露 config；窗口渲染 Accordion + 交互切换（收起 "Settings [+]" 无内容 / 展开 "Settings [-]" + 内容区，--expanded 参数 + vision 截图对比确认）；45 测试全绿；core 零 GPU/DAG 无环/单一 vello/winit
- **D10 交互对比**：--expanded CLI 参数（需 `-- --expanded` 传参）绕过 macOS 辅助功能权限，收起/展开两态截图对比（qa 按窗口 ID）
- **D10 P2（D11 待办）**：P2-1 mapper 无 hit-test（D11 多组件需事件路由，重点）；P2-2 跨平台 display_handle 未验证；P2-3 展开态文本右侧被窗口截断（D11 文本水平布局/换行优化）；P2-4 offscreen 手工 rect；增量单向
- 证据：tools/qa/d7_screenshots/（d10_final_collapsed.png / d10_final_expanded.png / d10_accordion_collapsed_verified.png）
- 设计基线：`tools/2025-09-01_rgui-greenfield-architecture.md`（5-crate 蓝图）
- 审计报告：`tools/2025-09-01_rgui-complexity-audit.md`
- 风险审查（重构）：`tools/2025-09-01_rgui-refactor-risk-review.md`
- D3 风险审查：`tools/2025-09-01_rgui-D3-risk-review.md`

## 目标 crate 拓扑（5 个，architect greenfield 定稿）
| crate | 职责 | 依赖隔离 / 硬约束 |
|---|---|---|
| `rgui-core` | 唯一逻辑核心（WidgetSpec/AppMessage/状态/diff/snapshot/布局/样式/组件/无障碍树），吸收 state+layout+style+components+a11y | 纯 Rust，**零 GPU/零平台/零 cssparser**（Cargo 依赖防火墙） |
| `rgui-render` | 渲染引擎，单一 vello 后端 | wgpu/vello/cosmic-text 隔离；删 skia；与 platform 互不相依 |
| `rgui-platform` | 平台层（窗口/事件循环/输入/IME/焦点） | winit 隔离；与 render 互不相依 |
| `rgui-macros` | 过程宏（derive + html!） | **proc-macro 必须独立** |
| `rgui` | 薄 facade（重导出 + 极薄启动协调 ≤200 行） | 依赖全部 4 个；仅装配 |

### 硬性原则
- **Cargo 依赖防火墙**：`rgui-core/Cargo.toml` 的 `[dependencies]` 绝不含 rgui-render/rgui-platform/rgui-macros/winit/wgpu/vello/cosmic-text。
- **依赖 DAG 无环**：render/platform/macros 只向下依赖 core；facade 依赖全部；render 与 platform 互不相依；core 绝不依赖任何外围。
- facade 公共 API 再导出清单与被合并模块一一核对，防悬空 use。
- lint 克制：仅 `unsafe_code = deny`；clippy default；todo/expect/unwrap 放宽到 warn。
- 组件统一 Tier 1 WidgetSpec；废弃 .rgui/.rhai/skia/script/devtools。
- 增量编译验收：**改数据/状态层（core::state）→ 不重编 render**（非"改 core"）。

## 待用户拍板的 4 处落点（architect greenfield §G）
① 是否保留薄 facade `rgui`（倾向保留）② 阶段 0 是否含样式系统 .rgss（建议 P1 增量）③ 内置组件范围（建议仅 Accordion+WaBadge）④ 拖放（建议 P1 不做）

## D3 — 从零 scaffold（最小可编译骨架）
**目标**：按 5-crate greenfield 蓝图，建立可编译的 workspace 骨架 + 核心接口契约（trait 签名占位），不实现业务逻辑。

**阶段 0 范围（用户已确认 4 落点）**：
- ✅ 保留薄 facade `rgui`（零配置 `App::run` 入口）
- ✅ 阶段 0 **不含**样式系统 .rgss 解析（`core::style` 仅占位模块，P1 再做）
- ✅ 组件范围 = **Accordion + WaBadge**（证 Tier 1 路径）
- ✅ 阶段 0 **不做**拖放（聚焦渲染+交互主循环）
- 渲染：单一 vello 后端；脚本/Rhai/devtools/skia 全部剔除

**验收标准**:
- [ ] workspace `Cargo.toml` 声明 **5 个成员**（core/render/platform/macros/rgui），无循环依赖（依赖图 DAG）
- [ ] 每个 crate 有 `src/lib.rs` + 模块骨架；`cargo check --workspace` 通过
- [ ] 核心 trait 契约（WidgetSpec/AppMessage/PersistState/主类型）以 `todo!()` 或最小定义占位，签名符合设计文档
- [ ] `rgui-macros` 为独立 proc-macro crate
- [ ] `core::style` 仅占位（P1 实现），不引入 cssparser
- [ ] `core::components` 含 Accordion + WaBadge 占位
- [ ] lint 配置克制（仅 `unsafe_code = deny`）
- [ ] 增量编译验证：改 `core::state` → 不重编 render

**增量编译验收口径（P1-C2 明确，D4 前回靠）**：
- Cargo 模型下 `rgui-render` 依赖 `rgui-core`（按 crate 粒度），任何 core 源码变更都会触发依赖它的 crate 做**接口兼容性校验**，但**不重编其实现**（已实测：改 core 后 `cargo check -p rgui-render` 0.19s finish，无 render 重编）。
- 本验收以**"改 `core::state`（仅数据层类型变更，不新增/改变 render 依赖的符号）→ 观察 render 不重编"**为口径。
- **crate 级口径的边界**：Cargo 无源码级增量（不做文件级 diff 决定是否重编依赖），因此"改 core 绝对不重编 render 的实现"在 crate 粒度下天然成立（render 编译产物不因 core 接口未变而重编）。
- **替代口径（若需更强隔离）**：引入**显式接口边界**——core 的 state 数据结构变更通过不泄漏到 render 的接口（render 仅依赖 core::geometry / core::traits 等稳定签名的纯类型）来隔离核心数据结构变更传播。state 内部字段变更不应要求 render 重编译任何实现代码。

## D4+ — 后续实现（TDD，dev 执行）

**阶段：D4 已放行（用户确认），按依赖顺序 + 小步快跑，核心实现 TDD**

### D4 目标：核心循环 + 状态 + 布局（TDD，严格 RED-GREEN-REFACTOR）
**顺序**（按依赖）：
1. **WidgetSpec 核心循环**：view/update/paint 生命周期 + WidgetRegistry + WidgetNode，跑通"状态变化 → 视图更新 → 重绘"最小闭环（先单测，不接渲染）。
2. **状态管理**：StateStore + diff(Patch/apply_patch) + snapshot（纯 Rust 单测，validate diff 正确性）。
3. **布局（Taffy 集成）**：LayoutEngine 封装 taffy，从 LayoutStyle 提取 → Taffy Style，minimal 布局测试。

**TDD 铁律**：先写失败测试（RED）→ 实现（GREEN）→ 重构（REFACTOR）；严禁先写实现再补测试；写代码前先写测试，跑通确认失败。

**节奏**：先跑通一个最小可运行示例（一个组件 → 渲染到屏幕），再全面铺开。D4 目标是"一个 widget 能看到"。

**验收标准**:
- [ ] WidgetSpec 核心循环最小闭环通过单测（view → update → view 更新）
- [ ] state diff/snapshot 单测全绿（Patch 应用正确）
- [ ] 布局 Taffy 集成单测全绿（LayoutStyle → Taffy Style 映射）
- [ ] 最小示例 `examples/demo`（或测试）运行，一个组件渲染到目标（离屏或无窗口验证）
- [ ] cargo check/test/clippy(核心安全类)/fmt 通过
- [ ] 严格 TDD：每步有 RED 测试证据

### D4 后续（验收通过后，视进度再拆）
- 渲染管线（vello 集成）→ 组件（Accordion/WaBadge）→ E2E

---

## D5 — 渲染管线（vello 离屏）【已通过，归档】

**阶段：D5 已完成（P0 清零）。先离屏渲染，单一 vello 后端。**
设计基准：greenfield §B.2（render 契约）

**目标**：先搭离屏渲染（vello 无窗口渲染到纹理/图像）+ 场景图接入 core 的 WidgetView，证明"能画出来"，再接窗口。

**范围**：
- `rgui-render/backend/vello.rs`：Vello 离屏渲染（RenderBackend = Enum 仅 Vello；渲染到纹理/CPU 图像，无窗口）
- `SceneGraph`：draw 指令列表（从 core::view::WidgetView 转换而来）
- `GlyphKey/GlyphAtlas/TextShaper`：字形缓存与文本整形（cosmic-text 接入）
- feature 门控：`vello-backend`（wgpu/vello/cosmic-text/fontdb 重型依赖经 feature 引入，D5 实现）
- 保持单一路径，不预留 skia

**验收结果**（dev + 总监复核 + reviewer 审查）：offscreen 离屏测试真实通过（像素断言）；43 测试全绿；core 防火墙成立；SceneGraph 纯度；全仓零 unsafe。P0 清零。

**【遗留 P1 → D6 处理】**：P1-1 offscreen 用手工 red_filled_rect 绕过 from_view（占位）；P1-2 from_view 转换是占位（未用 LayoutEngine/Props 映射）。

---

**阶段：D6 已放行（用户确认）——先离屏验证真实 from_view 转换正确（clean P1-1/P1-2），再接窗口。**
设计基准：greenfield §B.2/§C.2（render 契约）+ §C.1（layout 接入）

**目标**：把 D5 的"手工红色矩形"换成**真实的 WidgetView→SceneGraph 转换**，接布局 + 完整 Props 映射 + 文本路径，从离屏链路清掉 P1-1/P1-2，像素级 verify。

**范围**：
- `from_view(WidgetView)` 补全：递归转换 WidgetView 树 → SceneGraph/draw 指令；处理完整 Props（Color/Size/Str/Int/WidgetId 等，不再静默忽略）；文本路径接入（text.rs 补全 GlyphAtlas/TextShaper）
- **布局应用**：from_view 生成 SceneGraph 时调用 LayoutEngine（从 core::view::WidgetView 提取布局信息 → Taffy → 得到实际 bounds → 对应 draw），让布局真正作用于渲染
- 真实端到端测试：一个真实 WidgetView（含布局 + props + 文本）→ from_view → SceneGraph → 离屏渲染 → 像素级断言（替换 D5 的手工 rect 测试）
- feature 门控保持（vello/cosmic-text 等经 feature 引入）

**验收标准**:
- [ ] `from_view` 从真实 WidgetView 正确生成 SceneGraph（含布局应用、完整 Props、文本）
- [ ] 离屏端到端测试：真实 WidgetView（布局+props+文本）→ 像素级验证（替换手工 rect）
- [ ] 布局真正作用于渲染（从 WidgetView 布局信息算出实际 bounds → draw）
- [ ] core 零 GPU 防火墙仍成立
- [ ] cargo test --workspace --all-features 全绿
- [ ] 严格 TDD：真实 from_view 端到端测试先 RED，实现后 GREEN
- 窗口/事件循环（winit surface）留 D7（本阶段先离屏验证转换正确性）

---

## D8 — 收敛窗口逻辑进 rgui-platform（补齐 §C.3 契约）

**阶段：D8 已放行（用户确认优先项 = 收敛窗口逻辑进 platform）。**
设计基准：greenfield §B.3/§C.3（platform 契约）。reviewer P2-1 关键项：window_demo 直接 winit/wgpu 绕过 platform（契约名存实亡）。

**目标**：把 `window_demo` 里直接 winit/wgpu 的**窗口创建 + 事件循环 + surface 渲染**收敛进 `rgui-platform`（+ 必要配合），让 facade / 示例走 `rgui-platform` 公共 API，而非绕过它。补齐 greenfield §C.3 契约。

**范围**：
- `rgui-platform`：封装 winit Window 创建 + EventLoop (ApplicationHandler) + 事件循环，暴露 `platform` 级公共 API（window create / event loop / resize / input / close）。
- `rgui-render`：surface 创建/渲染封装（create_surface + resize + present），方便 platform/facade 调用。
- facade `rgui` app：通过 `rgui-platform`（而非直接 winit）创建窗口 + 事件循环 + 调用 render surface 渲染 + 事件→Coordinator→重绘。
- window_demo 改为走 platform 公共 API（经 facade），不再直接引 winit/wgpu。
- 保持单一 vello/winit；core 零 GPU/平台防火墙（window 逻辑在 platform，渲染 surface 在 render）。

**验收标准**:
- [ ] `rgui-platform` 提供 window + event_loop 封装公共 API（window create/event loop/resize/close）
- [ ] window_demo / facade 走 platform（源码无直接 winit:: / wgpu:: 引用，或仅在 platform/render 内部）
- [ ] 窗口仍能弹出并渲染组件（首帧稳定），截图验证不回归
- [ ] core 零 GPU/平台防火墙保持；DAG 无环
- [ ] cargo test --workspace --all-features 全绿
- [ ] 严格 TDD 或行为验证：窗口能创建+运行（不因收敛而破坏）

**后续 D8 续**（本阶段验收后）：Accordion/WaBadge 组件、文本真实字形、key-based reconcile、q a 交互前后对比。窗口逻辑收敛是其余依赖的基座。

---

## D7 — 窗口 + 事件循环（winit surface）【已通过，归档】

**阶段：D7 已完成（首帧时序修复）。**
设计基准：greenfield §B.3（platform 契约）+ §C.3

**目标**：把离屏渲染接入真实窗口：winit surface + 渲染到窗口 + 事件→更新→重绘，让真实组件显示在窗口里并响应交互。

**范围**：
- `rgui-platform`：window/event_loop（winit surface 创建 + 事件循环）+ focus/input
- `rgui-render`：接入窗口 surface（从离屏扩展到 surface 渲染）
- facade `rgui` app.rs：组装 window + 事件循环 + 渲染 + 事件→Coordinator 更新→重绘（minimal）
- **P2-1 嵌套布局坐标累加**（已修）
- showcase 示例：真实组件在窗口显示并响应（按钮点击改变内容）

**验收结果**（dev + 总监实测截图 + qa 截图 + reviewer 审查）：窗口首帧时序修复（ControlFlow::Poll + about_to_wait 每帧 request_redraw）后，窗口弹出即稳定渲染蓝色按钮组件（不依赖 focus）；P2-1 已修；全量测试全绿；reviewer PASS（P0 清零）。截图证据 tools/qa/d7_screenshots/d7_final_verified.png。

**⚠️ 用户指令（硬约束）：可视化验证必须靠实际截图证明（渲染到窗口真实可见），发布工单前先测试截图；截图作为验收证据递交。**

**遗留 → D8**：窗口逻辑收敛进 platform（替换直接 winit/wgpu）、文本真实字形、q a 交互前后对比、key-based reconcile、P1 跨平台。

---

## D9 — facade 入口统一 + 文本字形 + 按需重绘

**阶段：D9 已放行（用户确认：facade 入口统一 + 文本字形 + 按需重绘一起做，完整可用框架入口 demo）。**
设计基准：greenfield §B.5（facade 契约）。reviewer P2-1/P2-3/P2-8 + P1-1。

**目标**：实现 facade 统一入口 `App::run()`（让用户走 facade 而非绕过），接文本真实字形（cosmic-text 替换矩形块），改按需重绘（避免无条件每帧高 CPU）。目标产出：用户一个统一入口跑窗口渲染组件 + 清晰文本。

**范围**：
1. **facade 入口统一**：实现 `rgui::App::run()`（app.rs 的 todo! 补全）——让用户通过 `App::run()` 启动窗口 + 事件循环 + 渲染 + 交互，内部走 `rgui_platform` + `rgui_render`。window_demo 改为经 `rgui::App::run()`（不再绕过 facade）。
2. **文本真实字形**：rgui-render 接 cosmic-text 真实字形（fontdb + TextShaper + GlyphAtlas + draw_glyphs），替换当前 Str→DrawText 的矩形近似块，让文本清晰可辨。
3. **按需重绘**：about_to_wait 从"无条件每帧 request_redraw"改为"按需（有变更/dirty 标记才重绘）"，避免高 CPU（保留首帧触发）。
4. 清理 facade 残留 `dep:winit/dep:wgpu`（经 platform/render re-export，或仅在 facade 需要时保留）。

**硬约束**：单一 vello/winit；core 零 GPU/平台；platform/render 互不相依；DAG 无环。
**交付节奏**：先让 `rgui::App::run()` 能启动窗口跑起来（window_demo 走 facade），再接文本字形，再改按需重绘。目标 = 用户统一入口跑窗口 + 组件 + 清晰文本。

**验收标准**:
- [ ] `rgui::App::run()` 实现，window_demo 经 facade 启动（不再绕过）
- [ ] 文本清晰可辨（cosmic-text 真实字形，替换矩形块）
- [ ] 按需重绘（无变更不重绘，CPU 下降），首帧仍稳定渲染
- [ ] 窗口弹出可见 + 渲染组件 + 清晰文本（截图验证）
- [ ] core 零 GPU/平台；DAG 无环；单一 vello/winit
- [ ] cargo test --workspace --all-features 全绿
- [ ] 严格 TDD/行为验证：App::run() 启动窗口 + 文本渲染 + 按需重绘

---

## D10 — Accordion/WaBadge 组件 + 真实交互 demo

**阶段：D10 已放行（用户确认优先项 = 做真实业务组件 + 交互 demo）。**
设计基准：greenfield §B.1（组件契约 WidgetSpec / Tier 1）。核心循环已验证，现在接真实业务组件，验证 WidgetSpec 完整生命周期。

**目标**：实现 Accordion / WaBadge 组件（Tier 1 WidgetSpec Rust 实现），接真实交互（点击/展开/收起），做出一个真实可交互组件的完整 demo。从"窗口能用"到"真实组件"。

**范围**：
1. **Accordion 组件**：`rgui-core::components` 实现 Accumordion（折叠/展开容器，点击标题切换展开，子内容显示/隐藏），`impl WidgetSpec`。含真实交互（点击标题 → update → view 更新 → 重绘）。
2. **WaBadge 组件**：实现 WaBadge（徽章/标签，含整数 label 显示），`impl WidgetSpec`。
3. **真实交互 demo**：窗口显示 Accordion + WaBadge，点击 Accordion 标题展开/收起（交互完整生命周期验证），文本显示对应状态。
4. **App::run 暴露 config**（可选，若顺手）：让 `App::run` 支持 config（标题/尺寸），消除 AppConfig 死代码。
5. 保持统一入口（window_demo 或新例经 facade `App::run`）。

**硬约束**：Tier 1 WidgetSpec（纯 Rust）；core 零 GPU/平台；单一 vello/winit；DAG 无环。
**交付节奏**：先 Accordion（折叠/展开 + 交互），再 WaBadge，再做组合 demo（窗口显示两者 + 交互）。目标 = 用户窗口跑真实可交互组件。

**验收标准**:
- [ ] Accordion 实现（展开/收起 + 点击交互 → view 更新 → 重绘）
- [ ] WaBadge 实现（整数 label 显示）
- [ ] demo 窗口显示 Accordion + WaBadge，点击 Accordion 标题切换展开/收起（可视化截图验证交互）
- [ ] WidgetSpec 完整生命周期（view/update/measure/paint）验证通过
- [ ] core 零 GPU/平台；DAG 无环；单一 vello/winit
- [ ] cargo test --workspace --all-features 全绿
- [ ] 严格 TDD：组件行为测试先 RED 后 GREEN + 截图验证交互




