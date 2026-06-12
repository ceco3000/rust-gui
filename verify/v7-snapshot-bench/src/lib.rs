//! V7: 状态快照性能基准
//!
//! 测量 PersistState 序列化/反序列化延迟 + schema 迁移成本。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// PersistState — 满足快照契约的业务状态
pub trait PersistState: Serialize + Send + Sync + 'static {
    fn schema_name(&self) -> &'static str;
    fn schema_version(&self) -> u32;
}

// ── TODO 应用规模 (~50 widgets, ~5KB) ──

#[derive(Clone, Serialize, Deserialize)]
pub struct TodoState {
    pub tasks: Vec<TaskItem>,
    pub filter: String,
    pub selected_id: Option<u64>,
}

impl PersistState for TodoState {
    fn schema_name(&self) -> &'static str { "todo" }
    fn schema_version(&self) -> u32 { 1 }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TaskItem {
    pub id: u64,
    pub title: String,
    pub done: bool,
    pub tags: Vec<String>,
}

pub fn build_todo() -> TodoState {
    let tasks = (0..50).map(|i| TaskItem {
        id: i, title: format!("Task #{i}: 待办事项描述"), done: i % 3 == 0,
        tags: vec![format!("tag-{}", i % 5), "rust-gui".into()],
    }).collect();
    TodoState { tasks, filter: String::new(), selected_id: None }
}

// ── CRUD 管理后台规模 (~200 widgets, ~50KB) ──

#[derive(Clone, Serialize, Deserialize)]
pub struct CrudState {
    pub rows: Vec<HashMap<String, String>>,
    pub sort_column: String,
    pub sort_direction: String,
    pub search_text: String,
    pub page: u32,
    pub page_size: u32,
    pub selected_ids: Vec<u64>,
}

impl PersistState for CrudState {
    fn schema_name(&self) -> &'static str { "crud" }
    fn schema_version(&self) -> u32 { 1 }
}

pub fn build_crud() -> CrudState {
    let rows = (0..100).map(|i| {
        let mut row = HashMap::new();
        row.insert("id".into(), i.to_string());
        row.insert("name".into(), format!("记录 #{i}"));
        row.insert("status".into(), if i % 4 == 0 { "已完成".into() } else { "进行中".into() });
        row
    }).collect();
    CrudState { rows, sort_column: "date".into(), sort_direction: "desc".into(),
        search_text: String::new(), page: 1, page_size: 20, selected_ids: vec![] }
}

// ── 压力测试规模 (~1000 widgets, ~500KB) ──

#[derive(Clone, Serialize, Deserialize)]
pub struct PressureState {
    pub grid_rows: Vec<Vec<String>>,
    pub form_data: HashMap<String, String>,
    pub validation_errors: HashMap<String, Vec<String>>,
}

impl PersistState for PressureState {
    fn schema_name(&self) -> &'static str { "pressure" }
    fn schema_version(&self) -> u32 { 1 }
}

pub fn build_pressure() -> PressureState {
    let grid_rows = (0..500).map(|i| {
        (0..20).map(|j| format!("cell-{}-{}", i, j)).collect()
    }).collect();
    PressureState { grid_rows, form_data: HashMap::new(), validation_errors: HashMap::new() }
}
