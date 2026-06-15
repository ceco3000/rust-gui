//! DataGrid 组件——数据表格。

use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage as Am, PersistState as Ps, WidgetSpec};
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_macros::{AppMessage, PersistState};
use std::sync::Arc;

// ============================================================================
// DataGridState
// ============================================================================

/// 列定义。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ColumnDef {
    pub name: String,
    pub title: String,
    pub width: f64,
    pub sortable: bool,
}

impl ColumnDef {
    #[must_use]
    pub fn new(name: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            width: 120.0,
            sortable: true,
        }
    }
    #[must_use]
    pub fn width(mut self, w: f64) -> Self {
        self.width = w;
        self
    }
    #[must_use]
    pub fn sortable(mut self, v: bool) -> Self {
        self.sortable = v;
        self
    }
}

/// DataGrid 业务状态。
#[derive(Debug, Clone, serde::Serialize, PersistState)]
pub struct DataGridState {
    pub columns: Vec<ColumnDef>,
    pub rows: Vec<Vec<String>>,
    pub sort_column: Option<usize>,
    pub sort_ascending: bool,
    pub selected_row: Option<usize>,
}

impl Default for DataGridState {
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            sort_column: None,
            sort_ascending: true,
            selected_row: None,
        }
    }
}

impl DataGridState {
    #[must_use]
    pub fn new(columns: Vec<ColumnDef>) -> Self {
        Self {
            columns,
            ..Self::default()
        }
    }

    /// 添加数据行。
    pub fn add_row(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }

    /// 获取排序后的行索引。
    fn sorted_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..self.rows.len()).collect();
        if let Some(col) = self.sort_column {
            if col < self.columns.len() {
                indices.sort_by(|&a, &b| {
                    let va = self.rows[a].get(col).map(|s| s.as_str()).unwrap_or("");
                    let vb = self.rows[b].get(col).map(|s| s.as_str()).unwrap_or("");
                    if self.sort_ascending {
                        va.cmp(vb)
                    } else {
                        vb.cmp(va)
                    }
                });
            }
        }
        indices
    }
}

// ============================================================================
// DataGridMessage
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMessage)]
pub enum DataGridMessage {
    SortBy(usize),
    SelectRow(usize),
    CellEdited(usize, usize, String),
}

// ============================================================================
// DataGrid
// ============================================================================

pub struct DataGrid;

impl WidgetSpec for DataGrid {
    type State = DataGridState;
    type Message = DataGridMessage;

