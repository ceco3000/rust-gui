//! rgui DataGrid example — static data table demo.
//!
//! Demonstrates `build_scene_from_paint_data` low-level API for complex-state
//! widgets (DataGrid with columns + rows). DataGrid state cannot be expressed
//! as simple html! props, so this example uses the manual PaintLayerData path.

use rgui::app::{App, AppConfig};
use rgui::{
    ColumnDef, DataGrid, DataGridState, PaintContext, PaintLayerData, Rect, WidgetId, WidgetSpec,
    build_scene_from_paint_data,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — DataGrid")
            .window_size(500.0, 250.0),
    );
    app.register_defaults();

    // DataGrid: 3 columns, 3 rows
    let mut state = DataGridState::new(vec![
        ColumnDef::new("name", "Name").width(120.0),
        ColumnDef::new("age", "Age").width(80.0),
        ColumnDef::new("city", "City").width(140.0),
    ]);
    state.add_row(vec!["Alice".into(), "30".into(), "New York".into()]);
    state.add_row(vec!["Bob".into(), "25".into(), "London".into()]);
    state.add_row(vec!["Charlie".into(), "35".into(), "Tokyo".into()]);

    let total_width: f64 = state.columns.iter().map(|c| c.width).sum();
    let grid_height = 28.0 + 3.0 * 24.0; // header + 3 rows

    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, _tr: &rgui::TextRenderer| {
            let w = width as f64;
            let h = height as f64;

            let mut layers: Vec<PaintLayerData> = Vec::new();

            // --- DataGrid (centered) ---
            let grid_bounds = Rect::new(
                (w - total_width) / 2.0,
                (h - grid_height) / 2.0,
                total_width,
                grid_height,
            );
            let mut grid_ctx = PaintContext::new(grid_bounds);
            DataGrid.paint(&state, grid_bounds, &mut grid_ctx);
            layers.push(PaintLayerData::new(
                WidgetId::from_u64(1),
                0,
                grid_bounds,
                grid_ctx.into_operations(),
            ));

            build_scene_from_paint_data(&layers, frame, Some(_tr))
        },
    );

    println!("=== rgui DataGrid example ===\n");
    println!("Window: 500×250 (logical pixels)");
    println!("Columns: Name (120px), Age (80px), City (140px)");
    println!("3 rows: Alice, Bob, Charlie\n");
    println!("Static display — no interaction required.\n");

    app.run()
}
