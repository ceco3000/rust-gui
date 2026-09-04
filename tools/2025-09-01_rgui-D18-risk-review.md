# D18（key-based reconcile + 动态增删）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D18 交付（commit 20006e1）
> 基准：greenfield §B.1、D2、D5、D12（focused 残留 P2）、既有判据 + **流式编码判据**
> 范围：①key-based reconcile ②动态增删 ③focused 残留 ④架构边界 ⑤流式判据 ⑥防火墙 ⑦文档一致性
> 方法：只读代码核查（diff.rs/view.rs/d18_list.rs/focus.rs 逐行）+ 实测 `cargo test --workspace --all-features`

---

## 〇、结论速览

| # | 审查点 | 定级 |
|---|---|---|
| 1 | key-based reconcile | **PASS（MoveChild 复用；reorder/remove/add roundtrip 测试全面）** |
| 2 | 动态增删 | **PASS（ListRoot key 复用；RemoveChild 不误伤邻位）** |
| 3 | focused 残留 | **PASS（D12 P2 已修：set_focusable 移除焦点→清空）** |
| 4 | 架构边界 | **PASS（reconcile 在 core 逻辑层；WidgetView.key 纯类型；core 零 GPU）** |
| 5 | **流式判据** | **PASS（ListRoot iter().map().collect()；keyed iter().position()+b.iter().any()；无装箱/冗余 collect）** |
| 6 | 防火墙/DAG | **PASS（reconcile 在 core 零 GPU；单一 vello/winit；DAG 无环）** |
| 7 | 文档一致性 | **PASS（D1/D2/D5/D10 已同步；greenfield §B.1 doc 复核中）** |

**总评：D18 达标——key-based reconcile（key 匹配复用 MoveChild，替代索引重建）、动态增删（ListRoot key 复用，RemoveChild 不误伤邻位）、focused 残留（D12 P2 已修）全部正确，流式合规，69 测试全绿（实测），P0 清零。建议：放行（PASS）。** 无 P0/P1，2 条 P2 观察（见 §六）。

---

## 一、key-based reconcile（PASS）

### 1.1 WidgetView.key（view.rs:38-39）
```rust
pub struct WidgetView<M = ()> {
    ...
    pub key: Option<u64>,   // 组件复用 key；None = 位置型
}
```
- `key: Option<u64>`（view.rs:39）——**None=位置型（向后兼容），Some=keyed**。`map_message` 透传（view.rs:68）、Default None（view.rs:81）。✓

### 1.2 diff_children_keyed（diff.rs:98-141）
```rust
fn diff_children_keyed<M: Clone>(patches, a, b) {
    let mut list: Vec<WidgetView<M>> = a.to_vec();   // 模拟当前 a
    // 1. 移除 a 中 key 不在 b 的（倒序）                          diff.rs:103-110
    for i in (0..list.len()).rev() {
        let k = list[i].key;
        let in_b = b.iter().any(|bc| bc.key == k);
        if !in_b { patches.push(RemoveChild{index:i}); list.remove(i); }
    }
    // 2. 按 b 顺序重建：同 key 匹配（move+update）、b 独有（insert）  diff.rs:113-140
    for j in 0..b.len() {
        let src = match bj.key {
            Some(k) => list.iter().position(|c| c.key == Some(k)),
            None => (j < list.len()).then_some(j),
        };
        match src {
            Some(i) if i == j => { if !subtree_eq → ReplaceChild{j}; list[j]=bj; }
            Some(i) => { MoveChild{from:i,to:j}; list.remove(i); list.insert(j, c); if !subtree_eq → ReplaceChild{j}; }
            None => { InsertChild{j}; list.insert(j, bj); }
        }
    }
}
```
- **按 key 匹配复用**：`list.iter().position(|c| c.key == Some(k))`（diff.rs:116）找同 key 源索引；`i==j` → 内容 update（ReplaceChild），`i!=j` → MoveChild（diff.rs:127）+ 内容更新。**复用而非索引重建**。✓
- **触发条件**（diff.rs:62-66）：`a_all_key && b_all_key && len_sum>0` → keyed；否则位置型（向后兼容）。✓

### 1.3 MoveChild 语义（diff.rs:170-175）
```rust
Patch::MoveChild { from, to } => {
    if *from < view.children.len() {
        let child = view.children.remove(*from);
        let to = (*to).min(view.children.len());
        view.children.insert(to, child);
    }
}
```
- `from` 移出、`to` 插入（clamp 到 len）——`remove_at(from)` + `insert_at(to)` 语义，正确。✓

### 1.4 keyed 测试（diff.rs:300-357）
- `keyed_reorder_reuses_by_key`（300-313）：顺序交换 → 产 MoveChild（复用），收敛，key2 在前。✓
- `keyed_remove_middle_keeps_neighbors`（317-336）：删除中间 key2 → RemoveChild{1}，无 ReplaceChild（邻位复用），收敛。✓
- `keyed_add_remove_and_reorder_roundtrip_converges`（340-357）：增删重排收敛，key4 新增在前。✓
- **roundtrip 全面**：reorder / remove / add+remove+reorder 三类 keyed 场景。✓

---

## 二、动态增删（PASS）

