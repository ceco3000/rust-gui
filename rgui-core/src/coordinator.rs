//! 协调器——核心循环宿主（TDD 步骤 A 实现 GREEN）。
//!
//! `Coordinator<W: WidgetSpec>` 持有**具体组件实例 + 其状态**，驱动组件完成
//! 「状态变化 → 视图更新 → 重绘」的 view→update→view 最小闭环，纯 Rust 可测。
//!
//! 职责：
//! - `new(spec, state)`：绑定组件与其初始状态。
//! - `current_view(ctx)`：获当前状态下组件渲染的视图（不改变状态）。
//! - `dispatch(msg, ctx)`：调用 `W::update` 更新状态，再返回更新后视图。
//! - `name()/state()`：读取组件名/状态。
//!
//! D3 占位（泛型空壳）已替换为本实现；`WidgetSpec` 签名保持 §B.1 不变。

use crate::context::{UpdateContext, ViewContext};
use crate::traits::WidgetSpec;
use crate::view::WidgetView;
use std::marker::PhantomData;

/// 核心循环宿主（按具体 WidgetSpec 组件驱动）。
#[derive(Debug)]
pub struct Coordinator<W: WidgetSpec> {
    spec: W,
    state: W::State,
    _marker: PhantomData<()>,
}

impl<W: WidgetSpec> Coordinator<W> {
    /// 绑定组件与其初始状态。
    pub fn new(spec: W, state: W::State) -> Self {
        Self {
            spec,
            state,
            _marker: PhantomData,
        }
    }

    /// 获取当前状态下组件渲染的视图（只读，不修改状态）。
    pub fn current_view(&self, ctx: &ViewContext) -> WidgetView<W::Message> {
        self.spec.view(&self.state, ctx)
    }

    /// 派发消息：调用 `update` 更新状态，返回更新后的视图。
    pub fn dispatch(&mut self, msg: W::Message, ctx: &mut UpdateContext) -> WidgetView<W::Message> {
        self.spec.update(msg, &mut self.state, ctx);
        self.current_view(&ViewContext::default())
    }

    /// 组件名。
    pub fn name(&self) -> &'static str {
        self.spec.name()
    }

    /// 状态引用。
    pub fn state(&self) -> &W::State {
        &self.state
    }

    /// 无参便捷视图获取（使用默认 ViewContext）。
    pub fn current_view_default(&self) -> WidgetView<W::Message> {
        self.current_view(&ViewContext::default())
    }
}
