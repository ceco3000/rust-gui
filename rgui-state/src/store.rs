//! 状态存储——StateStore、三种状态分层、StoreAccess/StoreAccessMut。
//!
//! 定义源自 D2 §2-§4。

use rgui_core::geometry::{Point, Rect};
use rgui_core::id::WidgetId;
use rgui_core::traits::PersistState;
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt;

// ============================================================================
// InstanceState
// ============================================================================

/// 实例态：框架运行时持有的 per-widget 交互状态（D2 §2.2）。
#[derive(Debug, Clone)]
pub struct InstanceState {
    /// 当前是否有键盘焦点。
    pub focused: bool,
    /// 当前是否被鼠标悬停。
    pub hovered: bool,
    /// 命中测试矩形（窗口坐标）。
    pub hit_test_rect: Rect,
    /// 滚动偏移。
    pub scroll_offset: Point,
}

impl InstanceState {
    /// 创建默认的实例态。
    #[must_use]
    pub fn new() -> Self {
        Self {
            focused: false,
            hovered: false,
            hit_test_rect: Rect::ZERO,
            scroll_offset: Point::ZERO,
        }
    }
}

impl Default for InstanceState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// RenderLayoutCache 占位类型
// ============================================================================

/// 渲染与布局缓存（D2 §2.2）。
///
/// 占位类型——详细字段（Taffy 布局结果、字形 Atlas UV 等）
/// 在 rgui-render 实现时补充。
#[derive(Debug, Default)]
pub struct RenderLayoutCache;

// ============================================================================
// Subscription 和 SubscriptionLifetime
// ============================================================================

/// 订阅记录（D2 §3.1）。
#[derive(Debug, Clone)]
pub struct Subscription {
    /// 读取者（订阅者）的 WidgetId。
    pub subscriber: WidgetId,
    /// 订阅生命周期类型。
    pub lifetime: SubscriptionLifetime,
}

/// 订阅生命周期（D2 §3.1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionLifetime {
    /// 持久订阅：跨帧保持。
    Persistent,
    /// 临时订阅：仅本帧有效。
    Ephemeral,
}

// ============================================================================
// StateStore
// ============================================================================

/// 全局状态存储（D2 §3）。
///
/// 三类状态严格分离：
/// - `persistent`：可快照、可迁移的业务状态
/// - `instance`：交互状态，仅框架内部访问
/// - `caches`：渲染/布局缓存，不参与快照
#[allow(dead_code)]
pub struct StateStore {
    /// 持久业务状态。
    persistent: FxHashMap<WidgetId, Box<dyn PersistState>>,
    /// 实例态。
    instance: FxHashMap<WidgetId, InstanceState>,
    /// 渲染与布局缓存。
    caches: FxHashMap<WidgetId, RenderLayoutCache>,
    /// 脏标记集合。
    dirty: FxHashSet<WidgetId>,
    /// 订阅关系：被读取者 → 读取者列表。
    subscriptions: FxHashMap<WidgetId, Vec<Subscription>>,
    /// WidgetId 分配器（单调递增）。
    next_id: u64,
}

#[allow(dead_code)]
impl StateStore {
    /// 创建空的 StateStore。
    #[must_use]
    pub fn new() -> Self {
        Self {
            persistent: FxHashMap::default(),
            instance: FxHashMap::default(),
            caches: FxHashMap::default(),
            dirty: FxHashSet::default(),
            subscriptions: FxHashMap::default(),
            next_id: 0,
        }
    }

    /// 分配新的 WidgetId（D2 §3.2）。
    pub(crate) fn allocate_id(&mut self) -> WidgetId {
        self.next_id += 1;
        WidgetId::from_u64(self.next_id)
    }

    /// 插入持久状态。
    pub(crate) fn insert_persistent(&mut self, id: WidgetId, state: Box<dyn PersistState>) {
        debug_assert!(
            !self.persistent.contains_key(&id),
            "WidgetId {id:?} 已存在，不可重复插入"
        );
        self.persistent.insert(id, state);
    }

    /// 移除 widget 的所有状态。
    pub(crate) fn remove(&mut self, id: WidgetId) {
        self.persistent.remove(&id);
        self.instance.remove(&id);
        self.caches.remove(&id);
        self.dirty.remove(&id);
        self.subscriptions.remove(&id);
        for subs in self.subscriptions.values_mut() {
            subs.retain(|s| s.subscriber != id);
        }
    }

    /// 标记 widget 为 dirty。
    pub(crate) fn mark_dirty(&mut self, id: WidgetId) {
        self.dirty.insert(id);
    }

