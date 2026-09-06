import sys
sys.path.insert(0, "tools/qa")
import rgui_input_test as R

# 模拟各 fail_layer → suggest 字典填充
cases = [
    ("L2", "[mouse-event] left-press at logical=(170,22) in-region=false", "T1"),
    ("L3", "[mouse-event] left-press at logical=(170,22) in-region=true\n[hit] id=none", "T1"),
    ("L4", "[mouse-event] left-press at logical=(170,22) in-region=true\n[hit] id=1\n", "T1"),
]
for exp_layer, snap, case in cases:
    layer, ev, st = R.detect_layer(snap, case)
    suggest = R.SUGGEST_BY_LAYER.get(layer, R.SUGGEST_BY_LAYER["L2"])
    print(f"fail_layer={layer} 期望={exp_layer} | ev={ev}")
    print(f"  suggest: {suggest}\n")
print("PASS suggest:", R.SUGGEST_BY_LAYER["PASS"])
