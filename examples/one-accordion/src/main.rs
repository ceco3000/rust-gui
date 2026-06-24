//! Accordion 组件演示 —— Tier 2 架构（.rgui + .rhai）
//!
//! 展示用声明式格式定义的手风琴容器组件。
//! AC07: 点击标题栏 → expanded 状态翻转 → paint 脚本重新执行。
//! AC09: mode="single"/"single-collapsible" 兄弟联动——展开一个 item 自动折叠其他。
//!
//! 交互通过 `register_interaction` 注册点击回调。

use std::sync::{Arc, Mutex};

use rgui::app::{App, AppConfig};
use rgui_core::{
    geometry::Rect,
    traits::AppMessage,
    view::{PropValue, WidgetView},
};
use rgui_render::compute_view_layout;

/// Accordion demo 消息类型
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum AccordionMsg {
    /// Toggle 指定 index 的 AccordionItem expanded 状态
    Toggle(usize),
}

impl AppMessage for AccordionMsg {
    fn message_name(&self) -> &'static str {
        match self {
            AccordionMsg::Toggle(_) => "AccordionMsg::Toggle",
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 解析 .rgui
    let rgui_path = std::path::PathBuf::from("ui.rgui");
    let mut view: WidgetView<AccordionMsg> =
        rgui_devtools::rgui_parser::parse_rgui_file(&rgui_path)?;

    // 2. 初始布局（Tier 2 脚本需要 bounds，AC02）
    let initial_layout = compute_view_layout(
        &mut view,
        rgui_core::geometry::Size::new(500.0, 400.0),
        None,
    );

    // 3. 执行 Tier 2 Rhai paint 脚本
    rgui::paint_factory::execute_tier2_paint_scripts(&mut view, &initial_layout);

    // 4. 收集 AccordionItem 的 WidgetId（用于交互注册）
    let item_ids: Vec<_> = collect_accordion_item_ids(&view);

    // 5. 创建 App
    let config = AppConfig::default()
        .title("Accordion Demo — Tier 2 (.rgui + .rhai)")
        .window_size(500.0, 400.0);

    let mut app = App::new(config);

    // 5a. 注册 AccordionItem 点击交互（AC09: mode 协调）
    let mode = read_accordion_mode(&view);

    let expanded_states: Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(vec![
        true,  // Item 0 (Getting Started) 初始展开
        false, // Item 1 (Installation) 初始折叠
        false, // Item 2 (API Reference) 初始折叠（disabled）
    ]));

    for (i, &item_id) in item_ids.iter().enumerate() {
        let states = Arc::clone(&expanded_states);
        // 从布局引擎获取 item 的 bounds
        let item_rect = item_bounds(&initial_layout, item_id);
        let mode_clone = mode.clone();

        app.register_interaction(item_id, item_rect, "toggle", move |_action| {
            if let Ok(mut expanded) = states.lock() {
                if i < expanded.len() {
                    match mode_clone.as_str() {
                        "single" => {
                            if !expanded[i] {
                                // Expanding this item: collapse all others
                                for j in 0..expanded.len() {
                                    expanded[j] = false;
                                }
                                expanded[i] = true;
                            }
                            // In "single" mode, can't collapse the only open item
                        },
                        "single-collapsible" => {
                            if expanded[i] {
                                // Collapsing this item
                                expanded[i] = false;
                            } else {
                                // Expanding: collapse all others first
                                for j in 0..expanded.len() {
                                    expanded[j] = false;
                                }
                                expanded[i] = true;
                            }
                        },
                        _ => {
                            // "multiple" mode (default): independent toggle
                            expanded[i] = !expanded[i];
                        },
                    }
                }
            }
        });
    }

    // 6. 设置视图场景构建器（每帧回调）
    let template = view;
    let paint_fn: rgui_render::PaintFn<AccordionMsg> = Box::new(|_view, _bounds| Vec::new());
    let states = Arc::clone(&expanded_states);

    app.set_view_scene_builder(move |frame, width, height, tr| {
        let mut v = template.clone();
        let layout = compute_view_layout(
            &mut v,
            rgui_core::geometry::Size::new(f64::from(width), f64::from(height)),
            Some(tr),
        );

        // AC07: 注入 expanded props（从 shared state 读取最新值）
        if let Ok(expanded) = states.lock() {
            inject_expanded_props(&mut v, &expanded);
        }

        // AC07: 重新执行 Tier 2 paint 脚本
        rgui::paint_factory::execute_tier2_paint_scripts(&mut v, &layout);

        rgui_render::build_scene_from_view(&v, &layout, &paint_fn, frame, Some(tr))
    });

