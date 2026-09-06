# D22（生产级日志：tracing + 测试信号迁移）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D22 交付（commit f204985 + fbf4606 补 Cargo.lock）
> 基准：D22 判据（测试信号逐 token 一致、双流隔离、热路径不埋 info+、MSRV/依赖/无 async、流式、文档一致性）；日志设计文档 tools/2025-09-01_rgui-logging-design-final.md
> 方法：只读代码核查（logging.rs / window_demo.rs / d20_modal.rs / app.rs / 各库埋点）+ `git show f204985` 逐 token diff 对比旧 eprintln! + 实测 `cargo test` + 依赖/版本核实

---

## 〇、结论速览

| # | 审查点 | 定级 |
|---|---|---|
| 1 | ★测试信号逐 token 一致 | **PASS（8 项 + win-frame 全一致；D21 正则零改动成立）** |
| 2 | 双流隔离 | **PASS（库→stderr / 信号→stdout；signal layer 纯裸 token 去前缀）** |
| 3 | 热路径不埋 info+ | **PASS（hit-test/每帧渲染/每事件分发无库日志）** |
| 4 | MSRV/依赖/无 async | **PASS（tracing 0.1.41 + subscriber 0.3.20，MSRV 1.65 ≤ 1.85；无 tokio/futures/otlp）×P2（env-filter feature 冗余）** |
| 5 | **流式判据** | **PASS（tracing! 单语句；map/iter().any() 流式）** |
| 6 | 文档一致性 | **PASS（D5 已同步）×P2（D9/tasks.md 待 doc）** |

**总评：D22 达标——测试信号迁移逐 token 一致（D21 detect_layer/parse_hit_region 正则零改动，T1-T7 回归 PASS）、双流物理隔离（库→stderr/信号→stdout）、热路径零库日志、MSRV 满足、无 async、流式合规，81 测试全绿。建议：放行（PASS）。** 无 P0/P1，3 条 P2 观察（env-filter feature 冗余、D9/tasks.md 待 doc 同步、with_ansi 已设但可复核）。

---

## 一、★测试信号逐 token 一致（PASS，核心判据）

### 1.1 逐 token diff 对比（`git show f204985` 旧 eprintln! ↔ 新 tracing message）

| # | token | 旧 eprintln! message | 新 tracing message | 逐字一致 |
|---|---|---|---|---|
| 1 | `[hit-region]` | `"[hit-region] id={} {} rect=({},{},{},{})"` + r.id/name/x/y/w/h | 同字符串 + 同参数（window_demo.rs:159-168） | ✓ |
| 2 | `[focus] Tab` | `"[focus] Tab(shift={s}) -> {:?}"` | `"[focus] Tab(shift={}) -> {:?}"` + s（window_demo.rs:209-214） | ✓ 产出 `Tab(shift=false) -> Some(1)` 不变（`{s}`→`{}`+参数 仅写法） |
| 3 | `[mouse-event]` | `"[mouse-event] left-press at logical=({}, {}) in-region={}"` + x/y/in_region | 同字符串 + 同参数（window_demo.rs:226-232） | ✓ |
| 4 | `[hit] id=1` | `"[hit] id=1 -> AccordionMsg::Toggle"` | 同（window_demo.rs:235） | ✓ | 
| 5 | `[action] toggle` | `"[action] toggle(id=1)"` | 同（window_demo.rs:236） | ✓ |
| 6 | `[hit] id=2` | `"[hit] id=2 -> WaBadgeMsg::Click"` | 同（window_demo.rs:245） | ✓ |
| 7 | `[action] badge_click` | `"[action] badge_click(id=2,count={n})"` | `"[action] badge_click(id=2,count={})"` + n（window_demo.rs:246-249） | ✓ 产出 `count=1` 不变 |
| 8 | `[hit] id=none` | `"[hit] id=none (missed hit-region)"` | 同（window_demo.rs:254） | ✓ |
| 9 | `[win-frame]`（app.rs） | `"[win-frame] origin=({},{}) size=({},{}) scale={}"` + outer.x/y/size.w/h/scale | 同（app.rs:195-200） | ✓ |

