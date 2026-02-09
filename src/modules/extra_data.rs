//! ElectronBot 库的舵机控制数据操作。

use crate::modules::types::JointAngles;

/// 扩展数据缓冲区（32 字节，用于舵机控制）。
#[derive(Debug, Clone)]
pub struct ExtraData {
    /// 原始数据缓冲区。
    pub data: [u8; 32],
}

impl ExtraData {
    /// 创建新的扩展数据缓冲区。
    pub fn new() -> Self {
        Self { data: [0u8; 32] }
    }

    /// 清空所有数据。
    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    /// 设置原始数据。
    pub fn set_raw(&mut self, data: &[u8]) {
        if data.len() <= 32 {
            self.data[..data.len()].copy_from_slice(data);
        }
    }

    /// 获取原始数据。
    pub fn get_raw(&self) -> &[u8; 32] {
        &self.data
    }

    /// 获取数据切片。
    pub fn as_data(&self) -> &[u8] {
        &self.data
    }

    /// 获取数据可变切片。
    pub fn as_mut_data(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// 设置启用标志（字节 0）。
    pub fn set_enable(&mut self, enable: bool) {
        self.data[0] = if enable { 1 } else { 0 };
    }

    /// 获取启用标志。
    pub fn is_enabled(&self) -> bool {
        self.data[0] != 0
    }

    /// 设置舵机角度。
    pub fn set_joint_angles(&mut self, angles: &JointAngles, enable: bool) {
        self.set_enable(enable);
        let bytes = angles.to_bytes();
        self.data[1..25].copy_from_slice(&bytes);
    }

    /// 获取舵机角度。
    pub fn get_joint_angles(&self) -> JointAngles {
        let bytes: [u8; 24] = self.data[1..25].try_into().unwrap_or([0u8; 24]);
        JointAngles::from_bytes(&bytes)
    }

    /// 设置指定偏移的字节。
    pub fn set_byte(&mut self, offset: usize, value: u8) {
        if offset < 32 {
            self.data[offset] = value;
        }
    }

    /// 获取指定偏移的字节。
    pub fn get_byte(&self, offset: usize) -> Option<u8> {
        self.data.get(offset).copied()
    }

    /// 设置 16 位值。
    pub fn set_u16(&mut self, offset: usize, value: u16) {
        if offset + 1 < 32 {
            self.data[offset] = (value & 0xFF) as u8;
            self.data[offset + 1] = ((value >> 8) & 0xFF) as u8;
        }
    }

    /// 获取 16 位值。
    pub fn get_u16(&self, offset: usize) -> Option<u16> {
        if offset + 1 < 32 {
            Some(self.data[offset] as u16 | (self.data[offset + 1] as u16) << 8)
        } else {
            None
        }
    }

    /// 设置 32 位浮点数。
    pub fn set_f32(&mut self, offset: usize, value: f32) {
        if offset + 3 < 32 {
            let bytes = value.to_le_bytes();
            self.data[offset..offset + 4].copy_from_slice(&bytes);
        }
    }

    /// 获取 32 位浮点数。
    pub fn get_f32(&self, offset: usize) -> Option<f32> {
        if offset + 3 < 32 {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&self.data[offset..offset + 4]);
            Some(f32::from_le_bytes(bytes))
        } else {
            None
        }
    }
}

impl Default for ExtraData {
    fn default() -> Self {
        Self::new()
    }
}
