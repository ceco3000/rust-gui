# D0-D11 vs D8 全量穷举审计报告

> **审计日期**：2026-06-17
> **审计范围**：docs/D0-Rust GUI 框架总体设计.md 至 docs/D11-Cargo项目结构与发布策略.md（共 12 份设计文档）
> **对照基准**：docs/D8-阶段0开发任务分解.md（含 §2-§9.11 全部任务 + 两轮审计补充）

---

## 审计方法

1. 逐份、逐节阅读 D0-D11 全部设计文档
2. 提取所有 struct、enum、trait、fn、impl、算法、配置项
3. 与 D8 任务清单逐项对照（包括 §9.1-§9.11 审计补充任务）
4. 排除：纯架构描述、明确标注「阶段 2 预留」项、D8 §9.10 低优先级后续项（M-13~M-32）、已由 CC01-D05a/FEAT01-FEAT04 覆盖的项

---

## 遗漏清单

| # | 遗漏项 | 来源 | 优先级 | 说明 |
|---|--------|------|--------|------|
| M01 | **PaintContext::push_command(DrawCommand) 方法** | D1 §6.4（PaintContext 定义缺少此方法）；D10 §3 规范要点明确要求「`paint()` 使用 `ctx.push_command()` 而非自行管理 SceneGraph」；D10 §8 Button 示例也依赖此方法输出绘制指令 | P0 | PaintContext 当前只有 `glyph_atlas`、`clip_rect`、`render_cache` 三个字段，没有任何机制让 `paint()` 输出 DrawCommand。D10 多处引用 `ctx.push_command()` 但此方法在设计文档中不存在。需要在 PaintContext 中增加 `commands: &'a mut Vec<DrawCommand>` 字段或等价的 `push_command()` 方法 |
| M02 | **NodeHandle 类型定义** | D2 §2.2（InstanceState 字段 `pub node_handle: NodeHandle`） | P1 | `NodeHandle` 在 D2 中被引用但从未在任何设计文档中定义。它是 widget 树中 retained tree 的节点句柄，用于 InstanceState 关联 widget 实例与树节点。需要在 `rgui-core` 或 `rgui-layout` 中定义此类型 |
| M03 | **PathTessellation 类型定义** | D2 §2.2（RenderLayoutCache 字段 `pub path_tessellation: Option<PathTessellation>`） | P2 | `PathTessellation` 在 D2 中被引用但从未在任何设计文档中定义。它是复杂 SVG 路径的细分缓存。可延迟到阶段 2（Skia 路径渲染时），但类型占位需在 rgui-render 中预留 |
| M04 | **WidgetTree 父子关系维护逻辑** | D5 §3.1（WidgetTree 定义含 `parent`/`children`/`bounds` 映射）；D5 §4（hit_test/traverse_visual_order 依赖这些映射） | P0 | D8 P04a（CC 审计补充）已要求 `WidgetTree` 核心方法（`path_to_root()`、`root()` 等），但未明确要求实现 WidgetTree 的**增删改查**逻辑：当 Patch 执行 CreateWidget/RemoveWidget/MoveWidget 时，WidgetTree 的 `parent`/`children`/`bounds` 三个 FxHashMap 需要同步更新。此逻辑是 apply_patch（S06a）的一部分，但 S06a 只提到了 "create/destroy/move/reparent widget"，未明确要求同步更新 WidgetTree |
| M05 | **消息类型不匹配运行时检测** | D1 §11.7（边界情况） | P1 | 「MessageBinding 的 handler 期望特定消息类型；运行时检测：子组件发出消息后，框架通过 TypeId 检查消息类型是否匹配；不匹配时记录错误日志，丢弃消息」。D8 无此安全检测任务。这是防止类型擦除后（`Box<dyn AppMessage>`）消息类型错误传播的运行时防护 |
| M06 | **循环 CSS 变量引用检测** | D4 §10（边界情况处理） | P2 | 「循环变量引用 → 检测循环，使用 fallback 值」。当 `.rgss` 中 `var(--a)` 引用 `--a: var(--b)` 而 `--b: var(--a)` 时形成循环，需在变量解析阶段（StyleMerger::resolve_variables）检测并降级。D8 ST02/ST07 未覆盖此边界情况 |
| M07 | **FocusManager: 焦点 widget 移除时的回退** | D5 §10（边界情况处理） | P1 | 「焦点 widget 被移除 → 清除焦点，尝试聚焦最近兄弟节点」。此行为是 FocusManager 的健壮性要求，当前 D8 P06 未明确列出此逻辑 |
| M08 | **DrawCommand 枚举缺少 6 个变体** | D3 §3.1（vs D0 §5.3） | P1 | D0 定义的 DrawCommand 仅含 4 个变体（FillRect、FillPath、DrawGlyphs、DrawImage），D3 扩展为 10 个变体（新增 StrokePath、PushClip、PopClip、PushTransform、PopTransform、PushOpacity、PopOpacity）。D8 R02 描述为 "SceneGraph + SceneLayer + DrawCommand"，未区分变体数量。需确保实现时以 D3 的 10 变体为准 |
| M09 | **SceneLayer 新增字段** | D3 §3.1（vs D0 §5.3） | P1 | D0 的 SceneLayer 仅含 `z_index`、`bounds`、`commands` 三个字段。D3 新增 `widget_id: WidgetId`（调试/dirty追踪）、`opacity: f32`（动画/过渡）、`transform: Option<Transform>`（位移/缩放/旋转）。需确保实现包含全部 6 个字段 |
| M10 | **BlendMode 枚举** | D3 §3.3（DrawImage 字段 `blend_mode: BlendMode`） | P1 | D0 的 DrawImage 无 `blend_mode` 字段。D3 的 DrawImage 增加此字段，需定义 `BlendMode` 枚举（`SrcOver`、`Src`、`Multiply`、`Screen`、`Overlay`）。D8 R02 覆盖 DrawCommand 但未明确列出子类型 |
| M11 | **RenderError 枚举缺少 3 个变体** | D3 §5.1（vs D0 §8.3） | P2 | D0 定义的 RenderError 仅含 3 个变体（DeviceLost、SurfaceCreationFailed、ShaderCompilationFailed）。D3 新增 OutOfTextureMemory、Timeout、BackendUnavailable。D8 R04 需确保覆盖全部 6 个变体 |
| M12 | **SceneGraph.version 字段** | D3 §3.1 | P2 | D0 的 SceneGraph 无 `version: u64` 字段。D3 新增此字段用于调试和缓存失效。D8 R02 覆盖 SceneGraph 但未明确列出此字段 |
| M13 | **ViewContext 缺少 scale_factor 字段** | D1 §6.1（vs D0 §5.5） | P0 | D0 的 ViewContext 仅有 `theme`、`locale`、`window_size` 三个字段。D1 新增 `scale_factor: f64` 字段。CC01（DPI 端到端接线）要求 scale_factor 被 RenderParams 消费，但也需要 ViewContext 能访问 scale_factor 以支持 `view()` 中的响应式布局决策。需确保 ViewContext 实现包含此字段 |
| M14 | **UpdateContext 缺少 hover 字段** | D1 §6.2（vs D0 §5.5） | P1 | D0 的 UpdateContext 仅有 `store`、`event_sender`、`focus` 三个字段。D1 新增 `pub hover: Option<WidgetId>` 字段。用于 `update()` 中查询当前悬停状态。需确保实现包含此字段 |
| M15 | **Event 枚举缺少 MouseEnter/MouseLeave 变体** | D5 §2.1 | P1 | Event 枚举定义了 `MouseEnter { widget_id }` 和 `MouseLeave { widget_id }` 两个变体。这些事件需要平台层的鼠标追踪支持（winit 的 `CursorEntered`/`CursorLeft`），且需要通过命中测试→焦点管理链路更新 InstanceState 的 hovered 状态。D8 P03（winit→Event 转换）需覆盖这两个事件的映射逻辑 |

