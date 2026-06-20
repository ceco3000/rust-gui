use rgui::AppMessage;
use rgui::app::{App, AppConfig};
use rgui::paint_factory::default_paint_fn;
use rgui_core::geometry::{Rect, Size};
use rgui_devtools::rgui_parser::parse_rgui_file;
use rgui_layout::LayoutEngine;
use rgui_render::{build_scene_from_view, compute_view_layout};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, AppMessage)]
#[allow(dead_code)]
enum Msg { Noop }

fn register_click_interactions<M: AppMessage>(
    view: &rgui_core::view::WidgetView<M>,
    engine: &LayoutEngine,
    app: &mut App,
) {
    if let Some(widget_id) = view.id {
        if let Some(action) = view.props.get("onclick") {
            if let rgui_core::view::PropValue::Str(action_str) = action {
                if let Some(layout) = engine.get_layout(widget_id) {
                    let rect = Rect::new(
                        layout.result.position.x,
                        layout.result.position.y,
                        layout.result.size.width,
                        layout.result.size.height,
                    );
                    let action_owned = action_str.to_string();
                    app.register_interaction(widget_id, rect, &action_owned, move |_| {});
                }
            }
        }
    }
    for child in &view.children {
        register_click_interactions(child, engine, app);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rgui_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/ui.rgui"));
    let mut view: rgui_core::view::WidgetView<Msg> =
        parse_rgui_file(rgui_path).map_err(|e| format!(".rgui parse failed: {e}"))?;

    let layout = compute_view_layout(&mut view, Size::new(350.0, 200.0), None);

    let config = AppConfig::new()
        .title("rgui — WaBreadcrumb Demo")
        .window_size(350.0, 200.0);

    let mut app = App::new(config);
    register_click_interactions(&view, &layout, &mut app);

    let rhai_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/handlers.rhai"));
    app.load_rhai_scripts(&[rhai_path])
        .map_err(|e| format!(".rhai load failed: {e}"))?;

    let current_view = view;
    let paint_fn = default_paint_fn::<Msg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, tr: &rgui::TextRenderer| {
            let mut v = current_view.clone();
            let l = compute_view_layout(
                &mut v,
                Size::new(f64::from(width), f64::from(height)),
                Some(tr),
            );
            build_scene_from_view(&v, &l, &paint_fn, frame, Some(tr))
        },
    );

    println!("=== rgui WaBreadcrumb Demo ===\n");
    app.run()
}
