//! # rgui CRUD 入门示例——联系人管理应用
//!
//! 本示例演示 rgui 框架的基本用法，实现一个完整的 CRUD 应用：
//! - **C**reate（创建）：添加新联系人
//! - **R**ead（读取）：显示联系人列表
//! - **U**pdate（更新）：编辑联系人信息
//! - **D**elete（删除）：移除联系人
//!
//! ## 使用方法
//!
//! ```bash
//! cargo run -p crud
//! ```
//!
//! 窗口打开后：
//! - 点击 [添加] 创建新联系人
//! - 点击联系人行 → 选中
//! - 选中后点击 [删除] → 移除
//! - 选中后点击 [编辑] → 修改名称

use rgui::app::{App, AppConfig};
use rgui::prelude::*;
use std::sync::{Arc, Mutex};

// ============================================================================
// 第 1 步：定义数据结构
// ============================================================================

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

// ============================================================================
// 第 2 步：定义全局共享状态
// ============================================================================

/// 应用的全局状态。
///
/// rgui 当前使用 `Arc<Mutex<T>>` 在交互回调之间共享可变状态。
/// 后续版本将提供更完善的声明式状态管理。
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

    // ---- Create ----

    /// 使用编辑框中的信息添加新联系人。
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

    /// 获取联系人数量。
    fn contact_count(&self) -> usize {
        self.contacts.len()
    }

    // ---- Update ----

    /// 将选中的联系人信息改为编辑框中的值。
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

    /// 删除选中的联系人。
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

// ============================================================================
// 第 3 步：UI 布局常量
// ============================================================================
//
// 框架当前使用绝对坐标定位交互区域。
// 坐标原点在窗口左上角，x 向右递增，y 向下递增。

const INPUT_Y: f64 = 60.0; // 输入区域纵坐标
const BUTTON_Y: f64 = 120.0; // 按钮行纵坐标
const LIST_TOP: f64 = 170.0; // 列表起始纵坐标
const LIST_ROW_H: f64 = 28.0; // 列表每行高度
const LEFT: f64 = 20.0; // 左列 x 坐标
const LABEL_W: f64 = 40.0; // 标签宽度（姓名/邮箱/电话）
const INPUT_W: f64 = 160.0; // 输入框宽度

// ============================================================================
// 第 4 步：构建 UI
// ============================================================================

/// 主函数——构建应用并运行。
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 4.1 创建应用配置（标题 + 窗口尺寸）
    let mut app = App::new(
        AppConfig::new()
            .title("rgui CRUD 示例——联系人管理")
            .window_size(820.0, 600.0),
    );
    // 注册内置组件（Button、Label、TextField）
    app.register_defaults();

    // 4.2 创建共享状态
    let state = Arc::new(Mutex::new(AppState::new()));

    // 4.3 注册输入区域——姓名
    let s1 = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(101),
        Rect::new(LEFT + LABEL_W, INPUT_Y, INPUT_W, 28.0),
        "edit_name",
        move |_| {
            let _s = s1.lock().unwrap();
            println!("[操作提示] 点击姓名输入区");
        },
    );

    // 4.4 注册输入区域——邮箱
    let s2 = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(102),
        Rect::new(LEFT + LABEL_W, INPUT_Y + 32.0, INPUT_W, 28.0),
        "edit_email",
        move |_| {
            let _s = s2.lock().unwrap();
            println!("[操作提示] 点击邮箱输入区");
        },
    );

    // 4.5 注册输入区域——电话
    let s3 = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(103),
        Rect::new(LEFT + LABEL_W, INPUT_Y + 64.0, INPUT_W, 28.0),
        "edit_phone",
        move |_| {
            let _s = s3.lock().unwrap();
            println!("[操作提示] 点击电话输入区");
        },
    );

    // 4.6 [添加] 按钮——创建联系人
    //
    // 每次点击添加一个新联系人。
    // 如果编辑框非空，使用编辑框中的值；否则使用默认值。
    let s_add = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(201),
        Rect::new(20.0, BUTTON_Y, 80.0, 32.0),
        "add",
        move |_| {
            let mut guard = s_add.lock().unwrap_or_else(|e| e.into_inner());
            guard.add_contact();
            println!("联系人总数: {}", guard.contact_count());
        },
    );

    // 4.7 [编辑] 按钮——修改选中联系人
    let s_edit = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(202),
        Rect::new(110.0, BUTTON_Y, 80.0, 32.0),
        "edit",
        move |_| {
            let mut guard = s_edit.lock().unwrap_or_else(|e| e.into_inner());
            guard.edit_selected();
        },
    );

    // 4.8 [删除] 按钮——删除选中联系人
    let s_del = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(203),
        Rect::new(200.0, BUTTON_Y, 80.0, 32.0),
        "delete",
        move |_| {
            let mut guard = s_del.lock().unwrap_or_else(|e| e.into_inner());
            guard.delete_selected();
        },
    );

    // 4.9 [全清除] 按钮——清空所有联系人
    let s_clear = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(204),
        Rect::new(290.0, BUTTON_Y, 100.0, 32.0),
        "clear",
        move |_| {
            let mut guard = s_clear.lock().unwrap_or_else(|e| e.into_inner());
            guard.contacts.clear();
            guard.selected_index = None;
            guard.message = "已清空所有联系人".into();
        },
    );

    // 4.10 注册联系人列表行（最多 12 行）
    //
    // 每一行是一个可点击的区域，点击后选中该联系人。
    for i in 0..12_usize {
        let s_row = Arc::clone(&state);
        let y = LIST_TOP + i as f64 * LIST_ROW_H;
        app.register_interaction(
            WidgetId::from_u64(300 + i as u64),
            Rect::new(LEFT, y, 780.0, LIST_ROW_H),
            "select_row",
            move |_| {
                let mut guard = s_row.lock().unwrap_or_else(|e| e.into_inner());
                if i < guard.contacts.len() {
                    guard.select(i);
                }
            },
        );
    }

    // 4.11 运行应用——进入 winit 事件循环
    println!("\n=== rgui CRUD 示例 ===");
    println!("联系人管理应用已启动。");
    println!();
    println!("操作指引：");
    println!("  1. 点击 [添加] → 新建联系人");
    println!("  2. 点击列表中的某一行 → 选中");
    println!("  3. 选中后点击 [编辑] → 更新联系人姓名");
    println!("  4. 选中后点击 [删除] → 移除联系人");
    println!("  5. 点击 [全清除] → 清空列表");
    println!();

    app.run()
}