---

## 设计文档内部矛盾（非 D8 遗漏，需修复文档）

| # | 矛盾点 | 涉及文档 | 说明 |
|---|--------|---------|------|
| C1 | **paint() 输出机制不一致** | D0 §3.2 vs D10 §3/§8 | D0/D1 定义 `paint()` 签名为 `fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext)` 无返回值。D10 §3 要求「使用 `ctx.push_command()`」输出 DrawCommand，但 PaintContext 并无此方法。D10 §8 Button 示例中 paint() 返回 `Vec<DrawCommand>`（与 D0/D1 签名冲突）。建议统一：在 PaintContext 中增加 `push_command()` 方法 |
| C2 | **TextureData.format 字段存在性** | D0 §5.4 vs D3 §3.2 | D0 的 TextureData 无 `format` 字段，D3 的 TextureData 增加此字段。VelloBackend::register_texture() 需要 format 参数 |
| C3 | **WidgetId 复用策略矛盾** | D0 §7（不变式 6：不可复用）vs D1 §5.2 阶段 5（卸载后回收，可重新分配） | D8 §9.9 已标注此矛盾，建议以 D0 为准 |

---

## 附录 A：D8 覆盖率统计

### 按文档提取的实现项总数

| 文档 | 提取项数 | 主要类型 |
|------|---------|---------|
| D0 | ~45 | crate 结构、trait 体系、数据结构、Context 类型、SceneGraph、API 边界 |
| D1 | ~35 | WidgetView、WidgetRegistry、Context 详细字段、宏设计、边界情况 |
| D2 | ~35 | StateStore、三层状态、diff/Patch、订阅、快照/迁移 |
| D3 | ~50 | SceneGraph 扩展、DrawCommand 变体、RenderBackend、Vello 后端、GlyphAtlas、DirtyRegion |
| D4 | ~25 | .rgss 解析器、选择器引擎、属性映射表、主题、StyleMerger、热重载 |
| D5 | ~35 | Event 类型体系、EventRouter、WidgetTree、FocusManager、IME、快捷键、平台映射 |
| D6 | ~20 | AccessibilityTree、AccessKitBackend、AccessibilityBackend trait、WCAG 映射 |
| D7 | ~15 | DevTools、IpcMessage/IpcChannel、RestoreMetadata、降级策略 |
| D9 | ~8 | OffscreenTestRunner、assert_screenshot_matches、TestHarness、CI 矩阵 |
| D10 | ~3 | 组件开发规范（无新类型定义，以 Button 示例展示模式） |
| D11 | ~12 | workspace 配置、各 crate Cargo.toml、feature flags、发布脚本 |
| **总计** | **~283** | |

