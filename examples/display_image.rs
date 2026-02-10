//! 图片显示示例
//!
//! 展示如何从文件加载图片并显示到 ElectronBot 屏幕上。
//!
//! 运行方式：
//! ```bash
//! cargo run --example display_image
//! ```
//!
//! 功能：
//! - 从图片文件加载图片
//! - 自动缩放到 240x240
//! - 同时控制舵机做循环运动

use electron_bot::ElectronBot;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// 图片文件路径
const IMAGE_PATH: &str = "./assets/test.png";

/// 关节运动角度范围（度）
const JOINT_RANGE: f32 = 20.0;

/// 舵机中心角度（根据实际情况调整）
const JOINT_CENTER: [f32; 6] = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

/// 舵机运动速度（度/秒）
const MOTION_SPEED: f32 = 30.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    #[cfg(feature = "logging")]
    env_logger::init();

    println!("=== ElectronBot 图片显示示例 ===");
    println!("图片路径: {}", IMAGE_PATH);
    println!("按 Ctrl+C 退出");
    println!();

    let mut bot = ElectronBot::new();

    // 连接设备
    println!("正在连接设备...");
    match bot.connect() {
        Ok(_) => println!("设备连接成功！"),
        Err(e) => {
            eprintln!("连接失败: {:?}", e);
            return Ok(());
        }
    }

    // 加载图片
    println!("正在加载图片: {}...", IMAGE_PATH);
    match bot.set_image(IMAGE_PATH) {
        Ok(_) => println!("图片加载成功！"),
        Err(e) => {
            eprintln!("图片加载失败: {:?}", e);
            eprintln!("请确保 test.png 文件存在于当前目录");
            bot.disconnect();
            return Ok(());
        }
    }

    // 同步图片
    println!("正在同步图片...");
    match bot.sync() {
        Ok(_) => println!("图片同步成功！"),
        Err(e) => {
            eprintln!("图片同步失败: {:?}", e);
            bot.disconnect();
            return Ok(());
        }
    }

    println!("开始舵机运动...");
    println!("关节运动范围: ±{} 度", JOINT_RANGE);
    println!();

    // 计算舵机运动参数
    let motion_interval_ms = 50; // 运动更新间隔（毫秒）
    let angle_step = MOTION_SPEED * (motion_interval_ms as f32 / 1000.0);
    let cycle_count = ((JOINT_RANGE * 2.0) / angle_step) as u32;

    // 使用原子变量控制循环
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::SeqCst);
    })
    .expect("无法设置 Ctrl+C 处理器");

    let mut motion_step: i32 = 0;

    while running.load(Ordering::SeqCst) {
        // 计算当前角度（正弦波形运动）
        let phase = (motion_step as f32 / cycle_count as f32) * std::f32::consts::TAU;
        let offset = JOINT_RANGE * phase.sin();

        let angles: [f32; 6] = [
            JOINT_CENTER[0] + offset,
            JOINT_CENTER[1] + offset,
            JOINT_CENTER[2] + offset,
            JOINT_CENTER[3] + offset,
            JOINT_CENTER[4] + offset,
            JOINT_CENTER[5] + offset,
        ];

        // 设置舵机角度
        bot.set_joint_angles_easy(&angles)?;

        // 同步舵机数据
        match bot.sync() {
            Ok(_) => {
                if motion_step % 20 == 0 {
                    println!(
                        "角度: [{:6.1}, {:6.1}, {:6.1}, {:6.1}, {:6.1}, {:6.1}]",
                        angles[0], angles[1], angles[2], angles[3], angles[4], angles[5]
                    );
                }
            }
            Err(e) => {
                eprintln!("舵机同步失败: {:?}", e);
            }
        }

        // 更新运动步数
        motion_step += 1;
        if motion_step >= cycle_count as i32 {
            motion_step = 0;
        }

        // 控制循环速度
        thread::sleep(Duration::from_millis(motion_interval_ms));
    }

    println!();
    println!("正在断开连接...");

    // 停止舵机
    let stop_angles = JOINT_CENTER;
    let _ = bot.set_joint_angles_easy(&stop_angles);
    let _ = bot.sync();

    bot.disconnect();
    println!("程序已退出");

    Ok(())
}
