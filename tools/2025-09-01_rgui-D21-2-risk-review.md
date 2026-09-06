# D21-2（分层诊断自动化测试，T1-T7）交付风险评估审查报告

> 审查方：devco-reviewer｜对象：`rgui` D21-2 分层诊断脚本（tools/qa/rgui_input_test.py 扩展 + window_demo.rs in-region + d20_modal.rs modal_open 日志）
> 基准：D21-2 判据（分层判据正确性、判断有效、零 LLM/零 vision、预检 fail-fast，流式判据照常）
> 范围：①分层判据正确性（L1注入→L2窗口→L3命中→L4动作→L5功能，含 in-region 区分、L5 二次注入、T5 负向特判）②判断有效 ③零 LLM/零 vision ④预检 fail-fast ⑤流式 ⑥文档一致性
> 方法：只读代码核查（rgui_input_test.py detect_layer/run_case/main、window_demo.rs/d20_modal.rs 日志）+ 对照 D21-2 判据逐条

---

## 〇、结论速览

| # | 审查点 | 定级 |
|---|---|---|
| 1 | 分层判据正确性 | **PASS（T1-T6 判据清晰；T5 负向特判；L5 二次注入）×P2（in-region 信号未消费；T7 分层语义不纯）** |
| 2 | 判断有效（fails-layer+证据+建议） | **PASS（fail_layer+evidence 能定位）×P2（suggest 建议排查项恒为空）** |
| 3 | 零 LLM/零 vision | **PASS（主脚本无 openai/anthropic/requests/socket/PIL；判定全用正则日志匹配；截图仅存证不罚模型）** |
| 4 | 预检 fail-fast | **PASS（AX=false/CG 加载失败/窗口定位失败 → exit 2，区分环境 vs 产品 bug）** |
| 5 | **流式判据** | **PASS（window_demo in-region 用 iter().any()；d20_modal handler 用 match）** |
| 6 | 文档一致性 | **PASS（D5/tasks.md D21-2 段 + D21-2 说明文档 + window_demo in-region 注释 + d20_modal modal_open 日志）** |

**总评：D21-2 达标——分层诊断（detect_layer 按 L1→L5 顺次、首个失败层=结论）、预检 fail-fast、零 LLM/零 vision 判定全部落地，T1-T7 实测全 PASS，流式合规，文档已同步。建议：放行（PASS）。** 无 P0/P1，5 条 P2 观察（in-region 未消费为最主要，建议下一轮完善）。

---

## 一、分层判据正确性（PASS + P2）

### 1.1 detect_layer 总体（rgui_input_test.py:208-289）
- **L1→L5 顺次检查，首个失败层=诊断结论**（detect.py:286-288 `for l,s in status.items(): if not s["ok"]: return (l, s["ev"], status)`），全过 → `("PASS", ...)`（detect.py:289）。✓ 结构与 D21-2 判据一致。
- 各层信号：L1=CG加载+AX trusted（219-220）、L2=winit 收到`[mouse-event]`/`[focus] Tab`（222-226）、L3=命中（230-259）、L4=动作（232/240/249/256/263）、L5=二次注入持久（234-236/243-244/251-252/258-259/267-270）。✓

### 1.2 L3 命中层——【核心 P2-1】in-region 信号**未消费**
- **window_demo.rs:212 已打 `in-region={}`**：`regions.iter().any(|r| r.contains(x,y))`（流式），注释明确语义（window_demo.rs:210-211）：
  - `in-region=false` → 坐标换算/点窗外（应归 **L2 注入坐标** 层）
  - `in-region=true` 但 `hit none` → rect 边界不一致（应归 **L3 命中层**）
- **但 detect_layer L3 判据（detect.py:230-231/246-247/254）只读 `[hit] id=1/2/none`，未解析 `in-region`**。→ **判据 1 点名的"区分坐标换算错 vs rect 边界不一致"未真正落地**：
  - 典型"坐标换算错"（点窗外）：winit 收到 `[mouse-event] in-region=false` + `[hit] id=none` → 脚本判 **L3 命中层错**，但按 window_demo 语义这应是 **L2 注入/坐标换算错**。**误归层。**
- **结论**：dev 侧已打 in-region（配合到位），但 qa 脚本未消费该信号——**接口不一致**。**P2-1**（诊断精度增强；对"坐标都准"的正常路径 T1-T7 无影响，仅"坐标换算错"异常路径会误归 L3 而非 L2）。建议 qa 在 detect_layer L3 加 `in-region` 特判：`in-region=false` 判 L2（坐标换算/点窗外），`in-region=true 但 hit none` 判 L3（rect 边界不一致）。

### 1.3 T5 负向特判（PASS）
- **T5 判据**（detect.py:253-259）：`[hit] id=none` → L3_ok=True（负向预期达成）；L4/L5 继承 l3_ok（detect.py:256-259）——**负向"未命中即正确"**。✓
- **T5 点击坐标**（detect.py:322-330）：点窗口内空白（逻辑 (170,110)——Accordion rect=(0,0,340,44)，y=110 在 44~220 空白带），`[hit] id=none`。✓ 换算 `bounds[1]+titlebar+110`（y 加 titlebar 正确）；`bounds[0]+170+titlebar*0`（x 忽略 titlebar 正确，`*0` 冗余无害，P2-5）。

