//! IPC 协议——DisplayProcess 与 AppProcess 之间的消息协议。
//!
//! 阶段 1（MVP）不启用双进程，IPC 类型仅为阶段 2 预留。
//! 消息使用 postcard 二进制序列化，传输层使用 Unix Domain Socket / Named Pipe。
//!
//! 定义源自 D7 §7。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(unix)]
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::net::UnixStream;

/// IPC 消息最大允许大小（64 MiB），防止恶意或损坏的长度声明导致 OOM。
const MAX_MESSAGE_SIZE: usize = 64 * 1024 * 1024;

/// IPC 通信错误。
///
/// 未来可能新增错误变体，外部 match 时需处理通配分支。
#[derive(Debug, PartialEq, thiserror::Error)]
#[non_exhaustive]
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
///
/// 未来可能新增消息变体，外部 match 时需处理通配分支。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

// ---------------------------------------------------------------------------
// IpcChannel — Unix 平台（完整实现）
// ---------------------------------------------------------------------------

/// IPC 通道。
///
/// 提供 DisplayProcess 与 AppProcess 之间的消息传输能力。
/// 使用长度前缀帧协议：4 字节小端长度 + postcard 序列化负载。
///
/// 阶段 1（MVP）不启用双进程，本类型为阶段 2 预留。
#[cfg(unix)]
pub struct IpcChannel {
    stream: UnixStream,
    read_buffer: Vec<u8>,
}

#[cfg(unix)]
impl IpcChannel {
    /// 使用已连接的 [`UnixStream`] 创建 IPC 通道。
    ///
    /// 调用方负责建立连接（bind + connect 或 socketpair），
    /// `IpcChannel` 仅负责消息的序列化与传输。
    #[must_use]
    pub fn with_stream(stream: UnixStream) -> Self {
        Self {
            stream,
            read_buffer: Vec::with_capacity(4096),
        }
    }

    /// 发送 IPC 消息。
    ///
    /// 协议：4 字节小端长度前缀（`u32`）+ postcard 序列化负载。
    ///
    /// # 错误
    ///
    /// - [`IpcError::SerializationError`]：消息序列化失败。
    /// - [`IpcError::SendFailed`]：底层写入失败。
    /// - [`IpcError::Disconnected`]：对端已关闭连接。
    pub fn send(&mut self, msg: &IpcMessage) -> Result<(), IpcError> {
        let payload =
            postcard::to_allocvec(msg).map_err(|e| IpcError::SerializationError(e.to_string()))?;

        let len = payload.len() as u32;
        self.stream
            .write_all(&len.to_le_bytes())
            .map_err(map_write_error)?;
        self.stream
            .write_all(&payload)
            .map_err(map_write_error)?;

        Ok(())
    }

    /// 接收 IPC 消息。
    ///
    /// 先读取 4 字节长度前缀确定负载大小，再读取完整负载并反序列化。
    ///
    /// # 错误
    ///
    /// - [`IpcError::ReceiveFailed`]：底层读取失败或长度声明超出上限。
    /// - [`IpcError::SerializationError`]：负载反序列化失败。
    /// - [`IpcError::Disconnected`]：对端已关闭连接。
    pub fn recv(&mut self) -> Result<IpcMessage, IpcError> {
        let mut len_buf = [0u8; 4];
        self.stream
            .read_exact(&mut len_buf)
            .map_err(map_read_error)?;

        let len = u32::from_le_bytes(len_buf) as usize;
        if len > MAX_MESSAGE_SIZE {
            return Err(IpcError::ReceiveFailed(format!(
                "消息长度 {len} 超过最大允许值 {MAX_MESSAGE_SIZE}"
            )));
        }

        self.read_buffer.resize(len, 0);
        self.stream
            .read_exact(&mut self.read_buffer)
            .map_err(map_read_error)?;

        postcard::from_bytes(&self.read_buffer)
            .map_err(|e| IpcError::SerializationError(e.to_string()))
    }
}

/// 将写入 I/O 错误映射为 [`IpcError`]。
#[cfg(unix)]
fn map_write_error(e: std::io::Error) -> IpcError {
    match e.kind() {
        std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset => {
            IpcError::Disconnected
        },
        _ => IpcError::SendFailed(e.to_string()),
    }
}

