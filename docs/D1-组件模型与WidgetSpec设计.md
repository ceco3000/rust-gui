# rgui 组件模型与 WidgetSpec 设计（D1）

> 版本：0.1.0
> 工作目录：`/Users/chenchao/Documents/code/rust/RUST-GUI`
> 写实原则：本文与当前代码实际一致。核心契约对齐 greenfield §B.1；组件宿主已实现（D10）。部分条目（如组件注册表 register、derive 宏）为契约级/占位，已如实标注。

---

## 1. 定位

`rgui` 采用**唯一 Tier 1 WidgetSpec 组件模型**。一个 widget 由关联状态类型（`State: PersistState`）与消息类型（`Message: AppMessage`）定义，通过统一 trait 提供 view/update/measure/paint 生命周期。`.rgui` / `.rhai` 声明式路径、Rhai 脚本已废弃，无第二组件路径。

---

## 2. `WidgetSpec` trait（核心契约，来自 `rgui-core/src/traits.rs`）

```rust
pub trait WidgetSpec: Send + Sync + 'static {
    type State: PersistState;
    type Message: AppMessage;

    fn name(&self) -> &'static str;
    fn view(&self, state: &Self::State, ctx: &ViewContext) -> WidgetView<Self::Message>;
    fn update(&self, msg: Self::Message, state: &mut Self::State, ctx: &mut UpdateContext);
    fn measure(&self, state: &Self::State, constraints: BoxConstraints, ctx: &MeasureContext) -> Size;
    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext);
    fn accessibility(&self, _s: &Self::State, _c: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none()
    }
}
```

要点：
- `measure` 用 `BoxConstraints` + `MeasureContext` 返回 `Size`（对齐 greenfield §B.1 准确签名）。
- `accessibility` 有默认实现返回 `AccessibilityNode::none()`，可覆盖。
- **派生宏**：`#[derive(WidgetSpec)]` 来自 `rgui-macros`，当前为**透传输入**（不做展开，见 §9）。

---

## 3. 关联类型与约束

| 关联类型 | 约束 | 说明 |
|----------|------|------|
| `State` | `PersistState` | 组件状态，可快照/恢复（`schema_name`/`schema_version`/`as_any`/`as_any_mut`） |
| `Message` | `AppMessage` | 组件消息，跨边界传递（`message_name`） |

`AppMessage` / `PersistState` / `EventResult` 签名见 D0 §4（此处不重复）。

---

## 4. 组件生命周期

1. **`view`**：声明式构建视图树 `WidgetView<Self::Message>`（只读状态）。
2. **`update`**：处理消息，更新可变状态（生产新视图）。`EventResult` 用于事件传播决策（`Handled`/`Prevented`/`Continue(M)`）。
3. **`measure`**：计算尺寸（受 `BoxConstraints` 约束）。
4. **`paint`**：绘制到 `PaintContext`。
5. **`accessibility`**：产出无障碍信息（默认无）。

---

## 5. `Coordinator`——核心循环宿主（`rgui-core/src/coordinator.rs`）

持有**具体组件实例 + 其状态**，驱动组件完成「状态变化 → 视图更新 → 重绘」的 view→update→view 最小闭环，纯 Rust 可测（D3 占位空壳已替换为实现，`WidgetSpec` 签名保持 §B.1 不变）。

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `pub fn new(spec: W, state: W::State) -> Self` | 绑定组件与其初始状态 |
| `current_view` | `pub fn current_view(&self, ctx: &ViewContext) -> WidgetView<W::Message>` | 当前状态下视图（只读） |
| `dispatch` | `pub fn dispatch(&mut self, msg: W::Message, ctx: &mut UpdateContext) -> WidgetView<W::Message>` | 调用 `update` 更新状态，返回更新后视图 |
| `name` | `pub fn name(&self) -> &'static str` | 组件名 |
| `state` | `pub fn state(&self) -> &W::State` | 状态引用 |
| `current_view_default` | `pub fn current_view_default(&self) -> WidgetView<W::Message>` | 用默认 ViewContext 便捷获取视图 |

**实际使用**（见 `rgui/examples/demo.rs`）：`Coordinator::new(Counter, CounterState::default())` → `current_view(&ViewContext::default())` → `dispatch(Increment, &mut UpdateContext::default())` → `diff(v0, v1)` / `Snapshotter` 快照。

---

## 6. 组件注册表 `WidgetRegistry`（`rgui-core/src/registry.rs`）

| 类型 | 说明 |
|------|------|
| `RegistryError` | `DuplicateId` / `NotFound` |
| `WidgetRegistry` | `inner: RwLock<HashMap<String, Arc<dyn Any + Send + Sync>>>`；`new()`、`register(name, widget)` |

- **当前实现状态**：`register` 为**占位空实现**（body 空，注册逻辑实现阶段补全）；字段 `inner` 为 dead_code。注册表当前仅承载契约占位。

---

## 7. 视图 / 属性 / 上下文类型（`rgui-core`）

| 类型 | 说明（见 D0 §5/§6） |
|------|------|
| `WidgetView<M=()>` | 视图树节点（children/props/size/border(D16)/key(D18)） |
| `PropValue` | `Unit/Bool/Int/Float/Str/Color` 属性值 |
| `Key` | `Str(String)/Num(u64)` 稳定键 |
| `Color` | sRGB `u8×4` |
| `ViewContext`（D13：`pub focused: bool` 视图层焦点透传）/ `UpdateContext` / `MeasureContext` / `PaintContext` / `AccessContext` | 生命周期上下文 |

---

## 8. 内置组件（`rgui-core/src/components.rs`）

- `Accordion`（`AccordionState`/`AccordionMsg`）——折叠/展开容器。
- `WaBadge`（`WaBadgeState`/`WaBadgeMsg`）——徽标。

两者均为唯一 Tier 1 WidgetSpec 实现。规范与示例见 D10。

---

## 9. 当前实现状态（D10，写实）

1. **已实现**：`WidgetSpec` 五方法契约完整；`Coordinator` 完整实现（view→update→view 闭环）；`Accordion`/`WaBadge` 组件实现。
2. **契约级/占位**：`WidgetRegistry::register` 为空实现（占位）；`#[derive(WidgetSpec)]` 宏为透传；`Callback`/`MessageBinding`/`MessageHandler` 为占位类型。
3. **事件路由**：hit-test 命中未实现（P1，见 D5）——组件当前由 `Coordinator` 逐组件驱动，多组件命中待做。
