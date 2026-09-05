#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D21 rgui 鼠标键盘自动化测试脚本。

职责（devco-qa）：读日志 → 坐标换算 → 真实注入 → 断言 → BUG 报告。
工作目录：/Users/chenchao/Documents/code/rust/RUST-GUI
前置：AXIsProcessTrusted()=True（已授权）；系统 python3（自带 pyobjc Quartz/AppKit）。

用法（示例）：
    python3 tools/qa/rgui_input_test.py --log <demo_stderr.log> \
        --inject-clicks accordion --expect '[action] toggle' \
        --screenshot tools/qa/d7_screenshots/d21_click.png

坐标换算（核心，2026-09-05 实测修正）：
    CGEvent 鼠标坐标单位是全局 **point**（非物理像素）。原公式「origin物理+rect×scale」
    在 Retina(scale=2) 下坐标放大 2 倍、点击落在窗口外（键盘无位置概念故不受影响——
    这解释了此前「键盘通、鼠标不通」）。
    正确公式：screen_point = 窗口bounds左上(point) + 标题栏高 + rect中心(逻辑point)
    窗口 bounds 与标题栏高均从 CGWindowList 运行时实测，无系数估算。
"""
import argparse, re, subprocess, sys, time, json, os

# ---------------- Quartz/AppKit（运行时窗口定位 + 激活） ----------------
try:
    import Quartz
    import AppKit
    _QUARTZ = True
except Exception as _e:  # noqa: BLE001
    _QUARTZ = False
    print(f"[warn] Quartz/AppKit 不可用: {_e}（无法运行时定位窗口，回退日志 win-frame）",
          file=sys.stderr)

# ---------------- CoreGraphics (ctypes) ----------------
try:
    import ctypes
    _AS = '/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices'
    _L = ctypes.CDLL(_AS)
    _L.AXIsProcessTrusted.restype = ctypes.c_bool
    _L.CGEventSourceCreate.restype = ctypes.c_void_p
    _L.CGEventSourceCreate.argtypes = [ctypes.c_int]
    class _CGPoint(ctypes.Structure):
        _fields_ = [("x", ctypes.c_double), ("y", ctypes.c_double)]
    _L.CGEventCreateMouseEvent.restype = ctypes.c_void_p
    # ⚠️ CGPoint 按值传递（C 签名是 CGPoint 非 CGPoint*）——传指针会读到寄存器垃圾(0,0/nan)
    _L.CGEventCreateMouseEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint32, _CGPoint, ctypes.c_uint32]
    _L.CGEventCreateKeyboardEvent.restype = ctypes.c_void_p
    _L.CGEventCreateKeyboardEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint16, ctypes.c_bool]
    _L.CGEventPost.restype = None
    _L.CGEventPost.argtypes = [ctypes.c_uint32, ctypes.c_void_p]
    _L.CGEventCreate.restype = ctypes.c_void_p
    _L.CGEventCreate.argtypes = [ctypes.c_void_p]
    _CG = True
except Exception as _e:  # noqa: BLE001
    _CG = False
    print(f"[warn] CoreGraphics 加载失败: {_e}，注入禁用（仅解析/断言）", file=sys.stderr)

# 事件类型常量
KCG_HID = 0
MOUSE_MOVED, MOUSE_LEFT_DOWN, MOUSE_LEFT_UP, MOUSE_RIGHT_DOWN, MOUSE_RIGHT_UP = 5, 1, 2, 3, 4
KEY_DOWN, KEY_UP = 10, 11
TAB_KEYCODE = 48  # macOS Tab 键 keycode；Shift+Tab 需 flags shift=0x20000
SHIFT_FLAG = 1 << 17  # kCGEventFlagMaskShift = 0x20000


def ax_trusted() -> bool:
    """AX 授权状态（真实注入的前提）。"""
    if not _CG:
        return False
    try:
        return bool(_L.AXIsProcessTrusted())
    except Exception:
        return False


# ---------------- 日志解析（容错） ----------------
def parse_hit_region(line):
    """从日志行解析 `[hit-region] id=.. name rect=(x,y,w,h)`，容错字段顺序。

    日志格式：`[hit-region] id=1 accordion rect=(0,0,340,44)`——id 后跟组件名。
    返回 dict 含 id、name（无名字则为 None）、rect、center。
    """
    m = re.search(r'id\s*=\s*(\w+)', line)
    r = re.search(r'rect\s*=\s*\(([-0-9.]+)\s*,\s*([-0-9.]+)\s*,\s*([-0-9.]+)\s*,\s*([-0-9.]+)\)', line)
    if not (m and r):
        return None
    x, y, w, h = (float(r.group(i)) for i in range(1, 5))
    # id 之后、rect 之前第一个单词 = 组件名（如 accordion / wabadge）
    nm = re.search(r'id\s*=\s*\w+\s+([A-Za-z0-9_\-]+)', line)
    return {
        "id": m.group(1),
        "name": nm.group(1) if nm else None,
        "rect": (x, y, w, h),
        "center": (x + w / 2.0, y + h / 2.0),
    }


def parse_win_frame(line):
    """从日志行解析 `[win-frame] origin=(x,y) size=(w,h) scale=f`，容错。"""
    o = re.search(r'origin\s*=\s*\(([-0-9.]+)\s*,\s*([-0-9.]+)\)', line)
    s = re.search(r'size\s*=\s*\(([-0-9.]+)\s*,\s*([-0-9.]+)\)', line)
    sc = re.search(r'scale\s*=\s*([-0-9.]+)', line)
    if not (o and s):
        return None
    dx, dy = float(o.group(1)), float(o.group(2))
    sx, sy = float(s.group(1)), float(s.group(2))
    scale = float(sc.group(1)) if sc else 1.0
    return {"origin": (dx, dy), "size": (sx, sy), "scale": scale}


def read_log(path, since_ts=None):
    """读日志文本（全量或增量——路径为文件则读全部，若为流则 TODO）。"""
    if not path or not os.path.exists(path):
        return ""
    with open(path, "r", encoding="utf-8", errors="replace") as f:
        return f.read()


# ---------------- 坐标换算（核心） ----------------
def find_window_bounds(owner_substr="window_demo"):
    """CGWindowList 实时读窗口 bounds（point，含标题栏）+ owner PID。

    返回 ((x, y, w, h), pid)；未找到返回 (None, None)。运行时值 > 启动时日志值：
    窗口移动后日志 win-frame 过期，CGWindowList 永远准。
    """
    if not _QUARTZ:
        return None, None
    wins = Quartz.CGWindowListCopyWindowInfo(
        Quartz.kCGWindowListOptionOnScreenOnly | Quartz.kCGWindowListExcludeDesktopElements,
        Quartz.kCGNullWindowID,
    )
    for w in wins:
        if owner_substr in str(w.get("kCGWindowOwnerName", "")):
            b = w["kCGWindowBounds"]
            return (b["X"], b["Y"], b["Width"], b["Height"]), w.get("kCGWindowOwnerPID")
    return None, None


def activate_app(pid):
    """激活目标进程（synthetic 点击不激活非活动窗口的 workaround，macOS 26+ 实测必需）。"""
    if not _QUARTZ or not pid:
        return False
    app = AppKit.NSRunningApplication.runningApplicationWithProcessIdentifier_(pid)
    return bool(app.activateWithOptions_(AppKit.NSApplicationActivateIgnoringOtherApps))


def raise_window(pid):
    """AX raise 窗口到最前端（activate 只激活应用进程，窗口仍可能被其他窗口遮挡）。"""
    if not pid:
        return False
    r = subprocess.run(
        ["osascript", "-e",
         f'tell application "System Events" to tell (first process whose unix id is {pid}) '
         'to perform action "AXRaise" of window 1'],
        capture_output=True, text=True,
    )
    return r.returncode == 0


def to_screen(win_frame, hit, bounds=None):
    """屏幕绝对坐标(point) = 窗口bounds左上 + 标题栏高 + rect中心。

    CGEvent 坐标单位是全局 point；窗口 bounds(point, 含标题栏) 与 CGEvent 同体系。
    标题栏高 = bounds.h - 内容高(win_frame.size.y/scale)，实测值非估算。
    回退（无 CGWindowList）：win_frame origin 物理像素 ÷ scale + rect 中心。
    """
    cx, cy = hit["center"]
    if bounds:
        bx, by, _, bh = bounds
        scale = win_frame.get("scale", 1.0)
        content_h = win_frame["size"][1] / scale if scale else 0.0
        titlebar = bh - content_h
        return (bx + cx, by + titlebar + cy)
    ox, oy = win_frame["origin"]
    scale = win_frame.get("scale", 1.0) or 1.0
    return (ox / scale + cx, oy / scale + cy)


# ---------------- CGEvent 注入 ----------------
def _post_mouse(pt, ev_type):
    if not _CG:
        return False
    ev = _L.CGEventCreateMouseEvent(_L.CGEventSourceCreate(1), ev_type, _CGPoint(*pt), 0)
    _L.CGEventPost(KCG_HID, ev)
    return True


def _post_key(keycode, down, shift=False):
    if not _CG:
        return False
    src = _L.CGEventSourceCreate(1)
    ev = _L.CGEventCreateKeyboardEvent(src, keycode, down)
    # 设置 Shift flags（若 shift=True）
    if shift:
        _L.CGEventSetIntegerValueField(ev, 22, int(SHIFT_FLAG))  # 22 = kCGKeyboardEventKeyFlags
    _L.CGEventPost(KCG_HID, ev)
    return True


def click_at(pt):
    """在屏幕绝对坐标 pt 处点击（move + leftDown + leftUp）。"""
    _post_mouse(pt, MOUSE_MOVED)
    time.sleep(0.05)
    _post_mouse(pt, MOUSE_LEFT_DOWN)
    time.sleep(0.05)
    _post_mouse(pt, MOUSE_LEFT_UP)
    time.sleep(0.1)


def tab_key(shift=False):
    _post_key(TAB_KEYCODE, True, shift)
    time.sleep(0.05)
    _post_key(TAB_KEYCODE, False, shift)
    time.sleep(0.1)


# ---------------- 断言 / BUG 报告 ----------------
def assert_found(log_text, needle, label):
    """注入后日志是否出现预期事件。返回 (bool, 输出)。"""
    found = needle in log_text
    return found, f"{'✓' if found else '✗'} {label}: 期望 '...{needle}...' {'出现' if found else '未出现'}"


def bug_report(case, coords, expected, actual, screenshot=None, extra=None):
    """结构化 BUG 报告（回馈 dev）。"""
    report = {
        "case": case,
        "inject_coords": coords,
        "expected_event": expected,
        "actual_event": actual,
        "screenshot": screenshot,
        "note": extra or "",
        "ts": time.strftime("%Y-%m-%d %H:%M:%S"),
    }
    return json.dumps(report, ensure_ascii=False, indent=2)


# ---------------- 主流程 ----------------
def main():
    ap = argparse.ArgumentParser(description="rgui 鼠标键盘自动化测试（D21）")
    ap.add_argument("--log", help="demo 输出日志文件（stdout/stderr 重定向）")
    ap.add_argument("--inject-clicks", nargs="+",
                    help="命中 id 列表（从 [hit-region] 取 rect 中心点击），如 accordion badge")
    ap.add_argument("--tab", type=int, default=0, help="Tab 次数（focus_next）")
    ap.add_argument("--shift-tab", type=int, default=0, help="Shift+Tab 次数（focus_prev）")
    ap.add_argument("--expect", nargs="+", help="注入后应出现的日志子串（断言）")
    ap.add_argument("--screenshot", help="可选：截图保存路径（screencapture -l 窗口id 或全屏）")
    ap.add_argument("--win-id", help="窗口 id（截图用）")
    ap.add_argument("--dry-run", action="store_true", help="只解析+换算，不注入")
    a = ap.parse_args()

    print(f"[D21] AX trusted: {ax_trusted()} | CG 加载: {_CG}", file=sys.stderr)

    # 1. 读日志，收集 hit-region + win-frame
    text = read_log(a.log)
    hits = [parse_hit_region(l) for l in text.splitlines()]
    hits = [h for h in hits if h]
    frames = [parse_win_frame(l) for l in text.splitlines()]
    frames = [fr for fr in frames if fr]
    if not hits:
        print("[warn] 未解析到 [hit-region] —— 等 dev 打日志，或检查 --log 路径", file=sys.stderr)
    if not frames:
        print("[warn] 未解析到 [win-frame]，无法换算坐标（设 scale=1 origin=(0,0) 兜底）", file=sys.stderr)
        frames = [{"origin": (0, 0), "size": (0, 0), "scale": 1.0}]
    wf = frames[-1]
    print(f"[parsed] {len(hits)} hit-region | frame={wf}", file=sys.stderr)

    # 2. 运行时窗口定位 + 激活 + raise（窗口必须在最前端，否则点击落在遮挡窗口上）
    bounds, pid = find_window_bounds()
    if bounds:
        print(f"[window] bounds(point)={bounds} pid={pid}", file=sys.stderr)
        if pid and not a.dry_run:
            activated = activate_app(pid)
            raised = raise_window(pid)
            print(f"[window] activate={activated} raise={raised}", file=sys.stderr)
            time.sleep(0.5)
    else:
        print("[warn] 未在 CGWindowList 找到 window_demo 窗口（回退日志 win-frame 换算）",
              file=sys.stderr)

    # 3. 换算 + 注入（真实点击）
    inject_before = len(read_log(a.log).splitlines()) if a.log else 0
    clicked = []
    for hid in (a.inject_clicks or []):
        hit = next((h for h in hits if h["id"] == hid or h["name"] == hid), None)
        if not hit:
            print(f"[warn] id/name '{hid}' 未命中 hit-region", file=sys.stderr)
            continue
        pt = to_screen(wf, hit, bounds)
        print(f"[calc] id={hid} center={hit['center']} scale={wf['scale']} -> screen(point) {pt}",
              file=sys.stderr)
        if not a.dry_run:
            click_at(pt)
        clicked.append({"id": hid, "screen": pt, "rect_center": hit["center"]})

    # 4. Tab / Shift+Tab
    for _ in range(a.tab):
        if not a.dry_run:
            tab_key(False)
    for _ in range(a.shift_tab):
        if not a.dry_run:
            tab_key(True)

    # 5. 截图（可选）
    if a.screenshot and a.win_id:
        subprocess.run(["screencapture", "-x", "-l", str(a.win_id), a.screenshot],
                       check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    # 6. 增量断言（只看本次注入后新增行，避免历史日志误通过）
    after_lines = read_log(a.log).splitlines() if a.log else []
    fresh = "\n".join(after_lines[inject_before:])
    ok = True
    for needle in (a.expect or []):
        f, line = assert_found(fresh, needle, "断言")
        print(line)
        ok = ok and f

    # 7. BUG 报告（期望 vs 实际）
    if clicked and (a.expect) and not ok:
        actual = "期望事件未出现在日志"
        print("\n[BUG REPORT]\n" + bug_report(
            case=f"click:{','.join(a.inject_clicks)}",
            coords=[c["screen"] for c in clicked],
            expected=a.expect,
            actual=actual,
            screenshot=a.screenshot,
        ))


if __name__ == "__main__":
    main()
