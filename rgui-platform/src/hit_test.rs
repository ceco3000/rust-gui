//! 命中测试——确定哪个 widget 响应坐标点击（D5 §4）。

use rgui_core::geometry::{Point, Rect};
use rgui_core::id::WidgetId;

/// 命中测试器。
///
/// 使用 Vec 维护插入顺序，后注册的 widget 在视觉上更靠前，
/// hit_test 按 LIFO 顺序（后注册优先）匹配，正确反映视觉层级。
pub struct HitTester {
    bounds: Vec<(WidgetId, Rect)>,
}

impl HitTester {
    #[must_use]
    pub fn new() -> Self {
        Self {
            bounds: Vec::new(),
        }
    }

    pub fn register(&mut self, widget_id: WidgetId, bounds: Rect) {
        self.bounds.push((widget_id, bounds));
    }

    pub fn unregister(&mut self, widget_id: WidgetId) {
        self.bounds.retain(|(id, _)| *id != widget_id);
    }

    /// 命中最上层（最后注册）的 widget。
    ///
    /// 按 LIFO 顺序（后注册优先）遍历，返回第一个包含 point 的 widget。
    /// 这正确反映了视觉层级——在 Column 布局中，下方 widget 后注册，视觉上可覆盖上方。
    #[must_use]
    pub fn hit_test(&self, point: Point) -> Option<WidgetId> {
        self.bounds
            .iter()
            .rev() // LIFO: 后注册优先
            .find(|(_, bounds)| bounds.contains(point))
            .map(|(id, _)| *id)
    }

    pub fn clear(&mut self) {
        self.bounds.clear();
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.bounds.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bounds.is_empty()
    }
}

impl Default for HitTester {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for HitTester {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HitTester")
            .field("widgets", &self.bounds.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_test_within_bounds() {
        let mut tester = HitTester::new();
        let id = WidgetId::from_u64(1);
        tester.register(id, Rect::new(10.0, 10.0, 100.0, 50.0));
        assert_eq!(tester.hit_test(Point::new(50.0, 30.0)), Some(id));
    }

    #[test]
    fn hit_test_outside() {
        let mut tester = HitTester::new();
        let id = WidgetId::from_u64(1);
        tester.register(id, Rect::new(10.0, 10.0, 100.0, 50.0));
        assert_eq!(tester.hit_test(Point::new(0.0, 0.0)), None);
    }

    #[test]
    fn hit_test_unregister() {
        let mut tester = HitTester::new();
        let id = WidgetId::from_u64(1);
        tester.register(id, Rect::new(0.0, 0.0, 100.0, 50.0));
        tester.unregister(id);
        assert_eq!(tester.hit_test(Point::new(50.0, 25.0)), None);
    }

    #[test]
    fn hit_test_lifo_priority() {
        let mut tester = HitTester::new();
        // Section 1 (先注册, 上层)
        tester.register(
            WidgetId::from_u64(1),
            Rect::new(0.0, 0.0, 200.0, 100.0),
        );
        // Section 2 (后注册, 下层，但与 Section 1 重叠)
        tester.register(
            WidgetId::from_u64(2),
            Rect::new(0.0, 50.0, 200.0, 100.0),
        );
        // 点击重叠区域——应命中后注册的 Section 2
        assert_eq!(
            tester.hit_test(Point::new(100.0, 75.0)),
            Some(WidgetId::from_u64(2))
        );
    }
}
