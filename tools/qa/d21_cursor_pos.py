#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D21 光标位置可视化：move 鼠标到候选坐标，全屏截图供 vision 定位光标实际位置。"""
import ctypes, time, subprocess
AS='/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices'
L=ctypes.CDLL(AS)
L.AXIsProcessTrusted.restype=ctypes.c_bool
L.CGEventSourceCreate.restype=ctypes.c_void_p
L.CGEventSourceCreate.argtypes=[ctypes.c_int]
L.CGEventCreateMouseEvent.restype=ctypes.c_void_p
L.CGEventCreateMouseEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint32,ctypes.c_void_p,ctypes.c_uint32]
L.CGEventPost.restype=None
L.CGEventPost.argtypes=[ctypes.c_uint32,ctypes.c_void_p]
class CGPoint(ctypes.Structure):
    _fields_=[("x",ctypes.c_double),("y",ctypes.c_double)]
print("AX:",L.AXIsProcessTrusted())
src=L.CGEventSourceCreate(1)
# move 到物理像素 (1740,458) — win origin(1400,414)+accordion center(170,22)*scale2
pt=CGPoint(1740.0,458.0)
L.CGEventPost(0,L.CGEventCreateMouseEvent(src,5,ctypes.pointer(pt),0))
print("moved to (1740,458)")
time.sleep(0.3)
# 全屏截图(鼠标光标会出现在截图里)
subprocess.run(["screencapture","-x","/tmp/d21_cursor_1740_458.png"],check=False)
print("screenshot /tmp/d21_cursor_1740_458.png")
