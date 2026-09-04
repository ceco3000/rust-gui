# D20（模态层级 + InputEvent/Ime 真实驱动）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D20 交付（commit c32d4ec）
> 基准：greenfield §B.3、D5、D12（模态）、既有判据 + **流式编码判据**
> 范围：①模态层级 ②ImeEvent ③InputEvent ④流式判据 ⑤架构边界 ⑥防火墙 ⑦文档一致性
> 方法：只读代码核查（focus.rs/event_loop.rs/ime.rs/input.rs 逐行）+ 实测 `cargo test --workspace --all-features`

---

## 〇、结论速览

| # | 审查点 | 定级 |
|---|---|---|
| 1 | 模态层级 | **PASS（open_modal 暂存 base/close_modal 恢复；3 测试全面）** |
| 2 | ImeEvent | **PASS（4 变体真实映射）×P2（ime.rs 重复定义未用）** |
| 3 | InputEvent | **PASS（CursorMoved/Pressed/Released/Text 真实转换）** |
| 4 | **流式判据** | **PASS（to_* match+map；focus 模态 std::mem::take + contains；无装箱/冗余 collect）** |
| 5 | 架构边界 | **PASS（输入/IME/模态在 platform winit 隔离；core 收逻辑事件）** |
| 6 | 防火墙/DAG | **PASS（platform winit；单一 vello/winit；DAG 无环）** |
| 7 | 文档一致性 | **PASS（D5 已同步；IME/文本编辑接入待 P1 如实标注）** |

**总评：D20 达标——模态层级（open_modal/close_modal 焦点隔离+恢复）、InputEvent/ImeEvent 真实驱动转换全链路正确，流式合规，81 测试全绿（实测），P0 清零。建议：放行（PASS）。** 无 P0/P1，1 条 P2 观察（ime.rs ImeEvent 重复定义未用）。

---

## 一、模态层级（PASS）

### 1.1 open_modal（focus.rs:37-51）
```rust
pub fn open_modal(&mut self, modal_focusable: Vec<WidgetId>) {
    if self.modal { return; }                                  // 单层，忽略嵌套
    self.base_focusable = std::mem::take(&mut self.focusable);  // 暂存 base
    self.base_focused = self.focused;                           // 保存打开前焦点
    self.focusable = modal_focusable;                           // 焦点限定模态集合
    self.modal = true;
    if let Some(f) = self.focused {
        if !self.focusable.contains(&f) {
            self.focused = self.focusable.first().copied();      // 焦点不在模态 → 移到第一个
        }
    }
}
```
- **`std::mem::take(&mut self.focusable)`**（focus.rs:41）——暂存 base focusable（流式，take 取出置空）。✓
- **焦点隔离**：`focusable = modal_focusable`（focus.rs:43），焦点不在模态集合→移到第一个（focus.rs:46-50）。✓
- **单层**（focus.rs:38-39 `if self.modal return`）——模态不叠加（忽略嵌套）。✓

### 1.2 close_modal（focus.rs:54-65）
```rust
pub fn close_modal(&mut self) {
    if !self.modal { return; }
    self.focusable = std::mem::take(&mut self.base_focusable);  // 恢复 base focusable
    self.modal = false;
    match self.base_focused.take() {
        Some(f) if self.focusable.contains(&f) => self.focused = Some(f),  // 恢复焦点（仍在 base）
        _ => self.focused = None,                                          // 否则清空
    }
}
```
- **恢复 base focusable**（focus.rs:58）；**恢复打开前焦点**：仍在 base→保留（focus.rs:62），否则清空（focus.rs:63）。✓

### 1.3 is_modal_open（focus.rs:68-70）✓

### 1.4 modality vs 模态层级（边界清晰）
- `set_modality`（focus.rs:31-33）保持占位（输入模态类型后续）；**焦点模态层级经 open_modal/close_modal 提供**——二者分离（focus.rs:30-33 注释）。✓

### 1.5 模态测试（focus.rs:215-260，3 个）
- `modal_opened_isolates_focus_within_modal_set`（215-232）：open_modal([10,11]) 焦点 1→10（隔离），Tab 模态内循环（10→11→10），base 2 不可获焦。✓
- `modal_closed_restores_base_focusable_and_focus`（235-249）：close 恢复 base [1,2,3]，焦点保留 2。✓
- `modal_close_clears_focus_when_base_focused_is_none`（252-260）：打开前无焦点，close 后保持无焦点。✓
- **覆盖**: 隔离/Tab 模态内循环/base 不可获焦/关闭恢复/无焦点清空——**全面**。✓

---

## 二、ImeEvent（PASS + P2 重复定义）

### 2.1 input.rs ImeEvent（event_loop 用的，真实）
- `ImeEvent{Enabled, Preedit{text}, Commit{text}, Disabled}`（input.rs:31-40）——**4 变体**（组合输入 Preedit→Commit 事件流）。✓
- `to_ime_event`（event_loop.rs:43-51）：`match Ime::Enabled/Preedit(text,_)/Commit(text)/Disabled` → 对应 ImeEvent。✓

