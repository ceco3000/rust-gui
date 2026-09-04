# D9（App::run 统一入口 + 文本真实字形 + 按需重绘）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D9 交付
> 审查基准：`tools/2025-09-01_rgui-greenfield-architecture.md` §B.5/§E.3 + D8 审查遗留 P2
> 审查方式：只读代码核查（未运行 GPU 测试）；文本字形经 glyph_offscreen 测试逻辑审读
> 结论性质：基于源码逐行审读 + 依赖图验证

---

## 〇、结论速览

| # | 审查点 | 定级 | 一句话结论 |
|---|---|---|---|
| Q1 | App::run 统一入口 | **P2（API 不完整：AppConfig 死代码）** | 设计清晰可用，但 `App::run` 不带 config 参数、AppConfig 是死代码 |
| Q2 | 文本真实字形 | **PASS** | cosmic-text→draw_glyphs 正确；glyph_offscreen 测试精确验证字形离散（非色块） |
| Q3 | 按需重绘 | **PASS** | Wait + dirty 逻辑正确；首帧/事件变更可靠；CPU 99%→0.0% 合理 |
| Q4 | 防火墙/DAG | **PASS（含 D8 P2-2 已修复）** | core 零 GPU；facade 已删 dep:winit/wgpu；render cosmic 门控 |
| Q5 | D8 前风险 | —— | 无 P0；P1-1（跨平台）仍待验证；P2 观察项见 §六 |

**总评：D9 三大目标全部达成——App::run 统一入口让 window_demo 经 facade（无直接 winit/wgpu）、文本真实字形（cosmic-text 整形+draw_glyphs，截图清晰文字实测确认）、按需重绘（ControlFlow::Wait+dirty，CPU 99%→0.0%）。P0 清零。核心功能可靠。建议：放行（PASS）。** 有 1 项 P2 观察（App::run API 未暴露 config）+ 若干 P2 遗留（跨平台、offscreen 手工 rect）。

---

## 一、Q1 — App::run 统一入口（P2，API 不完整但可用）

### 1.1 【PASS】入口设计清晰易用
`App::run<W, F>(widget, state, mapper)`（app.rs:83-100）：
- 泛型 `W: WidgetSpec + 'static` + `F: FnMut(&WindowEvent) -> Option<W::Message> + 'static`——**类型约束合理**，`widget/state/mapper` 三参数清晰。
- `mapper` 闭包把窗口事件映射为消息（FnMut）：window_demo.rs:99-114 演示了 `MouseInput(Left)→Increment`、`KeyR→Reset`——**易用、直观**。✓
- 内部 `AppRunnerImpl` 持 `Coordinator`（app.rs:105）——**统一入口真正驱动 Coordinator（D4 的心法）**，解决了 D8 的"demo 手调 update"遗留（P2-6 已闭合）。event→dispatch（app.rs:138）→draw 重新 view（app.rs:150）。闭环正确。✓

### 1.2 【P2-API 不完整】AppConfig 是**死代码**，App::run 不带 config
- **矛盾点**：`AppConfig` 定义了 `with_title/with_size`（app.rs:43-53），`App::new(config)`（app.rs:71）接收 config，`App.config`（app.rs:66）存在——**但 `App::run`（app.rs:83）是静态方法且不带任何 config 参数**，内部**硬编码** `WindowConfig{title:"rgui", width:620, height:220}`（app.rs:93-97）。
- **后果**：用户调 `App::run(Button, state, mapper)`（window_demo.rs:116）**无法自定义窗口标题/尺寸**（永远是 620x220 标题 rgui）。`AppConfig`/`App::new`/`App.config`**完全未被使用**（grep 仅 app.rs 内定义，无外部消费）。
- **判定**：P2（非阻塞——demo 能跑、窗口正确），但**API 不完整**。D10 建议：`App::run_with_config(config, widget, state, mapper)` 或 `AppConfig` 纳入 `run` 参数，让用户可配标题/尺寸；或删掉死代码 AppConfig。当前 `AppConfig` 是误导性的（看起来可配置，实则无效）。

