import Quartz
wins = Quartz.CGWindowListCopyWindowInfo(
    Quartz.kCGWindowListOptionOnScreenOnly | Quartz.kCGWindowListExcludeDesktopElements,
    Quartz.kCGNullWindowID)
for w in wins:
    n = str(w.get("kCGWindowOwnerName", ""))
    if "modal" in n or "d20" in n:
        b = w["kCGWindowBounds"]
        print("owner:", n, "id:", w.get("kCGWindowNumber"), "pid:", w.get("kCGWindowOwnerPID"),
              "bounds:", b["X"], b["Y"], b["Width"], b["Height"])
