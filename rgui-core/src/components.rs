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

    fn view(&self, state: &Self::State, ctx: &ViewContext) -> WidgetView<Self::Message> {
        let mut root = WidgetView::empty();
        let content_h = if state.expanded { 130.0 } else { 44.0 };
        root.size = Some(Size::new(340.0, content_h));
        // D23 残留 P1-1：Accordion 内部纵向（标题行在上、内容在下），非横向并排 header+content
        root.layout_direction = Some(crate::layout::LayoutDirection::Column);
        // 样式驱动（D19/D23）：查样式表（默认主题回退）→ 获焦描边（accent systemBlue，pad 参数化）
        let style = ctx.styles.lookup("accordion");
        root.border = if ctx.focused {
            Some(
                crate::view::Border::new(
                    style.effective_border_color(Color::rgb(0, 122, 255)),
                    style.effective_border_width(3.0),
                )
                .with_pad(style.effective_border_pad(2.0)),
            )
        } else {
            None
        };
        // 前景/字号默认（D23：浅前景 #E8E8E8 + Body 13pt）
        let fg = style.effective_foreground(Color::rgb(232, 232, 232));
        let fs = style.effective_font_size(13.0);

        // 标题行（可点击，显示 macOS chevron：展开 ▾ / 收起 ▸）
        let mut header = WidgetView::empty();
        header.props = PropValue::Color(style.effective_background(Color::rgb(58, 58, 58)));
        header.size = Some(Size::new(340.0, 36.0));
        let chevron = if state.expanded { "▾" } else { "▸" };
        let mut title = WidgetView::empty();
        title.props = PropValue::Str(format!("{} {}", state.title, chevron));
        title.font_size = Some(fs);
        title.foreground = Some(fg);
        title.size = Some(Size::new(320.0, 36.0)); // 高度铺满 header（36），文字垂直居中
        header.children.push(title);
        root.children.push(header);

        // 内容（仅展开时显示；正文 Callout/Body 级小字号 + 语义前景，防溢出）
        if state.expanded {
            let mut content = WidgetView::empty();
            content.props = PropValue::Str(state.subtitle.to_string());
            content.font_size = Some(12.0); // Callout 12pt
            content.foreground = Some(fg);
            content.size = Some(Size::new(300.0, 84.0));
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

    fn focusable(&self) -> bool {
        true
    }
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

    fn view(&self, state: &Self::State, ctx: &ViewContext) -> WidgetView<Self::Message> {
        let mut root = WidgetView::empty();
        root.size = Some(Size::new(160.0, 40.0));
        // 样式驱动（D19/D23）：查样式表（默认主题回退）→ 控件灰背景 + accent 获焦描边
        let style = ctx.styles.lookup("wa_badge");
        root.props = PropValue::Color(style.effective_background(Color::rgb(58, 58, 58)));
        root.border = if ctx.focused {
            Some(
                crate::view::Border::new(
                    style.effective_border_color(Color::rgb(0, 122, 255)),
                    style.effective_border_width(3.0),
                )
                .with_pad(style.effective_border_pad(2.0)),
            )
        } else {
            None
        };
        let mut label = WidgetView::empty();
        label.props = PropValue::Str(format!("{}: {}", state.label, state.count));
        label.font_size = Some(style.effective_font_size(13.0));
        label.foreground = Some(style.effective_foreground(Color::rgb(232, 232, 232)));
        label.size = Some(Size::new(150.0, 40.0)); // 高度铺满 badge（40），文字垂直居中
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

    fn focusable(&self) -> bool {
        true
    }
}

// 保持 Color 导入有效（Accordion/WaBadge 视图用 Color 背景）。
#[allow(dead_code)]
fn _color_marker(_c: Color) {}
