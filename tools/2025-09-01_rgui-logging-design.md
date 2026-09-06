# rgui 生产级日志方案 · 技术细化设计

> 工作目录：`/Users/chenchao/Documents/code/rust/RUST-GUI`
> 设计方：devco-architect（方案设计师）　阶段：日志方案细化（供总监整合给用户审）
> 只做设计，不写实现代码。
> 基线约束（已核实）：edition=2021、MSRV=1.85、resolver=2、无 async 运行时（无 tokio/async-std）、winit 同步回调、5-crate（core/render/platform/macros/facade）。

---

## 0. 现状核实结论（设计前提）

实测（非假设）：
- **现有日志**：`eprintln!` 仅 **2 处**（`rgui/src/app.rs:190`、`205`）。总监说的 18 处应含 demo 里的 `print!/println!/eprintln!`（demo 源码在 `tools/qa/` 之外的构建产物，`window_demo`/`d20_modal` 是 `rgui` 的 demo 二进制），以及 D21 自动化脚本消费的 `[hit]`/`[action]`/`[focus]` 等分层信号——这些信号**实际是 demo 里打出的确定性文本日志**，被 `tools/qa/rgui_input_test.py` 的 `detect_layer` 用正则解析。
- **Cargo.lock 已有**：`log`、`tracing`、`tracing-core`（当前为**间接依赖**，非 rgui 直接声明；来源推测为 wgpu/vello 生态）。
- **D21 脚本消费的信号 token**（正则精确匹配，改格式必须同步适配）：
  - `[mouse-event] left-press at logical=(x,y) in-region=true|false`
  - `[focus] Tab(shift=bool) -> Some(N)|None(N)`、`[focus] click -> Some(N)`
  - `[hit] id=N`、`[hit] id=none`、`[hit] id=N -> XxxMsg::Yyy`
  - `[action] toggle(id=N)`、`[action] badge_click(id=N,count=N)`、`[action] modal_open`
  - `[hit-region]`、`rect=(x,y,w,h)`、`origin=(x,y)`、`size=(w,h)`、`scale=N`

---

## 1. 日志库选型（tracing vs log）

### 1.1 结论：**tracing**（主）+ `log` 桥（兼容既有生态）

**理由**：用户要"功能丰富稳定"，tracing 是 Rust 生态事实标准，满足结构化/span/级别/采样/可接 OTel 全部诉求；且 `tracing` 通过 `log` feature 与 `log` 生态完全互操作（wgpu/vello 内部用 `log`，可统一收口）。

### 1.2 MSRV 1.85 兼容性（已核实）

| crate | 版本（推荐） | MSRV | 结论 |
|---|---|---|---|
| `tracing` | **0.1.41**（最新 0.1.x） | 1.49+（宽松） | ✅ 1.85 远超 |
| `tracing-subscriber` | **0.3.20**（2025-08-29 最新） | **1.63.0**（0.3.18 起） | ✅ 1.85 兼容 |
| `tracing-core` | 0.1.33 | 随 tracing | ✅ |
| `tracing-appender`（可选，非滚动文件） | 0.2.3 | 1.63+ | 可选 |
| `log` | 0.4.x | 1.31+ | ✅ |
| `nu-ansi-term` | 0.46（ansi 彩色） | 1.60+ | ✅ |

> 版本选择依据：`tracing-subscriber 0.3.18` 把 MSRV 提到 1.63.0，0.3.20 为当前最新（修 ANSI 转义）。选 0.3.20 即可，1.85 无压力。**无任何 async 运行时要求**。

### 1.3 与「无 async 运行时」契合度（关键结论）

**tracing 完全不强制 async。** 澄清一个常见误解：

- `tracing` 是**同步日志门面**，`tracing::info!`/`trace!` 等宏在同步代码中直接调用，无需任何 executor。
- `tracing-subscriber` 的 `fmt::Layer` 是**同步 subscriber**，在调用线程内联格式化+写入，无后台线程、无 async。
- "异步/采样/OTel" 是 tracing 的**可选能力**（`tracing-futures`、`tracing-opentelemetry` 等独立 crate），**本项目不引入**。

**契合 winit 同步回调**：winit 事件循环是同步 `EventLoop::run` 回调，`tracing::info!` 在回调内同步打印，天然契合，无 `Send`/`await` 障碍。

### 1.4 需要的 feature（贴合同步 + 最小集）

```toml
# rgui 直接依赖（facade 或 workspace.dependencies）
tracing = { version = "0.1.41", default-features = false, features = ["std"] }
tracing-subscriber = { version = "0.3.20", default-features = false,
                       features = ["fmt", "ansi", "env-filter", "registry"] }
log = { version = "0.4", features = ["std"] }   # 桥：收 wgpu/vello 的 log! 宏
```