    fn name(&self) -> &'static str {
        "rgui_components::DataGrid"
    }

    fn view(&self, state: &Self::State, _ctx: &ViewContext) -> WidgetView<Self::Message> {
        // 表头
        let mut header_props: Vec<WidgetView<DataGridMessage>> = Vec::new();
        for (i, col) in state.columns.iter().enumerate() {
            let sort_indicator = if state.sort_column == Some(i) {
                if state.sort_ascending { " ▲" } else { " ▼" }
            } else {
                ""
            };
            header_props.push(WidgetView::new("Label").prop(
                "text",
                PropValue::Str(Arc::from(format!("{}{}", col.title, sort_indicator))),
            ));
        }

        // 数据行
        let sorted = state.sorted_indices();
        let mut row_views: Vec<WidgetView<DataGridMessage>> = Vec::new();
        for &row_idx in &sorted {
            let selected = state.selected_row == Some(row_idx);
            let _bg = if selected { "selected" } else { "normal" };

            let mut cells: Vec<WidgetView<DataGridMessage>> = Vec::new();
            for (col_idx, _col) in state.columns.iter().enumerate() {
                let text = state.rows[row_idx]
                    .get(col_idx)
                    .cloned()
                    .unwrap_or_default();
                cells.push(WidgetView::new("Label").prop("text", PropValue::Str(Arc::from(text))));
            }
            row_views.push(WidgetView::new("TableRow").prop("selected", PropValue::Bool(selected)));
        }

        WidgetView::new("DataGrid")
            .prop("row_count", PropValue::Int(state.rows.len() as i64))
            .prop("col_count", PropValue::Int(state.columns.len() as i64))
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _ctx: &mut UpdateContext) {
        match msg {
            DataGridMessage::SortBy(col_idx) => {
                if col_idx < state.columns.len() && state.columns[col_idx].sortable {
                    if state.sort_column == Some(col_idx) {
                        state.sort_ascending = !state.sort_ascending;
                    } else {
                        state.sort_column = Some(col_idx);
                        state.sort_ascending = true;
                    }
                }
            },
            DataGridMessage::SelectRow(row_idx) => {
                state.selected_row = if state.selected_row == Some(row_idx) {
                    None
                } else {
                    Some(row_idx)
                };
            },
            DataGridMessage::CellEdited(row_idx, col_idx, value) => {
                if let Some(row) = state.rows.get_mut(row_idx) {
                    if let Some(cell) = row.get_mut(col_idx) {
                        *cell = value;
                    }
                }
            },
        }
    }

    fn measure(
        &self,
        state: &Self::State,
        constraints: BoxConstraints,
        _ctx: &MeasureContext,
    ) -> Size {
        let total_width: f64 = state.columns.iter().map(|c| c.width).sum();
        let row_height = 28.0_f64;
        let total_height = row_height * (state.rows.len() + 1) as f64 + 4.0;
        Size::new(
            total_width.clamp(constraints.min_width, constraints.max_width),
            total_height.clamp(constraints.min_height, constraints.max_height),
        )
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let header_h = 28.0;
        let row_h = 24.0;

        // 表头背景
        if header_h <= bounds.size.height {
            ctx.fill_rect(
                Rect::new(
                    bounds.origin.x,
                    bounds.origin.y,
                    bounds.size.width,
                    header_h,
                ),
                Color::new(0.15, 0.18, 0.25, 1.0),
                0.0,
            );
        }

        // 表头列标题
        let mut x = bounds.origin.x + 4.0;
        for col in &state.columns {
            let col_w = col.width.min(bounds.size.width - (x - bounds.origin.x));
            if col_w <= 0.0 {
                break;
            }
            ctx.draw_text(
                &col.title,
                Rect::new(x, bounds.origin.y, col_w, header_h),
                Color::new(0.8, 0.8, 0.85, 1.0),
                13.0,
            );
            x += col_w;
        }

        // 数据行
        let start_y = bounds.origin.y + header_h;
        let row_count = state
            .rows
            .len()
            .min(((bounds.size.height - header_h) / row_h) as usize);
        for (row_idx, row) in state.rows.iter().take(row_count).enumerate() {
            let y = start_y + row_idx as f64 * row_h;
            let row_bg = if Some(row_idx) == state.selected_row {
                Color::new(0.12, 0.30, 0.55, 0.5)
            } else if row_idx % 2 == 0 {
                Color::new(0.12, 0.14, 0.20, 0.5)
            } else {
                Color::new(0.10, 0.12, 0.18, 0.5)
            };
            ctx.fill_rect(
                Rect::new(bounds.origin.x, y, bounds.size.width, row_h),
                row_bg,
                0.0,
            );

            let mut cx = bounds.origin.x + 4.0;
            for (col_idx, cell) in row.iter().enumerate() {
                let col_w = state
                    .columns
                    .get(col_idx)
                    .map(|c| c.width)
                    .unwrap_or(80.0)
                    .min(bounds.size.width - (cx - bounds.origin.x));
                if col_w <= 0.0 {
                    break;
                }
                ctx.draw_text(
                    cell,
                    Rect::new(cx, y, col_w, row_h),
                    if Some(row_idx) == state.selected_row {
                        Color::WHITE
                    } else {
                        Color::new(0.85, 0.85, 0.9, 1.0)
                    },
                    13.0,
                );
                cx += col_w;
            }
        }
    }

    fn accessibility(&self, state: &Self::State, _ctx: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label(format!(
            "数据表格: {} 列, {} 行",
            state.columns.len(),
            state.rows.len()
        ))
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> DataGridState {
        let mut state = DataGridState::new(vec![
            ColumnDef::new("name", "姓名").width(100.0),
            ColumnDef::new("age", "年龄").width(60.0),
        ]);
        state.add_row(vec!["Alice".into(), "30".into()]);
        state.add_row(vec!["Bob".into(), "25".into()]);
        state
    }

    #[test]
    fn name() {
        assert_eq!(DataGrid.name(), "rgui_components::DataGrid");
    }

    #[test]
    fn sort_by_column() {
        let mut state = make_state();
        DataGrid.update(
            DataGridMessage::SortBy(0),
            &mut state,
            &mut UpdateContext::default(),
        );
        assert_eq!(state.sort_column, Some(0));
        assert!(state.sort_ascending);
    }

    #[test]
    fn sort_toggle_direction() {
        let mut state = make_state();
        DataGrid.update(
            DataGridMessage::SortBy(1),
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(state.sort_ascending);
        DataGrid.update(
            DataGridMessage::SortBy(1),
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(!state.sort_ascending);
    }

    #[test]
    fn select_row() {
        let mut state = make_state();
        DataGrid.update(
            DataGridMessage::SelectRow(0),
            &mut state,
            &mut UpdateContext::default(),
        );
        assert_eq!(state.selected_row, Some(0));
        DataGrid.update(
            DataGridMessage::SelectRow(0),
            &mut state,
            &mut UpdateContext::default(),
        );
        assert_eq!(state.selected_row, None);
    }

    #[test]
    fn cell_edit() {
        let mut state = make_state();
        DataGrid.update(
            DataGridMessage::CellEdited(0, 0, "Carol".into()),
            &mut state,
            &mut UpdateContext::default(),
        );
        assert_eq!(state.rows[0][0], "Carol");
    }

    #[test]
    fn measure_respects_constraints() {
        let state = make_state();
        let sz = DataGrid.measure(
            &state,
            BoxConstraints::UNCONSTRAINED,
            &MeasureContext::default(),
        );
        assert!(sz.width > 0.0);
        assert!(sz.height > 0.0);
    }

    #[test]
    fn sorted_indices() {
        let mut state = make_state();
        state.sort_column = Some(1); // sort by age
        state.sort_ascending = true;
        let indices = state.sorted_indices();
        // Bob (25) should come before Alice (30)
        assert_eq!(indices[0], 1);
        assert_eq!(indices[1], 0);
    }

    #[test]
    fn view_has_counts() {
        let state = make_state();
        let v = DataGrid.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("row_count"));
        assert!(v.props.contains_key("col_count"));
    }

    #[test]
    fn paint_grid() {
        let state = make_state();
        let bounds = Rect::new(0.0, 0.0, 400.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        DataGrid.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3, "应绘制表头 + 行背景 + 单元格文本");
    }

    #[test]
    fn paint_empty_grid() {
        let state = DataGridState::new(vec![ColumnDef::new("id", "ID")]);
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        DataGrid.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 2, "至少绘制表头背景 + 列标题");
    }
}
