//! 运行时 `.rgui` 文件解析器——将 .rgui 文本/文件解析为 WidgetView 树。
//!
//! ## 设计
//!
//! 本模块实现 RG01 `.rgui` 格式规范的解析器，复用 H05 `html_reload` 的
//! quick-xml 架构。与 HTML 解析器不同，`.rgui` 额外支持：
//!
//! - HTML 风格事件（`onclick="fn()"`，而非 `on:event`）
//! - `${id.prop}` 表达式绑定
//! - `slot` 属性
//! - `data-*` 自定义数据属性
//! - 标签名大小写不敏感（统一转为小写）
//!
//! ### 与 `html!` 宏的差异
//!
//! | 特性 | `html!` 宏（编译期） | 运行时 .rgui 解析器 |
//! |------|---------------------|---------------------|
//! | 事件语法 | `on:event={Msg::Variant}` | `onclick="fn()"`（字符串） |
//! | 表达式 | `{rust_expr}` | `${id.prop}` 属性路径 |
//! | 动态表达式 | ✅ 完整 Rust | ❌ 仅属性路径 |
//! | 热重载 | ❌ | ✅ |
//!
//! ### 使用示例
//!
//! ```ignore
//! use rgui_devtools::rgui_parser::parse_rgui_str;
//!
//! let rgui = r#"<Button label="Save"/>"#;
//! let view = parse_rgui_str::<MyMessage>(rgui)?;
//! ```
//!
//! 设计源自 D8 RG02、RG01 .rgui 格式规范。

use ordered_float::OrderedFloat;
use rgui_core::AppMessage;
use rgui_core::id::WidgetId;
use rgui_core::view::{Color, PropValue, WidgetView};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::sync::Arc;

// ============================================================================
// Public Types
// ============================================================================

/// 状态绑定——`.rgui` 中 `expanded="{state.open}"` 的解析结果。
///
/// RS07：声明式数据绑定——`.rgui` 属性值中 `{state.xxx}` 表达式
/// 自动转换为 StateStore 订阅。每个绑定记录所属 prop 名称、
/// state key 和对应的 WidgetId。
#[derive(Debug, Clone)]
pub struct StateBinding {
    /// 属性名称（如 `"expanded"`）。
    pub prop_name: String,
    /// StateStore 中的 key（如 `"open"`，去除 `state.` 前缀）。
    pub state_key: String,
    /// 所属 widget 的 ID（由 `compute_view_layout` 分配）。
    /// 仅在调用 `collect_state_bindings` 时有值。
    pub widget_id: Option<WidgetId>,
}

// ============================================================================
// Error Types
// ============================================================================

/// .rgui 解析错误。
#[derive(Debug)]
pub enum RguiParseError {
    /// 语法/结构错误，携带位置信息。
    ParseError {
        /// 行号（1-based）。
        line: u64,
        /// 列号（1-based）。
        col: u64,
        /// 错误描述。
        message: String,
    },
    /// I/O 读取错误。
    IoError(std::io::Error),
    /// 验证错误（语义检查）。
    ValidationError {
        /// 错误描述。
        message: String,
    },
}

impl fmt::Display for RguiParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError { line, col, message } => {
                write!(f, ".rgui 解析错误 (行 {line}, 列 {col}): {message}")
            },
            Self::IoError(e) => write!(f, ".rgui 文件读取失败: {e}"),
            Self::ValidationError { message } => {
                write!(f, ".rgui 验证错误: {message}")
            },
        }
    }
}

impl std::error::Error for RguiParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for RguiParseError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

// ============================================================================
// Public API
// ============================================================================

/// 从 .rgui 字符串解析为 WidgetView 树。
///
/// # 类型参数
///
/// * `M` - 应用消息类型（仅用于类型标注；运行时解析器不处理消息绑定）。
///
/// # 错误
///
/// * `RguiParseError::ParseError` - .rgui 语法错误
/// * `RguiParseError::ValidationError` - 语义验证错误
///
/// # 示例
///
/// ```ignore
/// let view = parse_rgui_str::<MyMsg>(r#"<Button label="Save"/>"#)?;
/// assert_eq!(view.widget_type, "button");
/// ```
pub fn parse_rgui_str<M: AppMessage>(rgui: &str) -> Result<WidgetView<M>, RguiParseError> {
    let elements = parse_rgui_to_ast(rgui)?;
    if elements.is_empty() {
        return Err(RguiParseError::ParseError {
            line: 1,
            col: 1,
            message: ".rgui 内容为空".into(),
        });
    }
    if elements.len() > 1 {
        return Err(RguiParseError::ValidationError {
            message: format!(
                ".rgui 文件必须有且仅有一个根元素（找到 {} 个）",
                elements.len()
            ),
        });
    }
    let view = ast_to_widget_view(&elements[0]);
    validate_view(&view)?;
    Ok(view)
}

/// 从 `.rgui` 文件解析为 WidgetView 树。
///
/// 读取文件内容后调用 [`parse_rgui_str`]，然后扫描同目录下的
/// `<tag_name>.rgui` + `<tag_name>.rhai` 文件对，将匹配的节点标记为 Tier 2。
///
/// **搜索路径（Qt 风格多路径组件查找）：**
/// 1. `.rgui` 文件所在目录（用户自定义组件，高优先级）
/// 2. `rgui-components/src/`（框架内置 Tier 2 组件，回退路径）
///
/// Tier 2 标记：在节点 props 中注入 `_tier`、`_rhai_path`、`_rgui_path`。
pub fn parse_rgui_file<M: AppMessage>(path: &Path) -> Result<WidgetView<M>, RguiParseError> {
    let mut view = parse_rgui_file_without_tier2(path)?;

    let mut search_dirs = Vec::new();
    if let Some(parent_dir) = path.parent() {
        search_dirs.push(parent_dir.to_path_buf());
    }
    // 回退搜索路径：框架内置 Tier 2 组件目录
    if let Some(framework_dir) = framework_components_dir() {
        search_dirs.push(framework_dir);
    }

    if !search_dirs.is_empty() {
        mark_tier2_nodes(&mut view, &search_dirs);
        expand_component_files(&mut view, &search_dirs);
    }
    Ok(view)
}

/// 获取框架内置 Tier 2 组件目录 (`rgui-components/src/`)。
///
/// 通过编译期 `CARGO_MANIFEST_DIR` 推导 workspace 根目录，
/// 定位到 `rgui-components/src/`。如果路径不存在则返回 `None`。
fn framework_components_dir() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../rgui-components/src");
    if dir.is_dir() {
        Some(dir)
    } else {
        None
    }
}

/// 从 `.rgui` 文件解析为 WidgetView 树（不执行 Tier 2 扫描）。
///
/// 供内部复用（`parse_rgui_file` 在此基础上追加 Tier 2 扫描）。
fn parse_rgui_file_without_tier2<M: AppMessage>(
    path: &Path,
) -> Result<WidgetView<M>, RguiParseError> {
    let content = std::fs::read_to_string(path)?;
    parse_rgui_str(&content)
}

/// 在多路径列表中查找 `<tag_name>.rgui` + `<tag_name>.rhai` 文件对。
///
/// 按 `search_dirs` 顺序逐目录搜索（高优先级在前）。首个找到完整文件对的目录
/// 即返回结果。Qt 风格的多路径组件查找机制，避免组件定义与引用耦合。
///
/// 返回 `(rgui_path, rhai_path)`，如果所有路径都不存在完整文件对则返回 `None`。
/// 标签名统一转为小写后匹配文件名（`<MyCard>` → `mycard.rgui`）。
pub fn find_file_pair(
    search_dirs: &[std::path::PathBuf],
    tag_name: &str,
) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let tag_lower = tag_name.to_lowercase();
    for dir in search_dirs {
        let rgui_path = dir.join(format!("{tag_lower}.rgui"));
        let rhai_path = dir.join(format!("{tag_lower}.rhai"));
        if rgui_path.is_file() && rhai_path.is_file() {
            return Some((rgui_path, rhai_path));
        }
    }
    None
}

/// 递归遍历 WidgetView 树，标记有对应 `.rgui`+`.rhai` 文件对的节点为 Tier 2。
///
/// 按 `search_dirs` 顺序搜索组件文件对（高优先级在前——用户目录优先，框架目录回退）。
///
/// 匹配到文件对时，注入以下 props：
/// - `_tier` = `"2"` — Tier 2 标识
/// - `_rhai_path` — Rhai 脚本绝对路径
/// - `_rgui_path` — 子 .rgui 文件绝对路径
///
/// 未匹配的节点保持 Tier 1（走 WidgetRegistry 回退）。
pub fn mark_tier2_nodes<M: AppMessage>(
    view: &mut WidgetView<M>,
    search_dirs: &[std::path::PathBuf],
) {
    mark_tier2_nodes_recursive(view, search_dirs);
}

