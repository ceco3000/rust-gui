#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D21 坐标换算实测校准：把鼠标 move 到 Accordion 中心，读 demo 日志看命中哪个 id。
假设换算：screen = win_origin + rect_center × scale（dev 的 win-frame 为物理像素）。
Accordion rect=(0,0,340,44) center=(170,22), scale=2, win_origin 从 /tmp/rgui_demo.log 读。"""
import ctypes, re, sys, time, os

AS = '/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices'
L = ctypes.CDLL(AS)
L.AXIsProcessTrusted.restype = ctypes.c_bool
L.CGEventSourceCreate.restype = ctypes.c_void_p
L.CGEventSourceCreate.argtypes = [ctypes.c_int]
L.CGEventCreateMouseEvent.restype = ctypes.c_void_p
L.CGEventCreateMouseEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_void_p, ctypes.c_uint32]
L.CGEventPost.restype = None
L.CGEventPost.argtypes = [ctypes.c_uint32, ctypes.c_void_p]

class CGPoint(ctypes.Structure):
    _fields_ = [("x", ctypes.c_double), ("y", ctypes.c_double)]

print("AX trusted:", L.AXIsProcessTrusted())

# 从日志读 win-frame origin/scale
log = open("/tmp/rgui_demo.log").read()
m = re.search(r'\[win-frame\] origin=\(([-0-9.]+),\s*([-0-9.]+)\) size=\([^)]+\) scale=([-0-9.]+)', log)
if m:
    ox, oy, scale = float(m.group(1)), float(m.group(2)), float(m.group(3))
    print(f"win-frame origin=({ox},{oy}) scale={scale}")
else:
    ox, oy, scale = 1400.0, 414.0, 2.0
    print("fallback origin=(1400,414) scale=2")

# 读日志文件当前命中数作为基线
def hit_count():
    return len(re.findall(r'\[hit-region\]', open("/tmp/rgui_demo.log").read()))

# Accordion rect center (逻辑) -> screen
def to_screen(cx, cy):
    return (ox + cx * scale, oy + cy * scale)

# 两种假设测试点：
# A: dev 物理像素 origin 直接换算
ptA = to_screen(170, 22)
# B: 用 CGWindowList point (700,207) + scale=1 (CGEvent 用点) —— 若 A 不对则试 B
ptB = (700 + 170 * 1.0, 207 + 22 * 1.0)  # scale=1(点坐标系)

for label, pt in [("A_physical", ptA), ("B_point", ptB)]:
    before = hit_count()
    c = CGPoint(*pt)
    ev = L.CGEventCreateMouseEvent(L.CGEventSourceCreate(1), 5, ctypes.pointer(c), 0)  # moved
    L.CGEventPost(0, ev)
    time.sleep(0.3)
    after = hit_count()
    print(f"{label}: move to ({pt[0]:.0f},{pt[1]:.0f}) -> new hit-region lines: {after-before}")
