//! 渲染门面（保留模块，契约 §1.3 F）。D3 阶段 0：占位。

use rgui_render::SceneGraph;

/// 渲染器（D3 占位）。
#[derive(Debug, Default)]
pub struct Renderer {
    /// 场景图。
    pub scene: Option<SceneGraph>,
}

impl Renderer {
    /// 构造空渲染器。
    pub fn new() -> Self {
        Self::default()
    }
}