/// 递归辅助函数。
fn mark_tier2_nodes_recursive<M: AppMessage>(
    view: &mut WidgetView<M>,
    search_dirs: &[std::path::PathBuf],
) {
    // 检查当前节点是否有对应的文件对
    if let Some((rgui_path, rhai_path)) = find_file_pair(search_dirs, view.widget_type) {
        view.props
            .insert("_tier", PropValue::Str(std::sync::Arc::from("2")));
        view.props.insert(
            "_rhai_path",
            PropValue::Str(std::sync::Arc::from(
                rhai_path.to_string_lossy().to_string(),
            )),
        );
        view.props.insert(
            "_rgui_path",
            PropValue::Str(std::sync::Arc::from(
                rgui_path.to_string_lossy().to_string(),
            )),
        );
    }
    // 递归处理子节点
    for child in &mut view.children {
        mark_tier2_nodes_recursive(child, search_dirs);
    }
}

/// 递归展开组件节点——将标记为 Tier 2 的节点替换为其子 `.rgui` 文件的内容。
///
/// T207：遍历 WidgetView 树，对每个标记了 `_rgui_path` 的节点：
/// 1. 加载并解析子 `.rgui` 文件为 WidgetView 子树
/// 2. 递归展开子树（处理嵌套组件）
/// 3. 将当前节点替换为展开后的子树
///
/// 展开后保留 `_rhai_path` 供后续 Tier 2 paint 执行，清理 `_tier` 和 `_rgui_path`。
///
/// T209：展开后调用 [`replace_slots`] 将父组件传入的子节点注入到子组件模板的 `<slot />` 占位符中。
fn expand_component_files<M: AppMessage>(
    view: &mut WidgetView<M>,
    search_dirs: &[std::path::PathBuf],
) {
    // 1. 检查根节点本身是否为组件，如果是则用展开结果替换整个 view
    if let Some(mut expanded) = try_expand_node(view, search_dirs) {
        // T209: 替换 <slot /> 占位符——将父组件传入的子节点注入子组件模板
        let slot_children = std::mem::take(&mut view.children);
        replace_slots(&mut expanded, slot_children);
        *view = expanded;
    }

    // 2. 递归处理子节点
    let mut new_children = Vec::with_capacity(view.children.len());
    for mut child in view.children.drain(..) {
        // 先递归展开子节点的子树
        expand_component_files(&mut child, search_dirs);

        // 检查当前子节点是否为组件节点
        if let Some(mut expanded) = try_expand_node(&mut child, search_dirs) {
            // T209: 替换 <slot /> 占位符
            let slot_children = std::mem::take(&mut child.children);
            replace_slots(&mut expanded, slot_children);
            new_children.push(expanded);
        } else {
            new_children.push(child);
        }
    }
    view.children = new_children;
}

/// 尝试展开单个节点。如果节点有 `_rgui_path` 标记，加载子 .rgui 文件并返回展开后的 WidgetView。
/// 展开成功时清理 `_tier`/`_rgui_path` 标记，保留 `_rhai_path`。
///
/// T208：展开后将父节点 props 传入子组件作用域，解析 `{prop_name}` 绑定。
/// 展开失败时返回红底占位符。非组件节点返回 `None`。
fn try_expand_node<M: AppMessage>(
    node: &mut WidgetView<M>,
    search_dirs: &[std::path::PathBuf],
) -> Option<WidgetView<M>> {
    let rgui_path_str = match node.props.get("_rgui_path") {
        Some(PropValue::Str(s)) => s.clone(),
        _ => return None,
    };

    // T208: 收集父节点 props（排除内部标记），用于子组件 {prop_name} 绑定解析
    let parent_props: BTreeMap<&'static str, PropValue> = node
        .props
        .iter()
        .filter(|(k, _)| !k.starts_with('_'))
        .map(|(k, v)| (*k, v.clone()))
        .collect();

    let rgui_path = std::path::Path::new(rgui_path_str.as_ref());
    match expand_single_component::<M>(rgui_path, search_dirs) {
        Ok(mut expanded) => {
            // 保留 _rhai_path 供后续 paint 执行
            if let Some(rhai_path) = node.props.remove("_rhai_path") {
                expanded.props.insert("_rhai_path", rhai_path);
            }
            // 清理内部标记
            expanded.props.remove("_tier");
            expanded.props.remove("_rgui_path");

            // T208: 解析子组件中的 {prop_name} 绑定
            resolve_prop_bindings(&mut expanded, &parent_props);

            // T210: 外层 Props 附加——将父组件传入的 props 合并到展开后根节点
            // 子组件同名 prop 优先（不覆盖）
            for (key, value) in &parent_props {
                if !expanded.props.contains_key(key) {
                    expanded.props.insert(*key, value.clone());
                }
            }

            Some(expanded)
        },
        Err(e) => {
            log::warn!(
                "[rgui] expand_component_files: 无法展开组件 '{}' ({}): {e}",
                node.widget_type,
                rgui_path_str.as_ref(),
            );
            // 降级为红底占位符
            let placeholder = make_component_placeholder::<M>(node.widget_type);
            Some(placeholder)
        },
    }
}

// ============================================================================
// T209：<slot /> 内容替换
// ============================================================================

/// 命名 slot 子节点映射：slot 名称 → 子节点列表。
type NamedSlots<M> = BTreeMap<String, Vec<WidgetView<M>>>;

/// T209: 将父组件传入的子节点注入到子组件模板的 `<slot />` 占位符中。
///
/// ## 匹配规则
///
/// - **默认 slot**：`<slot />`（无 `name` 属性）→ 接收父组件中未标记 `slot="name"` 的子节点
/// - **命名 slot**：`<slot name="x" />` → 接收父组件中标记 `slot="x"` 的子节点
/// - **多默认 slot**：仅第一个默认 slot 接收子节点，其余为空
/// - **无 slot**：模板中无 `<slot />` 时，所有子节点被丢弃并记录 warn 日志
///
/// ## 验收标准（D8 §9.17 T209）
///
/// - 子组件含 `<slot />` → 父组件传入子节点正确内联到 slot 位置
/// - 多 `<slot name="x">` 按名称匹配
/// - 无 slot 时子节点丢弃 + 日志
fn replace_slots<M: AppMessage>(view: &mut WidgetView<M>, slot_children: Vec<WidgetView<M>>) {
    // 检查模板中是否有 slot 节点
    if !has_slot_nodes(view) {
        if !slot_children.is_empty() {
            log::warn!(
                "[rgui] replace_slots: 子组件模板无 <slot />，丢弃 {} 个传入子节点",
                slot_children.len()
            );
        }
        return;
    }

    // 分离命名 slot 和默认 slot 子节点
    let (default_children, named_children) = partition_slot_children(slot_children);

    // 递归替换 slot 节点（空列表时 slot 节点也被移除）
    let mut default_used = false;
    replace_slots_recursive(view, &default_children, &named_children, &mut default_used);
}

/// 检查 WidgetView 树中是否包含 `<slot />` 节点。
fn has_slot_nodes<M: AppMessage>(view: &WidgetView<M>) -> bool {
    if view.widget_type == "slot" {
        return true;
    }
    view.children.iter().any(|c| has_slot_nodes(c))
}

/// 将 slot 子节点分为默认组和命名组。
fn partition_slot_children<M: AppMessage>(
    slot_children: Vec<WidgetView<M>>,
) -> (Vec<WidgetView<M>>, NamedSlots<M>) {
    let mut default_children = Vec::new();
    let mut named_children: NamedSlots<M> = BTreeMap::new();

    for mut child in slot_children {
        if let Some(PropValue::Str(s)) = child.props.remove("slot") {
            let name = s.as_ref().to_string();
            if !name.is_empty() {
                named_children.entry(name).or_default().push(child);
                continue;
            }
        }
        default_children.push(child);
    }

    (default_children, named_children)
}

/// 递归替换 WidgetView 树中的 `<slot />` 节点。
///
/// - `default_used` 跟踪第一个默认 slot 是否已消耗（仅第一个默认 slot 接收子节点）
fn replace_slots_recursive<M: AppMessage>(
    view: &mut WidgetView<M>,
    default_children: &[WidgetView<M>],
    named_children: &NamedSlots<M>,
    default_used: &mut bool,
) {
    let mut new_children = Vec::with_capacity(view.children.len() + default_children.len());

    for mut child in view.children.drain(..) {
        if child.widget_type == "slot" {
            // 提取 slot name
            let slot_name = child.props.get("name").and_then(|v| {
                if let PropValue::Str(s) = v {
                    Some(s.as_ref().to_string())
                } else {
                    None
                }
            });

            if let Some(name) = slot_name {
                // 命名 slot：查找匹配的子节点
                if let Some(matched) = named_children.get(&name) {
                    new_children.extend(matched.iter().cloned());
                }
            } else if !*default_used {
                // 默认 slot：仅第一个接收子节点
                new_children.extend(default_children.iter().cloned());
                *default_used = true;
            }
            // 否则该 slot 为空（已消耗或无匹配）
        } else {
            // 非 slot 节点：递归处理
            replace_slots_recursive(&mut child, default_children, named_children, default_used);
            new_children.push(child);
        }
    }

    view.children = new_children;
}

