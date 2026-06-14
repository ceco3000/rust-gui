//! IPC 协议——DisplayProcess 与 AppProcess 之间的消息协议。
//!
//! 阶段 1（MVP）不启用双进程，IPC 类型仅为阶段 2 预留。
//! 消息使用 postcard 二进制序列化，传输层使用 Unix Domain Socket / Named Pipe。
//!
//! 定义源自 D7 §7。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// IPC 通信错误。
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    /// 连接失败。
    #[error("IPC 连接失败：{0}")]
    ConnectionFailed(String),

    /// 发送失败。
    #[error("IPC 发送失败：{0}")]
    SendFailed(String),

    /// 接收失败。
    #[error("IPC 接收失败：{0}")]
    ReceiveFailed(String),

    /// 序列化失败。
    #[error("IPC 序列化失败：{0}")]
    SerializationError(String),

    /// 连接断开。
    #[error("IPC 连接断开")]
    #[allow(dead_code)]
    Disconnected,

    /// 超时。
    #[error("IPC 超时")]
    #[allow(dead_code)]
    Timeout,
}

/// IPC 消息。
///
/// DisplayProcess 与 AppProcess 之间的通信协议。
/// 所有变体均可通过 postcard 进行二进制序列化。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcMessage {
    /// DisplayProcess → AppProcess：状态快照 + 元数据。
    RestoreState {
        /// 序列化的状态快照。
        snapshot: Vec<u8>,
        /// 恢复元数据（焦点、滚动位置等）。
        metadata: RestoreMetadata,
    },

    /// AppProcess → DisplayProcess：就绪信号。
    Ready {
        /// 当前 widget 数量。
        widget_count: u32,
    },

    /// AppProcess → DisplayProcess：场景图更新。
    SceneUpdate {
        /// 场景图数据（postcard 序列化）。
        scene_data: Vec<u8>,
        /// 是否为增量更新。
        incremental: bool,
    },

    /// DisplayProcess → AppProcess：输入事件。
    InputEvent {
        /// 事件数据（postcard 序列化）。
        event_data: Vec<u8>,
    },

    /// DisplayProcess → AppProcess：终止信号。
    Shutdown,

    /// AppProcess → DisplayProcess：可恢复错误。
    Error {
        /// 错误描述。
        message: String,
        /// 是否可恢复。
        recoverable: bool,
    },
}

/// 状态恢复元数据。
///
/// 记录快照前的交互上下文，用于快速重启后恢复用户交互状态。
/// 定义源自 D7 §8。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreMetadata {
    /// 焦点路径（widget ID 链）。
    pub focus_path: Vec<u64>,
    /// 各滚动容器的滚动位置。
    pub scroll_positions: HashMap<u64, (f64, f64)>,
    /// 当前路由。
    pub current_route: String,
    /// 窗口几何信息：(x, y, width, height)。
    pub window_geometry: (i32, i32, u32, u32),
}

impl Default for RestoreMetadata {
    fn default() -> Self {
        Self {
            focus_path: Vec::new(),
            scroll_positions: HashMap::new(),
            current_route: String::new(),
            window_geometry: (0, 0, 800, 600),
        }
    }
}

/// IPC 通道（阶段 2 预留）。
///
/// 阶段 1 不使用此类——仅定义接口，实际通信在阶段 2 实现。
#[allow(dead_code)]
pub struct IpcChannel {
    /// 读缓冲区。
    read_buffer: Vec<u8>,
}

impl IpcChannel {
    /// 创建新的 IPC 通道。
    #[must_use]
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            read_buffer: Vec::new(),
        }
    }
}

