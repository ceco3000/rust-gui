# D5（vello 离屏渲染 + SceneGraph）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D5 交付（rgui-render 离屏渲染 + 场景图）
> 审查基准：`tools/2025-09-01_rgui-greenfield-architecture.md` §B.2/§C.2/§D + D4 审查遗留
> 审查方式：只读代码核查 + 依赖方向分析（未运行 GPU 测试，逐行审 wgpu/vello 调用）
> 声明：未跑 `cargo test`（GPU 依赖），以下分析基于源码逐行审读；离屏像素断言依赖实际 GPU 环境，以 qa 注明的"环境依赖"为准。

---

## 〇、结论速览

| # | 审查点 | 定级 | 一句话结论 |
|---|---|---|---|
| Q1 | 离屏渲染正确性 | **P1（1 项测试盲区）** | readback 硬件路径正确，但测试用 `red_filled_rect` 绕过了 `from_view` 转换 |
| Q2 | SceneGraph 纯度 | **PASS（纯度达标）** | DrawCmd 纯 Rust、不泄漏 GPU 类型 ✓；但 from_view 是占位（固定矩形，未用布局）|
| Q3 | 防火墙 | **PASS** | core 零 GPU（Cargo.toml + 源码）；render 单一 vello 无 skia 残留 |
| Q4 | unsafe 与安全 | **PASS** | 全仓零 unsafe（离屏走 wgpu safe API）|
| Q5 | 增量编译 | **P2（正向未验证）** | 反向（改 render→core）真可达可验证；**正向（改 core 数据层→render）仍未是可测指标**（qa 只锁反向 INC1）|
| Q6 | 进入 D6 前风险 | —— | 见 §六：D6 前需确认的核心循环串联缺口 |

**总评：D5 的离屏硬件路径离屏 readback、SceneGraph 纯度、防火墙、unsafe 克制均达标，P0 清零，具备放行 D6 的条件。** 但有 1 项 P1（离屏测试真实性盲区：像素断言没经过真实 WidgetView→SceneGraph 转换）和 1 项核心观察（render 迄今仍是"能画出的证明模块"，尚未接入真实组件/状态循环）。**建议：有条件放行（CONDITIONAL PASS），P1 在 D6 串联时一并闭合。**

---

## 一、Q1 — 离屏渲染正确性（P1，测试真实性盲区）

### 1.1 【硬件路径正确性 — PASS】readback 流程严谨（逐行审读）
`VelloBackend::render_offscreen`（vello.rs:61-158）的 GPU readback 链路**完整且同步正确**：
1. `create_texture`（Rgba8Unorm + STORAGE_BINDING | COPY_SRC，vello.rs:71-84）——用法正确。
2. `render_to_texture`（vello.rs:97-110）——vello 0.9 离屏标准调用，参数正确。
3. `copy_texture_to_buffer`（vello.rs:116-131）——`bytes_per_row` 用 `aligned`（对齐 256），layout 正确。**关键点**：`TexelCopyBufferLayout.bytes_per_row = Some(aligned)`，与 buffer 大小 `aligned * height` 一致，无越界写。✓
4. `map_async(MapMode::Read)` + `device.poll(Waiter)`（vello.rs:137-143）——**同步读回正确**（`PollType::Wait` 阻塞等待到 map 完成，无数据竞争）。✓
5. `get_mapped_range` → 逐行去对齐 → `drop(data)` → `buffer.unmap()`（vello.rs:147-156）——**u32 row stride 解包正确**（从 `aligned` 行提取 `bytes_per_row` 紧致无空洞）。✓
6. 资源生命周期：texture/buffer 在函数作用域结尾**由 Rust 所有权自动 drop**（vello.rs:71/90 局部变量），**无资源泄漏**。

### 1.2 【P1-测试真实性】离屏测试绕过了真实转换路径
- `offscreen.rs:17`：`let scene = SceneGraph::red_filled_rect(64.0, 64.0);` ——**测试用手工构造的红色矩形**（scene_graph.rs:82-92），**不是**从 `WidgetView` 转换而来。
- 真实转换路径 `SceneGraph::from_view`（scene_graph.rs:58）**是 `#[allow(dead_code)]` 占位**，**没有任何测试调用它**（grep 全仓仅 scene_graph.rs:58 定义，无调用点）。
- **判定**：`offscreen_renders_red_rect_to_pixels`（offscreen.rs:14-32）验证的是 `render_offscreen` 对**手工 SceneGraph** 的像素正确性，**没有验证"WidgetView→SceneGraph→渲染"这条 D5 核心目标链路**。这不是缺陷（硬件/编码路径确实 work），但**是测试盲区**——它让"像素 R>200/G<60/B<60"断言成立，却掩盖了"from_view 转换是否正确"这一 D5 验收的核心（qa 清单 §2 SG1-SG7 全是"待 dev 实现"的契约锁定，未执行）。
- **定级：P1**（测试真实性 + 覆盖盲区，与 D5 验收目标直接相关）。D6 串联真实组件时需补一条"从含 Color 的 WidgetView → from_view → render_offscreen → 中心像素红色"的端到端测试。