    // 7. 加载 Rhai 事件脚本（占位）
    let rhai_path = std::path::PathBuf::from("handlers.rhai");
    if rhai_path.exists() {
        app.load_rhai_scripts(&[rhai_path.as_path()])?;
    }

    // 8. 启动事件循环
    app.run()
}

/// 从布局引擎获取 widget 的窗口绝对坐标 bounds（用于 hit test 交互区域注册）。
fn item_bounds(engine: &rgui_layout::LayoutEngine, id: rgui_core::id::WidgetId) -> Rect {
    let pos = engine
        .absolute_position(id)
        .unwrap_or(rgui_core::geometry::Point::new(0.0, 0.0));
    let size =
        engine
            .get_layout(id)
            .map_or(rgui_core::geometry::Size::new(100.0, 40.0), |cached| {
                rgui_core::geometry::Size::new(cached.result.size.width, cached.result.size.height)
            });
    Rect::new(pos.x, pos.y, size.width, size.height)
}

/// DFS 收集 AccordionItem 的 WidgetId（用于交互注册）。
fn collect_accordion_item_ids<M: AppMessage>(view: &WidgetView<M>) -> Vec<rgui_core::id::WidgetId> {
    let mut ids = Vec::new();
    collect_ids_recursive(view, &mut ids);
    ids
}

fn collect_ids_recursive<M: AppMessage>(
    view: &WidgetView<M>,
    ids: &mut Vec<rgui_core::id::WidgetId>,
) {
    if view.widget_type == "WaAccordionItem" {
        if let Some(id) = view.id {
            ids.push(id);
        }
    }
    for child in &view.children {
        collect_ids_recursive(child, ids);
    }
}

/// AC07: 将 expanded 状态注入到 WidgetView 树的 AccordionItem props 中。
fn inject_expanded_props<M: AppMessage>(view: &mut WidgetView<M>, states: &[bool]) {
    let mut idx = 0;
    inject_recursive(view, states, &mut idx);
}

fn inject_recursive<M: AppMessage>(view: &mut WidgetView<M>, states: &[bool], idx: &mut usize) {
    if view.widget_type == "WaAccordionItem" {
        if *idx < states.len() {
            view.props.insert("expanded", PropValue::Bool(states[*idx]));
        }
        *idx += 1;
    }
    for child in &mut view.children {
        inject_recursive(child, states, idx);
    }
}

/// AC09: 从 WidgetView 树中读取 Accordion 容器的 mode prop。
fn read_accordion_mode<M: AppMessage>(view: &WidgetView<M>) -> String {
    if view.widget_type == "WaAccordion" {
        return view
            .props
            .get("mode")
            .and_then(|v| match v {
                PropValue::Str(s) => Some(s.to_string()),
                _ => None,
            })
            .unwrap_or_else(|| "multiple".to_string());
    }
    for child in &view.children {
        let mode = read_accordion_mode(child);
        if mode != "multiple" {
            return mode;
        }
    }
    "multiple".to_string()
}

/// AC08: 展开全部非 disabled 的 AccordionItem。
///
/// `mode="single"` 时此函数为 no-op（单模式下只能展开一项）。
/// 返回 `true` 表示有状态变更，调用方应调用 `app.request_redraw()` 触发重绘。
#[allow(dead_code)]
fn expand_all_items<M: AppMessage>(view: &WidgetView<M>, states: &Arc<Mutex<Vec<bool>>>) -> bool {
    let mode = read_accordion_mode(view);
    if mode == "single" {
        // 单模式下无法同时展开全部项
        return false;
    }
    let disabled_map = collect_disabled_map(view);
    if let Ok(mut expanded) = states.lock() {
        let mut changed = false;
        for (i, is_expanded) in expanded.iter_mut().enumerate() {
            if !disabled_map.get(&i).copied().unwrap_or(false) && !*is_expanded {
                *is_expanded = true;
                changed = true;
            }
        }
        changed
    } else {
        false
    }
}

/// AC08: 折叠全部 AccordionItem。
///
/// 返回 `true` 表示有状态变更，调用方应调用 `app.request_redraw()` 触发重绘。
#[allow(dead_code)]
fn collapse_all_items(states: &Arc<Mutex<Vec<bool>>>) -> bool {
    if let Ok(mut expanded) = states.lock() {
        let mut changed = false;
        for is_expanded in expanded.iter_mut() {
            if *is_expanded {
                *is_expanded = false;
                changed = true;
            }
        }
        changed
    } else {
        false
    }
}

/// AC08: 从 WidgetView 树中收集 AccordionItem 的 disabled 状态。
///
/// 返回 HashMap<index, bool>，index 对应 items 在 DFS 中的出现顺序。
#[allow(dead_code)]
fn collect_disabled_map<M: AppMessage>(
    view: &WidgetView<M>,
) -> std::collections::HashMap<usize, bool> {
    let mut map = std::collections::HashMap::new();
    let mut idx = 0;
    collect_disabled_recursive(view, &mut map, &mut idx);
    map
}