impl Default for IpcChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_message_roundtrip_shutdown() {
        let msg = IpcMessage::Shutdown;
        let data = postcard::to_vec(&msg).expect("序列化失败");
        let recovered: IpcMessage = postcard::from_bytes(&data).expect("反序列化失败");
        assert!(matches!(recovered, IpcMessage::Shutdown));
    }

    #[test]
    fn test_ipc_message_roundtrip_ready() {
        let msg = IpcMessage::Ready { widget_count: 42 };
        let data = postcard::to_vec(&msg).expect("序列化失败");
        let recovered: IpcMessage = postcard::from_bytes(&data).expect("反序列化失败");
        match recovered {
            IpcMessage::Ready { widget_count } => assert_eq!(widget_count, 42),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_ipc_message_roundtrip_error() {
        let msg = IpcMessage::Error {
            message: "样式解析失败".into(),
            recoverable: true,
        };
        let data = postcard::to_vec(&msg).expect("序列化失败");
        let recovered: IpcMessage = postcard::from_bytes(&data).expect("反序列化失败");
        match recovered {
            IpcMessage::Error {
                message,
                recoverable,
            } => {
                assert_eq!(message, "样式解析失败");
                assert!(recoverable);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_ipc_message_roundtrip_restore_state() {
        let metadata = RestoreMetadata {
            focus_path: vec![1, 2, 3],
            scroll_positions: HashMap::from([(1u64, (100.0, 200.0))]),
            current_route: "/home".into(),
            window_geometry: (50, 50, 1024, 768),
        };
        let msg = IpcMessage::RestoreState {
            snapshot: vec![10, 20, 30],
            metadata,
        };
        let data = postcard::to_vec(&msg).expect("序列化失败");
        let recovered: IpcMessage = postcard::from_bytes(&data).expect("反序列化失败");
        match recovered {
            IpcMessage::RestoreState {
                snapshot,
                metadata,
            } => {
                assert_eq!(snapshot, vec![10, 20, 30]);
                assert_eq!(metadata.focus_path, vec![1, 2, 3]);
                assert_eq!(metadata.current_route, "/home");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_ipc_message_roundtrip_scene_update() {
        let msg = IpcMessage::SceneUpdate {
            scene_data: vec![0, 1, 2],
            incremental: true,
        };
        let data = postcard::to_vec(&msg).expect("序列化失败");
        let recovered: IpcMessage = postcard::from_bytes(&data).expect("反序列化失败");
        match recovered {
            IpcMessage::SceneUpdate {
                scene_data,
                incremental,
            } => {
                assert_eq!(scene_data, vec![0, 1, 2]);
                assert!(incremental);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_ipc_message_roundtrip_input_event() {
        let msg = IpcMessage::InputEvent {
            event_data: vec![5, 6, 7],
        };
        let data = postcard::to_vec(&msg).expect("序列化失败");
        let recovered: IpcMessage = postcard::from_bytes(&data).expect("反序列化失败");
        match recovered {
            IpcMessage::InputEvent { event_data } => {
                assert_eq!(event_data, vec![5, 6, 7]);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_restore_metadata_default() {
        let metadata = RestoreMetadata::default();
        assert!(metadata.focus_path.is_empty());
        assert!(metadata.scroll_positions.is_empty());
        assert_eq!(metadata.current_route, "");
        assert_eq!(metadata.window_geometry, (0, 0, 800, 600));
    }

    #[test]
    fn test_ipc_channel_default() {
        let channel = IpcChannel::default();
        assert!(channel.read_buffer.is_empty());
    }

    #[test]
    fn test_ipc_error_display() {
        let err = IpcError::ConnectionFailed("网络不可达".into());
        assert!(err.to_string().contains("网络不可达"));

        let err = IpcError::Disconnected;
        assert!(err.to_string().contains("断开"));
    }

    #[test]
    fn test_all_ipc_variants_serialize() {
        // 验证所有变体均可序列化
        let messages = vec![
            IpcMessage::Shutdown,
            IpcMessage::Ready { widget_count: 0 },
            IpcMessage::Error { message: String::new(), recoverable: false },
            IpcMessage::SceneUpdate { scene_data: vec![], incremental: false },
            IpcMessage::InputEvent { event_data: vec![] },
            IpcMessage::RestoreState { snapshot: vec![], metadata: RestoreMetadata::default() },
        ];
        for msg in &messages {
            let data = postcard::to_vec(msg).expect("序列化失败");
            assert!(!data.is_empty(), "序列化输出不应为空");
        }
    }
}
