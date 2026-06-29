//! Accordion 框架层核心模块——mode 协调 + 键盘导航 + 事件体系 + @method API + disabled + 属性同步。
//!
//! 翻译自 WA `<wa-accordion>` + `<wa-accordion-item>` 的全部公开 API。
//! 所有交互逻辑集中于此模块，组件目录（rgui-components/src/）仅存放 `.rgui` + `.rhai` 视觉脚本。
//!
//! # 架构
//!
//! - `AccordionContext`：容器/子项拓扑关系缓存
//! - `init()`：入口函数，执行全部注册
//! - R3: mode 协调（single / single-collapsible / multiple）
//! - R4: 父→子属性同步（iconPlacement / headingLevel / appearance）
//! - R5: disabled 拦截
//! - R6: 键盘导航 roving tabindex
//! - R7: 事件体系（wa-expand / wa-collapse / wa-accordion-item-trigger / after 事件）
//! - R8: @method API（expand / collapse / toggle / expandAll / collapseAll / focus）

use std::collections::HashMap;
use std::sync::Arc;

use rgui_core::context::UpdateContext;
use rgui_core::id::WidgetId;
use rgui_core::interaction::InteractionHost;
use rgui_core::traits::{AppMessage, EventResult};
use rgui_core::view::{PropValue, WidgetView};

// ============================================================================
// SubTask 2.1: AccordionContext + 容器/子项识别 + mode 协调 (R3)
// ============================================================================

/// Accordion 拓扑关系缓存——容器 ID → 子 item ID 列表 + 反向映射。
#[derive(Clone, Debug, Default)]
pub struct AccordionContext {
    /// 容器 WidgetId → 有序子 AccordionItem WidgetId 列表
    pub containers: HashMap<WidgetId, Vec<WidgetId>>,
    /// 子 AccordionItem WidgetId → 父容器 WidgetId
    pub item_to_parent: HashMap<WidgetId, WidgetId>,
    /// 容器 WidgetId → mode 字符串（"single" | "single-collapsible" | "multiple"）
    pub container_modes: HashMap<WidgetId, String>,
    /// 子 AccordionItem WidgetId → 是否 disabled
    pub item_disabled: HashMap<WidgetId, bool>,
}

impl AccordionContext {
    /// 获取容器的 mode，默认 "multiple"
    pub fn mode_of(&self, container_id: WidgetId) -> &str {
        self.container_modes
            .get(&container_id)
            .map(|s| s.as_str())
            .unwrap_or("multiple")
    }

    /// 判断 item 是否 disabled
    pub fn is_item_disabled(&self, item_id: WidgetId) -> bool {
        self.item_disabled.get(&item_id).copied().unwrap_or(false)
    }

    /// 获取 item 的父容器 mode
    pub fn parent_mode(&self, item_id: WidgetId) -> Option<&str> {
        self.item_to_parent
            .get(&item_id)
            .and_then(|&container_id| self.container_modes.get(&container_id))
            .map(|s| s.as_str())
    }
}

/// 入口函数——收集节点、注册全部 handler。
pub fn init<M: AppMessage>(app: &mut impl InteractionHost, view: &WidgetView<M>) {
    let ctx = collect_accordion_nodes(view);
    let ctx = Arc::new(ctx);

    // AC14: 将 props 中的初始 expanded 值同步到 WidgetStateStore，
    // 避免 handler 读取 store（空→false）与视觉（props 初始值 true）不一致。
    sync_initial_expanded_state(view, &ctx, app.widget_state_store());

    register_mode_coordination(app, &ctx);
    register_keyboard_navigation(app, &ctx);
    register_event_system(app, &ctx);
    register_method_api(app, &ctx);
}

/// 递归收集 Accordion 容器和 AccordionItem 叶子节点。
///
/// 通过 `_rhai_path` prop 识别：
/// - 包含 "accordion.rhai" → 容器
/// - 包含 "accordionitem" → 叶子
fn collect_accordion_nodes<M: AppMessage>(view: &WidgetView<M>) -> AccordionContext {
    let mut ctx = AccordionContext::default();
    collect_recursive(view, None, &mut ctx);
    ctx
}

/// AC14: 将 props 中的初始 expanded 值同步到 WidgetStateStore。
fn sync_initial_expanded_state<M: AppMessage>(
    view: &WidgetView<M>,
    ctx: &AccordionContext,
    store: &rgui_core::widget_state::WidgetStateStore,
) {
    sync_initial_expanded_recursive(view, ctx, store);
}

fn sync_initial_expanded_recursive<M: AppMessage>(
    view: &WidgetView<M>,
    ctx: &AccordionContext,
    store: &rgui_core::widget_state::WidgetStateStore,
) {
    if let Some(id) = view.id {
        if ctx.item_to_parent.contains_key(&id) {
            // 此节点是 AccordionItem，同步 expanded 初始值
            let initial_expanded = view
                .props
                .get("expanded")
                .and_then(|v| match v {
                    PropValue::Bool(b) => Some(*b),
                    _ => None,
                })
                .unwrap_or(false);
            store.insert(id, initial_expanded);
        }
    }
    for child in &view.children {
        sync_initial_expanded_recursive(child, ctx, store);
    }
}

