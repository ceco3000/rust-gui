#!/usr/bin/env python3
# 验证 detect_layer 的失败定位能力（L1-L5 各层失败 → 正确 fail_layer）
import sys
sys.path.insert(0, "tools/qa")
import rgui_input_test as R

# 模拟各层失败日志快照
tests = [
    # (日志, case, 期望 fail_layer)
    ("", "T1", "L2"),                       # 无事件 → L2 失败
    ("[mouse-event] left-press at logical=(170,22)\n[hit] id=none", "T1", "L3"),  # 无 [hit] id=1 → L3
    ("[mouse-event] left-press at logical=(170,22)\n[hit] id=1 -> AccordionMsg::Toggle", "T1", "L4"),  # 无 toggle → L4
    ("[mouse-event] left-press at logical=(170,22)\n[hit] id=1\n[action] toggle(id=1)\n", "T6", "L5"),  # T6 需 2 次
    ("[mouse-event] left-press at logical=(170,22)\n[hit] id=1\n[action] toggle(id=1)\n[action] toggle(id=1)\n", "T1", "PASS"),
    ("[focus] Tab(shift=false) -> Some(1)\n[focus] Tab(shift=false) -> Some(2)", "T2", "L4"),  # T2 无 Some(id) 移动? 实际有
]
print("=== detect_layer 验证 ===")
for log, case, exp in tests:
    layer, ev, st = R.detect_layer(log, case)
    ok = "✓" if layer == exp else "✗"
    print(f"{ok} case={case} 期望={exp} 实际={layer}  ev={ev}")