### 1.3 【P2-wgpu 29 稳健性】两处可优化（非缺陷）
- **阻塞性 `device.poll(Waiter)`**（vello.rs:140）：在**主线程/渲染循环**中用 `PollType::Wait` 会阻塞——D6 接入 winit 事件循环（每帧调用）时，会卡住 UI 线程。**P2**：D6 应考虑用 `poll(Maintain::Poll)` 或把 readback 移出每帧关键路径（仅 D5 的离屏一次性验证无所谓，但 D6 连续渲染必须改）。
- **`Renderer` 持有 `Device`+`Queue` 但 `render_offscreen` 每次重建 texture/buffer**：D5 一次性可接受；D6 需要"复用 render pass + 纹理池"时需重构。**P2**（性能观察）。

---

## 二、Q2 — SceneGraph 纯度（PASS，达标）

### 2.1 纯度达标（总监重点问）
- `DrawCmd`（scene_graph.rs:11-38）**纯 Rust 枚举**：`FillRect{x,y,width,height,color:Color}` / `DrawText{x,y,text,size,color}`。`Color` 来自 `rgui_core::view`（core 层），**无 wgpu/vello 类型**。✓
- `SceneGraph`（scene_graph.rs:42-45）持有 `cmds: Vec<DrawCmd>`，公共 API `new/from_view/red_filled_rect/cmds/push` 全返回纯 Rust 类型。✓
- **有无遗漏图元类型**：当前只有 `FillRect` + `DrawText`，**缺 `Path`/`Image`/渐变等**——但 greenfield §B.2 的 `RenderBackend{Vello}` + `SceneGraph` 仅要求矩形/文本，D5 范围内**未遗漏**（Path 在 path_tessellation.rs 是占位，D6+ 补）。**P2**：`DrawText` 目前是"用矩形近似文本"（vello.rs:187-198），D5 最小占位，D6 需接入真实字形（cosmic-text）——D6 时注意补齐。

### 2.2 【P1-同族】`from_view` 转换是占位（固定矩形，未用布局）
- `from_view`/`collect`（scene_graph.rs:58-79）：
  - 只处理 `PropValue::Color` 的节点 → 画 **固定 100x40 矩形**（scene_graph.rs:68-73）——**尺寸硬编码**，未用 `LayoutResult`/布局。
  - `z 顺序`：垂直偏移 `y + 40.0 * (i+1)`（scene_graph.rs:77）——**简单堆叠，无真实布局**。
  - **未用** `rgui_core::layout::LayoutEngine` 的结果。
- **判定**：这不是"正确转换"，是"能画出的最小占位"。ga 清单 SG6（布局 rect 应用）明确"待 dev"，**未执行**。**P1（与 Q1 的 from_view 盲区同源）**：D5 的 WidgetView→SceneGraph 不是真实正确的转换，仅是占位。**放行 D6 须基于"from_view 是占位、需在 D6/组件集成时补全"的认知，而非假设它已正确。**
- 另注：`from_view` 泛型 `M` 未使用 `view.props` 之外任何信息，且 `PropValue::Color` 匹配是穷举了吗？`PropValue` 有 `Unit/Bool/Int/Float/Str/Color` 六变体（view.rs:38-52），`collect` 只 `if let Color`，**其它变体被静默忽略**（不画、不报错）。**P2**：D5 最小可接受，但 D6 若节点 props 是 `Str`（文本）应转 `DrawText` 而非被吞掉——当前文本节点在 from_view 下**完全不会生成 DrawText**（需等 D6 真实转换）。

---

## 三、Q3 — 防火墙（PASS，达标）

### 3.1 core 零 GPU（Cargo.toml + 源码双验证）
- `rgui-core/Cargo.toml [dependencies]`：仅 `taffy optional` + 无 wgpu/vello/cosmic/fontdb/skrifa（D4 已核）。
- core 源码：`grep wgpu|vello|cosmic|skia rgui-core/src` → 仅 lib.rs:8 注释（"不允许依赖...wgpu/winit/vello"）。**无真实引用**。✓
- `cargo tree -p rgui-core`：`taffy→{arrayvec,grid,slotmap}`，纯 Rust。✓

