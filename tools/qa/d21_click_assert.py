#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D21 真实点击 Accordion 最小闭环：CGEvent 激活+点击(1740,458) → 读日志断言 [mouse-event]/[hit] id=1/[action] toggle。"""
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

def tail_lines():
    try: return open("/tmp/rgui_demo.log").read().splitlines()
    except FileNotFoundError: return []

def last_idx(patterns):
    lines=tail_lines()
    for i in range(len(lines)-1,-1,-1):
        if any(p in lines[i] for p in patterns): return i
    return -1

def log_find(pattern, from_idx=0):
    lines=tail_lines()
    # 记录本次点击后新增行(基于开始时的日志行数)
    return [l for l in lines[from_idx:] if re.search(pattern,l)]

start=len(tail_lines())
print("AX trusted:", L.AXIsProcessTrusted())
src=L.CGEventSourceCreate(1)

# 点击
def click(pt):
    for t in [5,1,2]:
        L.CGEventPost(0,L.CGEventCreateMouseEvent(src,t,ctypes.pointer(pt),0)); time.sleep(0.06)

# win-frame(物理像素) + accordion center
log="\n".join(tail_lines())
mf=re.search(r'\[win-frame\] origin=\(([-0-9.]+),\s*([-0-9.]+)\).*?scale=([-0-9.]+)',log)
mr=re.search(r'id=1 accordion rect=\(([-0-9.]+),\s*([-0-9.]+),\s*([-0-9.]+),\s*([-0-9.]+)\)',log)
ox,oy,scale=float(mf.group(1)),float(mf.group(2)),float(mf.group(3))
cx,cy=float(mr.group(1))+float(mr.group(3))/2.0, float(mr.group(2))+float(mr.group(4))/2.0
acc=CGPoint(ox+cx*scale, oy+cy*scale)
title=CGPoint(ox+200*scale, oy+10*scale)
print(f"accordion screen ({acc.x:.0f},{acc.y:.0f})")

# 激活(点标题栏)+点击 accordion
click(title); time.sleep(0.4)
click(acc); time.sleep(0.6)

# 断言: 读新增日志
new_log="\n".join(tail_lines()[start:])
print("=== 点击后新增日志 ===")
for pat in [r'mouse-event', r'hit\] id=1', r'action\] toggle', r'hit\] id=none', r'hit\] id=2']:
    got=[l for l in tail_lines()[start:] if re.search(pat,l)]
    print(f"  {pat}: {'✓ ' if got else ''}{got or '(无)'}")
