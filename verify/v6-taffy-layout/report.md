# V6: Taffy 布局 → 渲染坐标转换

📅 2026-06-12 | taffy 0.7

## 结果

| 用例 | 验证 | 状态 |
|------|------|------|
| FlexRow | 3 按钮 80px 水平，gap=12 padding=16 | ✅ |
| FlexColumn | 文本+输入框+按钮 垂直，gap=8 padding=16 | ✅ |
| Grid | 2 列 200px+1fr，gap=16 | ✅ |
| 嵌套 | FlexColumn > FlexRow [Icon, Title, Spacer, Button] | ✅ |

## 结论

✅ Taffy 布局结果可正确映射为渲染坐标。
