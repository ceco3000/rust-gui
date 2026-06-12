# V5: WidgetView diff 性能基准

📅 2026-06-12 | 🖥️ macOS 15 (Apple Silicon) | Rust 1.92.0

## 结果

| 基准 | 节点数 | 耗时 | 目标 |
|------|--------|------|------|
| diff_85nodes | ~85 | **3.2 µs** | < 1ms |
| diff_780nodes | ~780 | **24.3 µs** | < 1ms |
| diff_9330nodes | ~9,330 | **246 µs** | < 1ms |
| diff_no_change | ~780 | **18.5 µs** | < 1ms |
| diff_full_replace | 40 | **975 ns** | < 1ms |

## 结论

✅ 全部通过。1000 节点树 diff 约 31 µs（780 节点线性外推），仅为 1ms 目标的 3%。
9000+ 节点压力测试 246 µs，仍不足目标的 25%。
