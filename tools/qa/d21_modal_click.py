#!/usr/bin/env python3
# 独立测 d20_modal: 激活 + 点 modal 按钮中心(rect 400,0,400,60 => 逻辑(600,30)) + 读日志
import ctypes, sys, time, re, subprocess
sys.path.insert(0, "tools/qa")
import rgui_input_test as R

pid = 4625
print("activate:", R.activate_app(pid), "raise:", R.raise_window(pid))
time.sleep(0.6)

# 读 hit-region modul + win-frame
log = open("/tmp/rgui_modal.log").read()
mf = re.search(r'\[win-frame\] origin=\(([-0-9.]+),\s*([-0-9.]+)\) size=\(([-0-9.]+),\s*([-0-9.]+)\) scale=([-0-9.]+)', log)
mr = re.search(r'id=200 modal rect=\(([-0-9.]+),\s*([-0-9.]+),\s*([-0-9.]+),\s*([-0-9.]+)\)', log)
print("win-frame:", mf.groups() if mf else "none", "modal rect:", mr.groups() if mr else "none")
ox, oy, sw, sh, scale = [float(x) for x in mf.groups()]
rx, ry, rw, rh = [float(x) for x in mr.groups()]
cx, cy = rx + rw/2.0, ry + rh/2.0
# 用 to_screen 逻辑: bounds + titlebar + rect中心(point)
bounds = (700.0, 207.0, 520.0, 252.0)
wf = {"origin": (ox, oy), "size": (sw, sh), "scale": scale}
pt = R.to_screen(wf, {"center": (cx, cy)}, bounds)
print(f"modal center 逻辑({cx},{cy}) -> screen point {pt}")
start = len(open("/tmp/rgui_modal.log").read().splitlines())
R.click_at(pt)
time.sleep(0.6)
new = open("/tmp/rgui_modal.log").read().splitlines()[start:]
print("=== 点击后新增 ===")
for n in new:
    print(" ", n)
