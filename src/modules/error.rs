//! ElectronBot 库的错误类型定义。

use thiserror::Error;

/// 与 ElectronBot 通信时可能发生的错误。
#[derive(Debug, Error)]
pub enum BotError {
    #[error("未找到设备 (VID={0:04x}, PID={1:04x})")]
    DeviceNotFound(u16, u16),

    #[error("USB 错误: {0}")]
    UsbError(String),

    #[error("发送数据失败: {0}")]
    SendFailed(String),

    #[error("接收数据失败: {0}")]
    ReceiveFailed(String),

    #[error("图片错误: {0}")]
    ImageError(String),

    #[error("未连接到设备")]
    NotConnected,

    #[error("未找到接口")]
    InterfaceNotFound,
}