### D8 覆盖率

| 类别 | 数量 | 说明 |
|------|------|------|
| 总提取实现项 | ~283 | D0-D11 全部定义的类型/trait/方法/算法/配置 |
| D8 主任务覆盖 | ~245 | §2-§8.8 任务清单 |
| 审计补充覆盖 | ~20 | §9.1-§9.11（CC01-D05a, FEAT01-FEAT04） |
| 低优先级后续 | ~8 | §9.10（M-13~M-32） |
| 阶段 2 预留 | ~5 | WidgetLifecycle、Capability、SkiaBackend 完整实现等 |
| **本次审计新发现遗漏** | **15** | 本报告 M01-M15 |
| **D8 覆盖率** | **~93.5%** | (245 + 20) / (283 - 8 - 5) ≈ 265/283 |

### 遗漏项按优先级分布

| 优先级 | 数量 | 项 |
|--------|------|-----|
| P0（阻塞） | 3 | M01（PaintContext::push_command）、M04（WidgetTree 增删改查）、M13（ViewContext.scale_factor） |
| P1（核心） | 9 | M02（NodeHandle）、M05（消息类型不匹配检测）、M07（焦点回退）、M08（DrawCommand 变体）、M09（SceneLayer 字段）、M10（BlendMode）、M14（UpdateContext.hover）、M15（MouseEnter/Leave） |
| P2（增强） | 3 | M03（PathTessellation）、M06（循环CSS变量检测）、M11（RenderError 变体）、M12（SceneGraph.version） |

---

## 附录 B：逐文档详细对照索引

### D0 对照

