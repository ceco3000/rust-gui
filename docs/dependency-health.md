# 依赖健康度监控

> 按 [验证设计 §5.5](../docs/Rust%20GUI%20框架技术路线验证设计.md#55-依赖健康度持续监控) 的要求，每季度更新一次。

## 初始评估（2026-06-12）

| 依赖 | 当前版本 | 最近 release | 核心维护者 | 已知使用者 | 资金状态 | 风险 |
|------|---------|-------------|-----------|-----------|---------|------|
| **wgpu** | v24.0.5 | 活跃 | >5人 | Firefox, Servo, Deno, iced | Mozilla + 社区 | 🟢 低 |
| **winit** | v0.30.13 | 活跃 | >3人 | Rust GUI 生态标准 | 社区 | 🟢 低 |
| **Vello** | v0.8.0 | 活跃（v0.9.0 已发布） | Linebender (~3人) | Xilem, Bevy (bevy_vello) | 不明 | 🟡 中 |
| **cosmic-text** | v0.17.2 | 活跃（v0.19.0 已发布） | System76 | iced, COSMIC DE | System76 营收 | 🟢 低 |
| **Taffy** | v0.8.x | 活跃 | DioxusLabs | Dioxus, Bevy, Zed/GPUI, Servo, Slint | VC 支持 | 🟢 低 |
| **AccessKit** | v0.24.0 | 活跃 | STF + 社区 | egui, Slint, GTK 4.18, Bevy | STF 资助 | 🟡 中 |

## 更新记录

### 2026-06-12（初始记录）

- 所有依赖均处于活跃维护状态
- Vello v0.9.0 已发布（POC 验证使用 v0.8.0）
- cosmic-text v0.19.0 已发布（POC 验证使用 v0.17.2）
- 未发现任何依赖超过 12 个月无更新
