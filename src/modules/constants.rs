//! ElectronBot 库的常量定义。

/// ElectronBot 的 USB 厂商 ID。
pub const USB_VID: u16 = 0x1001;

/// ElectronBot 的 USB 产品 ID。
pub const USB_PID: u16 = 0x8023;

/// USB 超时时间（毫秒）。
pub const TIMEOUT_MS: u64 = 100;

/// 图片尺寸。
pub const FRAME_WIDTH: usize = 240;
pub const FRAME_HEIGHT: usize = 240;
pub const FRAME_SIZE: usize = FRAME_WIDTH * FRAME_HEIGHT * 3;
pub const PACKET_SIZE: usize = 512;
pub const PACKET_COUNT: usize = 84;
pub const TAIL_SIZE: usize = 224;
