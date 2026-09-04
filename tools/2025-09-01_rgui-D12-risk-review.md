# D12（FocusManager 焦点管理 + WidgetSpec::focusable + demo Tab 切换）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D12 交付（commit 705a83d）
> 基准：greenfield §B.3、D5 事件系统、既有判据 + **流式编码判据**
> 范围：①FocusManager 正确性 ②focusable 默认方法 ③demo Tab 导航 ④流式编码 ⑤防火墙 ⑥文档一致性
> 方法：只读代码核查（focus.rs/traits.rs/components.rs/window_demo.rs 逐行）+ 实测 `cargo test --workspace --all-features`

---

## 〇、结论速览

| # | 审查点 | 定级 |
|---|---|---|
| 1 | FocusManager 正确性 | **PASS（5 单测，循环回绕正确）+P2 边界观察** |
| 2 | WidgetSpec::focusable 默认方法 | **PASS（向后兼容，组件覆盖 true）** |
| 3 | demo Tab 导航 | **PASS** |
| 4 | **流式编码判据** | **PASS（move_focus 用 iter().position().map().rem_euclid() 全流式；无 dyn Iterator/冗余 collect）** |
| 5 | 防火墙/DAG | **PASS（focus 在 platform；focusable 在 core 纯 Rust）** |
| 6 | 文档一致性 | **PASS（D5 已同步）** |

**总评：D12 达标——FocusManager 焦点管理（Tab/Shift+Tab 循环切换）、WidgetSpec::focusable 向后兼容扩展、demo Tab 导航全部正确，流式编码贯彻到位，58 测试全绿（实测），P0 清零。建议：放行（PASS）。** 无 P0/P1，仅 2 条 P2 观察（详见 §五）。

---

## 一、FocusManager 正确性（PASS）

### 1.1 move_focus：iter().position + rem_euclid 循环回绕（正确）
`move_focus(dir)`（focus.rs:71-90）：
```rust
Some(c) => self.focusable.iter().position(|&x| x == c)
    .map(|i| self.focusable[((i as i32 + dir).rem_euclid(n)) as usize]),
None => Some(if dir > 0 { self.focusable[0] } else { self.focusable[self.focusable.len()-1] }),
```
- **有焦点**：`position()` 找当前 id 索引 i → `(i + dir).rem_euclid(n)` 回绕到新索引。`rem_euclid` 对**负 dir** 正确处理（focus_prev 用 dir=-1，`(-1).rem_euclid(3)=2`，从 index0 回绕到末尾——正确）。✓
- **无焦点**：dir>0 取 `focusable[0]`（第一个），dir<0 取 `focusable[last]`（末尾）——**符合"无焦点获焦第一个/上一个"语义**。✓
- **空列表**：`focusable.is_empty()` 提前 return None（focus.rs:72-74）。✓

### 1.2 focus_next / focus_prev（正确）
- `focus_next`（focus.rs:61-63）→ `move_focus(1)`；`focus_prev`（focus.rs:66-68）→ `move_focus(-1)`。✓
- 循环切换（末位↔首位）：`focus_next` 从 index2 → `(2+1).rem_euclid(3)=0` → `focusable[0]`（首位回绕）；`focus_prev` 从 index0 → `(0-1).rem_euclid(3)=2` → `focusable[2]`（首位回绕到末位）。✓

### 1.3 set_focus 拒绝非可获焦（正确）
`set_focus`（focus.rs:36-43）：`if self.focusable.contains(&widget_id)` → 设焦点返回 true；否则返回 false 且**不设焦点**（focus.rs:37-42）。✓

### 1.4 测试覆盖（5 单测，focus.rs:101-141）
- `focus_next_cycles_forward`（102-110）：1→2→3→回绕1，验证循环 + is_focused。✓
- `focus_prev_cycles_backward`（113-119）：2→1→回绕3。✓
- `focus_next_with_no_focus_takes_first`（122-126）：无焦点 → 获焦第一个。✓
- `set_focus_rejects_non_focusable`（129-134）：set_focus(99) 失败，focus 保持 None。✓
- `focus_next_on_empty_focusable_returns_none`（137-140）：空列表 → None。✓
- **覆盖**：循环前向/回绕、循环后向/回绕、无焦点取首个、拒绝非可获焦、空列表——**全面**。✓

### 1.5 【P2-观察】focused 残留（边界）
- 若 `focused` 指向的 id **不在 focusable 列表**（比如组件被移除后仍记着焦点），`move_focus` 的 `position()` 返回 None → `.map()` 返回 None → `?` 提前 return None，**但 `focused` 字段保持原残留值（未清除）**（focus.rs:76-87）。
- 当前 demo focusable 固定为 [1,2]，无此场景。**P2**：D13 组件动态增删时，`move_focus` 应在 `position()` 返回 None 时把 `focused` 置 None（或回落到首/末），避免残留失焦状态。

---

## 二、WidgetSpec::focusable 默认方法（PASS）