fn collect_recursive<M: AppMessage>(
    view: &WidgetView<M>,
    parent_container: Option<WidgetId>,
    ctx: &mut AccordionContext,
) {
    let rhai_path = view
        .props
        .get("_rhai_path")
        .and_then(|v| match v {
            PropValue::Str(s) => Some(s.as_ref()),
            _ => None,
        });

    let is_container = rhai_path.is_some_and(|s| s.contains("accordion.rhai"));
    let is_item = rhai_path.is_some_and(|s| s.contains("accordionitem"));

    if let Some(widget_id) = view.id {
        if is_container {
            // 初始化容器的 item 列表
            ctx.containers.entry(widget_id).or_default();

            // 读取 mode
            let mode = view
                .props
                .get("mode")
                .and_then(|v| match v {
                    PropValue::Str(s) => Some(s.to_string()),
                    _ => None,
                })
                .unwrap_or_else(|| "multiple".to_string());
            ctx.container_modes.insert(widget_id, mode);

            // 递归子节点，标记此容器为父容器
            for child in &view.children {
                collect_recursive(child, Some(widget_id), ctx);
            }
            return;
        }

        if is_item {
            if let Some(container_id) = parent_container {
                // 注册到容器
                ctx.containers
                    .entry(container_id)
                    .or_default()
                    .push(widget_id);
                ctx.item_to_parent.insert(widget_id, container_id);

                // 读取 disabled
                let disabled = view
                    .props
                    .get("disabled")
                    .and_then(|v| match v {
                        PropValue::Bool(b) => Some(*b),
                        _ => None,
                    })
                    .unwrap_or(false);
                ctx.item_disabled.insert(widget_id, disabled);
            }
        }
    }

    // 继续递归子节点（非容器路径）
    if !is_container {
        for child in &view.children {
            collect_recursive(child, parent_container, ctx);
        }
    }
}

/// 为每个 AccordionItem 注册 toggle handler，实现 mode 协调：
///
/// - `"multiple"`: 独立切换，不影响兄弟
/// - `"single"`: 折叠态→展开并折叠兄弟；展开态→不折叠（至少一个保持展开）
/// - `"single-collapsible"`: 展开态→折叠；折叠态→展开并折叠兄弟
///
/// disabled item 不注册交互 handler（disabled 完全冻结）。
fn register_mode_coordination(app: &mut impl InteractionHost, ctx: &Arc<AccordionContext>) {
    let store = app.widget_state_store().clone();
    let ctx_clone = Arc::clone(ctx);

    for (&container_id, item_ids) in &ctx.containers {
        let mode = ctx.mode_of(container_id).to_string();

        for &item_id in item_ids {
            let is_disabled = ctx.is_item_disabled(item_id);

            if is_disabled {
                // disabled item: 不注册交互 handler（完全冻结）
                continue;
            }

            // 初始化 store 中的 expanded 状态（如果尚未设置）
            if store.read::<bool>(item_id).is_none() {
                store.insert(item_id, false);
            }

            let store_h = store.clone();
            let ctx_h = Arc::clone(&ctx_clone);
            let mode_h = mode.clone();
            let container_id_h = container_id;

            // 注册 widget instance handler —— 处理 "toggle" action（用户点击/Enter/Space）
            app.register_widget_instance(
                item_id,
                Box::new(move |action: &str, _update_ctx: &mut UpdateContext| {
                    match action {
                        "toggle" => {
                            let expanded = store_h.read::<bool>(item_id).unwrap_or(false);
                            log::debug!(target: "rgui::core",
                                "[ACCORDION-DEBUG] toggle: WidgetId({item_id:?}) expanded={expanded}, mode=\"{mode_h}\""
                            );
                            handle_item_toggle(
                                &store_h,
                                item_id,
                                container_id_h,
                                expanded,
                                &mode_h,
                                &ctx_h,
                            );
                            let new_expanded = store_h.read::<bool>(item_id).unwrap_or(false);
                            log::debug!(target: "rgui::core",
                                "[ACCORDION-DEBUG] toggle: WidgetId({item_id:?}) → new_expanded={new_expanded}"
                            );
                            EventResult::Handled
                        }
                        // 键盘导航动作由容器 handler 处理
                        _ => {
                            log::debug!(target: "rgui::core",
                                "[ACCORDION-DEBUG] widget_instance: WidgetId({item_id:?}) action=\"{action}\" (not toggle, passing through)"
                            );
                            EventResult::Continue(String::new())
                        }
                    }
                }),
            );
        }
    }
}

/// 处理单个 item 的 toggle 操作（用户触发路径）。
///
/// 根据 mode 规则决定展开/折叠行为，派发 trigger 事件和 cancelable 事件。
fn handle_item_toggle(
    store: &rgui_core::widget_state::WidgetStateStore,
    item_id: WidgetId,
    container_id: WidgetId,
    expanded: bool,
    mode: &str,
    ctx: &AccordionContext,
) {
    // R5: disabled 检查
    if ctx.is_item_disabled(item_id) {
        return;
    }

    // R7: 派发 wa-accordion-item-trigger 事件（用户触发路径特有）
    dispatch_trigger_event(store, item_id);

    match mode {
        "single" => {
            if !expanded {
                // 折叠态 → 展开，同时折叠所有兄弟
                if !check_expand_veto(store, item_id) {
                    collapse_all_siblings(store, item_id, container_id, ctx);
                    expand_item(store, item_id, container_id);
                }
            }
            // 展开态 → 不折叠（至少一个保持展开）
        }
        "single-collapsible" => {
            if expanded {
                // 展开态 → 折叠
                if !check_collapse_veto(store, item_id) {
                    collapse_item(store, item_id, container_id);
                }
            } else {
                // 折叠态 → 展开，同时折叠所有兄弟
                if !check_expand_veto(store, item_id) {
                    collapse_all_siblings(store, item_id, container_id, ctx);
                    expand_item(store, item_id, container_id);
                }
            }
        }
        _ => {
            // "multiple"（默认）：独立切换
            if expanded {
                if !check_collapse_veto(store, item_id) {
                    collapse_item(store, item_id, container_id);
                }
            } else {
                if !check_expand_veto(store, item_id) {
                    expand_item(store, item_id, container_id);
                }
            }
        }
    }
}

