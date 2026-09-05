#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""关掉可能弹出的 Apple 菜单(Esc), 列出所有 window_demo 窗口 id, 精确定位最顶/最大者。"""
import ctypes, subprocess, time
AS='/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices'
L=ctypes.CDLL(AS)
L.AXIsProcessTrusted.restype=ctypes.c_bool
L.CGEventSourceCreate.restype=ctypes.c_void_p
L.CGEventCreateKeyboardEvent.restype=ctypes.c_void_p
L.CGEventCreateKeyboardEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint16,ctypes.c_bool]
L.CGEventPost.restype=None
L.CGEventPost.argtypes=[ctypes.c_uint32,ctypes.c_void_p]
src=L.CGEventSourceCreate(1)
# Esc keycode=53
L.CGEventPost(0,L.CGEventCreateKeyboardEvent(src,53,True)); time.sleep(0.05)
L.CGEventPost(0,L.CGEventCreateKeyboardEvent(src,53,False)); time.sleep(0.3)
print("sent Esc to close menu")
# 列出 window_demo 窗口
out=subprocess.run(["swift","-e",'import CoreGraphics; let l=CGWindowListCopyWindowInfo([.optionOnScreenOnly],kCGNullWindowID) as! [[String:Any]]; for w in l { if let n=w["kCGWindowOwnerName"] as? String, n=="window_demo" { let b=w["kCGWindowBounds"] as! [String:Int]; print("id:",w[kCGWindowNumber as String]!, "bounds:",b["X"]!,b["Y"]!,b["Width"]!,"x",b["Height"]!) } }'],capture_output=True,text=True).stdout.strip()
print("window_demo windows:\n"+out)
