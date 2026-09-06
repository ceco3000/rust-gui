import sys
sys.path.insert(0, "tools/qa")
import rgui_input_test as R
layer, ev, st = R.detect_layer('[focus] Tab(shift=false) -> Some(1)\n[focus] Tab(shift=false) -> Some(2)', "T2")
print("T2 focus 正常 →", layer, "|", ev)
layer2, ev2, _ = R.detect_layer('[focus] Tab(shift=false) -> None(0)\n[focus] Tab(shift=false) -> None(0)', "T2")
print("T2 focus 失效(None) →", layer2, "|", ev2)
