#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D21 最小闭环：真实 CGEvent 点击 Accordion 标题区中心(物理像素换算)，截图对比展开。
换算：screen_physical = win_origin_physical + rect_center_logical × scale。
Accordion rect=(0,0,340,44) center=(170,22)。期望点击后 Accordion 展开(出现 [-]/内容区)。"""
import ctypes, re, time, sys, subprocess

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
# 从日志读 win-frame(物理像素) + hit-region
log = open("/tmp/rgui_demo.log").read()
mf = re.search(r'\[win-frame\] origin=\(([-0-9.]+),\s*([-0-9.]+)\) size=\(([-0-9.]+),\s*([-0-9.]+)\) scale=([-0-9.]+)', log)
mr = re.search(r'id=1 accordion rect=\(([-0-9.]+),\s*([-0-9.]+),\s*([-0-9.]+),\s*([-0-9.]+)\)', log)
if not (mf and mr):
    print("日志缺 win-frame/hit-region", file=sys.stderr); sys.exit(1)
ox, oy, sw, sh, scale = float(mf.group(1)), float(mf.group(2)), float(mf.group(3)), float(mf.group(4)), float(mf.group(5))
rx, ry, rw, rh = (float(mr.group(i)) for i in range(1, 5))
cx, cy = rx + rw/2.0, ry + rh/2.0
screen_x, screen_y = ox + cx*scale, oy + cy*scale
print(f"win origin=({ox},{oy}) scale={scale} accordion center=({cx},{cy}) -> screen physical ({screen_x:.0f},{screen_y:.0f})")

def click(pt):
    c = CGPoint(*pt)
    for t, n in [(5, "move"), (1, "down"), (2, "up")]:
        L.CGEventPost(0, L.CGEventCreateMouseEvent(L.CGEventSourceCreate(1), t, ctypes.pointer(c), 0))
        time.sleep(0.05)

# 截图窗口(先用 CGWindowList 拿 id 用于截图——复用 swift)
# 点击 Accordion(物理像素换算)
click((screen_x, screen_y))
print("clicked Accordion at", (screen_x, screen_y))
time.sleep(0.5)

# 截图全屏留档(供 vision)
subprocess.run(["screencapture", "-x", "/tmp/d21_click_accordion_full.png"], check=False)
print("screenshot saved /tmp/d21_click_accordion_full.png")