### 3.2 render 单一 vello，无 skia 残留
- `rgui-render/Cargo.toml`：`vello-backend` = `{vello,wgpu,cosmic-text,fontdb,skrifa,pollster}`（Cargo.toml:24-31），**无 `skia-backend`/`skia-safe`/`offscreen` feature**。✓
- render 源码：`grep skia` → 仅注释（lib.rs:8"删除 skia"、vello.rs:2"无 skia"）。**无残留。**✓
- `RenderBackend`（vello.rs:15-18）**仅 `Vello(VelloBackend)` 一个变体**——单一渲染路径，greenfield §B.2 达标。✓

### 3.3 依赖方向
`render/Cargo.toml:11` 仅 `rgui-core`（+ GPU feature 内依赖），无 platform。render 对 core 有真实引用（vello.rs:10 `use Color`、scene_graph.rs:7 `use Color/WidgetView`、lib.rs:42/48 `LayoutResult/Color`）——**依赖是真实的，非伪造**。✓（注：lib.rs:31-34 `_marks_core_dep` 那个 dummy 函数是冗余装饰，render 本就用 core，可删，非缺陷）

---

## 四、Q4 — unsafe 与安全（PASS）

- **全仓 `grep unsafe` rgui-render/src + rgui-core/src → 空**。**D5 离屏渲染/GPGPU 操作零 unsafe 块**——这是重要的正面信号，说明 wgpu/vello 的 safe API（`map_async`/`device.poll`/`create_texture` 等）已覆盖所有需要手动 `unsafe` 的 GPU 交互点。
- **对比 M1 教训**：老项目 `error_boundary.rs:57` 那处 `unsafe { &mut *(...) }` 绕 borrow checker 的写法，在 D5 新代码**完全没有重演**。✓
- 安全边界清晰：GPU 资源操作被限制在 `vello.rs` 一个模块内，其余 scene_graph/glyph/text 是纯 Rust 数据结构。

**达标，无 P0/P1。**

---

## 五、Q5 — 增量编译（P2，正向仍未验证）

### 5.1 反向（改 render → core 不重编）：**真防火墙方向，可验证** ✓
- core 不依赖 render（§3 已证），改 render 源码**必然不触发 core 编译**。qa 的 INC1（qa 清单 §6）测的就是这个方向，**逻辑成立**。✓

### 5.2 正向（改 core 数据层 → render 不重编）：**仍未验证 / 仍不可达（D4 遗留）**
- 这是 D4 审查的 P1-C2 遗留，**D5 未解决**。核心原因：`rgui-render/Cargo.toml:11` 依赖 `rgui-core` **整个 crate**，Cargo 增量粒度是 crate 级——改 core 任何部分（含数据层 `state`）都会重编 render。
- **D5 新增事实**：render 的 `RenderLayoutCache`（lib.rs:42-48）**持有 `rgui_core::layout::LayoutResult` 字段**（lib.rs:42）——这是 render 对 core 布局类型的**强值依赖**。若改 core `layout` 结构，render 必然重编。而 greenfield §E.3 的"改数据层不重编 render"要成立，需把数据层（state/纯类型）从 core 物理拆出——**D5 没做，仍不可达**。
- **判定**：**Positive（改 core→render）增量验证在 D5 仍不是可测指标**。qa 清单只锁 INC1（反向），未锁正向（qa 清单 §6 只列 INC1）——这**有意规避**了不可达的正向验证，但会导致"D5 增量编译验收"只测了反向，遗漏了 GREENFIELD 更看重的"改数据层不重编 render"。**P2**：需总监决定是接受"改 core 即重编 render"（放弃正向隔离，理由：data 层与 render 同属一个重编单元，D6+ 组件复杂时收益有限），还是将数据层物理拆 crate（回退到多 crate，与 5-crate 收敛矛盾）。

---

## 六、Q6 — 进入 D6（窗口+事件循环）前需处理的风险

### 6.1 【P1-核心观察】核心循环未真正串联（render 是"孤岛证明模块"）
- 现状：`VelloBackend::render_offscreen` 能画出 `red_filled_rect`，但：
  - **没有**任何代码调用 `SceneGraph::from_view`（占位，`#[allow(dead_code)]`）。
  - **没有**把 `VelloBackend` 接入真实组件循环（Coordinator/StateStore 在 core，render 在 render crate，二者只在 facade 的 `render.rs`/`render_coord.rs` 里各自持 `Option<SceneGraph>` 占位，**未接通**）。
