#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D21 坐标单位实测：注入 mouseMoved 到候选坐标，读回 WindowServer 真实光标位置。
验证 CGEvent 坐标是 point 还是物理像素。不点击，最后把光标移回原位。"""
import Quartz
import time

src = Quartz.CGEventSourceCreate(Quartz.kCGEventSourceStateCombinedSessionState)

def cur():
    return Quartz.CGEventGetLocation(Quartz.CGEventCreate(None))

def move(x, y):
    ev = Quartz.CGEventCreateMouseEvent(src, Quartz.kCGEventMouseMoved, (x, y), Quartz.kCGMouseButtonLeft)
    Quartz.CGEventPost(Quartz.kCGHIDEventTap, ev)
    time.sleep(0.25)

orig = cur()
print(f"[1] 初始光标        : ({orig.x:.0f}, {orig.y:.0f})")

# 候选A：文档公式算出的 (1740,458)（物理像素体系）
move(1740.0, 458.0)
a = cur()
print(f"[2] 注入(1740,458) 后真实光标: ({a.x:.0f}, {a.y:.0f})")

# 候选B：point 体系 (870,257)（700+170, 235+22）
move(870.0, 257.0)
b = cur()
print(f"[3] 注入(870,257)  后真实光标: ({b.x:.0f}, {b.y:.0f})")

# 屏幕边界（point 单位）
bnd = Quartz.CGDisplayBounds(Quartz.CGMainDisplayID())
print(f"[4] 主屏 bounds    : {int(bnd.size.width)}x{int(bnd.size.height)} point")

# 恢复原位
move(orig.x, orig.y)
c = cur()
print(f"[5] 恢复光标        : ({c.x:.0f}, {c.y:.0f})")

# 结论
if (a.x, a.y) == (1740.0, 458.0):
    print(">>> 结论: CGEvent 坐标按 point 原样存储——(1740,458) 就是 point 坐标, 落在窗口右侧远处")
else:
    print(f">>> 结论: CGEvent 坐标发生了缩放/clamp——需进一步分析")