    /// 传播 dirty 标记到所有订阅者（递归）。
    pub(crate) fn propagate_dirty(&mut self, id: WidgetId) {
        if let Some(subscribers) = self.subscriptions.get(&id).cloned() {
            for sub in &subscribers {
                self.dirty.insert(sub.subscriber);
                self.propagate_dirty(sub.subscriber);
            }
        }
    }

    /// 清除本帧脏标记。
    pub(crate) fn clear_dirty(&mut self) {
        self.dirty.clear();
    }

    /// 获取当前脏标记集合（只读）。
    #[must_use]
    pub fn dirty_widgets(&self) -> &FxHashSet<WidgetId> {
        &self.dirty
    }

    /// 返回注册的 widget 数量。
    #[must_use]
    pub fn widget_count(&self) -> usize {
        self.persistent.len()
    }

    /// 是否包含指定 widget。
    #[must_use]
    pub fn contains(&self, id: WidgetId) -> bool {
        self.persistent.contains_key(&id)
    }

    // --- 订阅管理 ---

    /// 应用新订阅关系（D2 §6.2）。
    pub(crate) fn apply_subscriptions(
        &mut self,
        self_id: WidgetId,
        new_subs: Vec<(WidgetId, Subscription)>,
    ) {
        for subs in self.subscriptions.values_mut() {
            subs.retain(|s| s.subscriber != self_id);
        }
        for (target_id, sub) in new_subs {
            self.subscriptions.entry(target_id).or_default().push(sub);
        }
    }

    /// 清理无效订阅。
    pub(crate) fn cleanup_subscriptions(&mut self) {
        self.subscriptions
            .retain(|target_id, _| self.persistent.contains_key(target_id));
        for subs in self.subscriptions.values_mut() {
            subs.retain(|s| self.persistent.contains_key(&s.subscriber));
        }
    }

    /// 检测订阅图中的循环依赖（D2 §6.3）。
    #[must_use]
    pub fn detect_cycles(&self) -> Vec<Vec<WidgetId>> {
        let mut cycles = Vec::new();
        let mut visited = FxHashSet::default();
        let mut in_stack = FxHashSet::default();

        for &widget_id in self.subscriptions.keys() {
            if !visited.contains(&widget_id) {
                let mut path = Vec::new();
                self._dfs_detect_cycle(
                    widget_id,
                    &mut visited,
                    &mut in_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }
        cycles
    }

    fn _dfs_detect_cycle(
        &self,
        current: WidgetId,
        visited: &mut FxHashSet<WidgetId>,
        in_stack: &mut FxHashSet<WidgetId>,
        path: &mut Vec<WidgetId>,
        cycles: &mut Vec<Vec<WidgetId>>,
    ) {
        visited.insert(current);
        in_stack.insert(current);
        path.push(current);

        if let Some(subs) = self.subscriptions.get(&current) {
            for sub in subs {
                if in_stack.contains(&sub.subscriber) {
                    if let Some(pos) = path.iter().position(|&id| id == sub.subscriber) {
                        cycles.push(path[pos..].to_vec());
                    }
                } else if !visited.contains(&sub.subscriber) {
                    self._dfs_detect_cycle(sub.subscriber, visited, in_stack, path, cycles);
                }
            }
        }

        path.pop();
        in_stack.remove(&current);
    }
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for StateStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateStore")
            .field("widgets", &self.persistent.len())
            .field("dirty_count", &self.dirty.len())
            .field("subscription_count", &self.subscriptions.len())
            .finish()
    }
}

// ============================================================================
// StoreAccess
// ============================================================================

/// 只读状态访问句柄（D2 §4.1）。
#[allow(dead_code)]
pub struct StoreAccess<'a> {
    persistent: &'a FxHashMap<WidgetId, Box<dyn PersistState>>,
    instance: &'a FxHashMap<WidgetId, InstanceState>,
    current_reader: WidgetId,
    new_subscriptions: &'a mut Vec<(WidgetId, Subscription)>,
}

#[allow(dead_code)]
impl<'a> StoreAccess<'a> {
    /// 创建只读访问句柄（仅框架内部使用）。
    pub(crate) fn new(
        persistent: &'a FxHashMap<WidgetId, Box<dyn PersistState>>,
        instance: &'a FxHashMap<WidgetId, InstanceState>,
        current_reader: WidgetId,
        new_subscriptions: &'a mut Vec<(WidgetId, Subscription)>,
    ) -> Self {
        Self {
            persistent,
            instance,
            current_reader,
            new_subscriptions,
        }
    }