/// 折叠容器中除指定 item 外的所有兄弟（用于 single/single-collapsible 模式）。
/// disabled 的兄弟 item 抵抗折叠。
fn collapse_all_siblings(
    store: &rgui_core::widget_state::WidgetStateStore,
    except_item_id: WidgetId,
    container_id: WidgetId,
    ctx: &AccordionContext,
) {
    if let Some(siblings) = ctx.containers.get(&container_id) {
        for &sibling_id in siblings {
            if sibling_id == except_item_id {
                continue;
            }
            // R3: disabled item 抵抗 mode 协调折叠
            if ctx.is_item_disabled(sibling_id) {
                continue;
            }
            if let Some(expanded) = store.read::<bool>(sibling_id) {
                if expanded && !check_collapse_veto(store, sibling_id) {
                    collapse_item(store, sibling_id, container_id);
                }
            }
        }
    }
}

/// 展开单个 item（无 trigger 事件，有 cancelable expand + after 事件）。
fn expand_item(
    store: &rgui_core::widget_state::WidgetStateStore,
    item_id: WidgetId,
    container_id: WidgetId,
) {
    store.insert(item_id, true);
    // R7: 派发 wa-accordion-item-expanded（item 自身）
    dispatch_item_expanded(store, item_id);
    // 派发 wa-after-expand（容器）
    dispatch_after_expand(store, container_id, item_id);
}

/// 折叠单个 item（无 trigger 事件，有 cancelable collapse + after 事件）。
fn collapse_item(
    store: &rgui_core::widget_state::WidgetStateStore,
    item_id: WidgetId,
    container_id: WidgetId,
) {
    store.insert(item_id, false);
    // R7: 派发 wa-accordion-item-collapsed（item 自身）
    dispatch_item_collapsed(store, item_id);
    // 派发 wa-after-collapse（容器）
    dispatch_after_collapse(store, container_id, item_id);
}

// ============================================================================
// SubTask 2.2: 键盘导航 roving tabindex (R6)
// ============================================================================

/// 为 Accordion 容器注册 keydown handler，处理 Arrow Down/Up/Home/End。
///
/// 仅非 disabled item 参与导航。回绕使用 `% items.length`。
/// Enter/Space 触发 toggle（由 item handler 处理）。
fn register_keyboard_navigation(app: &mut impl InteractionHost, ctx: &Arc<AccordionContext>) {
    let store = app.widget_state_store().clone();
    let ctx_clone = Arc::clone(ctx);

    for (&container_id, _item_ids) in &ctx.containers {
        let store_h = store.clone();
        let ctx_h = Arc::clone(&ctx_clone);
        let container_id_h = container_id;

        // 获取非 disabled item 的有序列表
        let focusable_items: Vec<WidgetId> = ctx
            .containers
            .get(&container_id)
            .map(|items| {
                items
                    .iter()
                    .copied()
                    .filter(|&id| !ctx.is_item_disabled(id))
                    .collect()
            })
            .unwrap_or_default();

        // 初始化 roving tabindex —— 第一个非 disabled item 获得 tabindex=0
        if !focusable_items.is_empty() {
            // 在 store 中存储 roving 状态：当前 tabbable 索引
            let roving_key = roving_state_key(container_id_h);
            if store_h.read::<usize>(roving_key).is_none() {
                store_h.insert(roving_key, 0usize);
            }
        }

        // 注册容器上的 keyboard handler
        app.register_widget_instance(
            container_id_h,
            Box::new(move |action: &str, _update_ctx: &mut UpdateContext| {
                let items: Vec<WidgetId> = ctx_h
                    .containers
                    .get(&container_id_h)
                    .map(|ids| {
                        ids.iter()
                            .copied()
                            .filter(|&id| !ctx_h.is_item_disabled(id))
                            .collect()
                    })
                    .unwrap_or_default();

                if items.is_empty() {
                    return EventResult::Handled;
                }

                let roving_key = roving_state_key(container_id_h);
                let current_idx = store_h.read::<usize>(roving_key).unwrap_or(0);
                let items_len = items.len();

                match action {
                    "ArrowDown" => {
                        let next_idx = (current_idx + 1) % items_len;
                        store_h.insert(roving_key, next_idx);
                        // 聚焦新 item
                        store_h.insert(focus_request_key(container_id_h), items[next_idx]);
                        EventResult::Handled
                    }
                    "ArrowUp" => {
                        let prev_idx = if current_idx == 0 {
                            items_len - 1
                        } else {
                            current_idx - 1
                        };
                        store_h.insert(roving_key, prev_idx);
                        store_h.insert(focus_request_key(container_id_h), items[prev_idx]);
                        EventResult::Handled
                    }
                    "Home" => {
                        store_h.insert(roving_key, 0usize);
                        store_h.insert(focus_request_key(container_id_h), items[0]);
                        EventResult::Handled
                    }
                    "End" => {
                        let last_idx = items_len - 1;
                        store_h.insert(roving_key, last_idx);
                        store_h.insert(focus_request_key(container_id_h), items[last_idx]);
                        EventResult::Handled
                    }
                    // Enter/Space: 由 item handler 处理，容器不拦截
                    "Enter" | "Space" => EventResult::Continue(String::new()),
                    _ => EventResult::Continue(String::new()),
                }
            }),
        );
    }
}

