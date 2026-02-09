use image::DynamicImage;
use rusb::{Context, DeviceHandle, UsbContext};
use std::path::Path;
use thiserror::Error;

const USB_VID: u16 = 0x1001;
const USB_PID: u16 = 0x8023;
const TIMEOUT_MS: u64 = 100;

const FRAME_WIDTH: usize = 240;
const FRAME_HEIGHT: usize = 240;
const FRAME_SIZE: usize = FRAME_WIDTH * FRAME_HEIGHT * 3;
const PACKET_SIZE: usize = 512;
const PACKET_COUNT: usize = 84;
const TAIL_SIZE: usize = 224;

#[derive(Debug, Error)]
pub enum BotError {
    #[error("Device not found")]
    DeviceNotFound,
    #[error("USB error: {0}")]
    UsbError(String),
    #[error("Send failed: {0}")]
    SendFailed(String),
    #[error("Receive failed: {0}")]
    ReceiveFailed(String),
    #[error("Image error: {0}")]
    ImageError(String),
    #[error("Not connected")]
    NotConnected,
}

struct UsbDevice {
    handle: DeviceHandle<Context>,
    write_endpoint: u8,
    read_endpoint: u8,
}

pub struct ElectronBot {
    usb: Option<UsbDevice>,
    is_connected: bool,
    timestamp: u32,
    ping_pong_index: u8,
    frame_buffer_tx: [Vec<u8>; 2],
    extra_data_tx: [Vec<u8>; 2],
    extra_data_rx: [u8; 32],
}

impl ElectronBot {
    pub fn new() -> Self {
        Self {
            usb: None,
            is_connected: false,
            timestamp: 0,
            ping_pong_index: 0,
            frame_buffer_tx: [vec![0u8; FRAME_SIZE], vec![0u8; FRAME_SIZE]],
            extra_data_tx: [vec![0u8; 32], vec![0u8; 32]],
            extra_data_rx: [0u8; 32],
        }
    }

    /// Scan for device (similar to USB_ScanDevice)
    pub fn scan_devices() -> Vec<(u16, u16, String)> {
        let context = match rusb::Context::new() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let mut devices = Vec::new();
        for device in context.devices().unwrap().iter() {
            if let Ok(desc) = device.device_descriptor() {
                let info = format!("{:04x}:{:04x}", desc.vendor_id(), desc.product_id());
                devices.push((desc.vendor_id(), desc.product_id(), info));
            }
        }
        devices
    }

