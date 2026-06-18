//! rgui DataGrid example — html! 声明式渲染 + 自定义 PaintFn。
//!
//! 使用 html! 宏声明 WidgetView 树 + 自定义 PaintFn 捕获 DataGridState。
//! DataGrid 状态（columns + rows）无法表达为简单 props，故使用
//! Arc<DataGridState> 闭包捕获模式。

use std::sync::Arc;

use rgui::app::{App, AppConfig};
use rgui::{
    AppMessage, ColumnDef, DataGrid, DataGridState, PaintFn, PaintOp, Rect, Size,
    WidgetSpec, WidgetView, build_scene_from_view, compute_view_layout, html,
};

#[derive(Debug, Clone, PartialEq, AppMessage)]
enum Msg {
    _Dummy,
}

/// 自定义 PaintFn：对普通组件委托 default_paint_fn；对 DataGrid 使用捕获的 state。
fn datagrid_paint_fn<M: AppMessage>(grid_state: Arc<DataGridState>) -> PaintFn<M> {
    use rgui::paint_factory::default_paint_fn;
    let base = default_paint_fn::<M>();

    Box::new(move |view: &WidgetView<M>, bounds: Rect| -> Vec<PaintOp> {
        if view.widget_type == "DataGrid" {
            let mut ctx = rgui::PaintContext::new(bounds);
            DataGrid.paint(&grid_state, bounds, &mut ctx);
            ctx.into_operations()
        } else {
            base(view, bounds)
        }
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — DataGrid")
            .window_size(500.0, 250.0),
    );
    app.register_defaults();

    // DataGrid state: 3 columns, 3 rows
    let mut state = DataGridState::new(vec![
        ColumnDef::new("name", "Name").width(120.0),
        ColumnDef::new("age", "Age").width(80.0),
        ColumnDef::new("city", "City").width(140.0),
    ]);
    state.add_row(vec!["Alice".into(), "30".into(), "New York".into()]);
    state.add_row(vec!["Bob".into(), "25".into(), "London".into()]);
    state.add_row(vec!["Charlie".into(), "35".into(), "Tokyo".into()]);
    let grid_state = Arc::new(state);

    let paint_fn = datagrid_paint_fn::<Msg>(Arc::clone(&grid_state));
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, tr: &rgui::TextRenderer| {
            let w = width as f64;
            let h = height as f64;

            let mut view: WidgetView<Msg> = html! {
                <Center>
                    <DataGrid />
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(tr))
        },
    );

    println!("=== rgui DataGrid example ===\n");
    println!("Window: 500×250 (logical pixels)");
    println!("html! 声明式 + 自定义 PaintFn (Arc<DataGridState>)");
    println!("Columns: Name (120px), Age (80px), City (140px)");
    println!("3 rows: Alice, Bob, Charlie\n");

    app.run()
}
