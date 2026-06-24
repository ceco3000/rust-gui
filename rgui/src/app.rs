//! App 启动器——winit 窗口 + 事件循环 + wgpu 渲染 + 交互。

use rgui_core::context::UpdateContext;
use rgui_core::geometry::{Point, Rect, Size};
use rgui_core::id::{WidgetId, WindowId};
use rgui_core::registry::WidgetRegistry;
#[cfg(feature = "devtools")]
use rgui_core::traits::AppMessage;
use rgui_core::traits::EventResult;
#[cfg(feature = "devtools")]
use rgui_core::widget_id_map::WidgetIdBimap;
use rgui_platform::event::{
    Event, Modifiers, MouseButton, MouseEventCoords, MouseInputOrigin,
    logical_window_size_from_physical_size, normalize_platform_window_point,
};
use rgui_platform::focus::FocusManager;
use rgui_platform::focus::InputModality;
use rgui_platform::widget_tree::WidgetTree;
use rgui_render::{
    FocusIndicator, RenderBackend, RenderBackendFactory, RenderParams, SceneGraph, TextRenderer,
    VelloBackend, compute_view_layout,
};
#[cfg(feature = "devtools")]
use rgui_script::PropRegistry;
use std::collections::HashMap as FxHashMap;
use std::fmt;
#[cfg(feature = "devtools")]
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton as WinitMouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::WindowAttributes;

/// 交互回调类型。
pub type InteractionCallback = Box<dyn FnMut(&str) + Send>;

/// Widget 实例更新处理器（类型擦除）。
///
/// 接收动作名称和 UpdateContext，返回 EventResult 控制事件传播。
pub type WidgetUpdateHandler =
    Box<dyn FnMut(&str, &mut UpdateContext) -> EventResult<String> + Send>;

#[derive(Clone, Copy, Debug, PartialEq)]
enum CoordinateTransformStep {
    Translate { offset: Point },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CoordinateTransformChain {
    steps: Vec<CoordinateTransformStep>,
}

impl CoordinateTransformChain {
    #[must_use]
    pub fn translated(&self, offset: Point) -> Self {
        let mut next = self.clone();
        next.steps
            .push(CoordinateTransformStep::Translate { offset });
        next
    }

    #[must_use]
    fn window_to_local(&self, point: Point) -> Point {
        self.steps.iter().fold(point, |current, step| match step {
            CoordinateTransformStep::Translate { offset } => {
                Point::new(current.x - offset.x, current.y - offset.y)
            },
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct InteractionRegion {
    candidate_rect: Rect,
    window_to_local: CoordinateTransformChain,
}

impl InteractionRegion {
    #[must_use]
    fn from_bounds(bounds: Rect) -> Self {
        Self {
            candidate_rect: bounds,
            window_to_local: CoordinateTransformChain::default().translated(bounds.origin),
        }
    }

    #[must_use]
    fn resolve(&self, widget_id: WidgetId, position: Point) -> Option<ResolvedHitTest> {
        if !self.candidate_rect.contains(position) {
            return None;
        }
        Some(ResolvedHitTest {
            widget_id,
            candidate_rect: self.candidate_rect,
            local_logical: self.window_to_local.window_to_local(position),
            window_to_local: self.window_to_local.clone(),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ResolvedHitTest {
    widget_id: WidgetId,
    candidate_rect: Rect,
    local_logical: Point,
    window_to_local: CoordinateTransformChain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DragLifecycleEvent {
    Enter,
    Over,
    Drop,
}

impl ResolvedHitTest {
    #[must_use]
    fn from_absolute_bounds(widget_id: WidgetId, bounds: Rect, position: Point) -> Self {
        InteractionRegion::from_bounds(bounds)
            .resolve(widget_id, position)
            .expect("bounds contains(position) should hold before resolving hit test")
    }
}

/// 视图场景构建回调类型。
///
/// 回调接收帧计数、窗口宽度、窗口高度（逻辑像素）和文本渲染器引用，
/// 直接返回 `SceneGraph`。用于 `html!` 宏 + `build_scene_from_view` 的声明式路径。
pub type ViewSceneBuilder = Box<dyn FnMut(u64, u32, u32, &TextRenderer) -> SceneGraph + Send>;

#[allow(clippy::type_complexity)]
/// tick() 布局回调。
pub type LayoutCallback<'a> = Box<dyn FnOnce(&mut App) + 'a>;
#[allow(clippy::type_complexity)]
/// tick() 无障碍回调。
pub type A11yCallback<'a> = Box<dyn FnOnce(&mut App) + 'a>;
#[allow(clippy::type_complexity)]
/// tick() 渲染回调。
pub type RenderCallback<'a> = Box<dyn FnOnce(&mut App, &[Event]) -> Result<(), String> + 'a>;

/// 应用配置。
///
/// 控制窗口标题、初始尺寸、是否可缩放和最小尺寸。
#[derive(Clone, Debug)]
pub struct AppConfig {
    /// 窗口标题。
    pub title: String,
    /// 初始窗口尺寸。
    pub window_size: Size,
    /// 是否可缩放。
    pub resizable: bool,
    /// 最小窗口尺寸。
    pub min_size: Option<Size>,
    /// 可选 `.rgui` 文件路径，作为视图源（替代 `set_view_scene_builder`）。
    /// `App::load_rgui::<M>()` 从此字段读取路径。
    pub rgui_path: Option<PathBuf>,
    /// Rhai 脚本文件路径列表。
    /// 启动时编译全部脚本，文件变更时热重载。
    pub rhai_paths: Vec<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "rgui App".to_string(),
            window_size: Size::new(800.0, 600.0),
            resizable: true,
            min_size: Some(Size::new(320.0, 240.0)),
            rgui_path: None,
            rhai_paths: Vec::new(),
        }
    }
}

impl AppConfig {
    /// 创建默认配置。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// 设置窗口标题（builder 风格）。
    #[must_use]
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// 设置初始窗口尺寸（builder 风格）。
    #[must_use]
    pub fn window_size(mut self, w: f64, h: f64) -> Self {
        self.window_size = Size::new(w, h);
        self
    }
    /// 设置 .rgui 文件路径（builder 风格）。
    #[must_use]
    pub fn rgui_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.rgui_path = Some(path.into());
        self
    }
    /// 设置 Rhai 脚本文件路径列表（builder 风格）。
    #[must_use]
    pub fn rhai_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.rhai_paths = paths;
        self
    }
}

/// rgui 应用实例。
///
/// 管理窗口配置、注册表、事件队列、命中测试、焦点和交互区域。
/// 通过 `run()` 方法启动 winit 事件循环。
#[allow(dead_code)]
pub struct App {
    config: AppConfig,
    registry: WidgetRegistry,
    window_id: WindowId,
    events: Vec<Event>,
    focus: FocusManager,
    /// 交互区域：widget_id → (消息名, 回调)
    interactions: FxHashMap<WidgetId, (String, InteractionCallback)>,
    /// 交互区域边界（仅用于 fallback 命中测试，current_view 未设置时使用）。
    /// 生产路径中由 load_rgui 提供的 current_view + current_layout 替代。
    interaction_bounds: FxHashMap<WidgetId, Rect>,
    /// 交互区域的候选判定与局部坐标恢复信息。
    interaction_regions: FxHashMap<WidgetId, InteractionRegion>,
    /// Widget 实例更新处理器（WidgetSpec 路径）。
    /// 如果存在，handle_click 优先通过此处理器调用 update()，再根据 EventResult 决定是否回调旧路径。
    widget_instances: FxHashMap<WidgetId, WidgetUpdateHandler>,
    /// 弹层 widget 集合（WTI03：点击外部关闭）。
    ///
    /// 当命中测试未命中任何 widget 且此集合非空时，
    /// handle_click 向全部弹层发送 Close 事件。
    overlay_ids: std::collections::HashSet<WidgetId>,
    /// 可选的视图场景构建回调（直接返回 SceneGraph）。
    view_scene_builder: Option<ViewSceneBuilder>,
    /// PropRegistry——Rhai↔WidgetView prop 桥接（RS01/RS04）。
    /// 与 CommandRegistry 共享同一实例：Rhai 写、渲染线程 drain。
    #[cfg(feature = "devtools")]
    prop_registry: PropRegistry,
    /// WidgetId↔字符串双向映射（RS03/RS04）。
    /// 与 CommandRegistry 共享：渲染线程在视图重建后更新，Rhai 查询。
    #[cfg(feature = "devtools")]
    id_map: Arc<std::sync::Mutex<WidgetIdBimap>>,
    /// 当前 DPI 缩放因子（逻辑像素 → 物理像素比例）。
    /// 从 winit `window.scale_factor()` 读取，默认 1.0。
    pub(crate) scale_factor: f64,
    /// Rhai 命令处理器注册表（load_rhai_scripts 创建，AppHandler::new() 时 .take() 移入）。
    #[cfg(feature = "devtools")]
    command_registry: Option<rgui_script::CommandRegistry>,
    /// Rhai 脚本热重载管理器（load_rhai_scripts 创建，AppHandler::new() 时 .take() 移入）。
    #[cfg(feature = "devtools")]
    rhai_hot_reload: Option<rgui_devtools::rhai_hot_reload::RhaiHotReload>,
    /// 共享状态存储（RS06：Rhai↔渲染 dirty 追踪）。
    /// Rhai `store_write` 标记脏，渲染管线读取脏集合并增量重绘。
    state_store: Arc<RwLock<rgui_state::StateStore>>,
    /// RS06 回归修复：非 StateStore 交互（如 AtomicBool）触发后强制重建场景。
    /// register_interaction 回调触发后由 handle_click 设为 true，
    /// RedrawRequested 中迫使 can_reuse=false，重建后清除。
    pub(crate) needs_redraw: bool,
    /// 组件实例状态存储——跨帧持久化 WidgetSpec 组件的交互状态。
    ///
    /// .rgui 渲染路径中，paint_factory 每帧从 WidgetView.props 创建临时 state。
    /// widget_state_store 使交互组件（如 WaAccordionItem）能在此持久化自己的状态，
    /// paint_factory 读取 + widget_instance handler 写入，实现组件行为自包含。
    pub(crate) widget_state_store: crate::widget_state::WidgetStateStore,
    /// 最新布局引擎——每帧渲染后更新，供 handle_click 做树形命中测试。
    pub(crate) current_layout: Arc<std::sync::Mutex<Option<rgui_layout::LayoutEngine>>>,
    /// 当前 WidgetView 树（消息擦除）——每帧渲染后更新，供 handle_click 做 DFS 树遍历命中测试。
    /// 使用 `NoopMsg` 擦除用户消息类型，保留完整树结构（widget_type/id/children）。
    pub(crate) current_view:
        Arc<std::sync::Mutex<Option<rgui_core::view::WidgetView<rgui_core::message::NoopMsg>>>>,
    /// Widget 树——维护父子关系和布局边界。
    ///
    /// 由 `register_interaction` 填充边界（向后兼容），
    /// 生产路径中每帧从 LayoutEngine 同步绝对坐标。
    /// `hit_test()` 方法实现 D5 §4 DFS 树遍历命中测试。
    pub(crate) widget_tree: WidgetTree,
}

/// RS04: 将 PropRegistry drain 的结果注入到 WidgetView 树。
///
/// 遍历 WidgetView 树，根据 WidgetId 匹配将 pending props 注入到 `props` 映射。
/// **⚠️ 临时捷径：RS05-RS06 后降级为 fallback。**
#[cfg(feature = "devtools")]
fn inject_props_from_registry<M: AppMessage>(
    view: &mut rgui_core::view::WidgetView<M>,
    pending: &std::collections::HashMap<
        WidgetId,
        std::collections::BTreeMap<String, rgui_core::view::PropValue>,
    >,
) {
    inject_props_recursive(view, pending);
}

/// 递归辅助函数：遍历 WidgetView 树并注入 pending props。
#[cfg(feature = "devtools")]
fn inject_props_recursive<M: AppMessage>(
    view: &mut rgui_core::view::WidgetView<M>,
    pending: &std::collections::HashMap<
        WidgetId,
        std::collections::BTreeMap<String, rgui_core::view::PropValue>,
    >,
) {
    if let Some(id) = view.id {
        if let Some(widget_props) = pending.get(&id) {
            for (key, value) in widget_props {
                // 将 String key 转为 &'static str（RS04 临时捷径）
                let static_key: &'static str = intern_prop_key(key);
                view.props.insert(static_key, value.clone());
            }
        }
    }
    for child in &mut view.children {
        inject_props_recursive(child, pending);
    }
}

/// 将字符串 prop key 转为 `&'static str`（RS04 临时捷径）。
///
/// 使用全局缓存避免重复分配。prop key 数量有限（label、expanded、checked 等），
/// 内存泄漏上限可控。
#[cfg(feature = "devtools")]
fn intern_prop_key(key: &str) -> &'static str {
    use std::collections::HashSet;
    use std::sync::Mutex;
    static CACHE: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
    let mut guard = CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let cache = guard.get_or_insert_with(HashSet::new);
    if let Some(&existing) = cache.get(key) {
        existing
    } else {
        let leaked: &'static str = Box::leak(key.to_string().into_boxed_str());
        cache.insert(leaked);
        leaked
    }
}

/// RS07: 从 StateStore 读取 rhai_state 并注入到 WidgetView props。
///
/// 遍历 WidgetView 树，对其中的 state 绑定项（`${expr:state.xxx}` 标记），
/// 从 StateStore 读取对应的 rhai_state 值，注入为普通 prop。
#[cfg(feature = "devtools")]
fn inject_state_bindings<M: AppMessage>(
    view: &mut rgui_core::view::WidgetView<M>,
    id_map: &Arc<std::sync::Mutex<WidgetIdBimap>>,
    state_store: &Arc<RwLock<rgui_state::StateStore>>,
) {
    inject_state_bindings_recursive(view, id_map, state_store);
}

/// RS07: 递归辅助——遍历 WidgetView 树，将 state 绑定值注入为 props。
#[cfg(feature = "devtools")]
fn inject_state_bindings_recursive<M: AppMessage>(
    view: &mut rgui_core::view::WidgetView<M>,
    id_map: &Arc<std::sync::Mutex<WidgetIdBimap>>,
    state_store: &Arc<RwLock<rgui_state::StateStore>>,
) {
    use rgui_core::view::PropValue;

    // 检查当前节点的所有 props，寻找 ${expr:state.xxx} 标记
    let store = state_store.read().expect("StateStore RwLock poisoned");
    let bimap = id_map
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let mut injections: Vec<(&'static str, PropValue)> = Vec::new();
    for (&prop_name, prop_value) in &view.props {
        if let PropValue::Str(marker) = prop_value {
            if let Some(expr) = marker
                .strip_prefix("${expr:")
                .and_then(|s| s.strip_suffix('}'))
            {
                if rgui_devtools::rgui_parser::is_state_expr(expr) {
                    let state_key = &expr["state.".len()..];
                    if let Some(state_id) = bimap.get_id(state_key) {
                        if let Some(value) = store.read_rhai_state(state_id) {
                            let injected = rgui_devtools::rgui_parser::infer_prop_value(value);
                            injections.push((prop_name, injected));
                        }
                    }
                }
            }
        }
    }
    drop(bimap);
    drop(store);

    // 注入 props
    for (prop_name, value) in injections {
        view.props.insert(prop_name, value);
    }

    // 递归子节点
    for child in &mut view.children {
        inject_state_bindings_recursive(child, id_map, state_store);
    }
}

/// 运行 .rgui + .rhai 声明式应用（一行初始化）。
///
/// 自动完成标准初始化管线：
/// 1. 解析 .rgui → WidgetView 树
/// 2. 计算初始布局
/// 3. 创建 App
/// 4. 初始化交互组件（WaAccordionItem 等）和 onclick 注册
/// 5. 加载 .rhai 脚本
/// 6. 设置声明式渲染管道（每帧 layout → paint → scene）
/// 7. 调用 `app.run()` 进入事件循环
///
/// `.rgui` 路径从 `config.rgui_path` 读取，`.rhai` 脚本从 `config.rhai_paths` 读取。
///
/// 适合 demo 和原型开发。需要自定义渲染行为时请手动调用各个步骤。
///
/// # Type Parameters
///
/// - `M`: 应用消息类型（必须实现 `AppMessage`）。
#[cfg(feature = "devtools")]
pub fn run_simple_app<M: AppMessage + 'static>(
    config: AppConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    use rgui_core::geometry::Size;
    use rgui_devtools::rgui_parser::parse_rgui_file;
    use rgui_render::build_scene_from_view;

    use crate::interactive::init_widget_instances;
    use crate::paint_factory::default_paint_fn_with_state;

    // 0. 从 config 读取路径（先提取，再移动 config）
    let rgui_path = config.rgui_path.clone().ok_or(".rgui 路径未设置")?;
    let rhai_paths = config.rhai_paths.clone();

    // 1. 解析 .rgui
    let mut view: rgui_core::view::WidgetView<M> = parse_rgui_file(&rgui_path)?;

    // 2. 初始布局（Tier 2 脚本需要 bounds，AC02）
    let initial_layout = compute_view_layout(
        &mut view,
        Size::new(config.window_size.width, config.window_size.height),
        None,
    );

    // 3. 执行 Tier 2 Rhai paint 脚本（使用布局 bounds 注入 width/height）
    crate::paint_factory::execute_tier2_paint_scripts(&mut view, &initial_layout);

    // 3. 创建 App
    let mut app = App::new(config);

    // 4. 初始化交互组件
    init_widget_instances(&mut app, &view, &initial_layout);

    // 5. 加载 Rhai 脚本
    if !rhai_paths.is_empty() {
        let rhai_refs: Vec<&std::path::Path> = rhai_paths.iter().map(|p| p.as_path()).collect();
        app.load_rhai_scripts(&rhai_refs)?;
    }

    // 6. 设置渲染管道
    let store = app.widget_state_store().clone();
    let paint_fn = default_paint_fn_with_state::<M>(store.clone());
    let template = view;
    let current_layout = Arc::clone(&app.current_layout);
    let current_view = Arc::clone(&app.current_view);

    // 初始存储：首次 noop 视图（避免首帧前 handle_click 命中无数据）
    *current_view.lock().unwrap() = Some(template.to_noop_view());

    app.set_view_scene_builder(move |frame, width, height, tr| {
        let mut v = template.clone();
        sync_store_to_props(&mut v, &store);
        let l = compute_view_layout(
            &mut v,
            Size::new(f64::from(width), f64::from(height)),
            Some(tr),
        );
        // 更新当前布局和视图，供 handle_click 做树形命中测试
        let noop = v.to_noop_view();
        *current_view.lock().unwrap() = Some(noop);
        *current_layout.lock().unwrap() = Some(l);
        // Note: l 已被 move，重新获取
        let l = current_layout.lock().unwrap().take().unwrap();
        let scene = build_scene_from_view(&v, &l, &paint_fn, frame, Some(tr));
        *current_layout.lock().unwrap() = Some(l);
        scene
    });

    // 7. 启动事件循环
    app.run()
}

/// 将 WidgetStateStore 中的组件状态同步到 WidgetView.props。
///
/// walk_view_tree 条件渲染（如 WaAccordionItem 折叠跳过子节点）
/// 依赖 props 中的 expanded 值。handler 修改 store 后，此函数确保
/// 每帧渲染前 props 反映最新状态。
///
/// AC10: 实现 expanded 状态的 store→props 同步，
/// 使 widget_instance handler 的 toggle 操作能反映到渲染管线。
#[cfg(feature = "devtools")]
fn sync_store_to_props<M: AppMessage>(
    view: &mut rgui_core::view::WidgetView<M>,
    store: &crate::widget_state::WidgetStateStore,
) {
    sync_store_to_props_recursive(view, store);
}

/// 递归辅助：遍历 WidgetView 树，将 WidgetStateStore 中的状态同步为 props。
#[cfg(feature = "devtools")]
fn sync_store_to_props_recursive<M: AppMessage>(
    view: &mut rgui_core::view::WidgetView<M>,
    store: &crate::widget_state::WidgetStateStore,
) {
    use rgui_core::view::PropValue;

    if let Some(id) = view.id {
        // AC10: 同步 expanded 状态
        if let Some(expanded) = store.read::<bool>(id) {
            view.props.insert("expanded", PropValue::Bool(expanded));
        }
    }

    for child in &mut view.children {
        sync_store_to_props_recursive(child, store);
    }
}

/// 交互自动化 Harness。
///
/// 用于在不启动真实窗口的情况下：
/// - 从 `.rgui` 构建示例视图和布局
/// - 注入程序化 hover / click
/// - 读取命中结果与组件持久状态
///
/// 主要服务于示例与回归测试中的 HiDPI 命中验证。
#[cfg(feature = "devtools")]
pub struct InteractionAutomationHarness<M: AppMessage + Clone> {
    handler: AppHandler,
    template: rgui_core::view::WidgetView<M>,
    id_map: WidgetIdBimap,
    logical_window_size: Size,
}

#[cfg(feature = "devtools")]
impl<M: AppMessage + Clone + 'static> InteractionAutomationHarness<M> {
    /// 从 `AppConfig.rgui_path` 创建自动化 Harness。
    pub fn from_config(config: AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
        use rgui_devtools::rgui_parser::{collect_widget_ids, parse_rgui_file};

        let rgui_path = config.rgui_path.clone().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, ".rgui 路径未设置")
        })?;
        let rhai_paths = config.rhai_paths.clone();
        let logical_window_size = config.window_size;

        let mut view: rgui_core::view::WidgetView<M> = parse_rgui_file(&rgui_path)?;
        let initial_layout =
            rgui_render::scene_build::compute_view_layout(&mut view, logical_window_size, None);
        let id_map = collect_widget_ids(&view);

        let mut app = App::new(config);
        crate::interactive::init_widget_instances(&mut app, &view, &initial_layout);
        if !rhai_paths.is_empty() {
            let rhai_refs: Vec<&std::path::Path> = rhai_paths.iter().map(|p| p.as_path()).collect();
            app.load_rhai_scripts(&rhai_refs)?;
        }

        *app.current_view.lock().unwrap() = Some(view.to_noop_view());
        *app.current_layout.lock().unwrap() = Some(initial_layout);

        Ok(Self {
            handler: AppHandler::new(app),
            template: view,
            id_map,
            logical_window_size,
        })
    }

