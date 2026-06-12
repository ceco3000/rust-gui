# 开发计划

> 本计划基于 [Rust GUI 框架技术路线验证设计](./Rust%20GUI%20框架技术路线验证设计.md)，按执行顺序编排，仅记录开发项与完成状态。
>
> 状态图例：⬜ 待开始　🟦 进行中　✅ 已完成　⏸️ 暂缓　❌ 已取消

---

## 阶段 0：技术路线验证

### 主验证项

- [x] V1：[Vello + cosmic-text 协同渲染](./Rust%20GUI%20框架技术路线验证设计.md#v1-vello--cosmic-text-协同渲染) ✅（含 SwashCache 字形纹理渲染）
- [x] V2：[cosmic-text CJK 文本渲染质量](./Rust%20GUI%20框架技术路线验证设计.md#v2-cosmic-text-cjk-文本渲染质量) ✅（10 类全通过：简繁日韩、Emoji、Bidi、生僻字；[报告](../verify/v2-cjk-text/report.md)）
- [x] V3：[AccessKit 能力边界分析](./Rust%20GUI%20框架技术路线验证设计.md#v3-accesskit-能力边界分析) ✅（[报告](../verify/v3-accesskit-gap/report.md)）
- [x] V4：[渲染管线跨平台三端可运行](./Rust%20GUI%20框架技术路线验证设计.md#v4-渲染管线跨平台三端可运行) 🟦（macOS 编译已验证；[CI 配置](../verify/v4-cross-platform/.github/workflows/cross-platform.yml) 就绪；Linux/Windows 待 runner 验证）
- [x] V5：[WidgetView diff 性能基准](./Rust%20GUI%20框架技术路线验证设计.md#v5-widgetview-diff-性能基准) ✅（780 节点 24µs，9330 节点 246µs，大幅优于 1ms 目标；[报告](../verify/v5-diff-bench/report.md)）
- [x] V6：[Taffy 布局 → 渲染坐标转换](./Rust%20GUI%20框架技术路线验证设计.md#v6-taffy-布局--渲染坐标转换) ✅（4 用例全通过；FlexRow/FlexColumn/Grid/嵌套；[报告](../verify/v6-taffy-layout/report.md)）
- [x] V7：[状态快照性能基准](./Rust%20GUI%20框架技术路线验证设计.md#v7-状态快照性能基准) ✅（JSON 和 postcard 双格式通过，postcard 比 JSON 快 1.1-2.4x；[报告](../verify/v7-snapshot-bench/report.md)）
- [x] V9：[DataGrid 虚拟滚动性能前提确认](./Rust%20GUI%20框架技术路线验证设计.md#v9-datagrid-虚拟滚动性能前提确认) ✅（[确认](../verify/v9-datagrid/confirmation.md)）
- [x] V10：[cosmic-text IME 集成路径](./Rust%20GUI%20框架技术路线验证设计.md#v10-cosmic-text-ime-集成路径) ✅（winit IME + cosmic-text Buffer 链路就绪；macOS 手动验证待执行；[报告](../verify/v10-ime/report.md)）

### 基线测量

- [x] 基线测量：[Rust 快速重启端到端延迟](./Rust%20GUI%20框架技术路线验证设计.md#551-基线测量rust-快速重启端到端延迟) ✅（非判定项，总延迟估算 1.78s，远低于 5s 目标；[报告](../verify/baseline-restart/report.md)）

### 替代验证项（按需触发）

- [x] AV2：[Skia 替代 Vello](./Rust%20GUI%20框架技术路线验证设计.md#av2-skia-替代-vello) ✅（必须执行项；skia-safe 0.75 编译运行通过；[报告](../verify/av2-skia/report.md)）
- [ ] AV1：[Parley+Fontique 替代 cosmic-text](./Rust%20GUI%20框架技术路线验证设计.md#av1-parley--fontique-替代-cosmic-text) ⬜（V1 失败时触发）
- [ ] AV3：[直接平台 API 替代 AccessKit](./Rust%20GUI%20框架技术路线验证设计.md#av3-直接平台-api-替代-accesskit) ⬜（V3 致命缺口 / 维护中断 / 平台覆盖不足时触发）
- [ ] AV4：[Yoga 替代 Taffy](./Rust%20GUI%20框架技术路线验证设计.md#av4-yoga-替代-taffy) ⬜（Taffy 维护风险触发时触发）

---

## 里程碑判定

| 里程碑 | 条件 | 判定 |
|------|------|------|
| M1 | V1 + V3 + V4 完成 | 🔴 [决定主路线是否可以继续](./Rust%20GUI%20框架技术路线验证设计.md#53-里程碑含替代路径) |
| M2 | V2 + V5 + V7 + AV2 完成 | 🟡 [补充性能数据 + 渲染 B 方案就绪](./Rust%20GUI%20框架技术路线验证设计.md#53-里程碑含替代路径) |
| M2a | 若 V1 失败，AV1 完成 | 🔴 [确认文本替代方案可行](./Rust%20GUI%20框架技术路线验证设计.md#53-里程碑含替代路径) |
| M3 | V6 完成 | 🟡 [验证布局集成](./Rust%20GUI%20框架技术路线验证设计.md#53-里程碑含替代路径) |
| M4 | V10 完成 | 🟢 [IME 集成验证完成](./Rust%20GUI%20框架技术路线验证设计.md#53-里程碑含替代路径) |

---

## 验证结论决定树

详见 [§5.6 验证结论决定树](./Rust%20GUI%20框架技术路线验证设计.md#56-验证结论决定树含替代路径)

- **全部通过** → 进入阶段 0 开发
- **部分未通过但有缓解方案** → 更新路线书后进入阶段 0
- **主路线失败，替代通过** → 更新技术选型后进入阶段 0
- **主路线和替代均失败** → 触发技术评审

---

## 持续任务

- [ ] [依赖健康度监控](./Rust%20GUI%20框架技术路线验证设计.md#55-依赖健康度持续监控) ⬜
  > 监控告警触发对应替代验证项（AV1-AV4），详见 [§5.4 触发矩阵](./Rust%20GUI%20框架技术路线验证设计.md#54-替代技术验证触发矩阵)
  > 初始评估已记录于 [依赖健康度初始评估](./dependency-health.md)（2026-06-12）
