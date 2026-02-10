//! ElectronBot 库的 USB 底层操作。

use rusb::{Context, DeviceHandle, UsbContext};

use crate::modules::constants::{TIMEOUT_MS, USB_PID, USB_VID};

/// 内部 USB 设备句柄。
pub struct UsbDevice {
    /// 设备句柄。
    pub handle: DeviceHandle<Context>,
    /// 发送端点地址。
    pub write_endpoint: u8,
    /// 接收端点地址。
    pub read_endpoint: u8,
}

impl UsbDevice {
    /// 创建新的 USB 设备。
    pub fn new(handle: DeviceHandle<Context>, write_endpoint: u8, read_endpoint: u8) -> Self {
        Self {
            handle,
            write_endpoint,
            read_endpoint,
        }
    }

    /// 通过批量传输发送数据。
    pub fn transmit(&mut self, data: &[u8]) -> Result<bool, String> {
        let timeout = std::time::Duration::from_millis(TIMEOUT_MS);

        // 发送数据
        match self.handle.write_bulk(self.write_endpoint, data, timeout) {
            Ok(written) if written == data.len() => {
                #[cfg(feature = "logging")]
                log::debug!("USB transmit: {} bytes sent", written);
            }
            Ok(_written) => {
                #[cfg(feature = "logging")]
                log::warn!("USB transmit incomplete: {} of {}", _written, data.len());
                return Err("发送不完整".to_string());
            }
            Err(e) => {
                #[cfg(feature = "logging")]
                log::error!("USB transmit failed: {}", e);
                return Err(format!("发送失败: {}", e));
            }
        }

        // 如果需要，发送零包
        if data.len().is_multiple_of(512) {
            if let Err(e) = self.handle.write_bulk(self.write_endpoint, &[], timeout) {
                #[cfg(feature = "logging")]
                log::error!("USB zero packet failed: {}", e);
                return Err(format!("零包失败: {}", e));
            }
        }

        Ok(true)
    }

    /// 通过批量传输接收数据。
    pub fn receive(&mut self, data: &mut [u8]) -> Result<usize, String> {
        let timeout = std::time::Duration::from_millis(TIMEOUT_MS);
        match self.handle.read_bulk(self.read_endpoint, data, timeout) {
            Ok(read) => {
                #[cfg(feature = "logging")]
                log::debug!("USB receive: {} bytes received", read);
                Ok(read)
            }
            Err(e) => {
                #[cfg(feature = "logging")]
                log::error!("USB receive failed: {}", e);
                Err(format!("接收失败: {}", e))
            }
        }
    }

