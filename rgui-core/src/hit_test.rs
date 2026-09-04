//! 命中检测（hit-test）——多组件事件路由基础（D11）。
//!
//! 纯 Rust 几何：给定点击坐标，在若干已布局区域中命中第一个包含该点的区域，
//! 返回其 `id`，上层据此路由到对应组件的消息。
//!
//! 零 GPU/平台依赖（core 允许）。流式实现：`iter().find()`。

use crate::geometry::Rect;

/// 可命中区域（已布局的组件区）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitRegion {
    /// 区域矩形（窗口逻辑坐标）。
    pub rect: Rect,
    /// 区域标识（上层关联组件/消息）。
    pub id: u32,
}

impl HitRegion {
    /// 构造可命中区域。
    pub const fn new(rect: Rect, id: u32) -> Self {
        Self { rect, id }
    }

    /// 判断点 (x, y) 是否落在区域内（含边界）。
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.rect.x && x < self.rect.right() && y >= self.rect.y && y < self.rect.bottom()
    }
}

/// 命中检测：返回第一个包含点 (x, y) 的区域 id；无命中返回 `None`。
///
/// 按 `regions` 顺序取第一个命中（上层可据此做焦点/层级优先级；简单场景为遍历序）。
/// 流式：`iter().find()`（无手写循环 / 无中间 collect）。
pub fn hit_test(x: f32, y: f32, regions: &[HitRegion]) -> Option<u32> {
    regions.iter().find(|r| r.contains(x, y)).map(|r| r.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regions() -> Vec<HitRegion> {
        vec![
            HitRegion::new(Rect::new(0.0, 0.0, 340.0, 44.0), 1), // Accordion 标题区
            HitRegion::new(Rect::new(0.0, 110.0, 160.0, 40.0), 2), // WaBadge 区
        ]
    }

    #[test]
    fn hit_test_hits_first_region() {
        // 点落在 Accordion 标题区内
        assert_eq!(hit_test(50.0, 20.0, &regions()), Some(1));
    }

    #[test]
    fn hit_test_hits_second_region() {
        // 点落在 WaBadge 区内
        assert_eq!(hit_test(30.0, 120.0, &regions()), Some(2));
    }

    #[test]
    fn hit_test_misses_gap_between_regions() {
        // 落在两区域之间的空白（y=70 不在任何区域）
        assert_eq!(hit_test(50.0, 70.0, &regions()), None);
    }

    #[test]
    fn hit_test_respects_boundary() {
        // 边界右/下不含（半开区间）
        assert_eq!(hit_test(340.0, 20.0, &regions()), None);
        assert_eq!(hit_test(50.0, 44.0, &regions()), None);
    }

    #[test]
    fn hit_test_no_regions_returns_none() {
        assert_eq!(hit_test(10.0, 10.0, &[]), None);
    }
}
