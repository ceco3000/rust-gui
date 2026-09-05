#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D21 最小闭环 v2：先点击标题栏激活窗口 → 点击 Accordion 标题区(物理像素换算) → 截图断言展开。"""
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
src = L.CGEventSourceCreate(1)
log = open("/tmp/rgui_demo.log").read()
mf = re.search(r'\[win-frame\] origin=\(([-0-9.]+),\s*([-0-9.]+)\) size=\(([-0-9.]+),\s*([-0-9.]+)\) scale=([-0-9.]+)', log)
mr = re.search(r'id=1 accordion rect=\(([-0-9.]+),\s*([-0-9.]+),\s*([-0-9.]+),\s*([-0-9.]+)\)', log)
ox,oy,scale = float(mf.group(1)),float(mf.group(2)),float(mf.group(5))
rx,ry,rw,rh = (float(mr.group(i)) for i in range(1,5))
cx,cy = rx+rw/2.0, ry+rh/2.0
acc_pt = CGPoint(ox+cx*scale, oy+cy*scale)  # (1740,458)
title_pt = CGPoint(ox+200*2, oy+10*2)       # 标题栏

def click(pt):
    for t in [5,1,2]:
        L.CGEventPost(0, L.CGEventCreateMouseEvent(src, t, ctypes.pointer(pt), 0))
        time.sleep(0.06)

# 1 激活: 点击标题栏
click(title_pt); print("activated via title bar")
time.sleep(0.4)
# 2 点 Accordion
click(acc_pt); print(f"clicked Accordion at ({acc_pt.x:.0f},{acc_pt.y:.0f})")
time.sleep(0.5)
# 3 截图窗口(通过 CGWindowList id) — 用 swift 拿 id
import subprocess
wid = subprocess.run(
    ["swift","-e","import CoreGraphics; let l=CGWindowListCopyWindowInfo([.optionOnScreenOnly],kCGNullWindowID) as! [[String:Any]]; for w in l { if let n=w[\"kCGWindowOwnerName\"] as? String, n==\"window_demo\" { print(w[kCGWindowNumber as String]!) } }"],
    capture_output=True, text=True).stdout.strip()
if wid:
    subprocess.run(["screencapture","-x","-l",wid,"/tmp/d21_acc_expanded.png"], check=False)
    print(f"screenshot /tmp/d21_acc_expanded.png (win {wid})")
else:
    print("no window id")
