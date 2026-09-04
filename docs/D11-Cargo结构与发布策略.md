# rgui Cargo 项目结构与发布策略（D11）

> 版本：0.1.0
> 工作目录：`/Users/chenchao/Documents/code/rust/RUST-GUI`
> 写实原则：本文 workspace 配置 / crate 清单 / feature 划分 / 命令与当前 Cargo.toml 实际一致；命令均经实际执行验证（标注 ✅ / ⚠️）。

---

## 1. Workspace 配置（根 `Cargo.toml`）

```toml
[workspace]
resolver = "2"
members = [
    "rgui-core", "rgui-render", "rgui-platform", "rgui-macros", "rgui",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.85"
authors = ["The rgui Project Developers"]
license = "MIT OR Apache-2.0"

[workspace.lints.rust]
unsafe_code = "deny"

[workspace.lints.clippy]
# 保持 clippy 默认级别；不启用 pedantic/nursery/all 全量。
```

要点：

1. **resolver = "2"**（edition 2021 起默认，显式声明）。
2. **members 恰好 5 个 crate**，与目录结构一致。
3. **workspace 共享元数据**：`version`/`edition`/`rust-version`/`authors`/`license` 统一由各 crate `*.workspace = true` 继承。
4. **lint 克制**：仅 `unsafe_code = "deny"`（Rust 内存安全核心）；`[lints.clippy]` 保持默认，不开启 pedantic/nursery/all 全量（硬约束 D，避免全 deny 拖慢开发）。
5. **重型外部依赖**（wgpu/vello/cosmic-text/fontdb/skrifa/winit 等）按 feature 划分在 D4+ 实现对应后端时引入；D3 阶段 0 骨架不引入，确保可离线快速编译。

---

## 2. CRATE 清单与内部依赖（5 个）

| crate | 类型 | 内部依赖 | 重型外部依赖（feature 门控） |
|-------|------|----------|-------------------------------|
| `rgui-core` | lib | （无） | `taffy`（可选，`layout` feature） |
| `rgui-render` | lib | `rgui-core` | `vello` / `wgpu` / `cosmic-text` / `fontdb` / `skrifa` / `pollster`（可选） |
| `rgui-platform` | lib | `rgui-core` | `winit` / `raw-window-handle`（`winit` 默认启用） |
| `rgui-macros` | proc-macro | （无） | （无） |
| `rgui` | lib | `rgui-core` / `rgui-render` / `rgui-platform` / `rgui-macros` | （经 feature 传递） |

依赖防火墙（硬约束 A）：`rgui-core` 不得依赖 render/platform/macros/winit/wgpu/vello/cosmic-text。实证：core 仅声明可选 `taffy`。

---

## 3. Feature 划分（与各 crate `Cargo.toml` 一致）

| crate | 默认 feature | 其余 feature | 说明 |
|-------|-------------|--------------|------|
| `rgui-core` | `["layout"]` | `layout = ["dep:taffy"]` | 布局引擎（Taffy 纯 Rust）；默认开启以便离线编译核心数据结构 |
| `rgui-render` | `[]` | `vello-backend = ["dep:vello","dep:wgpu","dep:cosmic-text","dep:fontdb","dep:skrifa","dep:pollster"]` | 单一 vello 渲染路径；无 skia/offscreen 变体 |
| `rgui-platform` | `["winit"]` | `winit = ["dep:winit","dep:raw-window-handle", "rgui-core/layout"]` | winit 默认启用（platform 核心依赖，非可选）；启用时同时开启 core 的 layout |
| `rgui-macros` | `[]` | （无） | 无大型运行依赖 |
| `rgui` | `[]` | `window = ["rgui-render/vello-backend", "rgui-platform/winit"]` | 门面入口；`test-harness` 当前注释未开 |

**Feature 传递关系**：
- 启用 `rgui` 的 `window` → 自动启用 `rgui-render/vello-backend` + `rgui-platform/winit`。
- 启用 `rgui-platform` 的 `winit` → 自动启用 `rgui-core/layout`。
- `rgui` 为 facade，源码**不直接引用** `winit::` / `wgpu::`（经 platform/render 公共 API）。

---

## 4. 编译 / 验证命令

> 以下命令均在本机（macOS，cargo）实际执行。

| 命令 | 结果 | 备注 |
|------|------|------|
| `cargo check -p rgui-core` | ✅ 通过 | 2 个 dead_code warning（StateStore 派生 trait 被忽略） |
| `cargo test -p rgui-core` | ✅ 通过 | 单元测试 + `tests/layout_engine.rs` + `tests/d10_components.rs`（Accordion 展开/折叠）通过 |
| `cargo check -p rgui-platform --features winit` | ✅ 通过 | 需 `winit` feature |
| `cargo check --workspace` | ✅ 通过 | platform `default=["winit"]` 默认启用 winit，默认构建通过 |
| `cargo check --workspace --features window` | ✅ 通过 | 启用 vello-backend + winit + core/layout；2 个 dead_code warning（`WidgetRegistry.inner`、`StateStore.state`） |

**注意事项**：
1. platform 的 `winit` 已为**默认启用**（`default=["winit"]`），默认 `cargo check --workspace` 通过；如需含窗口+渲染功能用 `--features window`（传递 vello-backend + winit）。
2. 核心逻辑（`rgui-core`）可独立 `cargo check` / `cargo test`，不触发 render/platform 重编译（硬约束 E：改数据/状态层不重编 render，见 greenfield §E.3）。
3. 重型 GPU 依赖（wgpu/vello/cosmic-text）仅在启用 `vello-backend` 时才编译，默认不引入。

---

## 5. 发布策略（0.x semver）

1. **版本**：当前 `version = "0.1.0"`，CRATE 共享 workspace 版本。
2. **语义**：遵守 0.x semver——0.x 期间 minor（如 0.2.0）可含 breaking change，patch（0.1.x）仅向后兼容修复。
3. **发布节奏**：每通过 reviewer/qa 验收 + 文档同步检查的阶段里程碑，统一 bump 并进行 `git tag`。
4. **license**：`MIT OR Apache-2.0`。
5. **MSRV**：`rust-version = "1.85"`（实际；greenfield 曾探讨 1.75/1.80，最终以 render 依赖上限为准，当前为 1.85）。
6. **crate 名规范**：`rgui-*` 前缀；`rgui` 为 facade。
7. **文档随发布同步**：发布前必须完成 D 系列文档 + `CLAUDE.md` + greenfield 一致性核对，不一致不发布。

---

## 6. 工程约定（写入 CLAUDE.md 的约定）

1. **lint 门禁**：`cargo test` + `cargo fmt --check`；clippy 仅对核心安全类 lint `-D`，unwrap/todo/expect 放宽为 `warn` 或 `-A` 豁免。
2. **代码风格**：生产库代码用 `thiserror`/结构化错误，避免裸 `unwrap`。
3. **目录约定**：`tools/` 存架构与风险审查权威文档；`docs/` 存 D 系列权威设计文档；`.hermes/` 为本地技能。
