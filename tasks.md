# rgui 全新构建 · 任务分解与验收标准（tasks.md）

> 工作目录：`/Users/chenchao/Documents/code/rust/RUST-GUI`
> 主管线：devco-director（总监）
> 阶段：**D22 复核通过 ✅（生产级日志集成，dev 交付 + 总监实测复核，D21 T1-T7 全量回归零改动通过）。待 reviewer 审查 + doc 同步 + qa 资产收尾。**

> D22 已验证：tracing+subscriber 入 workspace（MSRV 1.85）；测试信号→stdout 纯 token（D21 脚本零改动）；库日志→stderr（app_start/window_created/vello_init）；RUST_LOG=off 帧路径零日志；81 测试全绿。提交：f204985(dev 功能)+fbf4606(dev Cargo.lock)+fe3871d(qa 脚本 strip)+e788df2(qa gitignore pyc)+1519d1e(设计/审查文档)。

---

## D22 — 生产级日志集成（tracing + 测试信号迁移）

**阶段：D22 已启动（用户确认「根据日志方案为项目添加日志」，方案见 tools/2025-09-01_rgui-logging-design-final.md）。** 按铁律走完整质量链。

**目标**：按最终设计文档实施生产级日志——`tracing`（库日志）+ 测试信号迁到 `rgui_test_signal` target，保持 D21 脚本可解析。

**选型（已定，调研确认）**：`tracing = "0.1.41"`（features `["std","log"]`）+ `tracing-subscriber = "0.3.20"`（features `["fmt","ansi","env-filter","registry"]`）+ `log`（桥，收 wgpu/vello）。MSRV 1.85 兼容（subscriber 0.3.20=1.63），无 async 冲突（用同步面）。

**范围**（dev 主实现）：
1. **Cargo 依赖**：workspace.dependencies 加 tracing/tracing-subscriber/log。
2. **subscriber 注册**：facade(App::run) 或 demo 入口早期注册 `tracing_subscriber::fmt()` + `EnvFilter`；库日志→stderr，测试信号→stdout（双 Layer/双 target）。
3. **库日志埋点**（logging-design-final.md §四）：app.rs app_start/app_shutdown/render_error；event_loop window_created/closed；render vello_init/glyph_atlas_rebuild/render_slow；core layout_dirty；focus focus_changed；style style_parse_warn；scene_graph scene_node_count。**热路径不埋 info+**（流式编码铁律）。
4. **替换 eprintln**：app.rs:205 render_error → `tracing::error!`；app.rs:190 win-frame → 迁 `rgui_test_signal`。
5. **测试信号迁移**：window_demo/d20_modal 的 `[hit-region]`/`[mouse-event]`/`[hit]`/`[action]`/`[focus]` 从 `eprintln!` 改为 `tracing::info!(target:"rgui_test_signal", "<原 token>")`，**message 文本 = 原 token 不变**（保证 D21 脚本正则零改动）。信号用轻量 event（不用 span）。

**【硬约束】流式编码优先；文档同步（D5/D9/tasks.md 标注库日志 vs 测试信号分层）+ 每阶段 git add + commit + push；受影响文档清单照列。**
**【如实标注】** 若 tracing fmt 无法做到"测试信号纯 message 输出"，如实标注并给 qa 退路（脚本 `_strip_tracing` ≤3 行）。

**验收标准**：
- [ ] tracing + subscriber 纳入 workspace.dependencies；MSRV 1.85 下 `cargo check --workspace` 过
- [ ] 原 2 处 eprintln! 替换（render_error→tracing::error!；win-frame→rgui_test_signal）
- [ ] 库日志埋点完成（§四清单），热路径无 info+ 埋点
- [ ] 测试信号迁 target="rgui_test_signal"，message=原 token
- [ ] **D21 T1-T7 全绿（脚本正则零改动或仅 ≤3 行 strip 适配）**
- [ ] `RUST_LOG=off` 帧路径零日志开销（性能基准）
- [ ] `cargo clippy -- -D warnings`（保留 unsafe_code=deny）过
- [ ] 文档同步 + 每阶段提交（铁律）

---

## D21 — 鼠标键盘自动化测试模式（真实注入 + 调试日志闭环）

**阶段：D21 完成 ✅（三大根因修复 + 全链路跑通）：①坐标单位 point（CGEvent 全局坐标单位是 point，非物理像素）②CGEventCreateMouseEvent 的 CGPoint 按值传参（ctypes 声明修正，旧 c_void_p 读到寄存器垃圾光标落在 (0,0)）③窗口需 AXRaise 置顶（activate 只激活进程不保证 z 序最前）。验证结果：点击 Accordion (870,261)pt→[action] toggle ✅；WaBadge×2→badge_click count=1→2 ✅；Tab×2→[focus] Some(1)/Some(2) ✅。代码提交 3d0f72a。**

