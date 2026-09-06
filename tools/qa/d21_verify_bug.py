#!/usr/bin/env python3
# 验证 BUG 报告格式(含 fail_layer / 证据 / 建议排查项), 模拟 L2 失败
import sys
sys.path.insert(0, "tools/qa")
import rgui_input_test as R

layer, ev, st = R.detect_layer("", "T1")
print(R.bug_report(
    case="T1",
    fail_layer=layer,
    evidence=ev,
    expected="[hit] id=1 + [action] toggle(id=1)",
    inject_meta={"bounds": [700,207,520,252], "scale": 2.0, "coords": "(870,261)"},
    screenshot="tools/qa/d7_screenshots/d21_t1.png",
    suggest="L2 失败: winit 未收到 mouse-event。排查: ①窗口是否激活/置前(activate+raise) ②CGEvent 坐标是否为 point(非物理像素) ③CGPoint 是否按值传递 ④demo 是否处理 MouseInput 分支",
))