/// T208: 递归解析 WidgetView 树中的 `{prop_name}` 绑定。
///
/// 对每个节点，遍历其所有 prop 值，如果 PropValue::Str 中包含 `{xxx}` 模式：
/// - 在 `parent_props` 中找到对应 key → 替换为 prop 的 Display 值
/// - 找不到 → 替换为空字符串（以满足验收标准：未绑定的 `{xxx}` 渲染为空字符串）
///
/// 递归处理所有子节点以支持嵌套组件。
fn resolve_prop_bindings<M: AppMessage>(
    view: &mut WidgetView<M>,
    parent_props: &BTreeMap<&'static str, PropValue>,
) {
    // 解析当前节点的所有 prop 值
    let mut resolved_props: BTreeMap<&'static str, PropValue> = BTreeMap::new();
    for (name, value) in &view.props {
        match value {
            PropValue::Str(s) => {
                let resolved = resolve_string_bindings(s, parent_props);
                if &resolved != s.as_ref() {
                    resolved_props.insert(*name, PropValue::Str(std::sync::Arc::from(resolved)));
                }
            },
            _ => {},
        }
    }
    for (name, value) in resolved_props {
        view.props.insert(name, value);
    }

    // 递归处理子节点
    for child in &mut view.children {
        resolve_prop_bindings(child, parent_props);
    }
}

/// 解析单个字符串中的 `{prop_name}` 绑定。
///
/// - `"{title}"` → 替换为 parent_props["title"] 的 Display 值
/// - `"Prefix {title} suffix"` → 混合文本和绑定
/// - 未找到的绑定 → 替换为空字符串
fn resolve_string_bindings(
    input: &str,
    parent_props: &BTreeMap<&'static str, PropValue>,
) -> String {
    let mut result = String::with_capacity(input.len());
    let mut i = 0;
    let bytes = input.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != b'}' {
                j += 1;
            }
            if j < bytes.len() {
                let binding = &input[i + 1..j];
                if let Some(prop_value) = parent_props.get(binding) {
                    result.push_str(&prop_value_to_string(prop_value));
                }
                // 未找到绑定 → 替换为空字符串（满足验收标准）
                i = j + 1;
                continue;
            }
        }
        result.push(input[i..].chars().next().unwrap_or('?'));
        i += 1;
    }
    result
}

/// 将 PropValue 转换为字符串表示（用于绑定替换）。
fn prop_value_to_string(value: &PropValue) -> String {
    match value {
        PropValue::Str(s) => s.to_string(),
        PropValue::Bool(b) => b.to_string(),
        PropValue::Int(i) => i.to_string(),
        PropValue::Float(f) => f.to_string(),
        PropValue::Color(c) => c.to_string(),
        PropValue::Size(s) => s.to_string(),
        PropValue::Rect(r) => r.to_string(),
        PropValue::Enum(e) => e.to_string(),
        PropValue::List(_)
        | PropValue::Map(_)
        | PropValue::Callback(_)
        | PropValue::PaintOps(_) => String::new(),
    }
}

/// 加载并展开单个组件——读取子 `.rgui` 文件，解析为 WidgetView，递归展开。
///
/// `search_dirs` 透传给内部 `mark_tier2_nodes` + `expand_component_files`，
/// 确保递归嵌套的 Tier 2 组件也能在多个搜索路径中找到。
///
/// 如果文件不存在或解析失败，返回错误。
fn expand_single_component<M: AppMessage>(
    rgui_path: &Path,
    search_dirs: &[std::path::PathBuf],
) -> Result<WidgetView<M>, RguiParseError> {
    // 递归解析子 .rgui（包含 mark_tier2_nodes + expand_component_files）
    let content = std::fs::read_to_string(rgui_path).map_err(|e| {
        RguiParseError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("无法读取组件文件 '{}': {e}", rgui_path.display()),
        ))
    })?;
    let mut child_view = parse_rgui_str(&content)?;
    if rgui_path.parent().is_some() {
        mark_tier2_nodes(&mut child_view, search_dirs);
        expand_component_files(&mut child_view, search_dirs);
    }
    Ok(child_view)
}

/// 为无法展开的组件创建红底占位符 WidgetView。
fn make_component_placeholder<M: AppMessage>(widget_type: &str) -> WidgetView<M> {
    let mut view = WidgetView::new("Container");
    view.props.insert(
        "background",
        PropValue::Str(std::sync::Arc::from("#FF0000")),
    );
    // 用 Label 显示组件名
    let mut label = WidgetView::new("Label");
    label.props.insert(
        "text",
        PropValue::Str(std::sync::Arc::from(format!("<{widget_type}>"))),
    );
    view.children.push(label);
    view
}

/// 从 WidgetView 树中收集 `id` 属性，建立字符串→WidgetId 双向映射。
///
/// 递归遍历整棵 WidgetView 树，查找所有携带 `id="xxx"` 属性的节点，
/// 建立正向（字符串→WidgetId）和反向（WidgetId→字符串）映射。
///
/// # 示例
///
/// ```ignore
/// use rgui_devtools::rgui_parser::{parse_rgui_str, collect_widget_ids};
///
/// let view = parse_rgui_str::<MyMsg>(r#"<Column><Button id="save" label="Save"/></Column>"#)?;
/// let bimap = collect_widget_ids(&view);
/// assert!(bimap.contains_name("save"));
/// ```
pub fn collect_widget_ids<M: AppMessage>(
    view: &WidgetView<M>,
) -> rgui_core::widget_id_map::WidgetIdBimap {
    let mut bimap = rgui_core::widget_id_map::WidgetIdBimap::new();
    collect_widget_ids_recursive(view, &mut bimap);
    bimap
}

/// 从 WidgetView 树中收集所有 `state.xxx` 声明式绑定（RS07）。
///
/// 递归遍历 WidgetView 树，查找标记为 `${expr:state.*}` 的 prop 值，
/// 为每个 state 绑定提取 widget_id、prop_name 和 state_key。
///
/// # 示例
///
/// ```ignore
/// let mut view = parse_rgui_str::<MyMsg>(r#"<WaAccordionItem expanded="${state.open}"/>"#)?;
/// // WidgetId 需先通过 compute_view_layout() 分配
/// let bindings = collect_state_bindings(&view);
/// assert_eq!(bindings.len(), 1);
/// assert_eq!(bindings[0].prop_name, "expanded");
/// assert_eq!(bindings[0].state_key, "open");
/// ```
pub fn collect_state_bindings<M: AppMessage>(view: &WidgetView<M>) -> Vec<StateBinding> {
    let mut bindings = Vec::new();
    collect_state_bindings_recursive(view, &mut bindings);
    bindings
}

/// 检测表达式是否为 `state.*` 形式（RS07）。
///
/// `{state.open}` → true、`{username.value}` → false。
/// 规则：必须以 `state.` 开头，且后面至少有一个字符。
pub fn is_state_expr(expr: &str) -> bool {
    expr.starts_with("state.") && expr.len() > "state.".len()
}

/// 递归辅助函数：遍历 WidgetView 树，收集 id 属性。
fn collect_widget_ids_recursive<M: AppMessage>(
    view: &WidgetView<M>,
    bimap: &mut rgui_core::widget_id_map::WidgetIdBimap,
) {
    // 检查当前节点的 id prop
    if let Some(PropValue::Str(id_str)) = view.props.get("id") {
        if let Some(widget_id) = view.id {
            bimap.insert(id_str, widget_id);
        }
    }
    // 递归处理子节点
    for child in &view.children {
        collect_widget_ids_recursive(child, bimap);
    }
}

/// 递归辅助函数：遍历 WidgetView 树，收集 state 绑定（RS07）。
///
/// 检查每个 prop 值，若存储为 `${expr:state.xxx}` 标记格式，
/// 则提取 state key 并创建 StateBinding。
fn collect_state_bindings_recursive<M: AppMessage>(
    view: &WidgetView<M>,
    bindings: &mut Vec<StateBinding>,
) {
    // 检查当前节点的所有 props
    for (&prop_name_static, prop_value) in &view.props {
        let prop_name = prop_name_static.to_string();
        if let PropValue::Str(value_str) = prop_value {
            // 检查是否为 ${expr:state.xxx} 标记格式
            if let Some(state_key) = extract_state_key_from_marker(value_str) {
                bindings.push(StateBinding {
                    prop_name,
                    state_key: state_key.to_string(),
                    widget_id: view.id,
                });
            }
        }
    }
    // 递归处理子节点
    for child in &view.children {
        collect_state_bindings_recursive(child, bindings);
    }
}

/// 从 `${expr:state.xxx}` 标记中提取 state key。
///
/// 成功提取返回 `Some("xxx")`，非 state 绑定返回 `None`。
fn extract_state_key_from_marker(marker: &str) -> Option<&str> {
    // marker 格式：${expr:state.xxx}
    let expr = marker.strip_prefix("${expr:")?.strip_suffix('}')?;
    if is_state_expr(expr) {
        // 去除 "state." 前缀
        Some(&expr["state.".len()..])
    } else {
        None
    }
}

// ============================================================================
// Internal AST Types
// ============================================================================

/// .rgui 元素（运行时 AST 节点）。
#[derive(Debug, Clone)]
struct RguiElement {
    tag_name: String,
    attributes: Vec<RguiAttribute>,
    children: Vec<RguiChild>,
}