**feature 说明**：
- `tracing`：`std`（非 no_std）+ **关闭 `log` feature**（避免 `log` 双重依赖；若需桥接则开 `log`）——本设计**开 `tracing/log`** 以便 wgpu/vello 的 `log!` 统一收口。
- `tracing-subscriber`：`fmt`（格式化输出）+ `ansi`（终端彩色）+ `env-filter`（RUST_LOG 环境变量过滤）+ `registry`（layer 组合必需）。**不选**：`json`（阶段 1 不需要）、`chrono`（时间戳用 std，避免拉 chrono 重依赖）、`smallvec`、`parking_lot`。
- **明确不引入**：`tracing-futures`、`tracing-opentelemetry`、`tracing-appender`（阶段 1 用 stderr/stdout + 外部重定向即可，不做滚动文件）。

---

## 2. 库日志点位清单（rgui-*/src 关键路径）

**原则**：只在关键路径埋点，**不每帧**（流式编码铁律）。级别约定：trace=极细、debug=排障、info=关键生命周期、warn=可恢复异常、error=不可恢复/渲染失败。

| 模块 | 位置（建议） | 级别 | 埋点内容 |
|---|---|---|---|
| `rgui/src/app.rs` | App 启动/退出 | info | `app_start`（version/feature）、`app_shutdown` |
| | 渲染错误（原 eprintln! 205 处） | error | `render_error`（含错误上下文，替代 eprintln!） |
| | 帧协调（**不每帧**，仅状态切换） | debug | `frame_coord`（首帧/重建/暂停） |
| `rgui/src/event_loop.rs` | 窗口创建/关闭 | info | `window_created`、`window_closed` |
| | 事件分发（**仅异常路径**，不每条事件） | debug | `event_dispatch`（未处理事件类型） |
| `rgui-render` | vello 后端初始化 | info | `vello_init`（backend/device/adapter） |
| | 字形 Atlas 重建（**低频**） | info | `glyph_atlas_rebuild`（容量/增长） |
| | 渲染耗时（**仅超阈值**，不每帧） | warn | `render_slow`（>16ms 告警） |
| `rgui-core::layout` | 布局根脏（**仅 dirty 子树**，不每帧） | debug | `layout_dirty`（脏节点数） |
| `rgui-platform::focus` | 焦点变更 | debug | `focus_changed`（WidgetId 迁移） |
| `rgui-core::style` | 样式解析失败 | warn | `style_parse_warn`（行号/原因） |
| `rgui-render::scene_graph` | 场景图节点数（**仅重建时**） | debug | `scene_node_count` |
| hit-test（interaction） | **不埋库日志**（见 §3，测试信号单独走） | — | 命中测试是每帧热路径，库日志不碰 |

**关键边界**：hit-test / 每帧渲染 / 每事件分发 **不做库日志**，避免风暴（§5）。原 `eprintln!` 2 处全部替换为 `tracing::error!`。

---

## 3. 测试信号日志承载（D21 detect_layer 迁移）

### 3.1 核心设计：**独立 target `rgui_test_signal` + 固定结构化字段**

测试信号与库日志**物理分离**，用 tracing 的 `target` 机制隔离：

```rust
// demo 里（非库），输出确定性测试信号：
tracing::info!(
    target: "rgui_test_signal",
    "[hit] id={id}", id = widget_id
);
tracing::info!(
    target: "rgui_test_signal",
    "[action] toggle(id={id})", id = widget_id
);
```

### 3.2 输出格式契约（保证脚本可解析）

用 `tracing-subscriber` 的 `fmt::Layer` 定制 **只对 `rgui_test_signal` target 输出"纯净格式"**：

```
# 测试信号输出格式（无级别前缀/无时间戳/无 target，纯裸文本，脚本正则零改动或最小改动）
[mouse-event] left-press at logical=(170,22) in-region=true
[hit] id=1
[action] toggle(id=1)
[focus] Tab(shift=false) -> Some(1)
```

**实现要点（设计层，非代码）**：
- 测试信号 target 的 `fmt::Layer` 配置 `with_target(false)` + `with_level(false)` + 自定义格式，**只输出 message 字段**（message 内已含 `[hit] id=1` 等确定性 token）。
- 这样 D21 脚本的 `re.search(r'\[hit\] id=1', ...)` 等正则 **保持原样不变**（因为 message 内容格式与现在 eprintln 的文本完全一致）。

### 3.3 为何用 target 而非独立 crate

