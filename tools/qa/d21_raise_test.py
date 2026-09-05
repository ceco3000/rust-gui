#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D21 窗口 raise 验证：AX API 把 window_demo 窗口 raise 到最前端（z 序最高），
然后注入点击，断言 [mouse-event]/[action] 日志。

背景：activateWithOptions 只激活应用进程，窗口仍可能被其他窗口遮挡（点击落空）。
raise 用 kAXRaiseAction + kAXFrontmostAttribute。"""
import time
import subprocess
import Quartz

LOG = "/tmp/rgui_demo3.log"
PID = 27823  # window_demo

def tail():
    return open(LOG).read().splitlines()

def z_order():
    """CGWindowList 从前往后列出 layer=0 窗口（[owner, pid, bounds]）。"""
    wins = Quartz.CGWindowListCopyWindowInfo(
        Quartz.kCGWindowListOptionOnScreenOnly | Quartz.kCGWindowListExcludeDesktopElements,
        Quartz.kCGNullWindowID)
    return [(w.get("kCGWindowOwnerName"), w.get("kCGWindowOwnerPID"), w["kCGWindowBounds"])
            for w in wins if w.get("kCGWindowLayer") == 0]

def idx_of(pid):
    return next((i for i, (_, p, _) in enumerate(z_order()) if p == pid), None)

print("raise 前 z 序(前5):")
for i, (n, p, b) in enumerate(z_order()[:5]):
    mark = " <== window_demo" if p == PID else ""
    print(f"  [{i}] {n} (pid={p}) bounds=({b['X']:.0f},{b['Y']:.0f},{b['Width']:.0f}x{b['Height']:.0f}){mark}")

# ---- AX raise（osascript，System Events 按 unix id 定位进程） ----
r = subprocess.run([
    "osascript", "-e",
    f'tell application "System Events" to tell (first process whose unix id is {PID}) '
    'to perform action "AXRaise" of window 1',
], capture_output=True, text=True)
print("osascript AXRaise:", r.stdout.strip() or r.stderr.strip(), f"(exit={r.returncode})")
time.sleep(0.5)

print("raise 后 z 序(前5):")
for i, (n, p, b) in enumerate(z_order()[:5]):
    mark = " <== window_demo" if p == PID else ""
    print(f"  [{i}] {n} (pid={p}) bounds=({b['X']:.0f},{b['Y']:.0f},{b['Width']:.0f}x{b['Height']:.0f}){mark}")

# ---- 注入点击 accordion (870,261) ----
src = Quartz.CGEventSourceCreate(Quartz.kCGEventSourceStateCombinedSessionState)
s = len(tail())
for t in [Quartz.kCGEventMouseMoved, Quartz.kCGEventLeftMouseDown, Quartz.kCGEventLeftMouseUp]:
    Quartz.CGEventPost(Quartz.kCGHIDEventTap,
                       Quartz.CGEventCreateMouseEvent(src, t, (870.0, 261.0), Quartz.kCGMouseButtonLeft))
    time.sleep(0.08)
time.sleep(0.5)
new = [l for l in tail()[s:] if l.startswith("[")]
print("点击后新增日志:", new)
