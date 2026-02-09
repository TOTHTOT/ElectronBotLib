//! ElectronBot 库的图片缓冲区操作。

use crate::modules::constants::{FRAME_HEIGHT, FRAME_SIZE, FRAME_WIDTH};
use crate::modules::types::Color;
use image::DynamicImage;

/// 图片缓冲区（用于 ElectronBot 显示屏）。
#[derive(Debug, Clone)]
pub struct ImageBuffer {
    /// RGB/BGR 像素数据。
    pub data: Vec<u8>,
}

impl ImageBuffer {
    /// 创建新的空图片缓冲区。
    pub fn new() -> Self {
        Self {
            data: vec![0u8; FRAME_SIZE],
        }
    }

    /// 用颜色填充缓冲区。
    pub fn clear(&mut self, color: Color) {
        let (r, g, b) = color.bgr();
        for i in 0..FRAME_SIZE / 3 {
            let idx = i * 3;
            self.data[idx] = b;
            self.data[idx + 1] = g;
            self.data[idx + 2] = r;
        }
    }

    /// 设置单个像素。
    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x >= FRAME_WIDTH || y >= FRAME_HEIGHT {
            return;
        }
        let idx = (y * FRAME_WIDTH + x) * 3;
        let (r, g, b) = color.bgr();
        self.data[idx] = b;
        self.data[idx + 1] = g;
        self.data[idx + 2] = r;
    }

    /// 获取单个像素。
    pub fn get_pixel(&self, x: usize, y: usize) -> Option<Color> {
        if x >= FRAME_WIDTH || y >= FRAME_HEIGHT {
            return None;
        }
        let idx = (y * FRAME_WIDTH + x) * 3;
        Some(Color::Custom(self.data[idx + 2], self.data[idx + 1], self.data[idx]))
    }

    /// 填充矩形。
    pub fn fill_rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: Color) {
        for dy in 0..height {
            for dx in 0..width {
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }

    /// 画圆。
    pub fn draw_circle(&mut self, cx: usize, cy: usize, radius: usize, color: Color) {
        let r2 = radius * radius;
        for y in 0..FRAME_HEIGHT {
            for x in 0..FRAME_WIDTH {
                let dx = x as i32 - cx as i32;
                let dy = y as i32 - cy as i32;
                if dx * dx + dy * dy <= r2 as i32 {
                    self.set_pixel(x, y, color);
                }
            }
        }
    }

    /// 从文件加载图片。
    pub fn load_from_file<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<(), String> {
        let img = image::open(path).map_err(|e| format!("打开图片失败: {}", e))?;
        self.load_from_image(&img);
        Ok(())
    }

    /// 从 DynamicImage 加载。
    pub fn load_from_image(&mut self, img: &DynamicImage) {
        let resized = img.resize_exact(
            FRAME_WIDTH as u32,
            FRAME_HEIGHT as u32,
            image::imageops::FilterType::Nearest,
        );
        let rgb = resized.to_rgb8();

        for (i, pixel) in rgb.pixels().enumerate() {
            let idx = i * 3;
            // 将 RGB 转换为 MCU 所需的 BGR
            self.data[idx] = pixel[2];
            self.data[idx + 1] = pixel[1];
            self.data[idx + 2] = pixel[0];
        }
    }

    /// 从原始 RGB/BGR 数据加载。
    pub fn load_from_data(&mut self, data: &[u8], width: usize, height: usize) -> Result<(), String> {
        if data.len() < width * height * 3 {
            return Err("数据太小".to_string());
        }

        if width == FRAME_WIDTH && height == FRAME_HEIGHT {
            // 直接复制并转换 BGR
            for i in 0..FRAME_SIZE {
                self.data[i] = data[i + 2];
            }
        } else {
            // 缩放到合适大小
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
                        self.data[dst_idx] = data[src_idx + 2];
                        self.data[dst_idx + 1] = data[src_idx + 1];
                        self.data[dst_idx + 2] = data[src_idx];
                    } else {
                        self.data[dst_idx] = 0;
                        self.data[dst_idx + 1] = 0;
                        self.data[dst_idx + 2] = 0;
                    }
                }
            }
        }

        Ok(())
    }

    /// 获取原始数据引用。
    pub fn as_data(&self) -> &[u8] {
        &self.data
    }

    /// 获取原始数据可变引用。
    pub fn as_mut_data(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

impl Default for ImageBuffer {
    fn default() -> Self {
        Self::new()
    }
}