- **target 零成本**：无需新增 crate，`tracing::info!(target: "rgui_test_signal", ...)` 一个参数即可。
- **隔离干净**：`EnvFilter` 可按 target 精确开关（`RUST_LOG=rgui_test_signal=info` 只看测试信号；`RUST_LOG=rgui=info` 只看库日志）。
- **脚本适配最小**：脚本仍抓 stderr/stdout 文本，正则不变（§6）。

### 3.4 迁移清单（demo 信号 token → tracing target）

| 现有 token | 迁移后（target=`rgui_test_signal`） | 级别 |
|---|---|---|
| `[mouse-event] left-press at logical=(x,y) in-region=b` | 同 message 文本 | info |
| `[hit] id=N` / `id=none` | 同 | info |
| `[hit] id=N -> XxxMsg::Yyy` | 同 | info |
| `[action] toggle(id=N)` | 同 | info |
| `[action] badge_click(id=N,count=N)` | 同 | info |
| `[action] modal_open/close` | 同 | info |
| `[focus] Tab(shift=b) -> Some(N)` | 同 | info |
| `[hit-region]` + `rect=(...)` + `origin=(...)` + `size=(...)` + `scale=...` | 同 | debug（或 info，脚本 pre-check 消费） |

> **信号级别统一 `info`**（保证 RUST_LOG 默认 info 下可见），`[hit-region]` 系列因脚本 `parse_hit_region` 消费，也置 info。

---

## 4. 日志分层（库日志 vs 测试信号隔离）

### 4.1 双 target 隔离

| 类别 | target | 用途 | 级别 | 污染风险 |
|---|---|---|---|---|
| 库日志 | `rgui`、`rgui_core`、`rgui_render`、`rgui_platform` | 开发者排障 | debug/info/warn/error | 不含 `[hit]`/`[action]` 等信号 token，不污染脚本 |
| 测试信号 | `rgui_test_signal` | D21 脚本输入 | info | 仅含确定性 token，无时间戳/级别前缀 |

### 4.2 环境变量开关

```
RUST_LOG=rgui=info,rgui_core=debug,rgui_render=warn,rgui_platform=info   # 库日志
RUST_LOG=rgui_test_signal=info                                            # 测试信号
RUST_LOG=off                                                               # 全关（性能）
```

**关键设计**：
- 测试信号与库日志用**两个独立 EnvFilter 规则**，互不干扰。跑 D21 时 `RUST_LOG=rgui_test_signal=info`（只出信号，脚本干净）；日常排障 `RUST_LOG=rgui=debug`（不出信号）。
- **默认（无 RUST_LOG）**：`LevelFilter::INFO`，库日志 info 可见、测试信号 info 可见（demo 默认打开信号）。

### 4.3 输出流隔离（可选增强）

- 库日志 → **stderr**（`fmt::Layer::with_writer(std::io::stderr)`）。
- 测试信号 → **stdout**（单独 `fmt::Layer` 用 `std::io::stdout`，只 filter `rgui_test_signal` target）。
- 这样脚本可 `--log <file>` 只重定向 stdout 收信号，stderr 留给排障，**物理隔离零污染**。

---

## 5. 性能 / 风暴防护

### 5.1 惰性跳过（tracing 零成本机制）

- `tracing::trace!`/`debug!` 在 `LevelFilter` 关闭时**不构造参数**（宏内 `if enabled` 短路），帧路径零开销。
- **关键热路径（hit-test/每帧渲染）绝不埋 `info!` 及以上级别**，最多 `trace!`（默认关闭）。

### 5.2 过滤策略

| 场景 | 配置 | 效果 |
|---|---|---|
| 生产发布 | `RUST_LOG=warn`（默认） | 只出 warn/error，帧路径零开销 |
| 排障 | `RUST_LOG=rgui=debug` | debug 级排障 |
| 全关（性能基准） | `RUST_LOG=off` | 完全零开销 |
| D21 自动化 | `RUST_LOG=rgui_test_signal=info` | 只出测试信号 |

### 5.3 节流（防风暴）

- **不引入 rate-limiter 依赖**（过度设计）。靠"埋点只在低频路径"（§2）+ "LevelFilter 惰性跳过"天然防风暴。
- `render_slow` 超阈值告警已带节流语义（仅 >16ms 才打，且每帧最多一次）。
- 若未来需采样：`tracing` 支持 `sample` rate（`tracing::subscriber::filter::LevelFilter` + `sampler`），但**阶段 1 不做**（克制）。

---

## 6. 文档 / 脚本适配（D21 detect_layer 正则方向）

### 6.1 结论：**脚本正则零改动**（最优路径）

因 §3.2 采用"测试信号输出纯净 message 文本（含原 token）"，D21 脚本的 `re.search(r'\[hit\] id=1', ...)` 等**全部正则保持不变**。

