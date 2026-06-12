# Rust GUI 框架技术路线书

> **文档目标：** 定义一个面向桌面优先、可扩展到移动端与 Web 的 Rust GUI 框架技术方案。
> 本文基于历史案例、Rust 语言约束和目标需求，推导架构边界、子系统划分和实施顺序。

> **文档定位：** 本文是技术方向与架构边界文档，用于说明目标能力、约束条件、阶段性取舍和当前主路线。
> 本文不把高风险问题表述为已解决实现，只定义当前具备依据的设计方向。

---

## 目录

1. [需求定义：目标能力与验收标准](#1-需求定义目标能力与验收标准)
2. [历史分析：现有框架的失败模式](#2-历史分析现有框架的失败模式)
3. [Rust 的约束与机会](#3-rust-的约束与机会)
4. [总架构设计](#4-总架构设计)
5. [关键子系统设计](#5-关键子系统设计)
    - [5.1 渲染管线](#51-渲染管线)
    - [5.2 组件模型](#52-组件模型)
    - [5.3 状态管理](#53-状态管理)
    - [5.4 开发反馈与热重载系统](#54-开发反馈与热重载系统)
    - [5.5 无障碍系统](#55-无障碍系统)
    - [5.6 布局引擎](#56-布局引擎)
    - [5.7 跨平台视觉一致性实现](#57-跨平台视觉一致性实现)
6. [完整数据流](#6-完整数据流)
7. [开发路线图](#7-开发路线图)
8. [与现有方案的差异化对比](#8-与现有方案的差异化对比)
9. [关键风险与缓解措施](#9-关键风险与缓解措施)
10. [技术方向可行性论证](#10-技术方向可行性论证)
    - [10.1 渲染与窗口系统](#101-渲染与窗口系统)
    - [10.2 文本系统](#102-文本系统)
    - [10.3 布局系统](#103-布局系统)
    - [10.4 无障碍](#104-无障碍)
    - [10.5 声明式架构与 diff](#105-声明式架构与-diff)
    - [10.6 开发反馈系统](#106-开发反馈系统)
    - [10.7 移动端与 Web](#107-移动端与-web)
11. [结论](#11-结论)

---

## 1. 需求定义：目标能力与验收标准

### 必达目标

| # | 需求 | 量化标准 |
|---|------|---------|
| H1 | **极速上手** | `cargo generate` 一键创建项目，10 分钟内运行一个包含路由、表单、列表的业务示例应用 |
| H2 | **迭代循环 < 5s** | `.rgss/.rgui/脚本` 变更保存到出图 ≤ 1 秒；Rust 业务逻辑变更的“保存到恢复运行”目标 ≤ 5 秒 |
| H3 | **跨平台一致** | Windows/macOS/Linux 三平台行为一致、布局一致、视觉高度一致；核心组件在基准字体与主题下截图差异可控 |
| H4 | **三类复杂组件** | DataGrid（虚拟滚动、排序、编辑）、Form（验证、联动）、RichText（选区、IME、Undo/Redo） |
| H5 | **WCAG 2.1 AA** | 自动生成 Accessibility Tree，屏幕阅读器可用，键盘导航完整 |
| H6 | **可持续社区** | ≥500 GitHub stars，≥1 个全职维护者，有机构或基金会背书 |
| H7 | **文档完备** | 官方教程覆盖「CRUD 应用」从头到尾的开发流程，新手可独立完成 |

### 扩展目标

| # | 需求 |
|---|------|
| I1 | 支持 iOS + Android 移动端 |
| I2 | WebAssembly 输出可用于真实生产应用 |

> **说明：** H1-H5/H7 主要由技术架构与组件能力承载；H6 属于项目治理与生态建设目标，
> 需要在路线图后期通过维护者机制、组件兼容政策、RFC 流程、赞助/基金会支持共同兑现，
> 不是仅靠运行时设计就能自动达成。
>
> **关于路由：** H1 中的「路由」在 MVP 阶段指基础页面/Tab 切换能力，可通过组件层的
> 条件渲染实现；完整的路由/导航框架（深度链接、嵌套路由、URL 参数解析）按 §7 路线图
> 安排在阶段 3 交付。

---

## 2. 历史分析：现有框架的失败模式

分析范围：Rust GUI 框架（Druid、OrbTk、Azul、Relm、SixtyFPS→Slint）+ 非 Rust 框架作为对照组（React、Flutter、Qt、SwiftUI）。

### 2.1 四类共性失败原因

#### 失败原因 1：渲染策略选择不当

```
渲染策略光谱：
  原生控件 ←————————————————————————————→ 完全自绘
  (Cocoa/WinUI/GTK)                    (像素级控制)

  走左边 → 跨平台不一致 + 绑定维护复杂度高
  走右边 → 需要自行实现文本/IME/无障碍
  走中间 → 两边的问题都有
```

| 框架 | 渲染策略 | 后果 |
|------|---------|------|
| **Relm** | 绑定 GTK | Linux 上可用，但 macOS/Windows 上界面风格与平台原生体验差异较大 |
| **Azul** | Firefox WebRender | 渲染引擎与框架耦合太深，WebRender 不再以独立库形态维护后项目失去基础依赖 |
| **OrbTk** | 自绘（SDL2 时代） | 文本靠 harfbuzz 手工绑定，IME 从未正常工作 |
| **Druid** | piet（自绘抽象层） | piet 与框架同步开发，资源分裂，两者都未成熟 |

**结论：** 如果目标是统一渲染结果、组件能力和平台行为，框架需要采用自绘路线，并在早期同时投入文本、IME 和无障碍能力。

---

#### 失败原因 2：状态管理选择不当

这是 Rust GUI **特有**的问题。借用检查器与 UI 图状引用关系的冲突，迫使每个框架做出某种架构取舍：

| 框架 | 状态管理方案 | 取舍 | 后果 |
|------|------------|------|---------|
| **Druid** | Lens 模式 | 强制使用 getter/setter 宏操作嵌套数据 | 复杂 UI 中 lens 链不可维护，类型错误信息无法理解 |
| **OrbTk** | 实体 ID + 全局存储 | 放弃编译期类型安全 | Widget ID 对不上时运行时崩溃 |
| **Relm** | GTK 信号 + 消息 | 接受 C 风格的回调模型 | 内存安全问题通过 unsafe 绕过 |
| **SixtyFPS** | 属性绑定 DSL | 把状态管理移出 Rust 类型系统 | DSL 与 Rust 之间总是有缝，复杂逻辑写两遍 |

**结论：** 单纯绕开借用检查器会把复杂度转移到运行时或 DSL。更合适的方向是基于所有权模型做差分更新，而不是构造近似 GC 的共享可变结构。

---

#### 失败原因 3：团队规模与目标范围不匹配

| 框架 | 核心团队 | 野心 | 结果 |
|------|---------|------|------|
| Druid | Raph Levien + ~3 贡献者 | 桌面 GUI 框架 + 渲染引擎 | 两者都未完成 |
| OrbTk | 1 人 + 少量 PR | 桌面 + 移动 + Web | 所有平台都不可用 |
| Azul | ~3 人 | 全功能桌面框架 | 维护负担超出能力 |
| **Slint**（仍在维护） | 公司 (~10 人) | 嵌入式 + 桌面 | 商业支撑与范围控制共同作用 |
| **iced**（仍在维护） | System76 (~5 人) | 桌面 GUI | 有公司投入且范围相对收敛 |

**结论：** GUI 框架通常需要 5-10 名全职工程师、至少 3 年持续开发，以及明确的长期资金来源。

---

#### 失败原因 4：组件生态的冷启动问题

```
没组件 → 写应用需要自己造一切 → 没人写应用
    ↑                                   ↓
没人用 ← 没有应用证明可用性 ← 没有成功案例
```

**已有框架的处理方式：**

| 框架 | 破局策略 |
|------|---------|
| **React** | 先在 Facebook 内部大规模使用，再开源 |
| **Flutter** | Google 全职团队 + Material Design 组件库自带 |
| **Qt** | 商业公司 20 年持续投入 + 工业客户买单 |
| **SwiftUI** | Apple 自带全部平台组件，开发者只需组合 |

**结论：** 如果框架目标是支撑真实业务应用，仅提供绘制能力不足以形成采用，至少需要覆盖 DataGrid、Form、RichText 这三类复杂组件。

---

### 2.2 成功框架的共性特征

| 特征 | React | Flutter | Qt | SwiftUI |
|------|-------|---------|-----|---------|
| **渲染** | 借浏览器 | 自绘 (Skia) | 自绘 + 可选原生 | 原生控件封装 |
| **语言** | JS（动态）| Dart（GC）| C++（无 GC）| Swift（ARC）|
| **声明式** | ✅ JSX | ✅ Widget 树 | ❌ 命令式 → QML 声明式 | ✅ |
| **热重载** | ✅ Fast Refresh | ✅ Hot Reload | ✅ QML Live Preview | ✅ Previews |
| **组件库** | 极丰富 | Material Design | 极丰富 | 完全自带 |
| **无障碍** | 靠浏览器 | ✅ 自建 | ✅ 自建 | ✅ 系统自带 |
| **公司** | Meta | Google | Qt Group | Apple |
| **从零到可用** | ~2 年 | ~3 年 | ~5 年 | ~2 年 |

**可归纳的共性：**
1. 渲染管线自绘或借用已有的成熟基础设施
2. 声明式是主流方向
3. 热重载或等价的快速反馈机制已成为主流框架的基础能力
4. 无障碍从 day 1 设计
5. 都有大公司/基金会支撑

---

### 2.3 Rust GUI 中常见的过度静态化问题

除了上述共性问题，Rust GUI 设计中还经常出现另一类问题：**过度强调静态建模与零运行时抽象。**

框架作者试图用 Rust 类型系统完全静态建模 GUI，而 GUI 本质上是动态的、有状态的、
需要运行时组织和工程取舍的。具体表现：
- 过度泛型化（每个 widget 都带类型参数，层层嵌套后类型签名无法阅读）
- 试图在类型层面消除所有运行时检查（最终 API 无法使用）
- 拒绝任何形式的运行时反射（但 GUI 天然需要一定灵活性）
- 追求「每个像素的绘制都经过借用检查」的洁癖

**结论：** GUI 层可以在安全 Rust 的前提下使用动态数据结构、状态索引、缓存和运行时调度。
这不是接受不安全代码，而是承认 GUI 作为动态系统需要运行时组织；需要严格压缩的是渲染热点路径上的额外抽象开销。
Rust 的安全保证应优先用于状态变更的可预测性、所有权边界和并发安全，而不是追求完全排除运行时结构。

→ 对策见 §3.3 原则 6 及 §5.2-5.3 的设计实现。

---

## 3. Rust 的约束与机会

### 3.1 不可改变的约束

| 约束 | 影响 | 不可绕过？ |
|------|------|-----------|
| **无稳定 ABI** | 无法用 `dlopen` 加载不同 Rust 版本编译的代码 | ✅ 不可绕过 |
| **借用检查器** | 无法直接表达 UI 图状的可变引用关系 | ✅ 不可绕过 |
| **编译速度** | 大型项目全量编译可能数十分钟 | ✅ 不可绕过（只能缓解）|
| **无运行时反射** | 无法在运行时动态调用方法或创建类型实例 | ✅ 不可绕过 |
| **学习曲线** | 上手比 JS/Python/Dart 慢 | ✅ 不可绕过 |

### 3.2 可以转化为优势的特性

| 特性 | 如何转化为优势 |
|------|--------------|
| **所有权 + 借用** | 状态变更因果链完全可追踪——天然适合时间旅行调试和状态快照 |
| **代数类型 (enum)** | 事件系统可用封闭集合表达，编译器强制处理所有情况 |
| **零成本抽象** | 渲染管线、布局引擎可达 C++ 级别性能 |
| **Send + Sync** | 多窗口、多线程 UI 可在类型系统约束下实现线程安全；其他框架通常依赖主线程约束或约定 |
| **trait 系统** | 组件接口用 trait 定义——类型安全的插件系统 |
| **Cargo + crates.io** | 组件分发和依赖管理天然解决 |

### 3.3 设计原则

```
原则 1: 不与借用检查器对抗，与之合作
        → 差分更新（diff-and-patch），而非共享可变引用

原则 2: 开发反馈不能被 Rust 全量编译限制
        → 「引擎」（Rust，预编译，不变）+「声明式 UI 资源」（即时重载）
        → Rust 代码默认走“保留状态的快速重启”，动态热替换只是实验增强项

原则 3: 自绘，但不从零开始
        → 组合成熟组件：cosmic-text（文本）+ wgpu（GPU 抽象）+ Vello（矢量渲染）

原则 4: 无障碍从 day 1 设计
        → Accessibility Tree 是独立数据结构，在布局阶段同步更新

原则 5: 框架提供第一方复杂组件
        → DataGrid、Form、RichText 作为第一方组件发布

原则 6: 动态数据与运行时组织准则
        → 布局计算、事件分发、样式匹配、脚本层可以安全地使用动态数据结构和运行时调度
        → 渲染热点内层循环避免动态分发；Scene 生成与 GPU 提交路径保持可批处理、可内联
```

---

## 4. 总架构设计

### 4.1 分层架构

```
┌─────────────────────────────────────────────────────────────┐
│                    应用层 (Application)                      │
│  ┌──────────────────────────┐  ┌──────────────────────────┐  │
│  │ .rgui 结构 + .rgss 样式   │  │ Rust 业务逻辑 + 自定义组件 │  │
│  │ (热重载 < 1s)            │  │ (快速重启 / 实验热替换)    │  │
│  └──────────────────────────┘  └──────────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    框架运行时 (Framework Runtime)             │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐  │
│  │ 组件树   │ │ 状态存储  │ │ 样式引擎  │ │  布局引擎     │  │
│  │ (差分)   │ │ (Flat)   │ │ (CSS-like)│ │  (Taffy)     │  │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────┘  │
│  ┌──────────┐ ┌──────────┐ ┌──────────────────────────┐   │
│  │ 事件路由  │ │ 无障碍树  │ │ 脚本运行时 (可选 Rhai)    │   │
│  └──────────┘ └──────────┘ └──────────────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│                    渲染引擎 (Render Engine)                   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐  │
│  │ 场景图   │ │ 矢量绘制  │ │ 文本渲染  │ │ 合成器       │  │
│  │ (Scene)  │ │ (Vello)  │ │ (cosmic) │ │ (wgpu)      │  │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    平台抽象层 (Platform)                      │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐  │
│  │ 窗口     │ │ 输入事件  │ │ IME      │ │ 无障碍桥接    │  │
│  │ (winit)  │ │ (winit)  │ │ (自建)   │ │ (accesskit)  │  │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 六项设计决策

#### 决策 1：三速反馈通道 —— 解决 H2（开发体验）

本设计将开发反馈拆分为不同速度层，而不是把所有变化都压到 Rust 编译链路上。

```
第 1 通道：资源热重载（.rgss / 图片 / 主题）
    加载方式: 运行时解析 + 即时应用
    变更频率: 最高（颜色、间距、视觉）
    目标延迟: < 200ms

第 2 通道：声明式结构热重载（.rgui）
    加载方式: 运行时解析 -> WidgetView -> diff -> patch
    变更频率: 高（布局、组合、消息绑定）
    目标延迟: < 1s

第 3 通道：Rust 逻辑反馈（.rs）
    基线路径: 增量编译 + 保留状态的快速重启
    实验路径: 受限场景的动态热替换
    变更频率: 较低（业务逻辑、自定义组件）
    目标延迟: 2-5s
```

**实现要求：** 本设计把反馈速度作为目标约束，但不把不稳定的 ABI 假设写成架构前提。实现路径如下：
1. 把高频变更尽量移到声明式资源层；
2. 把 Rust 代码变更的默认体验设计成“快编译 + 保留状态恢复”；
3. 仅把动态热替换视为额外增强，而不是框架成立的前提。

#### 决策 1.1：`.rgui` 的能力边界

`.rgui` 不是第二种编程语言，也不是带任意闭包的迷你 Rust。它只负责**声明 UI 结构**：

```
允许：
- widget 树结构
- 属性字面量
- 样式类名、主题变量引用
- 条件显示 / 列表渲染（受限声明式形式）
- 事件绑定到“消息名”或“命令名”

不允许：
- 任意 Rust 闭包
- 持久状态定义
- 直接调用平台 API
- 复杂业务逻辑
```

`.rgui` 的职责是描述“界面长什么样、发什么消息”，不是“消息来了以后具体做什么”。
业务逻辑仍在 Rust；需要频繁调整的轻量交互可选下沉到脚本层。

---

#### 决策 2：扁平状态存储 —— 应对借用检查器

```
传统方案（树形状态）：       本方案（扁平存储）：

Widget Tree                 StateStore（扁平 Arena）
├─ App 状态                  states[widget_1] = AppState {...}
│  ├─ Sidebar 状态            states[widget_2] = SidebarState {...}
│  │  ├─ MenuItem 状态        states[widget_3] = MenuState {...}
│  └─ Content 状态            states[widget_5] = ContentState {...}
│     └─ DataGrid 状态        states[widget_6] = GridState {...}

每个节点「拥有」自己状态      框架「拥有」所有状态
子节点访问父状态需 lens/消息  Widget 通过唯一 ID 访问自己状态
                              框架保证同一时刻只有一个 writer
```

**设计结果：**
1. 每个 widget 的状态互不冲突，借用检查器友好
2. 状态变更 → 自动标记受影响 widget → 仅它们重新渲染
3. 只有“持久业务状态”参与快照，因而可以做时间旅行调试
4. 不同 widget 的状态可以并发读取（`RwLock`）
5. Widget A 影响 Widget B？发事件到框架，框架排队处理——不需要互相持有引用

状态模型分为三类：

- **持久状态（Persistent State）**：业务数据，参与 diff、订阅和快照，可跨快速重启保留。
- **实例态（Instance State）**：焦点、悬浮、测量结果、节点句柄等，由运行时拥有，不对应用暴露。
- **缓存态（Render/Layout Cache）**：字形缓存、布局缓存、GPU 资源句柄，不参与快照，失效后可重建。

---

#### 决策 3：CSS 风格的样式系统

```
代码配置样式的约束：              类 CSS 样式表的特性：

Button::new()                     /* styles.rgss —— 热重载即时生效 */
  .background(Color::BLUE)        .data-grid {
  .corner_radius(8)                 background: var(--surface);
  .padding(12, 24)                  border-radius: 8px;
                                    /* 改颜色 → 保存 → 即刻看见 */
问题：                             }
1. 每次调整样式需重编译
2. 样式与逻辑耦合                  @media (width < 768px) {
3. 无法做主题切换（或非常笨拙）      .data-grid { font-size: 14px; }
4. 无法做响应式                    }
5. 设计师无法参与
                                   优势：
                                   1. 改样式无需编译（< 200ms）
                                   2. 关注点分离
                                   3. CSS 自定义属性做主题变量
                                   4. 响应式设计天然支持
                                   5. 熟悉 Web 样式语法的开发者可直接复用已有知识
```

---

#### 决策 4：组合成熟组件做渲染栈

不自己造渲染引擎：

```
图层：
┌────────────────────────────────────────┐
│            窗口表面 (winit)             │
├────────────────────────────────────────┤
│         wgpu (GPU 抽象层)               │
│    Metal (macOS) / Vulkan (Linux)      │
│    / DX12 (Windows) / WebGPU (WASM)    │
├────────────────────────────────────────┤
│         合成器                          │
├──────────────┬─────────────────────────┤
│  Vello       │  cosmic-text            │
│  矢量图形    │  文本渲染               │
│  · 路径填充  │  · 字形光栅化           │
│  · 渐变      │  · 双向文本 (Bidi)      │
│  · 图像      │  · Emoji                │
│  · 混合模式  │  · CJK 支持             │
└──────────────┴─────────────────────────┘
```

| 组件 | 理由 |
|------|------|
| **winit** | Rust 生态中使用广泛的窗口与输入库，可复用已有平台适配能力 |
| **wgpu** | Rust 社区标准 GPU 抽象，为桌面/移动提供统一后端，也为 WASM/WebGPU 保留演进路径 |
| **Vello** | GPU 计算着色器矢量渲染，适合高复杂度 2D 场景与批处理提交 |
| **cosmic-text** | 已被 iced 和 COSMIC DE 验证，文本渲染成熟 |

移动端（I1）：底层图形路径成立，但触摸、软键盘、安全区域仍需单独工程化
WebAssembly（I2）：保留 `wgpu` + WebGPU 的长期路径，是否达成真实生产可用以后续阶段验证为准

---

#### 决策 5：使用 accesskit 做无障碍

[accesskit](https://github.com/AccessKit/accesskit) 已被 egui、Slint 等项目使用，
可作为当前 Rust 生态中的无障碍桥接库：

```
框架的无障碍树              accesskit               平台 API

Widget Tree                NodeId → Role            macOS: NSAccessibility
├─ Window (window)         NodeId → Name            Windows: UI Automation  
├─ Toolbar (toolbar)       NodeId → Value           Linux: AT-SPI / D-Bus
├─ Button (button)         NodeId → Bounds
└─ DataGrid (table)        NodeId → Children
    ├─ Row (row)           NodeId → Actions
    └─ Cell (cell)
```

框架在布局计算后遍历 widget 树，生成 accesskit Node 并推送 TreeUpdate。
accesskit 负责平台 API 桥接。

WCAG 2.1 AA 还需要（框架内置）：焦点管理、键盘导航（Tab/方向键/Enter/Escape）、
高对比度主题、屏幕阅读器测试。

---

#### 决策 6：Rhai 作为可选脚本层

选择 [Rhai](https://rhai.rs/) 的理由：语法接近 Rust、嵌入式无外部依赖、
编译时类型检查脚本中的类型使用、与 Rust 双向互操作设计良好。

```rust
// Rust 端：注册类型和函数
engine.register_type::<DataGridState>();
engine.register_fn("sort_by", |s: &mut DataGridState, col: &str| {
    s.sort_by = col.to_string();
});

// Rhai 脚本端：UI 交互逻辑（热重载 < 100ms）
fn on_search(text) {
    if text.len() >= 2 {
        grid_state.filter_by(text);
    }
}
```

脚本层是可选的——简单应用可以全部用 Rust，
需要频繁调整交互逻辑时可以写在脚本中，实现**即时热重载**。

---

## 5. 关键子系统设计

### 5.1 渲染管线

#### 场景图（Widget 树 → 绘制指令）

```
Widget Tree (逻辑结构)         Scene Graph (绘制指令)

VBox                          Layer 1 (背景)
├─ HBox                       ├─ FillRect { bounds, color }
│  ├─ Icon                    ├─ DrawGlyphs { text, font, pos }
│  └─ TextField               └─ DrawPath { path, paint }
└─ DataGrid                   Layer 2 (内容)
   └─ row × N                 ├─ ClipRect { bounds }
                              │  ├─ DrawGlyphs { ... }
                              │  └─ DrawPath { border }
                              └─ Layer 3 (滚动条)
                                 └─ FillRect { ... }
```

仅 dirty widget 重新生成场景图片段。框架维护 `DirtyRegion` 列表，只重绘变化部分。

#### 帧循环

```rust
// 每帧的固定执行顺序（保证确定性）
fn tick(&mut self) {
    self.dispatch_events();     // 1. 事件分发 → 可能修改状态
    self.layout_dirty();        // 2. 仅 dirty widget 重新布局
    self.update_accessibility(); // 3. 无障碍树同步
    let scene = self.generate_scene(); // 4. 仅 dirty widget 重绘
    self.render(scene);         // 5. GPU 提交（Vello → wgpu）
}
```

目标：全流程 < 17ms（120fps 一帧预算）。

---

### 5.2 组件模型

```rust
/// 第三方组件实现的是“组件规范”，而不是自己持有整棵可变状态树
pub trait WidgetSpec: Send + Sync + 'static {
    type State: PersistState;
    type Message: AppMessage;

    fn name(&self) -> &'static str;

    /// 从持久状态派生声明式视图，应保持纯函数语义
    fn view(&self, state: &Self::State, ctx: &ViewContext) -> WidgetView<Self::Message>;

    /// 处理来自 UI 的消息，只能修改持久状态
    fn update(&self, msg: Self::Message, state: &mut Self::State, ctx: &mut UpdateContext);

    /// 纯测量：不允许偷偷写业务状态
    fn measure(&self, state: &Self::State, bc: SizeConstraints, ctx: &MeasureContext) -> Size;

    /// 绘制时可使用运行时缓存，但缓存由框架持有
    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext);

    fn accessibility(&self, state: &Self::State, ctx: &AccessContext) -> AccessibilityNode;
}
```

```rust
/// Rust 端的声明式视图 DSL
fn app(state: &AppState) -> WidgetView<Message> {
    ui! {
        <VBox spacing=12 padding=16 class="page">
            <HBox spacing=8>
                <TextField placeholder="搜索..."
                    value=state.search_text.as_str()
                    on_change=Message::SearchChanged />
            </HBox>
            <DataGrid columns=&state.columns rows=&state.rows
                virtual_scroll=true editable=true
                on_sort=Message::SortBy />
            <HBox spacing=8 justify=End>
                <Button variant=Primary on_click=Message::Save>
                    "保存"
                </Button>
                <Button variant=Secondary on_click=Message::Cancel>
                    "取消"
                </Button>
            </HBox>
        </VBox>
    }
}
```

**第三方组件：** 第三方 crate 发布的是 `WidgetSpec + PersistState + 消息类型 + 样式元数据` 的组合契约。
框架统一持有实例态和缓存态，因此第三方组件天然兼容 diff、快照、无障碍与快速重启。

---

### 5.3 状态管理（关键子系统）

#### 扁平状态存储 + 差分更新

```rust
pub struct StateStore {
    persistent: HashMap<WidgetId, Box<dyn PersistState>>, // 可快照业务状态
    instance: HashMap<WidgetId, RuntimeNodeState>,        // 焦点/命中测试/节点句柄
    caches: HashMap<WidgetId, RenderLayoutCache>,         // 布局/字形/GPU 缓存
    dirty: HashSet<WidgetId>,                             // 需重新计算的 widget
    subscriptions: HashMap<WidgetId, Vec<Subscription>>,  // 依赖关系
}

impl<'a> StoreAccess<'a> {
    pub fn state<T: PersistState>(&self) -> &T { /* 只读自己业务状态 */ }
    pub fn read<T: PersistState>(&self, target: WidgetId) -> &T {
        /* 读别人业务状态，自动建立订阅 */
    }
}

impl<'a> StoreAccessMut<'a> {
    pub fn state_mut<T: PersistState>(&mut self) -> &mut T {
        /* 写自己业务状态，自动标记 dirty */
    }
}
```

```rust
/// 只有满足快照契约的业务状态才能进入 persistent 区
pub trait PersistState: erased_serde::Serialize + Send + Sync + 'static {
    /// 用于快照/热重启恢复的稳定 schema 名称
    fn schema_name(&self) -> &'static str;

    /// 业务状态版本号；结构变更时递增
    fn schema_version(&self) -> u32;
}
```

快照系统只序列化 `PersistState`，并按 `schema_name + schema_version` 做迁移。
这保证“快速重启恢复状态”和“时间旅行调试”建立在显式契约之上，而不是隐含假设之上。

#### 差分更新流程

```
状态变更
  → 标记修改者 + 所有订阅者 dirty
  → 仅对 dirty widget 重新执行 view()（纯函数）
  → 生成新 WidgetView（轻量值类型）
  → 与旧 WidgetView 做结构化 diff：
      类型相同 → 更新属性
      类型不同 → 替换节点
      列表有 key → 按 key 做增删移（React reconciliation）
      列表无 key → 最小编辑距离
  → patch 到 Widget Tree
```

#### 时间旅行调试（内置，但只针对持久状态）

```
持久状态快照自动记录：
v1(初始) → v2(输入) → v3(排序) → v4(选中) → v5(编辑) → v6(当前)

开发者可跳转到任意历史业务状态、播放/暂停变更序列、
导出状态快照用于 bug 报告、在测试中回放序列做回归测试。
```

**实现含义：** `WidgetView` 是值类型，`view()` 只从持久状态派生视图。
实例态与缓存态由框架集中持有，因此：
1. diff 不会被组件私有可变引用破坏；
2. 快速重启时只恢复持久状态即可；
3. 时间旅行调试不必序列化 GPU/平台句柄。

---

### 5.4 开发反馈与热重载系统

开发反馈系统的设计目标是缩短保存到界面更新的时间，但“Rust 动态热替换”不是唯一技术路径。
本节定义的主路线建立在 Rust ABI 约束之上。

```
第 1 层：样式热重载 (.rgss)
  延迟: < 200ms
  方式: 重新解析 → 更新样式缓存 → 标记受影响节点 dirty → 重绘

第 2 层：结构热重载 (.rgui)
  延迟: < 1s
  方式: 重新解析结构 → 生成 WidgetView → diff → patch → 布局 + 重绘

第 3 层：脚本热重载 (.rhai，可选)
  延迟: < 500ms
  方式: 重新编译脚本函数 → 替换命令处理器 → 保留业务状态

第 4 层：Rust 逻辑反馈 (.rs)
  延迟目标: 2-5s
  基线机制: 增量编译 -> 启动新进程/新逻辑实例 -> 恢复持久状态 -> 回到当前界面
  实验机制: 受限的动态热替换，仅用于开发模式，不作为架构前提
```

#### 主路线：快速重启 + 状态恢复

- 系统约束：Rust 无稳定 ABI，因此 `dylib` 热替换不作为框架成立前提。
- 设计目标：优先保持业务状态、路由、焦点路径和滚动位置。
- 工程含义：当上述状态可恢复时，快速重启可覆盖多数开发反馈场景。

#### Rust 反馈层的设计要求

- 应用状态应可序列化为版本化快照。
- 组件树应由快照、路由和初始化参数重建。
- 快速重启失败时，系统应降级为普通重启，并保留失败前快照用于诊断。
- 若未来验证了某类组件可安全热替换，可把它作为 dev-only 增强项接入，但不得反向污染主架构。

---

### 5.5 无障碍系统

```rust
// DataGrid 根据持久状态和当前可见区生成无障碍节点
impl WidgetSpec for DataGrid {
    fn accessibility(&self, state: &GridState, ctx: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::new()
            .role(Role::Table)
            .name("数据表格")
            .description(format!("共 {} 行，{} 列", state.row_count, state.col_count))
            .add_action(Action::ScrollByPage)
            .add_action(Action::SortByColumn)
            .children(ctx.visible_rows().enumerate().map(|(i, row)| {
                AccessibilityNode::new()
                    .role(Role::Row)
                    .name(format!("第 {} 行", i + 1))
                    .children(row.cells().map(|cell| {
                        AccessibilityNode::new()
                            .role(Role::Cell)
                            .name(cell.column_name())
                            .value(cell.display_text())
                    }))
            }))
    }
}
```

框架内置：焦点指示器、键盘导航、颜色对比度警告（样式系统）、系统 DPI 缩放响应。

---

### 5.6 布局引擎

选择 [Taffy](https://github.com/DioxusLabs/taffy)：
- Rust 原生实现，零 FFI 开销
- 实现 CSS Flexbox + CSS Grid——Web 开发者已熟悉的模型
- 已被 Dioxus 在生产中验证
- 缓存：仅尺寸约束变更或子 widget 变化时才重算

```
/* 样式与布局的对应关系 */
.container {
    display: flex;            → Taffy Flexbox
    flex-direction: column;
    gap: 12px;                → 子元素间距
    padding: 16px;            → 内边距
}
.content {
    flex: 1;                  → 占据剩余空间
    overflow: scroll;         → 溢出滚动
}
@media (max-width: 768px) {   → 响应式断点
    .container { flex-direction: column; }
}
```

---

### 5.7 跨平台视觉一致性实现

H3 要求「三平台行为一致、布局一致、视觉高度一致；核心组件在基准字体与主题下截图差异可控」。
这需要四个具体实现要素。

#### 5.7.1 字体嵌入打包

跨平台像素差异的最大来源不是渲染引擎，而是**字体**。不同操作系统自带字体不同，即使同一字体的不同版本，字形度量、hinting 策略也可能不同。

实现方案如下：

```
字体策略：

1. 默认模式——框架内置基准字体
   ↓
   框架二进制内嵌 Noto Sans CJK（中日韩）+ Inter（拉丁/西里尔）
   所有组件默认使用内置字体渲染，不依赖系统字体
   cosmic-text 的 fontdb::Source::Binary 直接支持此模式

2. 应用可选模式——应用打包自定义字体
   ↓
   应用通过 .rgss 主题配置声明字体路径
   框架在初始化时加载应用自带字体，加入 FontSystem

3. 系统可选模式——显式选择系统字体
   ↓
   仅在应用明确配置时才使用系统字体
   不作为默认行为
```

`cosmic-text` 的 `FontSystem` 可同时管理多个字体源，因此三种模式可以共存。关键原则是：**默认路径不依赖系统，保证三平台视觉起点一致**。

#### 5.7.2 色彩空间管理

不同 GPU 和显示器对色彩的解释可能不同。为了控制视觉差异，框架需要显式管理色彩空间。

```
色彩空间约定：

内核色彩空间：sRGB 输入 + linear-sRGB 混合
    ↓
`.rgss` 与主题系统中的颜色字面量都以 sRGB 表达
进入渲染管线后，在需要渐变/混合/插值时统一转换到 linear-sRGB 计算
最终输出到 `*_Srgb` surface，由 GPU/平台完成标准 sRGB 编码

wgpu surface 配置：
    ↓
创建 swapchain 时显式指定：
  - format: Bgra8UnormSrgb 或 Rgba8UnormSrgb
  - present_mode: Fifo（保证 vsync 行为一致）
确保 GPU 输出与操作系统合成器之间色彩空间匹配

着色与混合约定：
    ↓
框架统一约束：颜色输入是 sRGB，混合/插值在线性空间完成
避免不同后端在隐式色彩转换上的行为漂移

高对比度主题：
    ↓
WCAG 2.1 AA 要求的对比度计算基于 sRGB 相对亮度公式
样式系统在编译时检查颜色对比度是否达标
```

#### 5.7.3 截图回归 CI 体系

仅有渲染与布局机制不足以验证跨平台一致性，还需要自动化验证体系。

```
CI 工作流：

                 ┌──────────┐
                 │  CI 触发  │
                 │ (每次 PR) │
                 └────┬─────┘
        ┌─────────────┼─────────────┐
        ▼             ▼             ▼
   ┌─────────┐  ┌─────────┐  ┌─────────┐
   │  macOS   │  │ Windows │  │  Linux  │
   │  Metal   │  │  DX12   │  │  Vulkan │
   └────┬─────┘  └────┬─────┘  └────┬─────┘
        │              │              │
        ▼              ▼              ▼
   生成截图 A    生成截图 B    生成截图 C
        │              │              │
        └──────────────┼──────────────┘
                       ▼
              ┌─────────────────┐
              │  像素 diff 引擎  │
              │  与基准截图对比   │
              └────────┬────────┘
                       ▼
              差异率 ≤ 阈值？
            ┌───Yes──┐──No───┐
            ▼        │       ▼
         通过       │    CI 失败
                    │   (需人工审查)
```

**截图基线采用两层结构：**

- **跨平台参考基线：** 固定字体、固定 DPI、固定窗口尺寸下，由 Linux/Vulkan 生成一组参考截图，用于监控跨平台视觉漂移趋势。
- **平台专属 Golden：** macOS/Metal、Windows/DX12、Linux/Vulkan 各自维护 golden，避免驱动与后端差异导致误报。

CI 中同时执行两类检查：平台内回归不得退化；跨平台对比不得超过容差阈值。

**测试场景：** 覆盖全部核心组件（Text、Button、TextField、Image、Container、DataGrid、Form、RichText），每个组件在亮/暗主题、多种尺寸和 DPI 下各取一组截图；CI 通过离屏渲染 runner 在三后端生成可重复截图。

**容差策略：**

| 对比方式 | 阈值 | 说明 |
|---------|------|------|
| 逐像素严格相等 | 不采用 | GPU 浮点精度差异会导致 1-2 像素偏移 |
| 像素匹配率 | ≥ 99.5% | 允许极少数抗锯齿边缘像素存在差异 |
| 色差阈值 | ΔE ≤ 1.0 | 超过此阈值视为视觉可感知差异 |

#### 5.7.4 GPU 浮点精度差异

Vello 使用 GPU compute shader 做路径填充和混合运算。同一段着色器在不同 GPU（NVIDIA/AMD/Apple Silicon/Intel）上可能因浮点精度差异产出不同的亚像素结果。

这不是 bug，是硬件物理差异。框架的策略不是消除它，而是**控制它**：

```
策略：

1. 抗锯齿一致性
   → Vello 使用 analytic AA（解析抗锯齿），而非硬件 MSAA
   → 这取决于计算着色器而非光栅化器，差异范围更可控

2. 渲染回退比较
   → CI 中 Linux 走 Vulkan，macOS 走 Metal，Windows 走 DX12
   → 差异集中在抗锯齿边缘 1-2 像素，不会出现大规模偏移

3. 容差而非严格匹配
   → 不使用逐像素 == 比较
   → 使用像素匹配率 ≥ 99.5% + 最大色差 ΔE ≤ 1.0 的双重标准

4. 首错定位
   → diff 失败时，CI 自动生成差异热力图
   → 开发者可直观定位是哪个组件、哪个平台的哪个区域出现了偏差
```

以上四个要素共同构成了 H3「跨平台视觉一致」的实现方案。它们从阶段 0 开始建设，在阶段 1 形成完整的自动化验证闭环。

---

## 6. 完整数据流

### 示例：用户点击「保存」按钮

```
1. 平台事件捕获 → winit → Event::MouseDown
2. 命中测试 → WidgetId = "save_btn_42"
3. 事件分发 → `WidgetSpec::update()` →
   `store.state_mut::<AppState>().status = Status::Saving`
4. 脏标记传播 →
   "save_btn_42" dirty + 所有订阅 AppState.status 的 widget dirty
5. 视图重建 → 仅 dirty widget 执行 view() → 新 WidgetView → diff → patch
6. 布局重算 → 仅受影响子树用 Taffy 重算
7. 无障碍更新 → 生成 accesskit TreeUpdate → 推送平台 API
8. 场景图生成 → 仅 dirty widget 执行 paint() → 合并到 Scene
9. GPU 渲染 → Scene 提交 Vello → wgpu → 呈现到屏幕

每帧预算：
  步骤 1-3: < 1ms
  步骤 4-6: < 5ms
  步骤 7:   < 1ms
  步骤 8:   < 2ms
  步骤 9:   < 8ms
  总计:     < 17ms（120fps 内）
```

---

## 7. 开发路线图

### 阶段 0：基础设施建设（0-6 个月）
```
团队: 3-5 人全职 + 公司/基金会
□ wgpu + Vello + cosmic-text 渲染管线                    ← 前置: 无
□ 5 个基本 widget: Text、Button、TextField、Image、Container ← 前置: 渲染管线
□ winit 窗口和输入事件                                    ← 前置: 无
□ 验证三平台可编译运行                                     ← 前置: 渲染管线 + winit
□ 确定基准字体许可证与打包策略                               ← 前置: 无
□ 建立截图回归、离屏渲染 runner、输入/焦点测试基建            ← 前置: 渲染管线 + 基本 widget
□ 建立 H2/H3 基准 harness（保存到首帧、跨平台截图 diff）      ← 前置: 截图回归基建

里程碑: 窗口显示 "Hello World" + 可点击按钮
```

### 阶段 1：核心框架（6-18 个月）
```
团队: 5-8 人
□ WidgetSpec + 持久状态/实例态/缓存态三分模型
□ 状态存储（扁平 Arena）+ 差分更新 + 版本化快照协议
□ Taffy 布局集成（Flexbox + Grid）
□ `.rgui` 结构语言 + `.rgss` 样式系统 + 热重载
□ 事件路由 + 基本无障碍集成
□ 状态迁移器（schema 版本升级）与失败回退机制
□ H2/H3/H5 自动化验收规范 v1
□ 开发者文档 v1

里程碑: TODO 应用（表单 + 列表 + 增删改）
```

### 阶段 2：关键组件与快速反馈（18-30 个月）
```
团队: 8-12 人
□ DataGrid（虚拟滚动 10 万行 60fps、排序、筛选、行选择、单元格编辑）
□ Form（声明式校验、跨字段联动、异步校验、错误展示）
□ RichText（选区、光标、Undo/Redo、IME、基础富文本格式）
□ 三层快速反馈（样式 / 结构 / Rust 快速重启）
□ 可选脚本层（Rhai）用于高频交互逻辑
□ WCAG 2.1 AA 合规（焦点、键盘导航、屏幕阅读器）
□ NVDA / VoiceOver / Orca 屏幕阅读器测试矩阵
□ CLI 工具 (cargo rgui new my-app)

里程碑: 管理后台 CRUD 应用，满足 H1/H2/H4/H5/H7
```

### 阶段 3：生态 + 移动端（30-42 个月）
```
团队: 10-15 人
□ 图表、日期选择器、下拉选择器、对话框、Toast
□ 路由/导航框架
□ iOS 适配（Metal、触摸、安全区域、输入法）
□ Android 适配（Vulkan、触摸、返回键）
□ 组件发布标准 + 5+ 完整示例应用
□ RFC 流程、兼容性政策、组件稳定性分级
□ 维护者/赞助者机制初版，开始兑现 H6 的治理要求

里程碑: 社区可贡献组件，移动端可运行，形成治理雏形
```

### 阶段 4：桌面 1.0 + Web Preview（42-54 个月）
```
团队: 10-15 人
□ WebAssembly Preview（WebGPU 后端 + wasm-opt 优化 + 能力子集）
□ 性能优化（1000+ widget 场景、内存、启动时间）
□ 安全审计 + 桌面 1.0 正式发布
□ 文档站、组件目录、长期维护/赞助结构定型

里程碑: 桌面端达到 1.0；移动端进入 Beta；Web 提供 Preview 验证 I2 路径
```

---

## 8. 与现有方案的差异化对比

| 维度 | 本设计 | iced | egui | Slint | Tauri | Xilem |
|------|--------|------|------|-------|-------|-------|
| **渲染** | Vello + cosmic | cosmic + wgpu | 自绘 | FemtoVG | WebView | Vello |
| **状态管理** | 扁平存储 + 差分 | Elm 架构 | 无状态 (即时) | 属性绑定 DSL | Web 前端自管 | 差分 |
| **开发反馈** | 资源/结构热重载 + Rust 快速重启 | ❌ | ❌ | ✅ DSL | ✅ Web HMR | ❌ |
| **样式** | CSS-like | 代码配置 | 代码配置 | DSL | CSS | ❌ |
| **脚本层** | Rhai（可选）| ❌ | ❌ | 自带 DSL | JS (Web) | ❌ |
| **无障碍** | accesskit (Day1) | 实验 | 🔴 基础 | ✅ | 靠浏览器 | ❌ |
| **DataGrid** | 第一方 | ❌ | ❌ | ❌ | 靠 Web | ❌ |
| **Form** | 第一方 | ❌ | ❌ | ❌ | 靠 Web | ❌ |
| **RichText** | 第一方 | ❌ | ❌ | ❌ | 靠 Web | ❌ |
| **移动端** | 阶段 3 目标（iOS/Android） | ❌ | 实验 | 嵌入式 | Beta | ❌ |
| **WASM** | 阶段 4 Preview（能力子集） | ✅ | ✅ | 演示 | ❌ | ❌ |
| **时间旅行调试** | ✅ 内置 | ❌ | ❌ | ❌ | ❌ | ❌ |

> **图例：** ✅ 生产可用 · 🔴 基础/实验性支持 · ❌ 不支持或不适用。评价基于各框架 2026 年中公开文档与社区状态。

---

## 9. 关键风险与缓解措施

| 风险 | 影响 | 概率 | 缓解 |
|------|------|------|------|
| **Vello 不成熟** | 渲染管线不可用 | 中 | 保留 Skia 作为 fallback 后端；渲染接口先抽象成 Scene API |
| **Rust 动态热替换不稳定** | 开发体验下降 | 中 | 主路线本来就采用快速重启 + 状态恢复；动态替换只作为 dev-only 实验特性 |
| **团队/资金不足** | 项目在阶段 2 前中止 | 高 | Day1 寻找公司/基金会；范围严格控制在 MVP；阶段 3 前建立赞助与维护者机制 |
| **组件生态不形成** | 有框架没人用 | 中 | 自带关键组件降低准入门槛 |
| **状态模型复杂度失控** | 核心运行时返工 | 中 | 从一开始区分持久状态/实例态/缓存态；所有高级能力都基于此模型建设 |
| **WASM 性能** | I2 不达标 | 中 | WebGPU 优先；必要时降级功能集，不承诺与桌面端完全等价 |
| **跨平台文本/IME 差异** | H3/H5 不达标 | 中高 | 基准字体打包、输入法测试矩阵、文本回归测试纳入阶段 0-2 |

---

## 10. 技术选型依据与可行性论证

> **目的：** 基于 2025-2026 年的公开资料和社区实践，说明本设计采用相关技术栈的依据、适用范围和阶段约束。
> 本章不把高风险问题表述为已解决实现，只说明当前选型判断和工程边界。

---

### 10.1 渲染与窗口系统

#### wgpu — GPU 抽象层

`wgpu` 是 Rust 图形生态中使用广泛的 GPU 抽象库。官方说明它是跨平台、纯 Rust 的图形 API，原生运行在 Vulkan、Metal、D3D12、OpenGL 上，并在 wasm 上支持 WebGL2 和 WebGPU。它也是 Firefox、Servo、Deno 的 WebGPU 实现核心。

> **引用：** [wgpu GitHub](https://github.com/gfx-rs/wgpu) · [wgpu docs.rs](https://docs.rs/crate/wgpu/latest)

**选型结论：** `wgpu` 作为统一 GPU 抽象层，适用于桌面优先路线，并为移动端与 Web 保留后续扩展空间。

#### winit — 平台窗口与输入抽象

`winit` 的功能范围明确覆盖桌面（Windows、macOS、Unix）、移动端（iOS、Android）和 Web。更新日志显示其持续修复 IME 与事件循环问题，并已加入 Android 软键盘支持和基础 iOS IME 支持。

> **引用：** [winit FEATURES](https://docs.rs/crate/winit/latest/source/FEATURES.md) · [winit changelog](https://docs.rs/winit/latest/winit/changelog/index.html)

**选型结论：** `winit` 适合作为平台窗口与输入抽象层。该库只覆盖窗口、事件和基础平台交互，因此在本设计中仅承担最底层平台职责。

#### Vello — GPU 2D 自绘

`Vello` 定位为 GPU compute-centric 2D 渲染器，建立在 `wgpu` 之上。目标环境是“所有支持 WebGPU 默认限制的环境”。但 crate 说明也明确指出：Web 端因依赖 compute shaders 和 WebGPU，浏览器支持仍在演进。

> **引用：** [vello crate](https://crates.io/crates/vello/0.8.0)

**选型结论：** Vello 适合作为桌面优先阶段的 2D 自绘实现。Web 端支持仍受浏览器 WebGPU 与 compute shader 条件限制，因此 Web 路线安排在后期阶段。

---

### 10.2 文本系统

#### cosmic-text — 文本处理

`cosmic-text` 提供 shaping、font discovery、font fallback、layout、rasterization、editing 等核心能力，支持 bidirectional text、ligatures、color emoji 和字体回退。其官方说明写明了 Linux、macOS、Windows 三平台具备完整功能集。它被 `iced` 直接集成，也被 COSMIC 桌面应用栈实际使用。

> **引用：** [cosmic-text crate](https://crates.io/crates/cosmic-text/0.17.2) · [cosmic-text API docs](https://pop-os.github.io/cosmic-text/cosmic_text/) · [iced text backend](https://github.com/iced-rs/iced/blob/master/graphics/src/text.rs)

**选型结论：** `cosmic-text` 可作为文本 shaping、layout 和 raster 的基础库。本设计据此不自研底层文本系统。

**工程约束：** RichText 编辑器级能力仍由框架自行实现；`cosmic-text` 只提供底层文本能力，不直接提供完整富文本编辑器组件。

---

### 10.3 布局系统

#### Taffy — 布局引擎

`Taffy` 直接实现了 CSS Block、Flexbox 和 CSS Grid 布局算法。其使用者包括 Servo、Bevy、Slint、Lapce/Floem、Zed/GPUI。这说明“Rust GUI 使用 CSS 风格布局模型”已经有多条技术路线的实际部署。

> **引用：** [taffy crate](https://crates.io/crates/taffy/0.10.1-experimental-cache-fix.1)

**选型结论：** `Taffy` 可作为布局引擎实现，并与 `.rgss` / CSS-like 样式系统保持语义一致。熟悉 Web 布局模型的开发者可直接复用 Flexbox 和 Grid 的语义。

---

### 10.4 无障碍

#### AccessKit — 无障碍桥接

AccessKit 官网直接写明：它就是为“自绘 UI 工具包”的屏幕阅读器和辅助技术支持而设计。官方列出的集成项目包括 Bevy、egui、Freya、Slint、Xilem。Slint 在其 `winit` 后端中也有完整的 `accesskit_winit` 接入代码。

> **引用：** [AccessKit 官网](https://accesskit.dev/) · [Slint accesskit 后端代码](https://github.com/slint-ui/slint/blob/master/internal/backends/winit/accesskit.rs)

**选型结论：** AccessKit 适合作为无障碍桥接层。本设计仍需自行实现焦点系统、语义树生成、控件级语义正确性和测试矩阵。

---

### 10.5 声明式架构与 diff

#### Xilem — 声明式视图与 diff 参考实现

Xilem 的架构文档明确写出了与本设计高度一致的技术路线：

- 每次状态变化后，调用用户提供的纯函数生成**轻量 view tree**；
- 将新 view tree 与旧 view tree 做比较；
- 根据差异更新 **retained element tree**（Masonry widget tree 或 DOM）。

它指出该架构受 React、Elm 和 SwiftUI 启发，并且把“静态类型赋能高效 diff”作为核心性能论点，与本设计中“WidgetView 是值类型、diff 在 Rust 值类型上做”的论述高度吻合。

> **引用：** [Xilem ARCHITECTURE.md](https://github.com/linebender/xilem/blob/main/xilem/ARCHITECTURE.md)

**选型结论：** Xilem 提供了“声明式视图 + diff-and-patch + retained runtime”路线的现实参照，可作为本设计高层架构的参考实现。

---

### 10.6 开发反馈系统

#### 动态热替换的工程边界

2026 年关于 Rust 动态热重载的实践文章明确指出：

- `dylib` 热替换本质上是**开发期技巧**，不适合作为通用插件或运行时边界方案；
- Rust 没有稳定 ABI，该做法在一般意义上并不可靠；
- 之所以能工作，是因为 host 和 worker 使用完全相同的编译器与编译参数——这适合于开发工具，但不应作为架构基石。

> **引用：** [I hotreload Rust and so can you](https://kampffrosch94.github.io/posts/hotreloading_rust/) （访问日期: 2026-06-12）

#### `xilem_core` 提供的主路线参考

`xilem_core` 文档明确提到了热重载的未来方向：

- **app process**：持有应用状态，可快速重编译、重启；
- **display process**：长生命周期，接收新的 view tree 来更新界面。

这种“双进程 + 状态恢复”的架构，与本设计修订后的“Rust 快速重启 + 恢复持久状态”路线完全一致。

> **引用：** [xilem_core hot reloading section](https://docs.rs/xilem_core/0.4.0/xilem_core/)

#### Dioxus 提供的资源热重载参考

Dioxus 的开发工具宣称：
- 标记/样式编辑可在毫秒级反馈；
- 启动命令 `dx serve --hotpatch` 可实验性更新 Rust 代码，但被标注为 experimental。

> **引用：** [dioxus-devtools](https://lib.rs/crates/dioxus-devtools)

**选型结论：**

- 资源和结构的即时热重载已有公开实现可供参考。
- Rust 代码变更的反馈主路线定义为“快速重启 + 状态恢复”。
- 动态热替换只作为开发模式下的实验增强项，不反向约束主架构。

---

### 10.7 移动端与 Web

#### 移动端

- `winit` 已支持 Android / iOS；
- `winit` 更新日志显示 Android 软键盘与基础 iOS IME 支持正在实际演进；
- `wgpu` 的平台覆盖为 Metal / Vulkan，因此移动端底层图形不构成原则性障碍。

> **引用：** [winit FEATURES](https://docs.rs/crate/winit/latest/source/FEATURES.md) · [winit changelog](https://docs.rs/winit/latest/winit/changelog/index.html)

**选型结论：** 移动端支持保留为后续阶段目标。实施顺序定义为桌面运行时稳定后再推进 iOS 和 Android 适配。

#### Web

- `wgpu` 支持 wasm 上的 WebGL2 / WebGPU；
- `winit` 支持 `wasm32-unknown-unknown`，窗口以 `<canvas>` 表示；
- 但 `Vello` 官方对 Web 的表述非常谨慎——指出浏览器 WebGPU 支持仍在演进。

> **引用：** [wgpu docs.rs](https://docs.rs/crate/wgpu/latest) · [winit crate](https://crates.io/crates/winit/0.29.15) · [vello crate](https://crates.io/crates/vello/0.8.0)

**选型结论：** Web 路线保留为长期方向。当前阶段不将 Web 生产可用性作为 1.0 架构成立条件。

---

## 11. 结论

### 11.1 架构结论

1. 本文采用自绘路线，并以 `winit + wgpu + Vello + cosmic-text + Taffy + accesskit` 作为基础设施组合。
2. 本文采用“声明式视图 + diff-and-patch + retained runtime + 扁平状态存储”作为高层架构。
3. 本文将开发反馈拆分为资源热重载、结构热重载、脚本热重载和 Rust 快速重启四层，不把动态热替换作为主路线前提。
4. 本文将业务状态、实例态和缓存态分层管理，并以版本化快照支撑快速重启、调试与测试。
5. 本文把 DataGrid、Form、RichText、无障碍和截图回归测试基建纳入早期阶段，而不是作为后续补充项。
6. 本文按桌面优先、移动端随后、Web 最后的顺序安排实施阶段。

### 11.2 执行条件

1. 该设计在技术上可实施，但实施前提是持续的工程投入，而不是单点技术突破。
2. 实施规模预计需要 10-15 名全职工程师和 4-5 年持续开发。
3. 项目需要稳定的资金来源、长期维护者机制和兼容性治理流程。
4. 在实施阶段，仍需继续补充 RichText、快速重启与状态迁移、第三方组件协议等专题设计文档。
