//! # rgui-state
//!
//! rgui 状态管理——StateStore、diff 算法、快照与迁移。

pub mod diff;
pub mod harness;
pub mod snapshot;
pub mod store;

pub use diff::{Patch, WidgetIdMap, apply_patch, diff, diff_props};
pub use harness::TestHarness;
pub use snapshot::{SchemaMigration, SchemaMigrationRegistry, Snapshot, Snapshotter};
pub use store::{
    InstanceState, StateStore, StoreAccess, StoreAccessMut, Subscription, SubscriptionLifetime,
};
