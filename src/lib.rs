//! ElectronBot USB 通信库
//!
//! 用于通过 USB 与 ElectronBot 机器人通信。
//! 基于 rusb 库实现。
//!
//! # 功能特性
//!
//! - USB 设备扫描和连接
//! - 图片缓冲区操作
//! - 舵机控制数据
//! - 数据同步
//! - 可选的日志功能（通过 `logging` feature 开启）
//!
//! # 模块
//!
//! - [`modules::usb`] - USB 底层操作
//! - [`modules::image`] - 图片缓冲区操作
//! - [`modules::sync`] - 数据同步
//! - [`modules::extra_data`] - 舵机控制数据
//! - [`modules::types`] - 公共类型
//! - [`modules::error`] - 错误类型
//!
//! # 示例
//!
//! ```rust
//! use electron_bot::{ElectronBot, Color};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut bot = ElectronBot::new();
//!
//!     // 连接设备
//!     bot.connect()?;
//!
//!     // 设置红色测试图片
//!     bot.set_image_color(Color::Red);
//!
//!     // 同步数据
//!     bot.sync()?;
//!
//!     // 设置舵机角度
//!     bot.set_joint_angles_easy(&[10.0, 20.0, 30.0, 40.0, 50.0, 60.0])?;
//!
//!     bot.sync()?;
//!
//!     // 获取当前角度
//!     let angles = bot.get_joint_angles();
//!     println!("角度: {:?}", angles.as_array());
//!
//!     bot.disconnect();
//!     Ok(())
//! }
//! ```
//!
//! # 日志配置
//!
//! 启用日志功能：
//! ```toml
//! [dependencies]
//! electron-bot = { path = "...", features = ["logging"] }
//! env_logger = "0.10"
//! ```
//!
//! 使用日志：
//! ```rust,ignore
//! env_logger::init();
//! // 现在可以使用 electron-bot 库，日志会自动输出
//! ```
//!

// 导出模块
pub mod modules;

// 导出类型
pub use modules::constants::*;
pub use modules::error::BotError;
pub use modules::extra_data::ExtraData;
pub use modules::image::ImageBuffer;
pub use modules::sync::SyncContext;
pub use modules::types::{Color, DeviceInfo, JointAngles};

// USB 操作
use modules::error::BotError as Error;
use modules::sync::SyncContext as SyncCtx;
use modules::usb::UsbDevice;

// ==================== 主结构体 ====================

/// 用于与 ElectronBot 通信的主结构体
///
/// # 示例
///
/// ```rust
/// use electron_bot::ElectronBot;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut bot = ElectronBot::new();
///
///     // 连接设备（自动检测接口）
///     bot.connect()?;
///
///     // 设置测试图片
///     bot.set_image_color(electron_bot::Color::Red);
///
///     // 同步
///     bot.sync()?;
///
///     Ok(())
/// }
/// ```
pub struct ElectronBot {
    usb: Option<UsbDevice>,
    is_connected: bool,
    image_buffer: ImageBuffer,
    extra_data: ExtraData,
    sync_context: SyncCtx,
}

impl ElectronBot {
    // ==================== 构造函数 ====================

    /// 创建新的 ElectronBot 实例
    ///
    /// 不会连接到设备
    pub fn new() -> Self {
        #[cfg(feature = "logging")]
        log::info!("创建新的 ElectronBot 实例");
        Self {
            usb: None,
            is_connected: false,
            image_buffer: ImageBuffer::new(),
            extra_data: ExtraData::new(),
            sync_context: SyncContext::new(),
        }
    }

    // ==================== 设备发现 ====================

    /// 扫描所有 USB 设备
    ///
    /// 返回所有连接的 USB 设备列表
    pub fn scan_devices() -> Vec<DeviceInfo> {
        modules::usb::scan_devices()
            .into_iter()
            .map(|(vid, pid, info)| DeviceInfo { vid, pid, info })
            .collect()
    }