/// .rgui 属性。
#[derive(Debug, Clone)]
struct RguiAttribute {
    name: String,
    value: RguiValue,
}

/// .rgui 属性值。
#[derive(Debug, Clone)]
enum RguiValue {
    /// 字符串字面量（`attr="val"`）。
    Str(String),
    /// 布尔标志（`flag`，无值属性）。
    Bool,
    /// `${id.prop}` 表达式。
    Expr(String),
}

/// .rgui 子节点。
#[derive(Debug, Clone)]
enum RguiChild {
    Element(RguiElement),
    Text(String),
}

// ============================================================================
// quick-xml based parser
// ============================================================================

use quick_xml::Reader;
use quick_xml::events::Event;

/// 将 .rgui 字符串解析为 `RguiElement` AST。
fn parse_rgui_to_ast(rgui: &str) -> Result<Vec<RguiElement>, RguiParseError> {
    let mut reader = Reader::from_str(rgui);
    reader.config_mut().trim_text(true);

    let mut elements = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let el = parse_element_with_children(&mut reader, e)?;
                elements.push(el);
                buf.clear();
            },
            Ok(Event::Empty(e)) => {
                let el = parse_self_closing(e)?;
                elements.push(el);
                buf.clear();
            },
            Ok(Event::Eof) => break,
            Ok(Event::Text(t)) => {
                let text = t.unescape().map_err(|e| parse_error(&reader, &e))?;
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    return Err(parse_error_msg(
                        &reader,
                        format!("意外的文本内容 '{trimmed}'——所有内容必须位于标签内"),
                    ));
                }
                buf.clear();
            },
            Ok(_) => {
                buf.clear();
            },
            Err(e) => {
                return Err(parse_error(&reader, &e));
            },
        }
    }

    Ok(elements)
}

/// 解析开始标签及其子节点和闭合标签。
fn parse_element_with_children(
    reader: &mut Reader<&[u8]>,
    start: quick_xml::events::BytesStart<'_>,
) -> Result<RguiElement, RguiParseError> {
    let tag_name = String::from_utf8_lossy(start.name().as_ref()).to_string();
    let attributes = parse_attributes(&start);

    let mut children = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let child = parse_element_with_children(reader, e)?;
                children.push(RguiChild::Element(child));
                buf.clear();
            },
            Ok(Event::Empty(e)) => {
                let child = parse_self_closing(e)?;
                children.push(RguiChild::Element(child));
                buf.clear();
            },
            Ok(Event::End(e)) => {
                let name_ref = e.name();
                let end_tag = String::from_utf8_lossy(name_ref.as_ref()).to_lowercase();
                if end_tag != tag_name.to_lowercase() {
                    return Err(parse_error_msg(
                        reader,
                        format!("闭合标签不匹配：期望 `</{tag_name}>`，但找到 `</{end_tag}>`"),
                    ));
                }
                break;
            },
            Ok(Event::Text(t)) => {
                let text = t.unescape().map_err(|e| parse_error(reader, &e))?;
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    children.push(RguiChild::Text(trimmed.to_string()));
                }
                buf.clear();
            },
            Ok(Event::Eof) => {
                return Err(parse_error_msg(
                    reader,
                    format!("未闭合的 `<{tag_name}>` 标签"),
                ));
            },
            Ok(_) => {
                buf.clear();
            },
            Err(e) => return Err(parse_error(reader, &e)),
        }
    }

    Ok(RguiElement {
        tag_name,
        attributes,
        children,
    })
}

/// 解析自闭合标签（`<tag attr="val" />`）。
fn parse_self_closing(e: quick_xml::events::BytesStart<'_>) -> Result<RguiElement, RguiParseError> {
    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
    let attributes = parse_attributes(&e);

    Ok(RguiElement {
        tag_name,
        attributes,
        children: Vec::new(),
    })
}

/// 解析属性列表。
fn parse_attributes(e: &quick_xml::events::BytesStart<'_>) -> Vec<RguiAttribute> {
    let mut attrs = Vec::new();
    let raw_bytes = e.as_ref();

    for attr_result in e.attributes() {
        match attr_result {
            Ok(attr) => {
                let name = String::from_utf8_lossy(attr.key.as_ref())
                    .to_lowercase()
                    .to_string();
                let raw_value = attr.value.as_ref();
                let value = if raw_value.is_empty() {
                    RguiValue::Bool
                } else {
                    let decoded = attr
                        .unescape_value()
                        .map(|cow| cow.to_string())
                        .unwrap_or_else(|_| String::from_utf8_lossy(raw_value).to_string());
                    // 检测 ${...} 表达式
                    if is_expr_binding(&decoded) {
                        RguiValue::Expr(extract_expr(&decoded))
                    } else {
                        RguiValue::Str(decoded)
                    }
                };
                attrs.push(RguiAttribute { name, value });
            },
            Err(err) => {
                // ExpectedEq 错误：从 raw bytes 中提取布尔属性名
                if let Some(pos) = extract_err_position(&err) {
                    if pos > 0 && pos <= raw_bytes.len() {
                        let attr_name = extract_attr_name_at(raw_bytes, pos).to_lowercase();
                        if !attr_name.is_empty() {
                            attrs.push(RguiAttribute {
                                name: attr_name,
                                value: RguiValue::Bool,
                            });
                        }
                    }
                }
            },
        }
    }

    attrs
}

/// 检测属性值是否为 `${...}` 表达式绑定。
fn is_expr_binding(value: &str) -> bool {
    value.starts_with("${") && value.ends_with('}')
}

/// 从 `${expr}` 中提取表达式内容。
fn extract_expr(value: &str) -> String {
    let inner = &value[2..value.len() - 1];
    inner.to_string()
}

/// 从 quick-xml 属性错误中提取字节位置。
fn extract_err_position(err: &dyn fmt::Display) -> Option<usize> {
    let err_str = format!("{err}");
    if let Some(pos_start) = err_str.find("position ") {
        let num_part = &err_str[pos_start + 9..];
        let num_str: String = num_part
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(pos) = num_str.parse::<usize>() {
            return Some(pos);
        }
    }
    if let Some(start) = err_str.find('(') {
        if let Some(end) = err_str[start..].find(')') {
            let num_str = &err_str[start + 1..start + end];
            if let Ok(pos) = num_str.parse::<usize>() {
                return Some(pos);
            }
        }
    }
    None
}

/// 从 raw bytes 中提取位于 position 处的属性名。
fn extract_attr_name_at(raw: &[u8], pos: usize) -> String {
    let end = pos.min(raw.len());
    if end == 0 {
        return String::new();
    }
    let attr_end = raw[..end]
        .iter()
        .rposition(|&b| !b.is_ascii_whitespace())
        .map(|p| p + 1)
        .unwrap_or(0);
    if attr_end == 0 {
        return String::new();
    }
    let attr_start = raw[..attr_end]
        .iter()
        .rposition(|&b| b.is_ascii_whitespace())
        .map(|p| p + 1)
        .unwrap_or(0);
    String::from_utf8_lossy(&raw[attr_start..attr_end]).to_string()
}

// ============================================================================
// AST → WidgetView 转换
// ============================================================================

