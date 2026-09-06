import Quartz
wins = Quartz.CGWindowListCopyWindowInfo(
    Quartz.kCGWindowListOptionOnScreenOnly | Quartz.kCGWindowListExcludeDesktopElements,
    Quartz.kCGNullWindowID)
for w in wins:
    if "window_demo" in str(w.get("kCGWindowOwnerName", "")):
        print(w["kCGWindowNumber"])
        break