/// 生成 roving tabindex 状态的 store key
fn roving_state_key(container_id: WidgetId) -> WidgetId {
    // 使用一个固定的偏移量确保 key 不与其他 WidgetId 冲突
    WidgetId::from_u64(container_id.as_u64().wrapping_add(0xAC00_0000_0000))
}

/// 生成 focus request 的 store key
fn focus_request_key(container_id: WidgetId) -> WidgetId {
    WidgetId::from_u64(container_id.as_u64().wrapping_add(0xFC00_0000_0000))
}

// ============================================================================
// SubTask 2.3: 事件体系 (R7)
// ============================================================================

/// 事件体系的 WidgetStateStore key 约定。
const EVENT_PREFIX: u64 = 0xE000_0000_0000;

fn event_key(item_id: WidgetId, event_name: &str) -> WidgetId {
    // 使用 item_id + event_name hash 生成唯一 key
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    item_id.as_u64().hash(&mut hasher);
    event_name.hash(&mut hasher);
    WidgetId::from_u64(EVENT_PREFIX | (hasher.finish() & 0x0000_FFFF_FFFF))
}

/// 注册事件体系——初始化 veto flag 存储。
fn register_event_system(app: &mut impl InteractionHost, _ctx: &Arc<AccordionContext>) {
    let _store = app.widget_state_store().clone();
    // Veto flags 和事件 flag 在 toggle 时按需创建/检查，无需预注册
}

/// 派发 wa-accordion-item-trigger 事件（通过 store flag）。
/// 仅用户交互路径（点击/Enter/Space）调用此函数。
fn dispatch_trigger_event(store: &rgui_core::widget_state::WidgetStateStore, item_id: WidgetId) {
    let key = event_key(item_id, "wa-accordion-item-trigger");
    store.insert(key, true);
}

/// 派发 wa-after-expand 事件（容器级别）。
fn dispatch_after_expand(
    store: &rgui_core::widget_state::WidgetStateStore,
    container_id: WidgetId,
    item_id: WidgetId,
) {
    let key = event_key(container_id, "wa-after-expand");
    store.insert(key, item_id.as_u64()); // 存储触发 item 的 ID
}

/// 派发 wa-after-collapse 事件（容器级别）。
fn dispatch_after_collapse(
    store: &rgui_core::widget_state::WidgetStateStore,
    container_id: WidgetId,
    item_id: WidgetId,
) {
    let key = event_key(container_id, "wa-after-collapse");
    store.insert(key, item_id.as_u64());
}

/// 派发 wa-accordion-item-expanded 事件（item 级别）。
fn dispatch_item_expanded(store: &rgui_core::widget_state::WidgetStateStore, item_id: WidgetId) {
    let key = event_key(item_id, "wa-accordion-item-expanded");
    store.insert(key, true);
}

/// 派发 wa-accordion-item-collapsed 事件（item 级别）。
fn dispatch_item_collapsed(store: &rgui_core::widget_state::WidgetStateStore, item_id: WidgetId) {
    let key = event_key(item_id, "wa-accordion-item-collapsed");
    store.insert(key, true);
}

/// 检查 wa_expand_veto flag——若存在且为 true，阻止展开。
fn check_expand_veto(
    store: &rgui_core::widget_state::WidgetStateStore,
    item_id: WidgetId,
) -> bool {
    let key = event_key(item_id, "wa_expand_veto");
    store.read::<bool>(key).unwrap_or(false)
}

/// 检查 wa_collapse_veto flag——若存在且为 true，阻止折叠。
fn check_collapse_veto(
    store: &rgui_core::widget_state::WidgetStateStore,
    item_id: WidgetId,
) -> bool {
    let key = event_key(item_id, "wa_collapse_veto");
    store.read::<bool>(key).unwrap_or(false)
}

/// 清除 veto flag（在一次 toggle 操作完成后）。
fn clear_veto_flags(store: &rgui_core::widget_state::WidgetStateStore, item_id: WidgetId) {
    store.remove(event_key(item_id, "wa_expand_veto"));
    store.remove(event_key(item_id, "wa_collapse_veto"));
}

/// 检查 wa-accordion-item-trigger 事件是否已派发。
pub fn consume_trigger_event(
    store: &rgui_core::widget_state::WidgetStateStore,
    item_id: WidgetId,
) -> bool {
    let key = event_key(item_id, "wa-accordion-item-trigger");
    if store.read::<bool>(key).unwrap_or(false) {
        store.remove(key);
        true
    } else {
        false
    }
}