- **风险**：D6 要接 winit 事件循环，第一件事就是"用户的 WidgetSpec 组件 → view → WidgetView → from_view → SceneGraph → render_offscreen → 窗口 surface"。**这条链路目前的 `from_view` 是固定矩形占位**，D6 若不重写 `from_view`，任何真实组件（哪怕一个按钮）都会渲染成 100x40 红色块。
- **定级：P1（D6 前置认知）**。不是 D5 缺陷，但必须在进入 D6 前明确"**from_view + 真实布局（LayoutEngine）是 D6 的核心工作量**"，否则 D6 会低估工作量。

### 6.2 【P1-同源】`from_view` 的 Props→图元映射缺失
现状：`collect`（scene_graph.rs:64-79）只处理 `PropValue::Color`，**`Str`/`Int`/`Bool`/`Float` 静默忽略**，文本节点不产生 `DrawText`。D6 组件（如按钮 label 是 `PropValue::Str`）会"有内容但不渲染"。**P1**（与 6.1 同源：D6 前必须补 `from_view` 的完整 Props→DrawCmd 映射）。

### 6.3 【P2】D5 遗留清理项
1. `default` feature 偏差：qa 上报的 core `default=["layout"]` vs greenfield §D `default=[]`（D5 验收清单 §5 FW 区已注，QA 上报 P2）。taffy 纯 Rust 不违反防火墙语义，但违反"default 空、重型隔离"原则。**建议 D6 拍板**：core `default=[]`（taffy 经依赖 enable）或接受现状。
2. `device.poll(wait)` 阻塞（vello.rs:140）——D6 接入每帧渲染需改 `poll(Maintain::Poll)` 或移出关键路径。
3. `_marks_core_dep` 冗余 dummy（lib.rs:31-34）——可删。
4. `from_view` 的 `#[allow(dead_code)]`（scene_graph.rs:57）——D6 接入后应移除，并补测试。
5. `DrawText` 用矩形近似（vello.rs:187-198）——D6 接 cosmic-text 真实字形时替换。

---

## 七、P0/P1 风险清单 + MERGE GATE 建议

### P0 风险清单：**无（P0 清零）**

### P1 风险清单（2 项，D6 前认知/闭合）
| # | 风险 | 位置 | 处置建议 |
|---|---|---|---|
| P1-1 | **离屏测试绕过真实转换**：像素断言只验 `red_filled_rect`，未验 `from_view`（`#[allow(dead_code)]` 占位，QA SG1-7 全未执行） | offscreen.rs:17 / scene_graph.rs:58 | D6 端到端补一条"WidgetView(Color)→from_view→render_offscreen→中心红"测试；并把 `#[allow(dead_code)]` 移除 |
| P1-2 | **from_view 转换是占位**：固定 100x40 矩形、未用 LayoutEngine、`Str/Int` 等 props 静默忽略 | scene_graph.rs:58-79 | 进入 D6 前明确这是核心工作量；D6 重写 `from_view`（用布局结果 + 完整 Props→DrawCmd 映射），否则真实组件全渲染成固定色块 |

### P2 观察清单（随 D6 处理，不阻塞放行）
1. core `default=["layout"]` vs greenfield `default=[]`（qa 已上报）。
2. `device.poll(wait)` 阻塞（D6 每帧需改）。
3. 增量正向（改 core→render）不可达——D4 遗留，需总监裁量是否接受"改 core 即重编 render"。
4. `DrawText` 矩形近似 / `_marks_core_dep` 冗余 / `from_view` allow(dead_code) 清理。

### MERGE GATE 建议：**有条件放行（CONDITIONAL PASS）**

- **放行理由（充分）**：离屏硬件路径（GPU 同步/readback/资源管理）逐行审读无缺陷；SceneGraph 纯 Rust 不泄漏 GPU 类型；core 防火墙零 GPU；全仓零 unsafe；render 单一 vello 无 skia 残留——**P0 清零，D5 的核心工程（能画）真实成立**。
- **条件（2 项 P1 转入 D6 待办，不伪造"已完成"）**：from_view 是占位、离屏测试绕过转换——**D6 接真实组件时必须重写 from_view 并补端到端测试**。请总监在 D6 派单时显式把"WidgetView→SceneGraph 真实转换 + 布局应用 + 文本路径"列入范围，不要当成 D5 已完成的"正确 SceneGraph 转换"。
- **一句话**：D5 把"能用 vello 画出来"证明到位，离屏同步/防火墙/unsafe 克制都做对了；但它还没证明"从用户组件到像素"的正确性——那是 D6 的核心，且当前 from_view 是占位。放行 D6，但务必知道 D6 的重头是真实转换。

---

*审查方：devco-reviewer｜只读审查（未运行 GPU 测试，离屏像素以 qa 环境依赖为准）；GPU  readback 流程经逐行审读无同步/泄漏缺陷；性能类观察（poll 阻塞、纹理复用）为 D6 前瞻。*
