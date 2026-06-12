# V9 确认：DataGrid 虚拟滚动性能前提

**日期**：2026-06-12
**方法**：Vello 0.8 API 文档确认

## 确认结果：三项能力均具备 ✅

### 1. Scene 增量更新
- `Scene::reset()` — 清空场景，不释放内存
- `Scene::append()` — 追加另一个场景
- 虚拟滚动模式：每帧 reset + 仅重建可见行

### 2. ClipRect 视口裁剪
- `scene.push_clip_layer()` — 推入裁剪层
- `scene.pop_layer()` — 退出裁剪层
- 用于将渲染范围限制在 DataGrid 可见视口内

### 3. GPU Buffer 动态更新
- `render_to_texture()` 每帧接收新 Scene
- 内部通过 Encoding 系统管理 GPU 资源

## 结论

✅ 无影响。三项能力均存在。实际性能验证在阶段 2 进行。