**目标**：建立"真实鼠标/键盘模拟"自动化测试模式——测试脚本读取项目调试日志（组件命中区坐标/窗口 frame/scale），换算成真实屏幕坐标，注入真实鼠标点击/键盘事件，断言 demo 实际响应（日志+截图），输出 BUG 报告回馈给 dev 修复。**"具体点哪里"由项目代码调试日志提供**（非脚本硬编码）。

**范围**：
1. **dev 调试日志**（关键：坐标来源）：
   - demo/组件层输出每个可交互组件的**命中区 rect（逻辑坐标）**（如 `[hit-region] id=1 accordion rect=(x,y,w,h)`）；hit-test 本就基于这些区域，只需在初始化/布局时打日志。
   - 输出**窗口 frame（屏幕位置）+ scale_factor**（供脚本坐标换算）。
2. **qa 测试脚本**：
   - 读调试日志 → 组件 rect + 窗口 frame + scale；
   - **坐标换算**（重点难点，point 体系）：屏幕绝对坐标 = CGWindowList 窗口 bounds 左上角(point) + 标题栏高 + 组件 hit-region 中心(逻辑 point)。CGEvent 全局坐标单位是 **point**（非物理像素）；window 的 `[win-frame]` 物理像素需换算。旧公式「窗口屏幕原点 + 组件逻辑坐标 × scale_factor」在 Retina scale=2 下坐标放大 2 倍，点击落在窗口右侧远处（已修正为 point 体系）；
   - **真实注入**：CGEvent 鼠标点击（leftDown/Up）/键盘 Tab/Shift+Tab；
   - **断言**：注入后 demo 日志是否出现预期事件（如 `[click] hit=1`、`[focus] -> Some(2)`）+ 截图 vision 确认视觉变化（Accordion 展开/焦点位移/模态开启）；
   - 期望 vs 实际不一致 → 输出**结构化 BUG 报告**（case/点击坐标/期望/实际）→ 回馈 dev。
3. **demo 支持**：window_demo 打命中日志、可响应测试（Accordion 展开、Tab 切换焦点、模态开启——用现有 D20 模态）。
4. **闭环验证**：跑一个完整 case（真实点击 Accordion → 断言展开；Tab → 断言焦点切到 WaBadge；开模态 → 断言焦点隔离），确认"读日志→注入→断言→报告"全链路通。

**【铁律】流式编码优先；文档同步（D5 事件文档 + tasks.md 标注测试调试日志用途）；每阶段 git add + commit（代码+同步文档+tasks.md 状态）+ push。受影响文档清单照列。**
**【如实标注】坐标换算成败是核心验收点；若 Retina/多屏换算失准，如实标注并用截图 vision 兜底，不虚构。"

**验收标准**：
- [x] demo 输出组件 hit-region rect（逻辑坐标）+ 窗口 frame + scale_factor 调试日志
- [x] 测试脚本能读日志、**正确坐标换算**（point 体系，Retina 实测点中组件）
- [x] **真实点击 Accordion → 展开**（截图+日志双重确认）
- [x] 真实 Tab/Shift+Tab → 焦点切换（日志 `[focus] -> Some(id)` 确认）
- [ ] **真实开模态 → 焦点隔离（D20 模态）——案例未测试，如实标注未测试**
- [x] 断言 + BUG 报告闭环输出（结构化，含 case/坐标/期望/实际）
- [x] 全量测试保持全绿（不因加日志破坏）
- [x] 文档同步 + 每阶段提交（铁律）

> **验收结论（D21-2）**：T1-T7 全部跑通（qa 脚本 + 总监实测复核 exit 0）。分层诊断 detect_layer 按 L1→L5 定位首个失败层；零 LLM/零 vision（openai/requests/http/anthropic/socket 全 0）；预检 fail-fast（exit 2 区分环境 vs 产品 bug）；BUG 报告含 fail_layer+evidence+suggest；D5 已标 in-region/模态日志（commit 5a7f0f3）；doc 核对 D21-2 说明（commit fadef2f）；reviewer 放行 PASS（P0/P1 清零）。

### D21-2 遗留 P2（reviewer 观察，不阻塞，待优化）
- **P2-1**：dev 已打 `[mouse-event] in-region`（window_demo.rs:212），但 qa detect_layer L3 未消费 in-region——"坐标换算错 vs rect 边界不一致"未区分，坐标换算错会误归 L3（应 L2）。→ 待 qa 消费 in-region 精确区分 L2/L3。
- **P2-2**：d20_modal.rs 无 [mouse-event]/in-region，T7 的 L2 依赖 Tab 段代偿，modal_open 被当 L3 信号（变体只点不 Tab 会误报 L2）。
- **P2-3**：suggest（建议排查项）恒为空（main 未传）。
- **P2-4**：`--all` 不含 T7（合理，T7 可选）。
- **P2-5**：T5 `titlebar*0` 冗余。