/// 将 RguiElement AST 转换为 WidgetView。
fn ast_to_widget_view<M: AppMessage>(el: &RguiElement) -> WidgetView<M> {
    let widget_type: &'static str = Box::leak(el.tag_name.clone().into_boxed_str());
    let mut view = WidgetView::new(widget_type);

    for attr in &el.attributes {
        match &attr.value {
            RguiValue::Str(s) => {
                let prop_value = infer_prop_value(s);
                let name: &'static str = Box::leak(attr.name.clone().into_boxed_str());
                view = view.prop(name, prop_value);
            },
            RguiValue::Bool => {
                let name: &'static str = Box::leak(attr.name.clone().into_boxed_str());
                view = view.prop(name, PropValue::Bool(true));
            },
            RguiValue::Expr(expr) => {
                // 表达式绑定：存储原表达式文本（运行时求值由后续阶段实现）
                let name: &'static str = Box::leak(attr.name.clone().into_boxed_str());
                // 用特殊前缀标记为表达式，后续表达式引擎可识别
                let marked = format!("${{expr:{expr}}}");
                view = view.prop(name, PropValue::Str(Arc::from(marked)));
            },
        }
    }

    // 子节点：slot 属性处理
    for child_el in &el.children {
        match child_el {
            RguiChild::Element(child) => {
                view = view.child(ast_to_widget_view(child));
            },
            RguiChild::Text(text) => {
                view = view.child(text_node(text));
            },
        }
    }

    // 文本内容语法糖：如有文本子节点且当前无 text 属性，添加 text prop
    // （注意：text prop 已在属性循环中设置，这里处理无显式 text 属性的情况）
    if !el.children.is_empty() && !view.props.contains_key("text") {
        // 检查第一个文本子节点
        let text_content: String = el
            .children
            .iter()
            .filter_map(|c| match c {
                RguiChild::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        if !text_content.is_empty() {
            view = view.prop("text", PropValue::Str(Arc::from(text_content)));
        }
    }

    view
}

/// 创建文本子节点（`<Label>Hello</Label>` 中的 "Hello" → `<Text text="Hello" />`）。
fn text_node<M: AppMessage>(text: &str) -> WidgetView<M> {
    WidgetView::new("text").prop("text", PropValue::Str(Arc::from(text)))
}

// ============================================================================
// 属性类型推断（与 html! 宏逻辑一致）
// ============================================================================

/// 从字符串字面量推断 `PropValue` 类型。
///
/// 推断规则（与 `html!` 宏一致）：
/// - `"true"` / `"false"` → `PropValue::Bool`
/// - 纯数字整数 → `PropValue::Int`
/// - 含小数点的数字 → `PropValue::Float`
/// - 十六进制颜色 `#RRGGBB` / `#RRGGBBAA` → `PropValue::Color`
/// - `"WxH"` 格式 → `PropValue::Size`
/// - 其他 → `PropValue::Str`
pub fn infer_prop_value(literal: &str) -> PropValue {
    // 布尔值
    if literal == "true" {
        return PropValue::Bool(true);
    }
    if literal == "false" {
        return PropValue::Bool(false);
    }

    // 十六进制颜色：#RRGGBB 或 #RRGGBBAA
    if let Some(color) = try_parse_hex_color(literal) {
        return PropValue::Color(color);
    }

    // "WxH" 尺寸格式（如 "48x48"、"100x200"）
    if let Some(size) = try_parse_size(literal) {
        return PropValue::Size(size);
    }

    // 整数（全数字，可能带前导负号）
    if let Ok(i) = literal.parse::<i64>() {
        return PropValue::Int(i);
    }

    // 浮点数（含小数点）
    if literal.contains('.') {
        if let Ok(f) = literal.parse::<f64>() {
            return PropValue::Float(OrderedFloat(f));
        }
    }

    // 默认：字符串
    PropValue::Str(Arc::from(literal))
}

/// 解析十六进制颜色字符串 `#RRGGBB` 或 `#RRGGBBAA`。
fn try_parse_hex_color(s: &str) -> Option<Color> {
    if !s.starts_with('#') {
        return None;
    }
    let hex = &s[1..];
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;
            Some(Color::new(r, g, b, 1.0))
        },
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()? as f64 / 255.0;
            Some(Color::new(r, g, b, a))
        },
        _ => None,
    }
}

/// 解析 `WxH` 尺寸格式（如 "48x48"、"100x200"）。
fn try_parse_size(s: &str) -> Option<rgui_core::geometry::Size> {
    if let Some(x_pos) = s.find('x') {
        let w_str = &s[..x_pos];
        let h_str = &s[x_pos + 1..];
        if let (Ok(w), Ok(h)) = (w_str.parse::<f64>(), h_str.parse::<f64>()) {
            return Some(rgui_core::geometry::Size::new(w, h));
        }
    }
    None
}

// ============================================================================
// 验证
// ============================================================================

fn validate_view<M: AppMessage>(_view: &WidgetView<M>) -> Result<(), RguiParseError> {
    // V7: 事件名检查保留给后续阶段（RH02 接线时验证）
    Ok(())
}

// ============================================================================
// Helper
// ============================================================================

fn parse_error(reader: &Reader<&[u8]>, e: &dyn fmt::Display) -> RguiParseError {
    RguiParseError::ParseError {
        line: 1,
        col: reader.buffer_position(),
        message: e.to_string(),
    }
}

