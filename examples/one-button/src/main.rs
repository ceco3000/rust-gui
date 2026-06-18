//! rgui single-button example using `html!` declarative syntax + `build_scene_from_view`.
//!
//! Demonstrates the simplest html! macro usage with the view-scene rendering pipeline.

use rgui::app::{App, AppConfig};
use rgui::paint_factory::default_paint_fn;
use rgui::{
    AppMessage, Rect, Size, WidgetId, WidgetView, build_scene_from_view, compute_view_layout, html,
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

    let paint_fn = default_paint_fn::<Msg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, _tr: &rgui::TextRenderer| {
            let w = width as f64;
            let h = height as f64;

            if frame == 0 {
                println!("[Example] Frame 0 scene build: logical size {w}x{h}");
            }

            // html! declarative UI definition
            let mut view: WidgetView<Msg> = html! {
                <Center>
                    <Button id="1" label="OK" on:click={Msg::Clicked} />
                </Center>
            };

            // Compute Taffy layout and build SceneGraph from WidgetView tree
            let layout = compute_view_layout(&mut view, Size::new(w, h));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(_tr))
        },
    );

    println!("=== rgui Single Button (html! 声明式渲染) ===\n");
    println!("UI defined by html! macro, rendered via build_scene_from_view.");
    println!("Window: 300x200 (logical pixels)");
    println!();
    println!("Click the [OK] button...");

    app.run()
}