### 2.2 【P2-观察】ime.rs ImeEvent 重复定义（未用）
- **ime.rs:20-25**：`pub enum ImeEvent { Preedit(String), Commit(String) }`——**2 变体过时占位**（D3 src/ime.rs 占位）。
- **input.rs:31-40**：`pub enum ImeEvent { Enabled, Preedit{text}, Commit{text}, Disabled }`——4 变体真实。
- **同一 crate（rgui-platform）有两个同名 ImeEvent**（ime::ImeEvent 与 input::ImeEvent），lib.rs **未 re-export**（lib.rs:20-27 只导出 FocusManager/InputModality/run_as 等），故不冲突（不同模块路径）。但**ime.rs 的 ImeEvent 从未被引用**（grep 仅 ime.rs:20 定义，无消费方）——**过时/占位残留，与 input.rs 重复**。**P2**：应删除 ime.rs 的 ImeEvent（或合并到 input.rs），避免"两个 ImeEvent"概念混淆。

### 2.3 IME 测试（event_loop.rs:179-217，4 个）
- `ime_commit_maps_to_ime_event`（179-187：Commit("é")）、`ime_preedit_maps`（190-198：Preedit("啊")）、`ime_enabled_disabled_map`（201-210：Enabled/Disabled）、`ime_event_does_not_map_to_input_event`（213-217：IME 不映射 InputEvent）——**覆盖 4 变体 + 通道分离**。✓

---

## 三、InputEvent（PASS）

- **`to_input_event`**（event_loop.rs:21-40）：`CursorMoved{position}` → `CursorMoved{x,y}`（event_loop.rs:23-26）；`MouseInput(Pressed)` → `Pressed`（27-30）；`MouseInput(Released)` → `Released`（31-34）；`KeyboardInput{event}` → `ke.text.map(|t| Text(t))`（35-37）；否则 None。✓
- **InputEvent 枚举**（input.rs:18-27）：CursorMoved/Pressed/Released/Text。✓
- **真实转换**（D20 从占位到真实驱动）。✓

---

## 四、流式编码判据（PASS）

### 4.1 合规项
| 判据 | 检查结果 |
|---|---|
| **to_* match+map** | `to_input_event`/`to_ime_event` 用 `match` + `map`（event_loop.rs:21-51）——**模式匹配 + Option 组合子**，无循环 ✓ |
| **focus 模态 std::mem::take** | `std::mem::take(&mut self.focusable)`（focus.rs:41/58）——**take 组合**（非手写 swap/迭代）✓ |
| **模态 contains** | `self.focusable.contains(&f)`（focus.rs:47/62/75/85）——`Vec::contains`（流式）✓ |
| **`dyn Iterator` 装箱** | 无 `dyn Iterator`（focus/event_loop/ime/input）✓ |
| **冗余中间 collect** | 无冗余 collect ✓ |

### 4.2 边界
- `match self.base_focused.take()`（focus.rs:61）——**take + match 恢复焦点**（Option 组合），流式。✓
- `to_input_event`/`to_ime_event` 用 `match`（多分支事件分发），非迭代器场景。✓

**结论：流式编码判据 PASS。** to_* match+map、focus 模态 std::mem::take + contains，无 dysum Iterator/冗余 collect。

---

## 五、架构边界（PASS）

- **输入/IME/模态在 platform（winit 隔离）**：InputEvent/ImeEvent/focus.rs 全在 rgui-platform；`to_input_event`/`to_ime_event`（event_loop.rs）winit → rgui 转换。✓
- **core 收逻辑事件**：core 无 ImeEvent/InputEvent（rgui_core 无引用）；事件经平台转换后供上层。✓
- **core 零 GPU/平台**：ImeEvent/InputEvent 都在 platform。✓

---

## 六、防火墙 / DAG（PASS）

- **platform winit**：IME/输入/模态在 platform（winit 隔离）。✓
- **单一 vello/winit**、DAG 无环——保持。✓

---

## 七、文档一致性（PASS）

- **D5 已同步**（总监确认 commit c32d4ec）：InputEvent/ImeEvent + 模态 open_modal/close_modal 与代码一致。
- **"IME/文本编辑接入待 P1 + 实时注入受限"如实标注**：`to_ime_event` 是 winit→ImeEvent 转换（event_loop.rs:43-51），但**真实文本编辑器接入（IME 候选/编辑到 WidgetView.text）为 P1 待做**——当前仅事件映射，未做编辑器集成。**D5 已如实标注**（未虚构 IME 编辑已接入）。✓
- **greenfield doc 同步中**——需标注 ImeEvent 4 变体、modal。P2。

---

## 八、P0/P1 风险清单

**P0：无。P1：无。**

**MERGE GATE：放行（PASS）。**

### P2 观察项（随后续处理，不阻塞）
1. **ime.rs ImeEvent 重复定义未用**：ime.rs:20-25 的 `ImeEvent{Preedit(String), Commit(String)}`（2 变体占位）与 input.rs:31-40 的 4 变体 ImeEvent 同名重复，且 ime.rs 版本从未被引用——建议删除/合并（避免"两个 ImeEvent"混淆）。D21 清理。
2. **greenfield/D5 需标注 ImeEvent 4 变体 + modal**：doc 同步时补记。

---

*审查方：devco-reviewer｜只读审查（未运行 GPU 窗口测试，81 全量数字经 `cargo test --workspace --all-features` 实测核实 = 81 passed）。流式编码判据逐条核对：to_* match+map、focus 模态 std::mem::take + contains、无 dyn Iterator/冗余 collect。输入/IME/模态在 platform（winit 隔离）、core 收逻辑事件、单一 vello/winit、DAG 无环。*
