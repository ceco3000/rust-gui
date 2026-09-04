//! 绘制工厂（保留模块，契约 §1.3 F）。

use rgui_core::geometry::Rect;
use rgui_core::view::Color;

/// 绘制工厂（D3 占位）。
#[derive(Debug, Default)]
pub struct PaintFactory;

impl PaintFactory {
    /// 构造占位工厂。
    pub fn new() -> Self {
        Self
    }

    /// 填充矩形。D3 占位。
    pub fn fill_rect(&mut self, _bounds: Rect, _color: Color) {
        // todo!("矩形填充在实现阶段补全")
    }
}
