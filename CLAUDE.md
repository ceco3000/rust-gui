# RUST-GUI — ECC 开发指引

> 本文档是 ECC 开发本项目的入口说明书。编写或修改任何代码前，必须按本文档查阅对应设计文档。

---

## 项目概述

一个面向桌面优先、可扩展到移动端与 Web 的 Rust GUI 框架（`rgui`）。

- **语言最低版本**: Rust 1.85+（stable）
- **架构**: 多 crate workspace，以 `rgui-core` 为核心，`rgui` facade crate 为统一入口
- **设计权威来源**: `docs/` 中的设计文档体系

### 当前状态

**开发阶段**——阶段 0 核心基础设施已完成。当前仅支持拉丁文字（Inter 嵌入字体），CJK 为 P2 跳过。

**工作目录：** `/Users/chenchao/Documents/code/rust/RUST-GUI`

---

## 常用命令

项目使用 Cargo workspace，以下命令应在 workspace 根目录执行：

| 命令 | 用途 |
|------|------|
| `cargo build` | 构建所有 crate |
| `cargo test` | 运行所有测试 |
| `cargo test -p <crate>` | 运行指定 crate 的测试 |
| `cargo fmt -- --check` | 检查代码格式（CI 通过条件） |
| `cargo fmt` | 自动格式化代码 |
| `cargo clippy -- -D warnings` | lint 检查（CI 通过条件） |
| `cargo llvm-cov` | 测试覆盖率报告（目标 80%+） |
| `cargo doc --open` | 生成并打开 API 文档 |

---

## 设计文档体系（开发前必须先查阅）

`docs/` 中的设计文档是**开发的唯一权威来源**。所有数据结构、trait 签名、模块边界和跨子系统契约均以这些文档为准，不允许偏离。

### 文档分层

```
docs/Rust GUI 框架技术路线书.md                 ← 技术方向与决策依据
  │
  └─ D0-Rust GUI 框架总体设计.md                 ← 总约束边界（入口文档）
       │
       ├─ D1-组件模型与WidgetSpec设计.md          ← 子系统详细设计（可并行阅读）
       ├─ D2-状态管理与差分更新设计.md
       ├─ D3-渲染管线与场景图设计.md
       ├─ D4-样式系统与rgss设计.md
       ├─ D5-事件系统与输入处理设计.md
       ├─ D6-无障碍系统设计.md
       ├─ D7-开发反馈系统设计.md
       │
       └─ D8-阶段0开发任务分解.md                 ← 工程实施文档
            └─ D9-测试策略与基础设施设计.md
                 └─ D10-组件开发规范与示例.md
       D11-Cargo项目结构与发布策略.md             ← 工程实施文档（独立于 D8-D10 链）
```

### 查阅规则（MANDATORY）

| 当 ECC 需要 | 必须先读 |
|------------|---------|
| 写任何代码（首次或新 session） | `docs/D0-Rust GUI 框架总体设计.md` |
| 定义/修改 trait 或公共 API | `docs/D0-Rust GUI 框架总体设计.md`（§2 Crate 结构、§3 核心 Trait 体系、§5 关键数据结构、§7 跨子系统不变式） |
| 实现组件（Button、TextField、DataGrid 等） | `docs/D1-组件模型与WidgetSpec设计.md` + `docs/D10-组件开发规范与示例.md` |
| 实现状态管理、diff、快照 | `docs/D2-状态管理与差分更新设计.md` |
| 实现渲染相关代码 | `docs/D3-渲染管线与场景图设计.md` |
| 实现样式解析、rgss、主题 | `docs/D4-样式系统与rgss设计.md` |
| 实现事件路由、焦点、键盘、IME | `docs/D5-事件系统与输入处理设计.md` |
| 实现无障碍功能 | `docs/D6-无障碍系统设计.md` |
| 实现热重载、devtools、双进程 | `docs/D7-开发反馈系统设计.md` |
| 配置 Cargo workspace、feature flags | `docs/D11-Cargo项目结构与发布策略.md` |
| 编写测试、配置 CI | `docs/D9-测试策略与基础设施设计.md` |
| 任务拆分、工作量估算 | `docs/D8-阶段0开发任务分解.md` |
| 理解技术决策背景 | `docs/Rust GUI 框架技术路线书.md` |

### 例外：不需要查阅文档的场景

以下操作不需要预先查阅设计文档，可直接执行：

- 修改注释、文档字符串、README
- 修改变量名、函数名（不涉及公共 API 签名变更）
- 修复 clippy warning、rustfmt 格式问题
- 添加/修改单元测试（不涉及新增公共 API）
- 修复编译错误（不改变原有接口语义）

### 关键约束

1. **D0 是约束边界**：D0 中定义的 crate 拆分、trait 签名、模块职责和跨子系统不变式对所有子系统具有约束力。代码与 D0 冲突时，以 D0 为准（除非 D0 被明确更新）。
2. **接口一致性**：D1-D7 之间的接口签名必须一致。修改某个子系统的公共 API 时，必须检查其他文档的相关引用是否受影响。
3. **前置阅读字段**：每份设计文档开头都有 `前置阅读` 字段，指示文档间依赖关系。开发前应确认已理解前置文档。
4. **`[V{N}]` 标注**：设计文档中标注 `[V{N}]` 的元素直接引用技术路线验证项的已验证模式，可直接使用。

