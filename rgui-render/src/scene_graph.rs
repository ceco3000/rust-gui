//! 场景图——绘制指令列表（greenfield §B.2 / §C.2）。
//!
//! 单一场景图路径：`SceneGraph` 持有一个 **纯 Rust 绘制指令列表**（矩形/文本原语），
//! 从 `rgui_core::view::WidgetView` 转换而来。不暴露任何 wgpu/vello 类型到公共 API；
//! 真正的 GPU 绘制在 `vello` 模块消费此指令列表完成。

use rgui_core::geometry::Size;
use rgui_core::layout::{LayoutEngine, LayoutResult, LayoutStyle};
use rgui_core::view::{Color, PropValue, WidgetView};

/// 无显式容器时使用的默认布局容器尺寸（D6：保证有可布局空间）。
const DEFAULT_CONTAINER: Size = Size::new(200.0, 200.0);

/// 绘制指令。
#[derive(Debug, Clone, PartialEq)]
pub enum DrawCmd {
    /// 填充矩形。
    FillRect {
        /// x 坐标。
        x: f32,
        /// y 坐标。
        y: f32,
        /// 宽度。
        width: f32,
        /// 高度。
        height: f32,
        /// 颜色。
        color: Color,
    },
    /// 绘制文本（D5 占位：text/字号/颜色；D17 加 width 供换行）。
    DrawText {
        /// x 坐标。
        x: f32,
        /// y 坐标。
        y: f32,
        /// 文本内容。
        text: String,
        /// 字号。
        size: f32,
        /// 颜色。
        color: Color,
        /// 文本区域可用宽度（逻辑；>0 换行，0/负 = 单行不截断）。
        width: f32,
    },
    /// 描边矩形（D16：获焦边框高亮；stroke_width 为描边宽度）。
    StrokeRect {
        /// x 坐标。
        x: f32,
        /// y 坐标。
        y: f32,
        /// 宽度。
        width: f32,
        /// 高度。
        height: f32,
        /// 描边颜色。
        color: Color,
        /// 描边宽度。
        stroke_width: f32,
    },
}

/// 场景图（绘制指令列表）。
#[derive(Debug, Clone, Default)]
pub struct SceneGraph {
    /// 绘制指令列表。
    cmds: Vec<DrawCmd>,
}

impl SceneGraph {
    /// 构造空场景图。
    pub fn new() -> Self {
        Self::default()
    }

    /// 从现成绘制指令构造（测试/程序化场景用）。
    pub fn from_cmds(cmds: Vec<DrawCmd>) -> Self {
        Self { cmds }
    }

    /// 从 `WidgetView` 转换（真实转换，D6）：
    ///
    /// - 递归遍历视图树。
    /// - 用 [`LayoutEngine`] 计算子节点**真实 bounds**（布局真正作用于渲染）。
    /// - 把节点 props 映射为绘制指令：
    ///   - `PropValue::Color(c)` → `FillRect`（用布局尺寸/位置）
    ///   - `PropValue::Str(s)` → `DrawText`
    ///   - `Unit/Bool/Int/Float/WidgetId` → 无图元（或尺寸提示）
    ///
    /// 容器尺寸：优先取根节点 `size` 建议，否则用 `DEFAULT_CONTAINER`（保证有可布局空间）。
    pub fn from_view<M>(view: &WidgetView<M>) -> Self {
        let mut graph = Self::new();
        let container = view.size.unwrap_or(DEFAULT_CONTAINER);
        let slot = LayoutResult::new(container, rgui_core::geometry::Point::new(0, 0));
        // 根节点：以容器为 bounds 起点（根节点 slot 复用容器尺寸）
        graph.emit_node(view, slot);
        graph
    }

