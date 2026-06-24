# D4：样式系统与 .rgss 设计

> **文档定位：** 本文档是《Rust GUI 框架总体设计》（D0）的子系统详细设计，定义 `.rgss` 样式语言语法、CSS 属性映射、选择器引擎、主题变量系统和样式热重载机制。

> **前置阅读：** [Rust GUI 框架总体设计](./Rust%20GUI%20框架总体设计.md)（D0）§4.7（rgui-style 公共 API）、[D1 组件模型与 WidgetSpec 设计](./D1-组件模型与WidgetSpec设计.md)（PropValue 类型、ViewContext）。

> **对应路线书章节：** §5.6、决策 3。

> **状态：** 初版。

---

## 目录

1. [设计目标与范围](#1-设计目标与范围)
2. [.rgss 语法定义](#2-rgss-语法定义)
3. [属性映射表](#3-属性映射表)
4. [选择器引擎](#4-选择器引擎)
5. [主题变量系统](#5-主题变量系统)
6. [样式合并与优先级](#6-样式合并与优先级)
7. [样式热重载](#7-样式热重载)
8. [响应式断点](#8-响应式断点)
9. [与其他子系统的交互](#9-与其他子系统的交互)
10. [边界情况处理](#10-边界情况处理)
11. [验证标准](#11-验证标准)

---

## 1. 设计目标与范围

### 1.1 本文档解决什么问题

1. 定义 `.rgss`（Rust GUI Style Sheet）的语法规范和文件格式
2. 定义 CSS 属性到框架内部属性的映射表
3. 定义选择器引擎的匹配算法和优先级计算
4. 定义主题变量系统的定义、引用和覆盖机制
5. 定义样式热重载的检测、重新解析和增量应用流程

### 1.2 设计原则

1. **CSS 兼容优先**：`.rgss` 语法尽量接近标准 CSS，降低学习成本
2. **编译期检查**：属性名和值类型在解析时验证，错误早期发现
3. **增量热重载**：样式变更只标记受影响的 widget dirty，不全量重建
4. **主题变量可覆盖**：应用可部分覆盖框架默认主题

---

## 2. .rgss 语法定义

### 2.1 文件格式

`.rgss` 文件使用类似 CSS 的语法，UTF-8 编码：

```css
/* 变量定义 */
:root {
    --color-primary: #3B82F6;
    --color-bg: #FFFFFF;
    --color-text: #1A1A2E;
    --font-base: "Inter", sans-serif;
    --spacing-unit: 8px;
    --radius-default: 6px;
}

/* 类型选择器 */
Button {
    font-size: 14px;
    font-weight: 500;
    border-radius: var(--radius-default);
    padding: 8px 16px;
}

Button[variant="primary"] { background-color: var(--color-primary); color: #FFFFFF; }
Button[disabled] { opacity: 0.5; pointer-events: none; }

/* 类选择器 */
.page { padding: 24px; background-color: var(--color-bg); }

/* ID 选择器 */
#main-header { height: 64px; padding: 0 24px; }

/* 后代组合器 */
VBox > HBox { spacing: 12px; }

/* 伪类 */
Button:hover { opacity: 0.9; }
Button:focus-visible { outline: 2px solid var(--color-primary); outline-offset: 2px; }
TextField:focus { border-color: var(--color-primary); }

/* 媒体查询 */
@media (max-width: 768px) {
    .page { padding: 12px; }
    HBox { flex-direction: column; }
}

@media (prefers-color-scheme: dark) {
    :root { --color-bg: #1A1A2E; --color-text: #E0E0E0; }
}
```

### 2.2 词法要素

| 元素 | 规则 |
|------|------|
| 注释 | `/* ... */` |
| 颜色 | `#RRGGBB`、`#RRGGBBAA`、`rgb(r,g,b)`、`rgba(r,g,b,a)` |
| 数值 | 整数（`12`）、浮点（`1.5`）、带单位（`14px`、`2em`、`100%`） |
| 变量引用 | `var(--name)` | ⚠️ 设计目标——AC01 CSS 变量系统（⬜ 未实现） |
| CSS 函数 | `calc()`、`min()`、`max()`、`clamp()` |

### 2.3 EBNF 语法（简化）

```ebnf
stylesheet    = { rule | media_query } ;
rule          = selector_list "{" { declaration } "}" ;
selector_list = selector { "," selector } ;
selector      = type_selector [attr_selector] [pseudo_class]
              | class_selector | id_selector | descendant_selector ;
type_selector = identifier ;
class_selector = "." identifier ;
id_selector   = "#" identifier ;
attr_selector = "[" identifier ("=" value)? "]" ;
pseudo_class  = ":" identifier ;
descendant_selector = selector ">" selector | selector " " selector ;
declaration   = property ":" value ";" ;
value         = number | string | color | variable_ref | function_call ;
variable_ref  = "var(" "--" identifier ")" ;
media_query   = "@media" "(" condition ")" "{" { rule } "}" ;
```

### 2.4 解析器实现策略

阶段 1 使用 `cssparser` crate（Mozilla 的 CSS 解析器）复用词法分析和选择器解析，只自定义属性值处理。阶段 2 如有需要可替换为自研解析器。

### 2.5 CSS 变量系统（AC01 —— 待实现）

CSS 自定义属性（custom properties）在 `.rgss` 中的设计与实现约定。

#### 2.5.1 变量定义

```css
:root {
  --wa-spacing: 16px;
  --wa-color-primary: #3B82F6;
}

Button {
  --button-padding: var(--wa-spacing);
}
```

- `:root {}` 块：全局变量，所有 `.rgss` 文件和组件可见
- 组件级 `:host {}` 或类型选择器内：局部变量，仅当前组件及子节点可见（CSS 级联继承模型）
- 子组件可覆盖父变量（重新声明同名变量）

#### 2.5.2 变量求值

`var(--name, fallback)` 语法在 StyleMerger 合并阶段求值：

```rust
/// 变量表——解析 `.rgss` 文件后填充
struct VariableTable {
    global: FxHashMap<String, PropValue>,   // :root {} 变量
    scoped: FxHashMap<WidgetId, FxHashMap<String, PropValue>>, // :host {} 变量
}
```

求值顺序：
1. 查找当前组件作用域（`:host {}`）
2. 未找到 → 查找父组件作用域（级联继承）
3. 未找到 → 查找全局作用域（`:root {}`）
4. 未找到 → 使用 fallback 值（若提供）或报错

#### 2.5.3 与 ThemeVariables 的关系

ST05 `ThemeVariables` 是编译期预定义主题色板（亮/暗色值），**注入**到 AC01 变量表的全局作用域中。`.rgss` 通过 `var(--wa-space-m)` 引用这些预定义主题变量，StyleMerger 在合并时一并求值。

> **实现任务：** [D8 §9.18 AC01](./D8-阶段0开发任务分解.md#918-accordion-tier-2-翻译补全任务2026-06-24-wa-源)

```rust
pub struct RgssParser { file_path: PathBuf }

impl RgssParser {
    pub fn parse(&self, source: &str) -> Result<StyleSheet, ParseError> {
        // 词法分析 → 解析规则 → 验证属性名和值类型
        todo!("阶段 1 实现")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("{file}:{line}:{col}: {message}")]
    WithLocation { file: String, line: usize, col: usize, message: String },
}
```

---

## 3. 属性映射表

### 3.1 布局属性（→ rgui-layout / Taffy）

| .rgss 属性 | 值 | Taffy 映射 |
|-----------|-----|-----------|
| `display` | `flex` \| `grid` \| `block` \| `none` | `Display::Flex/Grid/Block/None` |
| `flex-direction` | `row` \| `column` \| `row-reverse` \| `column-reverse` | `FlexDirection` |
| `flex-wrap` | `nowrap` \| `wrap` \| `wrap-reverse` | `FlexWrap` |
| `justify-content` | `start` \| `end` \| `center` \| `space-between` \| `space-around` \| `space-evenly` | `JustifyContent` |
| `align-items` | `start` \| `end` \| `center` \| `stretch` \| `baseline` | `AlignItems` |
| `align-self` | `auto` \| `start` \| `end` \| `center` \| `stretch` | `AlignSelf` |
| `gap` | `12px` | `gap: Size<12.0>` |
| `flex-grow` | `1` | `flex_grow: 1.0` |
| `flex-shrink` | `0` | `flex_shrink: 0.0` |
| `flex-basis` | `auto` \| `200px` | `flex_basis` |
| `grid-template-columns` | `100px 1fr 2fr` | Taffy grid 轨道 |
| `grid-template-rows` | `auto 1fr` | Taffy grid 轨道 |

### 3.2 视觉属性（→ DrawCommand）

| .rgss 属性 | 值 | PropValue |
|-----------|-----|-----------|
| `background-color` | `#RRGGBB` | `PropValue::Color(...)` |
| `color` | `#RRGGBB` | `PropValue::Color(...)` |
| `opacity` | `0.0-1.0` | `PropValue::Float(...)` |
| `border-radius` | `6px` | `PropValue::Float(...)` |
| `border-width` | `1px` | `PropValue::Float(...)` |
| `border-color` | `#RRGGBB` | `PropValue::Color(...)` |
| `box-shadow` | `x y blur color` | `PropValue::List(...)` |
| `visibility` | `visible` \| `hidden` | `PropValue::Enum(...)` |

### 3.3 文本属性（→ cosmic-text）

| .rgss 属性 | 值 | 说明 |
|-----------|-----|------|
| `font-family` | `"Inter", sans-serif` | 字体族 |
| `font-size` | `14px` | 字体大小 |
| `font-weight` | `400` \| `bold` | 字重 |
| `font-style` | `normal` \| `italic` | 字体样式 |
| `line-height` | `1.5` \| `24px` | 行高 |
| `letter-spacing` | `0.5px` | 字间距 |
| `text-align` | `start` \| `center` \| `end` \| `justify` | 文本对齐 |
| `text-overflow` | `clip` \| `ellipsis` | 溢出处理 |
| `white-space` | `normal` \| `nowrap` \| `pre` | 空白处理 |

### 3.4 尺寸与间距属性

| .rgss 属性 | 说明 |
|-----------|------|
| `width`、`height` | 尺寸（`auto` \| `100px` \| `100%`） |
| `min-width`、`max-width` | 宽度约束 |
| `min-height`、`max-height` | 高度约束 |
| `padding` | 内边距（1-4 值简写） |
| `margin` | 外边距（1-4 值简写） |
| `aspect-ratio` | 宽高比（`16/9`） |

---

## 4. 选择器引擎

### 4.1 选择器类型与 Specificity

```rust
/// 组合器类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CombinatorKind { Descendant, Child }

/// 属性比较运算符。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttrOp { Equal, Contains, StartsWith, EndsWith }

/// CSS 声明（属性名→值）。
#[derive(Debug, Clone)]
pub struct Declaration {
    pub property: String,
    pub value: PropValue,
}

/// 源文件位置（用于错误报告）。
#[derive(Debug, Clone)]
pub struct SourceLocation { pub file: String, pub line: usize, pub col: usize }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Selector {
    Type(String),                                          // Button
    Class(String),                                         // .page
    Id(String),                                            // #main-header
    Attribute { name: String, operator: AttrOp, value: Option<String> },  // [variant="primary"]
    PseudoClass(String),                                   // :hover
    Combinator { ancestor: Box<Selector>, descendant: Box<Selector>, kind: CombinatorKind },
}

/// CSS 特异性三元组 (a, b, c)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Specificity(pub u32, pub u32, pub u32);

impl Selector {
    pub fn specificity(&self) -> Specificity {
        match self {
            Selector::Id(_) => Specificity(1, 0, 0),
            Selector::Class(_) | Selector::Attribute { .. } | Selector::PseudoClass(_) => Specificity(0, 1, 0),
            Selector::Type(_) => Specificity(0, 0, 1),
            Selector::Combinator { ancestor, descendant, .. } => {
                let a = ancestor.specificity();
                let d = descendant.specificity();
                Specificity(a.0 + d.0, a.1 + d.1, a.2 + d.2)
            }
        }
    }
}
```

> **类型选择器与 widget_type 的映射**：`.rgss` 中的类型选择器（如 `Button`）匹配 `widget_type` 的短名（最后一段 `::` 分割）。例如 `"rgui_components::Button"` → 选择器 `Button`。应用可通过 `#[widget(name = "...")]` 自定义匹配名。

### 4.2 匹配算法

```rust
pub struct SelectorEngine { rules: Vec<StyleRule> }

pub struct StyleRule {
    pub selector: Selector,
    pub declarations: Vec<Declaration>,
    pub specificity: Specificity,
    pub source_location: Option<SourceLocation>,
}

impl SelectorEngine {
    /// 匹配 widget 的所有适用规则，按 specificity 合并。
    pub fn match_widget(
        &self, widget_type: &str, class_list: &[&str],
        pseudo_states: &[&str], attr_map: &BTreeMap<&str, PropValue>,
    ) -> BTreeMap<Arc<str>, PropValue> {
        let mut matched: Vec<&StyleRule> = self.rules.iter()
            .filter(|r| r.selector.matches(widget_type, class_list, pseudo_states, attr_map))
            .collect();

        matched.sort_by_key(|r| r.specificity);

        let mut result = BTreeMap::new();
        for rule in &matched {
            for decl in &rule.declarations {
                result.insert(Arc::from(decl.property.as_str()), decl.value.clone());
            }
        }
        result
    }
}
```

### 4.3 伪类状态

| 伪类 | 触发条件 | 更新时机 |
|------|---------|---------|
| `:hover` | 鼠标进入 widget 区域 | 命中测试阶段 |
| `:active` | 鼠标按下未释放 | 事件处理 |
| `:focus` | 键盘焦点在 widget | 焦点管理（D5） |
| `:focus-visible` | 键盘导航获得焦点 | 焦点管理（D5） |
| `:disabled` | widget `disabled` 属性为 true | 属性变更时 |
| `:checked` | 复选框选中 | 状态变更时 |

---

## 5. 主题变量系统

### 5.1 变量定义与引用

```css
:root {
    --color-primary: #3B82F6;
    --color-primary-hover: #2563EB;
    --spacing-sm: 4px;
    --spacing-md: 8px;
    --spacing-lg: 16px;
}

Button {
    background-color: var(--color-primary);
    padding: var(--spacing-md) var(--spacing-lg);
}
```

### 5.2 主题结构

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme { Light, Dark }

pub struct Theme {
    pub name: String,
    pub color_scheme: ColorScheme,
    pub variables: ThemeVariables,
}

pub struct ThemeVariables {
    variables: FxHashMap<String, PropValue>,
}

impl ThemeVariables {
    pub fn get(&self, name: &str) -> Option<&PropValue> {
        self.variables.get(name)
    }
    pub fn merge(&mut self, other: &ThemeVariables) {
        for (key, value) in &other.variables {
            self.variables.insert(key.clone(), value.clone());
        }
    }
}
```

### 5.3 默认主题

框架提供 `Theme::light()` 和 `Theme::dark()` 两个内置主题，包含完整的颜色、字体、间距变量定义（共约 30 个 CSS 自定义属性）。

### 5.4 主题切换

```
App::set_theme(Theme::dark()) → 替换 ViewContext::theme → 标记所有 widget dirty → 重新 view/diff/layout/paint → 新主题渲染
```

---

## 6. 样式合并与优先级

### 6.1 来源优先级（低→高）

```
1. 框架默认样式（rgui-core 内置）
2. 应用 .rgss 文件中的规则
3. 组件 inline style（html! 宏中的属性）
4. 主题变量（var() 解析）
5. !important 声明（最高）
```

### 6.2 合并实现

```rust
pub struct StyleMerger;

impl StyleMerger {
    pub fn merge(
        default_style: &BTreeMap<&'static str, PropValue>,
        rgss_matched: &BTreeMap<Arc<str>, PropValue>,
        inline_style: &BTreeMap<&'static str, PropValue>,
        theme: &Theme,
    ) -> BTreeMap<Arc<str>, PropValue> {
        let mut result = BTreeMap::new();
        for (k, v) in default_style { result.insert(Arc::from(*k), v.clone()); }
        for (k, v) in rgss_matched { result.insert(Arc::clone(k), v.clone()); }
        for (k, v) in inline_style { result.insert(Arc::from(*k), v.clone()); }
        Self::resolve_variables(&mut result, &theme.variables);
        result
    }
}
```

---

## 7. 样式热重载

```
文件系统监控 (notify crate)
  │  *.rgss 变更
  ▼
StyleHotReload
  │  重新解析 → 比较 diff → 找出受影响选择器 → 标记 widget dirty
  ▼
下一帧自动应用新样式 (< 200ms)
```

```rust
pub struct StyleHotReload {
    watcher: notify::RecommendedWatcher,
    stylesheets: FxHashMap<PathBuf, StyleSheet>,
    selector_index: FxHashMap<String, Vec<WidgetId>>,
}

impl StyleHotReload {
    pub fn new(style_dir: PathBuf) -> Result<Self, HotReloadError> { /* ... */ }

    pub fn handle_event(&mut self, event: notify::Event, state_store: &mut StateStore) {
        for path in &event.paths {
            if path.extension().map_or(false, |ext| ext == "rgss") {
                if let Ok(source) = std::fs::read_to_string(path) {
                    match self.reload_file(path, &source, state_store) {
                        Ok(n) => log::info!("样式热重载：{} —— {} 个 widget 受影响", path.display(), n),
                        Err(e) => log::warn!("样式热重载失败：{} —— {:?}", path.display(), e),
                    }
                }
            }
        }
    }
}
```

**降级策略**：语法错误时保持旧样式生效，不崩溃；在 DevTools 面板中显示错误信息。

---

## 8. 响应式断点

| 名称 | 最小宽度 | 典型设备 |
|------|---------|---------|
| `xs` | 0 | 手机竖屏 |
| `sm` | 640px | 手机横屏 |
| `md` | 768px | 平板 |
| `lg` | 1024px | 笔记本 |
| `xl` | 1280px | 桌面显示器 |
| `2xl` | 1536px | 大屏 |

使用方式：`@media (max-width: 768px) { ... }` 或通过 `ViewContext::window_size` 判断当前断点。

---

## 9. 与其他子系统的交互

| 子系统 | 交互 |
|--------|------|
| D1 组件模型 | `ViewContext::theme` 提供主题；`WidgetView::props` 含合并后样式 |
| D2 状态管理 | 样式变更 → dirty → diff + re-layout |
| D3 渲染管线 | 样式属性映射为 DrawCommand |
| D5 事件系统 | 伪类状态由事件系统更新 |
| D7 开发反馈 | 样式热重载是第 1 层反馈（< 200ms） |

---

## 10. 边界情况处理

| 情况 | 处理 |
|------|------|
| 未知属性名 | 警告日志，跳过，不阻塞 |
| 循环变量引用 | 检测循环，使用 fallback 值 |
| 热重载语法错误 | 保持旧样式，UI 中显示错误提示 |
| 大样式表（10000+ 规则） | 按 widget_type 建立反向索引 |

---

## 11. 验证标准

### 11.1 单元测试

| 验证项 | 预期结果 |
|--------|---------|
| .rgss 解析（含选择器、属性、变量） | 正确产出 StyleSheet |
| 选择器匹配 | 正确匹配和拒绝 |
| 优先级计算（ID > 类 > 类型） | ID 样式覆盖类样式 |
| 变量解析（var(--x)） | 正确替换为定义值 |
| 无效属性 | 警告日志，不崩溃 |

### 11.2 集成测试

| 验证项 | 预期结果 |
|--------|---------|
| 主题切换（亮→暗） | 所有 widget 颜色变更 |
| 样式热重载 | < 200ms 生效 |
| 响应式断点切换 | 布局随窗口尺寸调整 |

### 11.3 D0 不变式验证

- [ ] 不变式 1：rgui-style 不依赖 wgpu/winit
- [ ] 不变式 3：样式属性通过 props 传递，无副作用

---

> **下一步：** 本文档经评审确认后，进入 D5（事件系统与输入处理设计）。