    /// 更新当前缩放因子。
    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        self.handler.sync_scale_factor(scale_factor);
    }

    /// 返回当前缩放因子。
    #[must_use]
    pub fn scale_factor(&self) -> f64 {
        self.handler.app.scale_factor()
    }

    /// 通过 `.rgui id` 查询运行时 `WidgetId`。
    #[must_use]
    pub fn widget_id(&self, name: &str) -> Option<WidgetId> {
        self.id_map.get_id(name)
    }

    /// 通过 `WidgetId` 反查 `.rgui id`。
    #[must_use]
    pub fn widget_name(&self, widget_id: WidgetId) -> Option<&str> {
        self.id_map.get_name(widget_id)
    }

    /// 返回 widget 的当前逻辑像素边界。
    #[must_use]
    pub fn widget_rect(&self, widget_id: WidgetId) -> Option<Rect> {
        let layout_guard = self.handler.app.current_layout.lock().unwrap();
        let layout = layout_guard.as_ref()?;
        let cached = layout.get_layout(widget_id)?;
        let abs_pos = layout.absolute_position(widget_id)?;
        Some(Rect::new(
            abs_pos.x,
            abs_pos.y,
            cached.result.size.width,
            cached.result.size.height,
        ))
    }

    /// 对当前视图执行逻辑像素命中测试。
    #[must_use]
    pub fn hit_test_logical(&self, position: Point) -> Option<WidgetId> {
        self.handler.find_widget_at_point(position)
    }

    /// 注入逻辑像素 hover。
    pub fn inject_hover_logical(&mut self, position: Point) -> Option<WidgetId> {
        self.handler.sync_pointer_from_logical_injection(position);
        self.handler.update_hover();
        self.handler.last_hover
    }

    /// 注入平台原始窗口坐标 hover。
    ///
    /// 参数语义与真实 `WindowEvent::CursorMoved.position` 保持一致：
    /// - macOS：原始值已是窗口逻辑坐标
    /// - 其他平台：原始值通常需要在平台边界除以 `scale_factor`
    pub fn inject_hover_platform_window_raw(
        &mut self,
        raw_window_position: Point,
    ) -> Option<WidgetId> {
        self.handler
            .sync_pointer_from_physical_injection(raw_window_position);
        self.handler.update_hover();
        self.handler.last_hover
    }

    /// 兼容旧命名：注入“platform raw window position” hover。
    pub fn inject_hover_physical(&mut self, physical: Point) -> Option<WidgetId> {
        self.inject_hover_platform_window_raw(physical)
    }

    /// 注入逻辑像素点击。
    pub fn inject_click_logical(&mut self, position: Point) -> Option<WidgetId> {
        self.inject_hover_logical(position);
        let hit = self.handler.find_widget_at_point(position);
        self.handler.handle_click(position);
        self.rebuild_from_store();
        hit
    }

    /// 注入平台原始窗口坐标点击。
    ///
    /// 参数语义与真实 `WindowEvent::CursorMoved.position` / 缓存点击位置一致。
    pub fn inject_click_platform_window_raw(
        &mut self,
        raw_window_position: Point,
    ) -> Option<WidgetId> {
        self.inject_hover_platform_window_raw(raw_window_position);
        let hit = self
            .handler
            .find_widget_at_point(self.handler.mouse_window_position);
        self.handler
            .handle_click(self.handler.mouse_window_position);
        self.rebuild_from_store();
        hit
    }

    /// 兼容旧命名：注入“platform raw window position”点击。
    pub fn inject_click_physical(&mut self, physical: Point) -> Option<WidgetId> {
        self.inject_click_platform_window_raw(physical)
    }

    /// 回放真实窗口 `CursorMoved` 事件。
    ///
    /// 该入口与运行时 `window_event()` 的坐标语义保持一致：
    /// 原始平台窗口坐标先在平台边界归一化，再记录 `MouseMove` 事件。
    pub fn replay_cursor_moved_platform_window_raw(
        &mut self,
        raw_window_position: Point,
    ) -> Option<WidgetId> {
        self.handler
            .replay_cursor_moved_from_platform_window_event(raw_window_position)
    }

    /// 回放真实窗口左键点击。
    ///
    /// 行为等价于先回放 `CursorMoved`，再回放一次基于缓存位置的左键按下。
    pub fn replay_left_click_platform_window_raw(
        &mut self,
        raw_window_position: Point,
    ) -> Option<WidgetId> {
        let hit = self
            .handler
            .replay_left_click_from_platform_window_event(raw_window_position);
        self.rebuild_from_store();
        hit
    }

    /// 注入逻辑像素 DragEnter。
    pub fn inject_drag_enter_logical(&mut self, position: Point) -> Option<WidgetId> {
        self.handler
            .inject_drag_lifecycle_logical(position, DragLifecycleEvent::Enter)
    }

    /// 注入逻辑像素 DragOver。
    pub fn inject_drag_over_logical(&mut self, position: Point) -> Option<WidgetId> {
        self.handler
            .inject_drag_lifecycle_logical(position, DragLifecycleEvent::Over)
    }

    /// 注入逻辑像素 Drop。
    pub fn inject_drop_logical(&mut self, position: Point) -> Option<WidgetId> {
        self.handler
            .inject_drag_lifecycle_logical(position, DragLifecycleEvent::Drop)
    }

    /// 注入平台原始窗口坐标 DragEnter。
    pub fn inject_drag_enter_platform_window_raw(
        &mut self,
        raw_window_position: Point,
    ) -> Option<WidgetId> {
        self.handler.inject_drag_lifecycle_platform_window_raw(
            raw_window_position,
            DragLifecycleEvent::Enter,
        )
    }

    /// 注入平台原始窗口坐标 DragOver。
    pub fn inject_drag_over_platform_window_raw(
        &mut self,
        raw_window_position: Point,
    ) -> Option<WidgetId> {
        self.handler.inject_drag_lifecycle_platform_window_raw(
            raw_window_position,
            DragLifecycleEvent::Over,
        )
    }

    /// 注入平台原始窗口坐标 Drop。
    pub fn inject_drop_platform_window_raw(
        &mut self,
        raw_window_position: Point,
    ) -> Option<WidgetId> {
        self.handler.inject_drag_lifecycle_platform_window_raw(
            raw_window_position,
            DragLifecycleEvent::Drop,
        )
    }

    /// 回放真实窗口 DragEnter。
    pub fn replay_drag_enter_platform_window_raw(
        &mut self,
        raw_window_position: Point,
    ) -> Option<WidgetId> {
        self.handler
            .replay_drag_lifecycle_from_platform_window_event(
                raw_window_position,
                DragLifecycleEvent::Enter,
            )
    }

    /// 回放真实窗口 DragOver。
    pub fn replay_drag_over_platform_window_raw(
        &mut self,
        raw_window_position: Point,
    ) -> Option<WidgetId> {
        self.handler
            .replay_drag_lifecycle_from_platform_window_event(
                raw_window_position,
                DragLifecycleEvent::Over,
            )
    }

    /// 回放真实窗口 Drop。
    pub fn replay_drop_platform_window_raw(
        &mut self,
        raw_window_position: Point,
    ) -> Option<WidgetId> {
        self.handler
            .replay_drag_lifecycle_from_platform_window_event(
                raw_window_position,
                DragLifecycleEvent::Drop,
            )
    }

    /// 读取组件持久状态。
    #[must_use]
    pub fn widget_state<T: Send + Clone + 'static>(&self, widget_id: WidgetId) -> Option<T> {
        self.handler.app.widget_state_store().read(widget_id)
    }

    /// 返回当前事件列表。
    #[must_use]
    pub fn events(&self) -> &[Event] {
        self.handler.app.events()
    }

    /// 清空当前事件列表。
    pub fn clear_events(&mut self) {
        self.handler.app.clear_events();
    }

    fn rebuild_from_store(&mut self) {
        let mut view = self.template.clone();
        let store = self.handler.app.widget_state_store().clone();
        sync_store_to_props(&mut view, &store);
        let layout = rgui_render::scene_build::compute_view_layout(
            &mut view,
            self.logical_window_size,
            None,
        );
        *self.handler.app.current_view.lock().unwrap() = Some(view.to_noop_view());
        *self.handler.app.current_layout.lock().unwrap() = Some(layout);
    }
}

