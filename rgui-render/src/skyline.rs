//! Skyline bin-packing 分配器。
//!
//! 在 2D 纹理中分配矩形空间，使用 bottom-left skyline 算法。
//! O(n) 分配，空间利用率高。用于 GlyphAtlas 的字形空间管理。

/// Skyline 段：在给定 y 高度处，从 x 到 x+width 的水平区间。
#[derive(Debug, Clone, Copy)]
struct SkylineSegment {
    x: u32,
    y: u32,
    width: u32,
}

/// Bottom-left skyline 2D bin-packing 分配器。
pub struct SkylineAllocator {
    width: u32,
    height: u32,
    skyline: Vec<SkylineSegment>,
}

/// 分配结果：Atlas 中的矩形位置。
#[derive(Debug, Clone, Copy)]
pub struct Allocation {
    pub x: u32,
    pub y: u32,
}

impl SkylineAllocator {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            skyline: vec![SkylineSegment { x: 0, y: 0, width }],
        }
    }

    /// 分配 `(w, h)` 矩形，返回左上角坐标。
    pub fn allocate(&mut self, w: u32, h: u32) -> Option<Allocation> {
        if w > self.width || h > self.height {
            return None;
        }

        let (best_idx, best_y) = self.find_best_position(w, h)?;
        let segment = self.skyline[best_idx];
        let alloc_x = segment.x;

        let new_segments = self.update_skyline(best_idx, alloc_x, best_y, w, h);
        self.skyline.splice(best_idx..=best_idx, new_segments);
        self.merge_segments();

        Some(Allocation {
            x: alloc_x,
            y: best_y,
        })
    }

    fn find_best_position(&self, w: u32, h: u32) -> Option<(usize, u32)> {
        let mut best_idx = None;
        let mut best_y = u32::MAX;

        for i in 0..self.skyline.len() {
            let mut max_y = self.skyline[i].y;
            let mut remaining = w;
            let mut j = i;
            while remaining > 0 && j < self.skyline.len() {
                max_y = max_y.max(self.skyline[j].y);
                let consumed = remaining.min(self.skyline[j].width);
                remaining -= consumed;
                if remaining > 0 {
                    j += 1;
                }
            }
            if remaining > 0 {
                continue;
            }
            if max_y + h > self.height {
                continue;
            }
            if max_y < best_y {
                best_y = max_y;
                best_idx = Some(i);
            }
        }

        best_idx.map(|idx| (idx, best_y))
    }

    fn update_skyline(
        &self,
        start_idx: usize,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    ) -> Vec<SkylineSegment> {
        let new_top = y + h;
        let end_x = x + w;

        let affected: Vec<&SkylineSegment> = self
            .skyline
            .iter()
            .skip(start_idx)
            .take_while(|s| s.x < end_x)
            .collect();

        if affected.is_empty() {
            return vec![];
        }

        let mut result = Vec::new();

        let first = affected[0];
        if x > first.x {
            result.push(SkylineSegment {
                x: first.x,
                y: first.y,
                width: x - first.x,
            });
        }

        result.push(SkylineSegment {
            x,
            y: new_top,
            width: w,
        });

        // affected 在 line 106 已确保非空，此处直接引用最后一个元素
        let last = affected
            .last()
            .expect("affected should be non-empty as checked above");
        let last_end = last.x + last.width;
        if end_x < last_end {
            result.push(SkylineSegment {
                x: end_x,
                y: last.y,
                width: last_end - end_x,
            });
        }

        result
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width > self.width {
            self.skyline.push(SkylineSegment {
                x: self.width,
                y: 0,
                width: new_width - self.width,
            });
            self.width = new_width;
            self.merge_segments();
        }
        if new_height > self.height {
            self.height = new_height;
        }
    }

    pub fn free(&mut self, _x: u32, _y: u32, _w: u32, _h: u32) {
        // 释放操作：完整实现需追踪每个分配的精确位置并恢复 skyline。
        // 当前通过 LRU 淘汰 + 重建 Atlas 处理回收。
    }

    fn merge_segments(&mut self) {
        let mut i = 0;
        while i + 1 < self.skyline.len() {
            if self.skyline[i].y == self.skyline[i + 1].y {
                self.skyline[i].width += self.skyline[i + 1].width;
                self.skyline.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }

    #[cfg(test)]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[cfg(test)]
    pub fn height(&self) -> u32 {
        self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_single_fits() {
        let mut a = SkylineAllocator::new(256, 256);
        let alloc = a.allocate(64, 64).unwrap();
        assert_eq!(alloc.x, 0);
        assert_eq!(alloc.y, 0);
    }

    #[test]
    fn alloc_second_goes_beside_first() {
        let mut a = SkylineAllocator::new(256, 256);
        a.allocate(128, 128).unwrap();
        let b = a.allocate(128, 128).unwrap();
        assert_eq!(b.x, 128);
        assert_eq!(b.y, 0);
    }

    #[test]
    fn alloc_too_wide_returns_none() {
        let mut a = SkylineAllocator::new(64, 256);
        assert!(a.allocate(128, 64).is_none());
    }

    #[test]
    fn alloc_too_tall_returns_none() {
        let mut a = SkylineAllocator::new(256, 64);
        assert!(a.allocate(64, 128).is_none());
    }

    #[test]
    fn alloc_many_small_rects() {
        let mut a = SkylineAllocator::new(256, 256);
        for i in 0..100 {
            assert!(
                a.allocate(16, 16).is_some(),
                "allocation #{i} should succeed"
            );
        }
    }

    #[test]
    fn resize_allows_bigger_alloc() {
        let mut a = SkylineAllocator::new(128, 128);
        assert!(a.allocate(256, 64).is_none());
        a.resize(256, 128);
        assert!(a.allocate(256, 64).is_some());
    }
}