/// 检查 wa-after-expand 事件，返回触发 item 的 ID。
pub fn consume_after_expand(
    store: &rgui_core::widget_state::WidgetStateStore,
    container_id: WidgetId,
) -> Option<WidgetId> {
    let key = event_key(container_id, "wa-after-expand");
    let result = store.read::<u64>(key).map(WidgetId::from_u64);
    if result.is_some() {
        store.remove(key);
    }
    result
}

/// 检查 wa-after-collapse 事件，返回触发 item 的 ID。
pub fn consume_after_collapse(
    store: &rgui_core::widget_state::WidgetStateStore,
    container_id: WidgetId,
) -> Option<WidgetId> {
    let key = event_key(container_id, "wa-after-collapse");
    let result = store.read::<u64>(key).map(WidgetId::from_u64);
    if result.is_some() {
        store.remove(key);
    }
    result
}

// ============================================================================
// SubTask 2.4: @method API (R8)
// ============================================================================

/// 为 Container 和 Item 注册 @method handler。
///
/// Item 级别：expand() / collapse() / toggle() / focus()
/// Container 级别：expandAll() / collapseAll()
fn register_method_api(app: &mut impl InteractionHost, ctx: &Arc<AccordionContext>) {
    let store = app.widget_state_store().clone();
    let ctx_clone = Arc::clone(ctx);

    // Item 级别方法 handler
    for (&_container_id, item_ids) in &ctx.containers {
        for &item_id in item_ids {
            let is_disabled = ctx.is_item_disabled(item_id);
            let store_h = store.clone();
            let ctx_h = Arc::clone(&ctx_clone);

            // 找到 item 的父容器
            let container_id = ctx.item_to_parent.get(&item_id).copied();

            app.register_widget_instance(
                item_id,
                Box::new(move |action: &str, _update_ctx: &mut UpdateContext| {
                    // toggle 由本 handler 直接处理（不再转发给已被覆盖的 mode coordination handler）
                    if action == "toggle" {
                        if let Some(cid) = container_id {
                            let expanded = store_h.read::<bool>(item_id).unwrap_or(false);
                            let mode = ctx_h.mode_of(cid).to_string();
                            handle_item_toggle(&store_h, item_id, cid, expanded, &mode, &ctx_h);
                        }
                        return EventResult::Handled;
                    }

                    match action {
                        "expand" | "expand()" => {
                            // disabled item: no-op
                            if is_disabled || ctx_h.is_item_disabled(item_id) {
                                return EventResult::Handled;
                            }
                            let expanded = store_h.read::<bool>(item_id).unwrap_or(false);
                            if !expanded {
                                if !check_expand_veto(&store_h, item_id) {
                                    if let Some(cid) = container_id {
                                        let mode = ctx_h.mode_of(cid);
                                        if mode == "single" || mode == "single-collapsible" {
                                            collapse_all_siblings(&store_h, item_id, cid, &ctx_h);
                                        }
                                    }
                                    expand_item(&store_h, item_id,
                                        container_id.unwrap_or(WidgetId::from_u64(0)));
                                    dispatch_item_expanded(&store_h, item_id);
                                }
                            }
                            clear_veto_flags(&store_h, item_id);
                            EventResult::Handled
                        }
                        "collapse" | "collapse()" => {
                            if is_disabled || ctx_h.is_item_disabled(item_id) {
                                return EventResult::Handled;
                            }
                            let expanded = store_h.read::<bool>(item_id).unwrap_or(false);
                            if expanded {
                                if !check_collapse_veto(&store_h, item_id) {
                                    collapse_item(&store_h, item_id,
                                        container_id.unwrap_or(WidgetId::from_u64(0)));
                                    dispatch_item_collapsed(&store_h, item_id);
                                }
                            }
                            clear_veto_flags(&store_h, item_id);
                            EventResult::Handled
                        }
                        "focus" | "focus()" => {
                            // R8: focus() — 仍可聚焦 disabled item，但不可 toggle
                            if let Some(cid) = container_id {
                                // 更新 roving tabindex
                                if let Some(items) = ctx_h.containers.get(&cid) {
                                    let pos = items.iter().position(|&id| id == item_id);
                                    if let Some(idx) = pos {
                                        store_h.insert(roving_state_key(cid), idx);
                                    }
                                }
                                store_h.insert(focus_request_key(cid), item_id);
                            }
                            EventResult::Handled
                        }
                        _ => EventResult::Continue(String::new()),
                    }
                }),
            );
        }
    }

    // Container 级别方法 handler
    for (&container_id, _item_ids) in &ctx.containers {
        let store_h = store.clone();
        let ctx_h = Arc::clone(&ctx_clone);
        let container_id_h = container_id;
        let mode = ctx.mode_of(container_id).to_string();

        app.register_widget_instance(
            container_id_h,
            Box::new(move |action: &str, _update_ctx: &mut UpdateContext| {
                match action {
                    "expandAll" | "expandAll()" => {
                        // 仅 multiple 模式有效
                        if mode != "multiple" {
                            return EventResult::Handled;
                        }
                        if let Some(items) = ctx_h.containers.get(&container_id_h) {
                            for &item_id in items {
                                if ctx_h.is_item_disabled(item_id) {
                                    continue;
                                }
                                let expanded =
                                    store_h.read::<bool>(item_id).unwrap_or(false);
                                if !expanded && !check_expand_veto(&store_h, item_id) {
                                    expand_item(&store_h, item_id, container_id_h);
                                    dispatch_item_expanded(&store_h, item_id);
                                    clear_veto_flags(&store_h, item_id);
                                }
                            }
                        }
                        EventResult::Handled
                    }
                    "collapseAll" | "collapseAll()" => {
                        if let Some(items) = ctx_h.containers.get(&container_id_h) {
                            for &item_id in items {
                                if ctx_h.is_item_disabled(item_id) {
                                    continue;
                                }
                                let expanded =
                                    store_h.read::<bool>(item_id).unwrap_or(false);
                                if expanded && !check_collapse_veto(&store_h, item_id) {
                                    collapse_item(&store_h, item_id, container_id_h);
                                    dispatch_item_collapsed(&store_h, item_id);
                                    clear_veto_flags(&store_h, item_id);
                                }
                            }
                        }
                        EventResult::Handled
                    }
                    _ => EventResult::Continue(String::new()),
                }
            }),
        );
    }
}

