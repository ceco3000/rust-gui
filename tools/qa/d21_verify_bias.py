#!/usr/bin/env python3
# P2-1 构造验证：故意点偏移坐标(窗口外)，观察 demo 打 in-region=false → detect_layer 报 L2(而非 L3)
import sys, time
sys.path.insert(0, "tools/qa")
import rgui_input_test as R

# find window + 激活
bounds, pid = R.find_window_bounds()
print("bounds:", bounds, "pid:", pid)
R.activate_app(pid); R.raise_window(pid); time.sleep(0.5)

log = "/tmp/rgui_demo.log"
start = len(R.read_log(log).splitlines())
# 故意点窗口屏幕坐标偏移巨大处(窗口外, 如窗口左边 1000 点)——应 in-region=false
# 窗口 bounds 左上(700,207); 点 (bounds[0]-200, bounds[1]+20) 即窗口左侧外
pt = (bounds[0] - 200.0, bounds[1] + 20.0)
print(f"故意偏移点击 (窗口外): {pt}")
R.click_at(pt)
time.sleep(0.6)

fresh = "\n".join(R.read_log(log).splitlines()[start:])
print("=== 点击后新增日志 ===")
for n in R.read_log(log).splitlines()[start:]:
    print(" ", n)
# detect_layer 判定
layer, ev, st = R.detect_layer(fresh, "T1")
print(f"\n==> detect_layer 判定: {layer}  |  {ev}")
print("  结论: 坐标偏(窗口外)被正确归为", layer, "(期望 L2=坐标换算错, 非 L3=rect边界)")