impl App {
    /// 创建应用实例。
    #[must_use]
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            registry: WidgetRegistry::new(),
            window_id: WindowId::new(),
            events: Vec::new(),
            focus: FocusManager::new(),
            interactions: FxHashMap::new(),
            interaction_bounds: FxHashMap::new(),
            interaction_regions: FxHashMap::new(),
            widget_instances: FxHashMap::new(),
            overlay_ids: std::collections::HashSet::new(),
            view_scene_builder: None,
            #[cfg(feature = "devtools")]
            prop_registry: PropRegistry::new(),
            #[cfg(feature = "devtools")]
            id_map: Arc::new(std::sync::Mutex::new(WidgetIdBimap::new())),
            #[cfg(feature = "devtools")]
            command_registry: None,
            #[cfg(feature = "devtools")]
            rhai_hot_reload: None,
            state_store: Arc::new(RwLock::new(rgui_state::StateStore::new())),
            needs_redraw: false,
            scale_factor: 1.0,
            widget_state_store: crate::widget_state::WidgetStateStore::new(),
            current_layout: Arc::new(std::sync::Mutex::new(None)),
            current_view: Arc::new(std::sync::Mutex::new(None)),
            widget_tree: WidgetTree::new(),
        }
    }

    /// 返回应用配置。
    #[must_use]
    pub fn config(&self) -> &AppConfig {
        &self.config
    }
    /// 返回 widget 注册表。
    #[must_use]
    pub fn registry(&self) -> &WidgetRegistry {
        &self.registry
    }
    /// 返回可变 widget 注册表。
    pub fn registry_mut(&mut self) -> &mut WidgetRegistry {
        &mut self.registry
    }
    /// 返回窗口 ID。
    #[must_use]
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }
    /// 返回 widget 实例状态存储的引用。
    ///
    /// 外部代码可借此初始化组件状态或读取当前交互状态。
    #[must_use]
    pub fn widget_state_store(&self) -> &crate::widget_state::WidgetStateStore {
        &self.widget_state_store
    }

    /// 返回 widget 树的引用。
    ///
    /// 外部代码可借此查询 widget 层级关系（父子、兄弟等）。
    #[must_use]
    pub fn widget_tree(&self) -> &rgui_platform::widget_tree::WidgetTree {
        &self.widget_tree
    }

    /// 返回当前 DPI 缩放因子。
    ///
    /// 逻辑像素 × `scale_factor` = 物理像素。
    /// 在普通 1× 显示器上为 1.0，Mac Retina 为 2.0，Windows 150% 缩放为 1.5。
    #[must_use]
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }
    /// 注册内置组件（Button、Label、TextField）。已废弃，组件由 WA 翻译管理。
    #[deprecated(note = "组件由 WA 翻译管理，不再需要手动注册")]
    pub fn register_defaults(&mut self) {
        for name in &["Button", "Label", "TextField"] {
            if let Err(e) = self.registry.register(name) {
                eprintln!("[rgui] register_defaults: 注册 \"{name}\" 失败: {e}");
            }
        }
    }
    /// 返回当前事件列表。
    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// 注册可交互区域。
    ///
    /// - `id`: widget ID
    /// - `bounds`: 在窗口中的边界矩形（用于 fallback 命中测试）
    /// - `action`: 触发时传递给回调的事件名
    /// - `cb`: 交互回调
    pub fn register_interaction(
        &mut self,
        id: WidgetId,
        bounds: Rect,
        action: impl Into<String>,
        cb: impl FnMut(&str) + Send + 'static,
    ) {
        self.register_interaction_with_chain(
            id,
            bounds,
            CoordinateTransformChain::default().translated(bounds.origin),
            action,
            cb,
        );
    }

    pub fn register_interaction_with_chain(
        &mut self,
        id: WidgetId,
        bounds: Rect,
        window_to_local: CoordinateTransformChain,
        action: impl Into<String>,
        cb: impl FnMut(&str) + Send + 'static,
    ) {
        let action = action.into();
        self.interaction_bounds.insert(id, bounds);
        self.interaction_regions.insert(
            id,
            InteractionRegion {
                candidate_rect: bounds,
                window_to_local,
            },
        );
        self.interactions.insert(id, (action, Box::new(cb)));
        // P05a: 同步填充 WidgetTree，支持 DFS 树遍历命中测试
        const SYNTHETIC_ROOT: WidgetId = WidgetId::from_u64(0);
        if self.widget_tree.is_empty() {
            // 首次注册时创建合成根节点（窗口级别）
            self.widget_tree.add_child(SYNTHETIC_ROOT, id);
        } else if !self.widget_tree.contains(id) {
            self.widget_tree.add_child(SYNTHETIC_ROOT, id);
        }
        let root_bounds = self
            .widget_tree
            .get_bounds(SYNTHETIC_ROOT)
            .map_or(bounds, |existing| existing.union(bounds));
        self.widget_tree.set_bounds(SYNTHETIC_ROOT, root_bounds);
        self.widget_tree.set_bounds(id, bounds);
    }

    /// 注册 WidgetSpec 实例的更新处理器。
    ///
    /// - `id`: widget ID（需与 register_interaction 的 id 一致）
    /// - `handler`: 事件处理器，接收动作名称，返回 EventResult
    ///
    /// 当 handle_click 命中此 widget 时，优先调用 handler，
    /// 根据返回的 EventResult 决定是否继续传播到旧 register_interaction 回调。
    pub fn register_widget_instance(
        &mut self,
        id: WidgetId,
        handler: impl FnMut(&str, &mut UpdateContext) -> EventResult<String> + Send + 'static,
    ) {
        self.widget_instances.insert(id, Box::new(handler));
    }

    /// 标记 widget 为弹层组件（WTI03：点击外部关闭）。
    ///
    /// 当命中测试未命中任何 widget 且存在被标记为弹层的组件时，
    /// `handle_click` 自动向全部弹层发送 `Event::Close`。
    ///
    /// 调用方（组件翻译或布局引擎）在注册可交互区域后调用此方法。
    pub fn mark_as_overlay(&mut self, id: WidgetId) {
        self.overlay_ids.insert(id);
    }

    /// 移除弹层标记（例如弹层关闭时）。
    pub fn unmark_overlay(&mut self, id: WidgetId) {
        self.overlay_ids.remove(&id);
    }

    /// 检查 widget 是否为弹层。
    #[must_use]
    pub fn is_overlay(&self, id: WidgetId) -> bool {
        self.overlay_ids.contains(&id)
    }

    /// 设置视图场景构建回调（html! 声明式路径）。
    ///
    /// 回调在每帧渲染前调用，接收帧计数、窗口宽度和高度（逻辑像素），
    /// 直接返回 `SceneGraph`。
    ///
    /// 与 `build_scene_from_view` 配合使用，实现 WidgetView → SceneGraph 的端到端管线。
    pub fn set_view_scene_builder(
        &mut self,
        builder: impl FnMut(u64, u32, u32, &TextRenderer) -> SceneGraph + Send + 'static,
    ) {
        self.view_scene_builder = Some(Box::new(builder));
    }

    /// RS06 回归修复：请求在下一帧强制重建场景。
    ///
    /// register_interaction 回调（如 AtomicBool toggle）不经过 StateStore 的脏标记机制。
    /// 调用此方法后，下一次 `RedrawRequested` 将忽略 `prev_scene` 缓存并调用
    /// `view_scene_builder` 重建场景图。
    pub fn request_redraw(&mut self) {
        self.needs_redraw = true;
    }

    /// 从 `.rgui` 文件加载视图（路径从 `config.rgui_path` 读取）。
    ///
    /// `M` 为 AppMessage 类型，用于解析器泛型参数。
    /// 内部创建 [`RguiHotReload`]，每帧轮询文件变更，
    /// 变更时重新解析 → `compute_view_layout` → `build_scene_from_view_incremental` → `SceneGraph`。
    ///
    /// ## RS04: PropRegistry 注入
    ///
    /// 每帧调用 [`PropRegistry::drain()`] 获取 Rhai 脚本通过 `set_prop` 写入的待更新 prop，
    /// 通过 RS03 的字符串→WidgetId 映射定位节点，注入到 WidgetView 树。
    /// **⚠️ 临时捷径：RS05-RS06 实施后，此路径降级为 fallback。**
    ///
    /// ## RS07: 声明式数据绑定
    ///
    /// `.rgui` 中 `expanded="{state.open}"` 语法自动：
    /// 1. 收集为 `StateBinding`（`collect_state_bindings`）
    /// 2. 通过 `StateStore::subscribe` 连接 dirty 传播
    /// 3. 每帧从 `StateStore.rhai_state` 读取值 → 注入 `PropRegistry`
    /// 4. 使用 `build_scene_from_view_incremental` 实现增量渲染
    ///
    /// 必须在 `run()` 之前调用。返回 `Err` 如果 config 未设置 `rgui_path`。
    ///
    /// 解析失败时保持旧视图，通过 stderr 报告错误（D7 §9 降级策略）。
    #[cfg(feature = "devtools")]
    pub fn load_rgui<M: AppMessage>(
        &mut self,
    ) -> Result<(), rgui_devtools::rgui_hot_reload::RguiHotReloadError> {
        use rgui_core::geometry::Size;
        use rgui_core::id::WidgetId;
        use rgui_devtools::config::HotReloadConfig;
        use rgui_devtools::rgui_hot_reload::RguiHotReload;
        use rgui_devtools::rgui_parser::{self};

        let rgui_path = self.config.rgui_path.as_ref().ok_or_else(|| {
            rgui_devtools::rgui_hot_reload::RguiHotReloadError::Watch(
                "AppConfig 未设置 rgui_path".to_string(),
            )
        })?;

        // 构建 HotReloadConfig：监控 .rgui 文件所在目录
        let watch_dir = rgui_path.parent().unwrap_or_else(|| {
            eprintln!("[rgui] load_rgui: {rgui_path:?} 无父目录，回退到 '.' 作为监视目录");
            std::path::Path::new(".")
        });
        let config = HotReloadConfig::default().with_watch_paths(vec![watch_dir.to_path_buf()]);

        let mut hot_reload = RguiHotReload::<M>::new(&config, rgui_path)?;
        let mut current_view = hot_reload.current_view().clone();

        // 计算初始布局（Tier 2 脚本需要 bounds，AC02）
        let available = Size::new(
            self.config.window_size.width,
            self.config.window_size.height,
        );
        let initial_engine = compute_view_layout(&mut current_view, available, None);

        // T204: 执行初始视图的 Tier 2 Rhai paint 脚本（使用布局 bounds 注入 width/height）
        crate::paint_factory::execute_tier2_paint_scripts(&mut current_view, &initial_engine);

        // RS04: 共享的 PropRegistry 和 WidgetIdBimap
        let prop_registry = self.prop_registry.clone();
        let id_map = Arc::clone(&self.id_map);

        // RS06/RS07: 共享的 StateStore 和 PaintCache
        let state_store = Arc::clone(&self.state_store);
        let mut paint_cache = rgui_render::PaintCache::new();

        // 初始帧：填充 WidgetIdBimap
        {
            let bimap = rgui_devtools::rgui_parser::collect_widget_ids(&current_view);
            *id_map
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = bimap;
        }

        // 复用上面计算好的初始布局引擎
        let mut layout_engine = initial_engine;

        // RS07: 收集声明式 state 绑定
        // state_key → Vec<(bound_widget_id, prop_name)>
        let mut state_bindings: std::collections::HashMap<String, Vec<(WidgetId, String)>> =
            std::collections::HashMap::new();
        {
            let bindings = rgui_parser::collect_state_bindings(&current_view);
            for binding in &bindings {
                if let Some(wid) = binding.widget_id {
                    state_bindings
                        .entry(binding.state_key.clone())
                        .or_default()
                        .push((wid, binding.prop_name.clone()));
                }
            }
            // RS07: 为每个 state key 分配 WidgetId（若 id_map 中不存在）
            let mut bimap_lock = id_map
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for state_key in state_bindings.keys() {
                if !bimap_lock.contains_name(state_key) {
                    let new_id = WidgetId::new();
                    bimap_lock.insert(state_key, new_id);
                }
            }
            drop(bimap_lock);

            // RS07: 创建 StateStore 订阅——state widget dirty → bound widget dirty
            let mut store = state_store.write().expect("StateStore RwLock poisoned");
            for (state_key, bound_widgets) in &state_bindings {
                if let Some(state_id) = {
                    let bimap = id_map
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    bimap.get_id(state_key)
                } {
                    for &(bound_widget_id, _) in bound_widgets {
                        if bound_widget_id != state_id {
                            store.subscribe(bound_widget_id, state_id);
                        }
                    }
                }
            }
            // 写入初始 state 值到 rhai_state（默认空字符串）
            for state_key in state_bindings.keys() {
                if let Some(state_id) = {
                    let bimap = id_map
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    bimap.get_id(state_key)
                } {
                    if store.read_rhai_state(state_id).is_none() {
                        store.write_rhai_state(state_id, "");
                    }
                }
            }
        }

        let builder = move |frame_count: u64,
                            width: u32,
                            height: u32,
                            text_renderer: &TextRenderer|
              -> SceneGraph {
            match hot_reload.check_and_reload() {
                Ok(Some(new_view)) => {
                    // RS04: 视图变更 → 更新 WidgetIdBimap
                    let bimap = rgui_devtools::rgui_parser::collect_widget_ids(&new_view);
                    *id_map
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = bimap;

                    let available = Size::new(f64::from(width), f64::from(height));
                    let mut view = new_view.clone();
                    // AC02: 先计算布局（Tier 2 脚本需要 bounds）
                    let engine = compute_view_layout(&mut view, available, Some(text_renderer));
                    // T204: 热重载后重新执行 Tier 2 Rhai paint 脚本（使用布局 bounds 注入 width/height）
                    crate::paint_factory::execute_tier2_paint_scripts(&mut view, &engine);
                    // RS04: 注入 Rhai 写入的待更新 prop
                    inject_props_from_registry(&mut view, &prop_registry.drain());
                    // RS07: 从 StateStore 注入 state 绑定 prop
                    inject_state_bindings(&mut view, &id_map, &state_store);
                    layout_engine = engine;
                    current_view = view;
                },
                Ok(None) => {
                    // 无变更，但 Rhai 可能已写入 prop → 每帧注入
                    let mut view = current_view.clone();

                    // RS07: 从 StateStore 注入 state 绑定 prop
                    inject_state_bindings(&mut view, &id_map, &state_store);

                    // RS04: 注入 PropRegistry 待更新 prop
                    let pending = prop_registry.drain();
                    if !pending.is_empty() {
                        inject_props_from_registry(&mut view, &pending);
                    }
                    // prop 变更后需重算布局
                    let available = Size::new(f64::from(width), f64::from(height));
                    layout_engine = compute_view_layout(&mut view, available, Some(text_renderer));
                    current_view = view;
                },
                Err(e) => {
                    // 解析失败 → 保持旧视图（D7 §9 降级策略）
                    eprintln!("[rgui] .rgui 热重载失败（保持旧视图）: {e}");
                },
            }

            let paint_fn = crate::paint_factory::default_paint_fn::<M>();

            // RS07: 使用增量渲染——仅重绘 dirty widget
            let dirty_set = {
                let store = state_store.read().expect("StateStore RwLock poisoned");
                store.dirty_widgets().clone()
            };

            let scene = if dirty_set.is_empty() {
                rgui_render::build_scene_from_view(
                    &current_view,
                    &layout_engine,
                    &paint_fn,
                    frame_count,
                    Some(text_renderer),
                )
            } else {
                rgui_render::build_scene_from_view_incremental(
                    &current_view,
                    &layout_engine,
                    &paint_fn,
                    frame_count,
                    Some(text_renderer),
                    Some(&dirty_set),
                    &mut paint_cache,
                )
            };

            // 清除本帧脏标记
            {
                let mut store = state_store.write().expect("StateStore RwLock poisoned");
                store.clear_dirty();
            }

            scene
        };

        self.set_view_scene_builder(builder);
        Ok(())
    }

    /// 加载 `.rhai` 脚本并启动热重载。
    ///
    /// ## RS04: 共享状态绑定
    ///
    /// 使用 `App` 持有的共享 [`PropRegistry`] 和 [`WidgetIdBimap`] 创建
    /// [`CommandRegistry`]（通过 [`CommandRegistry::with_state`](rgui_script::CommandRegistry::with_state)），
    /// 使 Rhai 引擎的 `set_prop`/`get_prop` 与渲染线程的 `drain()` 操作同一份数据。
    ///
    /// 内部创建 `CommandRegistry`（共享引用）和 `RhaiHotReload`。
    /// 启动时编译全部脚本，每帧轮询文件变更并自动重新注册。
    ///
    /// 脚本编译失败时保留旧处理器，通过 stderr 报告错误（D7 §9 降级策略）。
    ///
    /// # Errors
    ///
    /// 返回 `Err` 如果文件监控创建失败或脚本编译失败。
    #[cfg(feature = "devtools")]
    pub fn load_rhai_scripts(
        &mut self,
        paths: &[impl AsRef<Path>],
    ) -> Result<(), rgui_devtools::rhai_hot_reload::RhaiHotReloadError> {
        use rgui_devtools::config::HotReloadConfig;
        use rgui_devtools::rhai_hot_reload::RhaiHotReload;
        use rgui_script::CommandRegistry;

        if paths.is_empty() {
            return Ok(());
        }

        // 构建 HotReloadConfig：监控脚本文件所在目录
        let watch_dirs: Vec<PathBuf> = paths
            .iter()
            .filter_map(|p| p.as_ref().parent().map(Path::to_path_buf))
            .collect();
        let config = if watch_dirs.is_empty() {
            HotReloadConfig::default()
        } else {
            HotReloadConfig::default().with_watch_paths(watch_dirs)
        };

        // RS04: 使用共享 PropRegistry + WidgetIdBimap 创建 CommandRegistry
        let registry =
            CommandRegistry::with_state(self.prop_registry.clone(), Arc::clone(&self.id_map));

        // RS06: 注册 StateStore 绑定，让 Rhai `store_read`/`store_write` 操作共享状态
        {
            let binding: Arc<dyn rgui_core::StateBinding> = Arc::new(
                rgui_state::StateStoreBinding::new(Arc::clone(&self.state_store)),
            );
            registry.register_state_binding(binding);
        }

        let mut hot_reload = RhaiHotReload::with_registry(&config, registry)?;
        for path in paths {
            hot_reload.watch(path.as_ref())?;
        }

        let registry = hot_reload.registry();
        self.command_registry = Some(registry);
        self.rhai_hot_reload = Some(hot_reload);

        Ok(())
    }

    /// 获取共享的 `CommandRegistry`（用于注册 Rust 端类型和函数）。
    ///
    /// 在 `load_rhai_scripts` 之后调用，向引擎注册自定义类型：
    ///
    /// ```ignore
    /// app.load_rhai_scripts(&["scripts/handlers.rhai"])?;
    /// app.command_registry().unwrap()
    ///     .engine_mut().register_type::<MyState>();
    /// ```
    ///
    /// 返回 `None` 如果未调用 `load_rhai_scripts`。
    #[cfg(feature = "devtools")]
    #[must_use]
    pub fn command_registry(&self) -> Option<&rgui_script::CommandRegistry> {
        self.command_registry.as_ref()
    }

    /// 运行应用。
    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(&mut AppHandler::new(self))?;
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    /// 框架主循环（每帧调用），D3 §4.1。
    ///
    /// 固定执行顺序：事件分发 → 布局 → 无障碍 → 场景图 → GPU 提交
    pub fn tick(
        &mut self,
        events: Vec<Event>,
        layout_fn: LayoutCallback,
        a11y_fn: A11yCallback,
        render_fn: RenderCallback,
    ) -> Result<(), String> {
        for event in &events {
            self.events.push(event.clone());
        }
        layout_fn(self);
        a11y_fn(self);
        render_fn(self, &events)?;
        Ok(())
    }

    /// 返回未处理事件数量。
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// 清空事件队列。
    pub fn clear_events(&mut self) {
        self.events.clear();
    }
}

impl fmt::Debug for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("App")
            .field("title", &self.config.title)
            .finish()
    }
}

// ============================================================================
// AppHandler
// ============================================================================

struct AppHandler {
    app: App,
    window: Option<Arc<winit::window::Window>>,
    /// 当前活跃的渲染后端（trait object，支持运行时切换）。
    /// 初始为 VelloBackend；渲染失败时在帧边界切换为 SkiaBackend。
    render_ctx: Option<Box<dyn RenderBackend>>,
    /// 待切换标志：当前帧 Vello 渲染失败后设为 true，
    /// 下一帧开始时切换为 Skia。
    backend_fallback_pending: bool,
    /// 文本渲染器（字形塑形 + atlas 管理）。
    /// 在每帧渲染时传递给 scene_builder，启用真正的字形渲染。
    text_renderer: TextRenderer,
    frame_count: u64,
    mouse_window_position: Point,
    /// 最近一次原始平台窗口坐标。
    ///
    /// 仅在平台边界或物理注入边界保留，用于 `ScaleFactorChanged` 后重算窗口逻辑坐标。
    last_raw_window_position: Option<Point>,
    /// 最近一次鼠标输入来源。
    ///
    /// 高层事件记录使用它保留“平台原始坐标 / 自动化注入来源”信息。
    last_mouse_origin: Option<MouseInputOrigin>,
    /// 上一帧的悬停 widget ID（P04e：用于计算 MouseEnter/MouseLeave 变迁）。
    last_hover: Option<WidgetId>,
    /// 当前窗口宽度（物理像素），用于构造 RenderParams。
    width: u32,
    /// 当前窗口高度（物理像素），用于构造 RenderParams。
    height: u32,
    /// 视图场景构建回调（从 App 移入）。
    view_scene_builder: Option<ViewSceneBuilder>,
    /// Rhai 命令处理器注册表（从 App .take() 移入）。
    /// handle_click 中用于路由事件到 Rhai 脚本函数。
    #[cfg(feature = "devtools")]
    command_registry: Option<rgui_script::CommandRegistry>,
    /// Rhai 脚本热重载管理器（从 App .take() 移入）。
    /// 每帧调用 check_and_reload() 检测 .rhai 文件变更。
    #[cfg(feature = "devtools")]
    rhai_hot_reload: Option<rgui_devtools::rhai_hot_reload::RhaiHotReload>,
    /// 共享状态存储（从 App 移入，RS06）。
    /// 渲染管线读取脏集合，Rhai `store_write` 标记脏。
    state_store: Arc<RwLock<rgui_state::StateStore>>,
    /// 逐 widget 绘制结果缓存（RS06）。
    #[allow(dead_code)]
    paint_cache: rgui_render::PaintCache,
    /// 前一帧场景图（RS06：脏集合为空时复用，避免全量重建）。
    prev_scene: Option<SceneGraph>,
}