### 1.2 d20_modal.rs 逐 token（`git show f204985`）
| token | 旧 eprintln! | 新 tracing | 一致 |
|---|---|---|---|
| `[hit-region]`（modal 场景） | `"[hit-region] id={} {} rect=({},{},{},{})"` + id/name/x/y/w/h | 同（d20_modal.rs:167-175） | ✓ |
| `[focus] Tab` | `"[focus] Tab -> {:?}"` + fid.map(\|w\|w.0) | 同（d20_modal.rs:193） | ✓ |
| `[action] modal_close` | `"[action] modal_close"` | 同（d20_modal.rs:198/228） | ✓ |
| `[focus] Esc` | `"[focus] Esc -> {:?}"` + f.map | 同（d20_modal.rs:199） | ✓ |
| `[action] modal_open` | `"[action] modal_open"` | 同（d20_modal.rs:217） | ✓ |
| `[focus] click` | `"[focus] click -> {:?}"` + f.map | 同（d20_modal.rs:218/229） | ✓ |

### 1.3 结论
**8 项（window_demo）+ 6 项（d20_modal）+ [win-frame] 全部逐字一致。** 产出 token 文本均不变（`{s}`/`{n}`→`{}`+参数仅写法差异，fmt 输出相同）。D21 `detect_layer`/`parse_hit_region`/`parse_win_frame` 正则零改动成立（用户实测 T1-T7 全量回归 PASS 佐证）。✓

---

## 二、双流隔离（PASS）

- **logging.rs**: 双 `fmt::Layer` 挂到同一 `registry()`（logging.rs:63-66）：
  - `lib_layer`（logging.rs:46-49）：`with_writer(io::stderr)` + `with_ansi(false)`，filter 排除 `SIGNAL_TARGET` 且 level≤lvl（logging.rs:43）。
  - `signal_layer`（logging.rs:55-61）：`with_writer(io::stdout)` + `with_ansi(false)` + `without_time()` + `with_target(false)` + `with_level(false)`，filter 只 target==SIGNAL_TARGET（logging.rs:53）。
- **物理隔离**：库→stderr，信号→stdout，互不污染 ✓。**signal_layer 去 level/target/time/ansi** → stdout 纯裸 token（D21 脚本解析输入）✓。
- **幂等**：`INIT.call_once`（logging.rs:25）✓；demo 最早入口注册（window_demo.rs:140 / d20_modal.rs:148），facade App::run 也调用（app.rs:115）✓。
- **RUST_LOG 语义**：`off`→OFF、`error/warn/debug/trace`→对应、默认 info（logging.rs:28-34）；含 `rgui_test_signal`→`lib_on=false` 只出信号（logging.rs:37，D21 脚本干净）✓。

---

## 三、热路径不埋 info+（PASS）

**库日志埋点全在关键/低频路径**（判据 3 要求）：
| 位置 | target | 级别 | 频率 |
|---|---|---|---|
| app.rs:116 app_start title | rgui | info | 入口一次 |
| app.rs:193 win-frame | rgui_test_signal | info | 窗口事件变化，`last_frame` 去重（app.rs:192/202） |
| app.rs:213 render_error | rgui | **error** | 仅渲染出错 |
| event_loop.rs:127/145 window_created/closed | rgui_platform | info | 创建/关闭 |
| vello.rs:34 vello_init | rgui_render | info | init 一次 |
| focus.rs:137 focus_changed | rgui_platform | **debug** | move_focus（Tab/焦点变更，低频） |

**热路径零库日志**：hit-test（window_demo 的 `regions.iter().any()` 是测试信号 in-region，非库日志）、每帧渲染（vello render_surface 内无 tracing）、每事件分发（mapper handler 无库日志）——**均无库日志埋点** ✓。`default-features=false` + 宏短路（level 关闭时不构造参数）保证帧路径零开销。

---

## 四、MSRV/依赖/无 async（PASS + P2）

