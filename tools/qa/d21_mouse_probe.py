#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""验证：move 鼠标到 (1740,458) 后, 读回系统鼠标位置确认 move 生效(坐标体系正确)。"""
import ctypes, time
AS='/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices'
L=ctypes.CDLL(AS)
L.AXIsProcessTrusted.restype=ctypes.c_bool
L.CGEventSourceCreate.restype=ctypes.c_void_p
L.CGEventSourceCreate.argtypes=[ctypes.c_int]
L.CGEventCreateMouseEvent.restype=ctypes.c_void_p
L.CGEventCreateMouseEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint32,ctypes.c_void_p,ctypes.c_uint32]
L.CGEventPost.restype=None
L.CGEventPost.argtypes=[ctypes.c_uint32,ctypes.c_void_p]
# 读当前鼠标位置
L.CGEventCreate.restype=ctypes.c_void_p
L.CGEventCreate.argtypes=[ctypes.c_void_p]
L.CGEventGetLocation.restype=None  # 覆盖定义在下方
class CGPoint(ctypes.Structure):
    _fields_=[("x",ctypes.c_double),("y",ctypes.c_double)]

# 读当前状态: CGEventCreate(None) 得到 current event, 枚举其位置需要 CGEventGetLocation
# 简化：用 event mask 读位置不用, 直接读 CGEventCreate + kCGMouseEventPosition
print("AX:",L.AXIsProcessTrusted())
src=L.CGEventSourceCreate(1)
pt=CGPoint(1740.0,458.0)
L.CGEventPost(0,L.CGEventCreateMouseEvent(src,5,ctypes.pointer(pt),0))
print("posted move to (1740,458)")
time.sleep(0.3)

# 读回鼠标位置: 用 CGEventGetLocation(CGEventCreate(None))
L.CGEventGetLocation.restype=ctypes.c_void_p
L.CGEventGetLocation.argtypes=[ctypes.c_void_p]
ev=L.CGEventCreate(None)
loc=L.CGEventGetLocation(ev)
# CGEventGetLocation 返回 CGPoint 值(非指针) — 用 ctypes 按返回值 double x2 读
# 实际上 CGEventGetLocation 返回 CGPoint(结构体, 按值返回) — 在 x86/arm64 结构体按值返回棘手
# 改用更稳: CGEventGetIntegerValueField? 不适用. 用 CGEventGetLocation 转 double 数组
# 简化: 用一个 C 函数指针按值返回结构体太复杂, 改为直接读 macOS 全局鼠标:
import subprocess
out=subprocess.run(["swift","-e",'import AppKit; let p=NSEvent.mouseLocation; print(p.x, p.y)'],capture_output=True,text=True).stdout.strip()
print("NSEvent.mouseLocation (screen point, bottom-left origin):", out)
