//! D18 示例：动态增删组件列表——key-based reconcile + 运行时 add/remove。
//!
//! 窗口显示若干项（每项带 key），左键点击 Add、右键点击 Remove，项动态增删，
//! 子节点按 key 复用（reconcile），删除首项时后续项不被误伤（索引不重建）。
//! 运行：cargo run -p rgui --features window --example d18_list

#![cfg(feature = "window")]

use std::any::Any;

use rgui::geometry::Size;
use rgui::traits::{AppMessage, PersistState, WidgetSpec};
use rgui::view::{Border, Color, PropValue, WidgetView};
use rgui::{App, AppConfig};
use rgui_platform::event_loop::{ElementState, MouseButton, WindowEvent};

/// 列表项（带 key 供 reconcile）。
#[derive(Debug, Clone)]
struct Item {
    key: u64,
    count: u32,
}

/// 列表状态。
#[derive(Debug, Clone)]
struct ListState {
    items: Vec<Item>,
    next_key: u64,
}

impl Default for ListState {
    fn default() -> Self {
        Self {
            items: vec![
                Item { key: 1, count: 11 },
                Item { key: 2, count: 22 },
                Item { key: 3, count: 33 },
            ],
            next_key: 4,
        }
    }
}

impl PersistState for ListState {
    fn schema_name() -> &'static str {
        "d18_list_state"
    }
    fn schema_version() -> u32 {
        0
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// 列表消息（Add 新增项 / Remove 删除首项）。
#[derive(Debug, Clone)]
enum ListMsg {
    Add,
    Remove,
}

impl AppMessage for ListMsg {
    fn message_name(&self) -> &'static str {
        match self {
            ListMsg::Add => "list.add",
            ListMsg::Remove => "list.remove",
        }
    }
}

/// 列表根组件：每项一个子视图（带 key），add/remove 动态增删。
struct ListRoot;

impl WidgetSpec for ListRoot {
    type State = ListState;
    type Message = ListMsg;

    fn name(&self) -> &'static str {
        "d18_list_root"
    }

    fn view(
        &self,
        state: &Self::State,
        _ctx: &rgui::context::ViewContext,
    ) -> WidgetView<Self::Message> {
        // 每项一个子视图：key = item.key（reconcile 复用），显示 "item{key}: {count}"
        let items: Vec<WidgetView<Self::Message>> = state
            .items
            .iter()
            .map(|it| {
                let mut child: WidgetView<Self::Message> = WidgetView::empty();
                child.key = Some(it.key);
                child.props = PropValue::Str(format!("item {}: {}", it.key, it.count));
                child.size = Some(Size::new(480.0, 24.0));
                child.border = Some(Border::new(Color::rgb(0, 122, 255), 3.0)); // accent systemBlue
                child
            })
            .collect();

        let mut root: WidgetView<Self::Message> = WidgetView::empty();
        root.children = items;
        root.props = PropValue::Str("d18 dynamic list".to_string());
        root.size = Some(Size::new(520.0, 200.0));
        root
    }

    fn update(
        &self,
        msg: Self::Message,
        state: &mut Self::State,
        _ctx: &mut rgui::context::UpdateContext,
    ) {
        match msg {
            ListMsg::Add => {
                let key = state.next_key;
                state.items.push(Item { key, count: 0 });
                state.next_key += 1;
            }
            ListMsg::Remove => {
                if !state.items.is_empty() {
                    state.items.remove(0); // 删除首项（后续项按 key 复用，索引不误伤）
                }
            }
        }
    }

    fn measure(
        &self,
        _state: &Self::State,
        _c: rgui::geometry::BoxConstraints,
        _ctx: &rgui::context::MeasureContext,
    ) -> Size {
        Size::new(520.0, 200.0)
    }

    fn paint(
        &self,
        _state: &Self::State,
        _b: rgui::geometry::Rect,
        _ctx: &mut rgui::context::PaintContext,
    ) {
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::new()
        .with_title("rgui d18 list")
        .with_size(520, 220);

    // 左键 Add、右键 Remove（动态增删；key-based reconcile 在 core diff 层保证复用）
    let mapper = move |event: &WindowEvent| -> Option<ListMsg> {
        match event {
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => match button {
                MouseButton::Left => Some(ListMsg::Add),
                MouseButton::Right => Some(ListMsg::Remove),
                _ => None,
            },
            _ => None,
        }
    };

    App::run(config, ListRoot, ListState::default(), mapper)?;
    Ok(())
}
