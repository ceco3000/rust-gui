//! 文本真实字形离屏验证（D9）：cosmic-text 整形 + vello draw_glyphs → 像素级字形轮廓。

#![cfg(feature = "vello-backend")]

use rgui_core::geometry::Size;
use rgui_core::view::{Border, Color, PropValue, WidgetView};
use rgui_render::scene_graph::{DrawCmd, SceneGraph};
use rgui_render::vello::VelloBackend;

#[test]
fn draw_text_produces_real_glyph_pixels() {
    let mut backend = VelloBackend::new().expect("creates wgpu vello backend");

    let scene = SceneGraph::from_cmds(vec![DrawCmd::DrawText {
        x: 8.0,
        y: 30.0,
        text: "rgui-123".to_string(),
        size: 48.0,
        color: Color::rgb(255, 255, 255),
        width: 0.0,
    }]);
    let w = 220u32;
    let h = 90u32;
    let pixels = backend
        .render_offscreen(&scene, w, h)
        .expect("offscreen render must succeed");

    let is_white = |i: usize| pixels[i] > 180 && pixels[i + 1] > 180 && pixels[i + 2] > 180;
    let white_count = (0..pixels.len() / 4).filter(|p| is_white(p * 4)).count();
    assert!(
        white_count > 30,
        "应渲染出明显字形像素（真实字形轮廓），got {white_count}"
    );

    let mut max_row_white = 0usize;
    for y in 0..h {
        let row = (0..w)
            .filter(|x| is_white(((y * w + x) * 4) as usize))
            .count();
        max_row_white = max_row_white.max(row);
    }
    assert!(
        max_row_white < (w as usize) - 20,
        "字形应离散而非整行填充：row max {max_row_white} vs width {w}"
    );
}

#[test]
fn empty_text_renders_nothing() {
    let mut backend = VelloBackend::new().expect("creates backend");
    let scene = SceneGraph::from_cmds(vec![DrawCmd::DrawText {
        x: 8.0,
        y: 30.0,
        text: "".to_string(),
        size: 48.0,
        color: Color::rgb(255, 255, 255),
        width: 0.0,
    }]);
    let pixels = backend.render_offscreen(&scene, 120, 60).expect("renders");
    let is_white = |i: usize| pixels[i] > 200 && pixels[i + 1] > 200 && pixels[i + 2] > 200;
    let white_count = (0..pixels.len() / 4).filter(|p| is_white(p * 4)).count();
    assert!(white_count < 8, "空文本不应产生字形像素，got {white_count}");
}

#[test]
fn button_and_text_render_both_fill_and_glyphs() {
    let mut backend = VelloBackend::new().expect("creates backend");
    let scene = SceneGraph::from_cmds(vec![
        DrawCmd::FillRect {
            x: 0.0,
            y: 0.0,
            width: 340.0,
            height: 120.0,
            color: Color::rgb(40, 80, 230),
        },
        DrawCmd::DrawText {
            x: 40.0,
            y: 70.0,
            text: "Click me (clicked 0)".to_string(),
            size: 56.0,
            color: Color::rgb(255, 255, 255),
            width: 0.0,
        },
    ]);
    let w = 480u32;
    let h = 240u32;
    let pixels = backend.render_offscreen(&scene, w, h).expect("renders");

    let blue_count = (0..pixels.len() / 4)
        .filter(|p| {
            let i = p * 4;
            pixels[i + 2] > 150 && pixels[i] < 120 && pixels[i + 1] < 150
        })
        .count();
    assert!(blue_count > 500, "应渲染蓝色按钮区域，got {blue_count}");

    let white_count = (0..pixels.len() / 4)
        .filter(|p| {
            let i = p * 4;
            pixels[i] > 190 && pixels[i + 1] > 190 && pixels[i + 2] > 190
        })
        .count();
    assert!(
        white_count > 30,
        "按钮上应渲染出白色文字字形，got {white_count}"
    );
}

