# rgui 样式系统与 rgss 设计（D4）

> 版本：0.1.0
> 工作目录：`/Users/chenchao/Documents/code/rust/RUST-GUI`
> 写实原则：本文与当前代码实际一致（`rgui-core/src/style/`）。**当前仅为骨架占位**——样式解析/主题/热重载未实现（P1），如实标注。

---

## 1. 定位

`.rgss` 样式系统由 `rgui-style` 并入 `rgui-core::style`（greenfield §F）。`.rgss` 解析是纯文本解析（非重型运行时）；并入 core 后走单一路径（热重载 = P1）。

- **当前实现状态**：**仅占位骨架**，不引入 cssparser / notify。无 `hot_reload` / `parser` / `theme` 子模块（样式解析/主题/热重载在 D4+ 补全，属 P1）。

---

## 2. 核心类型（`rgui-core/src/style/mod.rs`）

```rust
pub struct StyleSheet { pub rules: Vec<StyleRule> }   // 样式表
pub struct StyleRule  { pub selector: String }        // 样式规则（选择器串占位）
pub fn parse_rgss(_src: &str) -> StyleSheet { StyleSheet::default() }
```

| 类型 | 状态 |
|------|------|
| `StyleSheet { rules }` | 骨架（rules 列表） |
| `StyleRule { selector }` | 规则（仅 selector 占位） |
| `parse_rgss(_src) -> StyleSheet` | **占位**：返回默认空 `StyleSheet`，不做真实解析 |

模块级 `#![allow(dead_code)]`（类型为占位定义，避免未使用告警噪音）。

---

## 3. 当前实现状态（D10，写实）

1. **未实现（P1）**：.rgss 真实解析（需 cssparser）、主题系统、热重载——当前均无，属"有意缺失/待 P1"。
2. **已提供**：仅 `StyleSheet`/`StyleRule` 类型骨架 + `parse_rgss` 占位签名。
3. **单一路径**：样式仅占位；`rgui-devtools`/双热重载已删（有意缺失）。

---

## 4. 与依赖关系

- `parse_rgss` 返回 `StyleSheet`（非 `Result<..>`），且不依赖 cssparser/notify。
- 样式并入 core 零 GPU/平台（契约防火墙），无重型依赖。