---

## Crate 命名与结构

所有 crate 以 `rgui-` 为前缀。`rgui` 为顶层 facade crate。

```
rgui (facade) ─ 重新导出全部公共 API
  ├─ rgui-core     ← 核心 trait 与类型（零平台依赖）
  ├─ rgui-state    ← 状态管理、diff、快照
  ├─ rgui-render   ← 渲染引擎（GPU 相关）
  ├─ rgui-layout   ← 布局引擎（Taffy 封装）
  ├─ rgui-style    ← 样式系统、.rgss 解析
  ├─ rgui-platform ← 窗口、输入、IME、剪贴板
  ├─ rgui-a11y     ← 无障碍系统
  ├─ rgui-devtools ← 热重载、双进程通信
  ├─ rgui-macros   ← 过程宏（html!、derive）
  ├─ rgui-components ← 内置组件库
  └─ rgui-script   ← Rhai 脚本绑定（已实现）
```

依赖方向：所有 crate 依赖 `rgui-core`，严禁循环依赖。

---

## 核心 Trait（权威定义见 D0 §3）

以下为概览，**完整定义和签名的唯一权威来源是 `docs/D0-Rust GUI 框架总体设计.md` §3**——写代码涉及这些 trait 时必须 Read D0 原文确认签名，不要凭记忆：

- `WidgetSpec` — 组件规范（`init`、`update`、`paint`、`layout_info`、`accessibility`）
- `AppMessage` — 消息类型（`'static + Send + Sync + Debug + Clone + PartialEq`）
- `PersistState` — 可序列化状态
- `RenderBackend` — 渲染后端抽象
- `AccessibilityBackend` — 无障碍后端抽象

---

## 开发流程

0. **查阅设计文档**（按上方查阅规则）
1. **TDD** — 先写测试（RED），再实现（GREEN），最后重构（IMPROVE）
2. **代码审查** — 每次代码变更后用 `rust-reviewer` agent 审查
3. **覆盖率** — 目标 80%+，使用 `cargo-llvm-cov`
4. **提交** — 遵循 conventional commits 格式（`feat:`、`fix:`、`refactor:` 等）

---

## ECC 规则体系

本项目受两层 ECC 规则约束（均由 ECC 自动加载，无需手动执行）：

| 层级 | 路径 | 内容 | 适用范围 |
|------|------|------|---------|
| 通用 | `~/.claude/rules/ecc/common/` | 编码风格、测试 80%+、安全规范、Git 工作流、简体中文 | 所有语言 |
| Rust 专属 | `~/.claude/rules/ecc/rust/` | 函数式编程优先、unsafe 严格准入、error handling、ownership | `**/*.rs` 文件 |

**优先级**：Rust 专属规则覆盖通用规则中冲突的部分（例如 Rust 文件优先使用迭代器组合子而非通用规则中的「简洁优先」）。详细内容见各自规则文件，CLAUDE.md 不重复列举。

---

## 项目约定

### 开发哲学

- **先「有没有」再「好不好」**：最简单的实现优先，拒绝优化建议。能跑 > 优雅。
- **禁止凭空发明设计哲学**：源码溯源，声明≠实现，字段存在≠被使用。

### 框架设计

- **QT 对齐**：本框架仿照 QT 设计。全部框架逻辑（事件模型、信号/槽、布局系统、渲染管线）必须与 QT 一致。当设计决策存在歧义时，以 QT 的实现为准。
- **组件来源**：所有内置组件（Button、TextField、DataGrid、Slider 等）从项目同级的 `webawesome` 仓库翻译而来。`webawesome` 是 Web Components 的参考实现，本项目的 Rust 组件是其语义等价的翻译。
- **组件定义唯一入口**：用户编写页面**必须**使用 `.rgui` 文件定义布局 + `.rhai` 文件定义业务逻辑。**不支持**其他方式定义组件树（例如 Rust 代码直接构造 WidgetView）。框架层面仅支持此一种声明式路径。

### 文字渲染

- 当前仅支持拉丁文字（Inter 嵌入字体）。CJK 为 P2（D12 M15，Vello 0.9 `draw_glyphs()` 单字体限制）。
- 垂直居中：`baseline_y = bounds.y + (bounds.h + ascent) / 2`。Vello 坐标系 Y 轴向上。

### Git 工作流

- 工作流：`fetch → rebase → push`，禁止产生 merge commit
- 提交前：`cargo check` 零错误 + 清理全部 `println!`
- 修改配置文件前先说明目的并等待确认
- `auto-dev` 循环停止后必须 `git status --short` 检查未提交代码

### ECC 开发要求

- 所有新功能/修改通过 ECC 子代理：`delegate_task` + `ecc-rust-rules`/`ecc-hooks`/`ecc/tdd-workflow`
- 手动直接写文件仅限 CLAUDE.md 例外场景（注释/文档字符串/格式修复/编译错误/非新 API 测试）
