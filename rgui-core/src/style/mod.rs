//! 样式系统子模块（由 `rgui-style` 并入 `rgui-core`，greenfield 裁决 §F）。
//!
//! `.rgss` 解析是纯文本解析（非重型运行时）；并入 core 后走单一路径（热重载 = P1，§G）。
//!
//! ## D3 阶段 0 范围
//!
//! 仅占位：`StyleSheet` / `parse_rgss` 类型骨架。**不引入 cssparser / notify**。
//! 无 `hot_reload`/`parser`/`theme` 子模块（阶段 0 仅占位，样式解析/主题/热重载在 D4+ 补全）。
//! 类型为占位定义，故模块级静默未使用告警，避免噪音。

#![allow(dead_code)]

/// 样式表。
#[derive(Debug, Clone, Default)]
pub struct StyleSheet {
    /// 规则列表。
    pub rules: Vec<StyleRule>,
}

/// 样式规则。
#[derive(Debug, Clone, Default)]
pub struct StyleRule {
    /// 选择器串（占位）。
    pub selector: String,
}

/// 解析 `.rgss` 文本。D3 占位；实际解析（cssparser）在实现阶段补全。
pub fn parse_rgss(_src: &str) -> StyleSheet {
    StyleSheet::default()
}