| 实现项 | D0 章节 | D8 对应任务 |
|--------|---------|-----------|
| WidgetId, WindowId | §4.1, §5.1 | C02 ✅ |
| Rect, Size, Point, BoxConstraints, LayoutStyle | §4.1 | C03 ✅ |
| WidgetView\<M\> | §5.1 | C05 ✅ |
| PropValue enum (含 Callback) | §5.1 | C04 + C04a ✅ |
| Key | §5.1 | C04 ✅ |
| Color | §5.1 | C04 ✅ |
| MessageBinding\<M\> | §5.1 | C06 ✅ |
| WidgetSpec trait | §3.2 | C07 + C07a ✅ |
| PersistState trait | §3.3 | C08 ✅ |
| AppMessage trait | §3.4 | C09 ✅ |
| ViewContext | §5.5 | C10 ✅ |
| UpdateContext | §5.5 | C10 + C10a ✅ |
| MeasureContext | §5.5 | C10 ✅ |
| PaintContext | §5.5 | C10 ✅ |
| AccessContext | §5.5 | C10 ✅ |
| AccessibilityNode/Role/Action/State | §4.1 | C11 ✅ |
| WidgetRegistry | §4.1 | C12 ✅ |
| RenderBackend trait | §3.5 | R04 + R04a ✅ |
| TextureData, TextureFormat, TextureId | §5.4 | R04 ✅ |
| SceneGraph, SceneLayer, DrawCommand | §5.3 | R02 ✅ |
| EventSender | §5.5 | C10a ✅ |
| RenderError | §8.3 | R04 ✅ |
| Callback 类型 | §5.1 | C04a ✅ |
| Locale 类型 | §5.5 | FEAT04 ✅ |
| FontMetricsCache | §5.5 | R08a ✅ |
| GlyphAtlas | §5.5 | R07 ✅ |
| crate 级文档 | §8.5 | C13 ✅ |
| FxHashMap/FxHashSet 选择 | §5.2 | S02 ✅ |
| InstanceState | §5.2 | S02 ✅ |
| RenderLayoutCache | §5.2 | S02 ✅ |
| 帧循环 tick() | §6.1 | R12 ✅ |

### D1 对照

| 实现项 | D1 章节 | D8 对应任务 |
|--------|---------|-----------|
| WidgetSpec trait 完整签名 | §2.1 | C07 ✅ |
| PersistState trait（D1 增强版） | §2.4 | C08 ✅ |
| WidgetView 完整定义 + builder API | §3.1, §3.6 | C05 ✅ |
| Key 类型 | §3.2 | C04 ✅ |
| WidgetId 类型 | §3.3 | C02 ✅ |
| PropValue 完整变体 | §3.4 | C04 ✅ |
| MessageBinding + MessageHandler | §3.5 | C06 ✅ |
| WidgetRegistry + RegistryError | §4.1 | C12 ✅ |
| WidgetLifecycle trait | §5.3 | 阶段 2 预留 ❌ |
| ViewContext（含 scale_factor） | §6.1 | C10 + M13 ⚠️ |
| UpdateContext（含 hover） | §6.2 | C10 + M14 ⚠️ |
| EventSender（含 consumed, default_prevented） | §6.2 | C10a ✅ |
| MeasureContext + FontMetrics | §6.3 | C10 + R08a ✅ |
| PaintContext | §6.4 | C10 + M01 ⚠️ |
| AccessContext | §6.5 | C10 ✅ |
| html! 宏 | §7 | F03 ✅ |
| #[derive(WidgetSpec)] | §8.1 | F04 ✅ |
| #[derive(AppMessage)] | §8.2 | P10 ✅ |
| #[derive(PersistState)] | §8.3 | S09 ✅ |
| WidgetCapabilities trait | §9.3 | 阶段 2 预留 ❌ |
| 未注册组件降级（错误占位符） | §11.2 | C12（隐式） |
| 组件异常隔离（catch_unwind） | §11.3 | F02a ✅ |
| 循环订阅检测 | §11.4 | S05 ✅ |
| 消息类型不匹配检测 | §11.7 | M05 ⚠️ |
| WidgetSpec::default_measure() | §8.1 | C07a ✅ |

### D2 对照

| 实现项 | D2 章节 | D8 对应任务 |
|--------|---------|-----------|
| InstanceState（含 NodeHandle） | §2.2 | S02 + M02 ⚠️ |
| RenderLayoutCache（含 PathTessellation） | §2.2 | S02 + M03 ⚠️ |
| LayoutResult | §2.2 | S02/R10 ✅ |
| GlyphCacheEntry, GlyphKey | §2.2 | R07 ✅ |
| StateStore 完整结构 | §3.1 | S02 ✅ |
| Subscription, SubscriptionLifetime | §3.1 | S04 ✅ |
| StateStore 生命周期方法 | §3.3 | S02 ✅ |
| StoreAccess | §4.1 | S03 ✅ |
| StoreAccessMut | §4.2 | S03 ✅ |
| Patch\<M\> 枚举（6 变体） | §5.2 | S06 ✅ |
| diff(), diff_recursive(), resolve_id() | §5.3 | S06 ✅ |
| reconcile_children() | §5.4 | S06 ✅ |
| keyed_reconciliation() | §5.4 | S06 ✅ |
| positional_reconciliation() | §5.4 | S06 ✅ |
| diff_props(), PropDiff | §5.4 | S07 ✅ |
| WidgetIdMap, WidgetPath | §5.5 | S08 ✅ |
| apply_subscriptions() | §6.2 | S04 ✅ |
| cleanup_subscriptions() | §6.2 | S04 ✅ |
| detect_cycles() | §6.3 | S05 ✅ |
| Snapshot, SerializedState | §7.1 | S10 ✅ |
| Snapshotter | §7.2 | S10 ✅ |
| SchemaMigrationRegistry | §7.3 | S11 ✅ |
| SchemaMigration trait | §7.3 | S11 ✅ |
| MigrationError | §7.3 | S11 ✅ |
| RestoreMetadata | §8.2 | D04 ✅ |
| apply_patch（Patch 消费端） | §5.2-5.3 | S06a ✅ |

