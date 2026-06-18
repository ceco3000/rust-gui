//! rgui single-button example using `html!` declarative syntax + `build_scene_from_view`.
//!
//! Demonstrates the simplest html! macro usage with the view-scene rendering pipeline.

use rgui::app::{App, AppConfig};
use rgui::paint_factory::default_paint_fn;
use rgui::{
    AppMessage, Color, PaintContext, PaintLayerData, Rect, WidgetId, WidgetView,
    build_scene_from_paint_data, build_scene_from_view, html,
};

#[derive(Debug, Clone, PartialEq, AppMessage)]
enum Msg {
    Clicked,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui - Single Button (html! 声明式渲染)")
            .window_size(300.0, 200.0),
    );
    app.register_defaults();

    let btn_bounds = Rect::new(100.0, 80.0, 100.0, 40.0);

    app.register_interaction(WidgetId::from_u64(1), btn_bounds, "OK", move |action| {
        println!("  Button clicked: {action}");
    });

    let paint_fn = default_paint_fn();
    app.set_view_scene_builder(move |frame: u64, width: u32, height: u32| {
        let w = width as f64;
        let h = height as f64;

        if frame == 0 {
            println!("[Example] Frame 0 scene build: logical size {w}x{h}");
        }

        // html! declarative UI definition
        let view: WidgetView<Msg> = html! {
            <Center>
                <Button id="1" label="OK" on:click={Msg::Clicked} />
            </Center>
        };

        // Background layer
        let mut bg_ctx = PaintContext::new(Rect::new(0.0, 0.0, w, h));
        bg_ctx.fill_rect(
            Rect::new(0.0, 0.0, w, h),
            Color::new(14.0 / 255.0, 18.0 / 255.0, 28.0 / 255.0, 1.0),
            0.0,
        );
        let bg_layer = PaintLayerData::new(
            WidgetId::from_u64(0),
            -1,
            Rect::new(0.0, 0.0, w, h),
            bg_ctx.into_operations(),
        );

        let mut bg_scene = build_scene_from_paint_data(&[bg_layer], frame, None);

        // Build SceneGraph from WidgetView tree
        let view_scene =
            build_scene_from_view(&view, Rect::new(0.0, 0.0, w, h), &paint_fn, frame, None);

        // Merge: background behind view layers
        bg_scene.layers.extend(view_scene.layers);
        bg_scene
    });

    println!("=== rgui Single Button (html! 声明式渲染) ===\n");
    println!("UI defined by html! macro, rendered via build_scene_from_view.");
    println!("Window: 300x200 (logical pixels)");
    println!();
    println!("Click the [OK] button...");

    app.run()
}