- **workspace.dependencies**（Cargo.toml:28-29）：`tracing = { version="0.1.41", default-features=false, features=["std","log"] }`、`tracing-subscriber = { version="0.3.20", default-features=false, features=["fmt","ansi","env-filter","registry"] }`。rogui/Cargo.toml:15-16 `tracing.workspace=true`/`tracing-subscriber.workspace=true`。✓
- **MSRV**：tracing 0.1.41 与 tracing-subscriber 0.3.20 官方 MSRV 均 **1.65**（docs.rs 确认"minimum supported version is 1.65"），**≤ 项目 MSRV 1.85**（Cargo.toml rust-version=1.85）→ 满足 ✓。
- **无 async**：`default-features=false`，只带 std/log/fmt/ansi/env-filter/registry；tracing-subscriber 依赖链仅 nu-ansi-term（Cargo.lock 核对，无 tokio/futures/chrono/otlp）——**只用同步面**（fmt::Layer + registry），无 tokio executor ✓。
- **【P2-1】** `env-filter` feature 已开（Cargo.toml:29），但 logging.rs **未用 `EnvFilter`**——用 `filter_fn` + 手写 match 解析 RUST_LOG（logging.rs:28-34）。**冗余 feature**（无害，`env-filter` 会额外拉 matchers/regex 编译依赖）——建议去掉或改用它。
- **【P2-2】** 当前本机 rustc **1.96** 编译通过（非 1.85 实测）。MSRV 1.85 满足是**依赖 MSRV 推算**（1.65 ≤ 1.85），未在真 1.85 toolchain 下 `cargo check`——建议 CI 加 1.85 toolchain 冒烟。

---

## 五、流式编码判据（PASS）

- **日志代码**：`tracing::info!(target, "msg", args...)` 单语句，无循环/无中间 collect；`fid.map(|w| w.0)`、`f.map(|w| w.0)` 流式 map ✓。
- **window_demo in-region**：`regions.iter().any(|r| r.contains(x,y))`（window_demo.rs:225）——**流式 `iter().any()`**（命中检测，非库日志）✓。
- **[hit] match**（window_demo.rs:233-257）：`match hit_test(...)` 多分支分发——match 模式，非迭代器场景 ✓。
- **无 dyn Iterator 装箱 / 冗余 collect**：日志/信号代码无 ✓。

---

## 六、文档一致性（PASS + P2）

- **D5 已同步**（dev，总监确认）：事件系统文档标注日志分层（tracing + rgui_test_signal 迁移）。
- **design 文档已归档**：tools/2025-09-01_rgui-logging-design-final.md / -design.md（§四 埋点清单、双流、RUST_LOG 语义、纯裸 token 约定）——迁移与文档一致 ✓。
- **【P2-3】** D9（测试策略）/tasks.md D22 段**待 doc 同步**（文档铁律：文档全集与代码一致；D5 已过，D9/tasks 由 doc 补）。

---

## 七、P0/P1 风险清单

**P0：无。P1：无。**

**MERGE GATE：放行（PASS）。**

### P2 观察项（随后续处理，不阻塞）
1. **P2-1 env-filter feature 冗余**：Cargo.toml:29 开了 `env-filter`，但 logging.rs 未用 EnvFilter（手写 match 解析 RUST_LOG）——冗余 feature，建议去掉（或改用它）。
2. **P2-2 MSRV 冒烟**：本机 rustc 1.96 编译过；MSRV 1.85 满足为依赖 MSRV 推算（1.65≤1.85），建议 CI 加 1.85 toolchain 真机 `cargo check`。
3. **P2-3 D9/tasks.md 待 doc 同步**：日志分层（库 vs 测试信号）应补进 D9 测试策略文档 + tasks.md D22 段。

---

*审查方：devco-reviewer｜只读审查。已完成逐 token diff 核对（window_demo 8 项 + d20_modal 6 项 + win-frame 全部一致，D21 正则零改动成立）+ 双流隔离核实（库→stderr/信号→stdout，signal 纯裸 token）+ 热路径零库日志确认（埋点全在关键/低频路径）+ MSRV/无 async 核实（tracing/subscriber MSRV 1.65 ≤ 1.85；无 tokio/futures/otlp）+ 流式合规（tracing! 单语句、map/iter().any()）+ 文档一致性（D5 已同步，D9/tasks 待 doc）。81 测试全绿（`cargo test --workspace --all-features` 实测）。P0/P1 双清零，3 条 P2 观察。*