/// 将读取 I/O 错误映射为 [`IpcError`]。
#[cfg(unix)]
fn map_read_error(e: std::io::Error) -> IpcError {
    match e.kind() {
        std::io::ErrorKind::UnexpectedEof
        | std::io::ErrorKind::BrokenPipe
        | std::io::ErrorKind::ConnectionReset => IpcError::Disconnected,
        _ => IpcError::ReceiveFailed(e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// IpcChannel — 非 Unix 平台（存根，阶段 2 预留）
// ---------------------------------------------------------------------------

/// IPC 通道（阶段 2 预留）。
///
/// 当前平台尚未实现 IPC 传输层。
#[cfg(not(unix))]
pub struct IpcChannel {
    read_buffer: Vec<u8>,
}

#[cfg(not(unix))]
impl IpcChannel {
    /// 创建存根 IPC 通道。
    #[must_use]
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            read_buffer: Vec::new(),
        }
    }

    /// 发送 IPC 消息（当前平台尚未实现）。
    #[allow(dead_code)]
    pub fn send(&mut self, _msg: &IpcMessage) -> Result<(), IpcError> {
        Err(IpcError::SendFailed(
            "当前平台 IPC 尚未实现（阶段 2 预留）".into(),
        ))
    }

    /// 接收 IPC 消息（当前平台尚未实现）。
    #[allow(dead_code)]
    pub fn recv(&mut self) -> Result<IpcMessage, IpcError> {
        Err(IpcError::ReceiveFailed(
            "当前平台 IPC 尚未实现（阶段 2 预留）".into(),
        ))
    }
}

#[cfg(not(unix))]
impl Default for IpcChannel {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // IpcMessage 序列化往返测试（不依赖 IPC 通道）
    // ========================================================================

    #[test]
    fn test_ipc_message_roundtrip_shutdown() {
        let msg = IpcMessage::Shutdown;
        let data = postcard::to_allocvec(&msg).expect("序列化失败");
        let recovered: IpcMessage = postcard::from_bytes(&data).expect("反序列化失败");
        assert_eq!(recovered, IpcMessage::Shutdown);
    }

    #[test]
    fn test_ipc_message_roundtrip_ready() {
        let msg = IpcMessage::Ready { widget_count: 42 };
        let data = postcard::to_allocvec(&msg).expect("序列化失败");
        let recovered: IpcMessage = postcard::from_bytes(&data).expect("反序列化失败");
        assert_eq!(recovered, IpcMessage::Ready { widget_count: 42 });
    }

    #[test]
    fn test_ipc_message_roundtrip_error() {
        let msg = IpcMessage::Error {
            message: "样式解析失败".into(),
            recoverable: true,
        };
        let data = postcard::to_allocvec(&msg).expect("序列化失败");
        let recovered: IpcMessage = postcard::from_bytes(&data).expect("反序列化失败");
        assert_eq!(
            recovered,
            IpcMessage::Error {
                message: "样式解析失败".into(),
                recoverable: true,
            }
        );
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
            metadata: metadata.clone(),
        };
        let data = postcard::to_allocvec(&msg).expect("序列化失败");
        let recovered: IpcMessage = postcard::from_bytes(&data).expect("反序列化失败");
        assert_eq!(
            recovered,
            IpcMessage::RestoreState {
                snapshot: vec![10, 20, 30],
                metadata,
            }
        );
    }

    #[test]
    fn test_ipc_message_roundtrip_scene_update() {
        let msg = IpcMessage::SceneUpdate {
            scene_data: vec![0, 1, 2],
            incremental: true,
        };
        let data = postcard::to_allocvec(&msg).expect("序列化失败");
        let recovered: IpcMessage = postcard::from_bytes(&data).expect("反序列化失败");
        assert_eq!(
            recovered,
            IpcMessage::SceneUpdate {
                scene_data: vec![0, 1, 2],
                incremental: true,
            }
        );
    }

    #[test]
    fn test_ipc_message_roundtrip_input_event() {
        let msg = IpcMessage::InputEvent {
            event_data: vec![5, 6, 7],
        };
        let data = postcard::to_allocvec(&msg).expect("序列化失败");
        let recovered: IpcMessage = postcard::from_bytes(&data).expect("反序列化失败");
        assert_eq!(
            recovered,
            IpcMessage::InputEvent {
                event_data: vec![5, 6, 7],
            }
        );
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
    fn test_restore_metadata_serialization_roundtrip() {
        let original = RestoreMetadata {
            focus_path: vec![10, 20],
            scroll_positions: HashMap::from([(5u64, (0.0, 0.0)), (8u64, (50.0, 100.0))]),
            current_route: "/settings/profile".into(),
            window_geometry: (100, 200, 1920, 1080),
        };
        let data = postcard::to_allocvec(&original).expect("序列化失败");
        let recovered: RestoreMetadata = postcard::from_bytes(&data).expect("反序列化失败");
        assert_eq!(recovered, original);
    }

    #[test]
    fn test_ipc_error_display() {
        let err = IpcError::ConnectionFailed("网络不可达".into());
        assert!(err.to_string().contains("网络不可达"));

        let err = IpcError::SendFailed("管道写入错误".into());
        assert!(err.to_string().contains("管道写入错误"));

        let err = IpcError::ReceiveFailed("管道读取错误".into());
        assert!(err.to_string().contains("管道读取错误"));

        let err = IpcError::SerializationError("postcard 序列化错误".into());
        assert!(err.to_string().contains("postcard 序列化错误"));

        let err = IpcError::Disconnected;
        assert!(err.to_string().contains("断开"));

        let err = IpcError::Timeout;
        assert!(err.to_string().contains("超时"));
    }

    #[test]
    fn test_all_ipc_variants_serialize() {
        let messages = vec![
            IpcMessage::Shutdown,
            IpcMessage::Ready { widget_count: 0 },
            IpcMessage::Error {
                message: String::new(),
                recoverable: false,
            },
            IpcMessage::SceneUpdate {
                scene_data: vec![],
                incremental: false,
            },
            IpcMessage::InputEvent { event_data: vec![] },
            IpcMessage::RestoreState {
                snapshot: vec![],
                metadata: RestoreMetadata::default(),
            },
        ];
        for msg in &messages {
            let data = postcard::to_allocvec(msg).expect("序列化失败");
            assert!(!data.is_empty(), "序列化输出不应为空");
        }
    }

    #[test]
    fn test_ipc_message_empty_payload() {
        // 验证空 payload 的消息可正确序列化和反序列化
        let msg = IpcMessage::Ready { widget_count: 0 };
        let data = postcard::to_allocvec(&msg).unwrap();
        let back: IpcMessage = postcard::from_bytes(&data).unwrap();
        assert_eq!(back, msg);

        let msg = IpcMessage::InputEvent { event_data: vec![] };
        let data = postcard::to_allocvec(&msg).unwrap();
        let back: IpcMessage = postcard::from_bytes(&data).unwrap();
        assert_eq!(back, msg);
    }

    #[test]
    fn test_ipc_message_unicode_error_message() {
        let msg = IpcMessage::Error {
            message: "🔥 文件监控失败：路径包含非法字符 /tmp/测试/中文".into(),
            recoverable: false,
        };
        let data = postcard::to_allocvec(&msg).expect("序列化失败");
        let recovered: IpcMessage = postcard::from_bytes(&data).expect("反序列化失败");
        assert_eq!(recovered, msg);
    }

    #[test]
    fn test_restore_metadata_large_scroll_map() {
        let mut scroll_positions = HashMap::new();
        for i in 0..100 {
            scroll_positions.insert(i, (i as f64 * 10.0, i as f64 * 20.0));
        }
        let metadata = RestoreMetadata {
            focus_path: (0..50).collect(),
            scroll_positions,
            current_route: "/dashboard/analytics".into(),
            window_geometry: (0, 0, 2560, 1440),
        };
        let data = postcard::to_allocvec(&metadata).expect("序列化失败");
        let recovered: RestoreMetadata = postcard::from_bytes(&data).expect("反序列化失败");
        assert_eq!(recovered.focus_path.len(), 50);
        assert_eq!(recovered.scroll_positions.len(), 100);
        assert_eq!(recovered.window_geometry, (0, 0, 2560, 1440));
    }

    // ========================================================================
    // IpcChannel 传输层测试（仅 Unix 平台）
    // ========================================================================

    #[cfg(unix)]
    mod channel_tests {
        use super::*;

        /// 辅助函数：创建一对已连接的 IpcChannel。
        fn channel_pair() -> (IpcChannel, IpcChannel) {
            let (s1, s2) = UnixStream::pair().expect("创建 socket pair 失败");
            (IpcChannel::with_stream(s1), IpcChannel::with_stream(s2))
        }

        #[test]
        fn test_send_recv_shutdown() {
            let (mut a, mut b) = channel_pair();
            let msg = IpcMessage::Shutdown;
            a.send(&msg).unwrap();
            let received = b.recv().unwrap();
            assert_eq!(received, msg);
        }

        #[test]
        fn test_send_recv_ready() {
            let (mut a, mut b) = channel_pair();
            let msg = IpcMessage::Ready { widget_count: 99 };
            a.send(&msg).unwrap();
            let received = b.recv().unwrap();
            assert_eq!(received, msg);
        }

        #[test]
        fn test_send_recv_error() {
            let (mut a, mut b) = channel_pair();
            let msg = IpcMessage::Error {
                message: "组件渲染失败".into(),
                recoverable: true,
            };
            a.send(&msg).unwrap();
            let received = b.recv().unwrap();
            assert_eq!(received, msg);
        }

        #[test]
        fn test_send_recv_restore_state() {
            let (mut a, mut b) = channel_pair();
            let metadata = RestoreMetadata {
                focus_path: vec![1, 2, 3],
                scroll_positions: HashMap::from([(42u64, (300.0, 500.0))]),
                current_route: "/checkout".into(),
                window_geometry: (200, 100, 1280, 720),
            };
            let msg = IpcMessage::RestoreState {
                snapshot: vec![0xAA, 0xBB, 0xCC],
                metadata,
            };
            a.send(&msg).unwrap();
            let received = b.recv().unwrap();
            assert_eq!(received, msg);
        }

        #[test]
        fn test_send_recv_scene_update() {
            let (mut a, mut b) = channel_pair();
            let msg = IpcMessage::SceneUpdate {
                scene_data: vec![0; 1024],
                incremental: false,
            };
            a.send(&msg).unwrap();
            let received = b.recv().unwrap();
            assert_eq!(received, msg);
        }

        #[test]
        fn test_send_recv_input_event() {
            let (mut a, mut b) = channel_pair();
            let msg = IpcMessage::InputEvent {
                event_data: vec![0x01, 0x02, 0x03, 0x04],
            };
            a.send(&msg).unwrap();
            let received = b.recv().unwrap();
            assert_eq!(received, msg);
        }

        #[test]
        fn test_send_recv_multiple_messages() {
            let (mut a, mut b) = channel_pair();
            let messages = vec![
                IpcMessage::Ready { widget_count: 1 },
                IpcMessage::SceneUpdate {
                    scene_data: vec![1, 2, 3],
                    incremental: true,
                },
                IpcMessage::Error {
                    message: "测试错误".into(),
                    recoverable: false,
                },
                IpcMessage::Shutdown,
            ];
            for msg in &messages {
                a.send(msg).unwrap();
                let received = b.recv().unwrap();
                assert_eq!(received, *msg);
            }
        }

        #[test]
        fn test_send_recv_bidirectional() {
            let (mut a, mut b) = channel_pair();
            // a → b
            a.send(&IpcMessage::Ready { widget_count: 10 }).unwrap();
            assert_eq!(b.recv().unwrap(), IpcMessage::Ready { widget_count: 10 });

            // b → a
            b.send(&IpcMessage::Error {
                message: "OK".into(),
                recoverable: true,
            })
            .unwrap();
            assert_eq!(
                a.recv().unwrap(),
                IpcMessage::Error {
                    message: "OK".into(),
                    recoverable: true,
                }
            );
        }

        #[test]
        fn test_send_recv_large_payload() {
            // 大数据发送与接收必须在不同线程进行，避免 Unix socket buffer
            // 写满导致死锁。本测试模拟真实 IPC 场景：一端发送，另一端接收。
            let (mut a, mut b) = channel_pair();
            let scene_data: Vec<u8> = (0..102400).map(|i| (i % 256) as u8).collect();
            let msg = IpcMessage::SceneUpdate {
                scene_data: scene_data.clone(),
                incremental: false,
            };
            let msg_clone = msg.clone();

            let handle = std::thread::spawn(move || {
                let received = b.recv().unwrap();
                assert_eq!(received, msg_clone);
            });

            a.send(&msg).unwrap();
            handle.join().unwrap();
        }

        #[test]
        fn test_recv_disconnected_on_peer_close() {
            let (mut a, b) = channel_pair();
            // 丢弃 b，关闭其对端
            drop(b);
            // a 尝试读取应返回 Disconnected
            let result = a.recv();
            assert!(result.is_err());
            match result {
                Err(IpcError::Disconnected) => {},
                other => panic!("期望 Disconnected，得到 {other:?}"),
            }
        }

        #[test]
        fn test_send_to_closed_peer() {
            let (a, b) = channel_pair();
            // 先发一条消息确保缓冲区有数据
            drop(b);
            // 关闭对端后尝试发送
            let mut a = a;
            // EPIPE/BrokenPipe 可能需要一次写入才能触发
            let result = a.send(&IpcMessage::Shutdown);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                IpcError::Disconnected | IpcError::SendFailed(_)
            ));
        }

        #[test]
        fn test_recv_rejects_oversized_length_prefix() {
            // 向通道写入超过 MAX_MESSAGE_SIZE 的长度前缀，
            // 验证 recv() 返回 ReceiveFailed 而非尝试分配内存。
            let (mut a, mut b) = channel_pair();
            let oversized_len = (MAX_MESSAGE_SIZE as u32 + 1).to_le_bytes();

            use std::io::Write as _;
            a.stream.write_all(&oversized_len).unwrap();

            let result = b.recv();
            assert!(result.is_err());
            match result.unwrap_err() {
                IpcError::ReceiveFailed(msg) => {
                    assert!(msg.contains("超过最大允许值"));
                }
                other => panic!("期望 ReceiveFailed，得到 {other:?}"),
            }
        }

        #[test]
        fn test_max_message_size_constant_is_reasonable() {
            assert_eq!(MAX_MESSAGE_SIZE, 64 * 1024 * 1024);
            assert!(u32::MAX as usize > MAX_MESSAGE_SIZE);
        }

        #[test]
        fn test_all_six_variants_send_recv() {
            let (mut a, mut b) = channel_pair();

            let variants: Vec<IpcMessage> = vec![
                IpcMessage::Shutdown,
                IpcMessage::Ready { widget_count: 7 },
                IpcMessage::Error {
                    message: "e".into(),
                    recoverable: true,
                },
                IpcMessage::SceneUpdate {
                    scene_data: vec![1],
                    incremental: true,
                },
                IpcMessage::InputEvent {
                    event_data: vec![2],
                },
                IpcMessage::RestoreState {
                    snapshot: vec![3],
                    metadata: RestoreMetadata::default(),
                },
            ];

            for (i, msg) in variants.iter().enumerate() {
                a.send(msg).unwrap();
                let received = b.recv().unwrap();
                assert_eq!(received, *msg, "变体 {i} 往返失败");
            }
        }

        #[test]
        fn test_send_recv_with_large_restore_metadata() {
            // 并发 send/recv 避免 socket buffer 死锁。
            let (mut a, mut b) = channel_pair();
            let metadata = RestoreMetadata {
                focus_path: (0..1000).collect(),
                scroll_positions: (0..500).map(|i| (i, (i as f64, (i * 2) as f64))).collect(),
                current_route: "/very/deeply/nested/route/with/many/segments".into(),
                window_geometry: (0, 0, 3840, 2160),
            };
            let msg = IpcMessage::RestoreState {
                snapshot: vec![0xAB; 4096],
                metadata,
            };
            let msg_clone = msg.clone();

            let handle = std::thread::spawn(move || {
                let received = b.recv().unwrap();
                assert_eq!(received, msg_clone);
            });

            a.send(&msg).unwrap();
            handle.join().unwrap();
        }
    }
}