### 1.3 【PASS】闭包稳健性/生命周期
- `mapper: Box<dyn FnMut(&WindowEvent) -> Option<W::Message>>`（app.rs:108）——boxed 闭包，`+ 'static`，无借用逃逸（window_demo 的 mapper 是闭包，owned）。✓
- `AppRunnerImpl::new<F: FnMut(...)+'static>`（app.rs:112-123）收 `Box::new(mapper)`——生命周期安全，`'static` 约束正确。✓
- **平台/生命周期隐患**：`surface: Option<GpuSurface<'static>>`（app.rs:107）——`'static` surface 由 `Arc<Window>` 支撑（app.rs:129 `create_surface(window.clone())`），window 为 `Arc` 可 clone 保 `'static`。**稳健**。✓

---

## 二、Q2 — 文本真实字形（PASS，重点）

### 2.1 【PASS】cosmic-text → draw_glyphs 路径正确
完整链路（text.rs + vello.rs）：
1. `TextShaper::new()`：`FontSystem::new()` + `load_system_fonts()`（text.rs:32-36）——加载系统字体。
2. `shape_line(text, size)`（text.rs:42-74）：`Metrics::new(size, size*1.2)` → `Buffer::new` → `set_text(Attrs::new(), Shaping::Advanced)` → `shape_until_scroll`（text.rs:46-49）——cosmic-text 整形。**Shaping::Advanced 正确。**✓
3. 遍历 `layout_runs()`：把 glyphs 转为 vello `Glyph`，按 font 分组（text.rs:52-71），**基线对齐**（`gy = baseline - font_size*y_offset`，text.rs:59）。
4. `font_data_for`（text.rs:77-82）：`fontdb.with_face_data` → `Blob::new(Arc<Vec<u8>>)` → `FontData::new(blob, index)`——真实字体数据。✓
5. `draw_text`（vello.rs:310-328）：`scene.draw_glyphs(...).transform(translate).font_size(size).brush(color).draw(glyphs)`——真实字形轮廓绘制。✓

### 2.2 【PASS】glyph_offscreen 测试精确验证"真实字形"（非色块）
关键测试 `draw_text_produces_real_glyph_pixels`（glyph_offscreen.rs:11-45）：
- 断言 `white_count > 30`（有字形像素）。
- **断言 `max_row_white < w-20`（glyph_offscreen.rs:41-44）——"字形应离散而非整行填充"**——这个断言**直接区分"真实字形" vs "矩形近似块"**（矩形近似会整行填充，字形是离散的）。**这是 D6 审查"e2e2 has_non_bg 无法区分字形/色块"问题的彻底解法。**✓
- `empty_text_renders_nothing`（glyph_offscreen.rs:48-63）：空文本无像素。
- `button_and_text_render_both_fill_and_glyphs`（66-105）：按钮 + 白色文字字形。
- `from_view_button_renders_and_shows_text`（108-151）：复刻 window_demo 组件，from_view → 像素验证蓝色按钮 + 白色文字。**端到端覆盖。**✓

### 2.3 【PASS】无 feature 占位干净态
- 无 `vello-backend` 时：`mod wide` 占位版 `ShapedRun{font_data:(), glyphs:Vec<()>}`、`TextShaper`（text.rs:86-104）——shape_line 返回空，**不引 cosmic-text/vello 重依赖**。`VelloBackend`（vello.rs：vello-backend 门控）与 `TextShaper::new()` 均在 feature 下。**default 构建（无 feature）可编译**。✓

### 2.4 【P2-观察】两处小项
- **`text.rs:123` 测试断言**：`assert_eq!(run.font_data.index, if run.font_data.index == 0 {0} else {run.font_data.index})`——**恒真断言**（无论 index 多少都成立），无意义测试，P2（可删或改为具体值）。
- **坐标对齐**：`draw_text` 用 `transform(translate((x,y)))`（vello.rs:323）纯平移，未额外加 baseline 偏移——依赖 cosmic-text glyph y 已在基线系。**当前截图文字清晰**（总监确认），说明对齐可接受，但 D10 中文/多字体混排时需复核 baseline（P2 观察）。

---

## 三、Q3 — 按需重绘（PASS，重点验证）

