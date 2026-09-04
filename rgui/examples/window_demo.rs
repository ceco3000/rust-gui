//! D11 窗口示例：多组件 hit-test 事件路由——Accordion + WaBadge 同窗，各自响应点击。
//!
//! 点击坐标命中哪个组件的区域，就路由到该组件消息（Accordion 标题区 → Toggle；
//! WaBadge 区 → Click 计数）。经 facade `rgui::App::run`。
//! 运行：cargo run -p rgui --features window --example window_demo

#![cfg(feature = "window")]

use std::any::Any;
use std::cell::RefCell;

use rgui::geometry::Rect;
use rgui::hit_test::{hit_test, HitRegion};
use rgui::traits::{AppMessage, PersistState, WidgetSpec};
use rgui::view::WidgetView;
use rgui::{
    Accordion, AccordionMsg, AccordionState, App, AppConfig, FocusManager, WaBadge, WaBadgeMsg,
    WaBadgeState, WidgetId,
};
use rgui_platform::event_loop::{ElementState, KeyCode, MouseButton, PhysicalKey, WindowEvent};

// ===== 组合根：Accordion + WaBadge 同窗 =====

/// 组合根消息（路由到子组件）。
#[derive(Debug, Clone)]
enum DemoMsg {
    Accordion(AccordionMsg),
    Badge(WaBadgeMsg),
    /// 焦点切换到指定组件（Accordion=1 / WaBadge=2 / None=无焦点）。
    Focus(Option<WidgetId>),
}

impl AppMessage for DemoMsg {
    fn message_name(&self) -> &'static str {
        match self {
            DemoMsg::Accordion(m) => m.message_name(),
            DemoMsg::Badge(m) => m.message_name(),
            DemoMsg::Focus(_) => "demo.focus",
        }
    }
}

/// 组合根状态（持有两个子组件状态 + 获焦子 id）。
#[derive(Debug, Clone)]
struct DemoRootState {
    accordion: AccordionState,
    badge: WaBadgeState,
    /// 当前获焦子组件 id（Accordion=1 / WaBadge=2 / None=无焦点）。
    focused: Option<WidgetId>,
}

impl Default for DemoRootState {
    fn default() -> Self {
        Self {
            accordion: Accordion::initial_state(),
            badge: WaBadge::initial_state(),
            // 初始焦点 = 第一个可获焦组件（Accordion），Tab 循环切换
            focused: Some(WidgetId::new(1)),
        }
    }
}

impl PersistState for DemoRootState {
    fn schema_name() -> &'static str {
        "demo_root_state"
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

/// 组合根组件：横排展示 Accordion（左）+ WaBadge（右），各自响应点击。
struct DemoRoot;

impl WidgetSpec for DemoRoot {
    type State = DemoRootState;
    type Message = DemoMsg;

    fn name(&self) -> &'static str {
        "demo_root"
    }

    fn view(
        &self,
        state: &Self::State,
        _ctx: &rgui::context::ViewContext,
    ) -> WidgetView<Self::Message> {
        let mut root = WidgetView::empty();
        root.size = Some(rgui::geometry::Size::new(520.0, 220.0));
        // 左：Accordion 视图（消息提升为组合根消息）；按获焦状态设 focused 视图上下文
        let mut acc_ctx = rgui::context::ViewContext::default();
        acc_ctx.focused = state.focused == Some(WidgetId::new(1));
        let acc = Accordion
            .view(&state.accordion, &acc_ctx)
            .map_message(&DemoMsg::Accordion);
        // 右：WaBadge 视图
        let mut badge_ctx = rgui::context::ViewContext::default();
        badge_ctx.focused = state.focused == Some(WidgetId::new(2));
        let badge = WaBadge
            .view(&state.badge, &badge_ctx)
            .map_message(&DemoMsg::Badge);
        root.children.push(acc);
        root.children.push(badge);
        root
    }