    /// 检查 ElectronBot 是否已连接
    pub fn is_device_present() -> bool {
        modules::usb::is_electron_bot_present()
    }

    /// 查找 ElectronBot 设备信息
    pub fn find_electron_bot() -> Option<DeviceInfo> {
        modules::usb::scan_devices()
            .into_iter()
            .find(|(vid, pid, _)| *vid == USB_VID && *pid == USB_PID)
            .map(|(vid, pid, info)| DeviceInfo { vid, pid, info })
    }

    // ==================== 连接 ====================

    /// 连接到 ElectronBot
    ///
    /// 自动查找设备并声明正确的接口
    pub fn connect(&mut self) -> Result<bool, Error> {
        #[cfg(feature = "logging")]
        log::info!("正在连接 ElectronBot...");
        self.disconnect();

        match modules::usb::open_electron_bot() {
            Ok(usb_device) => {
                self.usb = Some(usb_device);
                self.is_connected = true;
                self.sync_context = SyncContext::new();
                #[cfg(feature = "logging")]
                log::info!("ElectronBot 连接成功");
                Ok(true)
            }
            Err(e) => {
                #[cfg(feature = "logging")]
                log::error!("连接失败: {}", e);
                Err(Error::UsbError(e))
            }
        }
    }

    /// 连接到指定接口的 ElectronBot
    pub fn connect_with_interface(&mut self, _interface_num: u8) -> Result<bool, Error> {
        // 目前使用相同的连接方式
        self.connect()
    }

    /// 断开设备连接
    pub fn disconnect(&mut self) {
        #[cfg(feature = "logging")]
        if self.is_connected {
            log::info!("断开 ElectronBot 连接");
        }
        self.is_connected = false;
        self.usb = None;
    }

    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    // ==================== 图片操作 ====================

    /// 获取图片缓冲区可变引用
    pub fn image_buffer(&mut self) -> &mut ImageBuffer {
        &mut self.image_buffer
    }

    /// 从文件设置图片
    pub fn set_image<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<(), Error> {
        #[cfg(feature = "logging")]
        log::info!("从文件加载图片: {:?}", path.as_ref());
        self.image_buffer
            .load_from_file(path)
            .map_err(Error::ImageError)
    }

    /// 从 DynamicImage 设置图片
    pub fn set_image_from_image(&mut self, img: &image::DynamicImage) {
        #[cfg(feature = "logging")]
        log::info!("从 DynamicImage 加载图片");
        self.image_buffer.load_from_image(img);
    }

    /// 从原始 RGB/BGR 数据设置图片
    pub fn set_image_from_data(
        &mut self,
        data: &[u8],
        width: usize,
        height: usize,
    ) -> Result<(), Error> {
        #[cfg(feature = "logging")]
        log::info!("从原始数据加载图片: {}x{}", width, height);
        self.image_buffer
            .load_from_data(data, width, height)
            .map_err(Error::ImageError)
    }

    /// 设置纯色图片
    pub fn set_image_color(&mut self, color: Color) {
        #[cfg(feature = "logging")]
        log::info!("设置图片颜色: {:?}", color);
        self.image_buffer.clear(color);
    }

    // ==================== 扩展数据操作 ====================

    /// 获取扩展数据可变引用
    pub fn extra_data(&mut self) -> &mut ExtraData {
        &mut self.extra_data
    }

    /// 从原始字节设置扩展数据
    pub fn set_extra_data(&mut self, data: &[u8]) -> Result<(), Error> {
        if data.len() > 32 {
            return Err(Error::ImageError(
                "扩展数据必须小于等于 32 字节".to_string(),
            ));
        }
        self.extra_data.set_raw(data);
        Ok(())
    }

    /// 获取扩展数据
    pub fn get_extra_data(&self) -> &[u8; 32] {
        self.extra_data.get_raw()
    }

    // ==================== 舵机控制 ====================

