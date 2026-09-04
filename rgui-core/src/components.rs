//! 组件子模块（统一 Tier 1 WidgetSpec，契约 greenfield §B.1）。
//!
//! D10：Accordion（折叠/展开容器）+ WaBadge（徽章/标签），实现 `WidgetSpec`
//! 完整生命周期 view/update/measure/paint，纯 Rust、零 GPU/平台。

use crate::context::{MeasureContext, PaintContext, UpdateContext, ViewContext};
use crate::geometry::{BoxConstraints, Rect, Size};
use crate::traits::{AppMessage, PersistState, WidgetSpec};
use crate::view::{Color, PropValue, WidgetView};
use std::any::Any;

// ===== Accordion =====

/// Accordion 消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccordionMsg {
    /// 点击标题，切换展开/收起。
    Toggle,
}

impl AppMessage for AccordionMsg {
    fn message_name(&self) -> &'static str {
        match self {
            AccordionMsg::Toggle => "accordion.toggle",
        }
    }
}

/// Accordion 状态。
#[derive(Debug, Clone, PartialEq)]
pub struct AccordionState {
    /// 标题。
    pub title: String,
    /// 展开后的内容说明。
    pub subtitle: String,
    /// 是否展开。
    pub expanded: bool,
}

impl Default for AccordionState {
    fn default() -> Self {
        Self {
            title: "Accordion".to_string(),
            subtitle: "details".to_string(),
            expanded: false,
        }
    }
}

impl PersistState for AccordionState {
    fn schema_name() -> &'static str {
        "accordion_state"
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

/// Accordion 组件（Tier 1 WidgetSpec）。
#[derive(Debug, Default, Clone)]
pub struct Accordion;

impl Accordion {
    /// 构造默认状态。
    pub fn initial_state() -> AccordionState {
        AccordionState::default()
    }
}

impl WidgetSpec for Accordion {
    type State = AccordionState;
    type Message = AccordionMsg;

    fn name(&self) -> &'static str {
        "accordion"
    }

    fn view(&self, state: &Self::State, _ctx: &ViewContext) -> WidgetView<Self::Message> {
        let mut root = WidgetView::empty();
        let content_h = if state.expanded { 130.0 } else { 44.0 };
        root.size = Some(Size::new(340.0, content_h));

        // 标题行（可点击，显示展开/收起状态标记）
        let mut header = WidgetView::empty();
        header.props = PropValue::Color(Color::rgb(90, 130, 220));
        header.size = Some(Size::new(340.0, 36.0));
        let marker = if state.expanded { "-" } else { "+" };
        let mut title = WidgetView::empty();
        title.props = PropValue::Str(format!("{} [{}]", state.title, marker));
        title.size = Some(Size::new(320.0, 28.0));
        header.children.push(title);
        root.children.push(header);

        // 内容（仅展开时显示）
        if state.expanded {
            let mut content = WidgetView::empty();
            content.props = PropValue::Str(format!("⌄ {}", state.subtitle));
            content.size = Some(Size::new(340.0, 84.0));
            root.children.push(content);
        }
        root
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _ctx: &mut UpdateContext) {
        match msg {
            AccordionMsg::Toggle => state.expanded = !state.expanded,
        }
    }

    fn measure(&self, state: &Self::State, _c: BoxConstraints, _ctx: &MeasureContext) -> Size {
        let h = if state.expanded { 130.0 } else { 44.0 };
        Size::new(340.0, h)
    }

    fn paint(&self, _state: &Self::State, _b: Rect, _ctx: &mut PaintContext) {}
}

// ===== WaBadge =====

/// WaBadge 消息（点击计数）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaBadgeMsg {
    /// 点击徽章，计数 +1。
    Click,
}

impl AppMessage for WaBadgeMsg {
    fn message_name(&self) -> &'static str {
        match self {
            WaBadgeMsg::Click => "wa_badge.click",
        }
    }
}

/// WaBadge 状态（整数 label）。
#[derive(Debug, Clone, PartialEq)]
pub struct WaBadgeState {
    /// 徽章文本。
    pub label: String,
    /// 数值（label 显示）。
    pub count: u32,
}

impl Default for WaBadgeState {
    fn default() -> Self {
        Self {
            label: "badge".to_string(),
            count: 0,
        }
    }
}

impl PersistState for WaBadgeState {
    fn schema_name() -> &'static str {
        "wa_badge_state"
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

/// WaBadge 组件（Tier 1 WidgetSpec）。
#[derive(Debug, Default, Clone)]
pub struct WaBadge;

impl WaBadge {
    /// 构造默认状态。
    pub fn initial_state() -> WaBadgeState {
        WaBadgeState::default()
    }
}

impl WidgetSpec for WaBadge {
    type State = WaBadgeState;
    type Message = WaBadgeMsg;

    fn name(&self) -> &'static str {
        "wa_badge"
    }

    fn view(&self, state: &Self::State, _ctx: &ViewContext) -> WidgetView<Self::Message> {
        let mut root = WidgetView::empty();
        root.size = Some(Size::new(160.0, 40.0));
        root.props = PropValue::Color(Color::rgb(120, 160, 210));
        let mut label = WidgetView::empty();
        label.props = PropValue::Str(format!("{}: {}", state.label, state.count));
        label.size = Some(Size::new(150.0, 26.0));
        root.children.push(label);
        root
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _ctx: &mut UpdateContext) {
        match msg {
            WaBadgeMsg::Click => state.count += 1,
        }
    }

    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _ctx: &MeasureContext) -> Size {
        Size::new(160.0, 40.0)
    }

    fn paint(&self, _state: &Self::State, _b: Rect, _ctx: &mut PaintContext) {}
}

// 保持 Color 导入有效（Accordion/WaBadge 视图用 Color 背景）。
#[allow(dead_code)]
fn _color_marker(_c: Color) {}