    /// 带重试的发送。
    pub fn transmit_with_retry(&mut self, data: &[u8], max_retries: usize) -> Result<bool, String> {
        for _retry in 0..max_retries {
            match self.transmit(data) {
                Ok(true) => return Ok(true),
                _ => {
                    #[cfg(feature = "logging")]
                    log::warn!("USB transmit retry {}/{}", _retry + 1, max_retries);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
        #[cfg(feature = "logging")]
        log::error!("USB transmit exceeded max retries");
        Err("超过最大重试次数".to_string())
    }

    /// 带重试的接收。
    pub fn receive_with_retry(
        &mut self,
        data: &mut [u8],
        max_retries: usize,
    ) -> Result<usize, String> {
        for retry in 0..max_retries {
            match self.receive(data) {
                Ok(read) if read > 0 => return Ok(read),
                _ => {
                    if retry < max_retries - 1 {
                        #[cfg(feature = "logging")]
                        log::debug!("USB receive retry {}/{}", retry + 1, max_retries);
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            }
        }
        #[cfg(feature = "logging")]
        log::error!("USB receive exceeded max retries");
        Err("超过最大重试次数".to_string())
    }
}

/// 扫描所有 USB 设备。
pub fn scan_devices() -> Vec<(u16, u16, String)> {
    #[cfg(feature = "logging")]
    log::info!("Scanning USB devices...");
    let context = match rusb::Context::new() {
        Ok(c) => c,
        Err(_e) => {
            #[cfg(feature = "logging")]
            log::error!("Failed to create USB context");
            return Vec::new();
        }
    };

    let mut devices = Vec::new();

    match context.devices() {
        Ok(dev_list) => {
            for device in dev_list.iter() {
                if let Ok(desc) = device.device_descriptor() {
                    devices.push((
                        desc.vendor_id(),
                        desc.product_id(),
                        format!("{:04x}:{:04x}", desc.vendor_id(), desc.product_id()),
                    ));
                }
            }
        }
        Err(_e) => {
            #[cfg(feature = "logging")]
            log::error!("Failed to get device list");
        }
    }

    #[cfg(feature = "logging")]
    log::info!("Found {} USB devices", devices.len());
    devices
}

/// 检查 ElectronBot 是否存在。
pub fn is_electron_bot_present() -> bool {
    let present = scan_devices()
        .iter()
        .any(|(vid, pid, _)| *vid == USB_VID && *pid == USB_PID);

    #[cfg(feature = "logging")]
    {
        if present {
            log::info!("ElectronBot device found");
        } else {
            log::info!("ElectronBot device not found");
        }
    }
    present
}

/// 打开 ElectronBot 设备并声明接口。
pub fn open_electron_bot() -> Result<UsbDevice, String> {
    #[cfg(feature = "logging")]
    log::info!(
        "Opening ElectronBot device (VID={:04x}, PID={:04x})...",
        USB_VID,
        USB_PID
    );

    let context = rusb::Context::new().map_err(|e| {
        #[cfg(feature = "logging")]
        log::error!("Failed to create USB context: {}", e);
        format!("创建上下文失败: {}", e)
    })?;

    for device in context
        .devices()
        .map_err(|e| {
            #[cfg(feature = "logging")]
            log::error!("Failed to get devices: {}", e);
            format!("获取设备失败: {}", e)
        })?
        .iter()
    {
        if let Ok(desc) = device.device_descriptor() {
            if desc.vendor_id() == USB_VID && desc.product_id() == USB_PID {
                #[cfg(feature = "logging")]
                log::info!("Found matching device, attempting to open...");

                // 尝试打开设备
                let handle = device.open().map_err(|e| {
                    #[cfg(feature = "logging")]
                    log::error!("Failed to open device: {}", e);
                    format!("打开设备失败: {}", e)
                })?;

                // 如果有内核驱动附着，先分离
                if let Ok(true) = handle.kernel_driver_active(0) {
                    #[cfg(feature = "logging")]
                    log::info!("Detaching kernel driver...");
                    if let Err(_e) = handle.detach_kernel_driver(0) {
                        #[cfg(feature = "logging")]
                        log::warn!("Failed to detach kernel driver");
                    }
                }

                // 获取活动配置
                if let Ok(config) = device.active_config_descriptor() {
                    #[cfg(feature = "logging")]
                    log::info!("Active configuration: {}", config.number());

                    // 尝试所有接口
                    for interface in config.interfaces() {
                        let interface_number = interface.number();
                        #[cfg(feature = "logging")]
                        log::info!("Trying interface {}...", interface_number);

                        for descriptor in interface.descriptors() {
                            // 声明接口
                            if let Err(_e) = handle.claim_interface(interface_number) {
                                #[cfg(feature = "logging")]
                                log::warn!("Failed to claim interface {}", interface_number);
                                continue;
                            }

                            #[cfg(feature = "logging")]
                            log::info!(
                                "Interface {} claimed, searching for bulk endpoints...",
                                interface_number
                            );

                            // 查找批量端点
                            let mut write_ep = 0x01u8;
                            let mut read_ep = 0x81u8;
                            let mut found_in = false;
                            let mut found_out = false;

                            for endpoint in descriptor.endpoint_descriptors() {
                                let addr = endpoint.address();
                                let dir = endpoint.direction();
                                let transfer_type = endpoint.transfer_type();

                                #[cfg(feature = "logging")]
                                log::debug!(
                                    "  Endpoint 0x{:02x}: dir={:?}, type={:?}",
                                    addr,
                                    dir,
                                    transfer_type
                                );

                                if transfer_type == rusb::TransferType::Bulk {
                                    if dir == rusb::Direction::In {
                                        read_ep = addr;
                                        found_in = true;
                                        #[cfg(feature = "logging")]
                                        log::debug!("    Found IN bulk endpoint: 0x{:02x}", addr);
                                    } else {
                                        write_ep = addr;
                                        found_out = true;
                                        #[cfg(feature = "logging")]
                                        log::debug!("    Found OUT bulk endpoint: 0x{:02x}", addr);
                                    }
                                }
                            }

                            if found_in && found_out {
                                #[cfg(feature = "logging")]
                                log::info!(
                                    "Successfully opened ElectronBot: IN=0x{:02x}, OUT=0x{:02x}",
                                    read_ep,
                                    write_ep
                                );
                                return Ok(UsbDevice::new(handle, write_ep, read_ep));
                            }

                            // 如果没有批量端点，释放接口
                            #[cfg(feature = "logging")]
                            log::warn!(
                                "No bulk endpoints found on interface {}, releasing...",
                                interface_number
                            );
                            let _ = handle.release_interface(interface_number);
                        }
                    }
                }

                #[cfg(feature = "logging")]
                log::error!("No suitable interface found on ElectronBot");
                return Err("未找到合适的接口".to_string());
            }
        }
    }

    #[cfg(feature = "logging")]
    log::error!("ElectronBot device not found");
    Err("未找到 ElectronBot".to_string())
}