### D21-2 ⭐ 分层诊断自动化测试（用户升级要求：脚本能判断问题在哪里 + 不调用大模型）

**目标**：在 D21 全链路跑通基础上，①保证全部交互路径可被鼠标/键盘触发（场景化覆盖）②脚本必须能**判断问题在哪里**（分层诊断，定位失败环节）③**脚本不调用大模型/无视觉判断**（纯确定性日志信号匹配）。

**分层诊断（Failure-Layer）——5 层，脚本顺次检查，首个失败层 = 问题所在**：
```
L1 注入层   脚本自检：AX trusted / CGEvent 加载 / 注入发出（点是否真的 post 成功）
L2 窗口层   winit 收到事件？看日志 [mouse-event] left-press at logical=(x,y) / [focus]
L3 命中层   hit_test 命中正确组件？看 [hit] id=1 / id=2 / id=none(missed)
L4 动作层   组件状态真正更新？看 [action] toggle / badge_click(count=N) / [focus] Some(id)
L5 功能层   二次注入验证持久效果（toggle 往返 / badge count 递增 / focus 移动）
```
脚本按 L1→L5 检查，**首个失败层 = 诊断结论**，BUG 报告输出 `fail_layer` + 该层证据 + 建议排查项（纯文本），全程零 LLM / 零 vision 判断（截图仅存证，人工可审）。

**场景化用例（T1-T7）**：
| # | 场景 | 注入 | 分层验收信号 |
|---|---|---|---|
| T1 | 点击 Accordion 展开 | 点 id=1 中心 | L3 [hit] id=1 + L4 [action] toggle |
| T2 | Tab 切换焦点 | Tab | L4 [focus] Some(1)→Some(2) |
| T3 | Shift+Tab 反向 | Shift+Tab | L4 [focus] Some(2)→Some(1) |
| T4 | 点击 WaBadge 计数 | 点 id=2 中心×2 | L4 badge_click count=1→2 |
| T5 | 点击未命中区（负向） | 点空白 | L4 [hit] id=none (missed) |
| T6 | toggle 往返（功能层） | 点 id=1 两次 | L4 toggle + L5 状态往返 |
| T7 | 开模态→焦点隔离（d20_modal，可选场景） | 开模态 | L4/L5 模态 focus 切换 |

**验收标准**：
- [ ] 脚本内置分层诊断器（detect_layer→[layer, evidence]），按 L1-L5 顺次检查、报告 fail_layer
- [ ] 脚本含注入前预检（AX trusted/CG 加载/窗口定位 fail-fast，区分"脚本环境问题 vs 产品 bug"）
- [ ] T1-T6 场景化用例（--case 单个 / --all 全量，退出码 0/1 接 CI）
- [ ] T7 模态场景（d20_modal，需 dev 输出可断言 modal 日志 + 激活/raise 协调）
- [ ] BUG 报告含 fail_layer + 该层证据 + 建议排查项（纯文本，**零 LLM / 零 vision 判断**）
- [ ] 全量测试保持全绿；文档同步（D5/tasks.md）+ 每阶段提交（铁律）

---

## D19 — 样式系统 + 样式驱动

> D19 已提交：4fa48fa(dev 样式驱动+Border.pad+AppConfig.stylesheet) / 737c6e3(dev fmt+D5补标) / 89723cd(doc greenfield §B.1+D0 补 ViewContext.styles)。74 测试全绿(69→74)。流式判据 PASS（iter().find()/链式 rule）。D19 闭合 D16 描边 pad 参数化。D19 P2：parse_rgss 仍占位（.rgss 文本解析留 P1，程序化构建经 StyleSheet::rule，不引 cssparser）。

---

## D19 — 样式系统 + 样式驱动

**阶段：D19 已启动（总监自主排期，用户授权完成全部开发）。**
设计基准：greenfield §B.1（StyleSheet/StyleRule）+ docs/D4（样式系统）+ docs/D10（组件规范）。

**目标**：把组件硬编码颜色/样式（D14 高亮色、组件配色）改为**样式驱动**——实现样式系统基础（StyleRule/StyleSheet，`core::style` 从占位到可用），组件从样式表取样式（颜色/描边/边框等），为后续 `.rgss` 解析/主题铺路。同时闭合 D16 的描边 pad 参数化（D19 可参数化）。

