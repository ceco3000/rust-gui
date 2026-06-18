//! rgui DataGrid example — static data table demo.
//!
//! Displays a centered DataGrid with 3 columns (Name, Age, City) and
//! 3 sample data rows on a dark background. No interaction required.

use rgui::app::{App, AppConfig};
use rgui::{
    Color, ColumnDef, DataGrid, DataGridState, PaintContext, PaintLayerData, Rect, WidgetId,
    WidgetSpec,
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

    app.set_scene_builder(move |_frame: u64, width: u32, height: u32| {
        let w = width as f64;
        let h = height as f64;

        let mut layers: Vec<PaintLayerData> = Vec::new();

        // --- background ---
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

        layers
    });

    println!("=== rgui DataGrid example ===\n");
    println!("Window: 500×250 (logical pixels)");
    println!("Columns: Name (120px), Age (80px), City (140px)");
    println!("3 rows: Alice, Bob, Charlie\n");
    println!("Static display — no interaction required.\n");

    app.run()
}