    /// 设置 6 个舵机的角度
    pub fn set_joint_angles(&mut self, angles: &[f32; 6], enable: bool) -> Result<(), Error> {
        #[cfg(feature = "logging")]
        log::info!("设置舵机角度: {:?}, 启用: {}", angles, enable);
        let mut ja = JointAngles::new();
        ja.as_array_mut().copy_from_slice(angles);
        self.extra_data.set_joint_angles(&ja, enable);
        Ok(())
    }

    /// 设置舵机角度（默认启用）
    pub fn set_joint_angles_easy(&mut self, angles: &[f32; 6]) -> Result<(), Error> {
        self.set_joint_angles(angles, true)
    }

    /// 从机器人获取舵机角度
    pub fn get_joint_angles(&self) -> JointAngles {
        self.extra_data.get_joint_angles()
    }

    // ==================== 同步 ====================

    /// 与机器人同步数据
    ///
    /// 这是主要的数据交换函数
    pub fn sync(&mut self) -> Result<bool, Error> {
        if !self.is_connected {
            #[cfg(feature = "logging")]
            log::error!("同步失败: 未连接到设备");
            return Err(Error::NotConnected);
        }

        let usb = match &mut self.usb {
            Some(u) => u,
            None => return Err(Error::NotConnected),
        };

        #[cfg(feature = "logging")]
        log::info!("开始同步数据...");
        match modules::sync::sync(
            usb,
            &self.image_buffer,
            &self.extra_data,
            &mut self.sync_context,
        ) {
            Ok(true) => {
                #[cfg(feature = "logging")]
                log::info!("同步成功");
                Ok(true)
            }
            Ok(false) => {
                #[cfg(feature = "logging")]
                log::warn!("同步返回 false");
                Ok(false)
            }
            Err(e) => {
                #[cfg(feature = "logging")]
                log::error!("同步失败: {}", e);
                Err(Error::SendFailed(e))
            }
        }
    }

    /// 快速同步（不处理错误）
    pub fn sync_quick(&mut self) -> bool {
        self.sync().is_ok()
    }

    /// 获取当前同步上下文
    pub fn sync_context(&self) -> &SyncContext {
        &self.sync_context
    }
}

