//! 坐标变换链——hit test 交互注册的窗口坐标到局部坐标转换。
//!
//! 定义在基础层（rgui_core），供框架层和组件层共用。

use crate::geometry::Point;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CoordinateTransformStep {
    Translate { offset: Point },
}

/// 窗口绝对坐标到叶子局部坐标的可逆变换链。
///
/// 每层容器在递归初始化时追加一次坐标系平移，
/// hit test 命中时沿链反向将窗口坐标还原为叶子局部坐标。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CoordinateTransformChain {
    pub(crate) steps: Vec<CoordinateTransformStep>,
}

impl CoordinateTransformChain {
    /// 追加一个平移变换（父→子坐标系），返回新链。
    #[must_use]
    pub fn translated(&self, offset: Point) -> Self {
        let mut next = self.clone();
        next.steps
            .push(CoordinateTransformStep::Translate { offset });
        next
    }

    /// 沿链反向变换：窗口坐标 → 叶子局部坐标。
    #[must_use]
    pub fn window_to_local(&self, point: Point) -> Point {
        self.steps.iter().fold(point, |current, step| match step {
            CoordinateTransformStep::Translate { offset } => {
                Point::new(current.x - offset.x, current.y - offset.y)
            }
        })
    }

    /// 沿链正向变换：叶子局部坐标 → 窗口坐标。
    #[must_use]
    pub fn local_to_window(&self, point: Point) -> Point {
        self.steps.iter().fold(point, |current, step| match step {
            CoordinateTransformStep::Translate { offset } => {
                Point::new(current.x + offset.x, current.y + offset.y)
            }
        })
    }
}
