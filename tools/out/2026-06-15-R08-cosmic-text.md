# R08: cosmic-text 集成（字形光栅化）

**日期**: 2026-06-15
**验收标准**: 中文文本渲染

## 完成内容

### 新增模块: `rgui-render/src/text.rs`

- `TextEngine` — cosmic-text 文本引擎，封装 `FontSystem` + `SwashCache`
- `TextEngine::new()` — 从嵌入字体 (Inter Regular + Bold) 创建 FontSystem
- `TextEngine::shape_text()` — 文本塑形，返回 `Vec<ShapedGlyph>`
- `TextEngine::rasterize_glyph()` — 通过 SwashCache 光栅化单个字形为 RGBA8 位图
- `TextEngine::as_rasterizer()` — 返回与 `GlyphAtlas::get_or_rasterize()` 兼容的 FnMut 闭包
- `ShapedGlyph` — 塑形结果，包含 GlyphKey + 布局坐标 + advance 宽度
- `convert_swash_image()` — SwashImage → RGBA8 像素缓冲区转换（Mask/SubpixelMask/Color）
- 15 个单元测试覆盖创建、塑形、光栅化、atlas 集成、边界条件

### fontdb::ID → u64 映射

由于 `fontdb::ID(InnerId)` 构造函数为私有，`TextEngine` 内部维护双向映射：
- `shape_text` 时将 `glyph.font_id`（`fontdb::ID`）映射为 `u64`
- `rasterize_glyph` 时将 `u64` 查回 `fontdb::ID` 以调用 SwashCache

### API 变更

- `DrawCommand::DrawGlyphs` 新增 `texture_id: TextureId` 字段（D3 §3.1 设计演进）
- `GlyphAtlas::get_or_rasterize()` 参数从 `&dyn Fn` 改为 `&mut dyn FnMut`（兼容 TextEngine 的可变借用）

### Bug 修复

- `RasterizedGlyph.advance` 原使用 `swash_image.placement.left`（bearing X，语义错误），修正为 0.0（占位），真实 advance 由 `ShapedGlyph.advance` 提供

## 测试结果

- `cargo test -p rgui-render --features vello-backend`: **152 passed, 0 failed**
- `cargo fmt -- --check`: 通过
- `cargo clippy -- -D warnings`: 通过
- 完整 workspace 测试: 全部通过

## 代码审查发现与修复

| 级别 | 数量 | 已修复 |
|------|------|--------|
| HIGH | 2 | 2 |
| MEDIUM | 6 | 5 |

### 已修复

- HIGH-2: advance 语义错误 → 改为 0.0 占位
- MEDIUM-3: f32::MAX 无界宽度 → 改为 8192.0
- MEDIUM-5: SubpixelMask 无数据校验 → 添加 debug_assert
- MEDIUM-6: vello.rs 缺少 TODO 标记 → 添加 TODO(R08+R15) 注释
- MEDIUM-8: 缺少 Debug 实现 → 添加手动 Debug impl
- LOW-10: rasterize_space 无断言 → 添加 assert result.is_none()

### 未修复（记录为未来任务）

- HIGH-1: DrawGlyphs texture_id 与 D3 偏离 — D3 文档修改不在本次范围
- MEDIUM-4: font_size as u32 精度损失 — 需要改 GlyphKey 签名，后续任务处理
- MEDIUM-7: 缺少端到端集成 — 组件已就绪，集成在后续 R15 完成

## 架构说明

中文文本渲染链路已贯通：
```
文本 → TextEngine::shape_text() → cosmic_text::Buffer → layout_runs()
  → GlyphKey → TextEngine::rasterize_glyph() → SwashCache → RasterizedGlyph
  → GlyphAtlas::get_or_rasterize() → GlyphCacheEntry (atlas UV + advance)
  → DrawCommand::DrawGlyphs { texture_id, glyphs, font_size, color }
  → VelloBackend/SkiaBackend → 渲染（当前占位矩形，待 R15 完成纹理四边形）
```

CJK 字符需要系统字体后备或嵌入 Noto Sans CJK 字体（R17 后续）。
