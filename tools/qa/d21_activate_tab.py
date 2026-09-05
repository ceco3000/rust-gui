#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""诊断：当前前台应用是谁 + 激活 window_demo 为前台后再注入 Tab。
用 NSRunningApplication activate + NSWorkspace.frontmostApplication。"""
import ctypes, time, sys, re

# 用 AppKit(PyObjC不可靠) -> 用 ctypes 调 ObjC 太复杂。改用 osascript(lldb?) 
# 简单方案：用 CGEventSource 激活 target(点击窗口标题栏) 再 Tab
AS = '/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices'
L = ctypes.CDLL(AS)
L.AXIsProcessTrusted.restype = ctypes.c_bool
L.CGEventSourceCreate.restype = ctypes.c_void_p
L.CGEventSourceCreate.argtypes = [ctypes.c_int]
L.CGEventCreateMouseEvent.restype = ctypes.c_void_p
L.CGEventCreateMouseEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_void_p, ctypes.c_uint32]
L.CGEventCreateKeyboardEvent.restype = ctypes.c_void_p
L.CGEventCreateKeyboardEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint16, ctypes.c_bool]
L.CGEventPost.restype = None
L.CGEventPost.argtypes = [ctypes.c_uint32, ctypes.c_void_p]

class CGPoint(ctypes.Structure):
    _fields_ = [("x", ctypes.c_double), ("y", ctypes.c_double)]

print("AX trusted:", L.AXIsProcessTrusted())
src = L.CGEventSourceCreate(1)

# 1. 先点击窗口标题栏(win 物理 origin=1400,414; 标题栏在顶部 y≈414+10)激活窗口
title_pt = CGPoint(1400 + 200*2, 414 + 10*2)  # 标题栏中心附近(物理)
for t, n in [(5, "move"), (1, "down"), (2, "up")]:
    L.CGEventPost(0, L.CGEventCreateMouseEvent(src, t, ctypes.pointer(title_pt), 0))
    time.sleep(0.05)
print("clicked title bar to focus window")
time.sleep(0.5)

# 2. 注入 Tab x2
def key(kc, down):
    L.CGEventPost(0, L.CGEventCreateKeyboardEvent(src, kc, down))
    time.sleep(0.05)

def focus_count():
    try:
        return len(re.findall(r'\[focus\] Tab', open("/tmp/rgui_demo.log").read()))
    except FileNotFoundError:
        return -1

before = focus_count()
for _ in range(2):
    key(48, True); key(48, False)
    time.sleep(0.1)
time.sleep(0.5)
after = focus_count()
print(f"Tab x2 after click title -> focus events: {after-before}")
lines = open("/tmp/rgui_demo.log").read().splitlines()
print("tail:", lines[-3:] if lines else "(empty)")
