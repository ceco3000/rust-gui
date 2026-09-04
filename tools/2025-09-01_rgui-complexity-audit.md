# rgui 复杂度审计与简洁设计方向

> 工作目录：`/Users/chenchao/Documents/code/rust/RUST-GUI`
> 生成：总监（devco-director）
> 编译基线：`cargo check --workspace` 通过（dev profile，56.49s）

---

## 一、现状画像

- **形态**：Cargo workspace，桌面优先的 Rust GUI 框架（`rgui`），QT 对齐。
- **规模**：框架 crate 12 个 + verify 验证期 crate（6 个在 workspace）+ examples 2 个 → 约 **5 万行 Rust**。
- **设计文档**：`docs/` 下 20+ 份文档，约 **1.5 万行 markdown**（D0–D14、技术路线书、验证设计、教程等）。
- **阶段**：阶段 0 核心基础设施已完成；仅支持拉丁文字（CJK 为 P2/P3 跳过）。

### 框架 crate 一览（源码行数 / lib.rs 是否纯转发壳）

| Crate | 行数 | lib.rs | 说明 |
|---|---|---|---|
| rgui-core | 5483 | 纯导出 | 核心类型/trait（零平台依赖） |
| rgui-render | 11116 | 有内容 | 渲染引擎（wgpu/vello/cosmic-text） |
| rgui-style | 6420 | 28 行 | .rgss 解析、主题、热重载 |
| rgui-devtools | 6519 | 有内容 | 热重载、双进程、状态快照 |
| rgui (facade) | 7709 | 纯重导出 | 顶层门面 **+ app.rs 5017 行** |
| rgui-state | 2680 | 16 行 | 状态管理/差分/快照 |
| rgui-platform | 3064 | 27 行 | 窗口/输入/IME |
| rgui-script | 2067 | 29 行 | Rhai 绑定 |
| rgui-macros | 1500 | 有内容 | 过程宏 |
| rgui-components | 852 | 10 行 | 内置组件（**当前近空壳**） |
| rgui-a11y | 619 | 16 行 | 无障碍 |
| rgui-layout | 609 | 9 行 | Taffy 封装 |
| rgui-template | 45 | — | 主入口模板 |

---

## 二、复杂度问题诊断

### 问题 1：crate 拆分过细，转发壳居多 —— 编译收益被接口维护成本抵消
- `rgui-layout`(609行)、`rgui-a11y`(619行)、`rgui-components`(852行)、`rgui-state`(2680行)、`rgui-platform`(3064行)、`rgui-script`(2067行) 的 `lib.rs` 基本是纯导出壳。
- 为"按 crate 增量编译"拆分 12 个 crate，但其中多个 crate 只封装几百行逻辑 → **拆分带来的增量收益，抵消不了 crate 间接口与模块边界维护成本**。拆分的粒度与"变更频率"不匹配。

### 问题 2：依赖方向已破坏 D0 不变式（设计意图 vs 实际实现脱节）
- **D0 §2.1 声称**："`rgui-core` 零依赖、`rgui-state` 纯 Rust 无平台依赖、严禁横向依赖"。
- **实际**：
  - `rgui-state` **依赖 `rgui-render` + `rgui-layout`**（`store.rs` 引用了 `GlyphKey`、`PathTessellation`、`LayoutResult`）→ 状态管理不应知道 GPU 字形缓存，**这是明显的架构污染**。
  - `rgui-platform` 依赖 `rgui-style`；`rgui-devtools` 依赖 `rgui-state`+`rgui-style`+`rgui-script`。
  - 所谓"无横向依赖"的干净分层并未实现。

### 问题 3：facade 门面职责过载（God Object）
- `rgui/src/app.rs` **5017 行、200 个顶层项**，混杂职责：
  - App 启动器 + winit 事件循环 + wgpu 渲染
  - 交互命中测试（`InteractionRegion`、`hit_test_*`）
  - 状态注入 / 属性注入 / 递归同步（`inject_state_bindings_*`、`sync_store_to_props_*`、`resolve_single_mode_conflicts_*`）
  - 测试自动化桩 `InteractionAutomationHarness`（约 40 个 `inject_*`/`replay_*` 方法）
  - 拖放处理
- 一个"门面"应只做重新导出与启动协调，**app.rs 却成了业务逻辑与测试设施的大杂烩**。

### 问题 4：多套"平行机制"并存（over-engineering）
- **渲染后端**：vello + skia 双后端抽象；但 skia 与 vello 版本冲突（0.78 vs 0.82）已被临时移除（`verify/av2-skia` 注释掉）→ 多后端抽象当前实际只有 vello 可用。
- **组件定义**：Tier 1 `WidgetSpec`（Rust 实现）+ Tier 2 `.rgui`/`.rhai`（声明式）两套并存。
- **组件实现处**：`Accordion` 已被迁出 `rgui-components`，实现在 `rgui` crate 内 → `rgui-components` 近空壳。
- **宏**：`html!` + `#[derive(WidgetSpec/AppMessage/PersistState)]`。
- **脚本**：Rhai 引擎 + paint 脚本两套执行路径。
- **热重载**：devtools watcher + style hot_reload 两套。