// ============================================================================
// SubTask 2.5: disabled 拦截 + 父→子属性同步 (R5 + R4)
// ============================================================================

/// 同步父 Accordion 的 iconPlacement 属性到所有直接子 AccordionItem。
///
/// 遍历 view 树，对每个 Accordion 容器，将其 icon_placement prop 注入到子 AccordionItem。
pub fn sync_icon_placement<M: AppMessage>(view: &mut WidgetView<M>) {
    sync_prop_to_children(view, "icon_placement", "icon_placement", |v| v);
}

/// 同步父 Accordion 的 headingLevel 属性到所有直接子 AccordionItem。
pub fn sync_heading_level<M: AppMessage>(view: &mut WidgetView<M>) {
    sync_prop_to_children(view, "heading_level", "heading_level", |v| v);
}

/// 同步父 Accordion 的 appearance 属性到所有直接子 AccordionItem。
pub fn sync_appearance<M: AppMessage>(view: &mut WidgetView<M>) {
    sync_prop_to_children(view, "appearance", "appearance", |v| v);
}

/// 通用父→子属性同步：遍历树，将 Accordion 容器的 prop 注入到子 AccordionItem。
fn sync_prop_to_children<M: AppMessage>(
    view: &mut WidgetView<M>,
    parent_prop_name: &'static str,
    child_prop_name: &'static str,
    _transform: fn(&PropValue) -> &PropValue,
) {
    sync_prop_recursive(view, parent_prop_name, child_prop_name);
}

fn sync_prop_recursive<M: AppMessage>(
    view: &mut WidgetView<M>,
    parent_prop_name: &'static str,
    child_prop_name: &'static str,
) {
    let is_accordion = view
        .props
        .get("_rhai_path")
        .and_then(|v| match v {
            PropValue::Str(s) => Some(s.as_ref().contains("accordion.rhai")),
            _ => None,
        })
        .unwrap_or(false);

    if is_accordion {
        // 读取容器属性值
        let prop_value = view.props.get(parent_prop_name).cloned();

        if let Some(ref value) = prop_value {
            // 注入到所有直接子 AccordionItem
            for child in &mut view.children {
                let is_item = child
                    .props
                    .get("_rhai_path")
                    .and_then(|v| match v {
                        PropValue::Str(s) => Some(s.as_ref().contains("accordionitem")),
                        _ => None,
                    })
                    .unwrap_or(false);

                if is_item {
                    child.props.insert(child_prop_name, value.clone());
                }
            }
        }

        // Accordion 的子节点不再递归（子节点是 AccordionItem，不是 Accordion）
        return;
    }

    // 非容器节点：递归子节点
    for child in &mut view.children {
        sync_prop_recursive(child, parent_prop_name, child_prop_name);
    }
}

/// 同步所有父属性（iconPlacement + headingLevel + appearance）到子 AccordionItem。
///
/// 作为便捷入口，一次性完成全部父→子属性同步。
pub fn sync_all_parent_props<M: AppMessage>(view: &mut WidgetView<M>) {
    sync_icon_placement(view);
    sync_heading_level(view);
    sync_appearance(view);
}

