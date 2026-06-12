# V10: cosmic-text IME 集成路径

📅 2026-06-12 | winit 0.30 | cosmic-text 0.17

## 验证项

| # | 验证点 | 结果 |
|---|--------|------|
| 1 | winit IME 事件 API 编译 | ✅ |
| 2 | cosmic-text Buffer 编辑集成 | ✅ |
| 3 | Preedit/Commit 事件处理逻辑 | ✅ |
| 4 | macOS CJK 输入法完整流程 | ⚠️ 需手动验证 |

## 手动验证

```
cargo run -p verify-v10-ime
→ 切换系统输入法到拼音，键入拼音，观察 Preedit/Commit 事件
```

## 结论

✅ winit IME + cosmic-text Buffer 链路代码就绪。