**范围**：
1. **样式系统基础**：`core::style` 从占位实现——`StyleRule{selector, properties}`（greenfield §B.1 已有类型）/`StyleSheet`，样式属性（color/背景/描边等）定义。
2. **样式驱动组件**：Accordion/WaBadge 配色/描边/字体等从样式表读取（而非硬编码 if-else 色值）。组件 view 查样式表取样式。
3. **默认样式/主题**：提供默认主题样式（当前硬编码色作为默认样式回退），组件在样式表命中时用样式、未命中用默认。
4. **描边 pad 参数化**（D16 P2）：描边 pad 经样式/属性参数化（而非硬编码 2.0）。
5. **`parse_rgss`**：greenfield §G 阶段0不含样式解析，D19 若投入可做基础解析；若过重，至少 StyleSheet 程序化构建（API 造样式），`.rgss` 文本解析留后续（如实标注）。
6. **demo 验证**：window_demo/d18_list 组件配色经样式驱动（改样式表 → 组件配色变，截图确认）。

【硬约束】流式编码优先；Tier 1 WidgetSpec；core 零 GPU/平台（样式在 core::style 纯 Rust）；DAG 无环；单一 vello/winit；不引入 cssparser（greenfield 硬约束）。
【每阶段提交】完成后 git add + commit（代码+同步文档+tasks.md 状态）+ push。受影响文档清单照列（D4 样式、D10 组件规范、D1 WidgetSpec、greenfield §B.1/§G 若有样式）。

完成后：①样式系统基础（StyleRule/StyleSheet）是否实现②组件样式驱动（配色/描边从样式表）③描边 pad 参数化（D16 P2）④parse_rgss 状态（实现/如实标注留后续）⑤流式编码⑥cargo check/test 结果⑦commit hash + 受影响文档清单。全程中文，TDD 先 RED 后 GREEN。

> D18 已提交：20006e1(dev keyed reconcile+动态增删 d18_list+focus 边界) / 1d6dc2d(doc greenfield §B.1+D0 补 WidgetView.key+MoveChild)。69 测试全绿(65→69)。流式判据 PASS（iter().map().collect()/position+any）。D18 闭合 D12 focused 残留边界 + D17 draw_text 高分屏复核。D18 P2：key 为 Option<u64> 手动分配无自动生成（无 key 子节点回退索引匹配），可考虑混合策略；diff_children_keyed 可优化减少两步 patch（效率微优化）。

---

## D18 — key-based reconcile + 动态增删组件

**阶段：D18 已启动（总监自主排期，用户授权完成全部开发）。**
设计基准：greenfield §B.1（WidgetRegistry/WidgetNode）+ docs/D2（状态 diff）+ docs/D10（组件规范）。

**目标**：实现 key-based reconcile（组件列表复用/按 key 定位）+ 组件动态增删（容器可增删子组件）——让组件容器/列表能正确增量更新（不整体重建），为构建列表/容器组件铺路。同时闭合 D17 的 draw_text 仿射组合顺序高分屏复核。

**范围**：
1. **key-based reconcile**：组件节点按 key（而非仅位置）识别/复用——更新子组件列表时，按 key 匹配复用已存在组件（move/update），而非按索引重建。核心在 Coordinator/WidgetNode 的 reconcile 逻辑。
2. **动态增删组件**：容器/列表支持运行时增删子组件（add/remove），并正确 reconcile（增加的补建、删除的移除、重排的复用）。demo（如一个可增删项列表）验证。
3. **focused 残留边界**（D12 P2 闭合）：focusable 列表动态变化时 focused 不置 None（增删后焦点正确）。
4. **draw_text 高分屏复核**（D17 P2）：`tf * translate` 组合顺序在多组件布局下实机验证。
5. **demo 验证**：可增删组件列表 demo（如动态 Accordion/WaBadge 项），key-based reconcile + 增删正确。

【硬约束】流式编码优先；Tier 1 WidgetSpec；core 零 GPU/平台（reconcile 在 core 逻辑层）；DAG 无环；单一 vello/winit。
【每阶段提交】完成后 git add + commit（代码+同步文档+tasks.md 状态）+ push。受影响文档清单照列（D2 状态 diff、D1 WidgetSpec、D10 组件规范、D5 若事件/焦点、greenfield §B.1 若有）。

完成后：①key-based reconcile 是否实现（按 key 复用非重建）②动态增删组件（add/remove 正确）③focused 残留边界（D12 P2）④draw_text 高分屏复核（D17 P2）⑤流式编码⑥cargo check/test 结果⑦commit hash + 受影响文档清单。全程中文，TDD 先 RED 后 GREEN。

