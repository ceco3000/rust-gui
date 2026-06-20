# 审核判决书：设计文档体系 D0-D13

**审核日期：** 2026-06-20
**文档类型：** `设计`
**审核目标：** 验证 14 份设计文档间的跨文档一致性、与代码实现的吻合度、以及文档内部的逻辑自洽

---

## 核心主张

1. D0 为约束边界，D1-D7 为子系统详细设计，D8 为任务分解，D12 为进度跟踪
2. 所有 trait 签名、crate 依赖、模块边界以 D0 为权威
3. D8/D12 之间的任务 ID 必须一一对应

---

## 辩护方观点

**立场：文档体系整体自洽，核心接口（WidgetSpec trait、crate 拓扑）跨文档一致。**

| # | 辩护理由 | 证据 |
|---|---------|------|
| 1 | D0 §3.2 WidgetSpec trait 签名与 D1 §2.1 完全一致 | 证据：D0 第 174-217 行的 6 个方法签名，与 D1 第 126-209 行一一匹配（参数名、类型、返回类型均一致） |
| 2 | D8 §9.16b 的 RS01-RS07 任务与 D12 跟踪行 ID 一致，依赖关系已对齐 | 证据：上次审核已修复编号冲突，D7/D8/D12 三文档 7 个任务 ID 统一 |
| 3 | 15 个示例均使用 .rgui + .rhai 格式，与 D7 §10.2 设计一致 | 证据：`examples/*/ui.rgui` 和 `examples/*/handlers.rhai` 全部存在 |

---

## 起诉方观点

**立场：发现 4 处不一致——2 处过时标记、1 处遗漏、1 处文档内歧义。**

| # | 质疑 | 证据 |
|---|------|------|
| 1 | D0 两处标注 `rgui-script（阶段 2 预留）`，但该 crate 已实现（RH01-RH05 ✅） | 证据：D0 第 117 行和第 886 行均写"阶段 2 预留"；D12 第 298-302 行 RH01-RH05 全部 ✅；workspace Cargo.toml 第 27 行含 `rgui-script`；git log 显示 `1f87db5` 已实现 |
| 2 | D11 的 workspace members 列表中缺少 `rgui-script` | 证据：D11 第 73-77 行列出了 11 个成员（rgui-core 到 examples/*），不含 rgui-script；但实际 `Cargo.toml` 第 27 行包含它 |
| 3 | D0 crate 依赖图中未显示 `rgui-devtools → rgui-script` 依赖 | 证据：D0 第 60-93 行的 ASCII 图中 rgui-devtools 箭头仅指向 rgui-state 和 rgui-style；D7 第 496 行明确写 `rgui-devtools → rgui-script (已添加)` |
| 4 | CLAUDE.md 写 `rgui-script ← Rhai 脚本绑定（阶段 2 预留）` | 证据：CLAUDE.md 第 118 行；实际已实现 |

---

## 证据核查表

| # | 来源方 | 证据描述 | 核查结果 | 核查方法 |
|---|--------|---------|:--:|------|
| 1 | 起诉方 | D0 第 117 行 "阶段 2 预留" | ✅ 核实 | `read_file` D0 第 117 行 |
| 2 | 起诉方 | D12 RH01 ✅ + workspace Cargo.toml 含 rgui-script | ✅ 核实 | `search_files` Cargo.toml + D12 |
| 3 | 起诉方 | D11 第 73 行不含 rgui-script | ✅ 核实 | `read_file` D11 第 73-77 行 |
| 4 | 起诉方 | D0 ASCII 图无 rgui-devtools→rgui-script | ✅ 核实 | `read_file` D0 第 60-93 行 |
| 5 | 起诉方 | D7 第 496 行写 rgui-devtools→rgui-script | ✅ 核实 | `read_file` D7 第 496 行 |
| 6 | 起诉方 | CLAUDE.md 第 118 行 "阶段 2 预留" | ✅ 核实 | `read_file` CLAUDE.md 第 118 行 |
| 7 | 辩护方 | D0 vs D1 WidgetSpec 签名一致 | ✅ 核实 | 两侧逐行对比，6 方法匹配 |
| 8 | 辩护方 | 15 示例均有 ui.rgui + handlers.rhai | ✅ 核实 | `find examples -name "*.rgui"` 15 个 |

---

## 逐标准审核

| 标准 | 投票 | 发现 |
|------|:--:|------|
| U1 目标范围清晰性 | ✅ | 每份文档有明确的前置阅读和定位声明 |
| U2 上下文需求对齐 | ✅ | D0-D13 分层清晰，从架构到实施 |
| U3 事实依据可靠 | 🟡 | rgui-script 状态标记与实际代码不一致（2 处 + CLAUDE.md） |
| U4 逻辑一致性 | 🟡 | D11 缺少 rgui-script 但 D8/D12 引用它 |
| U5 完整性 | 🟡 | D0 依赖图缺 rgui-devtools→rgui-script |
| U6 风险边界 | ✅ | 每份文档均有边界说明 |
| U7 可验证性 | ✅ | D8 验收标准可度量，D12 可跟踪 |
| U8 表达清晰度 | 🟢 | D13 §3.2 vs §4 的"容器"指代不够精确 |

---

## 严重度表

| 等级 | 位置 | 问题 |
|:--:|------|------|
| 🟡 S2 | D0 第 117、886 行 | `rgui-script（阶段 2 预留）` 标记过时 |
| 🟡 S2 | CLAUDE.md 第 118 行 | 同上 |
| 🟡 S2 | D11 第 73 行 | workspace members 缺少 rgui-script |
| 🟡 S2 | D0 第 60 行 | crate 依赖图缺 rgui-devtools→rgui-script 边 |
| 🟢 S3 | D13 第 103 行 | "容器组件"指代不清（WA翻译容器 vs 纯布局容器） |

---

## 判决：✅ 批准（轻微修改后）

**理由：** 核心接口（WidgetSpec、crate 拓扑）跨文档一致。发现的 4 个问题是 rgui-script 实现后未同步更新文档的遗留——均为过时标记，不影响代码正确性。

## 必须修复项

1. D0 第 117、886 行：`rgui-script（阶段 2 预留）` → `rgui-script（✅ 已实现）`
2. CLAUDE.md 第 118 行：同上
3. D11 第 73 行：workspace members 添加 `"rgui-script"`
4. D0 crate 图：rgui-devtools 下方增加 → rgui-script 的依赖线

## 建议后续行动

- D13 第 103 行：将"容器组件"明确为"WA 翻译的装饰容器（Card、Divider 等）"，与 §4 纯布局组件区分
- 建立 CI 检查：设计文档中的 `阶段 N 预留` 标记与 D12 状态自动比对