    /// 读取自身的持久状态。
    ///
    /// # Panics
    ///
    /// 如果类型不匹配（表示框架内部错误）。
    #[must_use]
    pub fn state<T: PersistState>(&self, self_id: WidgetId) -> &T {
        self.persistent
            .get(&self_id)
            .and_then(|boxed| (**boxed).as_any().downcast_ref::<T>())
            .expect("StateStore: 持久状态类型不匹配——框架内部错误")
    }

    /// 读取其他 widget 的持久状态，自动建立订阅。
    pub fn read<T: PersistState>(&mut self, target_id: WidgetId) -> Option<&T> {
        let state = self
            .persistent
            .get(&target_id)
            .and_then(|boxed| (**boxed).as_any().downcast_ref::<T>())?;

        self.new_subscriptions.push((
            target_id,
            Subscription {
                subscriber: self.current_reader,
                lifetime: SubscriptionLifetime::Persistent,
            },
        ));

        Some(state)
    }

    /// 读取实例态。
    #[must_use]
    pub(crate) fn instance_state(&self, id: WidgetId) -> Option<&InstanceState> {
        self.instance.get(&id)
    }
}

impl fmt::Debug for StoreAccess<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoreAccess")
            .field("current_reader", &self.current_reader)
            .finish()
    }
}

// ============================================================================
// StoreAccessMut
// ============================================================================

/// 可变状态访问句柄（D2 §4.2）。
///
/// 在 `update()` 中传递给组件。只能修改自身状态。
#[allow(dead_code)]
pub struct StoreAccessMut<'a> {
    persistent: &'a mut FxHashMap<WidgetId, Box<dyn PersistState>>,
    self_id: WidgetId,
    new_subscriptions: Vec<(WidgetId, Subscription)>,
}

#[allow(dead_code)]
impl<'a> StoreAccessMut<'a> {
    /// 创建可变访问句柄。
    pub(crate) fn new(
        persistent: &'a mut FxHashMap<WidgetId, Box<dyn PersistState>>,
        self_id: WidgetId,
    ) -> Self {
        Self {
            persistent,
            self_id,
            new_subscriptions: Vec::new(),
        }
    }

    /// 获取自身的可变持久状态。
    ///
    /// 调用者应在此之后标记 self_id 为 dirty。
    ///
    /// # Panics
    ///
    /// 如果类型不匹配（框架内部错误）。
    #[must_use]
    pub fn state_mut<T: PersistState>(&mut self) -> &mut T {
        self.persistent
            .get_mut(&self.self_id)
            .and_then(|boxed| (**boxed).as_any_mut().downcast_mut::<T>())
            .expect("StoreAccessMut: 持久状态类型不匹配——框架内部错误")
    }

    /// 读取其他 widget 的持久状态（只读），自动建立订阅。
    pub fn read<T: PersistState>(&mut self, target_id: WidgetId) -> Option<&T> {
        let state = self
            .persistent
            .get(&target_id)
            .and_then(|boxed| (**boxed).as_any().downcast_ref::<T>())?;

        self.new_subscriptions.push((
            target_id,
            Subscription {
                subscriber: self.self_id,
                lifetime: SubscriptionLifetime::Persistent,
            },
        ));

        Some(state)
    }

    /// 消费句柄，返回本帧新建立的订阅列表。
    pub(crate) fn finish(self) -> Vec<(WidgetId, Subscription)> {
        self.new_subscriptions
    }
}