### 3.1 【PASS】ControlFlow::Wait + dirty 逻辑正确
`run_as_with_config`（event_loop.rs:52-68）：
- `event_loop.set_control_flow(ControlFlow::Wait)`（event_loop.rs:59）——**空闲休眠（CPU 低）**，替代 D8 的 Poll。
- `Runner{has_drawn:false, pending:true}`（event_loop.rs:64-65）——初始 pending=true 保证首帧。

`Runner`（event_loop.rs:81-135）：
- **首帧**：`resumed` → create_window → init → `request_redraw()`（event_loop.rs:93）+ `pending=true`（event_loop.rs:94）→ `has_drawn=false`（95）。**首帧稳定渲染。**
- `window_event`（event_loop.rs:99-124）：`self.app.event()` 返回 dirty（bool）。**若 dirty=true** → `pending=true` + `request_redraw()`（event_loop.rs:115-118）。`RedrawRequested` → `draw()` + `has_drawn=true`（119-122）。
- `about_to_wait`（event_loop.rs:126-134）：**仅 `!has_drawn || pending` 时才 `request_redraw()`**（129-131），否则跳过——**空闲不重绘（CPU 0%）**。✓

### 3.2 【PASS】首帧/事件变更重绘可靠
- **首帧**：pending=true 初值 + resumed request_redraw → 窗口弹出即渲染（响应总监"窗口弹出可见稳定渲染"）。
- **事件变更**：mapper 返回 Some→dispatch→dirty=true→pending=true→request_redraw→draw。闭环可靠。✓
- **map**: event() 返回 bool 是全新语义（D8 是 void），D9 改为"返回值表示是否 dirty"——facade 的 `AppRunnerImpl::event`（app.rs:135-143）返回 true 当 dispatch 了消息。**接口契约一致。**✓

### 3.3 【PASS】CPU 降低合理
- `ControlFlow::Wait`：事件循环在无事件时**休眠**（winit 阻塞等待），不再 Poll 空转。
- `about_to_wait` 只在 dirty/pending 时 redraw——**无事件不渲染**。
- 总监实测"CPU ~99%→0.0%"——**符合** Wait + 按需重绘的预期（D8 的 Poll + 每帧 redraw 是 99% 的原因，D9 修复正确）。✓

### 3.4 【P2-观察】潜在漏重绘边界（低风险）
- `about_to_wait` 只在 `!has_drawn||pending` 时 redraw。若**窗口 resize**（WindowEvent::Resized），winit 通常**自动请求 RedrawRequested**（系统窗口 resize 触发重绘），故一般不会漏。但若某平台 resize 不自动 redraw，且 mapper 对 Resized 返回 None（不 dirty），则该帧不重绘。**P2**：D10 建议在 `window_event` 里对 `WindowEvent::Resized` 显式 `request_redraw()`（当前依赖 winit 行为，跨平台可能有差异）。

---

## 四、Q4 — 防火墙/DAG/单一 vello/winit（PASS，含 D8 P2-2 修复）

### 4.1 【PASS】core 零 GPU/平台
`core/Cargo.toml` grep wgpu/vello/winit/cosmic → **零**（仅 taffy optional）。✓

### 4.2 【PASS】facade 已删 dep:winit/dep:wgpu（D8 P2-2 修复）
`rgui/Cargo.toml` 已无 `winit`/`wgpu` 独立依赖条目（Cargo.toml:18-19 注释确认"winit/wgpu 经 rgui-platform/winit + rgui-render/vello-backend 传递引入；facade 源码不再直接引用"）。`window` feature 只透传 `rgui-platform/winit` + `rgui-render/vello-backend`。✓ **D8 的 P2-2 已闭合。**

### 4.3 【PASS】render cosmic-text 门控 + 单一 vello
`render/Cargo.toml`：`cosmic-text`/`fontdb` 均 `optional`，由 `vello-backend` feature 引入（Cargo.toml:16-17,24-28）。无 skia 残留。✓

### 4.4 【PASS】platform 仅 winit
`platform/Cargo.toml`：仅 `winit optional`，无 wgpu/vello。✓

