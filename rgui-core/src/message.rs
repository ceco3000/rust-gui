//! 内置消息类型：`NoopMsg`。

use crate::traits::AppMessage;

/// 空消息（表示无操作/占位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NoopMsg;

impl AppMessage for NoopMsg {
    fn message_name(&self) -> &'static str {
        "noop"
    }
}
