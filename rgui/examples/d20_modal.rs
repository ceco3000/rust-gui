//! D20 示例：模态浮层 + 焦点隔离（FocusManager.open_modal/close_modal）。
//!
//! 后台两个按钮（可 Tab 循环）；点击左半「打开模态」——焦点隔离在模态按钮内
//! （后台按钮不可获焦，Tab 只在模态内循环）；点击右半「关闭模态」——恢复后台焦点。
//! 运行：cargo run -p rgui --features window --example d20_modal

#![cfg(feature = "window")]

use std::any::Any;
use std::cell::RefCell;

use rgui::geometry::{Rect, Size};
use rgui::hit_test::HitRegion;
use rgui::traits::{AppMessage, PersistState, WidgetSpec};
use rgui::view::{Border, Color, PropValue, WidgetView};
use rgui::WidgetId;
use rgui::{App, AppConfig};
use rgui_platform::event_loop::{ElementState, KeyCode, MouseButton, PhysicalKey, WindowEvent};
use rgui_platform::focus::FocusManager;

// 组件 id（FocusManager 可获焦集合）
const BASE_A: WidgetId = WidgetId::new(100);
const BASE_B: WidgetId = WidgetId::new(101);
const MODAL_BTN: WidgetId = WidgetId::new(200);

/// 模态状态。
#[derive(Debug, Clone, PartialEq)]
struct ModalState {
    modal_open: bool,
}

impl Default for ModalState {
    fn default() -> Self {
        Self { modal_open: false }
    }
}

impl PersistState for ModalState {
    fn schema_name() -> &'static str {
        "d20_modal_state"
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

/// 模态消息。
#[derive(Debug, Clone)]
enum ModalMsg {
    OpenModal,
    CloseModal,
}

impl AppMessage for ModalMsg {
    fn message_name(&self) -> &'static str {
        match self {
            ModalMsg::OpenModal => "modal.open",
            ModalMsg::CloseModal => "modal.close",
        }
    }
}

/// 带模态浮层的根组件（D20）。
struct ModalRoot;

impl ModalRoot {
    fn leaf(id: WidgetId, focused: bool, label: &str) -> WidgetView<ModalMsg> {
        let mut v = WidgetView::empty();
        v.props = PropValue::Str(label.to_string());
        v.size = Some(Size::new(200.0, 30.0));
        v.border = if focused {
            Some(Border::new(Color::rgb(255, 230, 80), 3.0))
        } else {
            None
        };
        let _ = id;
        v
    }
}

impl WidgetSpec for ModalRoot {
    type State = ModalState;
    type Message = ModalMsg;

    fn name(&self) -> &'static str {
        "modal_root"
    }

    fn view(
        &self,
        state: &Self::State,
        _ctx: &rgui::context::ViewContext,
    ) -> WidgetView<Self::Message> {
        let mut root = WidgetView::empty();
        root.size = Some(Size::new(520.0, 220.0));
        // 后台两个按钮（模态打开时仍显示但获焦被模态隔离）
        root.children.push(Self::leaf(BASE_A, false, "(A) 后台一"));
        root.children.push(Self::leaf(BASE_B, false, "(B) 后台二"));
        // 模态浮层（打开时覆盖，焦点隔离）
        if state.modal_open {
            let mut modal = WidgetView::empty();
            modal.props = PropValue::Str("MODAL — 焦点隔离 (点击关闭)".to_string());
            modal.size = Some(Size::new(400.0, 60.0));
            modal.border = Some(Border::new(Color::rgb(200, 120, 220), 3.0));
            root.children.push(modal);
        }
        root
    }

    fn update(
        &self,
        msg: Self::Message,
        state: &mut Self::State,
        _ctx: &mut rgui::context::UpdateContext,
    ) {
        match msg {
            ModalMsg::OpenModal => state.modal_open = true,
            ModalMsg::CloseModal => state.modal_open = false,
        }
    }

    fn measure(
        &self,
        _state: &Self::State,
        _c: rgui::geometry::BoxConstraints,
        _ctx: &rgui::context::MeasureContext,
    ) -> Size {
        Size::new(520.0, 220.0)
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
    // D22：尽早注册日志（幂等）——hit-region 等测试信号须在 subscriber 后打印
    rgui::logging::init_logging();
    let config = AppConfig::new()
        .with_title("rgui d20 modal")
        .with_size(520, 220);

    // hit 区域（逻辑坐标，与渲染 flex Row 布局一致；id 与 FocusManager WidgetId 一致）；后台 A/B（横排）+ 模态浮层（打开时）
    let regions = [
        HitRegion::new(Rect::new(0.0, 0.0, 200.0, 30.0), 100), // 后台 A
        HitRegion::new(Rect::new(200.0, 0.0, 200.0, 30.0), 101), // 后台 B
        HitRegion::new(Rect::new(400.0, 0.0, 400.0, 60.0), 200), // 模态浮层（打开后）
    ];
    // D21-2：hit-region 日志（qa 换算坐标点击，触发开/关模态）
    for r in &regions {
        let name = match r.id {
            100 => "base_a",
            101 => "base_b",
            200 => "modal",
            _ => "?",
        };
        tracing::info!(
            target: "rgui_test_signal",
            "[hit-region] id={} {} rect=({},{},{},{})",
            r.id,
            name,
            r.rect.x,
            r.rect.y,
            r.rect.width,
            r.rect.height
        );
    }

    let focus = RefCell::new(FocusManager::new());
    // 初始可获焦 = 后台两个按钮
    focus.borrow_mut().set_focusable(vec![BASE_A, BASE_B]);
    focus.borrow_mut().set_focus(BASE_A);

    let mapper = move |event: &WindowEvent| -> Option<ModalMsg> {
        match event {
            WindowEvent::KeyboardInput { event: ke, .. } => {
                if ke.state == ElementState::Pressed {
                    if let PhysicalKey::Code(k) = ke.physical_key {
                        match k {
                            // 模态打开时焦点自动隔离在模态集合内
                            KeyCode::Tab => {
                                let fid = focus.borrow_mut().focus_next();
                                tracing::info!(target: "rgui_test_signal", "[focus] Tab -> {:?}", fid.map(|w| w.0));
                            }
                            KeyCode::Escape => {
                                focus.borrow_mut().close_modal();
                                let f = focus.borrow().focus();
                                tracing::info!(target: "rgui_test_signal", "[action] modal_close");
                                tracing::info!(target: "rgui_test_signal", "[focus] Esc -> {:?}", f.map(|w| w.0));
                                return Some(ModalMsg::CloseModal);
                            }
                            _ => {}
                        }
                    }
                }
                None
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                // 打开模态 → 焦点隔离到模态按钮；关闭 → 恢复后台
                focus.borrow_mut().open_modal(vec![MODAL_BTN]);
                focus.borrow_mut().set_focus(MODAL_BTN);
                let f = focus.borrow().focus();
                tracing::info!(target: "rgui_test_signal", "[action] modal_open");
                tracing::info!(target: "rgui_test_signal", "[focus] click -> {:?}", f.map(|w| w.0));
                Some(ModalMsg::OpenModal)
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                focus.borrow_mut().close_modal();
                let f = focus.borrow().focus();
                tracing::info!(target: "rgui_test_signal", "[action] modal_close");
                tracing::info!(target: "rgui_test_signal", "[focus] click -> {:?}", f.map(|w| w.0));
                Some(ModalMsg::CloseModal)
            }
            _ => None,
        }
    };

    App::run(config, ModalRoot, ModalState::default(), mapper)?;
    Ok(())
}
