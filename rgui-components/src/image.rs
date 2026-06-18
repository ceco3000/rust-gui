//! Image 组件——图片显示。
//!
//! 显示 RGBA 像素缓冲。阶段 0 最小实现：paint() 以 DrawImage 占位矩形渲染
//! （VelloBackend 当前渲染为洋红色半透明矩形）。未来阶段将集成 GPU 纹理上传。
//!
//! 关键依赖：`RenderBackend::update_texture()`（已在 R04a 中实现）。

use rgui_core::a11y::{AccessibilityNode, AccessibilityRole};
use rgui_core::context::{
    AccessContext, MeasureContext, PaintContext, PaintOp, UpdateContext, ViewContext,
};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage, PersistState, WidgetSpec};
use rgui_core::view::WidgetView;
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};
use std::sync::Arc;

/// 图像适应模式（D13 §4.5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ImageFit {
    /// 拉伸填满——图像缩放至完全填满目标区域（不保持宽高比）。
    Fill,
    /// 适应——缩放至完全包含在目标区域内（保持宽高比，可能留白）。
    #[default]
    Contain,
    /// 裁剪——缩放至完全覆盖目标区域（保持宽高比，超出部分裁剪）。
    Cover,
    /// 不缩放——以原始尺寸显示（超出目标区域部分裁剪）。
    None,
}

/// Image 组件状态。
///
/// 阶段 0 范围：`image_data` 为内存中 RGBA 像素缓冲（无文件加载）。
/// 阶段 0 范围外：文件解码（image crate）、SVG 渲染、网络图片。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct ImageState {
    /// RGBA 像素缓冲（阶段 0：仅支持内存中的 RGBA 数据）。
    /// 每个像素 4 字节（R, G, B, A）。
    #[serde(skip)]
    pub image_data: Option<Arc<[u8]>>,
    /// 图像宽度（像素）。
    pub width: u32,
    /// 图像高度（像素）。
    pub height: u32,
    /// 图像适应模式（默认 `Contain`）。
    pub fit: Option<ImageFit>,
}

impl ImageState {
    /// 返回当前 fit 模式，未设置时默认 `Contain`。
    fn fit_mode(&self) -> ImageFit {
        self.fit.unwrap_or_default()
    }
}

/// Image 消息类型（占位）。
///
/// Image 本身不产生交互消息，提供此枚举以满足 `WidgetSpec` 关联类型约束。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum ImageMessage {
    NoOp,
}

/// Image 组件（unit struct）。
pub struct Image;

impl Image {
    /// 根据 fit 模式和约束计算实际绘制矩形。
    ///
    /// 返回 `(draw_rect, clip_rect)`——在此阶段 `draw_rect` 即为最终位置，
    /// `clip_rect` 预留未来裁剪支持。
    fn compute_fit_rect(image_w: u32, image_h: u32, bounds: Rect, fit: ImageFit) -> Rect {
        if image_w == 0 || image_h == 0 {
            return Rect::ZERO;
        }

        let img_w = image_w as f64;
        let img_h = image_h as f64;
        let box_w = bounds.size.width;
        let box_h = bounds.size.height;

        match fit {
            ImageFit::Fill => bounds,
            ImageFit::Contain => {
                let scale = (box_w / img_w).min(box_h / img_h);
                let w = img_w * scale;
                let h = img_h * scale;
                let x = bounds.origin.x + (box_w - w) / 2.0;
                let y = bounds.origin.y + (box_h - h) / 2.0;
                Rect::new(x, y, w, h)
            },
            ImageFit::Cover => {
                let scale = (box_w / img_w).max(box_h / img_h);
                let w = img_w * scale;
                let h = img_h * scale;
                let x = bounds.origin.x + (box_w - w) / 2.0;
                let y = bounds.origin.y + (box_h - h) / 2.0;
                Rect::new(x, y, w, h)
            },
            ImageFit::None => {
                let w = img_w.min(box_w);
                let h = img_h.min(box_h);
                let x = bounds.origin.x;
                let y = bounds.origin.y;
                Rect::new(x, y, w, h)
            },
        }
    }
}

impl WidgetSpec for Image {
    type State = ImageState;
    type Message = ImageMessage;

