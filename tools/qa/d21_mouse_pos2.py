#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""用 CGEventGetLocation 读鼠标全局位置: 判断 move 是否生效 + 鼠标坐标体系。"""
import ctypes, time
AS='/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices'
L=ctypes.CDLL(AS)
L.AXIsProcessTrusted.restype=ctypes.c_bool
L.CGEventCreate.restype=ctypes.c_void_p
L.CGEventCreate.argtypes=[ctypes.c_void_p]
# CGEventGetLocation 返回 CGPoint(结构体按值) — arm64 打散到 x0/x1 寄存器, 用回调难以读.
# 改用: CGEventGetIntegerValueField(kCGMouseEventX/kCGMouseEventY) — 最通用
L.CGEventGetIntegerValueField.restype=ctypes.c_ulonglong
L.CGEventGetIntegerValueField.argtypes=[ctypes.c_void_p,ctypes.c_uint32]
L.CGEventSourceCreate.restype=ctypes.c_void_p
L.CGEventCreateMouseEvent.restype=ctypes.c_void_p
L.CGEventCreateMouseEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint32,ctypes.c_void_p,ctypes.c_uint32]
L.CGEventPost.restype=None
L.CGEventPost.argtypes=[ctypes.c_uint32,ctypes.c_void_p]
class CGPoint(ctypes.Structure):
    _fields_=[("x",ctypes.c_double),("y",ctypes.c_double)]

# kCGMouseEventX=1, kCGMouseEventY=2 (integer field)
def cur_pos():
    ev=L.CGEventCreate(None)  # current event state
    x=L.CGEventGetIntegerValueField(ev,1)
    y=L.CGEventGetIntegerValueField(ev,2)
    return (x,y)

print("AX:",L.AXIsProcessTrusted())
print("before move, mouse pos:", cur_pos())
src=L.CGEventSourceCreate(1)
pt=CGPoint(1740.0,458.0)
L.CGEventPost(0,L.CGEventCreateMouseEvent(src,5,ctypes.pointer(pt),0))
time.sleep(0.3)
print("after move to (1740,458), mouse pos:", cur_pos())