#[allow(dead_code)]
fn collect_disabled_recursive<M: AppMessage>(
    view: &WidgetView<M>,
    map: &mut std::collections::HashMap<usize, bool>,
    idx: &mut usize,
) {
    if view.widget_type == "WaAccordionItem" {
        let disabled = view
            .props
            .get("disabled")
            .and_then(|v| match v {
                PropValue::Bool(b) => Some(*b),
                _ => None,
            })
            .unwrap_or(false);
        map.insert(*idx, disabled);
        *idx += 1;
    }
    for child in &view.children {
        collect_disabled_recursive(child, map, idx);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use rgui_core::id::WidgetId;

    use super::*;

    /// 创建测试用的 Accordion WidgetView 树。
    fn make_test_view<M: AppMessage>() -> WidgetView<M> {
        WidgetView {
            id: Some(WidgetId::new()),
            widget_type: "WaAccordion",
            props: {
                let mut m = std::collections::BTreeMap::new();
                m.insert("mode", PropValue::Str("multiple".into()));
                m
            },
            children: Vec::new(),
            key: None,
            message_bindings: Vec::new(),
        }
    }

    /// 创建测试用的 AccordionItem WidgetView。
    fn make_item<M: AppMessage>(id: Option<WidgetId>) -> WidgetView<M> {
        WidgetView {
            id,
            widget_type: "WaAccordionItem",
            props: std::collections::BTreeMap::new(),
            children: Vec::new(),
            key: None,
            message_bindings: Vec::new(),
        }
    }

    /// 创建测试用的 disabled AccordionItem。
    fn make_disabled_item<M: AppMessage>(id: Option<WidgetId>) -> WidgetView<M> {
        WidgetView {
            id,
            widget_type: "WaAccordionItem",
            props: {
                let mut m = std::collections::BTreeMap::new();
                m.insert("disabled", PropValue::Bool(true));
                m
            },
            children: Vec::new(),
            key: None,
            message_bindings: Vec::new(),
        }
    }

    /// AC08: collapse_all_items 将全部展开的项折叠。
    #[test]
    fn test_collapse_all() {
        let states = Arc::new(Mutex::new(vec![true, true, false]));
        let changed = collapse_all_items(&states);
        assert!(changed);
        let result = states.lock().unwrap();
        assert_eq!(&*result, &[false, false, false]);
    }

    /// AC08: collapse_all_items 全部已折叠时无变更。
    #[test]
    fn test_collapse_all_noop() {
        let states = Arc::new(Mutex::new(vec![false, false, false]));
        let changed = collapse_all_items(&states);
        assert!(!changed);
    }

    /// AC08: expand_all_items 模式为 "single" 时 no-op。
    #[test]
    fn test_expand_all_single_mode_noop() {
        let mut view = WidgetView::<AccordionMsg> {
            id: Some(WidgetId::new()),
            widget_type: "WaAccordion",
            props: {
                let mut m = std::collections::BTreeMap::new();
                m.insert("mode", PropValue::Str("single".into()));
                m
            },
            children: Vec::new(),
            key: None,
            message_bindings: Vec::new(),
        };
        // 添加一个 AccordionItem
        view.children.push(make_item(Some(WidgetId::new())));

        let states = Arc::new(Mutex::new(vec![false]));
        let changed = expand_all_items(&view, &states);
        assert!(!changed, "single mode 下 expand_all 应为 no-op");
        let result = states.lock().unwrap();
        assert_eq!(&*result, &[false], "状态不应被修改");
    }

    /// AC08: expand_all_items 跳过 disabled 项。
    #[test]
    fn test_expand_all_skips_disabled() {
        let mut view = make_test_view::<AccordionMsg>();
        view.children.push(make_item(Some(WidgetId::new())));
        view.children
            .push(make_disabled_item(Some(WidgetId::new())));

        let states = Arc::new(Mutex::new(vec![false, false]));
        let changed = expand_all_items(&view, &states);
        assert!(changed);
        let result = states.lock().unwrap();
        assert_eq!(result[0], true, "非 disabled 项应展开");
        assert_eq!(result[1], false, "disabled 项应保持折叠");
    }

    /// AC08: expand_all_items 全部已展开时无变更。
    #[test]
    fn test_expand_all_already_expanded() {
        let mut view = make_test_view::<AccordionMsg>();
        view.children.push(make_item(Some(WidgetId::new())));

        let states = Arc::new(Mutex::new(vec![true]));
        let changed = expand_all_items(&view, &states);
        assert!(!changed);
    }
}
