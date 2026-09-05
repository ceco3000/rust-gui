#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D21 最小闭环 v3：osascript activate 前台 + CGEvent 点击 Accordion → 截图展开。"""
import ctypes, re, time, subprocess, sys

AS='/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices'
L=ctypes.CDLL(AS)
L.AXIsProcessTrusted.restype=ctypes.c_bool
L.CGEventSourceCreate.restype=ctypes.c_void_p
L.CGEventCreateMouseEvent.restype=ctypes.c_void_p
L.CGEventCreateMouseEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint32,ctypes.c_void_p,ctypes.c_uint32]
L.CGEventPost.restype=None
L.CGEventPost.argtypes=[ctypes.c_uint32,ctypes.c_void_p]
class CGPoint(ctypes.Structure):
    _fields_=[("x",ctypes.c_double),("y",ctypes.c_double)]

# 激活 window_demo
subprocess.run(["osascript","-e",'tell application "System Events" to set frontmost of process "window_demo" to true'],check=False)
print("activated")
time.sleep(0.3)

src=L.CGEventSourceCreate(1)
log=open("/tmp/rgui_demo.log").read()
mf=re.search(r'\[win-frame\] origin=\(([-0-9.]+),\s*([-0-9.]+)\) size=\(([-0-9.]+),\s*([-0-9.]+)\) scale=([-0-9.]+)',log)
mr=re.search(r'id=1 accordion rect=\(([-0-9.]+),\s*([-0-9.]+),\s*([-0-9.]+),\s*([-0-9.]+)\)',log)
ox,oy,scale=float(mf.group(1)),float(mf.group(2)),float(mf.group(5))
rx,ry,rw,rh=(float(mr.group(i)) for i in range(1,5))
cx,cy=rx+rw/2.0,ry+rh/2.0
acc=CGPoint(ox+cx*scale, oy+cy*scale)
print(f"accordion screen ({acc.x:.0f},{acc.y:.0f})")

def click(pt):
    for t in [5,1,2]:
        L.CGEventPost(0,L.CGEventCreateMouseEvent(src,t,ctypes.pointer(pt),0)); time.sleep(0.06)

# 先 move(让 winit 更新 cursor) 等 0.3s 再点击
L.CGEventPost(0,L.CGEventCreateMouseEvent(src,5,ctypes.pointer(acc),0)); time.sleep(0.3)
click(acc)
print("clicked")
time.sleep(0.6)

# 截图
wid=subprocess.run(["swift","-e",'import CoreGraphics; let l=CGWindowListCopyWindowInfo([.optionOnScreenOnly],kCGNullWindowID) as! [[String:Any]]; for w in l { if let n=w["kCGWindowOwnerName"] as? String, n=="window_demo" { print(w[kCGWindowNumber as String]!) } }'],capture_output=True,text=True).stdout.strip()
subprocess.run(["screencapture","-x","-l",wid,"/tmp/d21_v3_click.png"],check=False)
print(f"shot /tmp/d21_v3_click.png win={wid}")