### 6.2 若无法做到"纯 message 输出"（备选适配方向）

若 tracing `fmt` 无法完全去掉级别/时间戳前缀，脚本只需在 `detect_layer` 入口加一行**去前缀清洗**：

```python
# 备选：strip tracing 前缀，只保留 [token] 行
import re
def _strip_tracing(line: str) -> str:
    # 去掉 "2025-...  INFO rgui_test_signal: " 之类前缀
    m = re.match(r'^(?:\S+\s+)?\S+\s+(\[.*)$', line)
    return m.group(1) if m else line
# 在 detect_layer 内对每行先 _strip_tracing 再匹配现有正则
```

> **方向**：优先做 §6.1（零改动）；退路是 §6.2（脚本加一个 strip 函数，改动 ≤3 行，集中在 `rgui_input_test.py` 顶部）。

### 6.3 脚本需要适配的其他点

- `parse_hit_region` 消费的 `[hit-region]`、`rect=`、`origin=`、`size=`、`scale=` 若走 tracing，message 文本格式不变，正则不变。
- 唯一需确认：脚本当前从 `/tmp/rgui_demo.log` 读，日志落文件方式改为"stdout 重定向"后，脚本 `--log` 参数指向同一文件即可，无需改脚本读取逻辑。

---

## 7. 风险与规避

| # | 风险 | 影响 | 规避 |
|---|---|---|---|
| 1 | tracing 依赖新增 → 编译时间增加 | 中（tracing-subscriber 有较多依赖） | 只开 `fmt/ansi/env-filter/registry` 四 feature，不拉 `json/chrono/smallvec`；tracing 本身极轻 |
| 2 | MSRV 超限 | 低 | 已核实：tracing-subscriber 0.3.20 的 MSRV=1.63 < 1.85 ✅ |
| 3 | 与无 async 运行时冲突 | **无** | tracing 是同步门面，不强制 async；已澄清 §1.3 |
| 4 | winit 同步回调 + 日志阻塞 | 低 | `fmt::Layer` 同步内联写 stderr，事件循环内日志量极小，无阻塞风险 |
| 5 | 测试信号格式漂移 → D21 脚本失效 | **高**（核心风险） | §3.2 强制"message 文本 = 原 token 不变"，§6.1 零改动；加 CI 断言（T1-T7 全绿）作回归门禁 |
| 6 | 帧路径日志风暴 | 中 | §2 只埋低频路径 + §5 LevelFilter 惰性跳过 + 默认 warn |
| 7 | `log` 桥与 `tracing` 双重日志（wgpu 内部） | 低 | `tracing` 开 `log` feature，`tracing_subscriber` 注册 `LogTracer`，wgpu/vello 的 `log!` 统一收口到 tracing，无重复 |
| 8 | demo 是 `rgui` 的 bin 还是独立？ | 中（定位问题） | 需确认 demo（window_demo/d20_modal）编译产物归属；若为 `rgui` 内 `examples/` 或 bin target，日志依赖挂在 `rgui`；若独立 bin，依赖挂各自 Cargo.toml |

---

## 8. 待总监/用户确认的开放项

1. **demo 归属**：`window_demo`/`d20_modal` 是 `rgui` 的 example/bin 还是独立 crate？影响日志依赖挂载点（§7 风险 8）。
2. **测试信号输出流**：采用"stdout 信号 + stderr 库日志"物理隔离（§4.3），还是"统一 stderr"（简单但脚本需 strip）？**建议前者**。
3. **是否引入 tracing-appender 滚动文件**：阶段 1 建议不引入（用外部重定向），阶段 2 再议。
4. **默认日志级别**：生产默认 `warn`，demo 默认 `info`（含信号）。是否接受？

---

## 9. 交付验收清单（设计侧）

- [ ] tracing + tracing-subscriber（0.1.41 / 0.3.20）纳入 workspace.dependencies
- [ ] MSRV 1.85 下 `cargo check --workspace` 通过
- [ ] 原 2 处 `eprintln!` 替换为 `tracing::error!`
- [ ] 库日志埋点完成（§2 清单），热路径无 info+ 埋点
- [ ] 测试信号迁移到 `target="rgui_test_signal"`，message 文本 = 原 token
- [ ] D21 T1-T7 全绿（脚本正则零改动或仅 strip 适配）
- [ ] `RUST_LOG=off` 下帧路径零日志开销（性能基准不回归）
- [ ] `cargo clippy -- -D warnings`（仅 unsafe_code=deny 前提下）通过

> 本设计为技术细化方案，不含 Rust 实现代码。全程只读核实，未改动任何现有 Rust 源文件 / Cargo.toml / .git。
