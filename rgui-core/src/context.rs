//! Context 类型：`ViewContext` / `UpdateContext` / `PaintContext` / `AccessContext`。
//!
//! D3 阶段 0：最小占位定义。实际能力（状态访问、脏标记、资源句柄）在实现阶段补全。

use std::marker::PhantomData;

/// 视图构建上下文。
#[derive(Debug, Default)]
pub struct ViewContext {
    /// 当前组件是否获焦（D13 视图层焦点透传；组合根按子获焦状态设置，组件 view 读它绘制高亮）。
    pub focused: bool,
    _p: PhantomData<()>,
}

/// 更新上下文（处理消息、更新状态）。
#[derive(Debug, Default)]
pub struct UpdateContext {
    _p: PhantomData<()>,
}

impl UpdateContext {
    /// 构造空上下文。
    pub fn empty() -> Self {
        Self::default()
    }
}

/// 测量上下文（只读环境，对齐 greenfield §B.1）。
#[derive(Debug, Default)]
pub struct MeasureContext {
    _p: PhantomData<()>,
}

impl MeasureContext {
    /// 构造空测量上下文。
    pub fn empty() -> Self {
        Self::default()
    }
}

/// 绘制上下文。
#[derive(Debug, Default)]
pub struct PaintContext {
    _p: PhantomData<()>,
}

/// 无障碍上下文。
#[derive(Debug, Default)]
pub struct AccessContext {
    _p: PhantomData<()>,
}

impl AccessContext {
    /// 构造空上下文。
    pub fn empty() -> Self {
        Self::default()
    }
}
