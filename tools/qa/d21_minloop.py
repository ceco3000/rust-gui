#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D21 最小闭环 final：CGEvent 点击窗口标题栏激活 → move+点击 Accordion(物理像素) → 精确截图 id。
不依赖 osascript(会弹菜单)。窗口 id 从 swift 动态取(排除菜单,取 bounds 最大者)。"""
import ctypes, re, time, subprocess
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

def find_win_id():
    out=subprocess.run(["swift","-e",'import CoreGraphics; let l=CGWindowListCopyWindowInfo([.optionOnScreenOnly],kCGNullWindowID) as! [[String:Any]]; for w in l { if let n=w["kCGWindowOwnerName"] as? String, n=="window_demo" { let b=w["kCGWindowBounds"] as! [String:Int]; print(w[kCGWindowNumber as String]!, b["Width"]!) } }'],capture_output=True,text=True).stdout.split()
    # 取面积最大者(主窗口)
    best=None
    for i in range(0,len(out),2):
        wid=int(out[i]); w=int(out[i+1])
        if best is None or w>best[1]: best=(wid,w)
    return best[0] if best else None

wid=find_win_id()
print("window id:",wid)

src=L.CGEventSourceCreate(1)
log=open("/tmp/rgui_demo.log").read()
mf=re.search(r'\[win-frame\] origin=\(([-0-9.]+),\s*([-0-9.]+)\).*?scale=([-0-9.]+)',log)
mr=re.search(r'id=1 accordion rect=\(([-0-9.]+),\s*([-0-9.]+),\s*([-0-9.]+),\s*([-0-9.]+)\)',log)
ox,oy,scale=float(mf.group(1)),float(mf.group(2)),float(mf.group(3))
cx,cy=float(mr.group(1))+float(mr.group(3))/2.0, float(mr.group(2))+float(mr.group(4))/2.0

def click(pt):
    for t in [5,1,2]:
        L.CGEventPost(0,L.CGEventCreateMouseEvent(src,t,ctypes.pointer(pt),0)); time.sleep(0.06)

acc=CGPoint(ox+cx*scale, oy+cy*scale)
title=CGPoint(ox+200*scale, oy+10*scale)
print(f"title bar ({title.x:.0f},{title.y:.0f})  accordion ({acc.x:.0f},{acc.y:.0f})")

# 激活: 点标题栏
click(title); time.sleep(0.4)
# move + 点击 accordion
click(acc); print("clicked accordion")
time.sleep(0.6)

subprocess.run(["screencapture","-x","-l",str(wid),"/tmp/d21_final_click.png"],check=False)
print(f"shot /tmp/d21_final_click.png win={wid}")
