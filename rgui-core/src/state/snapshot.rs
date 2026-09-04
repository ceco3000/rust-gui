//! 快照：`Snapshot` / `Snapshotter` / `SchemaMigration`（greenfield §B.1 / §C.1 state/snapshot.rs）。
//!
//! D4 最小实现：全量快照（可序列化语义占位，序列化使得在 D6/D7）。当前提供：
//! - `Snapshot`：可序列化全量快照（含 schema 名/版本 + 状态数据）。
//! - `Snapshotter`：快照器（登记/创建快照）。
//! - `SchemaMigration`：schema 迁移占位。

use crate::id::WidgetId;
use crate::view::PropValue;
use std::collections::BTreeMap;

/// 快照。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Snapshot {
    /// schema 稳定名。
    pub schema_name: String,
    /// schema 版本。
    pub schema_version: u32,
    /// 实例状态表（WidgetId → 状态数据）。
    pub instances: BTreeMap<u64, PropValue>,
}

impl Snapshot {
    /// 构造带 schema 的空快照。
    pub fn new(schema_name: impl Into<String>, schema_version: u32) -> Self {
        Self {
            schema_name: schema_name.into(),
            schema_version,
            instances: BTreeMap::new(),
        }
    }

    /// 登记一个实例状态。
    pub fn insert_state(&mut self, id: WidgetId, value: PropValue) {
        self.instances.insert(id.0, value);
    }

    /// 读取指定实例状态。
    pub fn get_state(&self, id: WidgetId) -> Option<&PropValue> {
        self.instances.get(&id.0)
    }
}

/// 快照器。
#[derive(Debug, Clone, Default)]
pub struct Snapshotter {
    /// 当前 schema 名（默认）。
    schema_name: String,
    /// 当前 schema 版本。
    schema_version: u32,
}

impl Snapshotter {
    /// 构造快照器（默认 schema）。
    pub fn new() -> Self {
        Self {
            schema_name: "default".to_string(),
            schema_version: 0,
        }
    }

    /// 设置 schema。
    pub fn with_schema(mut self, name: impl Into<String>, version: u32) -> Self {
        self.schema_name = name.into();
        self.schema_version = version;
        self
    }

    /// 创建空快照。
    pub fn snapshot(&self) -> Snapshot {
        Snapshot::new(self.schema_name.clone(), self.schema_version)
    }
}

/// Schema 迁移（D4 最小占位）。
#[derive(Debug, Clone, Default)]
pub struct SchemaMigration {
    /// 迁移目标版本。
    pub target_version: u32,
}

impl SchemaMigration {
    /// 构造迁移。
    pub fn new(target_version: u32) -> Self {
        Self { target_version }
    }
}