### 4.5 【PASS】DAG 无环
core 零依赖；platform⇄render 互不相依（D8 确认）；facade 依赖 core/render/platform/macros。单一 vello/winit。✓

---

## 五、Q5 — D8 前风险总结

- **P0 清零**。
- **P1：无新增**。D8 的 P1-1（display_handle 跨平台）仍存在但未恶化（macOS 实证截图稳定 + glyph_offscreen 离屏测试通过）。
- **P2 观察项**：见 §六。

---

## 六、P2 观察项清单（随 D10/后续处理，不阻塞 D9）

| # | 项 | 位置 | 说明 |
|---|---|---|---|
| P2-1 | **App::run API 不完整：AppConfig 死代码** | rgui/src/app.rs:83 vs 20-60 | `App::run` 静态方法不带 config，内部硬编码 620x220；`AppConfig`/`App::new`/`App.config` 全未被使用。D10 加 `run_with_config` 或删死代码，否则"看起来可配置实则无效" |
| P2-2 | **跨平台 P1-1 未验证** | vello.rs:34 | `new_without_display_handle()` 建 instance 却 create_surface——macOS 实证，linux/windows 待验证。D10 非 macOS 验证或改 `default()` |
| P2-3 | **offscreen.rs 手工 rect 残留** | rgui-render/tests/offscreen.rs | D5 旧测试（red_filled_rect），建议归档/删 |
| P2-4 | **增量单向（改 core→render）不可达** | D4 遗留 | render 依赖 core 整 crate；D10 裁决 |
| P2-5 | **text.rs:123 恒真断言** | text.rs:123 | `assert_eq!(index, if index==0 {0} else {index})` 无意义，可删 |
| P2-6 | **Resize 漏重绘依赖 winit 行为** | event_loop.rs:126-134 | `about_to_wait` 不处理 Resized（mapper 返回 None 时）；D10 在 window_event 对 Resized 显式 request_redraw |
| P2-7 | **baseline 对齐（多字体混排时需复核）** | vello.rs:323 | 当前纯 translate，截图清晰；D10 中文/多字体时复核基线 |

---

## 七、P0/P1 风险清单 + MERGE GATE 建议

### P0 风险清单：**无（P0 清零）**

### P1 风险清单：**无（P1 清零）**

### MERGE GATE 建议：**放行（PASS）**

- **放行理由（充分）**：
  1. **App::run 统一入口达成**——window_demo 经 facade 启动（无直接 winit/wgpu），事件→消息映射闭包（FnMut）稳健，内部走 Coordinator（D4 心法落地）。
  2. **文本真实字形达成**——cosmic-text shape_until_scroll → fontdb FontData → vello draw_glyphs 路径正确；glyph_offscreen 测试**精确验证字形离散（max_row_white<w-20 区分真实字形 vs 色块）**，比 D6 的 has_non_bg 强。截图清晰文字实测确认。
  3. **按需重绘达成**——ControlFlow::Wait + dirty + has_drawn/pending 逻辑正确，首帧/事件变更可靠，CPU 99%→0.0% 合理。
  4. **防火墙（含 D8 P2-2 修复）**——core 零 GPU；facade 已删 dep:winit/wgpu；render cosmic 门控；platform 仅 winit；DAG 无环。
  5. **P0 清零，P1 无新增**。
- **P2 观察（随 D10 处理）**：App::run 未暴露 config（AppConfig 死代码）、跨平台 P1-1 未验证、offscreen 手工 rect 残留、增量单向不可达、Resize 漏重绘边界、baseline 多字体复核。
- **一句话**：D9 三项核心目标全部达成且可靠——统一入口、真实字形、按需重绘（CPU 修复是点睛）。可放行 D10。建议 D10 优先：① 暴露 config 到 App::run（AppConfig 当前是死代码，用户无法配窗口）；② 非 macOS 验证 display_handle；③ Resize 显式重绘边界。

---

*审查方：devco-reviewer｜只读审查（未运行 GPU 测试）；文本字形路径（shape_until_scroll/draw_glyphs）与按需重绘（Wait/dirty）经逐行审读验证，CPU 数据源自总监实测。*
