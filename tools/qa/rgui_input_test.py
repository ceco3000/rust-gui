#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""rgui 鼠标键盘自动化测试（D21-2 分层诊断 + 场景化）。devco-qa 主实现。

零 LLM / 零 vision 判断——全部用确定性日志信号匹配，分层诊断定位"问题在哪一层"。

核心：
- detect_layer(log_snapshot, case) -> (layer, evidence)：按 L1→L5 顺次检查，首个失败层 = 诊断结论。
  L1 注入层   脚本自检：AX trusted / CG 加载 / 注入发出
  L2 窗口层   winit 收到事件？[mouse-event] left-press at logical=(x,y) / [focus] Tab
  L3 命中层   hit_test 命中正确组件？[hit] id=1 / id=2 / id=none(missed)
  L4 动作层   组件状态更新？[action] toggle / badge_click(count=N) / [focus] Some(id)
  L5 功能层   二次注入验证持久（toggle 往返 / badge count 递增 / focus 移动）
- 预检 fail-fast：注入前 AX trusted / CG 加载 / 窗口定位，区分"脚本环境问题 vs 产品 bug"。
- 场景化用例 T1-T7：--case <T1..T7> 单个 / --all 全量；退出码 0 全过 / 1 有失败。
- BUG 报告：fail_layer + 该层证据 + 建议排查项，截图仅存证（人工可审）。

坐标换算（D21-2 实测修正）：CGEvent 鼠标坐标单位是全局 **point**。
  screen_point = 窗口bounds左上(point) + 标题栏高 + rect中心(逻辑point)；窗口 bounds/标题栏高用 Quartz 运行时实测。
⚠️ CGEventCreateMouseEvent 的 CGPoint 必须**按值传递**（传指针会读到寄存器垃圾 0,0/nan → 鼠标点击无效）。

用法：
  python3 tools/qa/rgui_input_test.py --log /tmp/rgui_demo.log --case T1
  python3 tools/qa/rgui_input_test.py --log /tmp/rgui_demo.log --all
