//! ElectronBot 库的公共类型定义。

/// 6 个舵机的角度。
#[derive(Debug, Clone, PartialEq)]
pub struct JointAngles(pub [f32; 6]);

impl JointAngles {
    /// 创建新的角度数组（初始化为零）。
    pub fn new() -> Self {
        Self([0.0; 6])
    }

    /// 获取角度数组的引用。
    pub fn as_array(&self) -> &[f32; 6] {
        &self.0
    }

    /// 获取角度数组的可变引用。
    pub fn as_array_mut(&mut self) -> &mut [f32; 6] {
        &mut self.0
    }

    /// 通过索引获取单个角度（0-5）。
    pub fn get(&self, index: usize) -> Option<f32> {
        self.0.get(index).copied()
    }

    /// 通过索引设置单个角度（0-5）。
    pub fn set(&mut self, index: usize, value: f32) -> Option<()> {
        self.0.get_mut(index).map(|v| *v = value)
    }

    /// 转换为字节（小端序）。
    pub fn to_bytes(&self) -> [u8; 24] {
        let mut bytes = [0u8; 24];
        for (i, &val) in self.0.iter().enumerate() {
            bytes[i * 4..i * 4 + 4].copy_from_slice(&val.to_le_bytes());
        }
        bytes
    }

    /// 从字节创建（小端序）。
    pub fn from_bytes(bytes: &[u8; 24]) -> Self {
        let mut angles = [0.0f32; 6];
        for i in 0..6 {
            let mut val_bytes = [0u8; 4];
            val_bytes.copy_from_slice(&bytes[i * 4..i * 4 + 4]);
            angles[i] = f32::from_le_bytes(val_bytes);
        }
        Self(angles)
    }
}

impl Default for JointAngles {
    fn default() -> Self {
        Self::new()
    }
}

/// 用于测试的常用颜色。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    /// 黑色 (0, 0, 0)
    Black,
    /// 白色 (255, 255, 255)
    White,
    /// 红色 (255, 0, 0)
    Red,
    /// 绿色 (0, 255, 0)
    Green,
    /// 蓝色 (0, 0, 255)
    Blue,
    /// 黄色 (255, 255, 0)
    Yellow,
    /// 青色 (0, 255, 255)
    Cyan,
    /// 品红色 (255, 0, 255)
    Magenta,
    /// 自定义 RGB 颜色
    Custom(u8, u8, u8),
}

impl Color {
    /// 获取 RGB 分量。
    pub fn rgb(&self) -> (u8, u8, u8) {
        match self {
            Color::Black => (0, 0, 0),
            Color::White => (255, 255, 255),
            Color::Red => (255, 0, 0),
            Color::Green => (0, 255, 0),
            Color::Blue => (0, 0, 255),
            Color::Yellow => (255, 255, 0),
            Color::Cyan => (0, 255, 255),
            Color::Magenta => (255, 0, 255),
            Color::Custom(r, g, b) => (*r, *g, *b),
        }
    }

    /// 获取 BGR 分量（用于 MCU）。
    pub fn bgr(&self) -> (u8, u8, u8) {
        let (r, g, b) = self.rgb();
        (b, g, r)
    }
}

/// 设备信息。
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// 厂商 ID。
    pub vid: u16,
    /// 产品 ID。
    pub pid: u16,
    /// 设备信息字符串。
    pub info: String,
}
