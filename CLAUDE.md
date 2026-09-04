# rgui — Rust GUI 框架

## 项目概述

一个面向桌面优先、跨平台的 Rust GUI 框架（`rgui`）。**当前为全新 5-crate 绿色架构**（D3-D10 已实现，见 `tools/2025-09-01_rgui-greenfield-architecture.md` 与 `docs/D0-总体设计.md`）。

- **语言**: Rust（edition 2021，MSRV 以 render 依赖上限为准，见 `docs/D11`）
- **架构**: 5 个 crate 的 Cargo workspace，`rgui-core` 为唯一零 GPU/零平台逻辑核心
- **设计权威来源**: `docs/D0-总体设计.md`（D0 顶层）、`docs/D1-D10`（各子系统，按 greenfield 全集）、`tools/2025-09-01_rgui-greenfield-architecture.md`（架构唯一权威）

## Crate 列表（5 个）

| Crate | 职责 | 关键依赖 |
|---|---|---|
| `rgui-core` | 唯一逻辑核心（WidgetSpec/AppMessage/状态/布局/样式/组件/无障碍树） | 纯 Rust，零 GPU/零平台 |
| `rgui-render` | 渲染引擎（单一 vello 后端，cosmic-text 字形） | wgpu/vello/cosmic-text（经 `vello-backend` feature） |
| `rgui-platform` | 平台层（winit 窗口/事件循环/输入/IME/焦点） | winit（**默认启用**，`default=["winit"]`） |
| `rgui-macros` | 过程宏（derive + html!） | proc-macro（必须独立） |
| `rgui` | 薄 facade（重导出 + `App::run` 统一入口） | 依赖全部 4 个 |

依赖方向：`render`/`platform`/`macros` 只向下依赖 `core`；`rgui` facade 依赖全部；`render` 与 `platform` **互不相依**（DAG 无环）。

## 常用命令

| 命令 | 用途 |
|---|---|
| `cargo check --workspace` | 检查所有 crate（默认 feature） |
| `cargo check --workspace --features window` | 检查含 window 功能 |
| `cargo test --workspace --all-features` | 运行所有测试 |
| `cargo fmt -- --check` | 格式检查 |
| `cargo run -p rgui --features window --example window_demo` | 运行窗口示例（Accordion 组件，点击/Space 切换展开；加 `-- --expanded` 初始展开） |

## 核心 Trait（权威定义见 `docs/D0-总体设计.md`）

- `WidgetSpec` — 组件规范（view/update/measure/paint/accessibility）
- `AppMessage` — 消息类型
- `PersistState` — 持久状态
- `EventResult` — 事件传播结果（Handled/Prevented/Continue(M)）

统一 Tier 1 `WidgetSpec`（Rust 原生）；已废弃 .rgui/.rhai 声明式、Rhai 脚本、devtools、skia 多后端（"有意缺失"，为架构克制）。

## 开发约定

- **文档是开发根基**：每次任务完成必须同步文档（docs/D0-D11、CLAUDE.md、greenfield、tasks.md），且必须被检查（文档未同步 = 不放行）。详见 `tasks.md`「🔒 铁律：文档同步」。
- **TDD**：先写失败测试（RED）→ 实现（GREEN）→ 重构（REFACTOR）；严禁先写实现再补测试。
- **依赖防火墙**：`rgui-core/Cargo.toml` 的 `[dependencies]` 绝不含 rgui-render/platform/macros、winit/wgpu/vello/cosmic-text。
- **lint 克制**：`unsafe_code = deny`，clippy default，todo/expect/unwrap 放宽到 warn。
- **单一机制**：只保留单一 vello 渲染、单一 winit 平台、Tier 1 WidgetSpec 组件。
- **git**：`fetch → rebase → push`，禁止 merge commit；提交前无 panic/error，`cargo check` 零错。