#[test]
fn from_view_button_renders_and_shows_text() {
    // 复刻 window_demo 的组件 view（Color 按钮 + 子 Str label）→ from_view → 离屏像素。
    let mut backend = VelloBackend::new().expect("creates backend");

    let mut button: WidgetView<()> = WidgetView::empty();
    button.props = PropValue::Color(Color::rgb(40, 80, 230));
    button.size = Some(Size::new(340.0, 120.0));
    let mut label: WidgetView<()> = WidgetView::empty();
    label.props = PropValue::Str("Click me (clicked 0)".to_string());
    label.size = Some(Size::new(320.0, 56.0));
    button.children.push(label);

    let scene = SceneGraph::from_view(&button);
    let mut has_fill = false;
    let mut has_text = false;
    for cmd in scene.cmds() {
        match cmd {
            DrawCmd::FillRect { .. } => has_fill = true,
            DrawCmd::DrawText { .. } => has_text = true,
            _ => {}
        }
    }
    assert!(has_fill, "from_view 应产出 FillRect（按钮）");
    assert!(has_text, "from_view 应产出 DrawText（label）");

    let w = 480u32;
    let h = 240u32;
    let pixels = backend.render_offscreen(&scene, w, h).expect("renders");
    let blue_count = (0..pixels.len() / 4)
        .filter(|p| {
            let i = p * 4;
            pixels[i + 2] > 150 && pixels[i] < 120 && pixels[i + 1] < 150
        })
        .count();
    assert!(
        blue_count > 500,
        "from_view 应渲染蓝色按钮，got {blue_count}"
    );
    let white_count = (0..pixels.len() / 4)
        .filter(|p| {
            let i = p * 4;
            pixels[i] > 190 && pixels[i + 1] > 190 && pixels[i + 2] > 190
        })
        .count();
    assert!(
        white_count > 30,
        "from_view 应渲染出白色文字字形，got {white_count}"
    );
}

#[test]
fn base_color_renders_srgb_282828_without_gamma_boost() {
    // D23 返工自证：填 #282828（sRGB 40）→ vello 输出应为 linear 分量（≈5，=linear(0.157)*255），
    // 而非被双 gamma 提亮（修复前读到 #6E=110）。屏幕 sRGB swapchain 会把 linear 5 编码回 #282828。
    let mut backend = VelloBackend::new().expect("creates backend");
    let scene = SceneGraph::from_cmds(vec![DrawCmd::FillRect {
        x: 0.0,
        y: 0.0,
        width: 340.0,
        height: 120.0,
        color: Color::rgb(40, 40, 40),
    }]);
    let w = 340u32;
    let h = 120u32;
    let pixels = backend.render_offscreen(&scene, w, h).expect("renders");
    let center = ((h / 2 * w + w / 2) * 4) as usize;
    let r = pixels[center];
    assert!(
        (r as i32 - 5).abs() <= 3,
        "#282828 的 linear 分量应为 ~5（sRGB 40→linear(0.157)*255≈5），got {r}（修复前会被 gamma 提亮为 ~110/#6E）"
    );
    assert_eq!(pixels[center + 1], r, "G 通道应一致（灰）");
    assert_eq!(pixels[center + 2], r, "B 通道应一致（灰）");
}

#[test]
fn border_view_produces_stroke_rect_and_pixels() {
    // D16：带 border 的 view → from_view 产 StrokeRect → vello 描边边框像素
    let mut backend = VelloBackend::new().expect("backend");

    let mut v: WidgetView<()> = WidgetView::empty();
    v.size = Some(Size::new(200.0, 100.0));
    v.border = Some(Border::new(Color::rgb(255, 230, 80), 3.0)); // 亮黄描边
    let scene = SceneGraph::from_view(&v);

    // 1. from_view 应产出 StrokeRect draw 指令
    let has_stroke = scene
        .cmds()
        .iter()
        .any(|c| matches!(c, DrawCmd::StrokeRect { .. }));
    assert!(has_stroke, "from_view 应产出 StrokeRect（描边）");

    // 2. 离屏渲染出描边像素（黄色边缘）
    let w = 240u32;
    let h = 140u32;
    let pixels = backend.render_offscreen(&scene, w, h).expect("renders");
    let yellow = (0..pixels.len() / 4)
        .filter(|p| {
            let i = p * 4;
            pixels[i] > 200 && pixels[i + 1] > 180 && pixels[i + 2] < 130
        })
        .count();
    assert!(yellow > 10, "应渲染描边边框像素（黄色），got {yellow}");
}
