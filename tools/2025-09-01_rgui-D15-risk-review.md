# D15（scale_factor 物理→逻辑坐标换算）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D15 交付（commit 36badd8）
> 基准：greenfield §B.3、D5、既有判据 + **流式编码判据**
> 范围：①scale_factor 正确性 ②坐标层归属 ③流式判据 ④防火墙 ⑤文档一致性
> 方法：只读代码核查（window.rs/app.rs/window_demo.rs 逐行）+ 实测 `cargo test --workspace --all-features`

---

## 〇、结论速览

| # | 审查点 | 定级 |
|---|---|---|
| 1 | scale_factor 正确性 | **PASS（to_logical 除换算 + thread_local 注入正确）** |
| 2 | 坐标层归属 | **PASS（scale_factor 在 platform/window 层；core 零引用收逻辑坐标）** |
| 3 | **流式判据** | **PASS（to_logical 纯表达式；platform_scale get；无 dyn Iterator/冗余 collect）** |
| 4 | 防火墙/DAG | **PASS（scale_factor 在 platform+winit；core 收逻辑坐标）×1 P2 观察（渲染物理/逻辑混用）** |
| 5 | 文档一致性 | **PASS（D5 已同步）** |

**总评：D15 达标——scale_factor 物理→逻辑坐标换算（to_logical ÷scale）、platform_scale thread_local 每事件注入、window_demo CursorMoved→to_logical→hit_test 全部正确，流式合规，63 测试全绿（实测），P0 清零。建议：放行（PASS）。** 无 P0/P1（1 条 P2 观察：渲染层 surface 尺寸用物理像素 vs SceneGraph 逻辑坐标，见 §二.3）。

---

## 一、scale_factor 正确性（PASS）

### 1.1 to_logical（window.rs:90-93）
```rust
pub fn to_logical(physical: (f64, f64), scale: f64) -> (f32, f32) {
    let s = if scale > 0.0 { scale } else { 1.0 };
    ((physical.0 / s) as f32, (physical.1 / s) as f32)
}
```
- **物理 ÷ scale = 逻辑**——正确。
- **防除零/非法**：`if scale > 0.0 { scale } else { 1.0 }`（window.rs:91）——scale<=0 用 1.0 兜底，防 panic。✓
- **测试**：`to_logical_divides_by_scale`（window.rs:105-110，Retina scale=2：物理(200,100)→逻辑(100,50)）、`to_logical_identity_at_scale_one`（113-117，scale=1 恒等）——**覆盖正确**。✓

### 1.2 platform_scale thread_local（window.rs:74-87）
- `thread_local! PLATFORM_SCALE: Cell<f64>`（window.rs:75-77，默认 1.0）；`set_platform_scale`（80-82）/`platform_scale`（85-87）——**thread_local 可变全局（当前线程），供 mapper 读**。
- **注入链路**：`AppRunnerImpl::event` 里 `set_platform_scale(window.scale_factor())`（app.rs:150）——**每个窗口事件先注入 scale，再调 mapper**（app.rs:151）。✓
- **时序正确**：winit 事件循环主线程，`event()` 先 set 后 mapper，mapper 读 `platform_scale()`（window_demo.rs:151）拿到最新值。✓
- **测试**：`platform_scale_defaults_to_one_and_can_set`（window.rs:120-125）：默认 1.0、set 2.0 读 2.0、复位 1.0——覆盖。✓

### 1.3 window_scale
- `window_scale(window)`（window.rs:96-98）：`window.scale_factor()`——winit 取窗口 scale。✓（未在 demo 直接用，demo 走 platform_scale thread_local，统一。）

---

## 二、坐标层归属（PASS + 1 P2 观察）

### 2.1 scale_factor 在 platform/window 层（winit 隔离），正确
- `to_logical`/`platform_scale`/`set_platform_scale`/`window_scale`（window.rs:73-98）——**全在 rgui-platform**（winit 隔离处）。✓
- core 零引用 scale_factor（grep `scale_factor/to_logical/platform_scale` in rgui-core/src → 空）——**core 零平台边界成立**。✓
- `hit_test`/`HitRegion`（core hit_test.rs）用**逻辑坐标**（window_demo.rs:132-135 regions 是逻辑 Rect）——core 收逻辑坐标。✓

### 2.2 demo 链路正确
- window_demo.rs:147-153：`CursorMoved{position}`（物理像素）→ `to_logical(position, platform_scale())` → `(lx,ly)` 缓存 → `MouseInput` 时 `hit_test(lx,ly)`（window_demo.rs:179-180）——**物理→逻辑→hit-test** 全链路换算正确。✓
- 修复了 D12 的 P2（"cursor 物理坐标直接当逻辑坐标"问题）——**D15 已闭合 D12 的 scale_factor P2 观察**。✓