    fn update(
        &self,
        msg: Self::Message,
        state: &mut Self::State,
        ctx: &mut rgui::context::UpdateContext,
    ) {
        match msg {
            DemoMsg::Accordion(m) => Accordion.update(m, &mut state.accordion, ctx),
            DemoMsg::Badge(m) => WaBadge.update(m, &mut state.badge, ctx),
            DemoMsg::Focus(fid) => state.focused = fid,
        }
    }

    fn measure(
        &self,
        _state: &Self::State,
        _c: rgui::geometry::BoxConstraints,
        _ctx: &rgui::context::MeasureContext,
    ) -> rgui::geometry::Size {
        rgui::geometry::Size::new(520.0, 220.0)
    }

    fn paint(&self, _state: &Self::State, _b: Rect, _ctx: &mut rgui::context::PaintContext) {}
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::new()
        .with_title("rgui hit-test demo")
        .with_size(520, 220);

    // hit 区域（逻辑坐标）：Accordion 标题区（左 0-340）+ WaBadge 区（右 340-520）
    let regions = [
        HitRegion::new(Rect::new(0.0, 0.0, 340.0, 44.0), 1), // Accordion 标题区
        HitRegion::new(Rect::new(340.0, 0.0, 180.0, 40.0), 2), // WaBadge 区
    ];

    // D21：输出每个可交互组件的命中区（逻辑坐标，与 hit_test 用的 regions 完全一致），
    // 自动化脚本据此换算屏幕绝对坐标"点哪里"。
    for r in &regions {
        let name = match r.id {
            1 => "accordion",
            2 => "wabadge",
            _ => "?",
        };
        eprintln!(
            "[hit-region] id={} {} rect=({},{},{},{})",
            r.id, name, r.rect.x, r.rect.y, r.rect.width, r.rect.height
        );
    }

    // 缓存光标位置（逻辑坐标待命中用）
    let cursor = RefCell::new((0.0f32, 0.0f32));
    // 跟踪 Shift 键状态（winit 把 modifiers 经 ModifiersChanged 单独传递，需自持）
    let shift = RefCell::new(false);
    // 焦点管理：Accordion(1) + WaBadge(2) 可获焦，Tab 循环切换（获焦/失焦由 unit 测试确定性验证）
    let focus = RefCell::new(FocusManager::new());
    focus
        .borrow_mut()
        .set_focusable(vec![WidgetId::new(1), WidgetId::new(2)]);

    let mapper = move |event: &WindowEvent| -> Option<DemoMsg> {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                // D15：物理像素 → 逻辑坐标（按窗口 scale_factor 换算），hit-test 用逻辑坐标
                let (lx, ly) = rgui_platform::window::to_logical(
                    (position.x, position.y),
                    rgui_platform::window::platform_scale(),
                );
                *cursor.borrow_mut() = (lx, ly);
                None
            }
            WindowEvent::ModifiersChanged(m) => {
                *shift.borrow_mut() = m.state().shift_key();
                None
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && event.physical_key == PhysicalKey::Code(KeyCode::Tab) =>
            {
                // Tab → focus_next；Shift+Tab → focus_prev（焦点循环）；DemoRoot 高亮获焦组件
                let s = *shift.borrow();
                let fid = if s {
                    focus.borrow_mut().focus_prev()
                } else {
                    focus.borrow_mut().focus_next()
                };
                eprintln!("[focus] Tab(shift={s}) -> {:?}", fid.map(|w| w.0));
                Some(DemoMsg::Focus(fid))
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let (x, y) = *cursor.borrow();
                match hit_test(x, y, &regions) {
                    Some(1) => Some(DemoMsg::Accordion(AccordionMsg::Toggle)),
                    Some(2) => Some(DemoMsg::Badge(WaBadgeMsg::Click)),
                    _ => None,
                }
            }
            _ => None,
        }
    };

    // 初始状态：默认收起；`--expanded` 则 Accordion 初始展开（qa 截"收起 vs 展开"对比）
    let mut state = DemoRootState::default();
    if std::env::args().any(|a| a == "--expanded") {
        state.accordion.expanded = true;
    }

    App::run(config, DemoRoot, state, mapper)?;
    Ok(())
}