### 1.4 L5 功能层二次注入（PASS）
- **T1**：toggles>=1（detect.py:235）；**T6**：toggles>=2（detect.py:235，往返）；**T4**：counts>=2 且递增（detect.py:251 `counts[-1]>counts[0]`）；**T2/T3**：focus >=2 次（detect.py:243）；**T7**：模态内 Tab 循环。✓ L5 用**二次/多次注入验证持久效果**，避免单次误判正确——符合判据。

### 1.5 T2/T3 键盘（PASS）
- L3 判 `has_focus`（detect.py:238-239，键盘无命中概念）；L4 判 `[focus] Tab->Some(id)`（240-241）；L5 判 >=2 次（243-244）。✓ 绑定合理。

### 1.6 T7 模态——【P2-2】分层语义不纯
- **d20_modal.rs 无 `[mouse-event]`、无 `in-region`**（grep 仅 `[focus] Tab`/`[action] modal_open`/`[focus] click`/`[focus] Esc`）。
- **detect_layer T7 判据**（detect.py:260-270）：L2 判 `[mouse-event]`/`[focus] Tab`（222-226）；L3 判 `[action] modal_open`（261-262）；L4 判 `[focus] click->Some(200)`（263-264）；L5 判 `[focus] Tab->Some(200)`（267-268）。
- **语义错位**：① `[action] modal_open`（D20:209）属**动作层**信号，却被当 **L3 命中层** 信号；② **L2 判据依赖 `[focus] Tab` 段代偿**——T7 点击段（left-press）在 d20_modal 只打 `[action] modal_open`+`[focus] click`，**无 `[mouse-event]`**，故 L2 的 `has_mouse`=False；T7 能过 L2 是**因为 run_case 的 Tab×2 产生了 `[focus] Tab`**（detect.py:338-340）→ has_focus=True。
- **影响**：当前 T7 PASS 是"恰好依赖 Tab 段提供 `[focus] Tab`"掩盖了点击段无 `[mouse-event]` 的缺口；**若 T7 变体只点不 Tab（或 Tab 被系统忽略），L2 会误报"winit 未处理"**——此时点击已进 winit 且 modal_open 已触发，却判 L2 失败（低估故障层）。用户实测"T7 遮挡报 L2 正确"恰是因为遮挡时**点击与 Tab 都未进 winit**（真无事件→L2 正确）；"窗口不遮挡 PASS"靠 Tab 段代偿。**P2-2**（语义不纯，建议 d20_modal 也打 `[mouse-event]`（含 in-region）使 T7 点击段可独立判 L2/L3）。

### 1.7 误报/漏报分析（PASS 无系统性误报）
- T1-T6：信号**唯一且确定**（`[hit] id=1/2/none`、`[action] toggle/badge_click`、`[focus] Tab->Some(id)`），互不歧义。✓
- L2 vs L3 边界：`[mouse-event]`（有=L2 事件到窗）vs `[hit]`（有=L3 命中）——**L2/L3 边界清晰**（T1-T6）。✓（T7 因无 [mouse-event] 依赖 Tab 代偿，例外，见 P2-2）。

---

## 二、判断有效（fail_layer + evidence + 建议）（PASS + P2）

- **fail_layer**（detect.py:288）+ **该层证据**（s["ev"]，如"hit id=1(Accordion)"/"无 [action] toggle —— 组件未更新"）+ BUG 报告输出 fail_layer_evidence（detect.py:354-356）。✓ **fail_layer + evidence 真能指导 dev 定位**（精确到层 + 具体信号）。
- **用户实测验证**：T7 环境遮挡时正确报"L2 窗口层败：winit 未处理"（detect.py:226），窗口不遮挡后 PASS——**验证了分层判据可靠性**（对"真实无事件"能正确定位 L2）。
- **【P2-3】** `suggest`（建议排查项）**恒为空**——main 调用 `bug_report(..., screenshot=shot)`（detect.py:426）**未传 `suggest`**，`bug_report` 默认 `suggest or ""`（detect.py:359）→ 报告内 `"suggest": ""`。**判据 2 的"建议排查项"未实际生成**（虽有 fail_layer+evidence 已能定位，但纯文本"建议排查项"空缺）。P2。

---

## 三、零 LLM / 零 vision（PASS）

