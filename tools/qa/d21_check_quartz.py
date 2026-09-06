import Quartz, AppKit
print("Quartz/AppKit OK")
# 试读窗口
wins = Quartz.CGWindowListCopyWindowInfo(
    Quartz.kCGWindowListOptionOnScreenOnly | Quartz.kCGWindowListExcludeDesktopElements,
    Quartz.kCGNullWindowID)
for w in wins:
    if "window_demo" in str(w.get("kCGWindowOwnerName", "")):
        b = w["kCGWindowBounds"]
        print("bounds:", b["X"], b["Y"], b["Width"], b["Height"], "pid:", w.get("kCGWindowOwnerPID"))
        break
