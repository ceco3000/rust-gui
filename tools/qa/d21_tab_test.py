#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D21 键盘链路验证：注入 Tab，读日志是否出现 [focus] Tab(shift=..) -> .."""
import ctypes, time, sys, re

AS = '/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices'
L = ctypes.CDLL(AS)
L.AXIsProcessTrusted.restype = ctypes.c_bool
L.CGEventSourceCreate.restype = ctypes.c_void_p
L.CGEventSourceCreate.argtypes = [ctypes.c_int]
L.CGEventCreateKeyboardEvent.restype = ctypes.c_void_p
L.CGEventCreateKeyboardEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint16, ctypes.c_bool]
L.CGEventPost.restype = None
L.CGEventPost.argtypes = [ctypes.c_uint32, ctypes.c_void_p]

print("AX trusted:", L.AXIsProcessTrusted())
src = L.CGEventSourceCreate(1)

def key(kc, down):
    L.CGEventPost(0, L.CGEventCreateKeyboardEvent(src, kc, down))
    time.sleep(0.05)

# 基线 focus 次数
def focus_count():
    return len(re.findall(r'\[focus\] Tab', open("/tmp/rgui_demo.log").read()))

before = focus_count()
for _ in range(2):
    key(48, True); key(48, False)  # Tab down/up
    time.sleep(0.1)
time.sleep(0.5)
after = focus_count()
print(f"Tab x2 -> focus events: {after-before} (before={before} after={after})")
# 读最新 focus 行
lines = open("/tmp/rgui_demo.log").read().splitlines()
focus_lines = [l for l in lines if "[focus]" in l]
print("last focus lines:", focus_lines[-3:] if focus_lines else "(none)")