> D17 已提交：2cfea91(dev 文本换行+render scale) / 05aea35(doc D3/D10/greenfield §B.2 补)。65 测试全绿(64→65)。流式判据 PASS（iter().position/line_y）。D17 闭合 D15 的 P2（渲染物理/逻辑尺寸混用）。D17 P2：换行宽用组件 size.width（若有 padding 需扣除，demo 无 padding 故正确）；draw_text 仿射组合顺序需高分屏实机验证（D18 复核）。

---

## D17 — 布局/文本换行（多组件布局 + 文本溢出）

**阶段：D17 已启动（总监自主排期，用户授权完成全部开发）。**
设计基准：greenfield §B.1/§C.1（layout）+ docs/D3（渲染）+ docs/D10（组件规范）+ docs/D2（布局）。

**目标**：解决文本溢出/换行 + 渲染层物理/逻辑尺寸统一——让组件文本（如 Accordion 展开态内容、WaBadge label）在窗口内正确布局（不截断、可换行），多组件同窗布局正确，且渲染 surface 尺寸与 SceneGraph 逻辑坐标匹配（Retina 无内容不匹配）。

**范围**：
1. **文本换行**：多行/长文本换行处理（cosmic-text 行布局，字形按行渲染，不溢出窗口/组件边界）。Accordion 展开态内容区文本、WaBadge label 文本溢出处理。
2. **渲染尺寸统一**：D15 P2——渲染 surface 用 inner_size（物理）vs SceneGraph 逻辑坐标，Retina 下内容尺寸不匹配。统一物理→逻辑（surface 尺寸也经 scale_factor 换算，SceneGraph 用逻辑坐标）。
3. **多组件布局**：同窗多组件布局正确（Accordion 左 + WaBadge 右，各自区域不重叠、边界正确）。
4. **demo 验证**：window_demo 多组件+文本布局正确（截图确认无截断、换行、边界合理）。

【硬约束】流式编码优先；Tier 1 WidgetSpec；core 零 GPU/平台（布局在 core::layout，文本整形在 render 但渲染层）；DAG 无环；单一 vello/winit。
【每阶段提交】完成后 git add + commit（代码+同步文档+tasks.md 状态）+ push。受影响文档清单照列（D2 布局、D3 渲染、D10 组件规范、D5 若文本/输入、greenfield §C.1 若有）。

完成后：①文本换行是否实现（多行/长文本不溢出）②渲染尺寸统一（物理→逻辑，Retina 无内容不匹配）③多组件布局正确④流式编码⑤cargo check/test 结果⑥commit hash + 受影响文档清单。全程中文，TDD 先 RED 后 GREEN。

> D16 已提交：b3ab6eb(dev StrokeRect+WidgetView.border) / 9c357d0(doc D3/D10/greenfield §B.2/D0 补描边)。64 测试全绿(63→64)。流式判据 PASS（iter().any()/if-let）。D16 闭合 D14 描边未实现项。D16 P2：描边 pad 硬编码 2.0（D19 可参数化）；获焦描边 = D14 背景变亮 + D16 描边叠加（视觉稍丰富，可选只留描边，功能无碍）。

---

## D16 — StrokeRect 描边边框（真焦点边框样式）

**阶段：D16 已启动（总监自主排期，用户授权完成全部开发）。**
设计基准：greenfield §B.2（render 图元）+ docs/D5（焦点/未实现项）+ docs/D10（组件规范）。

**目标**：实现描边矩形 draw 图元（StrokeRect），让获焦组件绘制**真描边边框高亮**（而非仅背景变亮，D14 的方向是背景高亮，D16 补描边边框样式），更接近真实 GUI 焦点边框。

**范围**：
1. **StrokeRect 图元**：render SceneGraph 加描边矩形 draw 指令（StrokeRect{rect,color,width}）——vello 绘制描边矩形（有 stroke API）。from_view 识别（WidgetView 若有 border prop/焦点描边 → StrokeRect）。
2. **core 组件边框**：WidgetSpec 组件声明边框（或获焦时产 Border 传 DrawCmd::StrokeRect），Accordion/WaBadge 获焦时描边边框高亮。
3. **demo 验证**：Tab/Shift+Tab 切换时获焦组件描边边框高亮（截图；离屏/单测验证 StrokeRect draw 指令），比背景变亮更清晰的焦点边框。
4. 保持 D15 逻辑坐标（scale_factor）；组件配色可用样式驱动（D19 再接入样式）。

【硬约束】流式编码优先；Tier 1 WidgetSpec；core 零 GPU/平台（描边是组件 view 的 draw 指令，render 渲染）；DAG 无环；单一 vello/winit。
【每阶段提交】完成后 git add + commit（代码+同步文档+tasks.md 状态）+ push。受影响文档清单照列（D5 焦点、D3 渲染 SceneGraph 图元、D10 组件规范、greenfield §B.2 若有 StrokeRect）。