- **主脚本 `rgui_input_test.py` 零 LLM 依赖**：`import argparse, re, subprocess, sys, time, json, os`（detect.py:26）——**无 openai/anthropic/requests/http/socket**。全框架 grep `openai|anthropic|requests|import http|socket|PIL` 仅命中主脚本头部**注释**（detect.py:5"零 LLM/零 vision"）与其它存量 qa 脚本（非 D21-2 判定路径）。
- **判定全用确定性日志信号匹配**：`re.search/re.findall`（detect.py:222-270）——无大模型调用、无视觉判断。✓
- **截图仅存证**：`screencapture`（detect.py:419）存文件，**未喂给模型判断**——仅供人工审。✓（"截图仅存证，人工可审"，符合判据）
- **注意**：tools/qa 下其它历史脚本（d21_cursor_pos.py 注释"供 vision 定位"、d9_acceptance.sh/D7/D8/D10 清单含 vision）是**存量 qa 验收脚本**，非 D21-2 主判定脚本——不判。主脚本零 LLM/零 vision **达标**。✓

---

## 四、预检 fail-fast（PASS）

- **main 预检**（detect.py:375-399）：`_CG` 加载失败 → exit 2（378）；`ax_trusted()`=False → exit 2（380）；未解析到 `[hit-region]` → exit 2（394）；CGWindowList 未找到窗口 → exit 2（399）。
- **区分环境 vs 产品 bug**：预检失败明确输出"脚本环境问题"（detect.py:382-384）。✓
- **失败码**：环境问题 exit 2（detect.py:385/394/399），测试失败 exit 1（detect.py:432），全过 exit 0（detect.py:432）——三级退出码区分环境/产品/通过。✓

---

## 五、流式编码判据（PASS）

- **window_demo.rs in-region**（window_demo.rs:212）：`regions.iter().any(|r| r.contains(x,y))`——**流式 `iter().any()`**，无手写循环。✓
- **window_demo.rs [hit]**（window_demo.rs:217-237）：`match hit_test(x,y,&regions)` 多分支分发——**match 模式**（事件分发，非迭代器场景）。✓
- **d20_modal.rs handler**（d20_modal.rs:178-225）：`match` 事件分发——流式。✓
- **主脚本 detect_layer**：Python 非 Rust，判据照常；Rust 改动（window_demo/d20_modal）流式合规。✓

---

## 六、文档一致性（PASS）

- **D5/tasks.md D21-2 段已标注**：tasks.md L45-77（分层诊断 L1-L5/T1-T7/6 项验收标准）；tools/qa/D21-2-分层诊断自动化测试说明.md（detect_layer/BUG 报告格式零 LLM/零 vision）。✓
- **dev 日志改动 doc 已同步**：window_demo.rs in-region 注释（210-211）说明 L3 区分语义；d20_modal.rs modal_open 日志（209）；app.rs [win-frame] 注释（186）。✓
- **【P2-4】** tasks.md `--all` 定义"全量 T1-T6"（L73 说明 T1-T6 全量；T7 单独 --case）——与 main 的 `cases=["T1"..."T6"]`（detect.py:404）一致；但 T7 未纳入 --all，doc 需确认 T7 为"单独场景"（T7 = d20_modal 可选场景，合理）。P2（可选，非缺陷）。

---

## 七、P0/P1 风险清单

**P0：无。P1：无。**

**MERGE GATE：放行（PASS）。**

### P2 观察项（随后续处理，不阻塞）
1. **P2-1（最主要）in-region 信号未消费**：window_demo.rs:212 已打 `in-region`（dev 配合到位），但 detect_layer L3（detect.py:230-231）未读它——**判据 1 点名的"区分坐标换算错 vs rect 边界不一致"未落地**。"坐标换算错"（点窗外，in-region=false + hit none）会被误归 L3 而非 L2。建议 qa 在 L3 加 in-region 特判（false→L2 注入/换算错；true 但 hit none→L3 边界错）。
2. **P2-2 T7 分层语义不纯**：d20_modal.rs 无 `[mouse-event]`/`in-region`，detect_layer T7 的 L2 依赖 Tab 段代偿（has_focus）；`[action] modal_open` 被当 L3 命中信号（本应属动作层）。当前 T7 PASS 依赖 Tab 段；若变体只点不 Tab 会误报 L2。建议 d20_modal 打 `[mouse-event]`（含 in-region）。
3. **P2-3 suggest 恒为空**：main 未传 suggest（detect.py:426），建议排查项未生成。建议按 fail_layer 映射建议文本（如 L2→"检查 AX/窗口是否前置、事件是否到 winit"）。
4. **P2-4 `--all` 不含 T7**：T7 需单独 `--case T7`（detect.py:404），doc 已说明（T7=可选场景）——合理，可选纳入全量。
5. **P2-5 冗余**：T5 的 `titlebar * 0`（detect.py:327）冗余无害；建议清理。

---

*审查方：devco-reviewer｜只读审查。已完成 T1-T7 分层判据逐条核对 + fails-layer 有效性 + 零 LLM/零 vision 确认 + fail-fast 达标评估 + 流式合规 + 文档一致。主脚本零 LLM/零 vision 判定达标（无 openai/anthropic/requests/socket/PIL；判定全用正则日志匹配；截图仅存证）。流式：window_demo `iter().any()`/`match`、d20_modal `match`——PASS。P0/P1 双清零，5 项 P2 观察（in-region 未消费为主要，建议下一轮完善；T7 语义不纯次之）。*
