#!/usr/bin/env python3
# P2-1 验证：in-region 消费 → 坐标偏→L2 / rect 边界→L3 / 正常→PASS
import sys
sys.path.insert(0, "tools/qa")
import rgui_input_test as R

# 场景日志快照（T1）
cases = [
    # (日志, 期望层, 说明)
    ("[mouse-event] left-press at logical=(170,22) in-region=false\n[hit] id=none\n[action] toggle(id=1)", "L2",
     "in-region=false → 坐标换算错/点窗外 → 归 L2(注入坐标问题)"),
    ("[mouse-event] left-press at logical=(170,22) in-region=true\n[hit] id=none\n", "L3",
     "in-region=true 但 [hit] id=none → rect 边界不一致 → L3"),
    ("[mouse-event] left-press at logical=(170,22) in-region=true\n[hit] id=1\n[action] toggle(id=1)\n[action] toggle(id=1)", "L1",  # 实际应 PASS
     "正常(坐标对+命中+动作) → PASS"),  # 期望错会显示, 这里硬编码应为 PASS
    ("", "L2", "无 [mouse-event] → winit 未处理 → L2"),
    ("[focus] Tab(shift=false) -> Some(1)\n[focus] Tab(shift=false) -> Some(2)", "L5", "不应为L5"),  # 实际应 PASS
]

for log, exp, note in cases:
    layer, ev, st = R.detect_layer(log, "T1")
    # 修正期望: 第3/5条应为PASS
    if "toggle(id=1)\n[action] toggle(id=1)" in log:
        exp = "PASS"
    if "[focus] Tab(shift=false) -> Some(1)\n[focus] Tab" in log:
        exp = "PASS"
    ok = "✓" if layer == exp else "✗"
    print(f"{ok} 期望={exp} 实际={layer}  | {note}")
    print(f"      ev={ev}")