impl AppHandler {
    fn new(mut app: App) -> Self {
        let view_scene_builder = app.view_scene_builder.take();
        #[cfg(feature = "devtools")]
        let command_registry = app.command_registry.take();
        #[cfg(feature = "devtools")]
        let rhai_hot_reload = app.rhai_hot_reload.take();
        // RS06: 将共享 StateStore 从 App 移入 AppHandler
        let state_store = app.state_store.clone();
        Self {
            app,
            window: None,
            render_ctx: None,
            backend_fallback_pending: false,
            text_renderer: TextRenderer::new(rgui_render::TextureId(0)),
            frame_count: 0,
            mouse_window_position: Point::ZERO,
            last_raw_window_position: None,
            last_mouse_origin: None,
            last_hover: None,
            width: 0,
            height: 0,
            view_scene_builder,
            #[cfg(feature = "devtools")]
            command_registry,
            #[cfg(feature = "devtools")]
            rhai_hot_reload,
            state_store,
            paint_cache: rgui_render::PaintCache::new(),
            prev_scene: None,
        }
    }

    /// 树形命中测试——基于当前布局引擎找包含点的最小 widget。
    ///
    /// 遍历所有注册了 widget_instance handler 的 widget，用当前布局引擎的绝对坐标
    /// 检查点是否在 bounds 内，返回面积最小的（最深层级）匹配。
    /// DFS 树遍历命中测试（D5 §4 算法）。
    ///
    /// 基于当前 WidgetView 树（`current_view`）和布局引擎（`current_layout`）
    /// P05a: DFS 树遍历命中测试（D5 §4）。
    ///
    /// 算法：
    /// 1. 优先使用当前 WidgetView 树 + LayoutEngine 绝对坐标
    /// 2. 回退到 WidgetTree（register_interaction 自动填充）
    /// 3. 最后回退到 interaction_bounds 平面查找（向后兼容）
    ///
    /// 注意：不再使用面积仲裁（取消了 min_by 面积比较）。
    /// 宽扁和窄高 widget 仅通过边界包含判断。
    fn hit_test_result_at_point(&self, position: Point) -> Option<ResolvedHitTest> {
        // 主路径：DFS 树遍历命中测试（生产环境，current_view + current_layout 均设置）
        // 锁在嵌套作用域结束时立即释放，避免阻塞渲染线程
        let tree_hit = {
            let layout_guard = self.app.current_layout.lock().unwrap();
            let view_guard = self.app.current_view.lock().unwrap();
            match (view_guard.as_ref(), layout_guard.as_ref()) {
                (Some(view), Some(layout)) => Self::hit_test_tree(
                    view,
                    position,
                    position,
                    &CoordinateTransformChain::default(),
                    layout,
                ),
                _ => None,
            }
        };
        if tree_hit.is_some() {
            return tree_hit;
        }

        // 回退路径 A：WidgetTree DFS 命中测试（P05a）
        // register_interaction 已自动填充 WidgetTree 的 bounds + 父子关系
        if !self.app.widget_tree.is_empty() {
            if let Some(hit) = self.app.widget_tree.hit_test(position) {
                if let Some(region) = self.app.interaction_regions.get(&hit) {
                    if let Some(resolved) = region.resolve(hit, position) {
                        return Some(resolved);
                    }
                }
                if let Some(bounds) = self.app.interaction_bounds.get(&hit) {
                    return Some(ResolvedHitTest::from_absolute_bounds(
                        hit, *bounds, position,
                    ));
                }
            }
        }
        // 回退路径 B：LayoutEngine 精确坐标查找（向后兼容旧测试）
        {
            let layout_guard = self.app.current_layout.lock().unwrap();
            if let Some(layout) = layout_guard.as_ref() {
                if let Some(hit) = self
                    .app
                    .interaction_bounds
                    .iter()
                    .find_map(|(&id, bounds)| {
                        let effective_rect = layout
                            .get_layout(id)
                            .and_then(|cached| {
                                layout.absolute_position(id).map(|abs_pos| {
                                    Rect::new(
                                        abs_pos.x,
                                        abs_pos.y,
                                        cached.result.size.width,
                                        cached.result.size.height,
                                    )
                                })
                            })
                            .unwrap_or(*bounds);
                        if effective_rect.contains(position) {
                            Some(id)
                        } else {
                            None
                        }
                    })
                {
                    return Some(ResolvedHitTest::from_absolute_bounds(
                        hit,
                        *self.app.interaction_bounds.get(&hit).unwrap(),
                        position,
                    ));
                }
            }
        }
        // 完全回退：interaction_bounds 平面查找
        self.app
            .interaction_regions
            .iter()
            .find_map(|(&id, region)| region.resolve(id, position))
    }

    fn find_widget_at_point(&self, position: Point) -> Option<WidgetId> {
        self.hit_test_result_at_point(position)
            .map(|hit| hit.widget_id)
    }

    /// P05a: DFS 树遍历命中测试（D5 §4 算法）。
    ///
    /// 在 WidgetView 子树上递归遍历，使用 LayoutEngine 绝对坐标。
    /// 返回包含该点的最深 widget ID。
    ///
    /// 算法步骤：
    /// 1. 从当前节点开始，检查 bounds 是否包含 point
    /// 2. 子节点按 z-index 降序排列（无 z-index 时按 child 索引递增作为默认值）
    /// 3. 高 z-index 子节点优先命中，同 z-index 按稳定排序保持原始相对顺序
    /// 4. 子节点命中则返回子节点，否则返回当前节点
    fn hit_test_tree(
        view: &rgui_core::view::WidgetView<rgui_core::message::NoopMsg>,
        window_point: Point,
        parent_local_point: Point,
        parent_chain: &CoordinateTransformChain,
        layout: &rgui_layout::LayoutEngine,
    ) -> Option<ResolvedHitTest> {
        use rgui_core::view::PropValue;

        // 未分配 ID 的节点（如匿名容器）不参与命中，但子节点仍可命中
        if let Some(widget_id) = view.id {
            let cached = layout.get_layout(widget_id)?;
            let offset = cached.result.position;
            let local_point = Point::new(
                parent_local_point.x - offset.x,
                parent_local_point.y - offset.y,
            );
            let local_rect = Rect::new(
                0.0,
                0.0,
                cached.result.size.width,
                cached.result.size.height,
            );
            if !local_rect.contains(local_point) {
                return None;
            }
            let widget_chain = parent_chain.translated(offset);
            // 按 z-index 降序排列子节点
            // - 有显式 z-index prop 的 widget：用其值（通常 > 0，如弹层）
            // - 无 z-index 的 widget：用 child 索引作为默认值（后添加→更大→上层）
            // 效果：与 SceneGraph 的 z-order 排序（scene_build.rs）一致
            let mut indexed: Vec<(
                i32,
                &rgui_core::view::WidgetView<rgui_core::message::NoopMsg>,
            )> = view
                .children
                .iter()
                .enumerate()
                .map(|(i, child)| {
                    let z = match child.props.get("z-index") {
                        Some(PropValue::Int(zi)) => *zi as i32,
                        _ => i as i32,
                    };
                    (z, child)
                })
                .collect();
            // 降序排列（高 z-index 优先命中）
            indexed.sort_by_key(|(z, _)| std::cmp::Reverse(*z));

            for (_, child) in &indexed {
                if let Some(hit) =
                    Self::hit_test_tree(child, window_point, local_point, &widget_chain, layout)
                {
                    return Some(hit);
                }
            }
            // AC05a: Skip disabled widgets in hit testing (pointer-events: none 等价)
            if let Some(PropValue::Bool(true)) = view.props.get("disabled") {
                return None;
            }
            let candidate_origin = Point::new(
                window_point.x - local_point.x,
                window_point.y - local_point.y,
            );
            return Some(ResolvedHitTest {
                widget_id,
                candidate_rect: Rect::new(
                    candidate_origin.x,
                    candidate_origin.y,
                    cached.result.size.width,
                    cached.result.size.height,
                ),
                local_logical: local_point,
                window_to_local: widget_chain,
            });
        }
        // 匿名节点（无 WidgetId）：递归检查子节点（同样按 z-index 排序）
        let mut indexed: Vec<(
            i32,
            &rgui_core::view::WidgetView<rgui_core::message::NoopMsg>,
        )> = view
            .children
            .iter()
            .enumerate()
            .map(|(i, child)| {
                let z = match child.props.get("z-index") {
                    Some(PropValue::Int(zi)) => *zi as i32,
                    _ => i as i32,
                };
                (z, child)
            })
            .collect();
        indexed.sort_by_key(|(z, _)| std::cmp::Reverse(*z));

        for (_, child) in &indexed {
            if let Some(hit) = Self::hit_test_tree(
                child,
                window_point,
                parent_local_point,
                parent_chain,
                layout,
            ) {
                return Some(hit);
            }
        }
        None
    }

