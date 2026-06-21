//! 组件实例状态存储——为 .rgui 渲染路径提供跨帧持久状态。
//!
//! .rgui 声明式渲染中，paint_factory 每帧从 WidgetView.props 创建临时 state。
//! 对于需要响应交互的组件（如 WaAccordionItem 点击切换展开/折叠），
//! 需要跨帧持久的状态存储，使得组件能自己管理自己的行为。
//!
//! WidgetStateStore 提供：
//! - 按 WidgetId 存储任意组件状态
//! - thread-safe 读写（Arc + Mutex，适配 PaintFn Sync 约束）
//! - 支持 paint_factory 读取 + widget_instance handler 写入

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rgui_core::id::WidgetId;

/// 跨帧持久化的组件实例状态存储。
///
/// Clone 会共享同一内部 HashMap（Arc 语义）。
#[derive(Clone, Default)]
pub struct WidgetStateStore {
    inner: Arc<Mutex<HashMap<WidgetId, Box<dyn Any + Send>>>>,
}

impl WidgetStateStore {
    /// 创建空的状态存储。
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 读取指定 widget 的状态（克隆出）。
    ///
    /// 返回 `None` 如果 widget 不存在或类型不匹配。
    #[must_use]
    pub fn read<T: Send + Clone + 'static>(&self, id: WidgetId) -> Option<T> {
        let guard = self.inner.lock().expect("WidgetStateStore lock poisoned");
        guard
            .get(&id)
            .and_then(|b| b.downcast_ref::<T>())
            .cloned()
    }

    /// 通过回调更新指定 widget 的状态。
    ///
    /// 如果 widget 不存在或类型不匹配，回调不会被执行。
    pub fn update<T: Send + 'static>(&self, id: WidgetId, f: impl FnOnce(&mut T)) {
        let mut guard = self.inner.lock().expect("WidgetStateStore lock poisoned");
        if let Some(b) = guard.get_mut(&id) {
            if let Some(state) = b.downcast_mut::<T>() {
                f(state);
            }
        }
    }

    /// 读取已有状态或初始化新状态（同时写入 store）。
    ///
    /// 用于 paint_factory：首帧从 WidgetView.props 初始化状态，
    /// 后续帧直接读取已有状态。
    pub fn get_or_init<T: Send + Clone + 'static>(
        &self,
        id: WidgetId,
        init: impl FnOnce() -> T,
    ) -> T {
        let mut guard = self.inner.lock().expect("WidgetStateStore lock poisoned");
        if let Some(b) = guard.get(&id) {
            if let Some(state) = b.downcast_ref::<T>() {
                return state.clone();
            }
        }
        let state = init();
        guard.insert(id, Box::new(state.clone()));
        state
    }

    /// 插入状态（如果已存在则替换）。
    pub fn insert<T: Send + 'static>(&self, id: WidgetId, state: T) {
        let mut guard = self.inner.lock().expect("WidgetStateStore lock poisoned");
        guard.insert(id, Box::new(state));
    }

    /// 检查指定 widget 是否有状态存储。
    #[must_use]
    pub fn contains(&self, id: WidgetId) -> bool {
        let guard = self.inner.lock().expect("WidgetStateStore lock poisoned");
        guard.contains_key(&id)
    }
}
