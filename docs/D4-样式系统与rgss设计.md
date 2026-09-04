# rgui 样式系统与 rgss 设计（D4）

> 版本：0.1.0
> 工作目录：`/Users/chenchao/Documents/code/rust/RUST-GUI`
> 写实原则：本文与当前代码实际一致（`rgui-core/src/style/`）。**D19 样式系统基础已实现**——`StyleProperties`/`StyleRule`/`StyleSheet`（`lookup` + 程序化 `rule` 构建）/**`default_theme`** 默认主题已可用；组件配色/描边从样式表驱动（`ViewContext.styles`）。`.rgss` 文本解析仍留 P1（D19 如实标注，不引入 cssparser）。

---

## 1. 定位

`.rgss` 样式系统由 `rgui-style` 并入 `rgui-core::style`（greenfield §F）。`.rgss` 解析是纯文本解析（非重型运行时）；并入 core 后走单一路径（热重载 = P1）。

- **当前实现状态（D19）**：样式系统基础（`StyleProperties`/`StyleSheet::lookup`/`default_theme`）已实现并驱动组件配色；`.rgss` 文本解析 / 主题切换 / 热重载留 P1（D19 不引入 cssparser / notify）。无 `hot_reload` / `parser` / `theme` 子模块（解析/主题/热重载在 P1 补全，如实标注）。

---

## 2. 核心类型（`rgui-core/src/style/mod.rs`）

```rust
pub struct StyleSheet { pub rules: Vec<StyleRule> }        // 样式表
pub struct StyleRule  { selector, properties: StyleProperties }  // 规则（selector + 属性）
pub struct StyleProperties { color, background, border_color, border_width, border_pad } // 样式属性（全部 Option<..>）
pub fn default_theme() -> StyleSheet                        // 默认主题（组件默认配色的权威来源）
pub fn default_style() -> &'static StyleSheet               // 默认样式表单例（OnceLock）
pub fn parse_rgss(_src) -> StyleSheet                       // 文本解析（P1 占位，不引入 cssparser）
impl StyleSheet { fn rule(sel, props) -> Self; fn lookup(sel) -> StyleProperties }  // 程序化构建 + 命中
```

| 类型 | 状态（D19 写实） |
|------|------|
| `StyleSheet { rules }` | 已实现：`rule` 程序化构建 + `lookup` 命中首条匹配规则 |
| `StyleRule { selector, properties }` | 已实现：selector + `StyleProperties` |
| `StyleProperties` | 已实现：`color/background/border_color/border_width/border_pad`（`None` = 未指定，组件回退默认） |
| `default_theme()` | 已实现：accordion/wa_badge 默认配色（当前各组件默认色的权威来源） |
| `default_style()` | 已实现：`&'static StyleSheet` 单例（供 `ViewContext.styles`） |
| `parse_rgss(_src) -> StyleSheet` | **占位（P1）**：返回默认主题，不做 `.rgss` 文本解析（不引入 cssparser） |

组件配色/描边经 `ViewContext.styles.lookup(selector)` 驱动（D19）：命中用样式、未命中回退默认。`WidgetView.border` 含 `pad`（D16 P2 参数化，描边外扩非硬编码 2.0）。

模块级 `#![allow(dead_code)]`（类型为占位定义，避免未使用告警噪音）。

---

## 3. 当前实现状态（D19，写实）

1. **已实现（D19）**：`StyleRule`/`StyleSheet` 基础——`StyleProperties`（color/background/border_color/border_width/border_pad）、`StyleSheet::rule` 程序化构建 + `lookup` 命中、`default_theme` 默认主题、`default_style` 单例；组件（Accordion/WaBadge）配色/描边从 `ViewContext.styles` 样式驱动（命中用样式、未命中回退默认）；`WidgetView.border` 含 `pad` 参数化（D16 P2）。
2. **未实现（P1）**：`.rgss` 文本真实解析（需 cssparser，D19 不引入）、主题系统（切换运行期主题）、热重载——如实标注。
3. **单一路径**：样式并入 core 零 GPU/平台（契约防火墙），无重型依赖。

---

## 4. 与依赖关系

- `parse_rgss` 返回 `StyleSheet`（非 `Result<..>`），且不依赖 cssparser/notify。
- 样式并入 core 零 GPU/平台（契约防火墙），无重型依赖。
