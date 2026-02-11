//! ElectronBot 库的数据同步操作。

use crate::modules::constants::{PACKET_COUNT, PACKET_SIZE, TAIL_SIZE};
use crate::modules::extra_data::ExtraData;
use crate::modules::image::ImageBuffer;
use crate::modules::types::JointAngles;
use crate::modules::usb::UsbDevice;

/// 同步操作结果。
pub type SyncResult = Result<bool, String>;

/// 同步上下文（用于乒乓缓冲）。
#[derive(Debug)]
pub struct SyncContext {
    /// 当前时间戳。
    pub timestamp: u32,
    /// 当前乒乓缓冲区索引。
    pub ping_pong_index: u8,
    /// 同步周期数。
    pub cycles: usize,
}

impl SyncContext {
    /// 创建新的同步上下文。
    pub fn new() -> Self {
        Self {
            timestamp: 0,
            ping_pong_index: 0,
            cycles: 4,
        }
    }

    /// 切换乒乓索引。
    pub fn toggle(&mut self) {
        self.timestamp += 1;
        self.ping_pong_index = if self.ping_pong_index == 0 { 1 } else { 0 };
    }

    /// 获取当前索引。
    pub fn current_index(&self) -> usize {
        self.ping_pong_index as usize
    }
}

impl Default for SyncContext {
    fn default() -> Self {
        Self::new()
    }
}

/// 尝试接收指定长度的数据，带重试
fn receive_with_retry(
    usb: &mut UsbDevice,
    buf: &mut [u8],
    expected_len: usize,
    max_retries: u32,
) -> Result<usize, String> {
    for retry in 0..max_retries {
        match usb.receive(buf) {
            Ok(_len) if _len == expected_len => {
                #[cfg(feature = "logging")]
                log::debug!("Received {} bytes on attempt {}", expected_len, retry + 1);
                return Ok(_len);
            }
            Ok(_len) => {
                #[cfg(feature = "logging")]
                log::warn!("Received {} bytes, expected {}", _len, expected_len);
            }
            Err(_) => {
                #[cfg(feature = "logging")]
                log::warn!("Receive failed (attempt {}/{})", retry + 1, max_retries);
            }
        }

        if retry < max_retries - 1 {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    Err(format!(
        "Failed to receive {} bytes after {} retries",
        expected_len, max_retries
    ))
}

/// 发送数据，带重试
fn transmit_with_retry(usb: &mut UsbDevice, data: &[u8], max_retries: u32) -> Result<(), String> {
    for retry in 0..max_retries {
        if usb.transmit(data).is_ok() {
            return Ok(());
        }

        #[cfg(feature = "logging")]
        log::warn!("Transmit failed (attempt {}/{})", retry + 1, max_retries);

        if retry < max_retries - 1 {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    Err("Transmit failed after retries".to_string())
}

/// 执行同步操作。
pub fn sync(
    usb: &mut UsbDevice,
    image_buffer: &ImageBuffer,
    extra_data: &ExtraData,
    context: &mut SyncContext,
) -> SyncResult {
    context.toggle();

    #[cfg(feature = "logging")]
    log::info!(
        "Sync started: timestamp={}, index={}",
        context.timestamp,
        context.current_index()
    );

    let data = image_buffer.as_data();
    let extra = extra_data.as_data();

    // 计算每次循环的偏移增量：84 * 512 + 192 = 43008 + 192 = 43200
    let _cycle_increment = PACKET_COUNT * PACKET_SIZE + 192;
    let mut frame_buffer_offset = 0usize;

    for _cycle in 0..context.cycles {
        #[cfg(feature = "logging")]
        log::debug!("Sync cycle {}/{}", _cycle + 1, context.cycles);

        // 1. 接收 32 字节 extra data（MCU 发送的请求）
        let mut rx_buf = [0u8; 32];
        if let Err(e) = receive_with_retry(usb, &mut rx_buf, 32, 5) {
            #[cfg(feature = "logging")]
            log::warn!("Packet receive failed: {}", e);
            // Suppress unused variable warning when logging is disabled
            #[cfg(not(feature = "logging"))]
            let _ = e;
        }

        // 2. 发送 84 个 512 字节包（带偏移）
        #[cfg(feature = "logging")]
        log::debug!(
            "Transmitting {} packets with offset {}...",
            PACKET_COUNT,
            frame_buffer_offset
        );

        for i in 0..PACKET_COUNT {
            let start = frame_buffer_offset + i * PACKET_SIZE;
            let end = start + PACKET_SIZE;

            if transmit_with_retry(usb, &data[start..end], 3).is_err() {
                #[cfg(feature = "logging")]
                log::error!("Failed to transmit packet {}", i);
            }
        }

        // 更新偏移量（84 * 512 = 43008）
        frame_buffer_offset += PACKET_COUNT * PACKET_SIZE;

        // 3. 准备尾数据（192 字节从当前偏移取 + 32 字节 extra data）
        let mut tail_data = [0u8; TAIL_SIZE];
        tail_data[..192].copy_from_slice(&data[frame_buffer_offset..frame_buffer_offset + 192]);
        tail_data[192..].copy_from_slice(extra);

        // 更新偏移量（加上 192）
        frame_buffer_offset += 192;

        // 4. 发送尾包（224 字节）
        #[cfg(feature = "logging")]
        log::debug!("Transmitting tail packet (224 bytes)...");

        if transmit_with_retry(usb, &tail_data, 3).is_err() {
            #[cfg(feature = "logging")]
            log::error!("Failed to transmit tail data");
        }
    }

    #[cfg(feature = "logging")]
    log::info!("Sync completed: timestamp={}", context.timestamp);
    Ok(true)
}

/// 快速同步（仅图片）。
pub fn sync_image(
    usb: &mut UsbDevice,
    image_buffer: &ImageBuffer,
    context: &mut SyncContext,
) -> SyncResult {
    #[cfg(feature = "logging")]
    log::info!("Starting image sync...");
    let extra = ExtraData::new();
    sync(usb, image_buffer, &extra, context)
}

/// 快速同步（带关节角度）。
pub fn sync_joints(
    usb: &mut UsbDevice,
    angles: &JointAngles,
    context: &mut SyncContext,
) -> SyncResult {
    #[cfg(feature = "logging")]
    log::info!("Starting joints sync with angles: {:?}", angles.as_array());
    let image = ImageBuffer::new();
    let mut extra = ExtraData::new();
    extra.set_joint_angles(angles, true);
    sync(usb, &image, &extra, context)
}
