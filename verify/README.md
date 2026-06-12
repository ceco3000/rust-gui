# 技术路线验证

> 本目录包含《Rust GUI 框架技术路线验证设计》中各项验证的 POC 代码和调研文档。
>
> **项目进度统一管理在 [开发计划](../docs/Rust%20GUI%20框架技术路线验证设计开发计划.md)。**

## 目录结构

```
verify/
├── README.md                    # 本文件——验证总入口
├── v1-vello-cosmic/             # V1: Vello + cosmic-text 协同渲染 POC
├── v2-cjk-quality/              # V2: CJK 文本渲染质量（扩展 V1）
├── v3-accesskit-gap/            # V3: AccessKit 能力边界调研
├── v4-cross-platform/           # V4: 跨平台 CI
├── v5-diff-bench/               # V5: WidgetView diff 基准
├── v6-taffy-layout/             # V6: Taffy 布局 → 渲染坐标
├── v7-snapshot-bench/           # V7: 状态快照基准
├── v10-ime/                     # V10: IME 集成
├── av1-parley-fontique/         # AV1: Parley+Fontique 替代
├── av2-skia/                    # AV2: Skia 替代
├── av3-platform-a11y/           # AV3: 直接平台 API 替代
└── av4-yoga/                    # AV4: Yoga 替代
```

## 验证报告

各验证项完成后，产出报告按 [验证设计 §6.3 模板](../docs/Rust%20GUI%20框架技术路线验证设计.md#63-验证报告模板) 编写。