    /// Connect to the robot (similar to USB_OpenDevice)
    pub fn connect(&mut self) -> Result<bool, BotError> {
        let context = rusb::Context::new().map_err(|e| BotError::UsbError(e.to_string()))?;

        for device in context.devices().map_err(|e| BotError::UsbError(e.to_string()))?.iter() {
            if let Ok(desc) = device.device_descriptor() {
                if desc.vendor_id() == USB_VID && desc.product_id() == USB_PID {
                    println!("Found ElectronBot, trying to open...");

                    match device.open() {
                        Ok(handle) => {
                            // Detach kernel driver if needed (both Windows and Linux)
                            if let Ok(true) = handle.kernel_driver_active(0) {
                                println!("Detaching kernel driver...");
                                if let Err(e) = handle.detach_kernel_driver(0) {
                                    eprintln!("Failed to detach kernel driver: {}", e);
                                }
                            }

                            // Get active configuration
                            if let Ok(config) = device.active_config_descriptor() {
                                println!("Active configuration: {}", config.number());

                                // Try all interfaces
                                for interface in config.interfaces() {
                                    let interface_number = interface.number();
                                    println!("Trying interface {}...", interface_number);

                                    for descriptor in interface.descriptors() {
                                        println!("  Interface class: 0x{:02x}", descriptor.class_code());

                                        // Claim interface
                                        match handle.claim_interface(interface_number) {
                                            Ok(_) => {
                                                println!("  Interface claimed!");

                                                // Find bulk endpoints
                                                let mut write_ep = 0x01u8;
                                                let mut read_ep = 0x81u8;
                                                let mut found_bulk_in = false;
                                                let mut found_bulk_out = false;

                                                for endpoint in descriptor.endpoint_descriptors() {
                                                    let addr = endpoint.address();
                                                    let dir = endpoint.direction();
                                                    let transfer_type = endpoint.transfer_type();

                                                    println!("    Endpoint 0x{:02x}: dir={:?}, type={:?}",
                                                             addr, dir, transfer_type);

                                                    if transfer_type == rusb::TransferType::Bulk {
                                                        if dir == rusb::Direction::In {
                                                            read_ep = addr;
                                                            found_bulk_in = true;
                                                        } else {
                                                            write_ep = addr;
                                                            found_bulk_out = true;
                                                        }
                                                    }
                                                }

                                                if found_bulk_in && found_bulk_out {
                                                    println!("Interface {}: IN=0x{:02x}, OUT=0x{:02x}",
                                                             interface_number, read_ep, write_ep);

                                                    self.usb = Some(UsbDevice {
                                                        handle,
                                                        write_endpoint: write_ep,
                                                        read_endpoint: read_ep,
                                                    });
                                                    self.is_connected = true;
                                                    self.timestamp = 0;
                                                    return Ok(true);
                                                } else {
                                                    println!("  No bulk endpoints found, releasing...");
                                                    let _ = handle.release_interface(interface_number);
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("  Failed to claim interface: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to open device: {}", e);
                        }
                    }
                }
            }
        }

        eprintln!("Could not connect to ElectronBot");
        Err(BotError::DeviceNotFound)
    }

    /// Disconnect (similar to USB_CloseDevice)
    pub fn disconnect(&mut self) {
        self.is_connected = false;
        self.usb = None;
    }

    /// Check connection status
    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    /// Bulk transmit (similar to USB_BulkTransmit)
    fn bulk_transmit(&mut self, endpoint: u8, data: &[u8]) -> Result<bool, BotError> {
        let usb = match &mut self.usb {
            Some(u) => u,
            None => return Err(BotError::NotConnected),
        };

        let timeout = std::time::Duration::from_millis(TIMEOUT_MS);

        // Write data
        match usb.handle.write_bulk(endpoint, data, timeout) {
            Ok(written) if written == data.len() => {}
            Ok(written) => {
                return Err(BotError::SendFailed(format!("Incomplete write: {} of {}", written, data.len())));
            }
            Err(e) => {
                return Err(BotError::SendFailed(e.to_string()));
            }
        }

        // Send zero-length packet if data size is multiple of 512 (like USBInterface.cpp)
        if data.len() % 512 == 0 {
            match usb.handle.write_bulk(endpoint, &[], timeout) {
                Ok(_) => {}
                Err(e) => {
                    return Err(BotError::SendFailed(format!("Zero packet failed: {}", e)));
                }
            }
        }

        Ok(true)
    }

    /// Bulk receive (similar to USB_BulkReceive)
    fn bulk_receive(&mut self, endpoint: u8, data: &mut [u8]) -> Result<usize, BotError> {
        let usb = match &mut self.usb {
            Some(u) => u,
            None => return Err(BotError::NotConnected),
        };

        let timeout = std::time::Duration::from_millis(TIMEOUT_MS);

        match usb.handle.read_bulk(endpoint, data, timeout) {
            Ok(read) => Ok(read),
            Err(e) => Err(BotError::ReceiveFailed(e.to_string())),
        }
    }

    /// Sync data with the robot
    pub fn sync(&mut self) -> Result<bool, BotError> {
        if !self.is_connected {
            return Err(BotError::NotConnected);
        }

        self.timestamp += 1;
        let index = self.ping_pong_index as usize;
        self.ping_pong_index = if self.ping_pong_index == 0 { 1 } else { 0 };

        let frame_buffer = self.frame_buffer_tx[index].clone();
        let extra_data = self.extra_data_tx[index].clone();
        let mut rx_buf = [0u8; 32];

        for _cycle in 0..4 {
            // Receive 32 bytes extra data (MCU request)
            let bytes_read = self.bulk_receive(self.usb.as_ref().unwrap().read_endpoint, &mut rx_buf)?;
            if bytes_read != 32 {
                return Err(BotError::ReceiveFailed(format!("Expected 32 bytes, got {}", bytes_read)));
            }
            self.extra_data_rx.copy_from_slice(&rx_buf);

            // Transmit buffer (84 packets of 512 bytes)
            for i in 0..PACKET_COUNT {
                let start = i * PACKET_SIZE;
                let end = start + PACKET_SIZE;
                if !self.bulk_transmit(self.usb.as_ref().unwrap().write_endpoint, &frame_buffer[start..end])? {
                    return Err(BotError::SendFailed("Failed to transmit buffer".to_string()));
                }
            }

            // Prepare frame tail with extra data
            let mut tail_data = [0u8; TAIL_SIZE];
            let tail_start = PACKET_COUNT * PACKET_SIZE;
            tail_data[..192].copy_from_slice(&frame_buffer[tail_start..tail_start + 192]);
            tail_data[192..].copy_from_slice(&extra_data);

            // Transmit frame tail & extra data
            if !self.bulk_transmit(self.usb.as_ref().unwrap().write_endpoint, &tail_data)? {
                return Err(BotError::SendFailed("Failed to transmit tail".to_string()));
            }
        }

        Ok(true)
    }

    /// Set image from file path
    pub fn set_image<P: AsRef<Path>>(&mut self, path: P) -> Result<(), BotError> {
        let img = image::open(path).map_err(|e| BotError::ImageError(e.to_string()))?;
        self.set_image_from_image(&img)
    }

    /// Set image from DynamicImage
    pub fn set_image_from_image(&mut self, img: &DynamicImage) -> Result<(), BotError> {
        let resized = img.resize_exact(
            FRAME_WIDTH as u32,
            FRAME_HEIGHT as u32,
            image::imageops::FilterType::Nearest,
        );
        let rgb = resized.to_rgb8();
        let index = self.ping_pong_index as usize;

        for (i, pixel) in rgb.pixels().enumerate() {
            let idx = i * 3;
            self.frame_buffer_tx[index][idx] = pixel[2];
            self.frame_buffer_tx[index][idx + 1] = pixel[1];
            self.frame_buffer_tx[index][idx + 2] = pixel[0];
        }

        Ok(())
    }

    /// Set image from raw RGB/BGR data
    pub fn set_image_from_data(&mut self, data: &[u8], width: usize, height: usize) -> Result<(), BotError> {
        if data.len() < width * height * 3 {
            return Err(BotError::ImageError("Data too small".to_string()));
        }

        let index = self.ping_pong_index as usize;

        if width == FRAME_WIDTH && height == FRAME_HEIGHT {
            for i in 0..FRAME_SIZE {
                self.frame_buffer_tx[index][i] = data[i + 2];
            }
        } else {
            let min_w = width.min(FRAME_WIDTH);
            let min_h = height.min(FRAME_HEIGHT);
            let offset_x = (FRAME_WIDTH - min_w) / 2;
            let offset_y = (FRAME_HEIGHT - min_h) / 2;

            for y in 0..FRAME_HEIGHT {
                for x in 0..FRAME_WIDTH {
                    let dst_idx = (y * FRAME_WIDTH + x) * 3;

                    if x >= offset_x && x < offset_x + min_w && y >= offset_y && y < offset_y + min_h {
                        let src_x = x - offset_x;
                        let src_y = y - offset_y;
                        let src_idx = (src_y * width + src_x) * 3;
                        self.frame_buffer_tx[index][dst_idx] = data[src_idx + 2];
                        self.frame_buffer_tx[index][dst_idx + 1] = data[src_idx + 1];
                        self.frame_buffer_tx[index][dst_idx + 2] = data[src_idx];
                    } else {
                        self.frame_buffer_tx[index][dst_idx] = 0;
                        self.frame_buffer_tx[index][dst_idx + 1] = 0;
                        self.frame_buffer_tx[index][dst_idx + 2] = 0;
                    }
                }
            }
        }

        Ok(())
    }

    /// Set image from a solid color
    pub fn set_image_from_color(&mut self, color: &[u8]) -> Result<(), BotError> {
        if color.len() < 3 {
            return Err(BotError::ImageError("Color must have 3 components (RGB)".to_string()));
        }

        let index = self.ping_pong_index as usize;
        for i in 0..FRAME_SIZE / 3 {
            let idx = i * 3;
            self.frame_buffer_tx[index][idx] = color[2];
            self.frame_buffer_tx[index][idx + 1] = color[1];
            self.frame_buffer_tx[index][idx + 2] = color[0];
        }

        Ok(())
    }

    /// Set extra data (up to 32 bytes)
    pub fn set_extra_data(&mut self, data: &[u8]) -> Result<(), BotError> {
        if data.len() > 32 {
            return Err(BotError::ImageError("Extra data must be <= 32 bytes".to_string()));
        }

        let index = self.ping_pong_index as usize;
        self.extra_data_tx[index][..data.len()].copy_from_slice(data);
        Ok(())
    }

    /// Get extra data received from robot
    pub fn get_extra_data(&self) -> &[u8; 32] {
        &self.extra_data_rx
    }

    /// Set joint angles for 6 servos
    pub fn set_joint_angles(&mut self, angles: &[f32; 6], enable: bool) -> Result<(), BotError> {
        if angles.len() != 6 {
            return Err(BotError::ImageError("Must provide exactly 6 angles".to_string()));
        }

        let index = self.ping_pong_index as usize;
        self.extra_data_tx[index][0] = if enable { 1 } else { 0 };

        for (j, angle) in angles.iter().enumerate() {
            let bytes = angle.to_le_bytes();
            for (i, byte) in bytes.iter().enumerate() {
                self.extra_data_tx[index][j * 4 + i + 1] = *byte;
            }
        }

        Ok(())
    }

    /// Get joint angles from robot
    pub fn get_joint_angles(&self) -> [f32; 6] {
        let mut angles = [0.0f32; 6];
        for j in 0..6 {
            let bytes = [
                self.extra_data_rx[j * 4 + 1],
                self.extra_data_rx[j * 4 + 2],
                self.extra_data_rx[j * 4 + 3],
                self.extra_data_rx[j * 4 + 4],
            ];
            angles[j] = f32::from_le_bytes(bytes);
        }
        angles
    }
}

impl Drop for ElectronBot {
    fn drop(&mut self) {
        self.disconnect();
    }
}
