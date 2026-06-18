//! rgui single-button example using `html!` declarative syntax.
//!
//! Demonstrates the simplest html! macro usage: a single button declarative UI.

use rgui::app::{App, AppConfig};
use rgui::{
    AppMessage, Button, ButtonState, Color, PaintContext, PaintLayerData, Rect, WidgetId,
    WidgetSpec, WidgetView, html,
};

#[derive(Debug, Clone, PartialEq, AppMessage)]
enum Msg {
    Clicked,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui - Single Button (html!)")
            .window_size(300.0, 200.0),
    );
    app.register_defaults();

    let btn_bounds = Rect::new(100.0, 80.0, 100.0, 40.0);

    app.register_interaction(WidgetId::from_u64(1), btn_bounds, "OK", move |action| {
        println!("  Button clicked: {action}");
    });

    app.set_scene_builder(move |_frame: u64, width: u32, height: u32| {
        let w = width as f64;
        let h = height as f64;

        if _frame == 0 {
            println!("[Example] Frame 0 scene build: logical size {w}x{h}");
        }

        // html! declarative UI definition
        let _view: WidgetView<Msg> = html! {
            <Center>
                <Button id="1" label="OK" on:click={Msg::Clicked} />
            </Center>
        };

        let mut layers: Vec<PaintLayerData> = Vec::new();

        // Background
        let mut bg_ctx = PaintContext::new(Rect::new(0.0, 0.0, w, h));
        bg_ctx.fill_rect(
            Rect::new(0.0, 0.0, w, h),
            Color::new(14.0 / 255.0, 18.0 / 255.0, 28.0 / 255.0, 1.0),
            0.0,
        );
        layers.push(PaintLayerData::new(
            WidgetId::from_u64(0),
            -1,
            Rect::new(0.0, 0.0, w, h),
            bg_ctx.into_operations(),
        ));

        // OK Button
        let mut btn_ctx = PaintContext::new(btn_bounds);
        Button.paint(&ButtonState::new("OK"), btn_bounds, &mut btn_ctx);
        layers.push(PaintLayerData::new(
            WidgetId::from_u64(1),
            0,
            btn_bounds,
            btn_ctx.into_operations(),
        ));

        layers
    });

    println!("=== rgui Single Button (html! syntax) ===\n");
    println!("UI defined by html! macro. Window: 300x200 (logical pixels)");
    println!();
    println!("Click the [OK] button...");

    app.run()
}
