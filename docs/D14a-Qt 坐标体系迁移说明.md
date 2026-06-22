# D14a：Qt 坐标体系迁移说明

> **文档定位：** 本文档给出旧坐标字段、旧自动化注入接口和旧命中测试辅助函数到 Qt 一致性坐标体系的迁移路径。
>
> **正式规范来源：** [D14-Qt 坐标一致性工程规范](./D14-%E9%BC%A0%E6%A0%87%E5%9D%90%E6%A0%87%E9%97%AE%E9%A2%98%E5%A4%84%E7%90%86%E6%B5%81%E7%A8%8B%E5%B7%A5%E7%A8%8B%E8%A7%84%E8%8C%83.md)。
>
> **适用范围：** 旧的单一 `position` 事件消费代码、仍以 `physical` 命名的平台原始注入辅助函数，以及直接依赖窗口绝对点的命中测试断言。

---

## 1. 迁移目标

Qt 一致性坐标体系要求把“窗口逻辑坐标”“接收者局部坐标”“平台原始窗口坐标证据”分开表达：

- `window_logical`：高层事件、命中测试和日志的统一窗口逻辑坐标。
- `local_logical`：目标接收者局部坐标，用于组件内部交互逻辑。
- `raw_window_position`：平台原始窗口输入，只用于归一化、调试、回放和回归断言。

迁移完成后，业务代码不再依赖模糊的单一 `position`，也不再把 `physical` 误写成高层稳定语义。

---

## 2. 旧字段到新字段映射

| 旧写法 / 旧习惯 | 新写法 | 何时使用 |
|------|------|------|
| `event.position`（窗口坐标语义） | `event.coords.window_logical` | 需要窗口级命中、hover、拖拽、日志时 |
| `event.position`（接收者局部语义） | `event.coords.local_logical` | 需要组件内部局部点击点时 |
| `event.delta` | `event.delta_window_logical` | 鼠标移动增量，且语义显式为窗口逻辑坐标 |
| “平台事件原始点直接参与命中测试” | 先 `normalize_platform_window_point()`，再使用 `window_logical` | 所有平台窗口原始输入 |
| “把原始平台点当成全局坐标” | 禁止；若未来需要全局坐标，新增独立字段 | 当前项目尚未提供稳定全局坐标 API |

### 2.1 事件消费示例

旧写法：

```rust
match event {
    Event::MouseDown { position, .. } => handle_click(*position),
    Event::MouseMove { position, delta, .. } => handle_hover(*position, *delta),
    _ => {}
}
```

新写法：

```rust
match event {
    Event::MouseDown { coords, .. } => handle_click(coords.window_logical),
    Event::MouseMove {
        coords,
        delta_window_logical,
        ..
    } => handle_hover(coords.window_logical, *delta_window_logical),
    _ => {}
}
```

若组件需要局部点，应显式读取：

```rust
if let Some(local) = coords.local_logical {
    handle_local_click(local);
}
```

---

## 3. 自动化注入接口迁移

| 旧接口 | 新接口 / 规范建议 | 说明 |
|------|------|------|
| `inject_hover_physical(position)` | `inject_hover_platform_window_raw(position)` | 旧接口仍可兼容，但新代码应使用更明确的命名 |
| `inject_click_physical(position)` | `inject_click_platform_window_raw(position)` | 强调输入是“平台原始窗口坐标”，不是高层物理语义 |
| `inject_hover_logical(position)` | 保持不变 | 直接验证逻辑命中链路 |
| `inject_click_logical(position)` | 保持不变 | 直接验证逻辑点击链路 |
| “直接构造平台事件再手写归一化” | `replay_cursor_moved_platform_window_raw()` / `replay_left_click_platform_window_raw()` | 回放路径统一走正式平台入口 |

### 3.1 平台原始窗口注入规则

- macOS：当前平台原始窗口输入与视觉逻辑点数值一致，不再乘 `scale_factor`。
- 非 macOS：使用 `logical_point * scale_factor` 构造平台原始窗口输入。
- 所有平台：进入高层事件后统一断言 `coords.window_logical` 与逻辑目标点一致。

---

## 4. 命中测试辅助函数迁移

| 旧写法 / 旧辅助方式 | 新写法 | 原因 |
|------|------|------|
| 直接断言 `find_widget_at_point(raw_position)` | `hit_test_logical(window_logical)` | 命中测试正式输入是逻辑窗口坐标 |
| 只断言命中 widget id | 同时断言 `coords.window_logical`、`coords.local_logical`、`origin.raw_window_position` | 需要验证多参考系一致性 |
| 在测试里手工 `/ scale_factor` 或 `* scale_factor` | 使用 `normalize_platform_window_point()` 或 Harness 封装 | 避免重复归一化逻辑散落 |
| 组件内部手工计算 `local = position - rect.origin` | 优先读取 `coords.local_logical` | 局部点应来自统一命中恢复链路 |

推荐断言顺序：

1. 先用 `hit_test_logical()` 确认窗口逻辑点命中目标。
2. 再用 `inject_hover_platform_window_raw()` / `inject_click_platform_window_raw()` 验证平台原始窗口输入路径。
3. 最后断言 `coords.local_logical` 与 `origin.raw_window_position`，确保窗口坐标、局部坐标和原始输入证据一致。

---

## 5. 迁移步骤清单

1. 搜索所有 `Event::MouseDown { position`、`Event::MouseMove { position`、`delta:` 等旧写法。
2. 区分消费方到底需要 `window_logical` 还是 `local_logical`。
3. 把自动化中的 `*_physical` 新增调用迁移到 `*_platform_window_raw`。
4. 删除测试内手写 DPI 换算，统一改走 Harness 或 `normalize_platform_window_point()`。
5. 为迁移后的路径补至少一组 `scale_factor = 1` 和 HiDPI 对照验证。
6. 若文档、注释或命名仍出现“单一 position 即全部语义”，同步修订到 D14 术语。

---

## 6. 常见错误

- 把 `raw_window_position` 当作组件层可直接消费的坐标。
- 在 `MouseDown`、`MouseUp` 路径重新做一次 DPI 换算。
- 使用 `local_logical` 与窗口绝对 `Rect` 直接比较。
- 继续以 `physical` 命名新测试辅助函数，导致读者误解其实际语义。
- 只验证命中结果，不验证 `window_logical`、`local_logical` 与原始输入证据三者关系。