fn parse_error_msg(reader: &Reader<&[u8]>, message: String) -> RguiParseError {
    RguiParseError::ParseError {
        line: 1,
        col: reader.buffer_position(),
        message,
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // 测试用消息类型
    #[derive(Debug, Clone, PartialEq)]
    enum TestMsg {
        Noop,
    }

    impl AppMessage for TestMsg {
        fn message_name(&self) -> &'static str {
            match self {
                Self::Noop => "noop",
            }
        }
    }

    // --- 解析测试 ---

    #[test]
    fn parse_simple_self_closing() {
        let view = parse_rgui_str::<TestMsg>(r#"<Button label="Save"/>"#).unwrap();
        assert_eq!(view.widget_type, "Button");
        assert_eq!(
            view.props.get("label"),
            Some(&PropValue::Str(Arc::from("Save")))
        );
    }

    #[test]
    fn parse_tag_name_case_insensitive() {
        // 标签名大小写不敏感——end tag 匹配忽略大小写，但 widget_type 保留原始大小写
        let view1 = parse_rgui_str::<TestMsg>(r#"<BUTTON label="A"/>"#).unwrap();
        let view2 = parse_rgui_str::<TestMsg>(r#"<button label="A"/>"#).unwrap();
        let view3 = parse_rgui_str::<TestMsg>(r#"<Button label="A"/>"#).unwrap();
        assert_eq!(view1.widget_type, "BUTTON");
        assert_eq!(view2.widget_type, "button");
        assert_eq!(view3.widget_type, "Button");
    }

    #[test]
    fn parse_nested_elements() {
        let view =
            parse_rgui_str::<TestMsg>(r#"<Column spacing="12"><Label text="Hello"/></Column>"#)
                .unwrap();
        assert_eq!(view.widget_type, "Column");
        assert_eq!(view.props.get("spacing"), Some(&PropValue::Int(12)));
        assert_eq!(view.children.len(), 1);
        assert_eq!(view.children[0].widget_type, "Label");
    }

    #[test]
    fn parse_boolean_attribute() {
        let view = parse_rgui_str::<TestMsg>(r#"<Button disabled/>"#).unwrap();
        assert_eq!(view.props.get("disabled"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn parse_text_content_sugar() {
        let view = parse_rgui_str::<TestMsg>(r#"<Label>Hello World</Label>"#).unwrap();
        // 文本语法糖：<Label>Hello World</Label> → text="Hello World"
        assert_eq!(
            view.props.get("text"),
            Some(&PropValue::Str(Arc::from("Hello World")))
        );
    }

    #[test]
    fn parse_explicit_text_overrides_content() {
        let view = parse_rgui_str::<TestMsg>(r#"<Label text="Override">Fallback</Label>"#).unwrap();
        assert_eq!(
            view.props.get("text"),
            Some(&PropValue::Str(Arc::from("Override")))
        );
    }

    #[test]
    fn parse_id_attribute() {
        let view =
            parse_rgui_str::<TestMsg>(r#"<TextField id="username" placeholder="Name"/>"#).unwrap();
        // id 作为 prop 存储
        assert_eq!(
            view.props.get("id"),
            Some(&PropValue::Str(Arc::from("username")))
        );
    }

    #[test]
    fn parse_hex_color() {
        let view = parse_rgui_str::<TestMsg>(r##"<Container bg="#3B82F6"/>"##).unwrap();
        let color = view.props.get("bg").unwrap();
        assert!(matches!(color, PropValue::Color(_)));
    }

    #[test]
    fn parse_int_float() {
        let view = parse_rgui_str::<TestMsg>(r#"<Container width="200" height="100.5"/>"#).unwrap();
        assert_eq!(view.props.get("width"), Some(&PropValue::Int(200)));
        assert!(matches!(
            view.props.get("height"),
            Some(PropValue::Float(_))
        ));
    }

    #[test]
    fn parse_size_format() {
        let view = parse_rgui_str::<TestMsg>(r#"<Image size="48x48"/>"#).unwrap();
        assert!(matches!(view.props.get("size"), Some(PropValue::Size(_))));
    }

    #[test]
    fn parse_string_values() {
        // 非特殊格式的字符串保持为 Str
        let view = parse_rgui_str::<TestMsg>(r#"<Button variant="Primary"/>"#).unwrap();
        assert_eq!(
            view.props.get("variant"),
            Some(&PropValue::Str(Arc::from("Primary")))
        );
    }

    #[test]
    fn parse_expr_binding() {
        let view = parse_rgui_str::<TestMsg>(r#"<Label text="${name.value}"/>"#).unwrap();
        let text_val = view.props.get("text").unwrap();
        // 表达式存储为标记格式
        if let PropValue::Str(s) = text_val {
            assert!(
                s.contains("expr:name.value"),
                "expected expr marker, got: {s}"
            );
        } else {
            panic!("expected Str, got {text_val:?}");
        }
    }

    #[test]
    fn parse_html_style_event() {
        // HTML 风格事件保留为属性（后续阶段 RH02 会接线到 Rhai）
        let view = parse_rgui_str::<TestMsg>(r#"<Button onclick="save()"/>"#).unwrap();
        // 事件存储为 prop
        assert!(view.props.contains_key("onclick"));
    }

    #[test]
    fn parse_oninput_event() {
        let view =
            parse_rgui_str::<TestMsg>(r#"<TextField oninput="update()" placeholder="Name"/>"#)
                .unwrap();
        let event_val = view.props.get("oninput").unwrap();
        assert_eq!(event_val, &PropValue::Str(Arc::from("update()")));
    }

    #[test]
    fn parse_slot_attribute() {
        let view = parse_rgui_str::<TestMsg>(r#"<Card><Label slot="header" text="Title"/></Card>"#)
            .unwrap();
        assert_eq!(view.children.len(), 1);
        let child = &view.children[0];
        assert_eq!(
            child.props.get("slot"),
            Some(&PropValue::Str(Arc::from("header")))
        );
    }

    #[test]
    fn parse_data_attributes() {
        let view =
            parse_rgui_str::<TestMsg>(r#"<Button data-action="delete" data-id="42"/>"#).unwrap();
        assert_eq!(
            view.props.get("data-action"),
            Some(&PropValue::Str(Arc::from("delete")))
        );
        assert_eq!(view.props.get("data-id"), Some(&PropValue::Int(42)));
    }

    #[test]
    fn parse_deeply_nested() {
        let view = parse_rgui_str::<TestMsg>(
            r#"<Column><Row><Button label="A"/><Button label="B"/></Row></Column>"#,
        )
        .unwrap();
        assert_eq!(view.widget_type, "Column");
        assert_eq!(view.children.len(), 1);
        let row = &view.children[0];
        assert_eq!(row.widget_type, "Row");
        assert_eq!(row.children.len(), 2);
    }

    // --- 错误测试 ---

    #[test]
    fn error_empty_input() {
        let result = parse_rgui_str::<TestMsg>("");
        assert!(result.is_err());
    }

    #[test]
    fn error_multiple_roots() {
        let result = parse_rgui_str::<TestMsg>(r#"<Button/><Label/>"#);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            format!("{err}").contains("根元素"),
            "expected root element error, got: {err}"
        );
    }

    #[test]
    fn error_unclosed_tag() {
        let result = parse_rgui_str::<TestMsg>(r#"<Column>"#);
        assert!(result.is_err());
    }

    #[test]
    fn error_mismatched_close_tag() {
        let result = parse_rgui_str::<TestMsg>(r#"<Column></Row>"#);
        assert!(result.is_err());
    }

    // --- 空白文本处理 ---

    #[test]
    fn whitespace_text_trimmed() {
        let view = parse_rgui_str::<TestMsg>("<Label>\n  Hello  \n</Label>").unwrap();
        assert_eq!(
            view.props.get("text"),
            Some(&PropValue::Str(Arc::from("Hello")))
        );
    }

    // --- 注释 ---

    #[test]
    fn parse_with_comments() {
        let view = parse_rgui_str::<TestMsg>("<!-- comment --><Button label=\"OK\"/>").unwrap();
        assert_eq!(view.widget_type, "Button");
    }

    // --- login.rgui 完整示例 ---

    #[test]
    fn parse_login_form() {
        let view = parse_rgui_str::<TestMsg>(
            r#"<Column spacing="16" padding="24">
    <Label text="Login" variant="Header"/>
    <TextField id="username" placeholder="Username" oninput="updatePreview()"/>
    <TextField id="password" placeholder="Password" password="true"/>
    <Label text="${username.text}"/>
    <Row spacing="8">
        <Button variant="Primary" onclick="login()">Login</Button>
        <Button onclick="cancel()">Cancel</Button>
    </Row>
</Column>"#,
        )
        .unwrap();
        assert_eq!(view.widget_type, "Column");
        assert_eq!(view.children.len(), 5);
        // 确认 id 属性被识别（作为 prop 存储）
        let text_field = &view.children[1];
        assert_eq!(
            text_field.props.get("id"),
            Some(&PropValue::Str(Arc::from("username")))
        );
        // 确认表达式绑定
        let expr_label = &view.children[3];
        assert!(expr_label.props.contains_key("text"));
    }

    // --- collect_widget_ids 测试（RS03）---
    // 注意：WidgetId 由 compute_view_layout() 分配，解析器本身不分配。
    // 以下测试手动设置 WidgetId 模拟布局后的状态。

    /// 辅助函数：为 WidgetView 树递归分配 WidgetId。
    fn assign_widget_ids<M: AppMessage>(view: &mut WidgetView<M>) {
        view.id = Some(rgui_core::id::WidgetId::new());
        for child in &mut view.children {
            assign_widget_ids(child);
        }
    }

    #[test]
    fn collect_single_widget_id() {
        let mut view = parse_rgui_str::<TestMsg>(r#"<Button id="save" label="Save"/>"#).unwrap();
        assign_widget_ids(&mut view);
        let bimap = collect_widget_ids(&view);

        assert_eq!(bimap.len(), 1);
        assert!(bimap.contains_name("save"));
        let id = bimap.get_id("save").unwrap();
        assert_eq!(bimap.get_name(id), Some("save"));
    }

    #[test]
    fn collect_multiple_widget_ids_nested() {
        let mut view = parse_rgui_str::<TestMsg>(
            r#"<Column id="root">
                <Label id="title" text="Hello"/>
                <Button id="submit" label="OK"/>
               </Column>"#,
        )
        .unwrap();
        assign_widget_ids(&mut view);
        let bimap = collect_widget_ids(&view);

        assert_eq!(bimap.len(), 3);
        assert!(bimap.contains_name("root"));
        assert!(bimap.contains_name("title"));
        assert!(bimap.contains_name("submit"));
    }

    #[test]
    fn collect_empty_when_no_id_attrs() {
        let mut view =
            parse_rgui_str::<TestMsg>(r#"<Column><Label text="Hello"/></Column>"#).unwrap();
        assign_widget_ids(&mut view);
        let bimap = collect_widget_ids(&view);

        assert!(bimap.is_empty());
    }

    #[test]
    fn collect_ignores_widgets_without_id() {
        let mut view = parse_rgui_str::<TestMsg>(
            r#"<Column>
                <Label id="named" text="A"/>
                <Label text="B"/>
               </Column>"#,
        )
        .unwrap();
        assign_widget_ids(&mut view);
        let bimap = collect_widget_ids(&view);

        // 只有 "named" 被收集；无 id 属性的 Label 被忽略
        assert_eq!(bimap.len(), 1);
        assert!(bimap.contains_name("named"));
    }

    #[test]
    fn collect_bidirectional_lookup_works() {
        let mut view = parse_rgui_str::<TestMsg>(r#"<Button id="btn_ok" label="OK"/>"#).unwrap();
        assign_widget_ids(&mut view);
        let bimap = collect_widget_ids(&view);

        let id = bimap.get_id("btn_ok").unwrap();
        assert_eq!(bimap.get_name(id), Some("btn_ok"));
    }

    // --- collect_state_bindings 测试（RS07）---

    #[test]
    fn collect_state_bindings_single() {
        let mut view =
            parse_rgui_str::<TestMsg>(r#"<WaAccordionItem expanded="${state.open}"/>"#).unwrap();
        assign_widget_ids(&mut view);
        let bindings = collect_state_bindings(&view);

        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].prop_name, "expanded");
        assert_eq!(bindings[0].state_key, "open");
        assert!(bindings[0].widget_id.is_some());
    }

    #[test]
    fn collect_state_bindings_multiple() {
        let mut view = parse_rgui_str::<TestMsg>(
            r#"<Column>
                <WaAccordionItem id="s1" expanded="${state.s1_open}"/>
                <WaAccordionItem id="s2" expanded="${state.s2_open}"/>
               </Column>"#,
        )
        .unwrap();
        assign_widget_ids(&mut view);
        let bindings = collect_state_bindings(&view);

        assert_eq!(bindings.len(), 2);
        // 两个绑定应有不同的 state keys
        let keys: Vec<&str> = bindings.iter().map(|b| b.state_key.as_str()).collect();
        assert!(keys.contains(&"s1_open"));
        assert!(keys.contains(&"s2_open"));
    }

    #[test]
    fn collect_state_bindings_mixed_with_non_state_exprs() {
        let mut view = parse_rgui_str::<TestMsg>(
            r#"<Column>
                <Label text="${username.value}"/>
                <WaAccordionItem expanded="${state.open}"/>
               </Column>"#,
        )
        .unwrap();
        assign_widget_ids(&mut view);
        let bindings = collect_state_bindings(&view);

        // 只有 state.open 是 state binding，username.value 是普通表达式
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].state_key, "open");
    }

    #[test]
    fn collect_state_bindings_no_bindings() {
        let mut view = parse_rgui_str::<TestMsg>(r#"<Button label="Save"/>"#).unwrap();
        assign_widget_ids(&mut view);
        let bindings = collect_state_bindings(&view);
        assert!(bindings.is_empty());
    }

    #[test]
    fn is_state_expr_detects_state_prefix() {
        assert!(is_state_expr("state.open"));
        assert!(is_state_expr("state.s1_open"));
        assert!(!is_state_expr("username.value"));
        assert!(!is_state_expr(""));
        assert!(!is_state_expr("state")); // 必须有 . 分隔符
    }

    // --- Tier 2 文件对扫描测试（T203）---

    #[test]
    fn find_file_pair_both_exist() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mycard.rgui"), "<Container/>").unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let result = find_file_pair(&[dir.path().to_path_buf()], "mycard");
        assert!(result.is_some());
        let (rgui_path, rhai_path) = result.unwrap();
        assert!(rgui_path.ends_with("mycard.rgui"));
        assert!(rhai_path.ends_with("mycard.rhai"));
    }

    #[test]
    fn find_file_pair_case_insensitive() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mycard.rgui"), "<Container/>").unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        // `<MyCard>` → mycard
        let result = find_file_pair(&[dir.path().to_path_buf()], "MyCard");
        assert!(result.is_some());
    }

    #[test]
    fn find_file_pair_only_rgui_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mycard.rgui"), "<Container/>").unwrap();
        // 无 .rhai 文件

        let result = find_file_pair(&[dir.path().to_path_buf()], "mycard");
        assert!(result.is_none());
    }

    #[test]
    fn find_file_pair_only_rhai_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();
        // 无 .rgui 文件

        let result = find_file_pair(&[dir.path().to_path_buf()], "mycard");
        assert!(result.is_none());
    }

    #[test]
    fn find_file_pair_neither_exist_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let result = find_file_pair(&[dir.path().to_path_buf()], "nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn mark_tier2_nodes_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mycard.rgui"), "<Container/>").unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let mut view = WidgetView::<TestMsg>::new("mycard");
        mark_tier2_nodes(&mut view, &[dir.path().to_path_buf()]);

        assert_eq!(
            view.props.get("_tier"),
            Some(&PropValue::Str(Arc::from("2")))
        );
        assert!(view.props.contains_key("_rhai_path"));
        assert!(view.props.contains_key("_rgui_path"));
        if let Some(PropValue::Str(rhai_path)) = view.props.get("_rhai_path") {
            assert!(rhai_path.contains("mycard.rhai"));
        } else {
            panic!("_rhai_path should be Str");
        }
    }

    #[test]
    fn mark_tier2_nodes_no_file_pair() {
        let dir = tempfile::tempdir().unwrap();
        // 不创建任何文件

        let mut view = WidgetView::<TestMsg>::new("button");
        mark_tier2_nodes(&mut view, &[dir.path().to_path_buf()]);

        // 无文件对 → 不应标记
        assert!(!view.props.contains_key("_tier"));
        assert!(!view.props.contains_key("_rhai_path"));
        assert!(!view.props.contains_key("_rgui_path"));
    }

    #[test]
    fn mark_tier2_nodes_nested() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mycard.rgui"), "<Container/>").unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();
        // 不创建 button 文件对

        let mut column = WidgetView::<TestMsg>::new("column")
            .child(WidgetView::<TestMsg>::new("mycard"))
            .child(WidgetView::<TestMsg>::new("button"));

        mark_tier2_nodes(&mut column, &[dir.path().to_path_buf()]);

        // mycard 应被标记
        let mycard = &column.children[0];
        assert_eq!(
            mycard.props.get("_tier"),
            Some(&PropValue::Str(Arc::from("2")))
        );

        // button 不应被标记（无文件对 → Tier 1）
        let button = &column.children[1];
        assert!(!button.props.contains_key("_tier"));
    }

    #[test]
    fn parse_rgui_file_expands_tier2_component() {
        // T207: 当文件对存在时，Tier 2 组件应自动展开
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(&rgui_path, r#"<Column><MyCard title="Hello"/></Column>"#).unwrap();
        std::fs::write(dir.path().join("mycard.rgui"), "<Container/>").unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        assert_eq!(view.widget_type, "Column");
        assert_eq!(view.children.len(), 1);
        // <MyCard> 展开为 mycard.rgui 的根 <Container>
        let expanded = &view.children[0];
        assert_eq!(expanded.widget_type, "Container");
        // _tier 标记应在展开后被清理
        assert!(!expanded.props.contains_key("_tier"));
        // _rhai_path 保留供后续 paint
        assert!(expanded.props.contains_key("_rhai_path"));
    }

    #[test]
    fn parse_rgui_file_no_tier2_when_no_file_pair() {
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(&rgui_path, r#"<Column><Button label="OK"/></Column>"#).unwrap();
        // 不创建 button.rgui/button.rhai

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        let button = &view.children[0];
        assert_eq!(button.widget_type, "Button");
        // 无文件对 → Tier 1，不应有 _tier prop
        assert!(!button.props.contains_key("_tier"));
    }

    #[test]
    fn parse_rgui_file_only_rgui_no_rhai_returns_tier1() {
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(&rgui_path, r#"<MyCard title="Hello"/>"#).unwrap();
        // 只创建 .rgui，不创建 .rhai——Tier 2 需要两者都存在
        std::fs::write(dir.path().join("mycard.rgui"), "<Container/>").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        // 只有 .rgui 无 .rhai → 应回退到 Tier 1
        assert!(!view.props.contains_key("_tier"));
    }

    // --- T207 组件展开测试 ---

    #[test]
    fn parse_rgui_file_expands_component() {
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(&rgui_path, r#"<Column><MyCard title="Hello"/></Column>"#).unwrap();
        std::fs::write(
            dir.path().join("mycard.rgui"),
            r#"<Container><Label text="Card content"/></Container>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        // <Column> 仍然是根节点
        assert_eq!(view.widget_type, "Column");
        // <MyCard> 已被展开为子 .rgui 的内容
        assert_eq!(view.children.len(), 1);
        let expanded = &view.children[0];
        // 展开后根节点应为子 .rgui 的根节点 (Container)
        assert_eq!(expanded.widget_type, "Container");
        // 子 .rgui 的内容应在其中
        assert_eq!(expanded.children.len(), 1);
        assert_eq!(expanded.children[0].widget_type, "Label");
        // 展开后不应保留 Tier 2 标记 props
        assert!(!expanded.props.contains_key("_tier"));
        assert!(!expanded.props.contains_key("_rgui_path"));
        // 但应保留内部标记（用于后续 Tier 2 渲染步骤）
        assert!(expanded.props.contains_key("_rhai_path"));
    }

    #[test]
    fn parse_rgui_file_expands_nested_components() {
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(&rgui_path, r#"<Outer title="Hello"/>"#).unwrap();
        // outer.rgui 引用 inner
        std::fs::write(
            dir.path().join("outer.rgui"),
            r#"<Column><Inner content="nested"/></Column>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("outer.rhai"), "fn paint() {}").unwrap();
        // inner.rgui 是叶子
        std::fs::write(
            dir.path().join("inner.rgui"),
            r#"<Label text="inner content"/>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("inner.rhai"), "fn inner_paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        // 根节点 <Outer> 被展开为 outer.rgui 的内容
        // outer.rgui 的根是 <Column>
        assert_eq!(view.widget_type, "Column");
        assert_eq!(view.children.len(), 1);
        // <Column> 的子节点 <Inner> 也应被展开为 inner.rgui 的内容
        let inner_expanded = &view.children[0];
        assert_eq!(inner_expanded.widget_type, "Label");
        assert_eq!(inner_expanded.children.len(), 0);
    }

    #[test]
    fn parse_rgui_file_expands_root_component() {
        // T207: 根节点本身是组件时也应展开
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(&rgui_path, r#"<MyCard title="Hello"/>"#).unwrap();
        // 创建 .rgui 和 .rhai 以标记为 Tier 2
        std::fs::write(dir.path().join("mycard.rgui"), "<Container/>").unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        // .rgui 文件存在，应正常展开
        assert_eq!(view.widget_type, "Container");
        assert!(!view.props.contains_key("_tier"));
    }

    #[test]
    fn parse_rgui_file_expansion_cleans_internal_markers() {
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(&rgui_path, r#"<MyCard title="Hello"/>"#).unwrap();
        std::fs::write(
            dir.path().join("mycard.rgui"),
            r#"<Container><Label text="hello"/></Container>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        // 展开后 _tier 和 _rgui_path 应被清理
        assert!(!view.props.contains_key("_tier"));
        assert!(!view.props.contains_key("_rgui_path"));
        // _rhai_path 保留给 Tier 2 paint 执行
        assert!(view.props.contains_key("_rhai_path"));
    }

    // --- T208 Props 传递测试 ---

    #[test]
    fn parse_rgui_file_resolves_simple_prop_binding() {
        // T208: <MyCard title="Hello"> → 子组件内 {title} 解析为 "Hello"
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(&rgui_path, r#"<MyCard title="Hello"/>"#).unwrap();
        std::fs::write(
            dir.path().join("mycard.rgui"),
            r#"<Container><Label text="{title}"/></Container>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        // 展开后根节点为 Container
        assert_eq!(view.widget_type, "Container");
        assert_eq!(view.children.len(), 1);
        let label = &view.children[0];
        assert_eq!(label.widget_type, "Label");
        // {title} 应解析为 "Hello"
        assert_eq!(
            label.props.get("text"),
            Some(&PropValue::Str(std::sync::Arc::from("Hello")))
        );
    }

    #[test]
    fn parse_rgui_file_resolves_nested_prop_binding_3_levels() {
        // T208: props 在嵌套 3 层以上保持传递
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(&rgui_path, r#"<Outer title="Hello"/>"#).unwrap();
        // outer.rgui 引用 middle
        std::fs::write(
            dir.path().join("outer.rgui"),
            r#"<Middle label="{title}"/>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("outer.rhai"), "fn paint() {}").unwrap();
        // middle.rgui 显示 label
        std::fs::write(dir.path().join("middle.rgui"), r#"<Label text="{label}"/>"#).unwrap();
        std::fs::write(dir.path().join("middle.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        // 展开后根节点为 Label (middle.rgui 的根)
        assert_eq!(view.widget_type, "Label");
        // {label} → "Hello" (通过 outer → middle 传递)
        assert_eq!(
            view.props.get("text"),
            Some(&PropValue::Str(std::sync::Arc::from("Hello")))
        );
    }

    #[test]
    fn parse_rgui_file_unresolved_binding_renders_empty() {
        // T208: 未绑定的 {xxx} 渲染为空字符串
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(&rgui_path, r#"<MyCard title="Hello"/>"#).unwrap();
        std::fs::write(
            dir.path().join("mycard.rgui"),
            r#"<Container><Label text="{unknown}"/></Container>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        assert_eq!(view.widget_type, "Container");
        let label = &view.children[0];
        // {unknown} → ""
        assert_eq!(
            label.props.get("text"),
            Some(&PropValue::Str(std::sync::Arc::from("")))
        );
    }
    // --- T209 <slot /> 内容替换测试 ---

    #[test]
    fn parse_rgui_file_default_slot_replaces_children() {
        // T209: 子组件含 <slot /> → 父组件传入子节点正确内联到 slot 位置
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(
            &rgui_path,
            r#"<MyCard title="Hello"><Label text="Child content"/></MyCard>"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("mycard.rgui"),
            r#"<Container><slot /></Container>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        assert_eq!(view.widget_type, "Container");
        assert_eq!(view.children.len(), 1);
        let slotted = &view.children[0];
        assert_eq!(slotted.widget_type, "Label");
        assert_eq!(
            slotted.props.get("text"),
            Some(&PropValue::Str(std::sync::Arc::from("Child content")))
        );
    }

    #[test]
    fn parse_rgui_file_named_slot_by_name() {
        // T209: 多 <slot name="x"> 按名称匹配
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(
            &rgui_path,
            r#"<MyCard>
                <Label slot="header" text="Header content"/>
                <Label slot="footer" text="Footer content"/>
            </MyCard>"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("mycard.rgui"),
            r#"<Column>
                <slot name="header" />
                <Container><Label text="Body"/></Container>
                <slot name="footer" />
            </Column>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        assert_eq!(view.widget_type, "Column");
        assert_eq!(view.children.len(), 3);
        assert_eq!(view.children[0].widget_type, "Label");
        assert_eq!(
            view.children[0].props.get("text"),
            Some(&PropValue::Str(std::sync::Arc::from("Header content")))
        );
        assert_eq!(view.children[1].widget_type, "Container");
        assert_eq!(view.children[2].widget_type, "Label");
        assert_eq!(
            view.children[2].props.get("text"),
            Some(&PropValue::Str(std::sync::Arc::from("Footer content")))
        );
    }

    #[test]
    fn parse_rgui_file_mixed_default_and_named_slots() {
        // T209: 默认 slot + 命名 slot 混合
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(
            &rgui_path,
            r#"<MyCard>
                <Label text="Default child"/>
                <Button slot="actions" label="OK"/>
            </MyCard>"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("mycard.rgui"),
            r#"<Column><slot /><slot name="actions" /></Column>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        assert_eq!(view.widget_type, "Column");
        assert_eq!(view.children.len(), 2);
        assert_eq!(view.children[0].widget_type, "Label");
        assert_eq!(view.children[1].widget_type, "Button");
    }

    #[test]
    fn parse_rgui_file_no_slot_discards_children() {
        // T209: 无 slot 时子节点丢弃（不 panic）
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(
            &rgui_path,
            r#"<MyCard><Label text="Should be discarded"/></MyCard>"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("mycard.rgui"),
            r#"<Container><Label text="Static content"/></Container>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        assert_eq!(view.widget_type, "Container");
        assert_eq!(view.children.len(), 1);
        assert_eq!(view.children[0].widget_type, "Label");
        assert_eq!(
            view.children[0].props.get("text"),
            Some(&PropValue::Str(std::sync::Arc::from("Static content")))
        );
    }

    #[test]
    fn parse_rgui_file_self_closing_component_no_children() {
        // T209: 自闭合 <MyCard /> 无子节点 → 展开成功，slot 为空
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(&rgui_path, r#"<MyCard title="Hello"/>"#).unwrap();
        std::fs::write(
            dir.path().join("mycard.rgui"),
            r#"<Container><slot /></Container>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        assert_eq!(view.widget_type, "Container");
        assert_eq!(view.children.len(), 0);
    }

    #[test]
    fn parse_rgui_file_nested_component_slots() {
        // T209: 嵌套组件中的 slot 替换
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(
            &rgui_path,
            r#"<Outer>
                <Label text="Grandchild"/>
            </Outer>"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("outer.rgui"),
            r#"<Column><Inner><slot /></Inner></Column>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("outer.rhai"), "fn paint() {}").unwrap();
        std::fs::write(
            dir.path().join("inner.rgui"),
            r#"<Container><slot /></Container>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("inner.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        assert_eq!(view.widget_type, "Column");
        assert_eq!(view.children.len(), 1);
        let inner = &view.children[0];
        assert_eq!(inner.widget_type, "Container");
        assert_eq!(inner.children.len(), 1);
        assert_eq!(inner.children[0].widget_type, "Label");
        assert_eq!(
            inner.children[0].props.get("text"),
            Some(&PropValue::Str(std::sync::Arc::from("Grandchild")))
        );
    }

    #[test]
    fn parse_rgui_file_multiple_default_slots_only_first_gets_children() {
        // T209: 多个默认 slot → 仅第一个接收子节点
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(&rgui_path, r#"<MyCard><Label text="Only one"/></MyCard>"#).unwrap();
        std::fs::write(
            dir.path().join("mycard.rgui"),
            r#"<Column><slot /><slot /></Column>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        assert_eq!(view.widget_type, "Column");
        assert_eq!(view.children.len(), 1);
        assert_eq!(view.children[0].widget_type, "Label");
    }

    // --- T210 外层 Props 附加测试 ---

    #[test]
    fn outer_props_attached_to_expanded_root() {
        // T210: <my_card onclick="fn"> 展开后根节点自动携带 onclick
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(&rgui_path, r#"<MyCard onclick="handleClick()"/>"#).unwrap();
        std::fs::write(
            dir.path().join("mycard.rgui"),
            r#"<Container><Label text="Card content"/></Container>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        // 展开后根节点为 Container
        assert_eq!(view.widget_type, "Container");
        // 外层 onclick 应附加到根节点
        assert_eq!(
            view.props.get("onclick"),
            Some(&PropValue::Str(std::sync::Arc::from("handleClick()")))
        );
        // 子节点内容应保留
        assert_eq!(view.children.len(), 1);
        assert_eq!(view.children[0].widget_type, "Label");
    }

    #[test]
    fn outer_props_child_priority() {
        // T210: 外层 props 不覆盖子组件同名 prop（子优先）
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(&rgui_path, r#"<MyCard title="Outer"/>"#).unwrap();
        std::fs::write(
            dir.path().join("mycard.rgui"),
            r#"<Container title="Child"/>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        assert_eq!(view.widget_type, "Container");
        // 子组件的 title 应保留（子优先）
        assert_eq!(
            view.props.get("title"),
            Some(&PropValue::Str(std::sync::Arc::from("Child")))
        );
    }

    #[test]
    fn outer_props_added_when_child_lacks_them() {
        // T210: 子组件没有的 prop，外层 prop 应附加到根节点
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(
            &rgui_path,
            r#"<MyCard variant="primary" onclick="handle()"/>"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("mycard.rgui"),
            r#"<Container><Label text="Card"/></Container>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        assert_eq!(view.widget_type, "Container");
        // variant 应附加到根节点（子组件未定义）
        assert_eq!(
            view.props.get("variant"),
            Some(&PropValue::Str(std::sync::Arc::from("primary")))
        );
        // onclick 应附加到根节点（子组件未定义）
        assert_eq!(
            view.props.get("onclick"),
            Some(&PropValue::Str(std::sync::Arc::from("handle()")))
        );
        // 子节点内容应保留
        assert_eq!(view.children.len(), 1);
    }

    #[test]
    fn outer_props_no_internal_markers_merged() {
        // T210: 内部标记（_ 前缀）不应合并到展开后根节点
        let dir = tempfile::tempdir().unwrap();
        let rgui_path = dir.path().join("main.rgui");
        std::fs::write(&rgui_path, r#"<MyCard title="Hello"/>"#).unwrap();
        std::fs::write(
            dir.path().join("mycard.rgui"),
            r#"<Container><Label text="{title}"/></Container>"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("mycard.rhai"), "fn paint() {}").unwrap();

        let view = parse_rgui_file::<TestMsg>(&rgui_path).unwrap();

        assert_eq!(view.widget_type, "Container");
        // _tier/_rgui_path/_rhai_path 不应出现在展开后的根节点
        assert!(!view.props.contains_key("_tier"));
        assert!(!view.props.contains_key("_rgui_path"));
        // _rhai_path 是 T207 显式保留的，不属于 T210 合并范围
        // 但应通过 _rhai_path 检查确认内部标记未被合并
        // title 应来自 T208 绑定解析（而非外层 prop 覆盖）
        let label = &view.children[0];
        assert_eq!(
            label.props.get("text"),
            Some(&PropValue::Str(std::sync::Arc::from("Hello")))
        );
    }
}
