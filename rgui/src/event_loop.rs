//! winit 事件循环 + 窗口创建/尺寸/关闭（契约 §3.1 模块 2）。
//! D3 阶段 0：占位骨架，不引入 winit。

use crate::render_coord::RenderCoord;

/// 事件循环编排（D3 占位）。
#[derive(Debug, Default)]
pub struct EventLoop {
    /// 渲染协调器引用。
    pub render_coord: Option<RenderCoord>,
}

impl EventLoop {
    /// 构造空事件循环。
    pub fn new() -> Self {
        Self::default()
    }

    /// 运行事件循环。D3 占位。
    pub fn run(self) {
        // todo!("winit 事件循环在实现阶段补全")
        let _ = self;
    }
}