"""
import argparse, re, subprocess, sys, time, json, os

# ---------------- Quartz/AppKit（运行时窗口定位 + 激活/raise） ----------------
try:
    import Quartz
    import AppKit
    _QUARTZ = True
except Exception as _e:  # noqa: BLE001
    _QUARTZ = False
    print(f"[warn] Quartz/AppKit 不可用: {_e}（无法运行时定位窗口，回退日志 win-frame）", file=sys.stderr)

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
    # ⚠️ CGPoint 按值传递（C 签名是 CGPoint 非 CGPoint*）
    _L.CGEventCreateMouseEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint32, _CGPoint, ctypes.c_uint32]
    _L.CGEventCreateKeyboardEvent.restype = ctypes.c_void_p
    _L.CGEventCreateKeyboardEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint16, ctypes.c_bool]
    _L.CGEventPost.restype = None
    _L.CGEventPost.argtypes = [ctypes.c_uint32, ctypes.c_void_p]
    _L.CGEventSetIntegerValueField.restype = None
    _L.CGEventSetIntegerValueField.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_int64]
    _L.CGEventCreate.restype = ctypes.c_void_p
    _L.CGEventCreate.argtypes = [ctypes.c_void_p]
    _CG = True
except Exception as _e:  # noqa: BLE001
    _CG = False
    print(f"[warn] CoreGraphics 加载失败: {_e}，注入禁用（仅解析/断言）", file=sys.stderr)

KCG_HID = 0
MOUSE_MOVED, MOUSE_LEFT_DOWN, MOUSE_LEFT_UP, MOUSE_RIGHT_DOWN, MOUSE_RIGHT_UP = 5, 1, 2, 3, 4
KEY_DOWN, KEY_UP = 10, 11
TAB_KEYCODE = 48
SHIFT_FLAG = 1 << 17


def ax_trusted():
    if not _CG:
        return False
    try:
        return bool(_L.AXIsProcessTrusted())
    except Exception:
        return False


# ---------------- 日志解析（容错） ----------------
def parse_hit_region(line):
    m = re.search(r'id\s*=\s*(\w+)', line)
    r = re.search(r'rect\s*=\s*\(([-0-9.]+)\s*,\s*([-0-9.]+)\s*,\s*([-0-9.]+)\s*,\s*([-0-9.]+)\)', line)
    if not (m and r):
        return None
    x, y, w, h = (float(r.group(i)) for i in range(1, 5))
    nm = re.search(r'id\s*=\s*\w+\s+([A-Za-z0-9_\-]+)', line)
    return {"id": m.group(1), "name": nm.group(1) if nm else None,
            "rect": (x, y, w, h), "center": (x + w / 2.0, y + h / 2.0)}


def parse_win_frame(line):
    o = re.search(r'origin\s*=\s*\(([-0-9.]+)\s*,\s*([-0-9.]+)\)', line)
    s = re.search(r'size\s*=\s*\(([-0-9.]+)\s*,\s*([-0-9.]+)\)', line)
    sc = re.search(r'scale\s*=\s*([-0-9.]+)', line)
    if not (o and s):
        return None
    return {"origin": (float(o.group(1)), float(o.group(2))),
            "size": (float(s.group(1)), float(s.group(2))),
            "scale": float(sc.group(1)) if sc else 1.0}


def read_log(path):
    if not path or not os.path.exists(path):
        return ""
    with open(path, "r", encoding="utf-8", errors="replace") as f:
        text = f.read()
    # D22：剥 tracing 前缀（若含），保持现有正则零改动。无前缀则 no-op。
    if text.splitlines() and re.search(r'rgui_test_signal|INFO|WARN|ERROR.*\b', text.splitlines()[0]):
        stripped = "\n".join(_strip_tracing(l) for l in text.splitlines())
        return stripped
    return text


# ---------------- 窗口定位 + 激活/raise ----------------
def find_window_bounds(owner_substr="window_demo"):
    if not _QUARTZ:
        return None, None
    try:
        wins = Quartz.CGWindowListCopyWindowInfo(
            Quartz.kCGWindowListOptionOnScreenOnly | Quartz.kCGWindowListExcludeDesktopElements,
            Quartz.kCGNullWindowID)
    except Exception:
        return None, None
    for w in wins:
        if owner_substr in str(w.get("kCGWindowOwnerName", "")):
            b = w["kCGWindowBounds"]
            return (b["X"], b["Y"], b["Width"], b["Height"]), w.get("kCGWindowOwnerPID")
    return None, None


def find_window_id(owner_substr="window_demo"):
    if not _QUARTZ:
        return None
    try:
        wins = Quartz.CGWindowListCopyWindowInfo(
            Quartz.kCGWindowListOptionOnScreenOnly | Quartz.kCGWindowListExcludeDesktopElements,
            Quartz.kCGNullWindowID)
    except Exception:
        return None
    for w in wins:
        if owner_substr in str(w.get("kCGWindowOwnerName", "")):
            return w.get("kCGWindowNumber")
    return None


def activate_app(pid):
    if not _QUARTZ or not pid:
        return False
    app = AppKit.NSRunningApplication.runningApplicationWithProcessIdentifier_(pid)
    return bool(app.activateWithOptions_(AppKit.NSApplicationActivateIgnoringOtherApps))


def raise_window(pid):
    if not pid:
        return False
    r = subprocess.run(["osascript", "-e",
        f'tell application "System Events" to tell (first process whose unix id is {pid}) '
        'to perform action "AXRaise" of window 1'],
        capture_output=True, text=True)
    return r.returncode == 0


def to_screen(win_frame, hit, bounds=None):
    cx, cy = hit["center"]
    if bounds:
        bx, by, _, bh = bounds
        scale = win_frame.get("scale", 1.0)
        content_h = win_frame["size"][1] / scale if scale else 0.0
        titlebar = bh - content_h
        return (bx + cx, by + titlebar + cy)
    oy = win_frame.get("scale", 1.0) or 1.0
    ox, oy0 = win_frame["origin"]
    return (ox / oy + cx, oy0 / oy + cy)


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
    if shift:
        _L.CGEventSetIntegerValueField(ev, 22, int(SHIFT_FLAG))
    _L.CGEventPost(KCG_HID, ev)
    return True


def click_at(pt):
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


# ---------------- 分层诊断（核心） ----------------
# D22 tracing 前缀剥离（可选：dev 用 tracing info!(target="rgui_test_signal", message=<token>)
# 且 fmt 渲染含级别/时间戳前缀时启用；若 message=原 token 零前缀则此函数 no-op）。
# 集中在此，不扩散到各 parse（保持 detect_layer 等正则零改动）。
import re as _re
_TRACING_RE = _re.compile(r'^.*?(?:rgui_test_signal|rgui)\s*[:=]\s*')  # 剥 "2026-.. INFO rgui_test_signal: message=" 前缀


def _strip_tracing(line):
    """剥掉 tracing fmt 前缀（时间戳/级别/target: message=），保留 message 原文供现有正则匹配。
    若 line 不含前缀则原样返回（零改动路径 no-op）。"""
    if not line:
        return line
    m = _re.match(r'^(?:\d{4}-\d{2}-\d{2}.*?)?(?:TRACE|DEBUG|INFO|WARN|ERROR)\S*\s*(?:\w*[_\w]*\s*[:=]\s*)*', line)
    if m:
        return line[m.end():]  # 剥到 message 正文
    # 兜底：剥 "rgui_test_signal:" / "target=..:" 前缀
    m2 = _re.match(r'^.*?(?:rgui_test_signal|message)[:=]\s*', line)
    if m2:
        return line[m2.end():]
    return line


# 按 fail_layer 生成纯文本排查建议（P2-3：建议无论如何不悬空）
SUGGEST_BY_LAYER = {
    "L1": "脚本环境问题：检查 AXIsProcessTrusted() 是否授权(系统设置→隐私与安全→辅助功能)；CoreGraphics 是否可加载(依赖 ApplicationServices)；确认不是脚本运行环境问题。",
    "L2": "窗口/坐标层：①确认注入前已 activate_app + raise_window(窗口须最前端,否则点击落遮挡窗口) ②检查坐标换算是否为全局 point(bounds左上+titlebar+rect中心), 勿用物理像素 ③in-region=false 说明点到了窗口外——重算坐标或确认窗口 bounds 未变 ④CGPoint 是否按值传递(指针会读到0,0/nan)。",
    "L3": "命中层：in-region=true 但 [hit] id=none → 检查 hit-region rect 是否与组件实际渲染/命中区域一致(dev 检查 rect 定义)；坐标是否落在组件 rect 内；hit_test 逻辑是否与 rect 匹配。",
    "L4": "动作层：组件状态未更新——检查 dev 的 update/事件响应链路(如 AccordionMsg::Toggle 是否真正处理)、组件 view/update 是否返回新状态。",
    "L5": "功能层：状态不能二次往返/持久——检查组件状态管理、是否每次注入都产生预期动作(如 toggle 往返需 toggle 出现2次、badge 需 count 递增、焦点需能移动)。",
}
SUGGEST_BY_LAYER["PASS"] = "全部层通过，无问题。"


def detect_layer(log_snapshot, case="T1"):
    """按 L1→L5 顺次检查，返回 (fail_layer, evidence_map, status_map)。

    L1 注入层：脚本自检（CG 加载 / AX trusted）
    L2 窗口层：winit 收到事件（[mouse-event] / [focus]）；**in-region=false → 坐标换算错/点窗外**
    L3 命中层：hit_test 命中（in-region=true 但 [hit] id=none → rect 边界不一致）
    L4 动作层：组件状态更新
    L5 功能层：二次注入验证持久
    首个失败层 = 诊断结论；全过 → ("PASS", ...)。
    """
    # L1 注入层：脚本自检
    l1_ok = _CG and ax_trusted()
    l1_ev = f"CG加载={_CG} AX trusted={ax_trusted()}"
    # L2 窗口层：winit 收到事件 + in-region 消费
    has_mouse_m = re.search(r'\[mouse-event\] left-press at logical=\(([-0-9.]+),\s*([-0-9.]+)\) in-region=(true|false)',
                            log_snapshot)
    has_focus = bool(re.search(r'\[focus\] Tab', log_snapshot))
    in_region = has_mouse_m.group(3) if has_mouse_m else None
    # 无 mouse-event 且无 focus → winit 未处理；in-region=false → 点窗外(坐标换算错) → L2 失败(注入坐标问题)
    if has_focus and not has_mouse_m:
        l2_ok = True
        l2_ev = "winit 收到 [focus] Tab(键盘场景)"
    elif has_mouse_m and in_region == "false":
        if case == "T5":
            l2_ok = True  # T5 负向：故意点组件区外，in-region=false 属预期，不归 L2
            l2_ev = "[mouse-event] in-region=false(负向 T5 预期点组件区外，不作为 L2 失败)"
        else:
            l2_ok = False  # 正向场景点窗口外(坐标换算错) → L2 失败
            l2_ev = "[mouse-event] left-press in-region=false —— 坐标换算错/点窗口外(归 L2 注入坐标问题)"
    elif in_region == "true":
        l2_ok = True
        l2_ev = "winit 收到 [mouse-event] in-region=true(事件到达窗口内)"
    elif has_mouse_m:
        l2_ok = True
        l2_ev = "winit 收到 [mouse-event](in-region 未解析)"
    else:
        l2_ok = False
        l2_ev = "无 [mouse-event]/[focus] —— winit 未处理"

    # 场景关键信号
    if case in ("T1", "T6"):
        l3_ok = bool(re.search(r'\[hit\] id=1', log_snapshot))
        if in_region == "true" and not l3_ok:
            l3_ev = "in-region=true 但无 [hit] id=1 —— rect 边界不一致(组件命中区与渲染/事件不一致)"
        elif l3_ok:
            l3_ev = f"hit id=1(Accordion) in-region={in_region}"
        else:
            l3_ev = ("[hit] id=none(missed)" if bool(re.search(r'\[hit\] id=none', log_snapshot)) else "无 [hit]")
        l4_ok = bool(re.search(r'\[action\] toggle\(id=1\)', log_snapshot))
        l4_ev = "[action] toggle(id=1) 出现" if l4_ok else "无 [action] toggle —— 组件未更新"
        toggles = len(re.findall(r'\[action\] toggle\(id=1\)', log_snapshot))
        l5_ok = toggles >= (2 if case == "T6" else 1)  # T6 需两次往返, T1 一次
        l5_ev = f"toggle 出现 {toggles} 次 (T6 需>=2/T1 需>=1)"
    elif case in ("T2", "T3"):
        l3_ok = has_focus
        l3_ev = "focus 事件(键盘无 hit 命中概念)" if has_focus else "无 focus 事件"
        l4_ok = bool(re.search(r'\[focus\] Tab\(shift=\w+\) -> Some\(\d+\)', log_snapshot))
        l4_ev = "[focus] Tab -> Some(id) 出现" if l4_ok else "无 [focus] -> Some(id)"
        foci = re.findall(r'\[focus\] Tab\(shift=(\w+)\) -> (Some|None)\((\d+)\)', log_snapshot)
        l5_ok = len(foci) >= 2
        l5_ev = f"focus 事件 {len(foci)} 次 (需>=2 才能验证移动)" if foci else "无 focus 移动证据"
    elif case == "T4":
        l3_ok = bool(re.search(r'\[hit\] id=2', log_snapshot))
        l3_ev = "hit id=2(WaBadge)" if l3_ok else ("[hit] id=none(missed)" if bool(re.search(r'\[hit\] id=none', log_snapshot)) else "无 [hit] id=2")
        counts = [int(c) for c in re.findall(r'\[action\] badge_click\(id=2,count=(\d+)\)', log_snapshot)]
        l4_ok = len(counts) >= 1
        l4_ev = f"badge_click 出现 {len(counts)} 次, count序列={counts}" if counts else "无 badge_click —— 组件未更新"
        l5_ok = len(counts) >= 2 and counts[-1] > counts[0]
        l5_ev = f"count 递增 {counts} (需>=2 且递增)" if counts else "无 badge 计数递增"
    elif case == "T5":  # 负向：预期未命中（点组件区外）→ 以 [hit] id=none 为判据
        # 负向特判：T5 故意点组件区外，in-region=false 属预期（组件区≈content, 非组件点即出区），不作为失败信号
        # 判据 = [hit] id=none（负向达成）；in-region 不参与 T5 判定（见 D21-2 说明 P2-1 依据）
        l3_ok = bool(re.search(r'\[hit\] id=none', log_snapshot))
        l3_ev = "[hit] id=none (missed hit-region) —— 负向预期达成" if l3_ok else "意外命中(负向场景应未命中)"
        l4_ok = l3_ok
        l4_ev = "负向场景：未命中即为正确" if l3_ok else "意外命中某组件(负向失败)"
        l5_ok = l3_ok
        l5_ev = "负向场景达成" if l3_ok else "负向场景未达成"
    elif case == "T7":  # 模态焦点(d20_modal): modal_open + 焦点隔离 + 模态内循环
        l3_ok = bool(re.search(r'\[action\] modal_open', log_snapshot))
        l3_ev = "[action] modal_open 出现" if l3_ok else "无 [action] modal_open —— 点击未打开模态"
        l4_ok = bool(re.search(r'\[focus\] click -> Some\(200\)', log_snapshot))
        l4_ev = "[focus] click -> Some(200) 焦点隔离到模态" if l4_ok else "无 [focus] click -> Some(200) —— 焦点未隔离到模态"
        # L5: 模态内 Tab 循环(焦点应仍 Some(200) 不逃到 base 100/101) + close 恢复
        tabs = re.findall(r'\[focus\] Tab -> (Some|None)\(\d+\)', log_snapshot)
        l5_focus = bool(re.search(r'\[focus\] Tab -> Some\(200\)', log_snapshot)) if tabs else False
        l5_ok = l5_focus
        l5_ev = ("模态内 Tab 循环到 Some(200)(焦点不逃到 base)" if l5_focus else
                 f"模态内 Tab 焦点序列 {tabs} —— 需全部落在模态集合(200)")
    else:
        l3_ok = l2_ok
        l3_ev = "未知场景，命中层按窗口层"
        l4_ok = l2_ok
        l4_ev = "未知场景"
        l5_ok = l2_ok
        l5_ev = "未知场景"

    status = {
        "L1": {"ok": l1_ok, "ev": l1_ev},
        "L2": {"ok": l2_ok, "ev": l2_ev},
        "L3": {"ok": l3_ok, "ev": l3_ev},
        "L4": {"ok": l4_ok, "ev": l4_ev},
        "L5": {"ok": l5_ok, "ev": l5_ev},
    }
    for l, s in status.items():
        if not s["ok"]:
            return (l, s["ev"], status)
    return ("PASS", "全部层通过", status)


# ---------------- 场景用例 ----------------
def run_case(case, log, win_frame, hits, bounds, pid):
    inject_before = len(read_log(log).splitlines())
    if pid:
        activate_app(pid); raise_window(pid); time.sleep(0.5)

    def hit_by_id(hid):
        return next((h for h in hits if h["id"] == hid or h["name"] == hid), None)

    def click_hid(hid, times=1):
        h = hit_by_id(hid)
        if not h:
            return None
        pt = to_screen(win_frame, h, bounds)
        for _ in range(times):
            click_at(pt)
        return pt

    if case == "T1":  # 点击 Accordion 展开
        click_hid("1")
        time.sleep(0.5)
    elif case == "T2":  # Tab 焦点(2 次, 验证 1→2 移动)
        tab_key(False); time.sleep(0.3)
        tab_key(False); time.sleep(0.3)
    elif case == "T3":  # Shift+Tab(2 次, 验证 2→1)
        tab_key(True); time.sleep(0.3)
        tab_key(True); time.sleep(0.3)
    elif case == "T4":  # WaBadge 计数 x2
        click_hid("2", 2)
        time.sleep(0.5)
    elif case == "T5":  # 未命中区（窗口内空白，负向）→ 预期 [hit] id=none
        # 点 Accordion 中心下方窗口内空白：逻辑 x=170(Accordion 中心x), y=110(在44~220空白带, 组件rect之外)
        titlebar = (bounds[3] - win_frame["size"][1] / win_frame.get("scale", 1.0)) if bounds else 0.0
        if bounds:
            pt = (bounds[0] + 170.0, bounds[1] + titlebar + 110.0)
            click_at(pt)
        time.sleep(0.5)
    elif case == "T6":  # toggle 往返（点两次）
        click_hid("1", 2)
        time.sleep(0.5)
    elif case == "T7":  # 模态焦点(d20_modal): 点窗口内任意位置开模态 → Tab → 恢复
        # d20_modal 左键任意位置 open_modal(非按 hit 判断); 点 base_a 中心(窗口内)触发
        click_hid("100", 1)   # 点 base_a(窗口内) → modal_open + 焦点隔离到 modal 200
        time.sleep(0.5)
        tab_key(False)         # 模态内 Tab 循环(焦点不逃到 base)
        time.sleep(0.3)
        tab_key(False)
        time.sleep(0.3)
    else:
        print(f"[warn] 未知场景 {case}", file=sys.stderr)

    after = read_log(log)
    fresh = "\n".join(after.splitlines()[inject_before:])
    return fresh


# ---------------- BUG 报告 ----------------
def bug_report(case, fail_layer, evidence, expected, inject_meta, screenshot=None, suggest=None):
    report = {
        "case": case,
        "fail_layer": fail_layer,
        "fail_layer_evidence": evidence,
        "expected": expected,
        "inject": inject_meta,
        "screenshot": screenshot,
        "suggest": suggest or "",
        "ts": time.strftime("%Y-%m-%d %H:%M:%S"),
    }
    return json.dumps(report, ensure_ascii=False, indent=2)


# ---------------- 主流程 ----------------
def main():
    ap = argparse.ArgumentParser(description="rgui 鼠标键盘自动化测试（D21-2 分层诊断）")
    ap.add_argument("--log", required=True, help="demo 输出日志文件")
    ap.add_argument("--case", help="单场景 T1..T7")
    ap.add_argument("--all", action="store_true", help="全量 T1-T6")
    ap.add_argument("--screenshot-dir", default="tools/qa/d7_screenshots", help="截图存证目录")
    ap.add_argument("--demo", default="window_demo", help="目标窗口 owner 名")
    a = ap.parse_args()

    # ---- 预检 fail-fast ----
    pre_err = []
    if not _CG:
        pre_err.append("CoreGraphics(CGEvent) 加载失败 —— 脚本环境问题")
    if not ax_trusted():
        pre_err.append("AXIsProcessTrusted()=False —— 未授权注入（脚本环境问题，需授权）")
    if pre_err:
        print("[PRE-CHECK 失败] 脚本环境问题：")
        for e in pre_err:
            print("  -", e)
        sys.exit(2)

    text = read_log(a.log)
    hits = [parse_hit_region(l) for l in text.splitlines()]
    hits = [h for h in hits if h]
    frames = [parse_win_frame(l) for l in text.splitlines()]
    frames = [fr for fr in frames if fr]
    if not hits:
        print("[PRE-CHECK] 未解析到 [hit-region] —— 检查 --log 是否来自已打日志的 demo", file=sys.stderr)
        sys.exit(2)
    wf = frames[-1] if frames else {"origin": (0, 0), "size": (0, 0), "scale": 1.0}
    bounds, pid = find_window_bounds(a.demo)
    if not bounds:
        print(f"[PRE-CHECK] 未在 CGWindowList 找到 '{a.demo}' 窗口 —— 确认 demo 已启动", file=sys.stderr)
        sys.exit(2)

    cases = (["T1", "T2", "T3", "T4", "T5", "T6"] if a.all else [a.case]) if not a.all else \
            (["T1", "T2", "T3", "T4", "T5", "T6"] if a.all else [a.case])
    if a.all:
        cases = ["T1", "T2", "T3", "T4", "T5", "T6"]

    any_fail = False
    for case in cases:
        if not case:
            continue
        fresh = run_case(case, a.log, wf, hits, bounds, pid)
        layer, ev, status = detect_layer(fresh, case)
        print(f"\n===== {case} =====")
        for l, s in status.items():
            print(f"  {l}: {'PASS' if s['ok'] else 'FAIL'}  {s['ev']}")
        # 截图存证
        wid = find_window_id(a.demo)
        shot = os.path.join(a.screenshot_dir, f"d21_{case.lower()}.png")
        if wid:
            subprocess.run(["screencapture", "-x", "-l", str(wid), shot], check=False,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        if layer != "PASS":
            any_fail = True
            exp = {"T1": "[hit] id=1+[action] toggle", "T2": "[focus] Some(1)",
                   "T3": "[focus] Some(2)->Some(1)", "T4": "badge_click count 1->2",
                   "T5": "[hit] id=none", "T6": "toggle 往返",
                   "T7": "[action] modal_open + [focus] click->Some(200)"}.get(case, "")
            suggest = SUGGEST_BY_LAYER.get(layer, SUGGEST_BY_LAYER["L2"])
            print("[BUG REPORT]\n" + bug_report(case, layer, ev, exp,
                   inject_meta={"bounds": bounds, "scale": wf.get("scale")},
                   screenshot=shot, suggest=suggest))
        else:
            print(f"  => {case} PASS（全层通过）")

    print(f"\n[D21-2] 结果: {'有失败(exit 1)' if any_fail else '全部通过(exit 0)'}")
    sys.exit(1 if any_fail else 0)


if __name__ == "__main__":
    main()
