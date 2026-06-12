# Rust GUI 框架技术路线验证设计

> **文档目标：** 在进入阶段 0 开发之前，用最小代价验证《Rust GUI 框架技术路线书》中关键技术假设的可行性。
> 本文定义验证项、验证方法、通过标准和执行顺序，用于指导验证代码的编写。

> **前置阅读：** [Rust GUI 框架技术路线书](./Rust%20GUI%20框架技术路线书.md)

---

## 目录

1. [验证目标与范围](#1-验证目标与范围)
2. [前置发现：需要验证的技术组合问题](#2-前置发现需要验证的技术组合问题)
   - [2.1 主技术路线选择理由：与替代方案的逐项对比](#21-主技术路线选择理由与替代方案的逐项对比)
     - [2.1.1 Vello vs Skia](#211-vello-vs-skia渲染引擎)
     - [2.1.2 cosmic-text vs Parley + Fontique](#212-cosmic-text-vs-parley--fontique文本引擎)
     - [2.1.3 Taffy vs Yoga](#213-taffy-vs-yoga布局引擎)
     - [2.1.4 AccessKit vs 直接平台 API](#214-accesskit-vs-直接平台-api无障碍)
     - [2.1.5 主技术栈的协同优势](#215-主技术栈的协同优势)
     - [2.1.6 选择总结](#216-选择总结)
3. [验证项总览](#3-验证项总览)
4. [关键验证项详设](#4-关键验证项详设)
   - [V1：Vello + cosmic-text 协同渲染](#v1-vello--cosmic-text-协同渲染)
   - [V2：cosmic-text CJK 文本渲染质量](#v2-cosmic-text-cjk-文本渲染质量)
   - [V3：AccessKit 能力边界分析](#v3-accesskit-能力边界分析)
   - [V4：渲染管线跨平台三端可运行](#v4-渲染管线跨平台三端可运行)
   - [V5：WidgetView diff 性能基准](#v5-widgetview-diff-性能基准)
   - [V6：Taffy 布局 → 渲染坐标转换](#v6-taffy-布局--渲染坐标转换)
   - [V7：状态快照性能基准](#v7-状态快照性能基准)
   - [V9：DataGrid 虚拟滚动性能前提确认](#v9-datagrid-虚拟滚动性能前提确认)
   - [V10：cosmic-text IME 集成路径](#v10-cosmic-text-ime-集成路径)
5. [执行顺序与时间规划](#5-执行顺序与时间规划)
6. [验证代码结构建议](#6-验证代码结构建议)
7. [验证结论判定标准](#7-验证结论判定标准)

---

## 1. 验证目标与范围

### 1.1 验证目标

逐项验证《Rust GUI 框架技术路线书》中提出的技术选型能否兑现 7 项必达目标（H1-H7）。验证的核心问题是：

> **文档提出的技术组合是否具备实现文档要求的能力？**

### 1.2 验证范围

- **在范围内**：渲染管线集成、文本质量、无障碍能力边界、性能关键路径、跨平台基础
- **不在范围内**：完整框架实现、生产级组件开发、移动端适配、社区治理

### 1.3 验证原则

1. **最小代价**：优先用调研和分析验证，只在必须时才写 POC 代码
2. **先致命后次要**：可能推翻技术路线的假设优先验证
3. **可复现**：所有 POC 验证结果应可独立复现
4. **可量化**：通过标准用数字说话，不用主观判断

---

## 2. 前置发现：需要验证的技术组合问题

在编写本验证设计之前进行的调研中发现了一个关键事实：

> **Xilem（最接近本路线书架构的参照项目）的文本栈选择的是 Parley + Fontique，而非 cosmic-text。** 两者都来自 Linebender 组织，但 Xilem 并未采用 cosmic-text。在实际项目中，iced 和 COSMIC DE 使用了 cosmic-text，但它们未使用 Vello 作为渲染器。

这意味着「**Vello + cosmic-text 的组合没有在任何既有项目中得到验证**」。这一组合是本路线书最大的单一技术不确定性，必须作为第一优先级验证项。

其他关键依赖的状态概要：

| 依赖 | 当前版本（2026 年中） | 实际使用案例 | 风险 |
|------|----------------------|-------------|------|
| **wgpu** | 成熟 | Firefox、Servo、Deno、iced | 🟢 低 |
| **winit** | 成熟 | Rust GUI 生态标准窗口库 | 🟢 低 |
| **Vello** | v0.8.0（Linebender 维护） | Xilem、Bevy（bevy_vello） | 🟡 中 |
| **cosmic-text** | v0.17.x（System76 维护） | iced、COSMIC DE | 🟢 低（独立使用） |
| **Vello + cosmic-text 组合** | 无已知案例 | — | 🔴 高 |
| **Taffy** | v0.8.x | Dioxus、Bevy、Zed/GPUI、Servo、Slint | 🟢 低 |
| **AccessKit** | v0.24.0（2026-02） | egui、Slint、GTK 4.18、Bevy | 🟡 中 |
| **Rhai** | 成熟 | 多种嵌入式场景 | 🟢 低 |

### 2.1 主技术路线选择理由：与替代方案的逐项对比

本节解释为什么路线书选择当前技术组合作为主路线，而非直接选用对应的替代技术。核心决策原则有三条：

1. **全 Rust 编译链优先**：避免 C++ FFI 依赖，保证 `cargo build` 一条命令完成全量构建
2. **依赖去中心化优先**：避免多个核心组件依赖单一组织，降低 Azul/WebRender 式集中风险
3. **能力完整性优先**：选择能满足路线书全部布局和渲染需求的技术

---

#### 2.1.1 Vello vs Skia（渲染引擎）

| 维度 | Vello（主路线） | Skia（替代路线） |
|------|----------------|-----------------|
| **语言** | 纯 Rust | C++（通过 `skia-safe` FFI 绑定） |
| **渲染模型** | GPU compute shader（现代 GPU 并行计算） | 传统 2D API（Immediate mode，CPU 驱动 GPU） |
| **编译链复杂度** | 纯 Rust，`cargo build` 一条命令 | 需 C++ 工具链（CMake/clang），CI 配置更复杂 |
| **首次全量编译** | 5-15 分钟（全部依赖） | 10-40 分钟（取决于是否预编译 Skia） |
| **二进制体积** | 较小（Rust LTO + 按需链接） | 较大（链接 Skia 静态库） |
| **与 wgpu 的集成** | 深度集成，共享 wgpu device/queue | 需独立的 GPU 上下文或通过 CPU 后端 |
| **维护方** | Linebender（小团队，资金不明） | Google（大公司，Chrome/Flutter/Android 依赖，20+ 年） |
| **批处理能力** | compute shader 天然支持并行批处理 | immediate mode，批处理需自行组织 |
| **成熟度** | Beta（v0.8.0），仍在快速迭代 | 生产级，数十亿设备验证 |

**选择 Vello 的原因**：

- **compute shader 模型适合 GUI 场景**。GUI 的典型帧包含大量路径（圆角矩形、边框）、渐变、混合模式——这些在传统 2D API 中是逐个调用的，而在 compute shader 模型中可以并行处理。路线书 §3.3 原则 6 要求「渲染热点路径避免动态分发，Scene 生成与 GPU 提交路径保持可批处理、可内联」——Vello 的架构天然支持这一点。
- **纯 Rust 编译链**。Skia 引入 C++ 编译依赖后，CI 矩阵的复杂度、调试难度、贡献者门槛都会显著上升。对于一个需要 10-15 人团队持续开发 4-5 年的项目，编译链的简洁性是长期生产力的基础。
- **成熟度劣势通过 fallback 缓解**。路线书 §9 已规划 Skia 作为 Vello 的 fallback 后端，验证设计中 AV2 负责确认 Skia 集成的可行性。这保证了 Vello 的风险可控。

---

#### 2.1.2 cosmic-text vs Parley + Fontique（文本引擎）

| 维度 | cosmic-text（主路线） | Parley + Fontique（替代路线） |
|------|----------------------|---------------------------|
| **成熟度** | v0.17.x，被 iced 和 COSMIC DE 生产使用 | 较新，目前主要在 Xilem（alpha）中使用 |
| **维护方** | System76（硬件公司，有营收驱动的维护动力） | Linebender（同 Vello，小团队） |
| **CJK 验证** | iced 社区有实际 CJK 使用案例 | 社区反馈较少，主要是 Xilem 内部使用 |
| **编辑 API** | `Buffer` 提供完整的编辑、选区、光标 API | 编辑能力需额外集成 |
| **IME 支持** | iced 已验证 macOS/Windows IME 集成 | 理论上可行，实际案例少 |
| **与 Vello 的关系** | 独立组织，无组织依赖关系 | **同属 Linebender**，存在集中风险 |

**选择 cosmic-text 的原因**：

- **依赖去中心化——这是最关键的理由**。如果主路线选择 Vello + Parley + Fontique，三个核心组件（渲染、文本布局、字体管理）全部依赖 Linebender 一个组织。路线书 §2.1 记录了 Azul 因 WebRender 停止独立维护而死亡的案例——将核心能力集中在单一组织上是历史已经验证过的失败模式。cosmic-text 由 System76 独立维护，有硬件销售收入支撑（不同于依靠捐赠或基金的开源项目），有 iced 作为独立使用者。选择它意味着**渲染链（Linebender）和文本链（System76）各自独立**，一个组织的失败不会同时摧毁两条链。
- **编辑能力更成熟**。路线书 H4 要求 RichText 组件支持选区、IME、Undo/Redo——cosmic-text 的 `Buffer` API 和 iced 的集成案例为这些需求提供了更充分的基础。Parley 的编辑能力验证案例更少。
- **CJK 验证更充分**。cosmic-text 被 COSMIC DE（面向终端用户的桌面环境）使用，CJK 用户的反馈渠道更直接。

**为什么不直接选择 Parley**：不是因为技术劣势，而是因为组织集中风险。如果 V1 验证通过（Vello+cosmic-text 集成可行），就没有理由把文本链也押注到 Linebender。如果 V1 失败，AV1 会验证 Parley 作为替代——此时集中风险仍然存在，但已被识别和接受。

---

#### 2.1.3 Taffy vs Yoga（布局引擎）

| 维度 | Taffy（主路线） | Yoga（替代路线） |
|------|---------------|----------------|
| **语言** | 纯 Rust | C 实现（通过 `yoga-rs` FFI） |
| **CSS Flexbox** | ✅ 完整支持 | ✅ 完整支持 |
| **CSS Grid** | ✅ **完整支持** | ❌ **不支持** |
| **性能** | 竞品基准测试中常优于 Yoga | 稳定但多数场景落后于 Taffy |
| **生态采用** | Dioxus、Bevy、Zed/GPUI、Servo、Slint 等 8+ 项目 | React Native（主要）、少数 Rust 项目 |
| **维护方** | DioxusLabs（VC 支持的公司） | Meta（大公司，但 Yoga 非其核心业务） |
| **CSS 属性映射** | 与 .rgss 样式系统直接对应（都是 CSS 语义） | Flexbox 部分对应，Grid 需框架自行实现 |

**选择 Taffy 的原因**：

- **CSS Grid 是不可放弃的能力**。路线书 §5.6 的布局设计明确依赖 Grid（`grid-template-columns: 200px 1fr`），§5.7 的响应式设计也依赖 Grid 的二维布局能力。如果切换到 Yoga，路线书中所有依赖 Grid 的布局场景都需要改为 Flexbox 嵌套模拟——这会导致组件树层级加深、布局计算量增大、.rgss 样式系统的语义复杂度上升。这不是简单的组件替换，而是整个布局策略的降级。
- **纯 Rust 零 FFI**。布局计算在每帧的热路径上（路线书 §6 数据流中步骤 6），任何 FFI 开销都会直接影响帧预算。Taffy 的纯 Rust 实现允许编译器内联优化，这在 Yoga 的 C FFI 路径上无法做到。
- **CSS 语义一致性**。Taffy 直接实现 CSS Flexbox 和 Grid 算法，其 API 与 CSS 属性命名高度一致。对于路线书 §5.6 定义的 CSS-like 样式系统（`.rgss`），Taffy 的属性映射是 1:1 的——不需要中间翻译层。Yoga 只有 Flexbox，即使是 Flexbox 部分的属性映射也需要额外的适配代码。

**为什么不直接选择 Yoga**：纯 Flexbox 无法满足路线书的需求。即使 Meta 的公司背书更强，能力缺陷使得 Yoga 不能作为主路线。

---

#### 2.1.4 AccessKit vs 直接平台 API（无障碍）

| 维度 | AccessKit（主路线） | 直接平台 API（替代路线） |
|------|-------------------|----------------------|
| **开发模式** | 一套 Rust API，三平台适配由 AccessKit 维护 | 三套独立 API，各自实现和维护 |
| **维护负担** | 由 AccessKit 社区共享 | 完全由框架团队承担 |
| **平台适配质量** | 持续改进（GTK 社区已在贡献） | 取决于框架团队的投入 |
| **已有集成案例** | GTK 4.18、egui、Slint、Bevy | 无完整的 Rust 自建方案 |
| **RichText 支持** | 🔴 基础文本通知可用，结构化富文本语义（段落、样式变化）不支持 | 🟢 可自行实现（但工作量大） |

**选择 AccessKit 的原因**：

- **平台适配是共享基础设施**。无障碍桥接的代码量主要在平台适配层（将统一的语义树翻译为 NSAccessibility / UI Automation / AT-SPI 调用），而不是在框架层的语义树生成。AccessKit 将这一层作为共享基础设施，GTK 4.18 的采用意味着 GNOME 社区也在贡献和维护这部分代码。框架自己维护三套适配代码的代价（路线书预估 6-12 个月额外工作量）远高于接受 AccessKit 当前的功能缺口。
- **RichText 缺口可分层处理**。V3 验证如果确认 AccessKit 的 RichText 支持不在短期内可用，替代方案不是「全部自建」，而是「AccessKit 处理基础控件 + 框架自建 RichText 桥接」。这与路线书 §5.5 的架构设计一致——无障碍树是框架层生成的独立数据结构，后端可插拔。

**为什么不直接自建**：维护成本不对等。GUI 框架是一个需要 10-15 人团队开发 4-5 年的项目，如果再增加三平台无障碍适配的长期维护负担，会直接挤占核心组件（DataGrid、Form、RichText）的开发资源。路线书 §2.1 失败原因 3 已经记录了「团队规模与目标不匹配」是 Rust GUI 框架的主要死因。

---

#### 2.1.5 主技术栈的协同优势

上述四个选择并非孤立的——它们组合在一起形成了四个替代路线无法同时满足的协同优势：

```
┌────────────────────────────────────────────────────────────┐
│                    主技术栈四大协同优势                      │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  1. 全 Rust 编译链（零 C++ FFI）                            │
│     cargo build 一条命令完成全量构建                         │
│     CI 矩阵简洁、贡献者门槛低、调试统一（rust-gdb/lldb）      │
│     替代路线中 AV2（Skia）破坏此优势                         │
│                                                            │
│  2. 依赖去中心化（5 组织，无单点）                           │
│     Linebender → Vello     (GPU 渲染)                       │
│     System76   → cosmic-text (文本)                         │
│     DioxusLabs → Taffy     (布局)                           │
│     社区       → winit     (窗口)                           │
│     STF+GTK    → AccessKit (无障碍)                         │
│     AV1+AV2 组合会将渲染+文本集中在 Linebender 一家          │
│                                                            │
│  3. CSS 语义全链路对齐                                      │
│     .rgss → Taffy → Vello Scene → wgpu                     │
│     样式属性 → 布局计算 → 渲染坐标 → GPU 提交               │
│     每个环节都使用 CSS 语义，不需要概念翻译层                 │
│     替代路线中 AV4（Yoga）破坏 Grid 语义对齐                 │
│                                                            │
│  4. GPU compute 批处理渲染路径                               │
│     Scene 构建 → compute shader 并行处理 → 单次提交          │
│     路线书 §6 数据流目标：全流程 < 17ms                       │
│     替代路线中 AV2（Skia）使用传统 API，无法达到同样并行度     │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

| 协同点 | 主路线 | 最佳替代组合 | 替代路线的损失 |
|--------|--------|------------|--------------|
| **全 Rust 编译链** | ✅ | AV1（Parley 替代 cosmic-text）仍保持 | AV2（Skia）破坏，引入 C++ 依赖 |
| **依赖去中心化** | ✅ 5 组织 | AV1 将文本从 System76 移到 Linebender | 渲染+文本集中到 Linebender 一家 |
| **CSS Grid 支持** | ✅ Taffy | AV1+AV2 仍保持 | AV4（Yoga）丧失 Grid |
| **GPU compute 批处理** | ✅ Vello | AV1 仍保持 | AV2（Skia）丧失 compute 优势 |
| **无障碍社区维护** | ✅ AccessKit + GTK | AV3（自建）丧失社区共享 | 全部维护成本由框架承担 |

#### 2.1.6 选择总结

| 主技术 | 替代技术 | 选择主技术的核心理由 | 触发替代的不可接受条件 |
|--------|---------|-------------------|---------------------|
| Vello | Skia | 纯 Rust + GPU compute + 编译快 | Vello 停止维护超过 12 个月且无社区接手 |
| cosmic-text | Parley+Fontique | 去中心化（不与 Vello 同组织） | Vello+cosmic-text 集成不可行，或 cosmic-text 停止维护 |
| Taffy | Yoga | CSS Grid 刚性需求 + 纯 Rust | Taffy 停止维护超过 12 个月 |
| AccessKit | 直接平台 API | 社区共享维护成本 + GTK 背书 | RichText 缺口不可接受且无法通过贡献补丁解决 |

---

## 3. 验证项总览

| # | 验证项 | 映射目标 | 涉及技术 | 风险 | 验证方式 | 预估工作量 |
|---|--------|---------|---------|------|---------|-----------|
| V1 | Vello + cosmic-text 协同渲染 | H3, H4 | Vello, cosmic-text, wgpu | 🔴 致命 | POC 代码 | 3-5 人天 |
| V2 | cosmic-text CJK 文本渲染质量 | H3, H4 | cosmic-text, 字体 | 🔴 致命 | POC 扩展 | 2-3 人天 |
| V3 | AccessKit 能力边界 | H5 | AccessKit | 🔴 致命 | 调研分析 + 实际验证 | 2-3 人天 |
| V4 | 渲染管线三平台可运行 | H3 | winit, wgpu, Vello | 🟡 重大 | CI 矩阵 | 3-5 人天 |
| V5 | WidgetView diff 性能基准 | H2 | 纯 Rust 代码 | 🟡 重大 | Benchmark | 2-3 人天 |
| V6 | Taffy 布局→渲染坐标转换 | H3, H4 | Taffy + 渲染管线 | 🟡 重大 | POC 扩展 | 3-5 人天 |
| V7 | 状态快照性能基准 | H2 | serde, PersistState | 🟡 重大 | Benchmark | 1-2 人天 |
| V9 | DataGrid 虚拟滚动前提确认 | H4 | Vello 场景图 | 🟢 低 | 文档确认 | 0.5 人天 |
| V10 | cosmic-text IME 集成路径 | H3, H4 | cosmic-text, winit | 🟡 重大 | POC 扩展 | 5-8 人天 |
| AV1 | Parley+Fontique 替代 cosmic-text | H3, H4 | Parley, Fontique, Vello | 🟡 重大 | POC 代码 | 2-3 人天 |
| AV2 | Skia 替代 Vello | H3, H4 | skia-safe, wgpu | 🟡 重大 | POC 代码（**必须执行**） | 2-3 人天 |
| AV3 | 直接平台 API 替代 AccessKit | H5 | NSAccessibility, UI Automation, AT-SPI | 🟡 重大 | 架构设计 + 原型 | 1-2 人天 |
| AV4 | Yoga 替代 Taffy | H3, H4 | yoga-rs | 🟢 低 | POC 代码 | 1-2 人天 |

**最小可行验证集（MVP 验证）**：V1 + V2 + V3 + V4，总工作量约 9-15 人天。这 4 项中任何一项被证伪都需要调整技术路线。

**替代技术验证触发条件**：AV2（Skia）为**验证阶段必须执行项**——Vello 是技术栈中维护风险最高的组件，其逃生通道必须在阶段 0 启动前确认可用。AV1、AV3、AV4 不要求在验证阶段全部执行——仅当对应主技术验证失败，或路线书 §2 中的依赖风险评估被触发时，才启动对应的替代验证项。详见 [§5.4 替代技术验证触发矩阵](#54-替代技术验证触发矩阵)。

---

## 4. 关键验证项详设

### V1：Vello + cosmic-text 协同渲染

#### 要验证的假设

cosmic-text 生成的字形光栅数据可以与 Vello 的场景图组合，在同一帧内完成绘制，无需额外的数据拷贝或格式转换开销。

#### 背景

- cosmic-text 通过 `SwashCache` 将字形光栅化为 8-bit alpha bitmap（或 subpixel RGBA bitmap）
- Vello 的 Scene 通过 `draw_image` 或 `draw_glyphs` 消费图像/字形数据
- 两者的集成点在于：cosmic-text 产生的 glyph bitmap 能否直接（或低开销转换后）传给 Vello

#### POC 代码结构

```
verify/v1-vello-cosmic/
├── Cargo.toml
└── src/
    └── main.rs          # 约 200 行
```

**依赖（Cargo.toml）**：

```toml
[package]
name = "verify-v1-vello-cosmic"
version = "0.1.0"
edition = "2021"

[dependencies]
winit = "0.30"
wgpu = "24"
vello = "0.8"
cosmic-text = "0.17"
# 若 cosmic-text 的 fontdb 版本与 Vello 间接依赖冲突，需调整版本
```

#### 关键代码路径

```
fn main():
  1. winit::EventLoop::new()
  2. wgpu::Instance → adapter → device → queue
  3. winit::Window → wgpu::Surface
  4. Vello::Renderer::new()
  5. cosmic_text::FontSystem::new()
  6. cosmic_text::Buffer::new() → set_text("你好，世界！Rust GUI 验证")
  7. 进入事件循环

fn render():
  1. cosmic_text::Buffer::shape_until_scroll()
  2. 遍历 glyph 布局结果
  3. SwashCache::get_image() → 获取字形 bitmap
  4. 将 bitmap 作为 Vello Image 添加到 Scene
  5. 或者：直接使用 Vello 的 draw_glyph() 接口（若存在）
  6. Vello::Renderer::render_to_surface()
```

#### 验证点

| # | 验证点 | 检查方法 |
|---|--------|---------|
| 1 | 字形 bitmap 能否转换为 Vello Image | 编译通过 + 运行时无 panic |
| 2 | 渲染帧包含文本内容 | 截图包含可辨认的文字 |
| 3 | 文本无模糊/断裂/缺失 | 视觉检查截图 |
| 4 | 帧率 ≥ 60fps | 帧计数器 |
| 5 | 无内存泄漏或 GPU 资源泄漏 | 运行 10 分钟后资源使用稳定 |

#### 通过标准

- macOS + Linux 至少一个平台可编译运行
- 窗口显示中文文本，字形清晰可辨
- 稳定运行 10 分钟无崩溃

#### 失败应对

| 失败模式 | 应对方案 |
|---------|---------|
| cosmic-text glyph bitmap 格式与 Vello Image 不兼容 | 增加中间转换步骤（SwashCache → raw RGBA buffer → wgpu Texture → Vello Image），测量额外开销 |
| 两者依赖冲突（如 wgpu 版本不一致） | 尝试调整版本组合；若无法解决，评估改用 Parley+Fontique（Xilem 的文本栈） |
| 性能不足（< 60fps） | 分析瓶颈：字形光栅化 → 缓存策略；纹理上传 → atlas 批处理 |

---

### V2：cosmic-text CJK 文本渲染质量

#### 要验证的假设

cosmic-text 的 harfbuzz shaping + 字体回退链可以正确渲染 CJK（中日韩）文本，包括简体中文、繁体中文、日文、韩文、中英混排、Emoji 和双向文本（Bidi）。

#### 测试文本矩阵

```
类别            测试文本
────────────────────────────────────────────────
简体中文         你好，世界！这是一个 Rust GUI 框架的技术验证。
繁体中文         這是一個繁體中文測試範例。
日文             こんにちは、世界！
韩文             안녕하세요, 세계!
中英混排         显示 1,024 条记录（共 10,240 条）
数字与符号       第 1/10 页 · 进度 95.5% · 金额 ¥12,800
Emoji            ✅ 通过  🔴 失败  🟡 警告  📊📋🔍
Bidi             English text مرحبا 混合排版 test
多字体混排       中文正文 with English terms 和日本語混在
生僻字           囧 𬭤 𬎆 㑳 㒭（surrogate pair）
```

#### 验证方法

在 V1 POC 基础上扩展，添加一个测试循环：遍历上述文本，每 2 秒切换一条，对每条文本截图保存。

#### 通过标准

- 所有测试文本均正确渲染
- 无 tofu（缺字方块 □）
- 无乱序
- Bidi 方向正确
- Emoji 彩色渲染（若 cosmic-text 配置正确）
- 生僻字正确回退（若内置字体未覆盖）

#### 字体配置

```
默认使用框架计划内置的字体：
  - Noto Sans CJK（覆盖中日韩）
  - Inter（覆盖拉丁/西里尔）
  - Noto Color Emoji（Emoji）

POC 中可通过 cosmic_text::fontdb::Source::Binary 嵌入字体数据。
```

---

### V3：AccessKit 能力边界分析

#### 要验证的假设

AccessKit 能够支撑 WCAG 2.1 AA 的全部要求，特别是 RichText 组件的无障碍支持。

#### 验证方法

此验证项以**调研分析**为主，辅以**最小实际验证**。

**调研部分**：通过阅读 AccessKit 的文档、源码和 GitHub Issues，逐项评定其能力覆盖情况。

**实际验证部分**：使用 egui 或 Slint（它们已集成 AccessKit）创建一个包含 Button + TextField + Checkbox 的简单界面，在至少一个平台上通过屏幕阅读器（macOS VoiceOver / Windows NVDA / Linux Orca）实际测试基础控件的无障碍行为，确认 AccessKit 桥接的实际质量。

#### 分析维度

| 能力维度 | 调研方法 | 目标结论 |
|---------|---------|---------|
| 基础控件支持 | 查阅 AccessKit 文档的 Role 枚举 | 列出已支持和缺失的 Role |
| RichText / Hypertext | 搜索 GitHub Issues 中的相关讨论 | 明确是否在路线图中，预计何时可用 |
| macOS 适配器质量 | 阅读 `accesskit_macos` 源码和已知问题 | 评估 macOS 上的风险等级 |
| IME 时的无障碍事件 | 查阅 winit + AccessKit 集成文档 | IME 组合态文本是否能通知屏幕阅读器 |
| 键盘导航支持 | 确认 AccessKit 是否提供焦点管理，还是框架自行实现 | 明确框架需要自行实现的部分 |
| 高对比度主题 | 确认是否有系统主题检测 API | 是否可查询系统高对比度设置 |
| Android 支持 | 查阅 `accesskit_android` 状态（v0.3.0） | 远期验证 |

#### 调研产出

一份不超过 5 页的调研报告（`verify/v3-accesskit-gap/report.md`），包含：

1. AccessKit 当前能力矩阵（按控件类型列出支持状态）
2. 与 WCAG 2.1 AA 成功标准的逐条映射
3. 识别出的能力缺口及缓解方案
4. 对 RichText 无障碍的专项建议

#### 通过标准

- 基础控件（Button、TextField、Checkbox、Table、List）的状态明确（支持/不支持/实验性）
- 基础控件（Button、TextField、Checkbox）的 AccessKit 桥接在至少一个平台上通过屏幕阅读器实际验证
- RichText 无障碍的实现路径明确（等待 AccessKit 支持 OR 框架自行桥接 OR 降级目标）
- 没有「未知」「待确认」的红色区域

---

### V4：渲染管线跨平台三端可运行

#### 要验证的假设

winit + wgpu + Vello 的依赖链可以在 macOS/Metal、Windows/DX12、Linux/Vulkan 三端成功编译并运行。

#### POC 代码结构

在 V1 的基础上增加离屏渲染能力和 CI 脚本：

```
verify/v4-cross-platform/
├── v1-vello-cosmic/          # 复用 V1 的渲染代码
├── .github/
│   └── workflows/
│       └── cross-platform.yml # CI 矩阵配置
├── scripts/
│   ├── headless-screenshot.sh  # macOS/Linux 离屏截图脚本
│   └── headless-screenshot.ps1 # Windows 离屏截图脚本
└── README.md                   # 运行说明
```

#### CI 矩阵设计

```yaml
# .github/workflows/cross-platform.yml
jobs:
  verify:
    strategy:
      matrix:
        os: [ubuntu-24.04, macos-15, windows-2025]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo build --release
      - run: cargo run --release -- --headless --screenshot verify.png
      - uses: actions/upload-artifact@v4
        with:
          name: screenshot-${{ matrix.os }}
          path: verify.png
```

#### 验证点

| # | 验证点 | 平台 |
|---|--------|------|
| 1 | 编译成功（无平台特定编译错误） | macOS / Windows / Linux |
| 2 | 运行时无 panic | macOS / Windows / Linux |
| 3 | 离屏截图不为空白/全黑 | macOS / Windows / Linux |
| 4 | 三平台截图基本一致（允许亚像素差异） | 跨平台对比 |

#### 通过标准

- 三平台均编译通过
- 三平台均生成有效截图（非空白、非全黑、包含文本内容）
- 截图内容视觉一致（相同的文本、布局）

#### 注意事项

- CI 中 GPU 可用性取决于 GitHub Actions runner（Linux 可用 `xvfb` + `swiftshader`，macOS 有 Metal 软件光栅化器，Windows 可能需要特殊 runner 或跳过）
- 若 CI 中无 GPU，可使用 `wgpu` 的 `vulkan-portability` 或 `angle` 后端做软件渲染
- 作为替代方案，可手动在三台物理机上运行并截图对比

---

### V5：WidgetView diff 性能基准

#### 要验证的假设

声明式视图的 diff-and-patch 可以在 1ms 内完成 1000 节点 WidgetView 树的比较。

#### POC 代码结构

```
verify/v5-diff-bench/
├── Cargo.toml
├── benches/
│   └── diff_bench.rs     # Criterion benchmark
└── src/
    ├── lib.rs            # WidgetView 类型定义
    └── diff.rs           # diff 算法实现
```

#### WidgetView 最小定义（用于 benchmark）

```rust
/// WidgetView 是轻量值类型，只描述结构
#[derive(Clone, PartialEq, Debug)]
pub struct WidgetView {
    pub widget_type: &'static str,
    pub id: Option<WidgetId>,
    pub key: Option<Key>,
    pub props: BTreeMap<String, PropValue>,
    pub children: Vec<WidgetView>,
}

/// 属性值类型
#[derive(Clone, PartialEq, Debug)]
pub enum PropValue {
    Str(String),
    Bool(bool),
    Num(f64),
    Color(u32),
    Size(Size),
}

/// diff 结果（patch 指令）
#[derive(Debug)]
pub enum Patch {
    UpdateProps {
        id: WidgetId,
        props: BTreeMap<String, PropValue>,
    },
    Replace {
        id: WidgetId,
        new: WidgetView,
    },
    Insert {
        parent: WidgetId,
        index: usize,
        child: WidgetView,
    },
    Remove {
        parent: WidgetId,
        index: usize,
    },
    Move {
        parent: WidgetId,
        from: usize,
        to: usize,
    },
}
```

#### Benchmark 场景

```rust
// 场景 1：100 节点，50% 属性变更
fn bench_100_nodes_50pct_prop_change(c: &mut Criterion) { ... }

// 场景 2：1000 节点，10% 属性变更
fn bench_1000_nodes_10pct_prop_change(c: &mut Criterion) { ... }

// 场景 3：1000 节点，30% 节点替换
fn bench_1000_nodes_30pct_replace(c: &mut Criterion) { ... }

// 场景 4：列表场景（100 项，keyed，插入/删除/重排各 20%）
fn bench_keyed_list_100_items(c: &mut Criterion) { ... }

// 场景 5：深嵌套场景（深度 50，每层 2 子节点，共 100 节点）
fn bench_deep_nesting_100_nodes(c: &mut Criterion) { ... }
```

#### 通过标准

| 场景 | 目标延迟（中位数，release 模式） |
|------|-------------------------------|
| 100 节点，50% 属性变更 | < 100µs |
| 1000 节点，10% 属性变更 | < 1ms |
| 1000 节点，30% 替换 | < 2ms |
| 100 项 keyed 列表（20% 变更） | < 500µs |
| 深嵌套 100 节点 | < 200µs |

#### 硬件基准

- 主要基准：Apple M1 Pro 或同等性能 ARM64 CPU
- 辅助基准：CI 中的 x86_64 Linux runner（GitHub Actions ubuntu-24.04）
- 结果需注明测试硬件，两个架构的数据均记录

---

### V6：Taffy 布局 → 渲染坐标转换

#### 要验证的假设

Taffy 生成的数值布局结果可以正确翻译为渲染坐标，实际渲染结果与预期布局一致。

#### POC 代码结构

```
verify/v6-taffy-layout/
├── Cargo.toml
├── src/
│   ├── main.rs          # 布局→渲染集成
│   ├── cases/
│   │   ├── flex_row.rs
│   │   ├── flex_column.rs
│   │   ├── grid.rs
│   │   └── nested.rs
│   └── screenshot.rs    # 截图对比工具
└── expected/             # 预期布局示意图（手绘 SVG 或描述文件）
    ├── flex_row.svg
    ├── flex_column.svg
    ├── grid.svg
    └── nested.svg
```

#### 测试用例

```
用例 1：Flexbox Row
  Container { display: flex; flex-direction: row; gap: 12px; padding: 16px }
  ├─ Button "保存" (width: 80px, height: 36px)
  ├─ Button "取消" (width: 80px, height: 36px)
  └─ Button "帮助" (width: 80px, height: 36px)

用例 2：Flexbox Column
  Container { display: flex; flex-direction: column; gap: 8px; padding: 16px }
  ├─ Text "用户设置" (font-size: 20px)
  ├─ TextField (height: 40px, flex-grow: 1)
  └─ Button "提交" (width: 120px, height: 36px)

用例 3：CSS Grid
  Container { display: grid; grid-template-columns: 200px 1fr; gap: 16px }
  ├─ Sidebar "导航" (grid-column: 1)
  └─ Content "内容区域" (grid-column: 2)

用例 4：嵌套布局
  Column { padding: 16; gap: 12 }
  ├─ Row { gap: 8 }  → [Icon, Title, Spacer, Button]
  ├─ Row { gap: 8 }  → [Sidebar, Content]
  │   ├─ Column { gap: 4 } → [NavItem × 5]
  │   └─ Column { gap: 8 } → [Card × 3]
  └─ Row { justify: end } → [Button × 2]
```

#### 验证点

| # | 验证点 | 方法 |
|---|--------|------|
| 1 | Taffy 计算的坐标是否传递给 Vello Scene 的正确 Rect | 截图与预期布局对比 |
| 2 | padding / margin / gap 是否正确生效 | 测量截图中元素间距 |
| 3 | flex-grow 是否正确分配剩余空间 | 测量元素宽度/高度比例 |
| 4 | 嵌套布局的子元素是否正确定位（相对 vs 绝对坐标） | 截图分析 |
| 5 | 窗口 resize 后布局是否重新计算 | 多尺寸截图对比 |

#### 通过标准

- 4 个测试用例的截图与预期布局肉眼一致
- resize 后布局正确更新

---

### V7：状态快照性能基准

#### 要验证的假设

持久状态的序列化和反序列化不会成为快速重启的性能瓶颈。

#### POC 代码结构

```
verify/v7-snapshot-bench/
├── Cargo.toml
├── benches/
│   └── snapshot_bench.rs
└── src/
    ├── lib.rs            # PersistState trait 最小定义
    └── fixtures.rs       # 测试数据生成
```

#### 基准测试场景

```rust
// 场景 1：TODO 应用规模
//   50 个 widget，总计 ~5KB 序列化后数据
fn bench_todo_app_serialize(c: &mut Criterion) { ... }
fn bench_todo_app_deserialize(c: &mut Criterion) { ... }

// 场景 2：CRUD 管理后台规模
//   200 个 widget（含 DataGrid 100 行），总计 ~50KB
fn bench_crud_serialize(c: &mut Criterion) { ... }
fn bench_crud_deserialize(c: &mut Criterion) { ... }

// 场景 3：压力测试
//   1000 个 widget，总计 ~500KB
fn bench_pressure_serialize(c: &mut Criterion) { ... }
fn bench_pressure_deserialize(c: &mut Criterion) { ... }

// 场景 4：Schema 迁移
//   模拟 v1 → v2 添加一个可选字段
fn bench_schema_migration_200_widgets(c: &mut Criterion) { ... }
```

#### 通过标准

| 场景 | 序列化 | 反序列化 |
|------|--------|---------|
| TODO 应用 (5KB) | < 1ms | < 1ms |
| CRUD 后台 (50KB) | < 10ms | < 100ms |
| 压力测试 (500KB) | < 50ms | < 500ms |

#### 实现提示

- 使用 `serde_json` 或 `postcard`（二进制格式，性能更优）
- `PersistState` trait 参考路线书 §5.3 的定义
- Schema 迁移器设计为 `trait SchemaMigration`，每个版本一个实现

---

### V9：DataGrid 虚拟滚动性能前提确认

#### 要验证的假设

Vello 的场景图模型支持增量更新和裁剪，能够支撑 100k 行数据的虚拟滚动渲染。

#### 验证方法

纯文档确认，无需编码。查阅 Vello 文档确认以下能力：

| 能力 | 确认方法 |
|------|---------|
| Scene 是否支持增量更新（仅更新变化片段） | 查阅 Vello API 和 architecture 文档 |
| 是否支持 ClipRect（视口裁剪） | 查阅 Vello Scene API |
| GPU buffer 动态更新性能 | 查阅 wgpu buffer 写入最佳实践 |

#### 通过标准

确认上述 3 项能力均在 Vello 的设计目标或 API 中存在。技术路线书中的风险缓解已覆盖（Skia fallback）。实际性能验证在阶段 2 到达时进行。

---

### V10：cosmic-text IME 集成路径

#### 要验证的假设

winit 的 IME 事件 + cosmic-text 的编辑缓冲 + Vello 的候选窗位置控制，可以形成完整的输入法交互链。

#### POC 代码结构

```
verify/v10-ime/
├── Cargo.toml
└── src/
    ├── main.rs          # IME 集成验证
    └── ime_tester.rs    # IME 事件序列模拟
```

#### IME 事件处理链

```
winit Ime::Preedit(value, cursor_range)
  → cosmic_text::Buffer::set_text()（包含组合态文本）
  → cosmic_text::Buffer::set_visible_area()（光标位置）
  → cosmic_text::Buffer::shape_until_scroll()
  → 渲染：绘制文本 + 绘制组合态下划线/高亮

winit Ime::Commit(text)
  → cosmic_text::Buffer::set_text()（提交文本替换组合态）
  → 渲染：正常文本
```

#### 验证点

| # | 验证点 | 平台依赖 |
|---|--------|---------|
| 1 | winit IME 事件能否正确触发 | macOS ≥ Windows > Linux |
| 2 | cosmic-text Buffer 能否处理 preedit（组合态）文本 | 跨平台 |
| 3 | 候选窗位置计算（光标屏幕坐标） | 需在 winit Window 上计算 |
| 4 | macOS 上的 CJK 输入法完整流程 | macOS |
| 5 | Windows 上的 CJK 输入法完整流程 | Windows |

#### 通过标准

- 至少 macOS 上可实现「拼音输入 → 候选窗显示 → 选字 → 文本确认」的完整流程
- 组合态文本与已确认文本视觉可区分

#### 已知难度

IME 是公认的复杂跨平台问题。winit 的 IME 支持在各平台的完整度不同：

- macOS：基本完整（`Ime::Preedit` + `Ime::Commit`）
- Windows：基本完整
- Linux：取决于输入法框架（fcitx5 / ibus），winit 对此支持相对较弱

此验证项可在 V1-V4 通过后再启动，不作为阻塞项。

#### 与 AV1 的交互

如果 V1 失败触发 AV1（切换到 Parley + Fontique 替代 cosmic-text），V10 的 IME 验证路径需要调整：
- `cosmic_text::Buffer` 的编辑 API 需替换为 Parley 对应的编辑接口
- 候选窗位置计算逻辑不变（依赖 winit Window 坐标，与文本引擎无关）
- AV1 执行时需同步验证 Parley 的 preedit 文本支持

---

### AV1：Parley + Fontique 替代 cosmic-text

#### 触发条件

以下任一情况发生时启动此验证：

1. **V1 失败**：Vello + cosmic-text 无法协同渲染，且无法通过中间转换层解决
2. **cosmic-text 维护风险触发**：System76 宣布停止 COSMIC DE 开发，或 cosmic-text 超过 12 个月无实质更新
3. **关键能力缺失**：cosmic-text 在 V2 验证中暴露出无法修复的 CJK/Bidi/Emoji 渲染缺陷

#### 背景

Parley + Fontique 是 Linebender 组织的文本栈（与 Vello 同组织），已在 Xilem 中实际使用。选择它作为替代方案的理由：

- 与 Vello 同组织，集成路径已有 Xilem 作为参照
- Parley 提供文本 shaping 和布局（基于 harfbuzz），功能集与 cosmic-text 重叠
- Fontique 提供字体发现和管理，对应 cosmic-text 的 FontSystem
- Xilem 项目已经验证了两者可以与 Vello 协同工作

#### 要验证的假设

Parley + Fontique + Vello 的组合可以正确渲染 CJK 文本，且集成复杂度在可接受范围内。

#### POC 代码结构

```
verify/av1-parley-fontique/
├── Cargo.toml
└── src/
    └── main.rs          # 约 200 行，复刻 V1 的功能
```

**依赖（Cargo.toml）**：

```toml
[package]
name = "verify-av1-parley-fontique"
version = "0.1.0"
edition = "2021"

[dependencies]
winit = "0.30"
wgpu = "24"
vello = "0.8"
parley = "0.2"
fontique = "0.1"
# 版本号以实际发布版本为准
```

#### 验证点

| # | 验证点 | 检查方法 |
|---|--------|---------|
| 1 | Parley + Fontique + Vello 能否在同一 wgpu 上下文中运行 | 编译通过 |
| 2 | CJK 文本能否正确渲染（使用 AV1 专用测试文本矩阵） | 截图对比 |
| 3 | Parley 的 glyph 输出格式与 Vello 的集成接口 | 代码路径确认 |
| 4 | Xilem 的集成代码可作为参考实现 | 阅读 Xilem 源码确认集成模式 |

#### 通过标准

- macOS 上编译通过并显示 CJK 文本
- 集成代码量在 200 行以内（说明切换成本可控）
- 字体回退链可正确覆盖 CJK

#### 与 V1 的关系

- V1 通过→AV1 可延期至阶段 1 末尾执行（作为后备验证）
- V1 失败→AV1 立即执行，结果将替代 V1 的结论

---

### AV2：Skia 替代 Vello

#### 触发条件

AV2 是**验证阶段必须执行项**——Vello 是整个技术栈中维护风险最高的组件，其逃生通道必须在阶段 0 启动前确认可用。

以下任一情况发生时**也必须**启动此验证（如果尚未在验证阶段执行）：

1. **Vello 维护风险触发**：Linebender 组织解散，Vello 超过 12 个月无实质更新且无社区接手
2. **V4 跨平台失败**：Vello 在某个目标平台上无法运行且短期内无修复计划
3. **性能不达标**：Vello 在后续验证中无法满足 DataGrid 虚拟滚动等性能要求

**执行时机**：在 V1+V4 通过后（约第 3-4 周）执行，与 V2 并行。

#### 背景

路线书 §9 已规划 Skia 作为 Vello 的 fallback 后端。Google Skia 是 Chrome/Flutter/Android 使用的 2D 图形库，Rust 生态通过 `skia-safe` 提供绑定。

选择 Skia 的理由：
- Google 持续维护 20+ 年，不会突然死亡
- Chrome、Flutter、Android 均依赖它，维护动力极强
- Rust 绑定（`skia-safe`）已成熟，被多个项目使用
- 内置完整的文本渲染（通过 HarfBuzz + ICU），不依赖 cosmic-text 的字形光栅化

**重要**：如果切换到 Skia，文本渲染可以统一使用 Skia 内置的文本 pipeline（Skia 自己处理 shaping + glyph rasterization），从而降低对 cosmic-text 字形光栅的依赖。但这并不意味着完全移除 cosmic-text——文本 shaping 和 Bidi 仍可复用 cosmic-text，只是字形光栅化由 Skia 完成。

#### 要验证的假设

`skia-safe` 可以在 macOS 上编译，并配合 wgpu（或 Skia 自己的 GPU 后端）渲染基本图形和文本。

#### POC 代码结构

```
verify/av2-skia/
├── Cargo.toml
└── src/
    └── main.rs          # 约 150 行
```

**依赖（Cargo.toml）**：

```toml
[package]
name = "verify-av2-skia"
version = "0.1.0"
edition = "2021"

[dependencies]
winit = "0.30"
skia-safe = "0.75"
```

#### 关键代码路径

```
fn main():
  1. winit::EventLoop::new() + Window::new()
  2. Skia::GPU::DirectContext::new_gl() 或 new_metal()
  3. Skia::Surface::new_render_target()
  4. Skia::Canvas → draw_rect() + draw_string("你好，世界！")
  5. 事件循环中：
     - Skia flush → 获取 pixel data
     - 通过 wgpu 或软绘制呈现在窗口中
```

#### 验证点

| # | 验证点 | 检查方法 |
|---|--------|---------|
| 1 | skia-safe 能否在 macOS 上编译 | `cargo build` |
| 2 | 能否创建 GPU 加速的 Skia Surface | 运行时无 panic |
| 3 | CJK 文本能否渲染 | 截图包含中文文字 |
| 4 | Skia 的文本渲染质量与 cosmic-text 对比 | 并排截图对比 |

#### 通过标准

- macOS 上编译通过
- 渲染包含中文文本的矩形
- 文本渲染质量不低于 cosmic-text（视觉评估）

#### 注意事项

- `skia-safe` 的编译时间较长（Skia 是大型 C++ 项目），首次构建可能 > 30 分钟
- 若 `skia-safe` 版本与当前系统不兼容，可使用 CPU 后端（RasterSurface）替代 GPU 后端进行验证
- 此验证项只需确认基本可行性，不要求达到 Vello 同等的渲染质量

---

### AV3：直接平台 API 替代 AccessKit

#### 触发条件

以下任一情况发生时启动此验证：

1. **V3 发现致命缺口**：AccessKit 的 RichText 支持不在其路线图中，且框架无法接受降级方案
2. **AccessKit 维护风险触发**：STF 资助结束 + 主要维护者离开 + GTK 社区未接手
3. **平台覆盖不足**：AccessKit 的 macOS 或 Android 适配器长期处于低质量状态

#### 背景

AccessKit 的本质是「平台无障碍 API 的统一抽象层」。如果它不可用，框架可以自行实现各平台的无障碍桥接。这不是从头实现无障碍（那将极其困难），而是直接调用各平台的原生 API。

各平台 API 的 Rust 调用途径：

| 平台 | API | Rust 调用途径 |
|------|-----|-------------|
| macOS | NSAccessibility | `objc2` crate 或 `icrate` crate |
| Windows | UI Automation | `windows` crate（COM 接口） |
| Linux | AT-SPI (D-Bus) | `zbus` crate |

#### 验证方法

此验证项以**架构设计**为主，辅以最小原型验证。不需要实现完整的三平台桥接。

#### 架构设计要点

框架必须将无障碍后端抽象为 trait，使 AccessKit 和直接平台 API 可以互换：

```rust
/// 框架内部的无障碍后端抽象
pub trait AccessibilityBackend: Send + Sync {
    /// 推送完整的无障碍树更新
    fn push_tree_update(&mut self, update: AccessibilityTreeUpdate);

    /// 通知焦点变化
    fn set_focus(&mut self, node_id: NodeId);

    /// 通知值变化（如滑块位置、文本内容）
    fn notify_value_change(&mut self, node_id: NodeId, value: &str);

    /// 通知选中状态变化
    fn notify_selection_change(&mut self, node_id: NodeId);
}

/// AccessibilityTreeUpdate 由框架生成，与后端无关
pub struct AccessibilityTreeUpdate {
    pub root: NodeId,
    pub nodes: Vec<AccessibilityNode>,
    pub focus: Option<NodeId>,
}

pub struct AccessibilityNode {
    pub id: NodeId,
    pub role: AccessibilityRole,
    pub name: String,
    pub description: String,
    pub value: Option<String>,
    pub bounds: Rect,
    pub children: Vec<NodeId>,
    pub actions: Vec<AccessibilityAction>,
    pub states: Vec<AccessibilityState>,
}
```

#### 最小原型：macOS NSAccessibility 调用验证

仅验证最低限度：能否从 Rust 创建一个 NSAccessibility 对象并通过系统 API 发送通知。

```
verify/av3-platform-a11y/
├── Cargo.toml
└── src/
    └── main.rs          # 约 100 行，macOS only
```

```rust
// 最小原型：创建 NSAccessibilityElement 并设置属性
use objc2::rc::Retained;
use objc2_foundation::NSObject;
// 使用 NSAccessibility 协议（macOS 系统框架）
```

#### 验证点

| # | 验证点 | 检查方法 |
|---|--------|---------|
| 1 | `objc2` 能否调用 NSAccessibility API | macOS 编译通过 |
| 2 | 能否创建 NSAccessibilityElement | 运行并验证 VoiceOver 可读取 |
| 3 | 设计 `AccessibilityBackend` trait 的 Platform 变体 | 设计审查（无需编码） |

#### 通过标准

- macOS 上可创建 NSAccessibilityElement 并通过 VoiceOver 验证
- `AccessibilityBackend` trait 设计覆盖基础控件（Button、TextField、Table、Grid）的需求
- 确认三平台 API 的调用路径均存在 Rust crate 支持

#### 工作量说明

此验证项以设计为主。如果发现需要完全自建三平台桥接，实际实现工作在阶段 1-2 中额外需要 6-12 个月。本验证项的目标是尽早确认「自建桥接在技术上可行」以及「框架的抽象层设计正确」，从而在需要时可以启动自建而不必推翻框架架构。

---

### AV4：Yoga 替代 Taffy

#### 触发条件

以下情况发生时启动此验证：

1. **Taffy 维护风险触发**：DioxusLabs 停止维护 Taffy，且社区无其他大型项目接手
2. **关键能力缺失**：Taffy 在实际使用中暴露出 CSS Grid 实现的正确性问题，影响核心布局场景

#### 背景

Yoga 是 Meta（Facebook）开发的跨平台布局引擎，用 C 实现，是 React Native 的布局核心。Rust 生态通过 `yoga-rs`（或 `stretch2`）提供绑定。

选择 Yoga 的理由：
- Meta 持续维护（React Native 依赖它）
- 实现了完整的 Flexbox 布局（但不支持 CSS Grid——这是主要能力差异）
- C 实现，性能稳定
- 已在生产环境中大规模验证

**能力差异**：Yoga 不支持 CSS Grid，仅支持 Flexbox。如果切换到 Yoga，框架需要自行实现 Grid 布局或降级 Grid 能力。这是 Avalon 方案的一个重要限制。

#### 要验证的假设

`yoga-rs` 可以在 macOS 上编译，计算基本 Flexbox 布局，且结果可翻译为渲染坐标。

#### POC 代码结构

```
verify/av4-yoga/
├── Cargo.toml
└── src/
    └── main.rs          # 约 100 行
```

**依赖（Cargo.toml）**：

```toml
[package]
name = "verify-av4-yoga"
version = "0.1.0"
edition = "2021"

[dependencies]
yoga = "0.6"
```

#### 验证代码

```rust
use yoga::{AlignItems, Direction, FlexDirection, JustifyContent, Node, StyleUnit};

fn main() {
    let mut root = Node::new();
    root.set_style(
        Style::new()
            .set_flex_direction(FlexDirection::Row)
            .set_padding(StyleUnit::Point(16.0.into()))
            .set_gap(StyleUnit::Point(12.0.into()))
    );

    let mut child1 = Node::new();
    child1.set_style(
        Style::new()
            .set_width(StyleUnit::Point(80.0.into()))
            .set_height(StyleUnit::Point(36.0.into()))
    );
    root.insert_child(&mut child1, 0);

    // ... 更多子节点

    root.calculate_layout(800.0, 600.0, Direction::LTR);
    println!("root layout: {:?}", root.get_layout());
    println!("child1 layout: {:?}", child1.get_layout());
}
```

#### 验证点

| # | 验证点 | 检查方法 |
|---|--------|---------|
| 1 | yoga-rs 能否在 macOS 上编译 | `cargo build` |
| 2 | Flexbox Row 布局计算结果是否正确 | 输出坐标与预期对比 |
| 3 | 布局结果是否可翻译为 Taffy 兼容的坐标格式 | 代码路径确认 |
| 4 | 不支持 Grid 的能力缺口有多大 | 评估：哪些路线书中的布局需求依赖 Grid |

#### 通过标准

- macOS 编译通过
- Flexbox Row + Column 布局计算正确
- Grid 能力缺口分析完成（明确哪些组件不可用 Flexbox 模拟）

#### 注意事项

- Yoga 不支持 Grid 是已知限制。如果 Taffy 真的不可用，需要额外设计 Grid 的替代方案（自行实现或降级为 table-based 布局）
- 此验证的优先级最低，因为 Taffy 的维护风险很低（使用者网络极广）

---

## 5. 执行顺序与时间规划

### 5.1 主技术验证执行顺序

```
第 1-2 周              第 3-4 周              第 5-6 周             第 7 周+
──────────             ──────────             ──────────             ────────
V1 🔴                  V2 🔴                  V6 🟡                  V10 🟡
Vello+cosmic-text       CJK 渲染质量           Taffy+渲染             IME 集成
集成 POC                测试矩阵               坐标转换                POC
  │                      │                      │
  ├─ V4 🟡（并行）       ├─ AV2 🟡（必须）      ├─ V5 🟡（并行）
  │  三平台编译           │  Skia 替代验证       │  diff 基准
  │                      │                      │
V3 🔴（第 1 周启动，    │                      ├─ V7 🟡（并行）
  独立调研，不依赖       │                      │  快照基准
  其他验证项）           │                      │
                         │                   V9 🟢（随时完成）
```

### 5.2 替代技术验证执行顺序

AV2（Skia 替代 Vello）为**验证阶段必须执行项**，已纳入 §5.1 主时间线（第 3-4 周）。其余替代验证项仅当触发条件满足时才启动：

```
替代验证执行方式：

┌─ 方式 1：主验证失败时立即启动 ──────────────────────┐
│                                                       │
│  V1 失败 → AV1 立即启动（Parley+Fontique 替代）       │
│  V1+V2 失败 → AV1 优先，AV1 失败再 AV2（如 AV2 未执行）│
│  V3 失败 → AV3 立即启动（无障碍架构设计 + 原型）      │
│  V6 失败 → AV4 可延期启动（Yoga 替代）                │
│                                                       │
├─ 方式 2：依赖维护风险触发时启动 ──────────────────────┤
│                                                       │
│  持续监控关键依赖的维护状态（§5.5）。                 │
│  当任何依赖超过 12 个月无实质更新时，                │
│  触发对应的替代验证。                                 │
│                                                       │
└───────────────────────────────────────────────────────┘
```

**替代验证执行优先级**：

| 优先级 | 替代项 | 启动条件 | 预估工期 | 与主验证的关系 |
|--------|--------|---------|---------|--------------|
| 1 | AV2 (Skia) | **验证阶段必须执行** + Vello 维护风险触发 | 2-3 人天 | 提供渲染引擎 B 方案，提前就绪 |
| 2 | AV1 (Parley+Fontique) | V1 失败 | 2-3 人天 | 替代 V1 的结论 |
| 3 | AV3 (直接平台 API) | V3 发现致命缺口 | 1-2 人天 | 提供无障碍 B 方案 |
| 4 | AV4 (Yoga) | Taffy 维护风险触发 | 1-2 人天 | 提供布局 B 方案 |

### 5.3 里程碑（含替代路径）

| 时间 | 里程碑 | 判定 |
|------|--------|------|
| 第 2 周末 | V1 + V3 + V4 完成 | 🔴 决定主路线是否可以继续 |
| 第 4 周末 | V2 + V5 + V7 + AV2 完成 | 🟡 补充性能数据 + 渲染 B 方案就绪 |
| 第 4 周末 | 若 V1 失败，AV1 完成 | 🔴 确认文本替代方案可行 |
| 第 6 周末 | V6 完成 | 🟡 验证布局集成 |
| 第 8 周末 | V10 完成 | 🟢 IME 集成验证完成 |
| 按需 | AV3 / AV4（按触发条件） | 🟡 确认各替代方案可行 |

> **说明**：原 V8（Rust 快速重启延迟）已从验证项降级为基线测量（见 §5.5.1），不作为 Go/No-Go 判定条件。

### 5.4 替代技术验证触发矩阵

以下矩阵定义了每条主技术路线失败时，应启动哪个替代验证，以及替代方案的技术代价。

| 主技术 | 失败场景 | 替代技术 | 替代验证 | 切换代价 | 能力损失 |
|--------|---------|---------|---------|---------|---------|
| **Vello** | 停止维护 / 跨平台失败 | Skia (skia-safe) | AV2 | 高（3-6 个月） | GPU compute shader 优势丧失，渲染性能需重新评估 |
| **Vello** | 性能不足 | Skia 或 CPU 渲染路径 | AV2 | 中-高 | 视具体性能瓶颈而定 |
| **cosmic-text** | 停止维护 | Parley + Fontique | AV1 | 中（1-3 个月） | 需重写文本 pipeline 集成代码，但功能集基本等价 |
| **cosmic-text** | CJK/IME 缺陷 | Parley + Fontique 或系统文本 API | AV1 | 中（1-3 个月） | 同上 |
| **Vello + cosmic-text** | 组合无法协同 | ① Parley + Fontique + Vello（AV1，优先，保留 Vello）<br>② Skia + 自带文本（AV2，AV1 失败时） | AV1 → AV2 | 中 / 高 | ① 保留 Vello，仅换文本栈（与 Xilem 对齐）<br>② 全栈替换，丧失 compute shader 优势 |
| **AccessKit** | 停止维护 | 直接平台 API 桥接 | AV3 | 高（6-12 个月） | 需自行维护三平台适配代码 |
| **AccessKit** | RichText 不支持 | 直接平台 API（仅 RichText 部分） | AV3 | 中（3-6 个月） | 仅自建 RichText 的无障碍桥接，其余仍用 AccessKit |
| **Taffy** | 停止维护 | Yoga (yoga-rs) | AV4 | 中（1-3 个月） | **丧失 CSS Grid 支持**，仅保留 Flexbox |
| **wgpu** | 停止维护 | 直接后端（ash/glyphon）或 glow | — | 极高（12+ 个月） | 当前不做专项验证。wgpu 风险极低，且切换代价过大，应优先推动社区接手而非自行替代 |

### 5.5 依赖健康度持续监控

在验证阶段和后续开发过程中，应持续监控关键依赖的健康状态：

| 监控指标 | 检查频率 | 告警阈值 |
|---------|---------|---------|
| 最近一次 release 距今 | 每月 | > 6 个月 → 关注；> 12 个月 → 告警 |
| 最近一次 commit 距今 | 每月 | > 3 个月 → 关注；> 6 个月 → 告警 |
| 核心维护者人数 | 每季度 | < 3 人 → 关注；< 2 人 → 告警 |
| 已知使用者数量 | 每半年 | < 3 个活跃项目 → 关注 |
| 是否存在社区 fork | 每半年 | 存在活跃 fork → 正面信号 |
| 资金/赞助状态 | 每半年 | 资金中断 → 告警 |

监控结果记录在 `verify/dependency-health.md` 中，每季度更新一次。

### 5.5.1 基线测量：Rust 快速重启端到端延迟

> **定位说明**：此内容原为 V8 验证项，经审核后从验证项降级为基线测量。其输出是量化数据而非 Go/No-Go 判定，不影响验证结论决定树。

#### 要验证的假设

增量编译 + 新进程启动 + 状态恢复的总延迟可控制在 5 秒以内。

#### 测量方法

在当前阶段**只做测量基线**，不实现完整系统。等到阶段 1 框架雏形完成后再做端到端验证。

#### 测量项

| # | 测量项 | 方法 | 预期占比 |
|---|--------|------|---------|
| 1 | 增量编译延迟 | `cargo build --timings` 测量改动单个函数后的重编译时间 | 60-80% |
| 2 | 进程启动 + 窗口创建 | 测量从 `cargo run` 到第一个 winit Event 的时间 | 5-10% |
| 3 | 状态反序列化 | 使用 V7 的基准数据 | < 5% |
| 4 | 组件树重建 | 从序列化状态重建 widget 树的时间 | 5-15% |
| 5 | 首帧渲染 | 从组件树到第一帧画面的时间 | 5-10% |

```
总延迟 = T_compile + T_startup + T_restore + T_rebuild + T_frame
目标: ≤ 5s
```

#### 输出标准

- 各项子指标可测量
- 总延迟分析表明 5s 目标在合理范围内（各部分之和接近或低于 5s）
- 识别出最大瓶颈并记录

#### 注意事项

- 增量编译延迟受项目规模、依赖数量和硬件影响很大
- 路线书阶段 1 项目规模预计 < 10,000 行框架代码 + 应用代码
- 若当前无法测量真实项目规模，可用 iced 或 egui 的示例项目作为替代来估算
- 此基线测量的结果不影响 Go/No-Go 判定，仅作为阶段 1 开发的参考数据

### 5.6 验证结论决定树（含替代路径）

```
V1 + V3 + V4 全部通过？
  ├─ 是 → 主路线成立
  │       ↓
  │       继续 V2 + V5 + V6 + V7（+ AV2 已在第 3-4 周完成）
  │       ↓
  │       全部通过 → ✅ 启动阶段 0 开发
  │       部分未通过 → 有缓解方案 → ⚠️ 更新路线书后启动
  │       注意：基线测量（原 V8）不作为 Go/No-Go 判定条件
  │
  └─ 否 → 哪个失败？
      │
      ├─ V1 失败（Vello+cosmic-text 不可行）
      │   ├─ AV1 执行 →
      │   │   ├─ AV1 通过 → ✅ 改用 Parley+Fontique，其余不变
      │   │   └─ AV1 失败 →
      │   │       ├─ AV2 执行（如未在验证阶段执行）→
      │   │       │   ├─ AV2 通过 → ✅ 改用 Skia（含 Skia 文本），其余不变
      │   │       │   └─ AV2 失败 → 🔴 渲染管线技术路线需要重新设计
      │   │       └─ 触发技术评审
      │   │
      │   └─ 同时 V2 失败（文本质量不达标）
      │       → AV1 优先（Parley+Fontique 是最小变更方案）
      │       → AV1 失败再 AV2
      │
      ├─ V3 失败（AccessKit 能力不足）
      │   ├─ 缺口可接受 → ⚠️ 降级目标，记录缺口，继续推进
      │   └─ 缺口不可接受（如 RichText 无障碍为零）
      │       └─ AV3 执行 →
      │           ├─ AV3 通过 → ✅ 框架用 AccessibilityBackend trait，
      │           │              阶段 1 先用 AccessKit，阶段 2 自建 RichText 桥接
      │           └─ AV3 失败 → 🔴 无障碍路线需要重新设计
      │
      └─ V4 失败（某平台不可运行）
          ├─ 单平台失败 → ⚠️ 缩小初始平台范围，该平台延后支持
          └─ 多平台失败 → AV2 执行（如未在验证阶段执行）
              ├─ AV2 通过 → ✅ 改用 Skia 渲染后端
              └─ AV2 失败 → 🔴 渲染管线需要大幅调整
```

---

## 6. 验证代码结构建议

### 6.1 仓库组织

验证代码放在 `verify/` 目录下，不与框架主代码混合：

```
RUST-GUI/
├── docs/
│   ├── Rust GUI 框架技术路线书.md
│   └── Rust GUI 框架技术路线验证设计.md    ← 本文档
├── verify/
│   ├── README.md                           ← 验证总入口
│   ├── dependency-health.md                ← 依赖健康度监控记录
│   ├── v1-vello-cosmic/
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   ├── v2-cjk-quality/
│   │   └── (扩展 V1 的测试用例)
│   ├── v3-accesskit-gap/                   ← 调研文档，非代码
│   │   └── report.md
│   ├── v4-cross-platform/
│   │   ├── .github/workflows/
│   │   └── scripts/
│   ├── v5-diff-bench/
│   │   ├── Cargo.toml
│   │   ├── benches/
│   │   └── src/
│   ├── v6-taffy-layout/
│   │   └── (在 V1 基础上扩展)
│   ├── v7-snapshot-bench/
│   │   ├── Cargo.toml
│   │   └── benches/
│   ├── v10-ime/
│   │   └── (在 V1 基础上扩展)
│   ├── av1-parley-fontique/                ← 替代验证：文本
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   ├── av2-skia/                           ← 替代验证：渲染
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   ├── av3-platform-a11y/                  ← 替代验证：无障碍
│   │   ├── design.md                       # AccessibilityBackend trait 设计
│   │   └── src/main.rs                     # macOS NSAccessibility 原型
│   └── av4-yoga/                           ← 替代验证：布局
│       ├── Cargo.toml
│       └── src/main.rs
└── (后续阶段 0 框架代码)
```

### 6.2 共享基础 crate

若多个验证项需要相同的渲染管线代码，可提取共享 crate：

```
verify/
├── verify-common/
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs          # 窗口创建、wgpu 初始化
├── v1-vello-cosmic/        # 依赖 verify-common（Vello 渲染）
├── v2-cjk-quality/         # 依赖 verify-common
├── v4-cross-platform/      # 依赖 verify-common
├── v6-taffy-layout/        # 依赖 verify-common
├── v10-ime/                # 依赖 verify-common
├── av1-parley-fontique/    # 依赖 verify-common（Parley+Fontique 替代）
└── av2-skia/               # 独立（Skia 不使用 Vello）
```

AV2（Skia）不与 verify-common 共享渲染代码，因为 Skia 使用的是自己的渲染后端而非 Vello。AV1 和 AV4 可以与 verify-common 共享窗口创建和 wgpu 初始化代码。AV2 的窗口创建代码从 verify-common 的 winit 初始化部分复制（约 30 行），在 POC 阶段不提取共享 crate 以保持 AV2 的独立性。

### 6.2.1 验证代码生命周期

验证阶段结束后：

1. `verify/` 目录整体归档，在 Git 中打 tag：`verify-{日期}`
2. 验证结论（Go/No-Go）记录在 `verify/README.md` 中
3. 如果结论为 Go，`verify-common/` 的窗口/wgpu 初始化代码可作为阶段 0 渲染管线的参考实现，但不应直接作为框架代码的基础
4. 验证代码不进入框架主仓库的 `main` 分支历史——使用独立的 `verify` 分支或在合并前 squash
5. 依赖健康度监控记录（`verify/dependency-health.md`）迁移到 `docs/` 目录下，作为持续维护的文档

---

### 6.3 验证报告模板

每个验证项完成后，产出简短的验证报告：

```markdown
# 验证报告：V{N} - {标题}

**日期**：YYYY-MM-DD
**执行人**：
**环境**：OS / Rust 版本 / 关键依赖版本

## 验证结果：通过 / 未通过 / 通过但有限制

## 关键数据
（性能数字、截图对比等）

## 发现的问题
（列出所有未通过的验证点）

## 对路线的影响
- [ ] 无影响，按原计划推进
- [ ] 需要调整实现方案（描述）
- [ ] 需要调整技术选型（描述）
```

---

## 7. 验证结论判定标准

### 7.1 主路线全部通过

所有验证项（至少 V1-V7）均通过 → 路线书的技术方案可行，可以启动阶段 0 开发。

### 7.2 主路线部分通过但有限制

部分验证项未通过，但有明确的缓解方案 → 更新路线书中的风险与缓解部分，在缓解措施到位后启动开发。

### 7.3 主路线失败，替代路线通过

V1/V3/V4 中某项判定为不可行，但对应的替代验证（AV1/AV2/AV3/AV4）通过 → 更新路线书中的技术选型，将失败的主技术替换为通过验证的替代技术。在替代技术集成到框架设计后，启动阶段 0 开发。

**替代路线判定示例**：

| 失败项 | 替代通过 | 结论 |
|--------|---------|------|
| V1 失败 | AV1 通过 | ✅ 使用 Parley+Fontique 替代 cosmic-text，其余不变 |
| V1+AV1 均失败 | AV2 通过 | ⚠️ 使用 Skia 替代 Vello+cosmic-text，渲染架构变更较大 |
| V3 致命缺口 | AV3 通过 | ✅ 使用 AccessibilityBackend trait 架构，先 AccessKit 后自建 |
| V4 多平台失败 | AV2 通过 | ⚠️ 使用 Skia 获得更广泛的平台兼容性，但丧失 compute shader 优势 |

### 7.4 主路线和替代路线均失败

V1/V3 及其对应的替代验证（AV1/AV2/AV3）均判定为不可行 → 需要召开技术评审，重新评估路线书的技术方向。可能需要考虑：

- 是否放弃自研渲染，使用 WebView（类似 Tauri 模式）
- 是否放弃 Rust 自绘，使用 Flutter/Dart FFI 混合方案
- 是否缩小目标范围（如仅支持单平台）

### 7.5 验证终止条件（主路线）

如果满足以下所有条件，则终止本次验证，认为主技术路线成立：

- V1-V4 全部通过
- V5-V7 的量化基准在目标范围内（或超标但有明确的优化路径）
- V3 报告中不存在「未知」状态的红色区域

### 7.6 验证终止条件（含替代路线）

如果满足以下条件之一，则终止本次验证，可启动阶段 0 开发（路线可能已调整）：

- 满足 §7.5 的主路线终止条件
- 或：主路线某项失败，但对应替代验证通过，且替代路线的能力损失在可接受范围内（参见 §5.4 触发矩阵中的「能力损失」列）

### 7.7 验证后交付物

验证阶段完成后，应产出以下交付物：

1. 各验证项（V1-V10）的验证报告（按 §6.3 模板）
2. 已执行的替代验证（AV1-AV4）的验证报告
3. `verify/dependency-health.md` —— 依赖健康度初始评估
4. 更新后的《Rust GUI 框架技术路线书》（如果技术选型有变更）
5. 阶段性结论文档，包含明确的「Go / No-Go / Go-with-changes」判定

---

> **下一步：** 本验证设计经评审确认后，按 §5.1 执行顺序启动 `verify/v1-vello-cosmic/` 的 POC 代码编写。同时建立 §5.5 的依赖健康度监控，按月度检查关键依赖状态。
