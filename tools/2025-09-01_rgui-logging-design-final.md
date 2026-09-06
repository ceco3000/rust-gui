# rgui 生产级日志方案 · 最终设计（结合开源实践调研）

> 设计方：devco-director（总监整合）+ devco-architect（技术细化）
> 阶段：完整方案（结合网络调研的开源最佳实践），供用户审批
> 调研方式：浏览器（Google Chrome CDP）搜索 Rust 日志最佳实践 + 开源项目做法 + Tokio 官方 tracing 文档
> 只做设计，不含实现代码。

---

## 〇、网络调研结论（开源实践依据）

### 0.1 选型共识（Rust 社区权威来源）

| 来源 | 结论 |
|---|---|
| **Blessed.rs**（社区 crate 权威清单）| "**Tracing is now the go-to crate for logging**"；log 是"简单/无 async 时的替代"，有结构化需求选 tracing |
| **Shuttle.dev《Logging in Rust》** | 对比：**有 async/请求上下文/生产可观测需求 → tracing**；简单/无 async → log |
| **Tokio 官方 tracing 指南** | tracing 是框架，收集**结构化、事件式**诊断；span 有开始/结束时间、可嵌套（表达时序/因果）；event 是单点事件；**span/event 都结构化**（typed data + message）|
| **Rustify《tracing vs log》** | 有请求上下文/生产可观测 → tracing；简单 → log |

### 0.2 开源项目通用实践（搜索汇总）

1. **库用接口、bin 用后端**：库 crate 只 `use tracing::info!`（不绑 subscriber）；由**可执行文件（demo/bin）在最早期**注册 `tracing_subscriber`（Tokio 原话："register your tracing subscriber as early as possible in main"）。
2. **Level 约定**：`trace`=极细（每帧/每事件）、`debug`=排障、`info`=关键生命周期、`warn`=可恢复异常、`error`=不可恢复。**热路径（每帧/每事件）最多 `trace`（默认关）**。
3. **LevelFilter/EnvFilter 惰性跳过**：tracing 宏在 filter 关闭时**不构造参数**（零成本），这是防风暴/性能的基石。
4. **span 只在需要关联上下文时用**（请求/帧/query 级），否则用轻量 event——避免过度 span 开销。
5. **subscriber 尽早 + Layer 可组合**：`FmtSubscriber`/`fmt()` 输出格式化；Layer 可组合（多输出流/多格式）；`with_thread_ids`、`with_target(false)` 等定制输出。
6. **backpressure/采样**：做采样（sample rate）选 tracing；`tracing-subscriber` 支持 filter + 采样；无 async 也能用同步 subscriber。

> **关键适配**：Tokio 指南强调 tracing 用于**异步系统**的并发任务关联；本项目 **winit 是同步回调**，无 async。因此项目**只用到 tracing 的同步面**（`tracing::info!` + 同步 `fmt::Layer`），**不需要** tokio executor / tracing-futures / async span——这也规避了"无 async 运行时"的顾虑。

---

## 一、日志库选型（最终）

### 1.1 结论：**tracing（主）+ log 桥**

**理由（调研 + 项目特性）**：
- 用户要"功能丰富稳定" → tracing 是 Rust 事实标准（Blessed.rs / Shuttle 共识），满足结构化/span/级别/采样/可接 OTel。
- 项目核心（winit 同步回调、无 async）→ 用 tracing **同步面**即可，无需 executor/async feature。
- wgpu/vello 内部用 `log!` → 开 `tracing/log` feature + `LogTracer`，**统一收口**（无双重日志）。

### 1.2 版本与 MSRV（已核实，无超限）

| crate | 版本 | MSRV | 结论 |
|---|---|---|---|
| `tracing` | 0.1.41 | 1.49+ | ✅ 1.85 远超 |
| `tracing-subscriber` | 0.3.20 | **1.63** | ✅ 1.85 兼容（0.3.18 起 MR 1.63）|
| `tracing-core` | 0.1.33 | 随 tracing | ✅ |
| `log` | 0.4.x | 1.31+ | ✅ |

### 1.3 需要的 feature（贴合同步 + 最小集）

```toml
# workspace.dependencies
tracing = { version = "0.1.41", default-features = false, features = ["std", "log"] }
tracing-subscriber = { version = "0.3.20", default-features = false,
                       features = ["fmt", "ansi", "env-filter", "registry"] }
log = { version = "0.4" }   # 桥：收 wgpu/vello 的 log!
```

**显式不引入**：async/otlp/futures/rolling-file（阶段 2 再议）。

---

## 二、日志分层（核心：库日志 vs 测试信号物理隔离）

### 2.1 双 target + 双输出流（调研实践 + 项目需测试信号）

| 类别 | target | 输出流 | 级别 | 用途 |
|---|---|---|---|---|
| **库日志** | `rgui`/`rgui_core`/`rgui_render`/`rgui_platform` | **stderr** | debug/info/warn/error | 开发者排障 |
| **测试信号** | `rgui_test_signal` | **stdout** | info | D21 `detect_layer` 脚本输入 |

> **物理隔离**（调研实践）使脚本可 `--log <file>` 只重定向 stdout 收测试信号，stderr 留排障，零污染。

### 2.2 环境变量开关（EnvFilter）