### D3 对照

| 实现项 | D3 章节 | D8 对应任务 |
|--------|---------|-----------|
| SceneGraph（含 version） | §3.1 | R02 + M12 ⚠️ |
| SceneLayer（含 widget_id, opacity, transform） | §3.1 | R02 + M09 ⚠️ |
| DrawCommand（10 变体） | §3.1 | R02 + M08 ⚠️ |
| PathData, PathCommand, FillRule | §3.3 | R02 ✅ |
| Paint, GradientStop, ImageRepeat | §3.3 | R02 ✅ |
| Stroke, LineCap, LineJoin | §3.3 | R02 ✅ |
| GlyphData | §3.3 | R02 ✅ |
| Transform | §3.3 | R02 ✅ |
| ClipRegion, TextureRef | §3.3 | R02 ✅ |
| BlendMode | §3.3 | M10 ⚠️ |
| TextureId, TextureData, TextureFormat | §3.2 | R04 ✅ |
| RenderSurface | §3.2 | R04/R06 ✅ |
| SceneGraphBuilder | §3.3 | R03 ✅ |
| tick() 帧循环 | §4.1 | R12 ✅ |
| RenderBackend trait（完整 7 方法） | §5.1 | R04 + R04a ✅ |
| BackendCapabilities | §5.1 | R04a ✅ |
| RenderParams | §5.1 | R04 ✅ |
| RenderError（6 变体） | §5.1 | R04 + M11 ⚠️ |
| RenderBackendFactory | §5.2 | R14 ✅ |
| VelloBackend | §6.3 | R05 ✅ |
| to_vello_* / to_wgpu_* 转换函数 | §6.2 | R05 ✅ |
| SkiaBackend | §7 | R13 (P2) |
| GlyphAtlas | §8.1 | R07 ✅ |
| GlyphKey, UploadRect | §8.1 | R07 ✅ |
| SkylineAllocator | §8.1 | R07 ✅ |
| Allocation, RasterizedGlyph | §8.1-8.2 | R07 ✅ |
| DirtyRegionTracker | §9 | R09 ✅ |
| GPU device lost 恢复 | §12.2 | R21 ✅ |
| 字形 Atlas 溢出处理 | §12.3 | R07 ✅ |
| 窗口最小化跳过渲染 | §12.4 | R22 ✅ |
| 后端帧边界切换 | §12.5 | FEAT03 ✅ |

### D4 对照

| 实现项 | D4 章节 | D8 对应任务 |
|--------|---------|-----------|
| RgssParser | §2.4 | ST02 ✅ |
| ParseError | §2.4 | ST02 ✅ |
| Selector 枚举 | §4.1 | ST03 ✅ |
| CombinatorKind, AttrOp | §4.1 | ST03 ✅ |
| Specificity | §4.1 | ST03 ✅ |
| Declaration, SourceLocation | §4.1 | ST02 ✅ |
| SelectorEngine + StyleRule | §4.2 | ST03 ✅ |
| Theme, ColorScheme, ThemeVariables | §5.2 | ST05 ✅ |
| Theme::light() / Theme::dark() | §5.3 | ST06 ✅ |
| StyleMerger | §6.2 | ST07 ✅ |
| !important 支持 | §6.1 | ST07a ✅ |
| StyleHotReload | §7 | ST08 ✅ |
| CSS 函数求值 (calc/min/max/clamp) | §2.2 | ST11 ✅ |
| 响应式断点 + @media | §8 | ST09 ✅ |
| 属性映射表（54+ CSS 属性） | §3 | ST04 ✅ |
| 循环变量引用检测 | §10 | M06 ⚠️ |

### D5 对照

