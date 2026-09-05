#!/usr/bin/env python3
# D21 注入链路验证：CGEvent mouseMoved/leftMouseDown/Up + Tab 键盘注入（ctypes）
import ctypes, time, struct

AS = '/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices'
L = ctypes.CDLL(AS)
L.CGEventSourceCreate.restype = ctypes.c_void_p
L.CGEventSourceCreate.argtypes = [ctypes.c_int]
L.CGEventCreateMouseEvent.restype = ctypes.c_void_p
L.CGEventCreateMouseEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_void_p, ctypes.c_uint32]
L.CGEventPost.restype = None
L.CGEventPost.argtypes = [ctypes.c_uint32, ctypes.c_void_p]
L.CGEventCreateKeyboardEvent.restype = ctypes.c_void_p
L.CGEventCreateKeyboardEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint16, ctypes.c_bool]
L.CGEventSetIntegerValueField.restype = None
L.CGEventSetIntegerValueField.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_int64]
L.CGEventCreate.restype = ctypes.c_void_p
L.CGEventCreate.argtypes = [ctypes.c_void_p]

# CGPoint struct (f64 x2)
class CGPoint(ctypes.Structure):
    _fields_ = [("x", ctypes.c_double), ("y", ctypes.c_double)]

src = L.CGEventSourceCreate(1)
print("source:", src)

# ---- 鼠标移动到 (500,400) 并点击 ----
# kCGEventMouseMoved=5, kCGEventLeftMouseDown=1, kCGEventLeftMouseUp=2
pt = CGPoint(500.0, 400.0)
ptr = ctypes.pointer(pt)
for ev_type, name in [(5, "MouseMoved"), (1, "LeftDown"), (2, "LeftUp")]:
    ev = L.CGEventCreateMouseEvent(src, ev_type, ptr, 0)
    # kCGHIDEventTap=0
    L.CGEventPost(0, ev)
    print(f"posted {name} -> {ev}")
    time.sleep(0.05)

# ---- 键盘 Tab (keycode=48) down/up ----
# kCGEventKeyDown=10, kCGEventKeyUp=11
for ev_type, down in [(10, True), (11, False)]:
    ev = L.CGEventCreateKeyboardEvent(src, 48, down)
    L.CGEventPost(0, ev)
    print(f"posted keyboard type={ev_type} down={down} -> {ev}")
    time.sleep(0.05)

print("injection done")
