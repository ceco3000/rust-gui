# D11：Cargo 项目结构与发布策略

> **文档定位：** 定义 Cargo workspace 配置、各 crate 的 feature flags 设计、crates.io 发布策略和版本管理规范。本文档是对 D0 §2（Crate 结构）的工程化补充。

> **前置阅读：** [Rust GUI 框架总体设计](./Rust%20GUI%20框架总体设计.md)（D0）§2（Crate 结构）、§2.5（MSRV）、§2.6（版本兼容性策略）、[D8 阶段 0 开发任务分解](./D8-阶段0开发任务分解.md)。

> **状态：** 初版——随实际发布过程迭代更新。

---

## 目录

1. [设计目标与范围](#1-设计目标与范围)
2. [Workspace 配置](#2-workspace-配置)
3. [各 Crate 的 Cargo.toml 骨架](#3-各-crate-的-cargotoml-骨架)
4. [Feature Flags 矩阵](#4-feature-flags-矩阵)
5. [依赖版本同步策略](#5-依赖版本同步策略)
6. [发布顺序与流程](#6-发布顺序与流程)
7. [crates.io 元数据规范](#7-cratesio-元数据规范)
8. [发布检查清单](#8-发布检查清单)
9. [CI/CD 发布流水线](#9-cicd-发布流水线)

---

## 1. 设计目标与范围

### 1.1 本文档解决什么问题

1. 定义 Cargo workspace 的完整配置（members、dependencies、lints）
2. 定义每个 crate 的 `Cargo.toml` 完整内容（依赖、feature flags、元数据）
3. 定义 crates.io 发布顺序、流程和检查清单
4. 定义 workspace 内依赖版本同步策略

### 1.2 与 D0 §2 的关系

D0 §2 定义了 crate 的**架构边界**（职责、依赖方向、公共 API）。本文档定义 crate 的**工程配置**（`Cargo.toml`、feature flags、发布流程）。两者互补：D0 回答"为什么这样拆分"，本文档回答"如何配置和发布这些 crate"。

---

## 2. Workspace 配置

### 2.1 目录布局

> **与 D0 §2 的关系**：D0 §2 定义了 9 个核心 crate。本文档额外列出 `rgui-devtools` 和 `rgui-components`（阶段 1 新增，分别对应开发反馈系统和内置组件库）。

```
rust-gui/
├── Cargo.toml              # workspace root
├── Cargo.lock              # 提交到 VCS
├── rust-toolchain.toml     # 固定工具链：stable
├── deny.toml               # cargo-deny 许可证审计
├── CHANGELOG.md
├── rgui-core/              # 核心类型与 trait（零平台依赖）
├── rgui-state/             # 状态管理
├── rgui-render/            # 渲染管线
├── rgui-layout/            # 布局引擎
├── rgui-style/             # 样式系统
├── rgui-platform/          # 平台抽象
├── rgui-a11y/              # 无障碍系统
├── rgui-devtools/          # 开发工具
├── rgui-macros/            # 过程宏
├── rgui/                   # facade crate
├── rgui-components/        # 内置组件库
└── examples/
    └── counter/
```

### 2.2 根 `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = [
    "rgui-core", "rgui-state", "rgui-render", "rgui-layout",
    "rgui-style", "rgui-platform", "rgui-a11y", "rgui-devtools",
    "rgui-macros", "rgui", "rgui-components",
    "examples/*",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.85"
authors = ["The rgui Project Developers"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/rgui-rs/rgui"
homepage = "https://rgui.rs"
documentation = "https://docs.rs/rgui"
readme = "README.md"
keywords = ["gui", "ui", "desktop", "widget", "cross-platform"]
categories = ["gui", "rendering", "development-tools"]

[workspace.dependencies]
# 内部 crate（path + version 双约束）
rgui-core = { path = "rgui-core", version = "0.1.0" }
rgui-state = { path = "rgui-state", version = "0.1.0" }
rgui-render = { path = "rgui-render", version = "0.1.0" }
rgui-layout = { path = "rgui-layout", version = "0.1.0" }
rgui-style = { path = "rgui-style", version = "0.1.0" }
rgui-platform = { path = "rgui-platform", version = "0.1.0" }
rgui-a11y = { path = "rgui-a11y", version = "0.1.0" }
rgui-devtools = { path = "rgui-devtools", version = "0.1.0" }
rgui-macros = { path = "rgui-macros", version = "0.1.0" }

# 外部依赖——渲染
wgpu = "24"
vello = "0.8"
cosmic-text = "0.12"

# 外部依赖——布局
taffy = "0.7"

# 外部依赖——平台
winit = "0.30"
accesskit = "0.17"
accesskit_winit = "0.23"

# 外部依赖——序列化与工具
serde = { version = "1", features = ["derive"] }
postcard = "1"
erased-serde = "0.4"
log = "0.4"
thiserror = "2"
notify = "7"
rustc-hash = "2"
ordered-float = "4"
serde_json = "1"

# 外部依赖——开发
criterion = "0.5"
image = "0.25"

[workspace.lints.rust]
unsafe_code = "deny"
missing_docs = "warn"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
cargo = "warn"
```

> **`Cargo.lock`**：提交到 VCS，确保所有开发者、CI 和发布流程使用一致的依赖版本。

### 2.3 `rust-toolchain.toml`

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "rust-docs"]
targets = ["wasm32-unknown-unknown"]
```

---

## 3. 各 Crate 的 Cargo.toml 骨架

### 3.1 `rgui-core`

```toml
[package]
name = "rgui-core"
description = "rgui 框架核心类型与 trait 定义——零平台依赖"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
keywords = ["gui", "widget", "core", "trait"]
categories = ["gui"]

[dependencies]
serde = { workspace = true, optional = true }
ordered-float.workspace = true
rustc-hash.workspace = true
thiserror.workspace = true

[features]
default = []
serde = ["dep:serde", "ordered-float/serde"]
```

### 3.2 `rgui-state`

```toml
[package]
name = "rgui-state"
description = "rgui 状态管理——StateStore、diff 算法、快照与迁移"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
keywords = ["gui", "state", "diff", "snapshot"]
categories = ["gui"]

[dependencies]
rgui-core.workspace = true
serde.workspace = true
postcard.workspace = true
rustc-hash.workspace = true
thiserror.workspace = true
erased-serde.workspace = true

[dev-dependencies]
criterion.workspace = true

[[bench]]
name = "diff_bench"
harness = false
```

### 3.3 `rgui-render`

```toml
[package]
name = "rgui-render"
description = "rgui 渲染管线——SceneGraph、Vello/Skia 后端、字形 Atlas"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
keywords = ["gui", "render", "gpu", "vello", "wgpu"]
categories = ["gui", "rendering"]

[dependencies]
rgui-core.workspace = true
rustc-hash.workspace = true
log.workspace = true
thiserror.workspace = true
wgpu = { workspace = true, optional = true }
vello = { workspace = true, optional = true }
cosmic-text = { workspace = true, optional = true }
skia-safe = { version = "0.82", optional = true }

[features]
default = ["vello-backend"]
vello-backend = ["dep:wgpu", "dep:vello", "dep:cosmic-text"]
skia-backend = ["dep:skia-safe"]
offscreen = ["dep:image"]
```

### 3.4 `rgui-layout`

```toml
[package]
name = "rgui-layout"
description = "rgui 布局引擎——Taffy 封装、CSS 属性映射、布局缓存"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
keywords = ["gui", "layout", "flexbox", "grid", "taffy"]
categories = ["gui"]

[dependencies]
rgui-core.workspace = true
taffy.workspace = true
rustc-hash.workspace = true
```

### 3.5 `rgui-style`

```toml
[package]
name = "rgui-style"
description = "rgui 样式系统——.rgss 解析器、选择器引擎、主题变量、热重载"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
keywords = ["gui", "css", "stylesheet", "theme"]
categories = ["gui"]

[dependencies]
rgui-core.workspace = true
serde.workspace = true
rustc-hash.workspace = true
thiserror.workspace = true
cssparser = "0.34"
notify = { workspace = true, optional = true }

[features]
default = []
hot-reload = ["dep:notify"]
```

### 3.6 `rgui-platform`

```toml
[package]
name = "rgui-platform"
description = "rgui 平台抽象——窗口管理、输入事件、IME、剪贴板"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
keywords = ["gui", "window", "input", "winit"]
categories = ["gui"]

[dependencies]
rgui-core.workspace = true
winit.workspace = true
rustc-hash.workspace = true
thiserror.workspace = true

[target.'cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))'.dependencies]
arboard = "3"
```

### 3.7 `rgui-a11y`

```toml
[package]
name = "rgui-a11y"
description = "rgui 无障碍系统——AccessKit 集成、无障碍树、焦点管理"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
keywords = ["gui", "accessibility", "a11y", "accesskit", "wcag"]
categories = ["gui", "accessibility"]

[dependencies]
rgui-core.workspace = true
accesskit.workspace = true
accesskit_winit.workspace = true
rustc-hash.workspace = true
thiserror.workspace = true
```

### 3.8 `rgui-devtools`

```toml
[package]
name = "rgui-devtools"
description = "rgui 开发工具——文件监控、热重载、快速重启、双进程通信"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
keywords = ["gui", "devtools", "hot-reload"]
categories = ["gui", "development-tools"]

[dependencies]
rgui-core.workspace = true
rgui-state.workspace = true     # 快照协议
rgui-style.workspace = true     # .rgss 监控
serde.workspace = true
serde_json.workspace = true
postcard.workspace = true
notify.workspace = true
log.workspace = true
thiserror.workspace = true
```

### 3.9 `rgui-macros`

```toml
[package]
name = "rgui-macros"
description = "rgui 宏——html! 宏、派生宏（WidgetSpec、AppMessage、PersistState）"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
keywords = ["gui", "macro", "derive", "dsl"]
categories = ["gui"]

[lib]
proc-macro = true

[dependencies]
syn = { version = "2", features = ["full", "extra-traits"] }
quote = "1"
proc-macro2 = "1"
```

### 3.10 `rgui`（Facade）

```toml
[package]
name = "rgui"
description = "rgui——Rust GUI 框架（facade crate，重新导出所有公共 API）"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
documentation.workspace = true
readme.workspace = true
keywords = ["gui", "ui", "desktop", "widget", "cross-platform"]
categories = ["gui", "rendering"]

[dependencies]
rgui-core.workspace = true
rgui-state.workspace = true
rgui-render.workspace = true
rgui-layout.workspace = true
rgui-style.workspace = true
rgui-platform.workspace = true
rgui-a11y.workspace = true
rgui-macros.workspace = true
rgui-devtools = { workspace = true, optional = true }

[features]
default = ["vello-backend"]
vello-backend = ["rgui-render/vello-backend"]
skia-backend = ["rgui-render/skia-backend"]
devtools = ["dep:rgui-devtools", "rgui-style/hot-reload"]
```

### 3.11 `rgui-components`

```toml
[package]
name = "rgui-components"
description = "rgui 内置组件库——Button、TextField、DataGrid、Form 等"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
keywords = ["gui", "components", "widgets"]
categories = ["gui"]

[dependencies]
rgui-core.workspace = true
ordered-float.workspace = true
serde.workspace = true
```

---

## 4. Feature Flags 矩阵

### 4.1 定义

| Crate | Feature | 默认 | 说明 |
|-------|---------|------|------|
| `rgui-core` | `serde` | 否 | 启用 PropValue 的 serde 序列化 |
| `rgui-render` | `vello-backend` | **是** | Vello + wgpu 渲染后端 |
| `rgui-render` | `skia-backend` | 否 | Skia CPU 光栅化 fallback |
| `rgui-render` | `offscreen` | 否 | 离屏渲染 + PNG 输出（测试用） |
| `rgui-style` | `hot-reload` | 否 | 文件监控 + 样式热重载 |
| `rgui` | `vello-backend` | **是** | 转发到 rgui-render |
| `rgui` | `skia-backend` | 否 | 转发到 rgui-render |
| `rgui` | `devtools` | 否 | 包含 rgui-devtools（开发模式） |

### 4.2 典型用户配置

```toml
# 生产应用
[dependencies]
rgui = "0.1"

# 开发模式（含热重载和 devtools）
[dev-dependencies]
rgui = { version = "0.1", features = ["devtools"] }

# 仅使用样式系统（无渲染/平台依赖）
[dependencies]
rgui-style = "0.1"
rgui-core = "0.1"

# 自定义渲染后端
[dependencies]
rgui-core = "0.1"
rgui-render = { version = "0.1", default-features = false }
```

---

## 5. 依赖版本同步策略

### 5.1 内部依赖

使用 path + version 双约束：

```toml
[dependencies]
rgui-core = { path = "../rgui-core", version = "=0.1.0" }
```

- `path`：本地开发使用源码
- `version = "=0.1.0"`：发布到 crates.io 时使用精确版本
- 发布脚本在 `cargo publish` 前确保 `version` 与当前版本一致

### 5.2 外部依赖

所有外部依赖版本集中在 workspace `[workspace.dependencies]`。各 crate 通过 `workspace = true` 引用，确保一致。升级流程：修改 workspace 版本 → `cargo update` → 全 workspace 测试。

### 5.3 版本号共享策略

所有 crate 共享同一个版本号（`version.workspace = true`）。少数 crate 变更 → 全部 crate bump 版本。好处：用户不需要记忆不同 crate 的版本兼容矩阵。

---

## 6. 发布顺序与流程

### 6.1 依赖拓扑决定的发布顺序

```
第 1 批（零依赖）：          rgui-core, rgui-macros
第 2 批（仅依赖 core）：      rgui-state, rgui-style, rgui-layout
第 3 批（依赖 core + 平台）： rgui-render, rgui-platform, rgui-a11y
第 4 批（横切依赖）：         rgui-devtools（依赖 state + style）
第 5 批（聚合）：             rgui-components, rgui（依赖所有）
```

### 6.2 发布脚本

```bash
#!/bin/bash
# scripts/publish.sh —— 按依赖顺序发布所有 crate
set -euo pipefail

PUBLISH_ORDER=(
    "rgui-core" "rgui-macros"
    "rgui-state" "rgui-style" "rgui-layout"
    "rgui-render" "rgui-platform" "rgui-a11y"
    "rgui-devtools"
    "rgui-components" "rgui"
)

echo "=== 检查发布条件 ==="
cargo fmt --check && cargo clippy --workspace -- -D warnings && cargo test --workspace

for crate in "${PUBLISH_ORDER[@]}"; do
    echo "=== 发布 $crate ==="
    cargo publish -p "$crate" --dry-run
    read -p "dry-run 通过，确认发布 $crate？(y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        cargo publish -p "$crate"
        echo "$crate 发布成功"
        sleep 10  # 等待 crates.io 索引更新
    else
        echo "跳过 $crate，发布中止"
        exit 1
    fi
done
echo "=== 全部发布完成 ==="
```

---

## 7. crates.io 元数据规范

每个 crate 的 description 和 keywords 必须唯一且有区分度：

| Crate | description | 特有 keywords |
|-------|-------------|-------------|
| `rgui-core` | 核心类型与 trait 定义——零平台依赖 | `core`, `trait` |
| `rgui-state` | 状态管理——StateStore、diff、快照 | `state`, `diff`, `snapshot` |
| `rgui-render` | 渲染管线——Vello/Skia 后端、字形 Atlas | `render`, `gpu`, `vello` |
| `rgui-layout` | 布局引擎——Taffy 封装 | `layout`, `flexbox`, `grid` |
| `rgui-style` | 样式系统——.rgss 解析器、主题 | `css`, `stylesheet`, `theme` |
| `rgui-platform` | 平台抽象——窗口、输入、IME | `window`, `input` |
| `rgui-a11y` | 无障碍——AccessKit 集成 | `accessibility`, `a11y`, `wcag` |
| `rgui-devtools` | 开发工具——热重载、快速重启 | `devtools`, `hot-reload` |
| `rgui-macros` | 过程宏——html!、派生宏 | `macro`, `derive`, `dsl` |
| `rgui-components` | 内置组件库 | `components`, `widgets` |
| `rgui` | Rust GUI 框架（facade） | `desktop`, `cross-platform` |

每个 crate 的 README 必须包含：一句话描述、架构位置、最小可用示例、feature flags 说明、指向 facade crate 的链接。

---

## 8. 发布检查清单

### 8.1 每次发布前

- [ ] `cargo fmt --check` 通过
- [ ] `cargo clippy --workspace -- -D warnings` 通过
- [ ] `cargo test --workspace` 通过
- [ ] `cargo test --workspace --all-features` 通过
- [ ] `cargo doc --no-deps --workspace` 无警告
- [ ] `cargo deny check` 通过（许可证审计）
- [ ] `CHANGELOG.md` 已更新
- [ ] 版本号在 workspace `Cargo.toml` 中已 bump
- [ ] `cargo publish --dry-run` 对每个 crate 通过

### 8.2 发布后

- [ ] Git tag 推送（如 `v0.1.0`）
- [ ] GitHub Release 创建（附 CHANGELOG）
- [ ] `docs.rs` 构建验证（等待 ~10 分钟）
- [ ] 用户群发布 release notes

---

## 9. CI/CD 发布流水线

```yaml
# .github/workflows/publish.yml
name: Publish

on:
  workflow_dispatch:
    inputs:
      dry_run:
        description: '仅 dry-run'
        type: boolean
        default: true

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --check
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo test --workspace --all-features
      - run: cargo doc --no-deps --workspace

  publish:
    needs: verify
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Dry run
        if: inputs.dry_run
        run: |
          for crate in rgui-core rgui-macros rgui-state rgui-style rgui-layout \
                       rgui-render rgui-platform rgui-a11y rgui-devtools \
                       rgui-components rgui; do
            cargo publish -p "$crate" --dry-run
          done
      - name: Publish
        if: ${{ !inputs.dry_run }}
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
        run: |
          for crate in rgui-core rgui-macros rgui-state rgui-style rgui-layout \
                       rgui-render rgui-platform rgui-a11y rgui-devtools \
                       rgui-components rgui; do
            cargo publish -p "$crate"
            sleep 10
          done
```

---

> **下一步：** D1-D11 全部设计文档完成。进入实现阶段（见 [D8 阶段 0 开发任务分解](./D8-阶段0开发任务分解.md)）。