完成后：①StrokeRect 图元是否实现（SceneGraph + vello 绘制）②获焦描边边框高亮（组件 + demo）③流式编码④cargo check/test 结果⑤commit hash + 受影响文档清单。全程中文，TDD 先 RED 后 GREEN。

> D15 已提交：36badd8(dev scale_factor) / 0cb8590(doc greenfield §B.3 补 scale_factor)。63 测试全绿(60→63)。流式判据 PASS。D15 闭合 D12 scale_factor P2。D15 P2（→D17）：渲染 surface 尺寸用 inner_size(物理) vs SceneGraph 逻辑坐标，Retina 下内容尺寸不匹配，D17 布局/渲染统一；to_logical 用全局 platform_scale 而非逐窗口 window_scale（多窗口/多显示器 scale 可能不同），多窗口时逐窗口换算，记后续改进。

---

> 阶段：**D20 完成 ✅（模态层级 + InputEvent/Ime 真实驱动，四层确认 + 收尾清理）。D3→D20 全部阶段完成，rgui 主体开发交付。用户授权"完成全部开发"范围内已全部完成。**

> D20 已提交：c32d4ec(dev 模态+InputEvent/ImeEvent) / eb1792f(doc greenfield §B.3) / 688343f(dev 清理 ime.rs 重复 ImeEvent)。81 测试全绿(74→81)。流式判据 PASS。D20 落地 D5 未实现项（模态焦点隔离 + 输入/IME 真实链路）。D20 P2：文本编辑组件接入(IME/InputEvent)待 P1（D5 如实标注）、open_modal 单层（多级模态留后续）、to_input_event CursorMoved 存物理坐标（供上层按需换算）。

---

## D20 — 模态层级 + InputEvent/Ime 真实驱动

**阶段：D20 已启动（总监自主排期，剩余开发最后一块，用户授权完成全部开发）。**
设计基准：greenfield §B.3（platform 输入/焦点/IME）+ docs/D5（事件/焦点/未实现项）。

**目标**：把 D5 标注的"未实现项"落地——`InputEvent/ImeEvent` 真实驱动（从占位类型到真实事件处理）、模态层级（FocusManager modal layer，模态浮层焦点隔离）。让输入（键盘/IME）与模态交互真实可用。

**范围**：
1. **InputEvent 真实驱动**：`InputEvent`（占位类型）补充，平台层真实事件→InputEvent（键盘/鼠标/滚动等），事件处理走真实链路（非仅占位）。
2. **ImeEvent 真实驱动**：ImeEvent（Preedit/Commit 占位）补充，IME 输入真实处理（汉字输入/组合输入的 Preedit→Commit 事件流，cosmic-text/文本编辑接入）。
3. **模态层级**：FocusManager modal layer——模态浮层（modal）打开时焦点隔离在模态内（模态外组件不获焦点），关闭恢复；对话框/模态浮层焦点管理。
4. **demo 验证**：模态浮层 demo（打开模态→焦点隔离在模态内→关闭恢复）；文本编辑输入 demo（IME 输入显示，若可行）。

【硬约束】流式编码优先；Tier 1 WidgetSpec；core 零 GPU/平台（输入/IME 在 platform/事件层，core 收逻辑事件）；DAG 无环；单一 vello/winit。
【每阶段提交】完成后 git add + commit（代码+同步文档+tasks.md 状态）+ push。受影响文档清单照列（D5 事件/焦点/IME、D1 WidgetSpec、greenfield §B.3 若有）。

完成后：①InputEvent 是否真实驱动（占位→真实）②ImeEvent 是否真实驱动（Preedit/Commit）③模态层级（FocusManager modal，焦点隔离）④流式编码⑤cargo check/test 结果⑥commit hash + 受影响文档清单。全程中文，TDD 先 RED 后 GREEN。

> 说明：若 IME/输入事件因 macOS 辅助功能权限（AXIsProcessTrusted=false）无法真实验证实时按键，则如实标注"逻辑链路由单测保证，实时注入待授权"，不虚构。模态层级为重点，IME 可做基础链路。

---

## 📌 剩余开发路线（总监自主排期，用户授权"完成全部开发"）

> 用户明确：不用再问优先做哪个，由总监决定，完成全部开发。以下为总监按依赖排序的主线（不再逐阶段提问，仅遇需用户拍板的架构决策时才上报）。