| 实现项 | D5 章节 | D8 对应任务 |
|--------|---------|-----------|
| Event 枚举（含 MouseEnter/Leave） | §2.1 | P03 + M15 ⚠️ |
| Modifiers, FocusSource | §2.1 | P03 ✅ |
| Key 枚举 | §2.2 | C04 ✅ |
| EventRouter | §3.1 | P04 ✅ |
| WidgetTree（数据结构） | §3.1 | P04a ✅ |
| WidgetTree 增删改查 | §3.1/§4 | M04 ⚠️ |
| EventSender（consume/prevent_default） | §3.2 | C10a ✅ |
| WidgetTree::hit_test() | §4 | P05 ✅ |
| WidgetTree::root(), traverse_visual_order() | §4 | P04a ✅ |
| FocusManager | §5.1 | P06 ✅ |
| FocusManager::is_focusable() | §5.1 | P06a ✅ |
| FocusManager tab 导航 | §5.1 | P06 ✅ |
| FocusManager 模态陷阱 | §5.1 | P06 ✅ |
| 焦点 widget 移除回退 | §10 | M07 ⚠️ |
| SpatialNavigation | §6.1 | P14 ✅ |
| ShortcutManager + KeyChord | §6.2 | P07 ✅ |
| ImeManager | §7 | P08 ✅ |
| DragData | §8 | P13 ✅ |
| convert_winit_event() | §9 | P03 ✅ |
| convert_button/convert_key/convert_ime/convert_scroll_delta | §9 | P03 ✅ |
| EventPhase 枚举 | §3.1 | P04（隐式） |
| ScrollDelta 类型 | §2.1 | P03（隐式） |

### D6 对照

| 实现项 | D6 章节 | D8 对应任务 |
|--------|---------|-----------|
| AccessibilityNode/Role/State/Action | §2.1 | C11/A01 ✅ |
| AccessibilityTree | §2.2 | A02 ✅ |
| AccessibilityBackend trait | §4 | A03 ✅ |
| AccessKitBackend | §3.3 | A03 ✅ |
| AnnouncePriority | §4 | A03 ✅ |
| 角色映射 (rgui→accesskit) | §3.2 | A04 ✅ |
| FocusIndicator（焦点轮廓） | §5.2 | A05 ✅ |
| WCAG 2.1 AA 审计 | §5 | A07 ✅ |

### D7 对照

| 实现项 | D7 章节 | D8 对应任务 |
|--------|---------|-----------|
| DevTools | §2.1 | D01/D02 ✅ |
| StyleHotReload（第 1 层） | §3 | ST08 ✅ |
| Fast restart 流程 | §5 | D05a ✅ |
| 双进程架构 | §6 | D03 ✅ |
| IpcMessage, IpcChannel | §7 | D03 ✅ |
| IpcError | §7 | D03 ✅ |
| RestoreMetadata | §8 | D04 ✅ |
| 降级策略（8 种） | §9 | D06 ✅ |

### D9 对照

| 实现项 | D9 章节 | D8 对应任务 |
|--------|---------|-----------|
| OffscreenTestRunner | §4 | T02 ✅ |
| assert_screenshot_matches() | §5 | T03 ✅ |
| TestHarness | §9 | T01 ✅ |
| CI 矩阵配置 | §7 | T04 ✅ |
| 覆盖率集成 | §8 | T05 ✅ |

### D11 对照

| 实现项 | D11 章节 | D8 对应任务 |
|--------|---------|-----------|
| Workspace 根 Cargo.toml | §2.2 | C01 ✅ |
| 各 crate Cargo.toml | §3 | C01/R01/ST01/P01/A01/D01/G01 ✅ |
| Feature flags 矩阵 | §4 | FEAT01 ✅ |
| deny.toml | §2.1 | FEAT02 ✅ |
| rust-toolchain.toml | §2.3 | C01（隐式） |
| workspace.lints 配置 | §2.2 | C01（隐式） |
| publish.sh | §6.2 | U01 ✅ |
| CI/CD publish pipeline | §9 | U02 ✅ |
| 版本号策略 + CHANGELOG | §5 | U03 ✅ |
| Cargo.lock 提交 VCS | §2.2 | C01（隐式，策略项） |

---

> **审计结论**：D8 经过两轮审计补充后覆盖率约 93.5%。本次审计发现 15 个新遗漏项，其中 3 个为 P0（阻塞级），最关键的 M01（PaintContext::push_command）直接影响 paint() 方法无法实际输出绘制指令。建议优先修复 M01 和 M04，其余项可随对应阶段实现时补充。
