// Accordion 容器 paint 脚本
// 绘制容器级背景 + 边框 + 圆角。子节点内容由 children 渲染。

// 外观颜色映射
let appearance = "outlined";  // 默认，后续由 prop 注入覆盖

// 背景色
let bg = rgba(1.0, 1.0, 1.0, 1.0);  // outlined 默认白底
let border_c = rgb(0.82, 0.82, 0.85);

// 圆角容器背景
fill_rect(0.0, 0.0, 400.0, 300.0, bg, 8.0);

// 四边 1px 边框
fill_rect(0.0, 0.0, 400.0, 1.0, border_c, 0.0);
fill_rect(0.0, 299.0, 400.0, 1.0, border_c, 0.0);
fill_rect(0.0, 0.0, 1.0, 300.0, border_c, 0.0);
fill_rect(399.0, 0.0, 1.0, 300.0, border_c, 0.0);

paint_children();