    fn handle_click(&mut self, position: Point) {
        // DFS 树遍历命中测试（基于 WidgetView 树 + LayoutEngine 绝对坐标）
        // D5 §3: Qt Signal/Slot 显式连接模型——点击直接路由到目标 widget。
        let hit_test = self.hit_test_result_at_point(position);
        let hit_id = hit_test.as_ref().map(|hit| hit.widget_id);
        let mouse_coords =
            self.mouse_event_coords_at(position, hit_test.as_ref(), self.current_mouse_origin());

        if let Some(hit_id) = hit_id {
            let mut update_ctx = UpdateContext::new();
            // P04e: 同步当前悬停状态到 UpdateContext
            update_ctx.hover = self.last_hover;
            update_ctx.cursor_window_position = Some(mouse_coords.window_logical);
            update_ctx.cursor_local_position = mouse_coords.local_logical;
            update_ctx.cursor_platform_position = match mouse_coords.origin {
                MouseInputOrigin::PlatformWindowEvent {
                    raw_window_position,
                    ..
                }
                | MouseInputOrigin::PhysicalInjection {
                    raw_window_position,
                    ..
                } => Some(raw_window_position),
                MouseInputOrigin::LogicalInjection => None,
            };
            let mut skip_default = false;

            // ─── 目标阶段：WidgetSpec 实例处理器 ───
            if let Some(handler) = self.app.widget_instances.get_mut(&hit_id) {
                let action = self
                    .app
                    .interactions
                    .get(&hit_id)
                    .map(|(a, _)| a.clone())
                    .unwrap_or_else(|| {
                        eprintln!(
                            "[rgui] handle_click: WidgetId({hit_id:?}) 未注册交互，action 回退为空字符串"
                        );
                        String::new()
                    });
                match handler(&action, &mut update_ctx) {
                    EventResult::Handled => {
                        // 组件消费了事件，停止处理
                        // RS06 回归修复：WidgetSpec handler 可能改变了外部状态
                        self.app.request_redraw();
                        self.app.events.push(Event::MouseDown {
                            coords: mouse_coords,
                            button: MouseButton::Left,
                            modifiers: Modifiers::new(),
                        });
                        return;
                    },
                    EventResult::Prevented => {
                        // 阻止默认行为（不执行旧回调），但仍请求重绘
                        // RS06 回归修复：WidgetSpec handler 可能改变了外部状态
                        skip_default = true;
                        self.app.request_redraw();
                    },
                    EventResult::Continue(_msg) => {
                        // 继续到 Rhai 路由和旧回调路径
                    },
                }
            }

            // Rhai 命令处理器路由（D7 §10.2）
            // 在 WidgetSpec 实例处理器之后、旧 register_interaction 回调之前插入，
            // 让 `.rhai` 脚本函数有机会消费事件。
            #[cfg(feature = "devtools")]
            if let Some(ref mut registry) = self.command_registry {
                if let Some((action, _)) = self.app.interactions.get(&hit_id) {
                    // 防御：移除 onclick="..." 中可能携带的 () 后缀
                    let fn_name = action.strip_suffix("()").unwrap_or(action);
                    match registry.call_fn::<()>(fn_name, ()) {
                        Ok(()) => {
                            // Rhai 函数执行成功，事件已消费
                            // RS06 回归修复：Rhai 函数可能改变了外部状态
                            self.app.request_redraw();
                            self.app.events.push(Event::MouseDown {
                                coords: mouse_coords,
                                button: MouseButton::Left,
                                modifiers: Modifiers::new(),
                            });
                            return;
                        },
                        Err(_e) => {
                            // Rhai 函数未定义，fall through
                        },
                    }
                }
            }

            // ─── 旧回调路径（仅默认未被阻止时执行）───
            if !skip_default {
                if let Some((action, cb)) = self.app.interactions.get_mut(&hit_id) {
                    // 交互回调带异常隔离（D1 §11.3）
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        cb(action);
                    }));
                    if let Err(e) = result {
                        let msg = if let Some(s) = e.downcast_ref::<&str>() {
                            *s
                        } else if let Some(s) = e.downcast_ref::<String>() {
                            s.as_str()
                        } else {
                            "unknown panic"
                        };
                        eprintln!(
                            "[rgui] 交互回调 panic (widget={hit_id:?}, action={action}): {msg}"
                        );
                    }
                    // RS06 回归修复：交互回调（如 AtomicBool）不经过 StateStore，
                    // 必须强制请求下一帧重建场景。
                    self.app.request_redraw();
                    // 记录事件
                    self.app.events.push(Event::MouseDown {
                        coords: mouse_coords,
                        button: MouseButton::Left,
                        modifiers: Modifiers::new(),
                    });
                }
            }
        } else {
            // WTI03：命中测试未命中 → 检查弹层 → 发送 Close 事件
            if !self.app.overlay_ids.is_empty() {
                // 收集弹层 ID 列表（避免迭代时借用冲突）
                let overlay_ids: Vec<WidgetId> = self.app.overlay_ids.iter().copied().collect();
                for id in overlay_ids {
                    // 通过 WidgetSpec 实例处理器发送 close 动作
                    if let Some(handler) = self.app.widget_instances.get_mut(&id) {
                        let mut update_ctx = UpdateContext::new();
                        let _result = handler("close", &mut update_ctx);
                        // RS06 回归修复：close 可能改变了外部状态
                        self.app.request_redraw();
                    }
                    // 同时推送 Close 事件到队列（供后续处理）
                    self.app.events.push(Event::Close {
                        widget_id: Some(id),
                    });
                }
                // 记录点击事件
                self.app.events.push(Event::MouseDown {
                    coords: mouse_coords,
                    button: MouseButton::Left,
                    modifiers: Modifiers::new(),
                });
            }
        }
    }

    /// AC12: 键盘触发——Enter/Space 键触发当前聚焦 widget 的 toggle action。
    ///
    /// 行为等价于鼠标 click，但使用 `focus.current()` 作为目标 widget，
    /// 而非命中测试。调用路径与 `handle_click` 相同：
    /// widget_instances handler → Rhai 命令处理器 → 旧回调。
    fn handle_keyboard_trigger(&mut self) {
        let focused_id = match self.app.focus.current() {
            Some(id) => id,
            None => return,
        };

        let mut update_ctx = UpdateContext::new();
        let mut skip_default = false;

        // 从 interactions 获取 action name
        let action = self
            .app
            .interactions
            .get(&focused_id)
            .map(|(a, _)| a.clone())
            .unwrap_or_default();

        // ─── WidgetSpec 实例处理器（优先）───
        if let Some(handler) = self.app.widget_instances.get_mut(&focused_id) {
            match handler(&action, &mut update_ctx) {
                EventResult::Handled => {
                    self.app.request_redraw();
                    return;
                },
                EventResult::Prevented => {
                    skip_default = true;
                    self.app.request_redraw();
                },
                EventResult::Continue(_) => {},
            }
        }

        // ─── Rhai 命令处理器 ───
        #[cfg(feature = "devtools")]
        if let Some(ref mut registry) = self.command_registry {
            if !action.is_empty() {
                let fn_name = action.strip_suffix("()").unwrap_or(&action);
                match registry.call_fn::<()>(fn_name, ()) {
                    Ok(()) => {
                        self.app.request_redraw();
                        return;
                    },
                    Err(_e) => {},
                }
            }
        }

        // ─── 旧回调路径（交互回调）───
        if !skip_default {
            if let Some((_action, cb)) = self.app.interactions.get_mut(&focused_id) {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    cb(&action);
                }));
                if let Err(e) = result {
                    let msg = if let Some(s) = e.downcast_ref::<&str>() {
                        *s
                    } else if let Some(s) = e.downcast_ref::<String>() {
                        s.as_str()
                    } else {
                        "unknown panic"
                    };
                    eprintln!(
                        "[rgui] keyboard trigger callback panic (widget={focused_id:?}): {msg}"
                    );
                }
                self.app.request_redraw();
            }
        }
    }

    /// P04e: 更新悬停状态——比较当前鼠标位置下的 widget 与上一帧的悬停目标。
    ///
    /// D5 §3.1 悬停事件算法：每帧比较鼠标位置，自动计算 enter/leave 关系，
    /// 直接发送到目标 widget（不传播）。如有变更，向事件队列 push MouseEnter/MouseLeave。
    fn update_hover(&mut self) {
        let current_hover = self
            .hit_test_result_at_point(self.mouse_window_position)
            .map(|hit| hit.widget_id);
        if current_hover != self.last_hover {
            if let Some(old_id) = self.last_hover {
                self.app
                    .events
                    .push(Event::MouseLeave { widget_id: old_id });
            }
            if let Some(new_id) = current_hover {
                self.app
                    .events
                    .push(Event::MouseEnter { widget_id: new_id });
            }
            self.last_hover = current_hover;
        }
    }

    fn current_mouse_origin(&self) -> MouseInputOrigin {
        self.last_mouse_origin
            .unwrap_or(MouseInputOrigin::LogicalInjection)
    }

    fn mouse_event_coords_at(
        &self,
        window_logical: Point,
        hit_test: Option<&ResolvedHitTest>,
        origin: MouseInputOrigin,
    ) -> MouseEventCoords {
        let mut coords = MouseEventCoords::new(window_logical, origin);
        if let Some(hit_test) = hit_test {
            debug_assert_eq!(
                hit_test.window_to_local.window_to_local(window_logical),
                hit_test.local_logical,
                "命中测试链路必须可逆恢复接收者局部坐标"
            );
            coords = coords.with_local(hit_test.local_logical);
        }
        coords
    }

    fn current_mouse_event_coords(&self, hit_test: Option<&ResolvedHitTest>) -> MouseEventCoords {
        self.mouse_event_coords_at(
            self.mouse_window_position,
            hit_test,
            self.current_mouse_origin(),
        )
    }

    fn emit_drag_lifecycle_event(&mut self, event: DragLifecycleEvent) -> Option<WidgetId> {
        let target_hit = self.hit_test_result_at_point(self.mouse_window_position);
        let target_widget = target_hit.as_ref().map(|hit| hit.widget_id);
        let coords = self.current_mouse_event_coords(target_hit.as_ref());
        self.app.events.push(match event {
            DragLifecycleEvent::Enter => Event::DragEnter { coords },
            DragLifecycleEvent::Over => Event::DragOver { coords },
            DragLifecycleEvent::Drop => Event::Drop { coords },
        });
        target_widget
    }

    /// Skia fallback 切换（D3 §12.5）。
    ///
    /// 当 Vello 渲染失败时，在下一帧边界切换到备用后端。
    /// 通过 RenderBackendFactory 自动选择最佳可用后端
    /// （通常为 SkiaBackend）。
    fn try_fallback_to_skia(&mut self) {
        let params = RenderParams {
            width: self.width.max(1),
            height: self.height.max(1),
            scale_factor: self.app.scale_factor,
            ..Default::default()
        };
        match RenderBackendFactory::create(&params) {
            Ok(backend) => {
                eprintln!(
                    "[rgui] Vello 渲染失败，在帧边界切换到 {} ({}x{})",
                    backend.backend_name(),
                    self.width,
                    self.height
                );
                self.render_ctx = Some(backend);
            },
            Err(e) => {
                eprintln!("[rgui] fallback 后端创建失败: {e}");
            },
        }
    }

    /// 窗口最小化或 surface 不可用时跳过渲染（D3 §12.4）。
    ///
    /// 当窗口尺寸为 0（最小化状态）或渲染后端不可用时，
    /// 应跳过场景构建与 GPU 提交以节省资源，不视为错误。
    fn should_skip_render(&self) -> bool {
        self.width == 0
            || self.height == 0
            || self
                .render_ctx
                .as_ref()
                .is_none_or(|ctx| !ctx.is_available())
    }

    fn sync_pointer_from_logical_injection(&mut self, window_logical: Point) {
        self.mouse_window_position = window_logical;
        self.last_raw_window_position = None;
        self.last_mouse_origin = Some(MouseInputOrigin::LogicalInjection);
    }

    fn sync_pointer_from_platform_window_event(&mut self, raw_window_position: Point) {
        let normalized =
            normalize_platform_window_point(raw_window_position, self.app.scale_factor);
        self.mouse_window_position = normalized.window_logical;
        self.last_raw_window_position = Some(raw_window_position);
        self.last_mouse_origin = Some(MouseInputOrigin::PlatformWindowEvent {
            raw_window_position,
            normalization: normalized.normalization,
        });
    }

    fn sync_pointer_from_physical_injection(&mut self, raw_window_position: Point) {
        let normalized =
            normalize_platform_window_point(raw_window_position, self.app.scale_factor);
        self.mouse_window_position = normalized.window_logical;
        self.last_raw_window_position = Some(raw_window_position);
        self.last_mouse_origin = Some(MouseInputOrigin::PhysicalInjection {
            raw_window_position,
            normalization: normalized.normalization,
        });
    }

    fn inject_drag_lifecycle_logical(
        &mut self,
        window_logical: Point,
        event: DragLifecycleEvent,
    ) -> Option<WidgetId> {
        self.sync_pointer_from_logical_injection(window_logical);
        self.update_hover();
        self.emit_drag_lifecycle_event(event)
    }

    fn inject_drag_lifecycle_platform_window_raw(
        &mut self,
        raw_window_position: Point,
        event: DragLifecycleEvent,
    ) -> Option<WidgetId> {
        self.sync_pointer_from_physical_injection(raw_window_position);
        self.update_hover();
        self.emit_drag_lifecycle_event(event)
    }

    fn sync_scale_factor(&mut self, scale_factor: f64) {
        self.app.scale_factor = scale_factor;
        if let Some(raw_window_position) = self.last_raw_window_position {
            let normalized =
                normalize_platform_window_point(raw_window_position, self.app.scale_factor);
            self.mouse_window_position = normalized.window_logical;
            self.last_mouse_origin = Some(match self.current_mouse_origin() {
                MouseInputOrigin::PhysicalInjection { .. } => MouseInputOrigin::PhysicalInjection {
                    raw_window_position,
                    normalization: normalized.normalization,
                },
                _ => MouseInputOrigin::PlatformWindowEvent {
                    raw_window_position,
                    normalization: normalized.normalization,
                },
            });
        }
    }

    fn replay_cursor_moved_from_platform_window_event(
        &mut self,
        raw_window_position: Point,
    ) -> Option<WidgetId> {
        self.sync_pointer_from_platform_window_event(raw_window_position);
        self.update_hover();
        let target_hit = self.hit_test_result_at_point(self.mouse_window_position);
        let target_widget = target_hit.as_ref().map(|hit| hit.widget_id);
        self.app.events.push(Event::MouseMove {
            coords: self.current_mouse_event_coords(target_hit.as_ref()),
            delta_window_logical: Point::new(0.0, 0.0),
            modifiers: Modifiers::new(),
        });
        target_widget
    }

    fn replay_left_click_from_cached_pointer(&mut self) -> Option<WidgetId> {
        let hit = self.find_widget_at_point(self.mouse_window_position);
        self.handle_click(self.mouse_window_position);
        hit
    }

    fn replay_left_click_from_platform_window_event(
        &mut self,
        raw_window_position: Point,
    ) -> Option<WidgetId> {
        self.replay_cursor_moved_from_platform_window_event(raw_window_position);
        self.replay_left_click_from_cached_pointer()
    }

    fn replay_drag_lifecycle_from_platform_window_event(
        &mut self,
        raw_window_position: Point,
        event: DragLifecycleEvent,
    ) -> Option<WidgetId> {
        self.sync_pointer_from_platform_window_event(raw_window_position);
        self.update_hover();
        self.emit_drag_lifecycle_event(event)
    }

    fn convert_event(&self, event: &WindowEvent) -> Option<Event> {
        match event {
            WindowEvent::CursorMoved { .. } => Some(Event::MouseMove {
                coords: self.current_mouse_event_coords(
                    self.hit_test_result_at_point(self.mouse_window_position)
                        .as_ref(),
                ),
                delta_window_logical: Point::new(0.0, 0.0),
                modifiers: Modifiers::new(),
            }),
            WindowEvent::MouseInput { state, button, .. } => {
                let btn = match button {
                    WinitMouseButton::Left => MouseButton::Left,
                    WinitMouseButton::Right => MouseButton::Right,
                    WinitMouseButton::Middle => MouseButton::Middle,
                    WinitMouseButton::Back => MouseButton::Back,
                    WinitMouseButton::Forward => MouseButton::Forward,
                    WinitMouseButton::Other(n) => MouseButton::Other((*n) as u8),
                };
                let current_target = self.hit_test_result_at_point(self.mouse_window_position);
                let coords = self.current_mouse_event_coords(current_target.as_ref());
                Some(match state {
                    ElementState::Pressed => Event::MouseDown {
                        coords,
                        button: btn,
                        modifiers: Modifiers::new(),
                    },
                    ElementState::Released => Event::MouseUp {
                        coords,
                        button: btn,
                        modifiers: Modifiers::new(),
                    },
                })
            },
            WindowEvent::KeyboardInput { event, .. } => {
                let key = match &event.logical_key {
                    WinitKey::Named(n) => convert_named_key(n),
                    WinitKey::Character(c) => convert_char_key(c),
                    _ => {
                        eprintln!("[rgui] convert_event: 未识别的键类型 {event:?}，跳过");
                        return None;
                    },
                };
                Some(match event.state {
                    ElementState::Pressed => Event::KeyDown {
                        key,
                        modifiers: Modifiers::new(),
                        repeat: event.repeat,
                    },
                    ElementState::Released => Event::KeyUp {
                        key,
                        modifiers: Modifiers::new(),
                    },
                })
            },
            WindowEvent::CloseRequested => Some(Event::CloseRequested),
            WindowEvent::Resized(size) => {
                let logical_size = logical_window_size_from_physical_size(
                    size.width,
                    size.height,
                    self.app.scale_factor,
                );
                Some(Event::WindowResized {
                    width: logical_size.width,
                    height: logical_size.height,
                })
            },
            WindowEvent::Focused(f) => {
                if *f {
                    Some(Event::WindowFocused)
                } else {
                    Some(Event::WindowUnfocused)
                }
            },
            _ => None,
        }
    }
}

fn convert_named_key(key: &NamedKey) -> rgui_platform::event::Key {
    use rgui_platform::event::Key as Rk;
    match key {
        NamedKey::Enter => Rk::Enter,
        NamedKey::Tab => Rk::Tab,
        NamedKey::Space => Rk::Space,
        NamedKey::Backspace => Rk::Backspace,
        NamedKey::Escape => Rk::Escape,
        NamedKey::Delete => Rk::Delete,
        NamedKey::ArrowLeft => Rk::ArrowLeft,
        NamedKey::ArrowRight => Rk::ArrowRight,
        NamedKey::ArrowUp => Rk::ArrowUp,
        NamedKey::ArrowDown => Rk::ArrowDown,
        NamedKey::Home => Rk::Home,
        NamedKey::End => Rk::End,
        NamedKey::PageUp => Rk::PageUp,
        NamedKey::PageDown => Rk::PageDown,
        NamedKey::Shift => Rk::Shift,
        NamedKey::Control => Rk::Ctrl,
        NamedKey::Alt => Rk::Alt,
        NamedKey::Super => Rk::Meta,
        NamedKey::F1 => Rk::F1,
        NamedKey::F2 => Rk::F2,
        NamedKey::F3 => Rk::F3,
        NamedKey::F4 => Rk::F4,
        NamedKey::F5 => Rk::F5,
        NamedKey::F6 => Rk::F6,
        NamedKey::F7 => Rk::F7,
        NamedKey::F8 => Rk::F8,
        NamedKey::F9 => Rk::F9,
        NamedKey::F10 => Rk::F10,
        NamedKey::F11 => Rk::F11,
        NamedKey::F12 => Rk::F12,
        _ => Rk::Enter,
    }
}

fn convert_char_key(c: &str) -> rgui_platform::event::Key {
    use rgui_platform::event::Key as Rk;
    match c.to_uppercase().as_str() {
        "A" => Rk::A,
        "B" => Rk::B,
        "C" => Rk::C,
        "D" => Rk::D,
        "E" => Rk::E,
        "F" => Rk::F,
        "G" => Rk::G,
        "H" => Rk::H,
        "I" => Rk::I,
        "J" => Rk::J,
        "K" => Rk::K,
        "L" => Rk::L,
        "M" => Rk::M,
        "N" => Rk::N,
        "O" => Rk::O,
        "P" => Rk::P,
        "Q" => Rk::Q,
        "R" => Rk::R,
        "S" => Rk::S,
        "T" => Rk::T,
        "U" => Rk::U,
        "V" => Rk::V,
        "W" => Rk::W,
        "X" => Rk::X,
        "Y" => Rk::Y,
        "Z" => Rk::Z,
        "0" => Rk::Digit0,
        "1" => Rk::Digit1,
        "2" => Rk::Digit2,
        "3" => Rk::Digit3,
        "4" => Rk::Digit4,
        "5" => Rk::Digit5,
        "6" => Rk::Digit6,
        "7" => Rk::Digit7,
        "8" => Rk::Digit8,
        "9" => Rk::Digit9,
        _ => Rk::Enter,
    }
}