```
RUST_LOG=rgui_test_signal=info     → 只出测试信号（跑 D21，脚本干净）
RUST_LOG=ngui=debug                → 只出库日志（排障，无信号污染）
RUST_LOG=ngui=info,ngui_core=debug → 组合
RUST_LOG=off                       → 全关（性能基准）
默认（无 RUST_LOG）：LevelFilter::INFO（库 info + 信号 info 可见）
```

---

## 三、测试信号承载（③ 保证可解析 + 脚本少改动）

demo 信号迁到 `tracing::info!(target: "rgui_test_signal", "[hit] id={id}", id=...)`，**message 文本 = 原 token 不变**。用 `fmt::Layer` + `with_target(false)/with_level(false)` 输出**纯裸文本**（无级别/时间戳/前缀）→ **D21 脚本正则零改动**。

- **最优**：`fmt::Layer` 对 `rgui_test_signal` target 输出纯净 message → 脚本正则不变。
- **退路**（若 tracing fmt 去不掉前缀）：脚本 `detect_layer` 加 ≤3 行 `_strip_tracing()` 清洗。

> 调研实践强调 span 用于关联；但**测试信号是确定性单点事件，用轻量 event 而非 span**，避免 span 结构化字段破坏脚本正则。

---

## 四、库日志埋点（只在关键/低频路径，流式编码铁律）

| 位置 | 级别 | 内容 |
|---|---|---|
| app.rs App 启动/退出 | info | `app_start`/`app_shutdown` |
| 渲染错误（原 eprintln! 205）| error | `render_error`（含上下文）|
| event_loop 窗口创建/关闭 | info | `window_created`/`window_closed` |
| render vello 初始化/Atlas 重建 | info | `vello_init`/`glyph_atlas_rebuild` |
| 渲染耗时超阈值 | warn | `render_slow`（>16ms，节流，每帧最多一次）|
| layout 脏子树 | debug | `layout_dirty` |
| focus 变更 | debug | `focus_changed` |
| style 解析失败 | warn | `style_parse_warn` |
| scene_graph 重建 | debug | `scene_node_count` |

**边界铁律**：hit-test / 每帧渲染 / 每事件分发 **不埋库日志**（调研实践：热路径最多 trace 默认关）。原 2 处 `eprintln!` 替换（app.rs:205→`tracing::error!`；app.rs:190 win-frame→迁 `rgui_test_signal`）。

---

## 五、性能 / 风暴防护（调研实践）

1. **惰性跳过**：tracing 宏在 filter 关时**不构造参数**（帧路径零开销）。
2. **热路径不埋 info+**（最多 trace，默认关）。
3. **默认 warn** 生产 + 只埋低频路径 → 天然无风暴。
4. **backpressure/采样**：阶段 1 不引入 rate-limiter（克制），靠埋点位置 + LevelFilter 惰性足够；未来需采样用 `tracing-subscriber` filter+sampler。
5. **`render_slow` 节流语义**：仅 >16ms 且每帧最多一次。

---

## 六、文档 / 脚本适配

- **脚本正则零改动**（最优路径，message=原 token）。
- 退路：`_strip_tracing()` ≤3 行，集中在 `rgui_input_test.py`。
- `parse_hit_region` 消费的 `[hit-region]`/`rect=`/`origin=`/`size=`/`scale=` 走 tracing 后 message 格式不变。

---

## 七、风险与规避

| # | 风险 | 级别 | 规避 |
|---|---|---|---|
| 1 | tracing-subscriber 增加编译时间 | 中 | 只开 fmt/ansi/env-filter/registry；tracing 本身轻 |
| 2 | MSRV 超限 | 低 | 已核实 1.63<1.85 |
| 3 | 无 async 冲突 | 无 | 只用同步面（Tokio 指南适配）|
| 4 | **测试信号格式漂移 → D21 脚本失效** | **高** | message=原 token 不变 + T1-T7 全绿 CI 门禁 |
| 5 | 帧路径日志风暴 | 中 | 热路径不埋 + LevelFilter 惰性 + 默认 warn |
| 6 | log 桥双重日志 | 低 | log feature + LogTracer 统一收口 |
| 7 | 测试信号被 span 字段污染 | 中 | 测试信号用轻量 event（不用 span），输出纯 message |

---

## 八、交付验收清单

- [ ] tracing + subscriber 纳入 workspace.dependencies；MSRV 1.85 下 `cargo check --workspace` 过
- [ ] 原 2 处 eprintln! 替换（render_error→tracing::error!；win-frame→rgui_test_signal）
- [ ] 库日志埋点完成（§四），热路径无 info+ 埋点
- [ ] 测试信号迁 target="rgui_test_signal"，message=原 token
- [ ] **D21 T1-T7 全绿（脚本正则零改动或仅 ≤3 行 strip 适配）**
- [ ] `RUST_LOG=off` 帧路径零日志开销（性能基准）
- [ ] `cargo clippy -- -D warnings`（保留 unsafe_code=deny）过
- [ ] 文档同步（D5/D9/tasks.md 库日志 vs 测试信号分层）+ 每阶段提交铁律

---

> 本方案为最终设计（结合网络调研的开源实践），不含 Rust 实现代码。
> 调研通过浏览器（Google Chrome CDP）完成：Blessed.rs / Shuttle.dev / Tokio 官方 tracing / Rustify 等。
