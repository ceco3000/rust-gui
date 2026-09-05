#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D21 鼠标点击闭环验证 v2：point 坐标体系（不再乘 scale）。
读 CGWindowList 真实窗口 bounds(point) → AppKit 激活 → 注入 move/down/up
到 Accordion 中心 → 读 demo 日志断言 [mouse-event]/[hit] id=1/[action] toggle。"""
import Quartz
import AppKit
import time
import re
import subprocess

LOG = "/tmp/rgui_demo2.log"

def tail():
    try:
        return open(LOG).read().splitlines()
    except FileNotFoundError:
        return []

def cur():
    return Quartz.CGEventGetLocation(Quartz.CGEventCreate(None))

# ---- 1. 找窗口 bounds（point，含标题栏）----
wins = Quartz.CGWindowListCopyWindowInfo(
    Quartz.kCGWindowListOptionOnScreenOnly | Quartz.kCGWindowListExcludeDesktopElements,
    Quartz.kCGNullWindowID,
)
target = None
for w in wins:
    name = w.get("kCGWindowOwnerName", "")
    title = w.get("kCGWindowName", "")
    if "window_demo" in name or "hit-test" in str(title):
        target = w
        break
if not target:
    print("!!! 未找到 window_demo 窗口")
    raise SystemExit(1)

b = target["kCGWindowBounds"]
print(f"窗口 bounds(point): x={b['X']:.0f} y={b['Y']:.0f} w={b['Width']:.0f} h={b['Height']:.0f}")
print(f"窗口层         : {target.get('kCGWindowLayer')}, pid={target.get('kCGWindowOwnerPID')}")

# 标题栏高 = 窗口总高 - 内容高(220)
titlebar = b["Height"] - 220.0
print(f"标题栏推算      : {titlebar:.0f} point")

# Accordion 中心(内容区 point) = bounds 左上 + 标题栏 + rect(0,0,340,44) 中心(170,22)
tx = b["X"] + 170.0
ty = b["Y"] + titlebar + 22.0
print(f"目标点击点(point): ({tx:.0f}, {ty:.0f})  [win-frame origin 物理(1400,414) ÷2 = (700,207) 对照]")

# ---- 2. AppKit 激活窗口（synthetic 点击不激活窗口的 workaround）----
pid = target["kCGWindowOwnerPID"]
app = AppKit.NSRunningApplication.runningApplicationWithProcessIdentifier_(pid)
ok = app.activateWithOptions_(AppKit.NSApplicationActivateIgnoringOtherApps)
print(f"激活窗口        : pid={pid} -> {ok}")
time.sleep(0.6)

# ---- 3. 注入 move/down/up ----
src = Quartz.CGEventSourceCreate(Quartz.kCGEventSourceStateCombinedSessionState)
start = len(tail())

def post(t, pos):
    ev = Quartz.CGEventCreateMouseEvent(src, t, pos, Quartz.kCGMouseButtonLeft)
    Quartz.CGEventPost(Quartz.kCGHIDEventTap, ev)

post(Quartz.kCGEventMouseMoved, (tx, ty)); time.sleep(0.08)
post(Quartz.kCGEventLeftMouseDown, (tx, ty)); time.sleep(0.08)
post(Quartz.kCGEventLeftMouseUp, (tx, ty)); time.sleep(0.6)

# ---- 4. 读回光标 + 日志断言 ----
after = cur()
print(f"注入后真实光标  : ({after.x:.0f}, {after.y:.0f})")

new = tail()[start:]
print("=== 点击后新增日志 ===")
for pat, label in [
    (r"\[mouse-event\]", "鼠标事件到达"),
    (r"\[hit\] id=1", "命中 Accordion"),
    (r"\[action\] toggle", "Toggle 动作"),
    (r"\[hit\] id=none", "命中落空(错误)"),
]:
    got = [l for l in new if re.search(pat, l)]
    print(f"  {'✅' if got else '❌'} {label}: {got or '(无)'}")

# ---- 5. 截图存证 ----
subprocess.run(["screencapture", "-x", "/tmp/d21_point_click.png"], check=False)
print("截图: /tmp/d21_point_click.png")