    fn name(&self) -> &'static str {
        "rgui_components::Image"
    }

    fn view(&self, _s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::Image")
    }

    fn update(&self, _: Self::Message, _: &mut Self::State, _: &mut UpdateContext) {}

    fn paint(&self, s: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        if s.width == 0 || s.height == 0 {
            return;
        }
        let draw_rect = Self::compute_fit_rect(s.width, s.height, bounds, s.fit_mode());
        if draw_rect.size.width > 0.0 && draw_rect.size.height > 0.0 {
            ctx.draw_image(draw_rect);
        }
    }

    fn measure(
        &self,
        s: &Self::State,
        _constraints: BoxConstraints,
        _ctx: &MeasureContext,
    ) -> Size {
        if s.width == 0 || s.height == 0 {
            return Size::ZERO;
        }
        Size::new(s.width as f64, s.height as f64)
    }

    fn accessibility(&self, _: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgui_core::Color;

    // ---- helpers ----

    /// 创建红色 100×100 RGBA 像素缓冲。
    fn red_100x100() -> Arc<[u8]> {
        let mut data = vec![0u8; 100 * 100 * 4];
        for y in 0..100 {
            for x in 0..100 {
                let idx = (y * 100 + x) * 4;
                data[idx] = 255; // R
                data[idx + 1] = 0; // G
                data[idx + 2] = 0; // B
                data[idx + 3] = 255; // A
            }
        }
        data.into()
    }

    // ---- name ----

    #[test]
    fn name() {
        assert_eq!(Image.name(), "rgui_components::Image");
    }

    // ---- view ----

    #[test]
    fn view_returns_widget_view_with_component_name() {
        let state = ImageState::default();
        let view = Image.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.widget_type, "rgui_components::Image");
        assert!(view.children.is_empty());
    }

    // ---- update is noop ----

    #[test]
    fn update_is_noop() {
        let mut state = ImageState::default();
        let mut ctx = UpdateContext::default();
        Image.update(ImageMessage::NoOp, &mut state, &mut ctx);
        assert_eq!(state.width, 0);
        assert_eq!(state.height, 0);
        assert!(state.image_data.is_none());
    }

    // ---- paint ----

    #[test]
    fn paint_zero_dimensions_skips() {
        let state = ImageState::default();
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        Image.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0);
    }

    #[test]
    fn paint_with_data_emits_draw_image() {
        let state = ImageState {
            image_data: Some(red_100x100()),
            width: 100,
            height: 100,
            fit: Some(ImageFit::Fill),
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        Image.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 1);
        let ops = ctx.into_operations();
        assert!(matches!(ops[0], PaintOp::DrawImage { .. }));
    }

    #[test]
    fn paint_fill_fits_exact_bounds() {
        let state = ImageState {
            image_data: Some(red_100x100()),
            width: 100,
            height: 100,
            fit: Some(ImageFit::Fill),
        };
        let bounds = Rect::new(10.0, 20.0, 200.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        Image.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        match &ops[0] {
            PaintOp::DrawImage { rect } => {
                assert_eq!(rect.origin.x, 10.0);
                assert_eq!(rect.origin.y, 20.0);
                assert_eq!(rect.size.width, 200.0);
                assert_eq!(rect.size.height, 300.0);
            },
            _ => panic!("expected DrawImage"),
        }
    }

    #[test]
    fn paint_contain_does_not_exceed_bounds() {
        let state = ImageState {
            image_data: Some(red_100x100()),
            width: 100,
            height: 100,
            fit: Some(ImageFit::Contain),
        };
        // Box is wider than tall → image scales to full height
        let bounds = Rect::new(0.0, 0.0, 300.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        Image.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        match &ops[0] {
            PaintOp::DrawImage { rect } => {
                // Should scale to fit height=100
                assert!(rect.size.width <= bounds.size.width);
                assert!(rect.size.height <= bounds.size.height);
                // centered vertically
                assert!((rect.size.height - 100.0).abs() < 0.01);
            },
            _ => panic!("expected DrawImage"),
        }
    }

    #[test]
    fn paint_contain_narrow_box() {
        let state = ImageState {
            image_data: Some(red_100x100()),
            width: 100,
            height: 200,
            fit: Some(ImageFit::Contain),
        };
        // Box is tall and narrow → image constrained by width
        let bounds = Rect::new(0.0, 0.0, 50.0, 400.0);
        let mut ctx = PaintContext::new(bounds);
        Image.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        match &ops[0] {
            PaintOp::DrawImage { rect } => {
                // width limited to 50, height scales proportionally
                assert!((rect.size.width - 50.0).abs() < 0.01);
                assert!(rect.size.height <= 400.0);
            },
            _ => panic!("expected DrawImage"),
        }
    }

    #[test]
    fn paint_cover_fills_bounds() {
        let state = ImageState {
            image_data: Some(red_100x100()),
            width: 100,
            height: 100,
            fit: Some(ImageFit::Cover),
        };
        // Square image in 300×100 → cover scales to fill width
        let bounds = Rect::new(0.0, 0.0, 300.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        Image.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        match &ops[0] {
            PaintOp::DrawImage { rect } => {
                // Cover should fill at least one dimension fully
                assert!(
                    (rect.size.width - 300.0).abs() < 0.01
                        || (rect.size.height - 100.0).abs() < 0.01
                );
            },
            _ => panic!("expected DrawImage"),
        }
    }

    #[test]
    fn paint_none_uses_original_size_capped() {
        let state = ImageState {
            image_data: Some(red_100x100()),
            width: 100,
            height: 100,
            fit: Some(ImageFit::None),
        };
        // Small box → image is capped
        let bounds = Rect::new(0.0, 0.0, 50.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        Image.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        match &ops[0] {
            PaintOp::DrawImage { rect } => {
                assert!(rect.size.width <= 50.0);
                assert!(rect.size.height <= 50.0);
            },
            _ => panic!("expected DrawImage"),
        }
    }

    #[test]
    fn paint_none_large_box_uses_original() {
        let state = ImageState {
            image_data: Some(red_100x100()),
            width: 100,
            height: 100,
            fit: Some(ImageFit::None),
        };
        // Large box → image uses original 100×100
        let bounds = Rect::new(0.0, 0.0, 500.0, 500.0);
        let mut ctx = PaintContext::new(bounds);
        Image.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        match &ops[0] {
            PaintOp::DrawImage { rect } => {
                assert_eq!(rect.size.width, 100.0);
                assert_eq!(rect.size.height, 100.0);
            },
            _ => panic!("expected DrawImage"),
        }
    }

    // ---- measure ----

    #[test]
    fn measure_zero_dimensions_returns_zero() {
        let state = ImageState::default();
        let ctx = MeasureContext::default();
        let size = Image.measure(&state, BoxConstraints::UNCONSTRAINED, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn measure_returns_image_dimensions() {
        let state = ImageState {
            width: 200,
            height: 150,
            ..Default::default()
        };
        let ctx = MeasureContext::default();
        let size = Image.measure(&state, BoxConstraints::UNCONSTRAINED, &ctx);
        assert_eq!(size.width, 200.0);
        assert_eq!(size.height, 150.0);
    }

    // ---- accessibility ----

    #[test]
    fn accessibility_returns_none() {
        let state = ImageState::default();
        let ctx = AccessContext::new(Rect::new(0.0, 0.0, 800.0, 600.0));
        let node = Image.accessibility(&state, &ctx);
        assert_eq!(node.role, AccessibilityRole::None);
    }

    // ---- compute_fit_rect ----

    #[test]
    fn compute_fit_fill_returns_bounds() {
        let r =
            Image::compute_fit_rect(100, 200, Rect::new(10.0, 20.0, 50.0, 60.0), ImageFit::Fill);
        assert_eq!(r.origin.x, 10.0);
        assert_eq!(r.origin.y, 20.0);
        assert_eq!(r.size.width, 50.0);
        assert_eq!(r.size.height, 60.0);
    }

    #[test]
    fn compute_fit_contain_scales_to_fit() {
        // 200×100 image in 100×100 box → constrained by width to 100×50
        let r = Image::compute_fit_rect(
            200,
            100,
            Rect::new(0.0, 0.0, 100.0, 100.0),
            ImageFit::Contain,
        );
        assert!((r.size.width - 100.0).abs() < 0.01);
        assert!((r.size.height - 50.0).abs() < 0.01);
        // horizontally centered
        assert!((r.origin.x - 0.0).abs() < 0.01);
        // vertically centered: (100 - 50) / 2 = 25
        assert!((r.origin.y - 25.0).abs() < 0.01);
    }

    #[test]
    fn compute_fit_cover_scales_to_fill() {
        // 100×200 image in 100×100 box → constrained by width to 100×200
        let r =
            Image::compute_fit_rect(100, 200, Rect::new(0.0, 0.0, 100.0, 100.0), ImageFit::Cover);
        assert!((r.size.width - 100.0).abs() < 0.01);
        assert!((r.size.height - 200.0).abs() < 0.01);
    }

    #[test]
    fn compute_fit_none_caps_to_bounds() {
        let r = Image::compute_fit_rect(200, 200, Rect::new(5.0, 10.0, 50.0, 50.0), ImageFit::None);
        assert_eq!(r.size.width, 50.0);
        assert_eq!(r.size.height, 50.0);
        assert_eq!(r.origin.x, 5.0);
        assert_eq!(r.origin.y, 10.0);
    }

    #[test]
    fn compute_fit_none_large_bounds() {
        let r =
            Image::compute_fit_rect(100, 100, Rect::new(0.0, 0.0, 500.0, 500.0), ImageFit::None);
        assert_eq!(r.size.width, 100.0);
        assert_eq!(r.size.height, 100.0);
    }

    #[test]
    fn compute_fit_zero_dimensions_returns_zero() {
        let r = Image::compute_fit_rect(0, 0, Rect::new(0.0, 0.0, 100.0, 100.0), ImageFit::Contain);
        assert_eq!(r, Rect::ZERO);
    }

    // ---- image_fit default ----

    #[test]
    fn image_fit_default_is_contain() {
        assert_eq!(ImageFit::default(), ImageFit::Contain);
    }

    #[test]
    fn state_default_fit_mode_is_contain() {
        let state = ImageState::default();
        assert_eq!(state.fit_mode(), ImageFit::Contain);
    }

    // ---- derive ----

    #[test]
    fn derive_state_schema_name() {
        assert_eq!(ImageState::schema_name(), "ImageState");
    }

    #[test]
    fn derive_message_name() {
        assert_eq!(ImageMessage::NoOp.message_name(), "no_op");
    }
}