- **d18_list.rs**（ListRoot）：`state.items.iter().map(...).collect()`（d18_list.rs:88-99）产子视图列表，每项 `child.key = Some(it.key)`（d18_list.rs:93）——**key 标识**。
- **update**：`Add` → `items.push(Item{key:next_key})`（d18_list.rs:110-114）；`Remove` → `items.remove(0)`（删除首项，d18_list.rs:115-118）。
- **key 复用**：删除首项后，后续项按 key 复用（diff_children_keyed 匹配），**索引不重建**（避免位置型会误伤邻位的问题）。✓
- **mapper**（d18_list.rs:139-152）：左键 Add、右键 Remove。✓

---

## 三、focused 残留（PASS，D12 P2 已修）

- `set_focusable`（focus.rs:30-34）：
```rust
if let Some(f) = self.focused {
    if !ids.contains(&f) {
        self.focused = None;   // 焦点被移除 → 清空
    }
}
```
- **D12 P2（set_focusable 后 focused 残留）已修复**：`set_focusable` 检查当前 focused 是否在新 focusable 列表，不在则清空（focus.rs:32-34）。**仍在新列表→保留，被移除→清空**。✓

---

## 四、架构边界（PASS）

- **reconcile 在 core 逻辑层**：`diff_children_keyed`（diff.rs，core state）；`WidgetView.key`（view.rs，core 纯类型）。✓
- **core 零 GPU/平台**：diff.rs/view.rs 无 wgpu/vello/winit；`key: Option<u64>` 纯值。✓
- **动态增删在 core**（d18_list State/update）。✓

---

## 五、流式编码判据（PASS）

### 5.1 合规项
| 判据 | 检查结果 |
|---|---|
| **ListRoot::view iter().map().collect()** | `items.iter().map(|it| {...}).collect()`（d18_list.rs:88-99）——**流式** ✓ |
| **keyed iter().position() + b.iter().any()** | `list.iter().position(...)`（diff.rs:116）、`b.iter().any(...)`（diff.rs:105）——**流式组合子** ✓ |
| **`dyn Iterator` 装箱** | 无 `dyn Iterator`（diff/view/d18_list）✓ |
| **冗余中间 collect** | 无冗余 collect ✓ |
| **倒序移除 for-rev** | diff.rs:103 `for i in (0..list.len()).rev()`——**倒序移除避免 index 位移**（关键正确性逻辑），非"可组合子替代的循环"（需 rev + 条件 remove），边界内 ✓ |

### 5.2 边界
- `diff_children_keyed` 用 `list` Vec 模拟已应用状态（diff.rs:100 `a.to_vec()` + move/insert/remove）——**状态模拟**（非纯函数式累积），用 Vec mutation 清晰表达"已应用 patch 的中间状态"比对函数式复杂累积更好读。**边界内**（明确的状态机逻辑）。✓
- `for ... rev()`（diff.rs:103）与 `for j in 0..b.len()`（diff.rs:113）——**顺序重建**（跟 b 顺序生成 patch），非可替代的迭代器组合（需按 index 生成 patch + 模拟 list 状态）。边界内。✓

**结论：流式编码判据 PASS。** ListRoot iter().map().collect()、keyed iter().position()+any()；无 dyn Iterator/冗余 collect；list 状态模拟 + rev 移除属边界内。

---

## 六、防火墙 / DAG（PASS）

- **reconcile 在 core（零 GPU）**：diff_children_keyed/key 全在 core state/view。✓
- **单一 vello/winit**、DAG 无环——保持。✓

---

## 七、文档一致性（PASS + P2 观察）

- **D1/D2/D5/D10 已同步**（总监确认 commit 20006e1）——MoveChild/diff_children_keyed/key 字段与代码一致。
- **greenfield §B.1 由 doc 复核中**——需标注 `WidgetView.key: Option<u64>`、`Patch::MoveChild`、keyed reconcile。P2 观察。

---

## 八、P0/P1 风险清单

**P0：无。P1：无。**

**MERGE GATE：放行（PASS）。**

### P2 观察项（随后续处理，不阻塞）
1. **greenfield/D2 需标注 keyed reconcile**：`WidgetView.key`、`Patch::MoveChild`、`diff_children_keyed`（非 keyed 时回退位置型）——doc 同步时补记。
2. **`diff_children_keyed` 的 list 模拟**：用 Vec 模拟已应用状态（diff.rs:100 + 手动 move/insert/remove）——功能正确，但若后续需"最小化 patch 数"（避免多余 MoveChild+ReplaceChild 对），可优化为 key 对齐后再判内容差异（当前 i!=j 时先 MoveChild 再 content update，可能产生"move+replace"两步而非一步）。P2（效率微优化，非正确性问题）。

---

*审查方：devco-reviewer｜只读审查（未运行 GPU 窗口测试，69 全量数字经 `cargo test --workspace --all-features` 实测核实 = 69 passed）。流式编码判据逐条核对：ListRoot iter().map().collect()、keyed iter().position()+b.iter().any()、无 dyn Iterator/冗余 collect；list 状态模拟 + rev 移除属边界内。reconcile 在 core 零 GPU、单一 vello/winit、DAG 无环。*
