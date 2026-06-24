# F06：《新手入门》CRUD 教程

> **文档定位：** 面向初学者的完整 rgui 教程，从环境搭建到完成一个 CRUD（创建 / 读取 / 更新 / 删除）应用。
>
> **适用读者：** 有基本 Rust 知识（`struct`、`enum`、`fn`、`match`）但无 GUI 框架经验的开发者。
>
> **所需时间：** 约 60-90 分钟
>
> **前置阅读：** [Rust GUI 框架总体设计](./D0-Rust%20GUI%20框架总体设计.md)（§2 Crate 结构、§3 核心 Trait 体系概述）
>
> **状态：** 初版。

---

## 目录

1. [环境搭建](#1-环境搭建)
2. [创建项目](#2-创建项目)
3. [项目结构解析](#3-项目结构解析)
4. [第一个 rgui 应用——Hello World](#4-第一个-rgui-应用hello-world)
5. [核心概念](#5-核心概念)
6. [CRUD 应用——联系人管理](#6-crud-应用联系人管理)
7. [编译与运行](#7-编译与运行)
8. [常见问题](#8-常见问题)
9. [下一步](#9-下一步)

---

## 1. 环境搭建

### 1.1 安装 Rust

如果你还没有安装 Rust，使用 rustup 安装：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

验证安装：

```bash
rustc --version   # 应显示 rustc 1.85 或更高版本
cargo --version   # 应显示 cargo 1.85 或更高版本
```

### 1.2 安装 cargo-generate（可选）

cargo-generate 是一个脚手架工具，可以一键创建 rgui 项目：

```bash
cargo install cargo-generate
```

如果安装速度慢，可以用国内镜像：

```bash
# 设置 cargo 镜像（~/.cargo/config.toml）
# 具体配置方法见 https://rsproxy.cn/
```

---

## 2. 创建项目

### 2.1 使用 cargo-generate（推荐，需要 rgui 模板）

```bash
cargo generate rgui
```

这将提示输入项目名称，然后创建一个完整的 rgui 项目骨架。

### 2.2 手动创建（了解底层结构）

也可以手动创建项目，这有助于理解 rgui 的项目结构：

```bash
cargo new my-crud-app
cd my-crud-app
```

然后在 `Cargo.toml` 中添加依赖：

```toml
[package]
name = "my-crud-app"
version = "0.1.0"
edition = "2021"

[dependencies]
rgui = "0.1"
```

> **注意：** rgui 当前尚未发布到 crates.io。在开发阶段，需要在 workspace 中使用 path 依赖。

---

## 3. 项目结构解析

一个 rgui 项目的典型目录结构：

```
my-crud-app/
├── Cargo.toml          # 项目配置与依赖
├── src/
│   └── main.rs         # 应用入口
```

与传统的 GUI 框架不同，rgui 的 crate 是分层设计的：

| Crate | 用途 | 开发者是否需要直接接触 |
|-------|------|----------------------|
| `rgui` | 顶层 facade，重新导出所有公共 API | **是**——应用程序只需依赖此 crate |
| `rgui-core` | 核心类型 WidgetId、Rect、WidgetSpec trait 等 | 了解即可，通过 rgui 使用 |
| `rgui-render` | 渲染引擎（Vello + wgpu） | 不需要 |
| `rgui-platform` | 窗口管理和输入事件（winit） | 不需要 |
| `rgui-state` | 状态管理 | 了解即可 |
| `rgui-style` | 样式系统 | 进阶需要 |
| `rgui-components` | 内置组件库 | **使用**——当前为空，Tier 2 (.rgui+.rhai) 示范组件见 `examples/one-accordion/` |
| `rgui-a11y` | 无障碍系统 | 进阶需要 |
| `rgui-macros` | 过程宏 | 进阶需要 |

应用程序开发者只需在 `Cargo.toml` 中添加 `rgui` 依赖，其他所有 crate 由 rgui 自动引入。

---

## 4. 第一个 rgui 应用——Hello World

先从最简单的例子开始。创建一个窗口，显示文字并支持点击交互。

### 4.1 完整代码

```rust
// src/main.rs

use rgui::app::{App, AppConfig};
use rgui::prelude::*;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建应用配置
    let config = AppConfig::new()
        .title("我的第一个 rgui 应用")
        .window_size(400.0, 300.0);
    let mut app = App::new(config);

    // 2. 注册内置组件
    app.register_defaults();

    // 3. 创建共享状态
    let count = Arc::new(Mutex::new(0i32));

    // 4. 注册交互区域（可点击的按钮）
    let c = Arc::clone(&count);
    app.register_interaction(
        WidgetId::from_u64(1),              // 唯一标识
        Rect::new(150.0, 130.0, 100.0, 40.0), // x, y, width, height
        "click",                            // 事件名称
        move |_| {
            let mut guard = c.lock().unwrap_or_else(|e| e.into_inner());
            *guard += 1;
            println!("点击次数: {}", *guard);
        },
    );

    // 5. 启动应用
    println!("rgui 应用已启动，点击窗口中间的区域查看控制台输出。");
    app.run()
}
```

### 4.2 逐行解读

**第 1 步：创建 `AppConfig`**
```rust
let config = AppConfig::new()
    .title("我的第一个 rgui 应用")  // 窗口标题
    .window_size(400.0, 300.0);     // 窗口宽高（逻辑像素）
```

`AppConfig` 使用 builder 模式配置窗口参数。常用的配置项：
- `title()`：窗口标题
- `window_size()`：窗口初始尺寸
- `resizable()`：是否可调整窗口大小（默认 true）

**第 2 步：`App::new()` 与 `register_defaults()`**
```rust
let mut app = App::new(config);
app.register_defaults();
```

`App` 是 rgui 应用的核心结构体。`register_defaults()` 注册内置组件（Label、Button、TextField），使它们可被框架识别。

**第 3 步：`Arc<Mutex<T>>` 共享状态**
```rust
let count = Arc::new(Mutex::new(0i32));
```

由于交互回调是 `FnMut` 闭包，需要共享可变状态。rgui 当前使用 `Arc<Mutex<T>>` 模式，这是 Rust 中最常用的跨线程共享可变状态的方式。

**第 4 步：`register_interaction()` 注册交互区域**
```rust
app.register_interaction(
    WidgetId::from_u64(1),
    Rect::new(150.0, 130.0, 100.0, 40.0),
    "click",
    move |_| { /* 回调逻辑 */ },
);
```

参数说明：
1. `WidgetId`：每个交互区域的唯一标识，由 `from_u64()` 创建。不同的交互区域必须使用不同的 ID。
2. `Rect`：区域位置和尺寸——`Rect::new(x, y, width, height)`，`(x, y)` 是左上角坐标，原点在窗口左上角。
3. `action`：事件名称字符串，用于区分不同事件。
4. 闭包：点击触发时的回调函数。

**第 5 步：`app.run()` 启动事件循环**
```rust
app.run()
```

这行代码启动 winit 事件循环，开始处理窗口事件和渲染。在此之后的所有代码不会执行，直到窗口关闭。

---

## 5. 核心概念

### 5.1 WidgetId —— 组件标识

`WidgetId` 是每个 widget 实例在运行时的唯一标识。框架用它来查找交互区域、维护状态和路由事件。

```rust
let id_a = WidgetId::from_u64(1);
let id_b = WidgetId::from_u64(2);
assert_ne!(id_a, id_b);  // 不同的 ID
```

`WidgetId` 是 `Copy` 类型，开销极低。在同一个应用中，每个交互区域必须使用唯一的 `WidgetId`。

### 5.2 Rect —— 布局几何

`Rect` 定义了一个轴对齐的矩形区域：

```rust
// 从左上角 (x, y) 和宽高创建
let rect = Rect::new(50.0, 80.0, 200.0, 40.0);
//               x,    y,   width,  height

// 也提供其他构造方法
Rect::from_ltrb(left, top, right, bottom);   // 从四边创建
Rect::ZERO;                                     // 零矩形常量

// 常用方法
rect.contains(point);    // 判断点是否在矩形内
rect.union(other);       // 并集（最小外接矩形）
rect.intersection(other); // 交集
rect.is_empty();         // 是否为空
```

### 5.3 App 与 AppConfig

```rust
let config = AppConfig::new()
    .title("我的应用")
    .window_size(800.0, 600.0)
    .resizable(true);
let mut app = App::new(config);
```

`App` 结构体提供了以下方法：

| 方法 | 用途 |
|------|------|
| `register_defaults()` | 注册内置组件（Button、Label、TextField） |
| `register_interaction()` | 注册可点击的交互区域 |
| `run()` | 启动窗口和事件循环 |
| `tick()` | 手动触发一帧（用于测试） |
| `registry()` / `registry_mut()` | 访问组件注册表 |

### 5.4 回调与共享状态

rgui 的交互回调使用 Rust 闭包。由于 `register_interaction` 要求回调是 `'static` 生命周期（即不借用局部变量），共享状态通过 `Arc<Mutex<T>>` 实现：

```rust
let state = Arc::new(Mutex::new(MyState::new()));

// 在闭包中克隆 Arc 引用
let s = Arc::clone(&state);
app.register_interaction(/* ... */, move |_| {
    let mut guard = s.lock().unwrap_or_else(|e| e.into_inner());
    guard.do_something();
});
```

- `Arc<T>`（Atomic Reference Counting）：线程安全的引用计数智能指针。`Arc::clone()` 增加引用计数，各闭包可独立持有同一份数据的引用。
- `Mutex<T>`：互斥锁，保证同一时间只有一个线程能访问数据。`.lock()` 返回 `MutexGuard<T>`，离开作用域时自动释放锁。
- `unwrap_or_else(|e| e.into_inner())`：处理锁已被 poisioned 的情况，获取内部数据继续使用。

### 5.5 组件（WidgetSpec）

rgui 的核心组件抽象是 `WidgetSpec` trait。每个组件（Button、Label、TextField 等）都实现了这个 trait：

```rust
pub trait WidgetSpec: Send + Sync + 'static {
    type State: PersistState;    // 组件持有的业务状态类型
    type Message: AppMessage;    // 组件产生的消息类型

    fn name(&self) -> &'static str;
    fn view(&self, state: &Self::State, ctx: &ViewContext) -> WidgetView<Self::Message>;
    fn update(&self, msg: Self::Message, state: &mut Self::State, ctx: &mut UpdateContext);
    fn measure(&self, state: &Self::State, constraints: BoxConstraints, ctx: &MeasureContext) -> Size;
    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext);
    fn accessibility(&self, state: &Self::State, ctx: &AccessContext) -> AccessibilityNode;
}
```

在入门阶段，你只需要使用已有的组件，不需要自己实现 `WidgetSpec`。框架提供了 9 个内置组件：

| 组件 | 用途 | 状态类型 | 消息类型 |
|------|------|---------|---------|
| `Button` | 按钮 | `ButtonState` | `ButtonMessage` |
| `Label` | 文本标签 | `LabelState` | `LabelMessage` |
| `TextField` | 文本输入框 | `TextFieldState` | `TextFieldMessage` |
| `CheckBox` | 复选框 | `CheckBoxState` | `CheckBoxMessage` |
| `RadioButton` | 单选按钮 | `RadioButtonState` | `RadioButtonMessage` |
| `Switch` | 开关 | `SwitchState` | `SwitchMessage` |
| `Slider` | 滑块 | `SliderState` | `SliderMessage` |
| `ProgressBar` | 进度条 | `ProgressBarState` | `ProgressBarMessage` |
| `DataGrid` | 数据表格 | `DataGridState` | `DataGridMessage` |

### 5.6 使用组件

虽然当前阶段 rgui 的交互系统基于 `register_interaction`，但组件本身是框架的核心抽象。使用组件分两步：

**创建组件状态：**
```rust
use rgui::{Button, ButtonState, Label, LabelState};

let btn_state = ButtonState::new("点击我");
let label_state = LabelState { text: "你好".into() };
```

**通过 WidgetSpec 方法操作组件：**
```rust
// 生成视图描述
let view_ctx = ViewContext::new(Size::new(400.0, 300.0));
let view = Button.view(&btn_state, &view_ctx);
println!("按钮视图: {:?}", view);

// 处理消息
let mut state = ButtonState::new("OK").disabled(true);
Button.update(ButtonMessage::Pressed, &mut state, &mut UpdateContext::default());
assert!(!state.pressed);  // disabled 状态下的 Pressed 被忽略

// 测量尺寸
let size = Button.measure(&state, BoxConstraints::UNCONSTRAINED, &MeasureContext::default());
println!("按钮尺寸: {}×{}", size.width, size.height);
```

---

## 6. CRUD 应用——联系人管理

现在让我们从零开始构建一个完整的 CRUD 应用：**联系人管理**。

### 6.1 功能设计

我们的联系人管理应用支持以下操作：

| 操作 | 说明 | CRUD 对应 |
|------|------|-----------|
| 添加联系人 | 在表单中填写姓名、邮箱、电话，点击[添加] | **C**reate |
| 查看联系人列表 | 所有联系人在列表中显示 | **R**ead |
| 选中联系人 | 点击列表中某一行，信息填入编辑框 | 更新准备 |
| 编辑联系人 | 修改选中的联系人信息，点击[编辑] | **U**pdate |
| 删除联系人 | 选中后点击[删除] | **D**elete |
| 清空全部 | 一键移除所有联系人 | 批量删除 |

### 6.2 目录结构

```
my-crud-app/
├── Cargo.toml
└── src/
    └── main.rs
```

### 6.3 第 1 步：Cargo.toml

```toml
[package]
name = "my-crud-app"
version = "0.1.0"
edition = "2021"
rust-version = "1.85"

[dependencies]
rgui = "0.1"
```

### 6.4 第 2 步：定义数据结构

在 `src/main.rs` 顶部，先定义联系人的数据结构：

```rust
use rgui::app::{App, AppConfig};
use rgui::prelude::*;
use std::sync::{Arc, Mutex};

/// 联系人。
#[derive(Debug, Clone)]
struct Contact {
    name: String,
    email: String,
    phone: String,
}

impl Contact {
    fn new(name: &str, email: &str, phone: &str) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
            phone: phone.into(),
        }
    }
}
```

### 6.5 第 3 步：定义应用状态

```rust
/// 应用的全局状态。
struct AppState {
    contacts: Vec<Contact>,
    selected_index: Option<usize>,
    edit_name: String,
    edit_email: String,
    edit_phone: String,
    message: String,
}

impl AppState {
    fn new() -> Self {
        // 初始演示数据
        let contacts = vec![
            Contact::new("Alice", "alice@example.com", "138-0001-0001"),
            Contact::new("Bob", "bob@example.com", "138-0002-0002"),
        ];
        Self {
            contacts,
            selected_index: None,
            edit_name: String::new(),
            edit_email: String::new(),
            edit_phone: String::new(),
            message: "欢迎使用联系人管理".into(),
        }
    }
}
```

### 6.6 第 4 步：实现 CRUD 操作

在 `AppState` 上实现四个操作方法：

```rust
impl AppState {
    // ---- Create ----
    fn add_contact(&mut self) {
        let name = if self.edit_name.is_empty() {
            "新联系人".into()
        } else {
            self.edit_name.clone()
        };
        let email = self.edit_email.clone();
        let phone = self.edit_phone.clone();
        self.contacts.push(Contact::new(&name, &email, &phone));
        self.selected_index = Some(self.contacts.len() - 1);
        self.message = format!("已添加: {name}");
        self.edit_name.clear();
        self.edit_email.clear();
        self.edit_phone.clear();
    }

    // ---- Read ----
    fn contact_count(&self) -> usize {
        self.contacts.len()
    }

    // ---- Update ----
    fn edit_selected(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx < self.contacts.len() {
                if !self.edit_name.is_empty() {
                    self.contacts[idx].name = self.edit_name.clone();
                }
                if !self.edit_email.is_empty() {
                    self.contacts[idx].email = self.edit_email.clone();
                }
                if !self.edit_phone.is_empty() {
                    self.contacts[idx].phone = self.edit_phone.clone();
                }
                self.message = format!("已更新: {}", self.contacts[idx].name);
                self.edit_name.clear();
                self.edit_email.clear();
                self.edit_phone.clear();
            }
        } else {
            self.message = "请先选中一个联系人".into();
        }
    }

    // ---- Delete ----
    fn delete_selected(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx < self.contacts.len() {
                let name = self.contacts[idx].name.clone();
                self.contacts.remove(idx);
                self.selected_index = None;
                self.message = format!("已删除: {name}");
            }
        } else {
            self.message = "请先选中一个联系人".into();
        }
    }

    /// 选中某个联系人（将信息填入编辑框）。
    fn select(&mut self, index: usize) {
        if index < self.contacts.len() {
            self.selected_index = Some(index);
            let c = &self.contacts[index];
            self.edit_name = c.name.clone();
            self.edit_email = c.email.clone();
            self.edit_phone = c.phone.clone();
            self.message = format!("已选中: {}", c.name);
        }
    }
}
```

### 6.7 第 5 步：定义布局常量

```rust
const INPUT_Y: f64 = 60.0;       // 输入区域纵坐标
const BUTTON_Y: f64 = 120.0;     // 按钮行纵坐标
const LIST_TOP: f64 = 170.0;     // 列表起始纵坐标
const LIST_ROW_H: f64 = 28.0;    // 列表每行高度
const LEFT: f64 = 20.0;          // 左列 x 坐标
const LABEL_W: f64 = 40.0;       // 标签宽度
const INPUT_W: f64 = 160.0;      // 输入框宽度
```

这些常量定义了 UI 元素的坐标位置。rgui 当前使用 **绝对坐标定位** 交互区域，坐标原点在窗口左上角。

### 6.8 第 6 步：构建 UI 并启动应用

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建应用配置
    let mut app = App::new(
        AppConfig::new()
            .title("联系人管理")
            .window_size(820.0, 600.0),
    );
    app.register_defaults();

    // 创建共享状态（Arc<Mutex<T>> 模式）
    let state = Arc::new(Mutex::new(AppState::new()));

    // 注册姓名输入区
    let s = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(101),
        Rect::new(LEFT + LABEL_W, INPUT_Y, INPUT_W, 28.0),
        "edit_name",
        move |_| { let _s = s.lock().unwrap(); },
    );

    // 注册邮箱输入区
    let s = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(102),
        Rect::new(LEFT + LABEL_W, INPUT_Y + 32.0, INPUT_W, 28.0),
        "edit_email",
        move |_| { let _s = s.lock().unwrap(); },
    );

    // 注册电话输入区
    let s = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(103),
        Rect::new(LEFT + LABEL_W, INPUT_Y + 64.0, INPUT_W, 28.0),
        "edit_phone",
        move |_| { let _s = s.lock().unwrap(); },
    );

    // [添加] 按钮
    let s = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(201),
        Rect::new(20.0, BUTTON_Y, 80.0, 32.0),
        "add",
        move |_| {
            let mut guard = s.lock().unwrap_or_else(|e| e.into_inner());
            guard.add_contact();
            println!("联系人总数: {}", guard.contact_count());
        },
    );

    // [编辑] 按钮
    let s = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(202),
        Rect::new(110.0, BUTTON_Y, 80.0, 32.0),
        "edit",
        move |_| {
            let mut guard = s.lock().unwrap_or_else(|e| e.into_inner());
            guard.edit_selected();
        },
    );

    // [删除] 按钮
    let s = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(203),
        Rect::new(200.0, BUTTON_Y, 80.0, 32.0),
        "delete",
        move |_| {
            let mut guard = s.lock().unwrap_or_else(|e| e.into_inner());
            guard.delete_selected();
        },
    );

    // [全清除] 按钮
    let s = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(204),
        Rect::new(290.0, BUTTON_Y, 100.0, 32.0),
        "clear",
        move |_| {
            let mut guard = s.lock().unwrap_or_else(|e| e.into_inner());
            guard.contacts.clear();
            guard.selected_index = None;
            guard.message = "已清空所有联系人".into();
        },
    );

    // 注册 12 行联系人列表
    for i in 0..12_usize {
        let s = Arc::clone(&state);
        let y = LIST_TOP + i as f64 * LIST_ROW_H;
        app.register_interaction(
            WidgetId::from_u64(300 + i as u64),
            Rect::new(LEFT, y, 780.0, LIST_ROW_H),
            "select_row",
            move |_| {
                let mut guard = s.lock().unwrap_or_else(|e| e.into_inner());
                if i < guard.contacts.len() {
                    guard.select(i);
                }
            },
        );
    }

    // 输出操作指引并启动应用
    println!("\n=== 联系人管理 CRUD 应用 ===");
    println!("操作指引：");
    println!("  1. 点击 [添加] → 新建联系人");
    println!("  2. 点击列表中的某一行 → 选中");
    println!("  3. 选中后点击 [编辑] → 更新联系人信息");
    println!("  4. 选中后点击 [删除] → 移除联系人");
    println!("  5. 点击 [全清除] → 清空列表");
    println!();

    app.run()
}
```

### 6.9 完整代码

完整代码见 `examples/crud/src/main.rs`。你也可以直接运行项目中的示例：

```bash
cargo run -p crud
```

### 6.10 关键要点总结

1. **`register_interaction(WidgetId, Rect, action, callback)`** 是当前 rgui 定义交互的主要方法
2. **`Arc<Mutex<T>>`** 是跨回调共享可变状态的模式
3. **`WidgetId`** 每个交互区域必须有唯一 ID
4. **`Rect::new(x, y, w, h)`** 使用绝对坐标定位
5. 应用逻辑集中在 `AppState` 中，保持了关注点分离

---

## 7. 编译与运行

```bash
# 编译并运行 CRUD 示例
cargo run -p crud

# 编译当前项目
cargo run

# 仅编译（不运行）
cargo check

# 运行测试
cargo test
```

### 7.1 编译说明

- rgui 依赖 `wgpu` 和 `vello`，第一次编译需要下载和编译 GPU 相关依赖，耗时较长（约 5-10 分钟）
- 后续增量编译通常只需数秒
- 确保系统已安装 Vulkan/Metal/DirectX 驱动（macOS 自带 Metal，Windows 自带 DirectX，Linux 需要 Vulkan 驱动）

### 7.2 运行时说明

- 窗口打开后，**点击交互区域**将在控制台输出日志
- 帧日志和渲染信息会实时输出到终端
- 关闭窗口即可结束应用

---

## 8. 常见问题

### Q: 编译失败，提示找不到 `rgui` crate

A: rgui 当前处于开发阶段，尚未发布到 crates.io。如果使用 workspace 内编译，需要将项目放在 rgui workspace 下，或使用 path 依赖。也可以使用 `cargo generate` 模板创建项目。

### Q: 窗口显示但点击没有反应

A: 确认 `register_interaction` 的 `Rect` 参数在窗口可见区域内，且 `WidgetId` 未被重复使用。

### Q: 如何调整 UI 布局

A: 修改 `Rect::new(x, y, width, height)` 中的坐标值。当前 rgui 使用绝对坐标，未来版本会引入布局引擎（基于 Taffy）。

### Q: 如何添加更多交互

A: 调用 `register_interaction()` 注册新的交互区域，确保使用不同的 `WidgetId`。

### Q: 框架的渲染实现是哪一种

A: 默认使用 Vello（基于 wgpu 的 GPU 渲染器）。也支持 Skia CPU 回退后端（启用 `skia-backend` feature）。

### Q: 如何在 macOS 上运行

A: macOS 完全支持。确保 Xcode Command Line Tools 已安装：
```bash
xcode-select --install
```

### Q: 如何调试应用

A: 方法：
1. 控制台输出：所有 `println!()` 输出会自动显示
2. 启用框架日志：设置环境变量 `RUST_LOG=debug`
3. 检查事件：`app.events()` 返回当前帧的事件列表

---

## 9. 下一步

完成本教程后，可以继续阅读以下资料深入了解 rgui：

| 文档 | 说明 |
|------|------|
| [D1 组件模型与 WidgetSpec 设计](./D1-组件模型与WidgetSpec设计.md) | 深入理解组件架构 |
| [D2 状态管理与差分更新设计](./D2-状态管理与差分更新设计.md) | 了解声明式状态管理 |
| [D3 渲染管线与场景图设计](./D3-渲染管线与场景图设计.md) | 渲染流水线详解 |
| [D4 样式系统与 rgss 设计](./D4-样式系统与rgss设计.md) | 样式系统（.rgss 文件） |
| [D5 事件系统与输入处理设计](./D5-事件系统与输入处理设计.md) | 事件路由和焦点管理 |
| [D8 阶段0开发任务分解](./D8-阶段0开发任务分解.md) | 项目里程碑和任务规划 |
| [D10 组件开发规范与示例](./D10-组件开发规范与示例.md) | 如何开发自定义组件 |

### 进阶练习

1. 给 `Contact` 添加更多字段（公司、职位、生日等）
2. 实现一个搜索功能，输入关键词过滤联系人列表
3. 添加数据持久化——将联系人保存到文件
4. 为联系人添加标签/分组功能
5. 将联系人按姓名排序显示

---

> **下一步：** 本教程对应的示例代码见 `examples/crud/`。如有问题，请查阅设计文档或在项目 Issues 中提问。