### 问题 5：设计文档与代码脱节，文档驱动负担重
- 1.5 万行设计文档，但已与代码多处不符（如 D0 说 `rgui-state` 纯 Rust，实则依赖 `rgui-render`）。
- "唯一权威来源"的强约束，在文档漂移后反而造成**认知负担与误导**。

### 问题 6：验证期遗留与仓库污染
- `verify/` 下 12 个验证 crate（v1–v10、av2-skia 等），仅 6 个在 workspace members，历史验证代码与正式框架混居。
- 仓库根存在**未跟踪污染**：`academic-review-report.md`、`expert_param_analysis.md`、`p0_verification_conservative_cryptographer.md`、`安全证明审查报告.md`、以及模板残留目录 **`{{pkgetc}}/`**（明显是模板占位符，不应在仓库根）。

---

## 三、简洁设计方向（供决策，建议与用户确认后由 architect 细化）

**核心原则：保留"按 crate 增量编译"这一动机（用户明确认可），但砍掉所有"不为当前目标服务"的复杂度；不新增 crate，优先合并转发壳。**

### 方向 1 —— 合并转发壳 crate，收敛到"高变更频率核心 + 重型依赖隔离"
建议把 12 个 crate 收敛为（示例，非定稿）：

| 合并后 | 纳入原 crate | 理由 |
|---|---|---|
| `rgui-core` | state、layout、a11y、macros、components 中与平台无关的纯逻辑 | 这些大多是几百行的转发壳；一个"核心"目录增量编译更简单 |
| `rgui-render` | 保留 | 重型 GPU 依赖（wgpu/vello），**必须单独隔离**，保增量编译收益 |
| `rgui-style` | 并入 core 或独立 | .rgss 解析依赖 cssparser 较重 |
| `rgui-platform` | 保留 | winit 平台依赖，**必须单独隔离** |
| `rgui` (facade) | 瘦身为纯重导出 + 启动器 | 砍掉 app.rs 的业务逻辑，独立成 `rgui-app`（若 UI 循环代码量大，可另拆） |
| `rgui-devtools`、`rgui-script` | **砍/降级为可选 feature** | 热重载/脚本是开发期增强，非框架核心，默认不用 |

### 方向 2 —— 打破 `rgui-state → rgui-render` 依赖
- 把 `GlyphKey`/`PathTessellation` 等渲染类型从状态层剥离，状态层只保留纯快照/差分逻辑。
- 解耦后可实现"改状态逻辑不重编渲染引擎"。

### 方向 3 —— 砍掉并行机制
- 渲染后端：先单一 vello，删 skia 双后端抽象（待真正需要时再引入）。
- 组件定义：只保留一条路径（Tier 1 WidgetSpec 或 Tier 2 声明式，二选一）。
- hot-reload / Rhai 脚本：统一走 `devtools` feature，默认关闭。

### 方向 4 —— 清理仓库
- 移出 `verify/` 验证期代码（或移入 `verify/` 独立 workspace / 独立仓库）。
- 删除模板残留 `{{pkgetc}}/`、未跟踪的分析报告（或移入 `tools/`）。
- 设计文档：只保留与当前实现一致的核心 D 系列 + 技术路线书，删 D14 工程规范类与过时文档或标注"历史已失效"。

---

## 四、已拍板的决策（用户确认 2025-09-01）

1. **crate 收敛粒度**：**适度收敛到 6-8 层**。保留 render/platform/GPU 重型依赖隔离，合并最细的转发壳（推荐项被采纳）。
2. **丢弃/降级子系统**：**全部丢弃**
   - `rgui-script`（Rhai 脚本）→ 删除
   - `rgui-devtools`（热重载/双进程）→ 删除
   - `rgui-a11y`（无障碍）→ 删除
   - skia 双后端抽象 → 删除，只保留 vello
3. **组件定义路径**：统一 **Tier 1 WidgetSpec（Rust 原生实现）**；Tier 2 声明式（.rgui/.rhai）废弃。

### 目标 crate 拓扑（6-8 层，供 architect 细化定稿）

| # | 目标 crate | 纳入/来源 | 依赖隔离 |
|---|---|---|---|
| 1 | `rgui-core` | 现 core + **state** + **layout** + **components** 纯逻辑 | 纯 Rust，零平台依赖 |
| 2 | `rgui-render` | 现 render | wgpu/vello/cosmic-text 重型 GPU 隔离 |
| 3 | `rgui-platform` | 现 platform | winit 重型平台隔离 |
| 4 | `rgui-style` | 现 style（含 .rgss 解析） | cssparser；若重依赖可并入 core |
| 5 | `rgui-macros` | 现 macros | proc-macro（**硬约束：必须独立 crate**） |
| 6 | `rgui` (facade) | 现 facade 瘦身 | 纯重导出 + 启动协调；app.rs 拆分 |

**硬性原则**：
- proc-macro crate 不可与普通 crate 合并（Rust 强约束）→ `rgui-macros` 保持独立。
- 打破 `rgui-state → rgui-render`：GPU 资源类型（GlyphKey/PathTessellation/字形缓存）移入 render 层，state 只保留纯状态/差分/快照逻辑。
- `rgui/src/app.rs`(5017行) 拆分：App 启动器 / 事件循环 / 渲染协调 / 交互命中 / 测试桩分离。
- 组件库回归 `rgui-core` 或独立，Tier 2 脚本路径移除。

## 五、原决策草案（已被上方决策取代，存档）