impl fmt::Debug for StoreAccessMut<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoreAccessMut")
            .field("self_id", &self.self_id)
            .finish()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;

    // --- 测试用类型 ---

    #[derive(Debug, Clone, PartialEq, serde::Serialize)]
    struct TestCounter {
        count: i32,
    }

    impl PersistState for TestCounter {
        fn schema_name() -> &'static str {
            "test_counter"
        }
        fn schema_version() -> u32 {
            1
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[derive(Debug, Clone, PartialEq, serde::Serialize)]
    struct TestLabel {
        text: String,
    }

    impl PersistState for TestLabel {
        fn schema_name() -> &'static str {
            "test_label"
        }
        fn schema_version() -> u32 {
            1
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    // --- StateStore 基本操作 ---

    #[test]
    fn store_new_is_empty() {
        let store = StateStore::new();
        assert_eq!(store.widget_count(), 0);
    }

    #[test]
    fn store_allocate_id_unique() {
        let mut store = StateStore::new();
        let id1 = store.allocate_id();
        let id2 = store.allocate_id();
        assert_ne!(id1, id2);
        assert!(id1.as_u64() < id2.as_u64());
    }

    #[test]
    fn store_insert_and_check() {
        let mut store = StateStore::new();
        let id = store.allocate_id();
        store.insert_persistent(id, Box::new(TestCounter { count: 0 }));
        assert!(store.contains(id));
        assert_eq!(store.widget_count(), 1);
    }

    #[test]
    fn store_remove_cleanup() {
        let mut store = StateStore::new();
        let id = store.allocate_id();
        store.insert_persistent(id, Box::new(TestCounter { count: 0 }));
        store.remove(id);
        assert!(!store.contains(id));
        assert_eq!(store.widget_count(), 0);
    }

    // --- Dirty 标记 ---

    #[test]
    fn store_mark_dirty() {
        let mut store = StateStore::new();
        let id = store.allocate_id();
        store.mark_dirty(id);
        assert!(store.dirty_widgets().contains(&id));
    }

    #[test]
    fn store_clear_dirty() {
        let mut store = StateStore::new();
        let id = store.allocate_id();
        store.mark_dirty(id);
        store.clear_dirty();
        assert!(store.dirty_widgets().is_empty());
    }

    // --- StoreAccessMut ---

    #[test]
    fn access_mut_read_self_state() {
        let mut store = StateStore::new();
        let id = store.allocate_id();
        store.insert_persistent(id, Box::new(TestCounter { count: 42 }));

        let mut access = StoreAccessMut::new(&mut store.persistent, id);
        let state: &mut TestCounter = access.state_mut();
        assert_eq!(state.count, 42);
    }

    #[test]
    fn access_mut_modify_self_state() {
        let mut store = StateStore::new();
        let id = store.allocate_id();
        store.insert_persistent(id, Box::new(TestCounter { count: 0 }));

        {
            let mut access = StoreAccessMut::new(&mut store.persistent, id);
            let state: &mut TestCounter = access.state_mut();
            state.count = 100;
        }

        // 验证修改
        if let Some(boxed) = store.persistent.get(&id) {
            let state = (**boxed).as_any().downcast_ref::<TestCounter>().unwrap();
            assert_eq!(state.count, 100);
        }
    }

    #[test]
    fn access_mut_read_other_state() {
        let mut store = StateStore::new();
        let self_id = store.allocate_id();
        let other_id = store.allocate_id();

        store.insert_persistent(self_id, Box::new(TestCounter { count: 0 }));
        store.insert_persistent(
            other_id,
            Box::new(TestLabel {
                text: "hello".into(),
            }),
        );

        let mut access = StoreAccessMut::new(&mut store.persistent, self_id);
        let label: &TestLabel = access.read(other_id).unwrap();
        assert_eq!(label.text, "hello");
        let subs = access.finish();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].0, other_id);
    }

    // --- 订阅传播 ---

    #[test]
    fn propagate_dirty_to_subscriber() {
        let mut store = StateStore::new();
        let id_a = store.allocate_id();
        let id_b = store.allocate_id();

        // B 读取了 A 的状态 → B 是 A 的订阅者
        let subs = vec![(
            id_a,
            Subscription {
                subscriber: id_b,
                lifetime: SubscriptionLifetime::Persistent,
            },
        )];
        store.apply_subscriptions(id_b, subs);

        store.mark_dirty(id_a);
        store.propagate_dirty(id_a);

        assert!(store.dirty_widgets().contains(&id_b));
    }

    // --- 循环检测 ---

    #[test]
    fn detect_cycles_finds_cycle() {
        let mut store = StateStore::new();
        let id_a = store.allocate_id();
        let id_b = store.allocate_id();

        let subs_a = vec![(
            id_b,
            Subscription {
                subscriber: id_a,
                lifetime: SubscriptionLifetime::Persistent,
            },
        )];
        store.apply_subscriptions(id_a, subs_a);

        let subs_b = vec![(
            id_a,
            Subscription {
                subscriber: id_b,
                lifetime: SubscriptionLifetime::Persistent,
            },
        )];
        store.apply_subscriptions(id_b, subs_b);

        let cycles = store.detect_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn detect_cycles_no_cycle() {
        let mut store = StateStore::new();
        let id_a = store.allocate_id();
        let id_b = store.allocate_id();

        let subs = vec![(
            id_a,
            Subscription {
                subscriber: id_b,
                lifetime: SubscriptionLifetime::Persistent,
            },
        )];
        store.apply_subscriptions(id_b, subs);

        let cycles = store.detect_cycles();
        assert!(cycles.is_empty());
    }
}