### 2.3 【P2-观察】渲染层 surface 尺寸（物理像素）vs SceneGraph 坐标（逻辑）混用
- `AppRunnerImpl::draw`（app.rs:164-169）：`let size = window.inner_size()`（**物理像素**）→ `render_surface(surface, &graph, size.width, size.height)` → `render_to_view(graph, &off_view, width, height)`（vello.rs:254，物理尺寸）→ vello 渲染。但 `SceneGraph` 的 draw 指令坐标是**逻辑坐标**（`from_view` 用组件逻辑 size，如 Accordion `343x...`/WaBadge `160x40` 逻辑）。
- **问题**：Retina 高分屏（scale=2）时，`inner_size()` 返回 2x 物理像素（如 1040x440），surface 配置为 1040x440，但 SceneGraph 组件坐标仍是逻辑（520 逻辑宽）——**内容在 2x surface 上只绘制逻辑区域（占一半尺寸），会缩小/偏移**。
- **当前 demo 状态**：截图（1x 逻辑显示）可能未暴露；hit_test 已走 to_logical 对齐（✓），但**渲染尺寸未做物理/逻辑统一**（draw 直接用 inner_size 物理 + 逻辑 scene 坐标）。
- **判定**：P2（非 D15 计算错误，属渲染管线物理/逻辑坐标统一问题）。**D17 布局/渲染时需统一**：要么 render 用逻辑尺寸再×scale，要么 from_view 输出物理坐标，或 render_surface 接受逻辑尺寸。**不阻塞 D15**（hit_test 换算正确），列入 D17 观察。

---

## 三、流式编码判据（PASS）

### 3.1 合规项
| 判据 | 检查结果 |
|---|---|
| **to_logical 纯表达式** | `(physical.0/s, physical.1/s)`——标量算术表达式，无迭代/收集 ✓ |
| **platform_scale get** | `PLATFORM_SCALE.with(|c| c.get())`——单值读取，无循环/收集 ✓ |
| **`dyn Iterator` 装箱** | 无 `dyn Iterator`。app.rs 的 `Box<dyn std::error::Error>`（错误类型）/`Box<dyn FnMut>`(mapper trait object) 是**错误/闭包 trait object，非 iterator 装箱**——不违反流式判据 ✓ |
| **冗余中间 collect** | 无冗余 collect ✓ |

### 3.2 边界
- `to_logical`/`platform_scale` 都是纯标量值处理，非迭代器场景——无"能组合子却循环"问题。✓

**结论：流式编码判据 PASS。** to_logical 纯表达式、platform_scale 单值 get，无装箱（Box<dyn FnMut/Error> 非 iterator）/冗余 collect。

---

## 四、防火墙 / DAG（PASS）

- **scale_factor 在 platform（winit）**：window.rs（rgui-platform）含 winit/scale 逻辑；core 零 scale_factor 引用。✓
- **core 收逻辑坐标**：hit_test/HitRegion 用逻辑 rect（window_demo.rs 传入逻辑）；core 无 scale_factor/物理坐标概念。✓
- **core 零 GPU**：无 wgpu/vello/winit；**单一 vello/winit**、DAG 无环——保持。✓

---

## 五、文档一致性（PASS）

- **D5 已同步 scale_factor 实现**（总监确认 commit 36badd8 含 D5 更新）——to_logical/platform_scale/window_demo 换算与代码一致。
- **greenfield/D1 由 doc 同步中**——需注意：scale_factor（物理→逻辑）在 D5 事件系统补记（cursor 逻辑坐标）。P2 观察。

---

## 六、P0/P1 风险清单

**P0：无。P1：无。**

**MERGE GATE：放行（PASS）。**

### P2 观察项（随后续处理，不阻塞）
1. **渲染层物理/逻辑尺寸混用**（app.rs:164 inner_size 物理 + vello.rs:254 + SceneGraph 逻辑坐标）：Retina 高分屏下内容尺寸不匹配（缩小/偏移）——D17 布局/渲染时需统一（render 用逻辑尺寸×scale 或 from_view 输出物理坐标）。非 D15 错误（hit_test 换算已正确），P2。
2. **greenfield/D1 需标注 scale_factor**：D5 事件系统 cursor 逻辑坐标（D15 扩展），doc 同步时补记。

---

*审查方：devco-reviewer｜只读审查（未运行 GPU 窗口测试，63 全量数字经 `cargo test --workspace --all-features` 实测核实 = 63 passed）。流式编码判据逐条核对：to_logical 纯表达式、platform_scale 单值 get、无 docker Iterator 装箱/冗余 collect；scale_factor 在 platform（winit）、core 收逻辑坐标、防火墙/DAG 达标。*
