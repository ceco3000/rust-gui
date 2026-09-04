//! 状态管理子模块（由 `rgui-state` 迁入，greenfield §B.1 / §C.1）。
//!
//! ## 设计约束（M1 教训制度化）
//!
//! `StateStore`/`Patch`/`Snapshot` 的**所有字段类型全部来自 `rgui-core` 自身**
//! （WidgetId/PropValue/Size），**绝不**含 `GlyphKey`/`PathTessellation`/`LayoutResult`。
//! 零 GPU / 零平台 / 零 `rgui-render` / `rgui-layout` 依赖（Cargo 依赖防火墙强保证）。
//!
//! D4 阶段：实现 `diff`/`apply_patch`（差分）+ `Snapshot`/`Snapshotter`（快照）。
//! 状态层为纯 Rust 数据层——变更数据层类型**不触发 render 重编**（硬约束 E）。

pub mod diff;
pub mod snapshot;

use crate::id::WidgetId;
use crate::traits::PersistState;
use std::any::Any;

pub use diff::{apply_patch, diff, Patch};
pub use snapshot::{SchemaMigration, Snapshot, Snapshotter};

/// 订阅生命周期。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SubscriptionLifetime {
    /// 持久订阅。
    #[default]
    Persistent,
    /// 瞬态订阅。
    Transient,
}

/// 状态存储（每 Widget 一份 InstanceState；订阅表；dirty 集）。
///
/// 泛型 `S` 为组件状态泛型。D4 实现最小宿主：持有状态 + dirty 标记 + 订阅集合。
#[derive(Debug, Clone)]
pub struct StateStore<S = InstanceState> {
    state: InstanceState,
    dirty: bool,
    subscriptions: Vec<Subscription>,
    _marker: std::marker::PhantomData<S>,
}

impl<S> StateStore<S> {
    /// 构造空状态存储。
    pub fn new() -> Self {
        Self {
            state: InstanceState::default(),
            dirty: false,
            subscriptions: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }

    /// 标记为 dirty（等待重绘）。
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// 是否 dirty。
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// 添加订阅。
    pub fn add_subscription(&mut self, sub: Subscription) {
        self.subscriptions.push(sub);
    }

    /// 订阅列表。
    pub fn subscriptions(&self) -> &[Subscription] {
        &self.subscriptions
    }
}

impl<S> Default for StateStore<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// 实例状态（组件 State 关联类型须满足 PersistState）。
#[derive(Debug, Clone, Default)]
pub struct InstanceState(#[allow(dead_code)] std::marker::PhantomData<()>);

impl PersistState for InstanceState {
    fn schema_name() -> &'static str {
        "instance_state"
    }
    fn schema_version() -> u32 {
        0
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// 订阅（subscriber 关注 target）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Subscription {
    /// 订阅方组件。
    pub subscriber: WidgetId,
    /// 被订阅组件。
    pub target: WidgetId,
    /// 生命周期。
    pub lifetime: SubscriptionLifetime,
}

impl Subscription {
    /// 构造订阅。
    pub const fn new(subscriber: WidgetId, target: WidgetId) -> Self {
        Self {
            subscriber,
            target,
            lifetime: SubscriptionLifetime::Persistent,
        }
    }
}

/// 状态绑定（`Arc<RwLock<StateStore>>` 封装，D4 最小占位）。
#[derive(Debug, Clone, Default)]
pub struct StoreBinding {
    _marker: std::marker::PhantomData<()>,
}