| 阶段 | 内容 | 依据（来源标注） |
|---|---|---|
| **D15** | scale_factor/DPI 换算（hit-test/焦点/坐标在高分屏/多显示器正确） | reviewer D11 多次点名 P2、D5 未实现项、D14 P2 |
| **D16** | StrokeRect 描边边框（真焦点边框样式） | 用户在意的方向，D14 留后续、D5 未实现项 |
| **D17** | 布局/文本换行（多组件布局 + 文本溢出处理） | D10 P2-3（展开态文本截断）、D14 后布局 |
| **D18** | key-based reconcile + 动态增删组件（组件复用/容器能力） | D3/D11 后续项、reviewer 提过 |
| **D19** | D4 样式系统 + 样式驱动（颜色硬编码→样式） | D14 P2（高亮硬编码）、greenfield §G 阶段0不含样式= P1 |
| **D20** | 模态层级 + InputEvent/Ime 真实驱动 | D5 标注未实现、D13 P2 |

**边界**：跨平台验证（Linux/Windows）无测试机，标"待有条件再补"；实时键盘注入需 macOS 辅助功能权限（AXIsProcessTrusted=falsed），标"待授权"。非"可完成开发"，如实标注。

**自主执行原则**：每阶段走完整质量链（dev 交付 → 总监实测复核 → reviewer 审查(流式判据) → qa 验收(文档一致性) → doc 同步）→ 通过后自动进入下一阶段；定期向用户呈报阶段进度，不逐阶段索要决策。

---

## D14 — 获焦高亮增强视觉（边框/背景高亮）

---

## D14 — 获焦高亮增强视觉（边框/背景高亮）

**阶段：D14 已放行（用户确认：获焦高亮增强视觉——边框高亮而非 ▶ 文字前缀）。**
设计基准：greenfield §B.1（组件视图）+ docs/D5（焦点）+ docs/D10（组件规范）。

**目标**：把焦点指示从 D13 的文字前缀 `▶` 升级为真正的视觉高亮——获焦组件绘制**焦点边框高亮/背景变化**（而非仅文字前缀），更接近真实 GUI 的焦点样式。

**范围**：
1. **获焦绘制样式**：Accordion/WaBadge 获焦时绘制**边框高亮**（如彩色边框/背景填充），未获焦无。这需要组件 view 产出**矩形边框 draw 指令**（FillRect/StrokeRect）而非仅文字前缀。
2. **绘制能力**：确认 render/view 是否有绘制**边框/描边矩形**的能力（若仅 FillRect，可能需加描边指令或加粗边框——先查 scene_graph/vello 现有 draw 图元）。
3. **demo 验证**：Tab/Shift+Tab 切换焦点时，获焦组件边框高亮变化（截图；离屏/单测验证高亮 draw 指令），比 D13 的 ▶ 前缀更直观。
4. 保持 D13 的焦点透传机制（ViewContext.focused），仅升级绘制表现（边框/背景）。

**硬约束**：流式编码优先；Tier 1 WidgetSpec；core 零 GPU/平台（获焦高亮是组件 view 的 draw 指令，render 渲染）；DAG 无环；单一 vello/winit。
【每阶段提交】完成后 git add + commit（代码+同步文档+tasks.md 状态）+ push。受影响文档清单照列（D5 焦点视觉、D1 ViewContext、D10 组件规范、greenfield 若有）。

完成后：①边框/背景高亮是否实现（获焦绘制样式）②焦点高亮变化验证（截图/测试，优于 ▶ 前缀）③流式编码④cargo check/test 结果⑤commit hash + 受影响文档清单。全程中文，TDD 先 RED 后 GREEN。

> D13 已提交：8e59f85(dev ViewContext.focused+获焦高亮▶+demo) / 292770f(doc greenfield+D1补ViewContext.focused,UpdateContext占位对齐)。60 测试全绿(58→60)。流式判据 PASS（ViewContext.focused 纯值传递，无 dyn Iterator/冗余 collect）。获焦高亮未泄漏进渲染层（reviewer 确认）。**

> D13 P2（D14 可参考）：获焦高亮增强视觉(边框而非▶前缀)、模态层级、scale_factor DPI、实时键盘注入需辅助功能权限。

---

## D13 — 焦点视觉显示（获焦组件高亮）

**阶段：D13 已放行（用户确认：焦点视觉显示——获焦组件高亮，视图层透传）。**
设计基准：greenfield §B.1（WidgetSpec view/paint）+ docs/D5（事件/焦点）。

**目标**：让用户看到当前焦点在哪个组件——获焦组件在视图层高亮（Accordion/WaBadge 获焦时绘制焦点指示，如边框高亮/背景变化），视图层透传焦点状态。

