#!/usr/bin/env python3
# D21 原子化权限探测：AX 授权 + CGEvent 创建/发布能力（ctypes 调 ApplicationServices）
import ctypes, sys

AS = '/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices'
out = []
try:
    L = ctypes.CDLL(AS)
    L.AXIsProcessTrusted.restype = ctypes.c_bool
    out.append(f"AX trusted: {L.AXIsProcessTrusted()}")
except Exception as e:
    out.append(f"AX probe err: {e}")

try:
    L.CGEventSourceCreate.restype = ctypes.c_void_p
    L.CGEventSourceCreate.argtypes = [ctypes.c_int]
    src = L.CGEventSourceCreate(1)  # kCGEventSourceStateHIDSystemState
    out.append(f"CGEventSourceCreate(1) -> {src}")
    # 测试鼠标事件创建
    L.CGEventCreateMouseEvent.restype = ctypes.c_void_p
    L.CGEventCreateMouseEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint32,
                                          ctypes.c_void_p, ctypes.c_uint32]
    # CGPoint 需按结构体传，跳过（仅测创建 source）
    out.append("CGEvent API 加载成功")
except Exception as e:
    out.append(f"CGEvent probe err: {e}")

print("\n".join(out))
