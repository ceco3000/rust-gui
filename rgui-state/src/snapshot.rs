//! 快照与迁移协议（D2 §7-§8）。
//!
//! 快照捕获所有持久状态的某时刻副本。

use rgui_core::id::WidgetId;
use rgui_core::traits::PersistState;
use rustc_hash::FxHashMap;
use std::fmt;

// ============================================================================
// Snapshot
// ============================================================================

/// 序列化后的单个 widget 状态（D2 §7.1）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SerializedState {
    pub schema_name: String,
    pub schema_version: u32,
    pub data: Vec<u8>,
}

/// 状态快照（D2 §7.1）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    pub framework_version: String,
    pub sequence: u64,
    pub states: FxHashMap<u64, SerializedState>,
}

impl Snapshot {
    #[must_use]
    pub fn new(framework_version: impl Into<String>, sequence: u64) -> Self {
        Self {
            framework_version: framework_version.into(),
            sequence,
            states: FxHashMap::default(),
        }
    }
}

// ============================================================================
// Snapshotter
// ============================================================================

/// 快照管理器（D2 §7.2）。
pub struct Snapshotter {
    snapshots: Vec<Snapshot>,
    max_snapshots: usize,
    next_sequence: u64,
}

impl Snapshotter {
    #[must_use]
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: Vec::with_capacity(max_snapshots),
            max_snapshots,
            next_sequence: 0,
        }
    }

    /// 从 StateStore 创建快照。
    ///
    /// 当前为最小实现——实际序列化将在 postcard + erased-serde
    /// 集成完善后进行完整实现。
    pub fn capture(
        &mut self,
        persistent: &FxHashMap<WidgetId, Box<dyn PersistState>>,
        framework_version: &str,
    ) -> Snapshot {
        let mut states = FxHashMap::default();

        for &_id in persistent.keys() {
            // TODO: 使用 postcard + erased_serde 完成序列化
            // 当前暂存空数据
            states.insert(
                _id.as_u64(),
                SerializedState {
                    schema_name: "unknown".to_string(),
                    schema_version: 0,
                    data: Vec::new(),
                },
            );
        }

        let snapshot = Snapshot {
            framework_version: framework_version.to_string(),
            sequence: self.next_sequence,
            states,
        };

        self.next_sequence += 1;

        if self.snapshots.len() >= self.max_snapshots {
            self.snapshots.remove(0);
        }
        self.snapshots.push(snapshot.clone());

        snapshot
    }

    #[must_use]
    pub fn get(&self, sequence: u64) -> Option<&Snapshot> {
        self.snapshots.iter().find(|s| s.sequence == sequence)
    }

    #[must_use]
    pub fn sequences(&self) -> Vec<u64> {
        self.snapshots.iter().map(|s| s.sequence).collect()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }
}

impl fmt::Debug for Snapshotter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Snapshotter")
            .field("count", &self.snapshots.len())
            .field("next_sequence", &self.next_sequence)
            .finish()
    }
}

// ============================================================================
// SchemaMigration
// ============================================================================

/// Schema 迁移 trait（D2 §7.3）。
pub trait SchemaMigration: Send + Sync {
    fn schema_name(&self) -> &str;
    fn source_version(&self) -> u32;
    fn target_version(&self) -> u32;
    fn migrate(&self, data: &[u8]) -> Result<Vec<u8>, MigrationError>;
}

/// 迁移错误。
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("找不到从版本 {from} 到 {to} 的迁移路径（schema: {schema}）")]
    NoPath { schema: String, from: u32, to: u32 },
    #[error("迁移执行失败：{0}")]
    ExecutionFailed(String),
}

/// Schema 迁移注册表（D2 §7.3）。
#[derive(Default)]
pub struct SchemaMigrationRegistry {
    migrations: FxHashMap<String, Vec<Box<dyn SchemaMigration>>>,
}

impl SchemaMigrationRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            migrations: FxHashMap::default(),
        }
    }

    pub fn register<M: SchemaMigration + 'static>(&mut self, migration: M) {
        self.migrations
            .entry(migration.schema_name().to_string())
            .or_default()
            .push(Box::new(migration));
    }

    #[must_use]
    pub fn current_version(&self, schema_name: &str) -> u32 {
        self.migrations
            .get(schema_name)
            .and_then(|ms| ms.iter().map(|m| m.target_version()).max())
            .unwrap_or(1)
    }

    pub fn migrate(
        &self,
        schema_name: &str,
        from_version: u32,
        data: &[u8],
    ) -> Result<Vec<u8>, MigrationError> {
        let migrations =
            self.migrations
                .get(schema_name)
                .ok_or_else(|| MigrationError::NoPath {
                    schema: schema_name.to_string(),
                    from: from_version,
                    to: self.current_version(schema_name),
                })?;

        let mut current_version = from_version;
        let mut current_data = data.to_vec();

        while current_version < self.current_version(schema_name) {
            let next = migrations
                .iter()
                .find(|m| m.source_version() == current_version)
                .ok_or_else(|| MigrationError::NoPath {
                    schema: schema_name.to_string(),
                    from: current_version,
                    to: self.current_version(schema_name),
                })?;

            current_data = next.migrate(&current_data)?;
            current_version = next.target_version();
        }

        Ok(current_data)
    }
}

impl fmt::Debug for SchemaMigrationRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SchemaMigrationRegistry")
            .field("schemas", &self.migrations.len())
            .finish()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_new() {
        let snap = Snapshot::new("0.1.0", 0);
        assert_eq!(snap.sequence, 0);
        assert!(snap.states.is_empty());
    }

    #[test]
    fn snapshotter_new_is_empty() {
        let s = Snapshotter::new(10);
        assert!(s.is_empty());
    }

    #[test]
    fn migration_registry_default_version() {
        let registry = SchemaMigrationRegistry::new();
        assert_eq!(registry.current_version("unknown"), 1);
    }
}