**范围**：
1. **视图层焦点透传**：`WidgetView`/`paint` 能感知当前组件是否获焦（focus 状态传入 view/paint，或 Widget 经某种方式读取 focus）。
2. **获焦高亮绘制**：Accordion/WaBadge 获焦时绘制焦点指示（如高亮边框/背景），未获焦一般。
3. **demo 验证**：Tab/Shift+Tab 切换焦点时，获焦组件视觉高亮变化（截图确认）。
4. 保持不动 core 契约（若需 WidgetSpec 加 focus 上下文入口，遵循向后兼容——或经 paint/context 传 focus）。

**硬约束**：流式编码优先；Tier 1 WidgetSpec；core 零 GPU/平台（焦点视觉在组件 view/paint，绘制由 render）；DAG 无环；单一 vello/winit。
【每阶段提交】完成后 git add + commit（代码+同步文档+tasks.md 状态）+ push。受影响文档清单照列（D5/D1 组件 view + greenfield 若有焦点视觉）。

完成后：①获焦高亮是否实现（视图层透传 + 绘制）②Tab 切换时焦点高亮变化（截图/测试）③流式编码④cargo check/test 结果⑤commit hash + 受影响文档清单。全程中文，TDD 先 RED 后 GREEN。

> D12 已提交：705a83d(dev FocusManager+focusable+demo Tab) / 4f61906(doc greenfield+D0补focusable) / 39a8bde(dev demo补Shift+Tab)。58 测试全绿(52→58)。流式编码判据 PASS（move_focus iter().position()+rem_euclid，无 dyn Iterator/冗余 collect）。**

> D12 P2（D13 待办）：focused 残留边界（focusable 列表变化时 focused 不置 None）、scale_factor DPI 换算、模态层级、Tab 实时注入需辅助功能权限。

> D11 已提交：f252e16(dev hit-test+map_message+WaBadge点击+多组件demo) / 37f01bc(doc greenfield+D0补hit-test)。52 测试全绿(45→52)。流式编码判据 PASS（hit_test iter().find()/map_message into_iter().map().collect()，无 dyn Iterator/冗余 collect）。文档同步铁律+流式编码铁律+每阶段提交 首次完整运转验证通过。

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

## 🔒 铁律：Rust 流式编码 + 每阶段提交（强制）

**Rust 编码优先使用流式（iterator 组合子）写法。** 依据 Rust 零成本抽象——流式代码经编译器（LLVM 内联+优化）生成的机器码等价于手写循环，是编译器优化过的最优形式；手写代码不必然最优。

> 用户明确：代码尽可能流式编写，编译器处理的一定是优化过的。检查也加入此要求。所有写 Rust 的代码相关角色（dev/reviewer）执行。**每阶段完成须直接提交代码 + 同步后的文档。**

### 流式编码规则
1. **优先用 iterator 链**（`iter().map().filter().fold()` 等组合子），替代显式 `for` 循环 + 中间变量 + `push`/`collect` 手写。
2. **零成本边界**（避免踩坑，防教条）：
   - 迭代器**保持具体类型**（`impl Iterator` / 泛型 `I: IntoIterator`），**不要装箱成 `dyn Iterator`**（装箱引入动态分发/堆分配，破坏零成本）。
   - **避免不必要的中间 `collect`**（每 collect 一次 = 一次堆分配 + 一次中间 Vec；可用 `.collect::<Vec<_>>()` 仅当确需物化，或用 `Vec::from_iter`/`extend` 直接)。
   - 纯链式能表达的，不要中途 break/手动 push 混写；但**确实复杂到流式伤可读时**（如复杂嵌套归并、带 break 的搜索），手写循环可读性更高，不强扭。
3. **只读遍历用 `iter()`，可变用 `iter_mut()`，按值用 `into_iter()`**；优先 `find`/`any`/`all`/`position` 而非手写循环 break。
4. **reviewer 检查新增「流式编码」判据**：生产代码出现"能用组合子却用显式循环 + push"的，或 `dyn Iterator` 装箱、冗余 `collect` 的，记为 P2 建议（不设 P0/P1，除非明显性能/可读倒退）。

### 每阶段提交（与文档同步铁律配合）
- **每个开发阶段完成 → 直接 `git add + commit` 该阶段代码 + 同步后的文档 + 更新 tasks.md 状态**，并 push。不积压，不"写完再说"。
- 提交信息：`type(rgui): <阶段> <描述>`（conventional commits），含"受影响文档已同步"说明。

### 检查点（验收必须包含）
- [ ] 新 Rust 代码优先流式（iterator 组合子），无装箱 `dyn Iterator`、无冗余中间 `collect`
- [ ] 该阶段**代码 + 同步文档 + tasks.md 状态**已 git commit + push
- [ ] reviewer 已按流式编码判据检查

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