    /// 为单个节点产生绘制指令，并对子节点递归布局。
    fn emit_node<M>(&mut self, view: &WidgetView<M>, slot: LayoutResult) {
        // 提取子节点建议尺寸
        let child_sizes: Vec<Size> = view
            .children
            .iter()
            .map(|c| c.size.unwrap_or_else(|| Size::new(100.0, 40.0)))
            .collect();

        // 用布局引擎计算每个子节点的真实 bounds（在根 slot 内）
        // D23 残留：容器 LayoutStyle（方向 Column/Row + padding）由 view 声明，供 compute_children 布局
        let engine = LayoutEngine::new();
        let container_style = LayoutStyle::default()
            .with_direction(view.layout_direction.unwrap_or_default())
            .with_padding(view.padding);
        let child_slots = engine.compute_children(slot.size, &child_sizes, container_style);

        // 本节点图元（根据 props）
        match &view.props {
            PropValue::Color(color) => {
                let size = view.size.unwrap_or(slot.size);
                self.cmds.push(DrawCmd::FillRect {
                    x: slot.position.x as f32,
                    y: slot.position.y as f32,
                    width: size.width,
                    height: size.height,
                    color: *color,
                });
            }
            PropValue::Str(text) => {
                let size = view
                    .size
                    .unwrap_or_else(|| Size::new(size_hint(text), 20.0));
                // D23：字号用语义正文（Body 13pt 默认）而非 view.size.height（原误用容器高做字号）
                let font_size = view.font_size.unwrap_or(13.0);
                self.cmds.push(DrawCmd::DrawText {
                    x: slot.position.x as f32,
                    // D23 缺陷3：文字垂直居中——y = slot 顶 + (节点高 - 字号)/2（原 y=slot 顶未居中）
                    y: slot.position.y as f32 + (size.height - font_size) / 2.0,
                    text: text.clone(),
                    size: font_size,
                    // D23：语义前景色（非硬编码纯白；默认浅前景 #E8E8E8）
                    color: view.foreground.unwrap_or_else(|| Color::rgb(232, 232, 232)),
                    // 文本区域宽（D17 供 shapeLine 换行；>0 换行）
                    width: size.width,
                });
            }
            _ => {}
        }

        // 递归子节点（把父节点 slot.position 累加到子节点相对位置，P2-1 修复）
        for (child, child_slot) in view.children.iter().zip(child_slots.iter()) {
            let accumulated = LayoutResult::new(
                child_slot.size,
                rgui_core::geometry::Point::new(
                    slot.position.x + child_slot.position.x,
                    slot.position.y + child_slot.position.y,
                ),
            );
            self.emit_node(child, accumulated);
        }

        // 描边焦点环（D23 缺陷2 深挖：内缩半宽 + **画在子节点之后**——原画序在子节点前，
        // header/content 子视图后画会覆盖焦点环顶边/上部（仅露底部横线），因此焦点环必须最后画（最上层））。
        // 内缩半宽使描边落在组件 bounds 内、四边完整。
        if let Some(b) = &view.border {
            let inset = b.width / 2.0;
            let size = view.size.unwrap_or(slot.size);
            self.cmds.push(DrawCmd::StrokeRect {
                x: slot.position.x as f32 + inset,
                y: slot.position.y as f32 + inset,
                width: size.width - 2.0 * inset,
                height: size.height - 2.0 * inset,
                color: b.color,
                stroke_width: b.width,
            });
        }
    }

    /// 构造一个填满 `w x h` 的红色矩形场景（离屏演示/测试用）。
    pub fn red_filled_rect(w: f32, h: f32) -> Self {
        Self {
            cmds: vec![DrawCmd::FillRect {
                x: 0.0,
                y: 0.0,
                width: w,
                height: h,
                color: Color::rgb(255, 0, 0),
            }],
        }
    }

    /// 绘制指令列表（只读）。
    pub fn cmds(&self) -> &[DrawCmd] {
        &self.cmds
    }

    /// 追加一条绘制指令。
    pub fn push(&mut self, cmd: DrawCmd) {
        self.cmds.push(cmd);
    }
}

/// 文本宽度粗略提示（字符数 * 字号系数，用于布局）。
fn size_hint(text: &str) -> f32 {
    text.chars().count() as f32 * 20.0
}
