//! 组件状态占位模块。与 `state` 子模块（StateStore）区分：
//! `widget_state` 为单组件状态单元，`state` 为全局状态存储。

use crate::traits::PersistState;

/// 组件状态单元（D3 占位）。
#[derive(Debug, Clone, Default)]
pub struct WidgetState {
    _marker: std::marker::PhantomData<()>,
}

impl WidgetState {
    /// 构造空状态。
    pub fn new() -> Self {
        Self::default()
    }
}

// 保持 PersistState 契约导入有效（组件状态须满足它）
#[allow(dead_code)]
fn _persist_marker<S: PersistState>() {}
