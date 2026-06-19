// handlers.rhai — 按钮点击事件处理器
//
// .rgui 中的 onclick="handle_click()" 会路由到此函数。
// Rhai print 内置函数输出到标准输出。

fn handle_click() {
    print("🎯 rgui button clicked via Rhai!");
}
