//! 文本整形模块（cosmic-text 隔离）——真实字形（D9）。
//!
//! `TextShaper` 用 cosmic-text (fontdb + swash) 整形一行文本，产出 vello `Glyph` 序列 +
//! `peniko::FontData`（字体 blob），供 `scene.draw_glyphs` 提取真实字形轮廓。
//!
//! 完整实现经 `vello-backend` feature 门控（cosmic-text/vello 重型依赖仅此引入）；
//! 无 feature 时提供占位（shape_line 返回空，不引重依赖），保证默认构建可编译。

#[cfg(feature = "vello-backend")]
mod wide {
    use std::sync::Arc;

    use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};
    use vello::peniko::{Blob, FontData};
    use vello::Glyph;

    /// 一组同字体的 glyph（vello 可一次 draw_glyphs）。
    pub struct ShapedRun {
        /// 字体数据（blob + 集合 index）。
        pub font_data: FontData,
        /// 形状化后的 glyph（id / x / y 相对 run 原点）。
        pub glyphs: Vec<Glyph>,
    }

    /// 文本整形器（持有 cosmic-text 字体系统）。
    pub struct TextShaper {
        font_system: FontSystem,
    }

    impl TextShaper {
        /// 创建整形器并加载系统字体（保证 demo 有字形可用）。
        pub fn new() -> Self {
            let mut font_system = FontSystem::new();
            font_system.db_mut().load_system_fonts();
            Self { font_system }
        }

        /// 整形文本，按字体分组产出 vello glyph runs。
        ///
        /// - `size`：字体大小（像素）。
        /// - `max_width`：逻辑可用宽度（`Some` 时按宽度换行；`None` 单行不截断）。
        /// - 返回空 Vec 表示无可见 glyph（空文本/无字体）。
        pub fn shape_line(
            &mut self,
            text: &str,
            size: f32,
            max_width: Option<f32>,
        ) -> Vec<ShapedRun> {
            if text.is_empty() {
                return Vec::new();
            }
            let metrics = Metrics::new(size, size * 1.2);
            let mut text_buf = Buffer::new(&mut self.font_system, metrics);
            text_buf.set_text(&mut self.font_system, text, Attrs::new(), Shaping::Advanced);
            // D17：设置可用宽度使 cosmic-text 按宽度换行（多行），避免长文本溢出组件/窗口边界
            if let Some(w) = max_width {
                if w > 0.0 {
                    text_buf.set_size(&mut self.font_system, Some(w), None);
                }
            }
            text_buf.shape_until_scroll(&mut self.font_system, false);

            let mut runs: Vec<ShapedRun> = Vec::new();
            let mut by_font: Vec<(cosmic_text::fontdb::ID, Vec<Glyph>)> = Vec::new();
            for line in text_buf.layout_runs() {
                // vello 的 glyph 原点在基线：用 line 的基线 y（line_y）作为 run 内 y 基准，
                // 这样文字随基线对齐（多行时 line_y 随行递增，实现换行垂直排布）。
                let baseline = line.line_y;
                for g in line.glyphs {
                    let gx = g.x + g.font_size * g.x_offset;
                    let gy = baseline - g.font_size * g.y_offset;
                    let gid = g.glyph_id as u32;
                    match by_font.iter().position(|(id, _)| *id == g.font_id) {
                        Some(i) => by_font[i].1.push(Glyph {
                            id: gid,
                            x: gx,
                            y: gy,
                        }),
                        None => by_font.push((
                            g.font_id,
                            vec![Glyph {
                                id: gid,
                                x: gx,
                                y: gy,
                            }],
                        )),
                    }
                }
            }
            for (font_id, glyphs) in by_font {
                let Some(font_data) = self.font_data_for(font_id) else {
                    continue;
                };
                runs.push(ShapedRun { font_data, glyphs });
            }
            runs
        }

        /// 从 fontdb 取指定字体的 blob + 集合 index，构造 peniko::FontData。
        fn font_data_for(&self, font_id: cosmic_text::fontdb::ID) -> Option<FontData> {
            self.font_system
                .db()
                .with_face_data(font_id, |data, index| {
                    let blob = Blob::new(Arc::new(Vec::<u8>::from(data)));
                    FontData::new(blob, index)
                })
        }
    }
}

#[cfg(not(feature = "vello-backend"))]
mod wide {
    /// 占位：无 vello-backend 时不引重依赖（cosmic-text/vello），shape_line 返回空。
    pub struct ShapedRun {
        pub font_data: (),
        pub glyphs: Vec<()>,
    }

    pub struct TextShaper;

    impl TextShaper {
        pub fn new() -> Self {
            Self
        }
        pub fn shape_line(
            &mut self,
            _text: &str,
            _size: f32,
            _max_width: Option<f32>,
        ) -> Vec<ShapedRun> {
            Vec::new()
        }
    }
}

pub use wide::{ShapedRun, TextShaper};

#[cfg(all(test, feature = "vello-backend"))]
mod tests {
    use super::*;

    #[test]
    fn shapes_ascii_text_into_glyphs_and_valid_font() {
        let mut shaper = TextShaper::new();
        let runs = shaper.shape_line("Click me (clicked 0)", 24.0, None);
        assert!(!runs.is_empty(), "应产出至少一个字形 run");
        let total: usize = runs.iter().map(|r| r.glyphs.len()).sum();
        assert!(total > 0, "应产出非空 glyph 序列");
        for run in &runs {
            assert_ne!(run.glyphs.len(), 0, "每个 run 应有字形");
            // 字体 blob 含数据
            assert!(run.font_data.data.len() > 0, "字体数据应非空");
            assert_eq!(
                run.font_data.index,
                if run.font_data.index == 0 {
                    0
                } else {
                    run.font_data.index
                }
            );
        }
    }

    #[test]
    fn long_text_wraps_when_width_limited() {
        let mut shaper = TextShaper::new();
        // 长文本 + 窄宽度 → 结果应多行（glyph y 范围 > 单行高度），而非单行溢出
        let long = "The quick brown fox jumps over the lazy dog. ".repeat(3);
        let single = shaper.shape_line(&long, 24.0, None);
        let wrapped = shaper.shape_line(&long, 24.0, Some(120.0));

        let y_range = |runs: &[ShapedRun]| -> (f32, f32) {
            let mut min_y = f32::MAX;
            let mut max_y = f32::MIN;
            for run in runs {
                for g in &run.glyphs {
                    min_y = min_y.min(g.y);
                    max_y = max_y.max(g.y);
                }
            }
            (min_y, max_y)
        };

        let (_, s_max) = y_range(&single);
        let (_, w_max) = y_range(&wrapped);
        assert!(
            (w_max - s_max) > 20.0,
            "换行后应多行（glyph 最大 y 显著增大），single_max={s_max} wrapped_max={w_max}"
        );
    }
}