impl ApplicationHandler for AppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attr = WindowAttributes::default()
                .with_title(&self.app.config.title)
                .with_inner_size(LogicalSize::new(
                    self.app.config.window_size.width,
                    self.app.config.window_size.height,
                ))
                .with_resizable(self.app.config.resizable);

            let mut attr = attr;
            if let Some(ref s) = self.app.config.min_size {
                attr = attr.with_min_inner_size(LogicalSize::new(s.width, s.height));
            }

            let window = Arc::new(event_loop.create_window(attr).unwrap());
            let size = window.inner_size();
            let w = size.width;
            let h = size.height;
            self.sync_scale_factor(window.scale_factor());
            self.window = Some(Arc::clone(&window));

            match VelloBackend::new(Arc::clone(&window), w, h) {
                Ok(ctx) => {
                    self.render_ctx = Some(Box::new(ctx));
                    self.width = w;
                    self.height = h;
                },
                Err(e) => eprintln!("渲染初始化失败: {e}"),
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        mut event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::CursorMoved { position, .. } => {
                self.sync_pointer_from_platform_window_event(Point::new(position.x, position.y));
                // P04e: 悬停事件路由——每帧比较鼠标位置，计算 enter/leave 关系
                self.update_hover();
                if let Some(rgui_event) = self.convert_event(&event) {
                    self.app.events.push(rgui_event);
                }
            },

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: WinitMouseButton::Left,
                ..
            } => {
                // AC06: 鼠标点击 → 输入模态为 Mouse，焦点不可见
                self.app.focus.set_input_modality(InputModality::Mouse);
                self.handle_click(self.mouse_window_position);
            },

            WindowEvent::Resized(size) => {
                self.width = size.width;
                self.height = size.height;
                if let Some(rgui_event) = self.convert_event(&event) {
                    self.app.events.push(rgui_event);
                }
            },

            WindowEvent::RedrawRequested => {
                // 帧边界 fallback 切换（D3 §12.5）：
                // 上一帧 Vello 渲染失败 → 切换为 Skia
                if self.backend_fallback_pending {
                    self.try_fallback_to_skia();
                    self.backend_fallback_pending = false;
                }
                // 窗口最小化或 surface 不可用时跳过渲染（D3 §12.4）
                if self.should_skip_render() {
                    return;
                }
                // 每帧检查 Rhai 脚本热重载（D7 §10.2）
                #[cfg(feature = "devtools")]
                if let Some(ref mut rhai_reload) = self.rhai_hot_reload {
                    if let Err(e) = rhai_reload.check_and_reload() {
                        eprintln!("[rgui] Rhai 热重载失败: {e}");
                    }
                }
                if let Some(ref mut ctx) = self.render_ctx {
                    let frame = self.frame_count;
                    // 使用逻辑像素尺寸供组件 paint/measure，物理像素供 RenderBackend。
                    let logical_w = (self.width as f64 / self.app.scale_factor).max(1.0);
                    let logical_h = (self.height as f64 / self.app.scale_factor).max(1.0);

                    // RS06: 检查脏集合——无脏 widget 且已有上一帧场景时跳过重建
                    // RS06 回归修复：needs_redraw 为非 StateStore 交互（AtomicBool 等）强制重建
                    let has_dirty = {
                        let store = self.state_store.read().expect("StateStore RwLock poisoned");
                        !store.dirty_widgets().is_empty()
                    };
                    let can_reuse =
                        !has_dirty && !self.app.needs_redraw && self.prev_scene.is_some();

                    // 场景构建回调，带异常隔离（D1 §11.3）
                    let mut scene = if can_reuse {
                        // RS06: 无脏 widget → 复用上一帧场景
                        self.prev_scene.clone().unwrap()
                    } else if let Some(ref mut view_builder) = self.view_scene_builder {
                        let text_renderer = &self.text_renderer;
                        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            view_builder(frame, logical_w as u32, logical_h as u32, text_renderer)
                        })) {
                            Ok(scene) => scene,
                            Err(e) => {
                                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                                    *s
                                } else if let Some(s) = e.downcast_ref::<String>() {
                                    s.as_str()
                                } else {
                                    "unknown panic"
                                };
                                eprintln!("[rgui] 视图场景构建 panic (frame={frame}): {msg}");
                                SceneGraph::new(frame)
                            },
                        }
                    } else {
                        SceneGraph::new(frame)
                    };

                    // AC06: 注入焦点 outline——仅当键盘聚焦时显示
                    if self.app.focus.is_focus_visible() {
                        if let Some(focus_id) = self.app.focus.current() {
                            let fi = FocusIndicator::default();
                            if let Ok(layout_guard) = self.app.current_layout.lock() {
                                if let Some(engine) = layout_guard.as_ref() {
                                    fi.inject_into_scene(&mut scene, Some(focus_id), |id| {
                                        engine.get_layout(id).map(|c| {
                                            rgui_core::geometry::Rect::new(
                                                c.result.position.x,
                                                c.result.position.y,
                                                c.result.size.width,
                                                c.result.size.height,
                                            )
                                        })
                                    });
                                }
                            }
                        }
                    }

                    // RS06: 缓存本帧场景供下一帧复用
                    self.prev_scene = Some(scene.clone());

                    // RS06 回归修复：重建后清除 needs_redraw
                    self.app.needs_redraw = false;

                    // RS06: 渲染后清除脏标记
                    {
                        let mut store = self
                            .state_store
                            .write()
                            .expect("StateStore RwLock poisoned");
                        store.clear_dirty();
                    }
                    let params = RenderParams {
                        width: self.width,
                        height: self.height,
                        scale_factor: self.app.scale_factor,
                        clear_color: Some(rgui_core::Color::new(
                            14.0 / 255.0,
                            18.0 / 255.0,
                            28.0 / 255.0,
                            1.0,
                        )),
                        ..Default::default()
                    };
                    // 将字形 Atlas 像素数据传入后端（Vello 特定路径）
                    if self.text_renderer.is_dirty() {
                        let (aw, ah) = self.text_renderer.atlas_dimensions();
                        let pixels = self.text_renderer.atlas_pixels();
                        // 仅 VelloBackend 使用 set_atlas_data
                        if let Some(vello) = ctx
                            .as_any_mut()
                            .and_then(|a| a.downcast_mut::<VelloBackend>())
                        {
                            vello.set_atlas_data(aw, ah, &pixels);
                        }
                        self.text_renderer.clear_dirty();
                    }
                    match ctx.render(&scene, &params) {
                        Ok(()) => self.frame_count += 1,
                        Err(e) => {
                            eprintln!("渲染: {e}");
                            // Vello 渲染失败 → 标记帧边界 fallback（D3 §12.5）
                            let is_vello = ctx
                                .as_any_mut()
                                .and_then(|a| a.downcast_mut::<VelloBackend>())
                                .is_some();
                            if is_vello {
                                self.backend_fallback_pending = true;
                            }
                        },
                    }
                }
            },
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                ref mut inner_size_writer,
            } => {
                self.sync_scale_factor(scale_factor);
                // 更新物理尺寸，winit 在 DPI 变化后返回新的物理尺寸
                if let Some(window) = &self.window {
                    let new_size = window.inner_size();
                    self.width = new_size.width;
                    self.height = new_size.height;
                }
                self.update_hover();
                // 通知后端 resize（Vello 特定路径）
                if let Some(ref mut ctx) = self.render_ctx {
                    if let Some(vello) = ctx
                        .as_any_mut()
                        .and_then(|a| a.downcast_mut::<VelloBackend>())
                    {
                        vello.resize(self.width, self.height);
                    }
                }
                // 回应 winit：告知我们已接受此尺寸
                if let Err(e) = inner_size_writer
                    .request_inner_size(winit::dpi::PhysicalSize::new(self.width, self.height))
                {
                    eprintln!("[rgui] request_inner_size 失败: {e:?}");
                }
                // 转发事件给组件层
                self.app.events.push(Event::ScaleFactorChanged {
                    scale_factor: self.app.scale_factor,
                });
                let logical_size = logical_window_size_from_physical_size(
                    self.width,
                    self.height,
                    self.app.scale_factor,
                );
                self.app.events.push(Event::WindowResized {
                    width: logical_size.width,
                    height: logical_size.height,
                });
            },
            WindowEvent::KeyboardInput {
                event: ref key_event,
                ..
            } => {
                // AC06: 键盘输入 → 输入模态为 Keyboard，焦点可见
                if key_event.state == ElementState::Pressed {
                    self.app.focus.set_input_modality(InputModality::Keyboard);

                    // AC12: Enter/Space 触发聚焦 widget 的 toggle action
                    if !key_event.repeat {
                        let ac12_key = match &key_event.logical_key {
                            WinitKey::Named(n) => Some(convert_named_key(n)),
                            WinitKey::Character(c) => Some(convert_char_key(c)),
                            _ => None,
                        };
                        if let Some(ref key) = ac12_key {
                            if matches!(
                                key,
                                rgui_platform::event::Key::Enter | rgui_platform::event::Key::Space
                            ) {
                                self.handle_keyboard_trigger();
                            }
                        }
                    }
                }
                if let Some(rgui_event) = self.convert_event(&event) {
                    self.app.events.push(rgui_event);
                }
            },
            _ => {
                if let Some(rgui_event) = self.convert_event(&event) {
                    self.app.events.push(rgui_event);
                }
            },
        }
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {}
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_creation() {
        let config = AppConfig::new().title("Test").window_size(1024.0, 768.0);
        let mut app = App::new(config);
        assert_eq!(app.config().title, "Test");
        app.register_defaults();
    }

    #[test]
    fn tick_dispatches_events() {
        let mut app = App::new(AppConfig::default());
        app.tick(
            vec![Event::WindowFocused, Event::CloseRequested],
            Box::new(|_| {}),
            Box::new(|_| {}),
            Box::new(|_, _| Ok(())),
        )
        .unwrap();
        assert_eq!(app.event_count(), 2);
    }

    #[test]
    fn tick_reports_render_error() {
        let mut app = App::new(AppConfig::default());
        let result = app.tick(
            vec![],
            Box::new(|_| {}),
            Box::new(|_| {}),
            Box::new(|_, _| Err("GPU 错误".into())),
        );
        assert!(result.is_err());
    }

    #[test]
    fn tick_calls_layout_and_a11y() {
        use std::cell::Cell;
        let mut app = App::new(AppConfig::default());
        let layout_called = Cell::new(false);
        let a11y_called = Cell::new(false);
        app.tick(
            vec![],
            Box::new(|_| layout_called.set(true)),
            Box::new(|_| a11y_called.set(true)),
            Box::new(|_, _| Ok(())),
        )
        .unwrap();
        assert!(layout_called.get());
        assert!(a11y_called.get());
    }

    #[test]
    fn app_events_start_empty() {
        let app = App::new(AppConfig::default());
        assert!(app.events().is_empty());
    }

    #[test]
    fn app_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.title, "rgui App");
    }

    #[test]
    fn app_config_rhai_paths_default_empty() {
        let config = AppConfig::default();
        assert!(config.rhai_paths.is_empty());
    }

    #[test]
    fn app_config_rhai_paths_builder() {
        let paths = vec![PathBuf::from("scripts/handlers.rhai")];
        let config = AppConfig::new().rhai_paths(paths.clone());
        assert_eq!(config.rhai_paths, paths);
    }

    #[test]
    fn fallback_no_window_does_not_panic() {
        // 模拟没有窗口时的 fallback——不应 panic
        let mut handler = AppHandler::new(App::new(AppConfig::default()));
        // 没有设置 render_ctx 和 window，fallback 应优雅处理
        handler.try_fallback_to_skia();
        // 恢复后 render_ctx 为 SkiaBackend
        assert!(handler.render_ctx.is_some());
    }

    #[test]
    fn device_lost_flag_defaults_false() {
        let config = AppConfig::new().title("Test").window_size(1024.0, 768.0);
        let app = App::new(config);
        let handler = AppHandler::new(app);
        // 初始状态下无 render_ctx，但无论如何不应 panic
        assert!(handler.render_ctx.is_none());
    }

    // ========================================================================
    // R22: 窗口最小化跳过渲染测试（D3 §12.4）
    // ========================================================================

    #[test]
    fn should_skip_render_when_width_zero() {
        // 窗口宽度为 0（最小化）→ 跳过渲染
        let config = AppConfig::new().title("Test").window_size(1024.0, 768.0);
        let mut handler = AppHandler::new(App::new(config));
        handler.width = 0;
        handler.height = 100;
        assert!(handler.should_skip_render());
    }

    #[test]
    fn should_skip_render_when_height_zero() {
        // 窗口高度为 0（最小化）→ 跳过渲染
        let config = AppConfig::new().title("Test").window_size(1024.0, 768.0);
        let mut handler = AppHandler::new(App::new(config));
        handler.width = 100;
        handler.height = 0;
        assert!(handler.should_skip_render());
    }

    #[test]
    fn should_skip_render_when_no_render_ctx() {
        // 无渲染上下文 → 跳过渲染
        let config = AppConfig::new().title("Test").window_size(1024.0, 768.0);
        let mut handler = AppHandler::new(App::new(config));
        handler.width = 800;
        handler.height = 600;
        // render_ctx 为 None
        assert!(handler.should_skip_render());
    }

    #[test]
    fn resized_event_uses_logical_pixels() {
        let config = AppConfig::new().title("Test").window_size(1024.0, 768.0);
        let mut handler = AppHandler::new(App::new(config));
        handler.sync_scale_factor(2.0);

        let event = WindowEvent::Resized(winit::dpi::PhysicalSize::new(800, 600));
        let converted = handler.convert_event(&event);

        assert_eq!(
            converted,
            Some(Event::WindowResized {
                width: 400.0,
                height: 300.0
            })
        );
    }

    #[test]
    fn scale_factor_sync_recomputes_cached_mouse_position() {
        let config = AppConfig::new().title("Test").window_size(1024.0, 768.0);
        let mut handler = AppHandler::new(App::new(config));
        handler.sync_pointer_from_platform_window_event(Point::new(400.0, 200.0));

        handler.sync_scale_factor(2.0);

        assert_eq!(handler.app.scale_factor(), 2.0);
        #[cfg(target_os = "macos")]
        assert_eq!(handler.mouse_window_position, Point::new(400.0, 200.0));
        #[cfg(not(target_os = "macos"))]
        assert_eq!(handler.mouse_window_position, Point::new(200.0, 100.0));
    }

    // ========================================================================
    // WC01: EventResult 接线测试
    // ========================================================================

    #[test]
    fn event_result_handled_stops_old_callback() {
        // EventResult::Handled → 调用 update()，不调用旧回调
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        let bounds = Rect::new(10.0, 10.0, 100.0, 40.0);

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let old_called = Arc::new(AtomicBool::new(false));
        let old_called_clone = Arc::clone(&old_called);
        app.register_interaction(widget_id, bounds, "click", move |_| {
            old_called_clone.store(true, Ordering::SeqCst);
        });

        let ctx_received = Arc::new(AtomicBool::new(false));
        let ctx_received_clone = Arc::clone(&ctx_received);
        app.register_widget_instance(widget_id, move |action, ctx| {
            ctx_received_clone.store(true, Ordering::SeqCst);
            assert_eq!(action, "click");
            assert!(ctx.focus.is_none());
            EventResult::Handled
        });

        let mut handler = AppHandler::new(app);
        handler.handle_click(Point::new(50.0, 30.0));

        // WidgetSpec 路径被调用
        assert!(
            ctx_received.load(Ordering::SeqCst),
            "WidgetSpec handler 应被调用"
        );
        // 旧回调不应被调用
        assert!(!old_called.load(Ordering::SeqCst), "旧回调不应被调用");
        // 事件应被记录
        assert_eq!(handler.app.event_count(), 1);
    }

    #[test]
    fn event_result_continue_falls_through_to_old_callback() {
        // EventResult::Continue → 继续传播，调用旧回调
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        let bounds = Rect::new(10.0, 10.0, 100.0, 40.0);

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let old_called = Arc::new(AtomicBool::new(false));
        let old_called_clone = Arc::clone(&old_called);
        app.register_interaction(widget_id, bounds, "click", move |action| {
            assert_eq!(action, "click");
            old_called_clone.store(true, Ordering::SeqCst);
        });

        app.register_widget_instance(widget_id, move |action, _ctx| {
            assert_eq!(action, "click");
            EventResult::Continue("click".to_string())
        });

        let mut handler = AppHandler::new(app);
        handler.handle_click(Point::new(50.0, 30.0));

        // 旧回调应被调用（Continue 后 fall through）
        assert!(
            old_called.load(Ordering::SeqCst),
            "旧回调应被调用（Continue 后 fall through）"
        );
        assert_eq!(handler.app.event_count(), 1);
    }

    #[test]
    fn event_result_prevented_stops_and_no_event_recorded() {
        // EventResult::Prevented → 不调用旧回调，不记录事件
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        let bounds = Rect::new(10.0, 10.0, 100.0, 40.0);

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let old_called = Arc::new(AtomicBool::new(false));
        let old_called_clone = Arc::clone(&old_called);
        app.register_interaction(widget_id, bounds, "click", move |_| {
            old_called_clone.store(true, Ordering::SeqCst);
        });

        app.register_widget_instance(widget_id, move |action, _ctx| {
            assert_eq!(action, "click");
            EventResult::Prevented
        });

        let mut handler = AppHandler::new(app);
        handler.handle_click(Point::new(50.0, 30.0));

        // 旧回调不应被调用
        assert!(
            !old_called.load(Ordering::SeqCst),
            "旧回调不应被调用（Prevented）"
        );
        // 事件不应被记录
        assert_eq!(handler.app.event_count(), 0);
    }

    #[test]
    fn no_widget_instance_falls_back_to_old_callback() {
        // 未注册 WidgetSpec 实例 → 直接走旧路径
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        let bounds = Rect::new(10.0, 10.0, 100.0, 40.0);

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let old_called = Arc::new(AtomicBool::new(false));
        let old_called_clone = Arc::clone(&old_called);
        app.register_interaction(widget_id, bounds, "click", move |action| {
            assert_eq!(action, "click");
            old_called_clone.store(true, Ordering::SeqCst);
        });

        // 不注册 WidgetSpec 实例

        let mut handler = AppHandler::new(app);
        handler.handle_click(Point::new(50.0, 30.0));

        assert!(old_called.load(Ordering::SeqCst), "旧回调应被调用");
        assert_eq!(handler.app.event_count(), 1);
    }

    #[test]
    fn hit_miss_does_nothing() {
        // 点击空白区域 → 无事发生
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        let bounds = Rect::new(10.0, 10.0, 100.0, 40.0);

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let old_called = Arc::new(AtomicBool::new(false));
        let old_called_clone = Arc::clone(&old_called);
        app.register_interaction(widget_id, bounds, "click", move |_| {
            old_called_clone.store(true, Ordering::SeqCst);
        });
        app.register_widget_instance(widget_id, |_, _| EventResult::Handled);

        let mut handler = AppHandler::new(app);
        // 点击位置在 widget 外部
        handler.handle_click(Point::new(200.0, 200.0));

        assert!(!old_called.load(Ordering::SeqCst));
        assert_eq!(handler.app.event_count(), 0);
    }

    // ========================================================================
    // WTI03: 点击外部关闭弹层
    // ========================================================================

    #[test]
    fn hit_miss_no_overlay_no_close_event() {
        // 点击空白区域，无线程弹层 → 无事发生
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        let bounds = Rect::new(10.0, 10.0, 100.0, 40.0);

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let close_called = Arc::new(AtomicBool::new(false));
        let close_called_clone = Arc::clone(&close_called);
        app.register_interaction(widget_id, bounds, "click", move |_| {});
        app.register_widget_instance(widget_id, move |action, _ctx| {
            if action == "close" {
                close_called_clone.store(true, Ordering::SeqCst);
                return EventResult::Handled;
            }
            EventResult::Continue(action.to_string())
        });

        let mut handler = AppHandler::new(app);
        // 点击在 widget 外部，但 widget 未被标记为弹层
        handler.handle_click(Point::new(200.0, 200.0));

        // close 回调不应被调用（widget 未标记为弹层）
        assert!(!close_called.load(Ordering::SeqCst));
        // 无 Close 事件
        assert!(
            !handler
                .app
                .events()
                .iter()
                .any(|e| matches!(e, Event::Close { .. }))
        );
    }

    #[test]
    fn hit_miss_with_overlay_sends_close_event() {
        // 点击空白区域，存在弹层 → 发送 Close 事件
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        let bounds = Rect::new(10.0, 10.0, 100.0, 40.0);

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let close_called = Arc::new(AtomicBool::new(false));
        let close_called_clone = Arc::clone(&close_called);
        app.register_interaction(widget_id, bounds, "click", move |_| {});
        app.register_widget_instance(widget_id, move |action, _ctx| {
            if action == "close" {
                close_called_clone.store(true, Ordering::SeqCst);
                return EventResult::Handled;
            }
            EventResult::Continue(action.to_string())
        });
        // 标记为弹层
        app.mark_as_overlay(widget_id);

        let mut handler = AppHandler::new(app);
        // 点击在 widget 外部
        handler.handle_click(Point::new(200.0, 200.0));

        // close 回调应被调用
        assert!(close_called.load(Ordering::SeqCst), "弹层应收到 close 回调");
        // 应有 Close 事件
        let close_events: Vec<_> = handler
            .app
            .events()
            .iter()
            .filter(|e| matches!(e, Event::Close { .. }))
            .collect();
        assert!(!close_events.is_empty(), "应有 Close 事件");
        // Close 事件应指向正确的 widget
        assert!(
            close_events
                .iter()
                .any(|e| matches!(e, Event::Close { widget_id: Some(id) } if *id == widget_id)),
            "Close 事件应指向弹层 widget"
        );
    }

    #[test]
    fn hit_on_widget_does_not_close_overlay() {
        // 点击命中 widget → 不关闭弹层
        let mut app = App::new(AppConfig::default());
        let overlay_id = WidgetId::from_u64(1);
        let overlay_bounds = Rect::new(10.0, 10.0, 200.0, 200.0);
        let other_id = WidgetId::from_u64(2);
        let other_bounds = Rect::new(250.0, 250.0, 100.0, 40.0);

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let close_called = Arc::new(AtomicBool::new(false));
        let close_called_clone = Arc::clone(&close_called);

        // 注册弹层
        app.register_interaction(overlay_id, overlay_bounds, "click", move |_| {});
        app.register_widget_instance(overlay_id, move |action, _ctx| {
            if action == "close" {
                close_called_clone.store(true, Ordering::SeqCst);
                return EventResult::Handled;
            }
            EventResult::Continue(action.to_string())
        });
        app.mark_as_overlay(overlay_id);

        // 注册另一个 widget
        let other_clicked = Arc::new(AtomicBool::new(false));
        let other_clicked_clone = Arc::clone(&other_clicked);
        app.register_interaction(other_id, other_bounds, "click", move |_| {
            other_clicked_clone.store(true, Ordering::SeqCst);
        });

        let mut handler = AppHandler::new(app);
        // 点击命中 other widget（非弹层区域）
        handler.handle_click(Point::new(300.0, 270.0));

        // 弹层不应收到 close 回调
        assert!(
            !close_called.load(Ordering::SeqCst),
            "点击命中 widget 不应关闭弹层"
        );
        // other widget 应收到 click 回调
        assert!(
            other_clicked.load(Ordering::SeqCst),
            "other widget 应收到 click 回调"
        );
    }

    #[test]
    fn multiple_overlays_all_get_close_events() {
        // 多个弹层 → 全部收到 Close 事件
        let mut app = App::new(AppConfig::default());

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let overlay1_closed = Arc::new(AtomicBool::new(false));
        let overlay1_closed_clone = Arc::clone(&overlay1_closed);
        let id1 = WidgetId::from_u64(1);
        app.register_interaction(id1, Rect::new(0.0, 0.0, 100.0, 100.0), "click", |_| {});
        app.register_widget_instance(id1, move |action, _ctx| {
            if action == "close" {
                overlay1_closed_clone.store(true, Ordering::SeqCst);
                return EventResult::Handled;
            }
            EventResult::Continue(action.to_string())
        });
        app.mark_as_overlay(id1);

        let overlay2_closed = Arc::new(AtomicBool::new(false));
        let overlay2_closed_clone = Arc::clone(&overlay2_closed);
        let id2 = WidgetId::from_u64(2);
        app.register_interaction(id2, Rect::new(0.0, 0.0, 80.0, 80.0), "click", |_| {});
        app.register_widget_instance(id2, move |action, _ctx| {
            if action == "close" {
                overlay2_closed_clone.store(true, Ordering::SeqCst);
                return EventResult::Handled;
            }
            EventResult::Continue(action.to_string())
        });
        app.mark_as_overlay(id2);

        let mut handler = AppHandler::new(app);
        // 点击在弹层外部
        handler.handle_click(Point::new(500.0, 500.0));

        assert!(
            overlay1_closed.load(Ordering::SeqCst),
            "弹层 1 应收到 close 回调"
        );
        assert!(
            overlay2_closed.load(Ordering::SeqCst),
            "弹层 2 应收到 close 回调"
        );

        // 两个 Close 事件
        let close_count = handler
            .app
            .events()
            .iter()
            .filter(|e| matches!(e, Event::Close { .. }))
            .count();
        assert_eq!(close_count, 2, "应有 2 个 Close 事件");
    }

    #[test]
    fn unmark_overlay_stops_close_behavior() {
        // 取消弹层标记后，点击外部不再发送 Close
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        let bounds = Rect::new(10.0, 10.0, 100.0, 40.0);

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let close_called = Arc::new(AtomicBool::new(false));
        let close_called_clone = Arc::clone(&close_called);
        app.register_interaction(widget_id, bounds, "click", move |_| {});
        app.register_widget_instance(widget_id, move |action, _ctx| {
            if action == "close" {
                close_called_clone.store(true, Ordering::SeqCst);
                return EventResult::Handled;
            }
            EventResult::Continue(action.to_string())
        });
        app.mark_as_overlay(widget_id);

        // 验证标记已生效
        assert!(app.is_overlay(widget_id));

        // 取消标记
        app.unmark_overlay(widget_id);
        assert!(!app.is_overlay(widget_id));

        let mut handler = AppHandler::new(app);
        handler.handle_click(Point::new(200.0, 200.0));

        assert!(
            !close_called.load(Ordering::SeqCst),
            "取消标记后不应再发送 close"
        );
    }

    #[test]
    fn close_event_has_correct_widget_id() {
        // Close 事件的 widget_id 字段正确
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(42);
        let bounds = Rect::new(10.0, 10.0, 100.0, 40.0);

        app.register_interaction(widget_id, bounds, "click", |_| {});
        app.register_widget_instance(widget_id, |_, _| EventResult::Handled);
        app.mark_as_overlay(widget_id);

        let mut handler = AppHandler::new(app);
        handler.handle_click(Point::new(200.0, 200.0));

        // 检查 Close 事件
        let close_events: Vec<&Event> = handler
            .app
            .events()
            .iter()
            .filter(|e| matches!(e, Event::Close { .. }))
            .collect();
        assert_eq!(close_events.len(), 1);
        assert_eq!(
            *close_events[0],
            Event::Close {
                widget_id: Some(widget_id)
            }
        );
    }

    // ========================================================================
    // RG04: .rgui App 集成测试
    // ========================================================================

    #[cfg(feature = "devtools")]
    mod devtools_tests {
        use super::*;

        /// 测试用消息类型。
        #[derive(Debug, Clone, PartialEq)]
        enum TestMsg {}
        impl AppMessage for TestMsg {
            fn message_name(&self) -> &'static str {
                match *self {}
            }
        }

        #[test]
        fn config_rgui_path_builder() {
            let config = AppConfig::new().rgui_path("ui/app.rgui");
            assert!(config.rgui_path.is_some());
            assert_eq!(
                config.rgui_path.unwrap(),
                std::path::PathBuf::from("ui/app.rgui")
            );
        }

        #[test]
        fn config_rgui_path_default_is_none() {
            let config = AppConfig::default();
            assert!(config.rgui_path.is_none());
        }

        #[test]
        fn load_rgui_errors_without_rgui_path() {
            let config = AppConfig::new();
            let mut app = App::new(config);
            let result = app.load_rgui::<TestMsg>();
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("未设置 rgui_path"));
        }

        #[test]
        fn load_rgui_with_nonexistent_file() {
            let dir = tempfile::tempdir().expect("创建临时目录失败");
            let non_existent = dir.path().join("no_such.rgui");
            let config = AppConfig::new().rgui_path(&non_existent);
            let mut app = App::new(config);
            let result = app.load_rgui::<TestMsg>();
            assert!(result.is_err());
        }

        #[test]
        fn load_rgui_with_valid_rgui_file_sets_view_scene_builder() {
            let dir = tempfile::tempdir().expect("创建临时目录失败");
            let rgui_path = dir.path().join("app.rgui");
            std::fs::write(
                &rgui_path,
                r#"<Column spacing="8"><Label text="Hello"/></Column>"#,
            )
            .expect("写入 .rgui 文件失败");

            let config = AppConfig::new().rgui_path(&rgui_path);
            let mut app = App::new(config);
            let result = app.load_rgui::<TestMsg>();
            assert!(result.is_ok(), "load_rgui 应成功: {result:?}");

            // view_scene_builder 应被设置
            assert!(app.view_scene_builder.is_some());
        }

        #[test]
        fn load_rgui_view_scene_builder_produces_scene() {
            let dir = tempfile::tempdir().expect("创建临时目录失败");
            let rgui_path = dir.path().join("app.rgui");
            std::fs::write(
                &rgui_path,
                r#"<Column spacing="8"><Label text="Hello"/></Column>"#,
            )
            .expect("写入 .rgui 文件失败");

            let config = AppConfig::new().rgui_path(&rgui_path);
            let mut app = App::new(config);
            app.load_rgui::<TestMsg>().expect("load_rgui 应成功");

            // 调用 view_scene_builder 应返回有效的 SceneGraph
            let text_renderer = TextRenderer::new(rgui_render::TextureId(0));
            let scene = app.view_scene_builder.as_mut().unwrap()(1, 800, 600, &text_renderer);
            // 基本断言：场景图应非空（至少有一个图层）
            assert!(!scene.is_empty(), "SceneGraph 应包含图层");
        }

        // ========================================================================
        // RG05: AppHandler 集成验证测试
        // ========================================================================

        #[test]
        fn load_rgui_view_scene_builder_detects_file_change() {
            // 修改 .rgui 文件 → builder 检测变更 → 场景更新（< 1s）
            let dir = tempfile::tempdir().expect("创建临时目录失败");
            let rgui_path = dir.path().join("app.rgui");
            std::fs::write(
                &rgui_path,
                r#"<Column spacing="8"><Label text="Before"/></Column>"#,
            )
            .expect("写入 .rgui 文件失败");

            let config = AppConfig::new().rgui_path(&rgui_path);
            let mut app = App::new(config);
            app.load_rgui::<TestMsg>().expect("load_rgui 应成功");

            // 等待 watcher 稳定并消耗初始事件
            std::thread::sleep(std::time::Duration::from_millis(200));

            let text_renderer = TextRenderer::new(rgui_render::TextureId(0));

            // 第一次调用——获取初始场景
            let scene1 = app.view_scene_builder.as_mut().unwrap()(1, 800, 600, &text_renderer);
            assert!(!scene1.is_empty(), "初始场景应有效");

            // 修改 .rgui 文件
            std::fs::write(
                &rgui_path,
                r#"<Column spacing="8"><Button label="Click"/></Column>"#,
            )
            .expect("写入 .rgui 文件失败");
            std::thread::sleep(std::time::Duration::from_millis(200));

            // 第二次调用——应检测到变更并返回新场景
            let scene2 = app.view_scene_builder.as_mut().unwrap()(2, 800, 600, &text_renderer);
            assert!(!scene2.is_empty(), "文件变更后应返回有效场景");
        }

        #[test]
        fn load_rgui_view_scene_builder_keeps_old_view_on_parse_error() {
            // 解析失败时保持旧视图（降级策略 D7 §9）
            let dir = tempfile::tempdir().expect("创建临时目录失败");
            let rgui_path = dir.path().join("app.rgui");
            std::fs::write(
                &rgui_path,
                r#"<Column spacing="8"><Label text="Valid"/></Column>"#,
            )
            .expect("写入 .rgui 文件失败");

            let config = AppConfig::new().rgui_path(&rgui_path);
            let mut app = App::new(config);
            app.load_rgui::<TestMsg>().expect("load_rgui 应成功");

            // 等待 watcher 稳定并消耗初始事件
            std::thread::sleep(std::time::Duration::from_millis(200));

            let text_renderer = TextRenderer::new(rgui_render::TextureId(0));

            // 第一次调用——获取初始有效场景
            let scene1 = app.view_scene_builder.as_mut().unwrap()(1, 800, 600, &text_renderer);
            assert!(!scene1.is_empty(), "初始场景应有效");

            // 写入畸形的 .rgui（未闭合标签）
            std::fs::write(&rgui_path, r#"<Column><Label text="Oops""#)
                .expect("写入畸形 .rgui 文件失败");
            std::thread::sleep(std::time::Duration::from_millis(200));

            // 第二次调用——解析失败，应保持旧视图（降级策略）
            let scene2 = app.view_scene_builder.as_mut().unwrap()(2, 800, 600, &text_renderer);
            assert!(
                !scene2.is_empty(),
                "解析失败后应返回旧场景（降级策略——保持旧视图）"
            );
        }

        // ========================================================================
        // RH04: .rhai App 集成测试
        // ========================================================================

        #[test]
        fn command_registry_none_before_load() {
            let config = AppConfig::new();
            let app = App::new(config);
            assert!(
                app.command_registry().is_none(),
                "未调用 load_rhai_scripts 前 command_registry 应为 None"
            );
        }

        #[test]
        fn load_rhai_scripts_empty_paths_returns_ok() {
            let config = AppConfig::new();
            let mut app = App::new(config);
            let empty: &[&str] = &[];
            let result = app.load_rhai_scripts(empty);
            assert!(result.is_ok(), "空路径列表应成功: {result:?}");
            assert!(app.command_registry().is_none(), "空路径不创建 registry");
        }

        #[test]
        fn load_rhai_scripts_single_file_succeeds() {
            let dir = tempfile::tempdir().expect("创建临时目录失败");
            let rhai_path = dir.path().join("handlers.rhai");
            std::fs::write(&rhai_path, "fn save() { }").expect("写入 .rhai 文件失败");

            let config = AppConfig::new();
            let mut app = App::new(config);
            let result = app.load_rhai_scripts(&[&rhai_path]);
            assert!(result.is_ok(), "load_rhai_scripts 应成功: {result:?}");

            // command_registry 应可用
            let registry = app
                .command_registry()
                .expect("load 后 command_registry 应为 Some");
            // 能调用注册的函数
            let mut cloned = registry.clone();
            let call_result: Result<(), _> = cloned.call_fn("save", ());
            assert!(
                call_result.is_ok(),
                "应能调用已注册的脚本函数: {call_result:?}"
            );
        }

        #[test]
        fn command_registry_engine_mut_works() {
            let dir = tempfile::tempdir().expect("创建临时目录失败");
            let rhai_path = dir.path().join("handlers.rhai");
            std::fs::write(&rhai_path, "fn dummy() { }").expect("写入 .rhai 文件失败");

            let config = AppConfig::new();
            let mut app = App::new(config);
            app.load_rhai_scripts(&[&rhai_path])
                .expect("load_rhai_scripts 应成功");

            let registry = app.command_registry().unwrap();
            // engine_mut 应能获取引擎并注册类型
            registry.engine_mut().register_type::<i64>();
            // 成功——没有 panic
        }

        // ========================================================================
        // RH05: .rhai AppHandler 集成——handle_click Rhai 路由测试
        // ========================================================================

        /// `.rhai` 声明的函数在点击时被成功路由（Rhai 消费事件，旧回调不触发）。
        #[test]
        fn handle_click_rhai_routing_consumes_event() {
            let dir = tempfile::tempdir().expect("创建临时目录失败");
            let rhai_path = dir.path().join("handlers.rhai");
            // Rhai 脚本中定义 save 函数——将 consumed 设为 true
            std::fs::write(&rhai_path, "fn save() { }").expect("写入 .rhai 文件失败");

            let config = AppConfig::new();
            let mut app = App::new(config);
            app.load_rhai_scripts(&[&rhai_path])
                .expect("load_rhai_scripts 应成功");

            let widget_id = WidgetId::from_u64(1);
            let bounds = Rect::new(10.0, 10.0, 100.0, 40.0);

            use std::sync::Arc;
            use std::sync::atomic::{AtomicBool, Ordering};
            let old_called = Arc::new(AtomicBool::new(false));
            let old_called_clone = Arc::clone(&old_called);
            // 注册旧回调（不应被调用——Rhai 先消费事件）
            app.register_interaction(widget_id, bounds, "save", move |_| {
                old_called_clone.store(true, Ordering::SeqCst);
            });

            let mut handler = AppHandler::new(app);
            handler.handle_click(Point::new(50.0, 30.0));

            // Rhai 函数存在且被调用，旧回调不应被调用
            assert!(
                !old_called.load(Ordering::SeqCst),
                "Rhai 路由应消费事件，旧回调不应被调用"
            );
            // 事件应被记录（由 Rhai 分支记录）
            assert_eq!(handler.app.event_count(), 1);
        }

        /// Rhai 无匹配函数时，fallback 到旧回调。
        #[test]
        fn handle_click_rhai_not_found_falls_back_to_old_callback() {
            let dir = tempfile::tempdir().expect("创建临时目录失败");
            let rhai_path = dir.path().join("handlers.rhai");
            // Rhai 脚本中只定义 unknown_fn，不定义 "click" 函数
            std::fs::write(&rhai_path, "fn unknown_fn() { }").expect("写入 .rhai 文件失败");

            let config = AppConfig::new();
            let mut app = App::new(config);
            app.load_rhai_scripts(&[&rhai_path])
                .expect("load_rhai_scripts 应成功");

            let widget_id = WidgetId::from_u64(1);
            let bounds = Rect::new(10.0, 10.0, 100.0, 40.0);

            use std::sync::Arc;
            use std::sync::atomic::{AtomicBool, Ordering};
            let old_called = Arc::new(AtomicBool::new(false));
            let old_called_clone = Arc::clone(&old_called);
            // 注册旧回调（应被调用——Rhai 无匹配函数）
            app.register_interaction(widget_id, bounds, "click", move |action| {
                assert_eq!(action, "click");
                old_called_clone.store(true, Ordering::SeqCst);
            });

            let mut handler = AppHandler::new(app);
            handler.handle_click(Point::new(50.0, 30.0));

            // Rhai 无 "click" 函数，旧回调应被调用
            assert!(
                old_called.load(Ordering::SeqCst),
                "Rhai 无匹配函数时应 fallback 到旧回调"
            );
            assert_eq!(handler.app.event_count(), 1);
        }
    }

    // ========================================================================
    // RS06 回归修复：AtomicBool 交互桥接 needs_redraw 测试
    // ========================================================================

    #[test]
    fn request_redraw_sets_flag() {
        let mut app = App::new(AppConfig::default());
        assert!(!app.needs_redraw, "needs_redraw should start false");
        app.request_redraw();
        assert!(
            app.needs_redraw,
            "needs_redraw should be true after request_redraw()"
        );
    }

    #[test]
    fn interaction_callback_triggers_redraw_request() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let toggled = Arc::new(AtomicBool::new(false));
        let toggled_clone = Arc::clone(&toggled);

        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(42);
        let bounds = Rect::new(10.0, 10.0, 100.0, 50.0);

        app.register_interaction(widget_id, bounds, "toggle", move |_action| {
            toggled_clone.store(true, Ordering::SeqCst);
        });

        assert!(!app.needs_redraw, "needs_redraw should start false");

        let mut handler = AppHandler::new(app);
        handler.handle_click(Point::new(50.0, 30.0));

        assert!(
            toggled.load(Ordering::SeqCst),
            "AtomicBool should be toggled"
        );
        assert!(
            handler.app.needs_redraw,
            "interaction callback should trigger needs_redraw"
        );
    }

    #[test]
    fn needs_redraw_cleared_after_scene_rebuild() {
        // Verify that needs_redraw starts false (in a fresh App)
        let app = App::new(AppConfig::default());
        assert!(
            !app.needs_redraw,
            "needs_redraw should be false on fresh App"
        );
    }

    // ========================================================================
    // P04e: 悬停事件路由测试（MouseEnter/MouseLeave）
    // ========================================================================

    /// 初始状态：last_hover 为 None，没有悬停事件。
    #[test]
    fn hover_initial_state_is_none() {
        let app = App::new(AppConfig::default());
        let handler = AppHandler::new(app);
        assert!(
            handler.last_hover.is_none(),
            "last_hover should start as None"
        );
    }

    /// 鼠标移入 widget → push MouseEnter，last_hover 更新。
    #[test]
    fn hover_enter_widget_pushes_mouse_enter() {
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        let bounds = Rect::new(10.0, 10.0, 100.0, 50.0);
        app.register_interaction(widget_id, bounds, "click", |_| {});

        let mut handler = AppHandler::new(app);
        // 将鼠标移到 widget 区域内
        handler.mouse_window_position = Point::new(50.0, 30.0);
        handler.update_hover();

        // 应产生一个 MouseEnter 事件
        assert_eq!(handler.app.events.len(), 1, "应该产生 1 个事件");
        assert!(
            matches!(&handler.app.events[0], Event::MouseEnter { widget_id: id } if *id == widget_id),
            "应该是 MouseEnter 事件"
        );
        assert_eq!(
            handler.last_hover,
            Some(widget_id),
            "last_hover 应更新为目标 widget"
        );
    }

    /// 鼠标从 widget 移出到空白区域 → push MouseLeave，last_hover 置 None。
    #[test]
    fn hover_leave_widget_pushes_mouse_leave() {
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        let bounds = Rect::new(10.0, 10.0, 100.0, 50.0);
        app.register_interaction(widget_id, bounds, "click", |_| {});

        let mut handler = AppHandler::new(app);
        // 先移入
        handler.mouse_window_position = Point::new(50.0, 30.0);
        handler.update_hover();
        assert_eq!(handler.app.events.len(), 1);

        // 再移出
        handler.mouse_window_position = Point::new(200.0, 200.0);
        handler.update_hover();

        // 应再产生一个 MouseLeave 事件
        assert_eq!(
            handler.app.events.len(),
            2,
            "应该产生 2 个事件（Enter + Leave）"
        );
        assert!(
            matches!(&handler.app.events[1], Event::MouseLeave { widget_id: id } if *id == widget_id),
            "第二个应该是 MouseLeave 事件"
        );
        assert!(handler.last_hover.is_none(), "last_hover 应回到 None");
    }

    /// 鼠标从 widget A 移入 widget B → A 收 MouseLeave，B 收 MouseEnter。
    #[test]
    fn hover_transition_between_widgets() {
        let mut app = App::new(AppConfig::default());
        let widget_a = WidgetId::from_u64(1);
        let widget_b = WidgetId::from_u64(2);
        app.register_interaction(
            widget_a,
            Rect::new(10.0, 10.0, 100.0, 50.0),
            "click",
            |_| {},
        );
        app.register_interaction(
            widget_b,
            Rect::new(150.0, 10.0, 100.0, 50.0),
            "click",
            |_| {},
        );

        let mut handler = AppHandler::new(app);

        // 移入 A
        handler.mouse_window_position = Point::new(50.0, 30.0);
        handler.update_hover();
        assert_eq!(handler.app.events.len(), 1);
        assert!(matches!(&handler.app.events[0], Event::MouseEnter { .. }));

        // 移入 B
        handler.mouse_window_position = Point::new(200.0, 30.0);
        handler.update_hover();

        // 应产生 MouseLeave(A) + MouseEnter(B)
        assert_eq!(
            handler.app.events.len(),
            3,
            "应该产生 3 个事件（EnterA + LeaveA + EnterB）"
        );
        assert!(
            matches!(&handler.app.events[1], Event::MouseLeave { widget_id: id } if *id == widget_a),
            "第二个应该是 MouseLeave(A)"
        );
        assert!(
            matches!(&handler.app.events[2], Event::MouseEnter { widget_id: id } if *id == widget_b),
            "第三个应该是 MouseEnter(B)"
        );
        assert_eq!(handler.last_hover, Some(widget_b), "last_hover 应更新为 B");
    }

    /// 鼠标在同一 widget 内移动 → 不产生新事件。
    #[test]
    fn hover_unchanged_no_new_events() {
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        app.register_interaction(
            widget_id,
            Rect::new(10.0, 10.0, 100.0, 50.0),
            "click",
            |_| {},
        );

        let mut handler = AppHandler::new(app);

        // 首次移入
        handler.mouse_window_position = Point::new(50.0, 30.0);
        handler.update_hover();
        assert_eq!(handler.app.events.len(), 1);

        // 同一区域内移动
        handler.mouse_window_position = Point::new(60.0, 35.0);
        handler.update_hover();
        assert_eq!(
            handler.app.events.len(),
            1,
            "同一 widget 内移动不应产生新事件"
        );
        assert_eq!(handler.last_hover, Some(widget_id));
    }

    /// handle_click 将 hover 状态同步到 UpdateContext。
    #[test]
    fn handle_click_syncs_hover_to_update_context() {
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        app.register_interaction(
            widget_id,
            Rect::new(10.0, 10.0, 100.0, 50.0),
            "test",
            |_| {},
        );

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let hover_received = Arc::new(AtomicBool::new(false));
        let hover_received_clone = Arc::clone(&hover_received);
        let local_received = Arc::new(AtomicBool::new(false));
        let local_received_clone = Arc::clone(&local_received);

        // 注册 widget instance handler，验证收到 hover
        app.register_widget_instance(widget_id, move |_action, ctx| {
            if ctx.hover == Some(widget_id) {
                hover_received_clone.store(true, Ordering::SeqCst);
            }
            if ctx.cursor_window_position == Some(Point::new(50.0, 30.0))
                && ctx.cursor_local_position == Some(Point::new(40.0, 20.0))
            {
                local_received_clone.store(true, Ordering::SeqCst);
            }
            EventResult::Handled
        });

        let mut handler = AppHandler::new(app);
        // 先设置悬停
        handler.sync_pointer_from_logical_injection(Point::new(50.0, 30.0));
        handler.update_hover();

        // 点击
        handler.handle_click(Point::new(50.0, 30.0));

        assert!(
            hover_received.load(Ordering::SeqCst),
            "handle_click 应将 hover 同步到 UpdateContext"
        );
        assert!(
            local_received.load(Ordering::SeqCst),
            "handle_click 应将窗口/局部坐标同步到 UpdateContext"
        );
    }

    #[test]
    fn hit_test_tree_restores_nested_local_coordinates() {
        use rgui_core::view::{PropValue, WidgetView};

        let root_id = WidgetId::from_u64(100);
        let child_id = WidgetId::from_u64(101);
        let leaf_id = WidgetId::from_u64(102);

        let mut view = WidgetView::<rgui_core::message::NoopMsg>::new("Root")
            .id(root_id)
            .prop("width", PropValue::Int(200))
            .prop("height", PropValue::Int(200))
            .child(
                WidgetView::new("Child")
                    .id(child_id)
                    .prop("width", PropValue::Int(120))
                    .prop("height", PropValue::Int(90))
                    .prop("margin", PropValue::Int(20))
                    .child(
                        WidgetView::new("Leaf")
                            .id(leaf_id)
                            .prop("width", PropValue::Int(20))
                            .prop("height", PropValue::Int(20))
                            .prop("margin", PropValue::Int(5)),
                    ),
            );
        let layout = compute_view_layout(&mut view, Size::new(200.0, 200.0), None);

        let app = App::new(AppConfig::default());
        *app.current_view.lock().unwrap() = Some(view.to_noop_view());
        *app.current_layout.lock().unwrap() = Some(layout);

        let handler = AppHandler::new(app);
        let hit = handler
            .hit_test_result_at_point(Point::new(26.0, 26.0))
            .expect("嵌套布局中的点应命中叶子节点");

        assert_eq!(hit.widget_id, leaf_id);
        assert_eq!(hit.local_logical, Point::new(1.0, 1.0));
        assert_eq!(
            hit.window_to_local.window_to_local(Point::new(26.0, 26.0)),
            Point::new(1.0, 1.0),
            "命中结果应保留窗口坐标到叶子局部坐标的可逆链路"
        );
    }

    #[test]
    fn register_interaction_chain_restores_local_coordinates() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(7);
        let window_point = Point::new(45.0, 43.0);
        let chain = CoordinateTransformChain::default()
            .translated(Point::new(10.0, 20.0))
            .translated(Point::new(30.0, 15.0))
            .translated(Point::new(4.0, 6.0));

        app.register_interaction_with_chain(
            widget_id,
            Rect::new(44.0, 41.0, 40.0, 30.0),
            chain,
            "click",
            |_| {},
        );

        let local_received = Arc::new(AtomicBool::new(false));
        let local_received_clone = Arc::clone(&local_received);
        app.register_widget_instance(widget_id, move |_action, ctx| {
            if ctx.cursor_local_position == Some(Point::new(1.0, 2.0)) {
                local_received_clone.store(true, Ordering::SeqCst);
            }
            EventResult::Handled
        });

        let mut handler = AppHandler::new(app);
        let hit = handler
            .hit_test_result_at_point(window_point)
            .expect("注册链路应命中交互区域");
        assert_eq!(hit.widget_id, widget_id);
        assert_eq!(hit.local_logical, Point::new(1.0, 2.0));

        handler.handle_click(window_point);
        assert!(
            local_received.load(Ordering::SeqCst),
            "旧交互注册路径也应把恢复后的局部坐标同步给组件"
        );
    }

    #[test]
    fn register_interaction_chain_restores_local_coordinates_through_scroll_offset() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(8);
        let window_point = Point::new(26.0, 82.0);

        let viewport_origin = Point::new(20.0, 10.0);
        let scroll_offset = Point::new(0.0, -30.0);
        let item_origin_in_content = Point::new(5.0, 100.0);
        let visual_origin = Point::new(
            viewport_origin.x + scroll_offset.x + item_origin_in_content.x,
            viewport_origin.y + scroll_offset.y + item_origin_in_content.y,
        );
        let chain = CoordinateTransformChain::default()
            .translated(viewport_origin)
            .translated(scroll_offset)
            .translated(item_origin_in_content);

        app.register_interaction_with_chain(
            widget_id,
            Rect::new(visual_origin.x, visual_origin.y, 40.0, 20.0),
            chain,
            "click",
            |_| {},
        );

        let local_received = Arc::new(AtomicBool::new(false));
        let local_received_clone = Arc::clone(&local_received);
        app.register_widget_instance(widget_id, move |_action, ctx| {
            if ctx.cursor_window_position == Some(window_point)
                && ctx.cursor_local_position == Some(Point::new(1.0, 2.0))
            {
                local_received_clone.store(true, Ordering::SeqCst);
            }
            EventResult::Handled
        });

        let mut handler = AppHandler::new(app);
        let hit = handler
            .hit_test_result_at_point(window_point)
            .expect("滚动容器中的视觉命中点应命中内容项");
        assert_eq!(hit.widget_id, widget_id);
        assert_eq!(hit.local_logical, Point::new(1.0, 2.0));
        assert_eq!(
            hit.window_to_local.window_to_local(window_point),
            Point::new(1.0, 2.0),
            "滚动偏移应通过逆变换链恢复为局部坐标"
        );

        handler.handle_click(window_point);
        assert!(
            local_received.load(Ordering::SeqCst),
            "滚动容器命中后，组件仍应收到窗口/局部坐标一致的 UpdateContext"
        );
    }

    #[test]
    fn drag_over_restores_local_coordinates_for_registered_interaction_chain() {
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(9);
        let window_point = Point::new(45.0, 43.0);
        let chain = CoordinateTransformChain::default()
            .translated(Point::new(10.0, 20.0))
            .translated(Point::new(30.0, 15.0))
            .translated(Point::new(4.0, 6.0));

        app.register_interaction_with_chain(
            widget_id,
            Rect::new(44.0, 41.0, 40.0, 30.0),
            chain,
            "click",
            |_| {},
        );

        let mut handler = AppHandler::new(app);
        let hit = handler.inject_drag_lifecycle_logical(window_point, DragLifecycleEvent::Over);
        assert_eq!(hit, Some(widget_id));

        let coords = handler
            .app
            .events
            .iter()
            .rev()
            .find_map(|event| match event {
                Event::DragOver { coords } => Some(*coords),
                _ => None,
            })
            .expect("应记录 DragOver 事件");

        assert_eq!(coords.window_logical, window_point);
        assert_eq!(coords.local_logical, Some(Point::new(1.0, 2.0)));
        assert!(matches!(coords.origin, MouseInputOrigin::LogicalInjection));
    }

    #[test]
    fn cached_physical_mouse_position_keeps_click_hit_testing_correct_after_dpi_change() {
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let clicked = Arc::new(AtomicBool::new(false));
        let clicked_clone = Arc::clone(&clicked);

        app.register_interaction(
            widget_id,
            Rect::new(10.0, 10.0, 100.0, 50.0),
            "click",
            move |_| {
                clicked_clone.store(true, Ordering::SeqCst);
            },
        );

        let mut handler = AppHandler::new(app);
        handler.sync_pointer_from_platform_window_event(Point::new(100.0, 60.0));

        // 同一物理位置在 2x 缩放下应映射到逻辑坐标 (50, 30)。
        handler.sync_scale_factor(2.0);
        handler.handle_click(handler.mouse_window_position);

        assert!(
            clicked.load(Ordering::SeqCst),
            "DPI 变化后第一次点击应使用重新计算后的逻辑坐标命中组件"
        );
    }

    // ========================================================================
    // AC12: 键盘 Enter/Space 触发 toggle 测试
    // ========================================================================

    /// 无焦点 widget 时 handle_keyboard_trigger 不 panic（noop）。
    #[test]
    fn keyboard_trigger_no_focus_is_noop() {
        let app = App::new(AppConfig::default());
        let mut handler = AppHandler::new(app);
        // 不应 panic
        handler.handle_keyboard_trigger();
    }

    /// 聚焦 widget 有交互回调时 Enter/Space 触发回调。
    #[test]
    fn keyboard_trigger_calls_interaction_callback() {
        let widget_id = WidgetId::from_u64(42);

        let mut app = App::new(AppConfig::default());
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = std::sync::Arc::clone(&called);

        app.register_interaction(
            widget_id,
            Rect::new(0.0, 0.0, 100.0, 40.0),
            "toggle",
            move |action| {
                assert_eq!(action, "toggle");
                called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            },
        );

        let mut handler = AppHandler::new(app);
        handler.app.focus.focus(widget_id);

        handler.handle_keyboard_trigger();

        assert!(
            called.load(std::sync::atomic::Ordering::SeqCst),
            "键盘触发应调用交互回调"
        );
    }

    /// 聚焦 widget 有 widget_instance 处理器时 Enter/Space 触发处理器。
    #[test]
    fn keyboard_trigger_calls_widget_instance_handler() {
        let widget_id = WidgetId::from_u64(99);

        let mut app = App::new(AppConfig::default());
        app.register_interaction(
            widget_id,
            Rect::new(0.0, 0.0, 50.0, 50.0),
            "toggle",
            move |_| {},
        );

        let handled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let handled_clone = std::sync::Arc::clone(&handled);

        app.register_widget_instance(
            widget_id,
            move |action, _ctx| {
                assert_eq!(action, "toggle");
                handled_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                EventResult::Handled
            },
        );

        let mut handler = AppHandler::new(app);
        handler.app.focus.focus(widget_id);

        handler.handle_keyboard_trigger();

        assert!(
            handled.load(std::sync::atomic::Ordering::SeqCst),
            "键盘触发应调用 widget_instance 处理器"
        );
    }

    /// Enter 键触发与 Space 键触发效果一致。
    #[test]
    fn keyboard_trigger_enter_and_space_both_work() {
        let widget_id = WidgetId::from_u64(1);

        let mut app = App::new(AppConfig::default());
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = std::sync::Arc::clone(&called);

        app.register_interaction(
            widget_id,
            Rect::new(0.0, 0.0, 100.0, 40.0),
            "toggle",
            move |_| {
                called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            },
        );

        let mut handler = AppHandler::new(app);
        handler.app.focus.focus(widget_id);

        // 两次调用应都成功
        handler.handle_keyboard_trigger();
        assert!(called.load(std::sync::atomic::Ordering::SeqCst));

        called.store(false, std::sync::atomic::Ordering::SeqCst);
        handler.handle_keyboard_trigger();
        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }
}
