# RG01：`.rgui` 格式规范

> **文档定位：** `.rgui` 是 rgui 框架的 UI 声明格式，遵循 HTML5 语法规则。如果你会写 HTML，你就能写 `.rgui`。

> **前置阅读：**
> - [HTML Living Standard](https://html.spec.whatwg.org/)（§13.2 Parsing HTML documents）— `.rgui` 的语法基准
> - [D0 Rust GUI 框架总体设计](./D0-Rust%20GUI%20框架总体设计.md)（§3 核心 Trait、§5 关键数据结构）
> - [D1 组件模型与 WidgetSpec 设计](./D1-组件模型与WidgetSpec设计.md)（§3 WidgetView 数据类型、§7 html! 宏设计）

> **设计原则：** `.rgui` 不发明新语法。所有语法均来自 HTML5——学习 HTML 的基础语法即可编写 `.rgui`。

> **状态：** 初版。

---

## 目录

1. [基础语法（HTML5）](#1-基础语法html5)
2. [元素](#2-元素)
3. [属性](#3-属性)
4. [表达式](#4-表达式)
5. [事件处理器](#5-事件处理器)
6. [脚本关联](#6-脚本关联)
7. [与 html! 宏的关系](#7-与-html-宏的关系)
8. [WidgetView 映射](#8-widgetview-映射)
9. [验证规则](#9-验证规则)
10. [完整示例](#10-完整示例)

---

## 1. 基础语法（HTML5）

`.rgui` 的语法规则与 HTML5 完全一致。以下规则覆盖全部 `.rgui` 语法：

### 1.1 元素

```html
<!-- 开放-闭合 -->
<Button>Click</Button>

<!-- 自闭合 -->
<TextField placeholder="Name"/>
<TextField placeholder="Name">   <!-- 也可省略 / -->
```

### 1.2 属性

```html
<!-- name="value" — 与 HTML 完全相同 -->
<Button text="Save" variant="primary"/>

<!-- 布尔属性：存在即为 true，省略即为 false — 与 HTML 完全相同 -->
<Button disabled/>          <!-- disabled=true -->
<TextField readonly/>      <!-- readonly=true -->
```

### 1.3 事件处理器

```html
<!-- onclick="handler()" — 与 HTML 完全相同 -->
<Button onclick="save()">Save</Button>
<TextField oninput="update()" placeholder="Name"/>
```

### 1.4 注释

```html
<!-- 与 HTML 完全相同 -->
<!-- 这是注释 -->
```

### 1.5 文本内容

```html
<!-- 标签之间的文本自动转为 Text 子组件 — 与 HTML 完全相同 -->
<Label>Hello World</Label>
```

---

## 2. 元素

### 2.1 组件元素

标签名**不区分大小写**（与 HTML5 一致）。解析器将标签名转为小写后查询 `WidgetRegistry`。

```html
<Button/>      <!-- → WidgetView::new("button") -->
<button/>      <!-- 同上，等价 -->
<BUTTON/>      <!-- 同上，等价 -->
```

示例中使用 `PascalCase` 仅为可读性，非强制要求。

### 2.2 文本内容

元素内的文本内容自动转为 `text` 属性（文本语法糖）：

```html
<Button>Save</Button>
<!-- 等价于 <Button text="Save"/> -->
```

当同时有文本内容和显式 `text` 属性时，显式属性优先：

```html
<Label text="Override">Fallback</Label>
<!-- 结果：text="Override" -->
```

### 2.3 嵌套

元素可任意嵌套，形成树结构，与 HTML 完全相同：

```html
<Column spacing="12">
    <Row>
        <Label text="Name"/>
        <TextField placeholder="Enter name"/>
    </Row>
    <Row>
        <Button onclick="save()">Save</Button>
        <Button onclick="cancel()">Cancel</Button>
    </Row>
</Column>
```

### 2.4 `slot` 属性

与 HTML Web Components 的 `<slot>` 机制一致：

```html
<Card>
    <Label slot="header" text="Title"/>
    <Label slot="body" text="Content"/>
    <Button slot="footer" onclick="ok()">OK</Button>
</Card>
```

无 `slot` 属性的子元素进入 `default` 插槽。

### 2.5 `id` 属性

与 HTML 的 `id` 属性语义相同——为元素分配唯一标识符，表达式可通过 `id` 引用元素：

```html
<TextField id="username" placeholder="Name"/>
```

---

## 3. 属性

### 3.1 属性值类型推断

属性值均为字符串，解析器根据字面量格式自动推断 `PropValue` 类型：

| 字面量 | 推断类型 | 示例 |
|--------|---------|------|
| `"hello"` | `Str` | `text="hello"` |
| `"true"` `"false"` | `Bool` | `disabled="true"` |
| `"42"` `"-10"` | `Int` | `spacing="12"` |
| `"3.14"` | `Float` | `opacity="0.8"` |
| `"#FF0000"` | `Color` | `fill="#FF0000"` |
| `"100x200"` | `Size` | `size="48x48"` |
| `"Primary"`（单 PascalCase 词） | `Enum` | `variant="Primary"` |
| `disabled`（无值属性） | `Bool(true)` | `<Button disabled>` |
| `"${...}"`（表达式） | 动态求值 | 见 §4 |

### 3.2 `data-*` 属性

与 HTML 的 `data-*` 属性语义相同——存储自定义数据：

```html
<Button data-action="delete" data-id="42" onclick="handle()">Delete</Button>
```

---

## 4. 表达式

表达式是 `.rgui` 对 HTML5 的唯一扩展——用于标记内的轻量数据绑定，消灭"A 变了 B 跟着变"的简单转发脚本。

### 4.1 对标参考

| 框架 | 表达式语法 | 能力范围 | 设计哲学 |
|------|----------|---------|---------|
| **FXML** | `${var.property}` | 属性路径 | 最小——逻辑交给 Controller |
| **QML** | `property: expr` | 完整 JS | 内联代码 |
| **Vue** | `{{ expr }}` + `v-bind` | 完整 JS | 模板负责视图逻辑 |
| **Angular** | `{{ expr }}` + `[attr]` | 完整 TS | 模板负责视图逻辑 |
| **React JSX** | `{expr}` | 完整 JS | 它就是代码 |
| **`.rgui`** | **`${id.prop.path}`** | **属性路径链** | **最小——复杂逻辑走 `.rhai`** |

### 4.2 语法

```
${id.property.path}
```

- `${...}` 包裹表达式
- `id` 指向当前文件中 `id` 属性标记的元素
- `.property` 访问元素属性（支持多层链：`id.prop1.prop2`）
- 只能出现在**属性值**中，不能出现在文本内容中

```html
<TextField id="name" placeholder="Enter name"/>
<Label text="${name.text}"/>             <!-- 引用 name 的 text 属性 -->
<Label text="${counter.value}"/>          <!-- 引用 counter 的 value 属性 -->
<Label text="${list.selectedIndex}"/>     <!-- 多层属性路径 -->
```

### 4.3 允许的操作

| 操作 | 示例 | 说明 |
|------|------|------|
| 属性访问 | `${name.text}` | 读取目标元素的属性值 |
| 多层路径 | `${name.text.length}` | `.` 分隔的深度属性访问 |

### 4.4 明确禁止的操作

禁止不等于框架做不到——而是这些能力属于 `.rhai` 脚本层，不应承载在标记中。

| 禁止操作 | 错误示例 | 替代方案 |
|---------|---------|---------|
| 算术运算 | ~~`${a.value + b.value}`~~ | `.rhai` 中计算后赋给新属性 |
| 方法调用 | ~~`${items.length()}`~~ | `.rhai` 中调用 |
| 三元/条件 | ~~`${cond ? a : b}`~~ | `.rhai` 中用 if-else |
| 字面量 | ~~`${"hello"}`~~ | 直接用 `text="hello"` |
| 逻辑运算 | ~~`${a && b}`~~ | `.rhai` 中判断后设布尔属性 |
| 字符串拼接 | ~~`${"Hello, " + name.text}`~~ | `.rhai` 中拼接 |

**禁止理由**：参考 FXML 的设计——`${}` 只做属性路径引用，不做计算。FXML 文档原文：*"expression binding ... binds a property value to a variable or expression"*，但 `constants and operators table` 中的运算符支持在 `.rgui` 阶段 0 中明确不包含，因为每加一个运算符都在侵蚀 `.rhai` 的职责边界。

### 4.5 与字面量的区分

- 以 `${` 开头 → 表达式，动态求值
- 不以 `${` 开头 → 字面量，按 §3.1 规则推断类型
- 字面量需要包含 `${` 字符串时，前置 `\` 转义：`text="\${not-an-expr}"`

### 4.6 运行时行为

表达式在以下时机重新求值：
- 初始化（首次加载 `.rgui`）
- 依赖的属性值变化时

实现细节由 RD02 解析器定义。

---

## 5. 事件处理器

### 5.1 语法

与 HTML 完全相同：`onevent="handler()"`（全小写事件名）：

```html
<Button onclick="save()">Save</Button>
<TextField oninput="update()"/>
<CheckBox onchange="toggle()"/>
<ScrollView onscroll="handleScroll()"/>
```

### 5.2 已知事件

| 属性 | message_name | 触发时机 |
|------|-------------|---------|
| `onclick` | `"click"` | 点击 |
| `oninput` | `"text_changed"` | 文本输入变化 |
| `onchange` | `"value_changed"` | 值变化 |
| `onfocus` | `"focus_in"` | 获得焦点 |
| `onblur` | `"focus_out"` | 失去焦点 |
| `onkeydown` | `"key_down"` | 按键 |
| `onscroll` | `"scroll_changed"` | 滚动 |
| `onsubmit` | `"submit"` | 表单提交 |

### 5.3 运行时映射

`onclick="save()"` → 框架查找 `.rhai` 脚本中注册的 `fn save()`，生成 `MessageBinding`。

---

## 6. 脚本关联

`.rgui` 文件中不嵌入脚本。关联 `.rhai` 脚本的方式由构建工具或运行时配置决定（不在 `.rgui` 标记层定义）。

推荐方式（与 HTML `<script>` 一致的语义）：

```html
<link rel="script" href="app.rhai"/>
```

---

## 7. 与 `html!` 宏的关系

| | `.rgui` 文件 | `html!` 宏 |
|--|:-----------:|:---------:|
| 定位 | 文件格式，运行时可加载 | Rust 过程宏，编译期展开 |
| 表达式 | `${id.prop}` 属性引用 | `{rust_expr}` 完整 Rust 表达式 |
| 事件 | `onclick="fn()"`（字符串） | `on:click={Msg::Variant}`（Rust 表达式） |
| 动态表达式 | ❌ 不支持 | ✅ `{rust_expr}` |
| 条件/循环 | ❌ 不支持 | ✅ `{cond ? <A/> : <B/>}` |
| 热重载 | ✅ | ❌ |

---

## 8. WidgetView 映射

| `.rgui` 语法 | WidgetView builder |
|-------------|-------------------|
| `<TagName/>` | `WidgetView::new("tagname")`（标签名转为小写） |
| `id="myId"` | `.id(WidgetId::from_str("myId"))` |
| `attr="value"` | `.prop("attr", PropValue::from("value"))` |
| `attr="${id.prop}"` | `.prop_expr("attr", "id.prop")`（表达式，运行时求值） |
| `onclick="fn()"` | `.on(0, "click", rhai_handler_ref)` |
| `<Tag>text</Tag>` | `.prop("text", "text")`（文本语法糖） |
| 嵌套 `<Child/>` | `.child(child_view)` |
| `slot="name"` | `.slot_child("name", child_view)` |
| `data-*` | `.prop("data-*", value)`（透传） |

### 映射示例

```html
<!-- .rgui 输入 -->
<Column spacing="12" id="root">
    <TextField id="name" placeholder="Name" oninput="updatePreview()"/>
    <Label text="${name.text}"/>
    <Button onclick="save()" variant="primary">Save</Button>
</Column>

<!-- 等价 WidgetView 构建 -->
WidgetView::new("column")
    .id(WidgetId::from_str("root"))
    .prop("spacing", PropValue::Int(12))
    .child(
        WidgetView::new("textfield")
            .id(WidgetId::from_str("name"))
            .prop("placeholder", PropValue::Str(Arc::from("Name")))
            .on(..., "text_changed", /* → Rhai fn updatePreview() */)
    )
    .child(
        WidgetView::new("label")
            .prop_expr("text", "name.text")   <!-- 表达式绑定 -->
    )
    .child(
        WidgetView::new("button")
            .prop("variant", PropValue::Enum(Arc::from("primary")))
            .prop("text", PropValue::Str(Arc::from("Save")))
            .on(..., "click", /* → Rhai fn save() */)
    )
```

---

## 9. 验证规则

| # | 规则 | 来源 |
|---|------|------|
| V1 | 根元素数量 = 1 | HTML 文档约定 |
| V2 | 标签名（转小写后）在 WidgetRegistry 中注册 | rgui 特有 |
| V3 | 标签正确闭合，无交叉嵌套 | HTML well-formed |
| V4 | `id` 值在文件内唯一 | HTML 标准 |
| V5 | `${id.prop}` 中引用的 `id` 在当前文件存在 | rgui 特有 |
| V6 | `${}` 中不包含禁止操作（§4.4） | rgui 特有 |
| V7 | 事件名在全小写已知事件表内 | HTML 惯例 |
| V8 | `slot` 值非空字符串 | Web Components 标准 |

---

## 10. 完整示例

### 10.1 登录表单（含表达式）

`login.rgui`：

```html
<Column spacing="16" padding="24">
    <Label text="Login" variant="header"/>

    <TextField id="username"
        placeholder="Username"
        oninput="updatePreview()"/>

    <TextField id="password"
        placeholder="Password"
        password="true"/>

    <!-- 表达式绑定：实时预览输入内容 -->
    <Label text="${username.text}"/>

    <Row spacing="8">
        <Button variant="primary" onclick="login()">Login</Button>
        <Button onclick="cancel()">Cancel</Button>
    </Row>
</Column>
```

### 10.2 带 Slot 的卡片

```html
<Card>
    <Label slot="header" text="Notification"/>

    <Column slot="body" spacing="4">
        <Label text="You have 3 new messages"/>
        <Label text="Last updated: 2 min ago"/>
    </Column>

    <Row slot="footer" spacing="8">
        <Button onclick="dismiss()">Dismiss</Button>
        <Button variant="primary" onclick="view()">View</Button>
    </Row>
</Card>
```

### 10.3 表单联动（表达式消除脚本）

```html
<Column spacing="12">
    <Slider id="fontSize" min="12" max="48" value="16"/>

    <!-- 不需要 .rhai：Slider 变化时 Label 字体大小自动同步 -->
    <Label text="Resizable Text" fontSize="${fontSize.value}"/>

    <CheckBox id="boldToggle" checked="true"/>
    <Label text="Bold: ${boldToggle.checked}"/>

    <TextField id="name" placeholder="Your name"/>
    <Label text="Hello, ${name.text}"/>
</Column>
```

---

## 附录：已知组件类型

| 类别 | 类型 |
|------|------|
| 基础 | `Accordion`, `AccordionItem`（Tier 2 示范组件） |
| 容器/布局 | `Container`, `Row`, `Column`, `Padding`, `Center`, `Expanded`, `SizedBox`, `Card`, `Divider`, `Image`, `ScrollView`, `Stack`, `ListView` |
| 隐式 | `Text` |
