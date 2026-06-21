//! 框架内置消息类型。
//!
//! 本模块定义框架内部使用的消息类型，与用户自定义的 `AppMessage` 实现互补。

use crate::traits::AppMessage;

/// 空操作消息类型——用于消息类型擦除。
///
/// `NoopMsg` 是一个零变体枚举，用于需要 `AppMessage` 类型参数但实际不处理消息的场景
/// （如命中测试仅需要 widget 树结构，不关心消息内容）。
///
/// # 用途
///
/// - 命中测试：`WidgetView<NoopMsg>` 保留完整树结构（widget_type/id/children），
///   但剥离消息绑定，供 `handle_click` 做 DFS 树遍历命中测试。
///
/// # 注意
///
/// 此类型不可构造——任何 `match` 都是静态不可达的。
/// 因此 `message_name()` 永远不被调用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoopMsg {}

impl AppMessage for NoopMsg {
    fn message_name(&self) -> &'static str {
        match *self {}
    }
}
