#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D21 加时序重试：mouseMoved → sleep 0.8 → mouseDown/Up，读日志看 [mouse-event]/[hit]。"""
import ctypes, re, time
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
print("AX:",L.AXIsProcessTrusted())
src=L.CGEventSourceCreate(1)
# Accordion 中心物理像素
log=open("/tmp/rgui_demo.log").read()
mf=re.search(r'\[win-frame\] origin=\(([-0-9.]+),\s*([-0-9.]+)\).*?scale=([-0-9.]+)',log)
mr=re.search(r'id=1 accordion rect=\(([-0-9.]+),\s*([-0-9.]+),\s*([-0-9.]+),\s*([-0-9.]+)\)',log)
ox,oy,scale=float(mf.group(1)),float(mf.group(2)),float(mf.group(3))
cx,cy=float(mr.group(1))+float(mr.group(3))/2.0, float(mr.group(2))+float(mr.group(4))/2.0
pt=CGPoint(ox+cx*scale, oy+cy*scale)
print(f"click accordion at ({pt.x:.0f},{pt.y:.0f})")

start=len(open("/tmp/rgui_demo.log").read().splitlines())
# move + 长等待
L.CGEventPost(0,L.CGEventCreateMouseEvent(src,5,ctypes.pointer(pt),0)); time.sleep(0.8)
# click (down/up)
for t in [1,2]:
    L.CGEventPost(0,L.CGEventCreateMouseEvent(src,t,ctypes.pointer(pt),0)); time.sleep(0.15)
time.sleep(0.8)
# 读新增
new=open("/tmp/rgui_demo.log").read().splitlines()[start:]
print("=== 新增日志 ===")
for p in ['mouse-event','hit] id=1','hit] id=2','hit] id=none','action] toggle','action] badge_click']:
    got=[l for l in new if p in l]
    print(f"  {p}: {got or '(无)'}")
