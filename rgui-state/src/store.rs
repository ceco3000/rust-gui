//! 状态存储——StateStore、三种状态分层、StoreAccess/StoreAccessMut。
//!
//! 定义源自 D2 §2-§4。

use rgui_core::geometry::{Point, Rect};
use rgui_core::id::{NodeHandle, WidgetId};
use rgui_core::traits::PersistState;
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt;
use std::sync::{Arc, RwLock};

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
    /// widget 在 retained tree 中的节点句柄（D2 §2.2）。
    pub node_handle: NodeHandle,
    /// 滚动偏移。
    pub scroll_offset: Point,
}

impl InstanceState {
    /// 创建默认的实例态。
    #[must_use]
    pub fn new(widget_id: WidgetId) -> Self {
        Self {
            focused: false,
            hovered: false,
            hit_test_rect: Rect::ZERO,
            node_handle: NodeHandle::new(widget_id),
            scroll_offset: Point::ZERO,
        }
    }
}

impl Default for InstanceState {
    fn default() -> Self {
        Self::new(WidgetId::new())
    }
}

// ============================================================================
// RenderLayoutCache — D2 §2.2
// ============================================================================

/// 渲染与布局缓存（D2 §2.2）。
///
/// 框架运行时持有，为每个挂载组件维护一份。缓存 Taffy 布局结果、
/// 字形 Atlas UV 坐标、路径细分数据和上次绘制颜色。
///
/// 生命周期：组件挂载 → 组件卸载。不参与序列化或快照。
#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct RenderLayoutCache {
    /// 最近一次布局结果。
    pub layout: Option<rgui_layout::LayoutResult>,

    /// 字形逐字缓存（glyph key → atlas UV + 尺寸）。
    pub glyph_cache: FxHashMap<rgui_render::GlyphKey, rgui_render::GlyphCacheEntry>,

    /// 路径细分缓存（用于复杂 SVG 路径），阶段 2 前为占位类型。
    pub path_tessellation: Option<rgui_render::PathTessellation>,

    /// 最后一次绘制使用的颜色（用于脏检测优化）。
    pub last_paint_color: Option<rgui_core::Color>,
}

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
    /// Rhai 脚本状态——key-value string 存储（RS05）。
    rhai_state: FxHashMap<WidgetId, String>,
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
            rhai_state: FxHashMap::default(),
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
        self.rhai_state.remove(&id);
        for subs in self.subscriptions.values_mut() {
            subs.retain(|s| s.subscriber != id);
        }
    }

    /// 标记 widget 为 dirty。
    pub(crate) fn mark_dirty(&mut self, id: WidgetId) {
        self.dirty.insert(id);
    }

    /// 传播 dirty 标记到所有订阅者。
    ///
    /// 使用迭代 DFS + dirty 去重，天然防御订阅图中的循环——
    /// 当节点已被标记 dirty 时，不会重复推入栈中，避免无限传播。
    pub(crate) fn propagate_dirty(&mut self, id: WidgetId) {
        let mut stack = vec![id];
        while let Some(current) = stack.pop() {
            if let Some(subscribers) = self.subscriptions.get(&current) {
                for sub in subscribers {
                    // 仅在首次成功标记 dirty 时继续传播，
                    // 这同时避免了循环图中的无限递归和重复访问
                    if self.dirty.insert(sub.subscriber) {
                        stack.push(sub.subscriber);
                    }
                }
            }
        }
    }

    /// 清除本帧脏标记。
    pub fn clear_dirty(&mut self) {
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
// StateStoreBinding — Rhai↔StateStore bridge (RS05)
// ============================================================================

/// Bridge between Rhai scripts and [`StateStore`] persistent state.
///
/// Wraps a shared `StateStore` behind `Arc<RwLock<...>>` so Rhai closures
/// can access it through the [`rgui_core::StateBinding`] trait.
///
/// # Thread safety
///
/// `StateStoreBinding` is `Send + Sync` because all access goes through
/// `RwLock`. Rhai closures (registered via `CommandRegistry`) capture
/// `Arc<dyn StateBinding>` and call `store_read`/`store_write` from
/// potentially different threads.
#[derive(Clone)]
pub struct StateStoreBinding {
    store: Arc<RwLock<StateStore>>,
}

impl StateStoreBinding {
    /// Create a new binding wrapping the given `StateStore`.
    #[must_use]
    pub fn new(store: Arc<RwLock<StateStore>>) -> Self {
        Self { store }
    }
}

impl rgui_core::StateBinding for StateStoreBinding {
    fn store_read(&self, widget_id: WidgetId) -> String {
        let store = self
            .store
            .read()
            .expect("StateStoreBinding: RwLock poisoned");
        store
            .rhai_state
            .get(&widget_id)
            .cloned()
            .unwrap_or_default()
    }

    fn store_write(&self, widget_id: WidgetId, value: &str) {
        let mut store = self
            .store
            .write()
            .expect("StateStoreBinding: RwLock poisoned");
        store.rhai_state.insert(widget_id, value.to_string());
        store.mark_dirty(widget_id);
        store.propagate_dirty(widget_id);
    }
}

impl std::fmt::Debug for StateStoreBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateStoreBinding")
            .field("store", &"<Arc<RwLock<StateStore>>>")
            .finish()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rgui_core::StateBinding;
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

    // --- apply_subscriptions 替换旧订阅 ---

    #[test]
    fn apply_subscriptions_replaces_old_subs() {
        let mut store = StateStore::new();
        let self_id = store.allocate_id();
        let target_a = store.allocate_id();
        let target_b = store.allocate_id();

        // 第一帧：B 订阅 A
        let subs_old = vec![(
            target_a,
            Subscription {
                subscriber: self_id,
                lifetime: SubscriptionLifetime::Persistent,
            },
        )];
        store.apply_subscriptions(self_id, subs_old);

        // 第二帧：B 改为订阅 C（不再订阅 A）
        let subs_new = vec![(
            target_b,
            Subscription {
                subscriber: self_id,
                lifetime: SubscriptionLifetime::Persistent,
            },
        )];
        store.apply_subscriptions(self_id, subs_new);

        // 修改 A 不应触发 B 的 dirty
        store.mark_dirty(target_a);
        store.propagate_dirty(target_a);
        assert!(
            !store.dirty_widgets().contains(&self_id),
            "旧订阅应被清除，A 的变更不应传播到 B"
        );

        // 修改 C 应触发 B 的 dirty
        store.clear_dirty();
        store.mark_dirty(target_b);
        store.propagate_dirty(target_b);
        assert!(
            store.dirty_widgets().contains(&self_id),
            "新订阅应生效，C 的变更应传播到 B"
        );
    }

    // --- cleanup_subscriptions ---

    #[test]
    fn cleanup_subscriptions_removes_stale_targets() {
        let mut store = StateStore::new();
        let subscriber_id = store.allocate_id();
        let target_id = store.allocate_id();

        // 建立订阅
        store.insert_persistent(subscriber_id, Box::new(TestCounter { count: 0 }));
        store.insert_persistent(target_id, Box::new(TestLabel { text: "x".into() }));

        let subs = vec![(
            target_id,
            Subscription {
                subscriber: subscriber_id,
                lifetime: SubscriptionLifetime::Persistent,
            },
        )];
        store.apply_subscriptions(subscriber_id, subs);

        // 移除被订阅的 target
        store.remove(target_id);

        // 清理
        store.cleanup_subscriptions();

        // 验证 target 的订阅链被清理
        assert!(!store.subscriptions.contains_key(&target_id));
    }

    #[test]
    fn cleanup_subscriptions_removes_stale_subscribers() {
        let mut store = StateStore::new();
        let subscriber_id = store.allocate_id();
        let target_id = store.allocate_id();

        // 建立订阅
        store.insert_persistent(subscriber_id, Box::new(TestCounter { count: 0 }));
        store.insert_persistent(target_id, Box::new(TestLabel { text: "x".into() }));

        let subs = vec![(
            target_id,
            Subscription {
                subscriber: subscriber_id,
                lifetime: SubscriptionLifetime::Persistent,
            },
        )];
        store.apply_subscriptions(subscriber_id, subs);

        // 移除订阅者（通过 remove 已清理订阅，但再调用 cleanup 也不应 panic）
        store.remove(subscriber_id);
        store.cleanup_subscriptions();

        // 验证 target 不再有订阅者
        if let Some(subs) = store.subscriptions.get(&target_id) {
            assert!(subs.is_empty(), "cleanup 应移除已卸载订阅者的订阅记录");
        }
    }

    // --- remove 同步清理订阅关系 ---

    #[test]
    fn remove_widget_cleans_subscription_graph() {
        let mut store = StateStore::new();
        let id_a = store.allocate_id();
        let id_b = store.allocate_id();
        let id_c = store.allocate_id();

        store.insert_persistent(id_a, Box::new(TestCounter { count: 0 }));
        store.insert_persistent(id_b, Box::new(TestCounter { count: 0 }));
        store.insert_persistent(id_c, Box::new(TestCounter { count: 0 }));

        // C 读取 A 和 B 的状态
        let subs_c = vec![
            (
                id_a,
                Subscription {
                    subscriber: id_c,
                    lifetime: SubscriptionLifetime::Persistent,
                },
            ),
            (
                id_b,
                Subscription {
                    subscriber: id_c,
                    lifetime: SubscriptionLifetime::Persistent,
                },
            ),
        ];
        store.apply_subscriptions(id_c, subs_c);

        // B 读取 A 的状态
        let subs_b = vec![(
            id_a,
            Subscription {
                subscriber: id_b,
                lifetime: SubscriptionLifetime::Persistent,
            },
        )];
        store.apply_subscriptions(id_b, subs_b);

        // 移除 C
        store.remove(id_c);

        // A 的订阅链中不应再包含 C（但 B 还在）
        if let Some(subs) = store.subscriptions.get(&id_a) {
            let has_b = subs.iter().any(|s| s.subscriber == id_b);
            let has_c = subs.iter().any(|s| s.subscriber == id_c);
            assert!(has_b, "B 的订阅应保留");
            assert!(!has_c, "C 的订阅应被移除");
        }
    }

    // --- StoreAccess（只读）订阅建立 ---

    #[test]
    fn store_access_read_establishes_subscription() {
        let mut store = StateStore::new();
        let reader_id = store.allocate_id();
        let target_id = store.allocate_id();

        store.insert_persistent(reader_id, Box::new(TestCounter { count: 0 }));
        store.insert_persistent(
            target_id,
            Box::new(TestLabel {
                text: "hello".into(),
            }),
        );

        let mut new_subs: Vec<(WidgetId, Subscription)> = Vec::new();

        {
            let mut access =
                StoreAccess::new(&store.persistent, &store.instance, reader_id, &mut new_subs);
            let label: &TestLabel = access.read(target_id).unwrap();
            assert_eq!(label.text, "hello");
        }

        assert_eq!(new_subs.len(), 1);
        assert_eq!(new_subs[0].0, target_id);
        assert_eq!(new_subs[0].1.subscriber, reader_id);
    }

    // --- Ephemeral 订阅 ---

    #[test]
    fn ephemeral_subscription_propagates_in_current_frame() {
        let mut store = StateStore::new();
        let id_a = store.allocate_id();
        let id_b = store.allocate_id();

        store.insert_persistent(id_a, Box::new(TestCounter { count: 0 }));
        store.insert_persistent(id_b, Box::new(TestCounter { count: 0 }));

        // 建立临时订阅（模拟 view() 上下文）
        let subs = vec![(
            id_a,
            Subscription {
                subscriber: id_b,
                lifetime: SubscriptionLifetime::Ephemeral,
            },
        )];
        store.apply_subscriptions(id_b, subs);

        // 即使 Ephemeral，传播机制照常工作
        store.mark_dirty(id_a);
        store.propagate_dirty(id_a);
        assert!(store.dirty_widgets().contains(&id_b));
    }

    // --- 多订阅者传播 ---

    #[test]
    fn propagate_dirty_to_multiple_subscribers() {
        let mut store = StateStore::new();
        let source = store.allocate_id();
        let sub1 = store.allocate_id();
        let sub2 = store.allocate_id();
        let sub3 = store.allocate_id();

        let subs = vec![
            (
                source,
                Subscription {
                    subscriber: sub1,
                    lifetime: SubscriptionLifetime::Persistent,
                },
            ),
            (
                source,
                Subscription {
                    subscriber: sub2,
                    lifetime: SubscriptionLifetime::Persistent,
                },
            ),
            (
                source,
                Subscription {
                    subscriber: sub3,
                    lifetime: SubscriptionLifetime::Persistent,
                },
            ),
        ];
        // 所有订阅者共享同一来源
        store.apply_subscriptions(sub1, vec![subs[0].clone()]);
        store.apply_subscriptions(sub2, vec![subs[1].clone()]);
        store.apply_subscriptions(sub3, vec![subs[2].clone()]);

        store.mark_dirty(source);
        store.propagate_dirty(source);

        assert!(store.dirty_widgets().contains(&sub1));
        assert!(store.dirty_widgets().contains(&sub2));
        assert!(store.dirty_widgets().contains(&sub3));
    }

    // --- 链式传播 ---

    #[test]
    fn propagate_dirty_through_chain() {
        let mut store = StateStore::new();
        let id_a = store.allocate_id();
        let id_b = store.allocate_id();
        let id_c = store.allocate_id();

        // B → A（B 订阅 A）
        // C → B（C 订阅 B）
        store.apply_subscriptions(
            id_b,
            vec![(
                id_a,
                Subscription {
                    subscriber: id_b,
                    lifetime: SubscriptionLifetime::Persistent,
                },
            )],
        );
        store.apply_subscriptions(
            id_c,
            vec![(
                id_b,
                Subscription {
                    subscriber: id_c,
                    lifetime: SubscriptionLifetime::Persistent,
                },
            )],
        );

        // A dirty → B dirty → C dirty（链式传播）
        store.mark_dirty(id_a);
        store.propagate_dirty(id_a);

        assert!(store.dirty_widgets().contains(&id_b));
        assert!(store.dirty_widgets().contains(&id_c));
    }

    // --- 循环防护 ---

    #[test]
    fn propagate_dirty_with_cycle_does_not_overflow() {
        let mut store = StateStore::new();
        let id_a = store.allocate_id();
        let id_b = store.allocate_id();

        // 构造 A → B → A 循环
        store.apply_subscriptions(
            id_b,
            vec![(
                id_a,
                Subscription {
                    subscriber: id_b,
                    lifetime: SubscriptionLifetime::Persistent,
                },
            )],
        );
        store.apply_subscriptions(
            id_a,
            vec![(
                id_b,
                Subscription {
                    subscriber: id_a,
                    lifetime: SubscriptionLifetime::Persistent,
                },
            )],
        );

        // 即使存在循环，传播也应正常终止（不栈溢出）
        store.mark_dirty(id_a);
        store.propagate_dirty(id_a);

        // 循环中的所有节点均应标记 dirty
        assert!(store.dirty_widgets().contains(&id_a));
        assert!(store.dirty_widgets().contains(&id_b));
    }

    // --- RenderLayoutCache ---

    #[test]
    fn render_layout_cache_default_is_empty() {
        let cache = RenderLayoutCache::default();
        assert!(cache.layout.is_none());
        assert!(cache.glyph_cache.is_empty());
        assert!(cache.path_tessellation.is_none());
        assert!(cache.last_paint_color.is_none());
    }

    #[test]
    fn render_layout_cache_path_tessellation_field() {
        let mut cache = RenderLayoutCache::default();
        let pt = rgui_render::PathTessellation::default();
        cache.path_tessellation = Some(pt.clone());
        assert!(cache.path_tessellation.is_some());
        assert_eq!(
            format!("{:?}", cache.path_tessellation.as_ref().unwrap()),
            format!("{:?}", pt)
        );
    }

    #[test]
    fn render_layout_cache_layout_field() {
        let mut cache = RenderLayoutCache::default();
        let layout = rgui_layout::LayoutResult {
            size: rgui_core::Size::new(100.0, 50.0),
            position: rgui_core::Point::new(10.0, 20.0),
        };
        cache.layout = Some(layout.clone());
        assert!(cache.layout.is_some());
        let l = cache.layout.as_ref().unwrap();
        assert_eq!(l.size.width, 100.0);
        assert_eq!(l.size.height, 50.0);
        assert_eq!(l.position.x, 10.0);
        assert_eq!(l.position.y, 20.0);
    }

    #[test]
    fn render_layout_cache_last_paint_color_field() {
        let mut cache = RenderLayoutCache::default();
        let color = rgui_core::Color::new(1.0, 0.0, 0.0, 1.0);
        cache.last_paint_color = Some(color.clone());
        assert!(cache.last_paint_color.is_some());
        assert_eq!(cache.last_paint_color.as_ref().unwrap(), &color);
    }

    // --- StateStoreBinding（RS05）---

    #[test]
    fn state_store_binding_write_then_read() {
        let store = Arc::new(RwLock::new(StateStore::new()));
        let id = {
            let mut s = store.write().unwrap();
            s.allocate_id()
        };
        let binding = StateStoreBinding::new(Arc::clone(&store));

        binding.store_write(id, "hello rhai");
        let result = binding.store_read(id);
        assert_eq!(result, "hello rhai");
    }

    #[test]
    fn state_store_binding_read_unknown_widget_returns_empty() {
        let store = Arc::new(RwLock::new(StateStore::new()));
        let binding = StateStoreBinding::new(store);
        let unknown = WidgetId::new();

        let result = binding.store_read(unknown);
        assert_eq!(result, "");
    }

    #[test]
    fn state_store_binding_write_overwrites_previous_value() {
        let store = Arc::new(RwLock::new(StateStore::new()));
        let id = {
            let mut s = store.write().unwrap();
            s.allocate_id()
        };
        let binding = StateStoreBinding::new(Arc::clone(&store));

        binding.store_write(id, "first");
        binding.store_write(id, "second");
        assert_eq!(binding.store_read(id), "second");
    }

    #[test]
    fn state_store_binding_write_marks_widget_dirty() {
        let store = Arc::new(RwLock::new(StateStore::new()));
        let id = {
            let mut s = store.write().unwrap();
            let wid = s.allocate_id();
            s.insert_persistent(wid, Box::new(TestCounter { count: 0 }));
            wid
        };
        let binding = StateStoreBinding::new(Arc::clone(&store));

        binding.store_write(id, "some value");

        let dirty = {
            let s = store.read().unwrap();
            s.dirty_widgets().contains(&id)
        };
        assert!(dirty, "widget should be marked dirty after store_write");
    }

    #[test]
    fn state_store_binding_write_propagates_dirty_to_subscribers() {
        let mut store = StateStore::new();
        let id_a = store.allocate_id();
        store.insert_persistent(id_a, Box::new(TestCounter { count: 0 }));
        let id_b = store.allocate_id();
        store.insert_persistent(id_b, Box::new(TestCounter { count: 0 }));

        // B subscribes to A
        store.apply_subscriptions(
            id_b,
            vec![(
                id_a,
                Subscription {
                    subscriber: id_b,
                    lifetime: SubscriptionLifetime::Persistent,
                },
            )],
        );

        let store = Arc::new(RwLock::new(store));
        let binding = StateStoreBinding::new(Arc::clone(&store));

        binding.store_write(id_a, "changed");

        let s = store.read().unwrap();
        assert!(s.dirty_widgets().contains(&id_a), "source should be dirty");
        assert!(
            s.dirty_widgets().contains(&id_b),
            "subscriber should be dirty via propagation"
        );
    }

    #[test]
    fn state_store_binding_clone_shares_state() {
        let store = Arc::new(RwLock::new(StateStore::new()));
        let id = {
            let mut s = store.write().unwrap();
            s.allocate_id()
        };
        let binding1 = StateStoreBinding::new(Arc::clone(&store));
        let binding2 = binding1.clone();

        binding1.store_write(id, "shared");
        assert_eq!(binding2.store_read(id), "shared");
    }

    // --- RS06: propagate_dirty cycle safety ---

    #[test]
    fn propagate_dirty_handles_cycles_without_infinite_loop() {
        let mut store = StateStore::new();
        let id_a = store.allocate_id();
        let id_b = store.allocate_id();
        let id_c = store.allocate_id();

        // A → B → C → A (cycle)
        store.insert_persistent(id_a, Box::new(TestCounter { count: 0 }));
        store.insert_persistent(id_b, Box::new(TestCounter { count: 0 }));
        store.insert_persistent(id_c, Box::new(TestCounter { count: 0 }));

        store.apply_subscriptions(
            id_a,
            vec![(
                id_b,
                Subscription {
                    subscriber: id_a,
                    lifetime: SubscriptionLifetime::Persistent,
                },
            )],
        );
        store.apply_subscriptions(
            id_b,
            vec![(
                id_c,
                Subscription {
                    subscriber: id_b,
                    lifetime: SubscriptionLifetime::Persistent,
                },
            )],
        );
        store.apply_subscriptions(
            id_c,
            vec![(
                id_a,
                Subscription {
                    subscriber: id_c,
                    lifetime: SubscriptionLifetime::Persistent,
                },
            )],
        );

        // 验证确实检测到循环
        let cycles = store.detect_cycles();
        assert!(!cycles.is_empty(), "should detect cycle");

        // propagate_dirty 不应无限循环——通过 dirty 去重保证
        store.mark_dirty(id_a);
        store.propagate_dirty(id_a);

        // A、B、C 都应在脏集合中
        assert!(store.dirty_widgets().contains(&id_a));
        assert!(store.dirty_widgets().contains(&id_b));
        assert!(store.dirty_widgets().contains(&id_c));

        // 没有重复标记——每个 widget 只出现一次
        assert_eq!(store.dirty_widgets().len(), 3);

        // 清除后正常
        store.clear_dirty();
        assert!(store.dirty_widgets().is_empty());
    }
}