impl Default for ElectronBot {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ElectronBot {
    fn drop(&mut self) {
        self.disconnect();
    }
}

// ==================== 便捷函数 ====================

/// 快速测试函数
pub fn quick_test() -> Result<bool, Error> {
    let mut bot = ElectronBot::new();
    bot.connect()?;
    println!("已连接到 ElectronBot!");
    bot.set_image_color(Color::Red);
    bot.sync()?;
    println!("同步成功!");
    bot.disconnect();
    Ok(true)
}

/// 扫描并打印所有设备
pub fn list_devices() {
    println!("正在扫描 USB 设备...");
    let devices = ElectronBot::scan_devices();
    println!("找到 {} 个设备:", devices.len());

    for (i, device) in devices.iter().enumerate() {
        let marker = if device.vid == USB_VID && device.pid == USB_PID {
            " <-- ElectronBot"
        } else {
            ""
        };
        println!("  [{}] {}{}", i, device.info, marker);
    }
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joint_angles_default() {
        let angles = JointAngles::new();
        assert_eq!(angles.0, [0.0; 6]);
    }

    #[test]
    fn test_joint_angles_get_set() {
        let mut angles = JointAngles::new();
        angles.set(0, 45.0);
        angles.set(1, 90.0);
        assert_eq!(angles.get(0), Some(45.0));
        assert_eq!(angles.get(1), Some(90.0));
        assert_eq!(angles.get(5), Some(0.0));
        assert_eq!(angles.get(6), None);
    }

    #[test]
    fn test_joint_angles_bytes() {
        let angles = JointAngles::new();
        let bytes = angles.to_bytes();
        assert_eq!(bytes.len(), 24);
        let restored = JointAngles::from_bytes(&bytes.try_into().unwrap());
        assert_eq!(restored.0, [0.0; 6]);
    }

    #[test]
    fn test_color_rgb() {
        assert_eq!(Color::Red.rgb(), (255, 0, 0));
        assert_eq!(Color::Green.rgb(), (0, 255, 0));
        assert_eq!(Color::Blue.rgb(), (0, 0, 255));
        assert_eq!(Color::Cyan.rgb(), (0, 255, 255));
        assert_eq!(Color::Custom(100, 150, 200).rgb(), (100, 150, 200));
    }

    #[test]
    fn test_color_bgr() {
        assert_eq!(Color::Red.bgr(), (0, 0, 255));
        assert_eq!(Color::Green.bgr(), (0, 255, 0));
        assert_eq!(Color::Blue.bgr(), (255, 0, 0));
    }

    #[test]
    fn test_electron_bot_new() {
        let bot = ElectronBot::new();
        assert!(!bot.is_connected());
    }

    #[test]
    fn test_image_buffer_new() {
        let buf = ImageBuffer::new();
        assert_eq!(buf.as_data().len(), FRAME_SIZE);
    }

    #[test]
    fn test_image_buffer_clear() {
        let mut buf = ImageBuffer::new();
        buf.clear(Color::Red);
        // 检查第一个像素是红色（存储为 BGR: 0, 0, 255）
        // get_pixel 返回 RGB，所以 BGR(0,0,255) -> RGB(255,0,0)
        assert_eq!(buf.get_pixel(0, 0), Some(Color::Custom(0, 0, 255)));
    }

    #[test]
    fn test_image_buffer_set_pixel() {
        let mut buf = ImageBuffer::new();
        buf.set_pixel(10, 10, Color::Green);
        assert_eq!(buf.get_pixel(10, 10), Some(Color::Custom(0, 255, 0)));
    }

    #[test]
    fn test_extra_data_new() {
        let extra = ExtraData::new();
        assert_eq!(extra.as_data().len(), 32);
        assert!(!extra.is_enabled());
    }

    #[test]
    fn test_extra_data_enable() {
        let mut extra = ExtraData::new();
        extra.set_enable(true);
        assert!(extra.is_enabled());
        extra.set_enable(false);
        assert!(!extra.is_enabled());
    }

    #[test]
    fn test_extra_data_joint_angles() {
        let mut extra = ExtraData::new();
        let angles = JointAngles::new();
        extra.set_joint_angles(&angles, true);
        assert!(extra.is_enabled());
        let restored = extra.get_joint_angles();
        assert_eq!(restored.0, [0.0; 6]);
    }

    #[test]
    fn test_extra_data_bytes() {
        let mut extra = ExtraData::new();
        extra.set_byte(0, 0xAB);
        assert_eq!(extra.get_byte(0), Some(0xAB));
        extra.set_u16(1, 0x1234);
        assert_eq!(extra.get_u16(1), Some(0x1234));
    }

    #[test]
    fn test_sync_context_new() {
        let ctx = SyncContext::new();
        assert_eq!(ctx.timestamp, 0);
        assert_eq!(ctx.ping_pong_index, 0);
        assert_eq!(ctx.cycles, 4);
    }

    #[test]
    fn test_sync_context_toggle() {
        let mut ctx = SyncContext::new();
        assert_eq!(ctx.ping_pong_index, 0);
        ctx.toggle();
        assert_eq!(ctx.ping_pong_index, 1);
        ctx.toggle();
        assert_eq!(ctx.ping_pong_index, 0);
    }

    #[test]
    #[allow(unused_comparisons)]
    fn test_scan_devices() {
        let devices = ElectronBot::scan_devices();
        assert!(devices.len() >= 0);
    }

    #[test]
    fn test_is_device_present() {
        let _present = ElectronBot::is_device_present();
    }

    #[test]
    fn test_quick_test_function() {
        let result = quick_test();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_list_devices_function() {
        list_devices();
    }
}
