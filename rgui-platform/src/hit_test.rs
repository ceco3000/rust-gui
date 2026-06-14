//! 命中测试——确定哪个 widget 响应坐标点击（D5 §4）。

use rgui_core::geometry::{Point, Rect};
use rgui_core::id::WidgetId;
use rustc_hash::FxHashMap;

/// 命中测试器。
#[derive(Default)]
pub struct HitTester {
    bounds: FxHashMap<WidgetId, Rect>,
}

impl HitTester {
    #[must_use]
    pub fn new() -> Self {
        Self {
            bounds: FxHashMap::default(),
        }
    }

    pub fn register(&mut self, widget_id: WidgetId, bounds: Rect) {
        self.bounds.insert(widget_id, bounds);
    }

    pub fn unregister(&mut self, widget_id: WidgetId) {
        self.bounds.remove(&widget_id);
    }

    #[must_use]
    pub fn hit_test(&self, point: Point) -> Option<WidgetId> {
        self.bounds
            .iter()
            .filter(|(_, &bounds)| bounds.contains(point))
            .max_by_key(|(widget_id, _)| widget_id.as_u64())
            .map(|(&id, _)| id)
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
}