/// 判断 item 是否 disabled（供外部使用）。
///
/// 从 WidgetView props 读取 disabled 属性。
pub fn is_item_disabled_from_view<M: AppMessage>(view: &WidgetView<M>) -> bool {
    view.props
        .get("disabled")
        .and_then(|v| match v {
            PropValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false)
}

// ============================================================================
// SubTask 2.1 补充: 公开 API
// ============================================================================

/// 获取 Accordion 容器的所有子 AccordionItem 的 WidgetId（按顺序）。
pub fn get_container_items(ctx: &AccordionContext, container_id: WidgetId) -> Vec<WidgetId> {
    ctx.containers.get(&container_id).cloned().unwrap_or_default()
}

/// 获取 item 所属的父容器 WidgetId。
pub fn get_parent_container(ctx: &AccordionContext, item_id: WidgetId) -> Option<WidgetId> {
    ctx.item_to_parent.get(&item_id).copied()
}

/// 判断 widget 是否为 Accordion 容器（通过 _rhai_path）。
pub fn is_accordion_container<M: AppMessage>(view: &WidgetView<M>) -> bool {
    view.props
        .get("_rhai_path")
        .and_then(|v| match v {
            PropValue::Str(s) => Some(s.as_ref().contains("accordion.rhai")),
            _ => None,
        })
        .unwrap_or(false)
}

/// 判断 widget 是否为 AccordionItem（通过 _rhai_path）。
pub fn is_accordion_item<M: AppMessage>(view: &WidgetView<M>) -> bool {
    view.props
        .get("_rhai_path")
        .and_then(|v| match v {
            PropValue::Str(s) => Some(s.as_ref().contains("accordionitem")),
            _ => None,
        })
        .unwrap_or(false)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rgui_core::view::PropValue;

    /// 构造一个简单的 AccordionItem view（用于测试）
    fn make_item_view<M: AppMessage>(
        id: u64,
        expanded: bool,
        disabled: bool,
    ) -> WidgetView<M> {
        let mut view = WidgetView::new("WaAccordionItem");
        view.id = Some(WidgetId::from_u64(id));
        view.props.insert(
            "_rhai_path",
            PropValue::Str("accordionitem.rhai".into()),
        );
        view.props.insert("expanded", PropValue::Bool(expanded));
        view.props.insert("disabled", PropValue::Bool(disabled));
        view
    }

    /// 构造一个简单的 Accordion 容器 view（用于测试）
    fn make_accordion_view<M: AppMessage>(
        id: u64,
        mode: &str,
        children: Vec<WidgetView<M>>,
    ) -> WidgetView<M> {
        let mut view = WidgetView::new("WaAccordion");
        view.id = Some(WidgetId::from_u64(id));
        view.props.insert(
            "_rhai_path",
            PropValue::Str("accordion.rhai".into()),
        );
        view.props.insert("mode", PropValue::Str(mode.into()));
        view.children = children;
        view
    }

    #[test]
    fn test_collect_single_container_with_items() {
        let item1 = make_item_view::<rgui_core::message::NoopMsg>(10, false, false);
        let item2 = make_item_view::<rgui_core::message::NoopMsg>(20, true, false);
        let view = make_accordion_view::<rgui_core::message::NoopMsg>(
            1,
            "single",
            vec![item1, item2],
        );

        let ctx = collect_accordion_nodes(&view);

        let container_id = WidgetId::from_u64(1);
        assert_eq!(ctx.containers.len(), 1);
        assert!(ctx.containers.contains_key(&container_id));

        let items = &ctx.containers[&container_id];
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], WidgetId::from_u64(10));
        assert_eq!(items[1], WidgetId::from_u64(20));

        assert_eq!(ctx.mode_of(container_id), "single");
    }

    #[test]
    fn test_collect_multiple_mode_default() {
        let item = make_item_view::<rgui_core::message::NoopMsg>(10, false, false);
        // 不设置 mode prop → 默认 "multiple"
        let mut view = make_accordion_view::<rgui_core::message::NoopMsg>(
            1,
            "multiple",
            vec![item],
        );
        view.props.remove("mode");

        let ctx = collect_accordion_nodes(&view);
        assert_eq!(ctx.mode_of(WidgetId::from_u64(1)), "multiple");
    }

    #[test]
    fn test_disabled_item_detection() {
        let enabled = make_item_view::<rgui_core::message::NoopMsg>(10, false, false);
        let disabled = make_item_view::<rgui_core::message::NoopMsg>(20, false, true);
        let view = make_accordion_view::<rgui_core::message::NoopMsg>(
            1,
            "multiple",
            vec![enabled, disabled],
        );

        let ctx = collect_accordion_nodes(&view);
        assert!(!ctx.is_item_disabled(WidgetId::from_u64(10)));
        assert!(ctx.is_item_disabled(WidgetId::from_u64(20)));
    }

    #[test]
    fn test_item_to_parent_mapping() {
        let item = make_item_view::<rgui_core::message::NoopMsg>(10, false, false);
        let view = make_accordion_view::<rgui_core::message::NoopMsg>(
            1,
            "multiple",
            vec![item],
        );

        let ctx = collect_accordion_nodes(&view);
        assert_eq!(
            ctx.item_to_parent.get(&WidgetId::from_u64(10)),
            Some(&WidgetId::from_u64(1))
        );
    }

    #[test]
    fn test_nested_accordion_not_supported() {
        // 嵌套 Accordion 不在 scope 内，但应优雅处理（内层忽略，不 panic）
        let inner_item =
            make_item_view::<rgui_core::message::NoopMsg>(30, false, false);
        let inner = make_accordion_view::<rgui_core::message::NoopMsg>(
            2,
            "single",
            vec![inner_item],
        );
        let outer_item =
            make_item_view::<rgui_core::message::NoopMsg>(10, false, false);
        let mut outer_view = make_accordion_view::<rgui_core::message::NoopMsg>(
            1,
            "multiple",
            vec![outer_item],
        );
        // 将内层 Accordion 作为外在 item 的子节点（模拟嵌套）
        // 这在当前实现中不会导致 panic，但内层 item 不会被收集
        outer_view.children.push(inner);

        let ctx = collect_accordion_nodes(&outer_view);
        // 外层容器应有 1 个直接 item（outer_item）
        let container1 = WidgetId::from_u64(1);
        assert_eq!(ctx.containers[&container1].len(), 1);
    }

    #[test]
    fn test_sync_icon_placement() {
        let item = make_item_view::<rgui_core::message::NoopMsg>(10, false, false);
        let mut view = make_accordion_view::<rgui_core::message::NoopMsg>(
            1,
            "multiple",
            vec![item],
        );
        view.props
            .insert("icon_placement", PropValue::Str("start".into()));

        sync_icon_placement(&mut view);

        // 子 item 应获得 icon_placement prop
        let child = &view.children[0];
        let placement = child.props.get("icon_placement").and_then(|v| match v {
            PropValue::Str(s) => Some(s.as_ref()),
            _ => None,
        });
        assert_eq!(placement, Some("start"));
    }

    #[test]
    fn test_sync_appearance() {
        let item = make_item_view::<rgui_core::message::NoopMsg>(10, false, false);
        let mut view = make_accordion_view::<rgui_core::message::NoopMsg>(
            1,
            "multiple",
            vec![item],
        );
        view.props
            .insert("appearance", PropValue::Str("filled".into()));

        sync_appearance(&mut view);

        let child = &view.children[0];
        let appearance = child.props.get("appearance").and_then(|v| match v {
            PropValue::Str(s) => Some(s.as_ref()),
            _ => None,
        });
        assert_eq!(appearance, Some("filled"));
    }

    #[test]
    fn test_is_accordion_container() {
        let view = make_accordion_view::<rgui_core::message::NoopMsg>(1, "multiple", vec![]);
        assert!(is_accordion_container(&view));
    }

    #[test]
    fn test_is_accordion_item() {
        let view = make_item_view::<rgui_core::message::NoopMsg>(10, false, false);
        assert!(is_accordion_item(&view));
    }

    #[test]
    fn test_is_item_disabled_from_view() {
        let view = make_item_view::<rgui_core::message::NoopMsg>(10, false, true);
        assert!(is_item_disabled_from_view(&view));

        let view2 = make_item_view::<rgui_core::message::NoopMsg>(20, false, false);
        assert!(!is_item_disabled_from_view(&view2));
    }

    #[test]
    fn test_accordion_context_empty() {
        let ctx = AccordionContext::default();
        assert!(ctx.containers.is_empty());
        assert!(ctx.item_to_parent.is_empty());
        assert_eq!(ctx.mode_of(WidgetId::from_u64(1)), "multiple");
        assert!(!ctx.is_item_disabled(WidgetId::from_u64(1)));
    }

    #[test]
    fn test_get_container_items() {
        let item1 = make_item_view::<rgui_core::message::NoopMsg>(10, false, false);
        let item2 = make_item_view::<rgui_core::message::NoopMsg>(20, false, false);
        let view = make_accordion_view::<rgui_core::message::NoopMsg>(
            1,
            "multiple",
            vec![item1, item2],
        );
        let ctx = collect_accordion_nodes(&view);

        let items = get_container_items(&ctx, WidgetId::from_u64(1));
        assert_eq!(items.len(), 2);

        let empty = get_container_items(&ctx, WidgetId::from_u64(99));
        assert!(empty.is_empty());
    }

    #[test]
    fn test_get_parent_container() {
        let item = make_item_view::<rgui_core::message::NoopMsg>(10, false, false);
        let view = make_accordion_view::<rgui_core::message::NoopMsg>(
            1,
            "multiple",
            vec![item],
        );
        let ctx = collect_accordion_nodes(&view);

        assert_eq!(
            get_parent_container(&ctx, WidgetId::from_u64(10)),
            Some(WidgetId::from_u64(1))
        );
        assert_eq!(get_parent_container(&ctx, WidgetId::from_u64(99)), None);
    }

    #[test]
    fn test_sync_all_parent_props() {
        let item = make_item_view::<rgui_core::message::NoopMsg>(10, false, false);
        let mut view = make_accordion_view::<rgui_core::message::NoopMsg>(
            1,
            "multiple",
            vec![item],
        );
        view.props
            .insert("icon_placement", PropValue::Str("start".into()));
        view.props
            .insert("heading_level", PropValue::Str("2".into()));
        view.props
            .insert("appearance", PropValue::Str("filled".into()));

        sync_all_parent_props(&mut view);

        let child = &view.children[0];
        assert!(child.props.contains_key("icon_placement"));
        assert!(child.props.contains_key("heading_level"));
        assert!(child.props.contains_key("appearance"));
    }

    #[test]
    fn test_mode_of_multiple_containers() {
        let item1 = make_item_view::<rgui_core::message::NoopMsg>(10, false, false);
        let item2 = make_item_view::<rgui_core::message::NoopMsg>(20, false, false);
        let container1 = make_accordion_view::<rgui_core::message::NoopMsg>(
            1,
            "single",
            vec![item1],
        );
        let container2 = make_accordion_view::<rgui_core::message::NoopMsg>(
            2,
            "single-collapsible",
            vec![item2],
        );

        // 构造一个包含两个容器的根 view
        let mut root = WidgetView::<rgui_core::message::NoopMsg>::new("Root");
        root.children = vec![container1, container2];

        let ctx = collect_accordion_nodes(&root);
        assert_eq!(ctx.containers.len(), 2);
        assert_eq!(ctx.mode_of(WidgetId::from_u64(1)), "single");
        assert_eq!(
            ctx.mode_of(WidgetId::from_u64(2)),
            "single-collapsible"
        );
    }

    #[test]
    fn test_empty_accordion_collection() {
        let view = make_accordion_view::<rgui_core::message::NoopMsg>(1, "multiple", vec![]);
        let ctx = collect_accordion_nodes(&view);
        assert!(ctx.containers.contains_key(&WidgetId::from_u64(1)));
        assert!(ctx.containers[&WidgetId::from_u64(1)].is_empty());
    }
}
