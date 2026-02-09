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

/// 执行同步操作。
pub fn sync(
    usb: &mut UsbDevice,
    image_buffer: &ImageBuffer,
    extra_data: &ExtraData,
    context: &mut SyncContext,
) -> SyncResult {
    context.toggle();

    #[cfg(feature = "logging")]
    log::info!("Sync started: timestamp={}, index={}", context.timestamp, context.current_index());

    let data = image_buffer.as_data();
    let extra = extra_data.as_data();

    for _cycle in 0..context.cycles {
        #[cfg(feature = "logging")]
        log::debug!("Sync cycle {}/{}", _cycle + 1, context.cycles);

        // 准备尾数据
        let mut tail_data = [0u8; TAIL_SIZE];
        let tail_start = PACKET_COUNT * PACKET_SIZE;
        tail_data[..192].copy_from_slice(&data[tail_start..tail_start + 192]);
        tail_data[192..].copy_from_slice(extra);

        // 尝试接收数据
        let mut rx_buf = [0u8; 32];
        let mut received = false;

        for _retry in 0..3 {
            #[cfg(feature = "logging")]
            log::debug!("Receive attempt {}/3", _retry + 1);

            // 尝试发送小包触发通信
            if usb.transmit(&tail_data[..8]).is_ok() {
                std::thread::sleep(std::time::Duration::from_millis(5));
                match usb.receive(&mut rx_buf) {
                    Ok(32) => {
                        #[cfg(feature = "logging")]
                        log::debug!("Received 32 bytes from device");
                        received = true;
                        break;
                    }
                    Ok(_read) => {
                        #[cfg(feature = "logging")]
                        log::warn!("Received {} bytes, expected 32", _read);
                    }
                    Err(_e) => {
                        #[cfg(feature = "logging")]
                        log::warn!("Receive failed");
                    }
                }
            }

            // 直接尝试接收
            match usb.receive(&mut rx_buf) {
                Ok(32) => {
                    #[cfg(feature = "logging")]
                    log::debug!("Received 32 bytes from device");
                    received = true;
                    break;
                }
                Ok(_read) => {
                    #[cfg(feature = "logging")]
                    log::warn!("Received {} bytes, expected 32", _read);
                }
                Err(_e) => {
                    #[cfg(feature = "logging")]
                    log::warn!("Receive failed");
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        if !received {
            #[cfg(feature = "logging")]
            log::warn!("Failed to receive data in cycle {}", cycle + 1);
        }

        std::thread::sleep(std::time::Duration::from_millis(2));

        // 发送缓冲区（84 个 512 字节包）
        #[cfg(feature = "logging")]
        log::debug!("Transmitting {} packets...", PACKET_COUNT);
        for i in 0..PACKET_COUNT {
            let start = i * PACKET_SIZE;
            let end = start + PACKET_SIZE;
            match usb.transmit(&data[start..end]) {
                Ok(_) => {}
                Err(_e) => {
                    #[cfg(feature = "logging")]
                    log::error!("Failed to transmit packet {}: {}", i, _e);
                }
            }
            std::thread::sleep(std::time::Duration::from_micros(50));
        }

        // 发送尾数据
        match usb.transmit(&tail_data) {
            Ok(_) => {
                #[cfg(feature = "logging")]
                log::debug!("Tail data transmitted");
            }
            Err(_e) => {
                #[cfg(feature = "logging")]
                log::error!("Failed to transmit tail data");
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    #[cfg(feature = "logging")]
    log::info!("Sync completed: timestamp={}", context.timestamp);
    Ok(true)
}

/// 快速同步（仅图片）。
pub fn sync_image(usb: &mut UsbDevice, image_buffer: &ImageBuffer, context: &mut SyncContext) -> SyncResult {
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