- **默认方法**（traits.rs:68-71）：`fn focusable(&self) -> bool { false }`——**默认不可获焦，向后兼容**（已有组件实现不需改）。✓
- **组件覆盖**：Accordion（components.rs:123-125 true）、WaBadge（components.rs:220-222 true）——两个可交互组件标记可获焦。✓
- **契约一致性**：这是对 greenfield §B.1 WidgetSpec 的**向后兼容扩展**（加了一个带默认实现的方法），不破坏 `WidgetSpec` 原五方法签名——**不构成契约漂移**（新增默认方法不改既有实现）。✓ 需在 D0/greenfield 同步标注（D12 文档同步中，见 §六）。

---

## 三、demo Tab 导航（PASS）

- **`FocusManager` 实例**（window_demo.rs:128）：`RefCell<FocusManager>`（closure 可变捕获，合理），`set_focusable(vec![WidgetId::new(1), WidgetId::new(2)])`（window_demo.rs:129）。
- **mapper 拦截 Tab**（window_demo.rs:138-146）：`KeyboardInput(Tab, Pressed)` → `focus.borrow_mut().focus_next()` → `eprintln!("[focus] Tab -> {:?}", nxt.map(|w| w.0))`（window_demo.rs:143-144），返回 None（Tab 不产生组件消息，只切焦点）。✓
- **日志 `[focus] Tab -> id`**：`nxt.map(|w| w.0)` 输出焦点 id（window_demo.rs:144）。✓
- **Tab 循环**：Accordion(1) → WaBadge(2) → Accordion(1)，经 focus_next 回绕。✓
- **wire**：Tab 键→focus_next→切换焦点，不触发组件 update（None），仅日志。**逻辑正确**（Tab 导航不改变组件状态）。

---

## 四、流式编码判据（PASS）

### 4.1 合规项（全流式）
| 判据 | 检查结果 |
|---|---|
| **iter().find/position 替代手写循环** | `move_focus` 用 `iter().position().map()`（focus.rs:79-81）；`hit_test` 用 `iter().find().map()`——**均组合子，无显式 for 循环** ✓ |
| **`dyn Iterator` 装箱** | 全仓 grep `dyn Iterator`/`Box<dyn Iterator>`（core/platform/facade）→ **空**，无装箱 ✓ |
| **冗余中间 collect** | 无冗余 collect（map_message 的 collect 必要）✓ |
| **rem_euclid 正确使用** | `(i+dir).rem_euclid(n)` 循环回绕——**流式 + 正确** ✓ |

### 4.2 focusable / 组件 view（无违规）
- `focusable()` 是标量方法返回 bool，非迭代器场景。✓
- 组件 view（Accordion/WaBadge）用 `header.children.push(title)` 等——**声明式视图树向量构造**，非"能组合子却循环"（D11 已判为合法，边界内）。✓

**结论：流式编码判据 PASS。** move_focus（iter().position+rem_euclid）是流式典范；全仓无 dyn Iterator/冗余 collect/"能组合子却循环"。

---

## 五、防火墙 / DAG（PASS）

- **focus.rs 在 `rgui-platform`**（platform/src/focus.rs）——符合 greenfield §B.3"焦点管理原生在 platform"（winit 隔离处）。✓
- **`WidgetSpec::focusable` 在 `rgui-core`**（traits.rs:68-71）——纯 Rust 契约（`fn focusable(&self) -> bool`），零 GPU/平台。✓
- **core 零 GPU**：traits.rs 无 wgpu/vello/winit；focus.rs（platform）依赖 `crate::input::InputModality` + `rgui_core::id::WidgetId`——**platform → core 单向**，无反向。✓
- **单一 vello/winit**、DAG 无环——保持。✓

---

## 六、文档一致性（PASS）

- **D5 已同步焦点管理**（总监确认 commit 705a83d 含 D5 文档更新）——FocusManager/move_focus/循环切换/mapper Tab 拦截与代码一致。
- **greenfield/D0 由 doc 同步中**——需注意：`WidgetSpec::focusable()` 是**新增默认方法**，D0 §4.3 / greenfield §B.1 应标注"focusable 默认方法（D12 扩展）"，避免与"四方法契约"描述不一致。**P2 观察**（doc 同步中，需在 D0/greenfield 补记）。

---

## 七、P0/P1 风险清单

**P0：无。P1：无。**

**MERGE GATE：放行（PASS）。**

### P2 观察项（随 D13 处理，不阻塞）
1. **set_focusable 后 focused 残留**（focus.rs:76-87）：若 focused 不在 focusable 列表，move_focus 返回 None 但 focused 不置 None——D13 动态增删组件时需处理（失焦清理/回落）。
2. **WidgetSpec::focusable 需在 D0/greenfield 标注**：新增默认方法（D12 扩展），doc 同步时在 D0 §4.3 / greenfield §B.1 补记，避免契约描述不一致。

---

*审查方：devco-reviewer｜只读审查（未运行 GPU 窗口测试，58 全量数字经 `cargo test --workspace --all-features` 实测核实 = 58 passed）。流式编码判据逐条核对：move_focus（iter().position()+rem_euclid）全流式、无 dyn Iterator/冗余 collect；focus.rs 在 platform、focusable 在 core 纯 Rust，防火墙/DAG 达标。*
