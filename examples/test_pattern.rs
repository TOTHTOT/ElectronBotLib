//! 测试图案示例
//!
//! 展示如何生成随机色块测试图案并同时控制舵机循环运动。
//!
//! 运行方式：
//! ```bash
//! cargo run --example test_pattern
//! ```
//!
//! 功能：
//! - 每 2 秒切换一次随机色块图案（40x40 平铺）
//! - 所有关节同时循环运动 ±20 度

use electron_bot::ElectronBot;
use std::thread;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 色块大小
const BLOCK_SIZE: usize = 40;

/// 关节运动角度范围（度）
const JOINT_RANGE: f32 = 20.0;

/// 舵机中心角度（根据实际情况调整）
const JOINT_CENTER: [f32; 6] = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

/// 图案切换间隔（毫秒）
const PATTERN_INTERVAL_MS: u64 = 2000;

/// 舵机运动速度（度/秒）
const MOTION_SPEED: f32 = 30.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    #[cfg(feature = "logging")]
    env_logger::init();

    println!("=== ElectronBot 测试图案示例 ===");
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

    // 初始化随机数生成器
    let mut rng = rand::thread_rng();
    let mut pattern_counter = 0u32;

    println!("开始显示测试图案...");
    println!("色块大小: {}x{} 像素", BLOCK_SIZE, BLOCK_SIZE);
    println!("关节运动范围: ±{} 度", JOINT_RANGE);
    println!();

    // 计算舵机运动参数
    let motion_interval_ms = 50; // 运动更新间隔（毫秒）
    let angle_step = MOTION_SPEED * (motion_interval_ms as f32 / 1000.0); // 每次更新角度变化
    let cycle_count = ((JOINT_RANGE * 2.0) / angle_step) as u32; // 完成一次摆动需要的次数

    // 使用原子变量控制循环
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::SeqCst);
    })
    .expect("无法设置 Ctrl+C 处理器");

    let mut current_pattern: Option<electron_bot::ImageBuffer> = None;
    let mut motion_step: i32 = 0;
    let mut last_pattern_time = std::time::Instant::now();

    while running.load(Ordering::SeqCst) {
        let now = std::time::Instant::now();

        // 检查是否需要切换图案
        if now.duration_since(last_pattern_time).as_millis() >= PATTERN_INTERVAL_MS as u128 {
            // 生成新的测试图案
            let mut buffer = electron_bot::ImageBuffer::new();
            buffer.render_test_pattern(&mut rng, BLOCK_SIZE);
            current_pattern = Some(buffer);
            pattern_counter += 1;
            last_pattern_time = now;

            println!(
                "[图案 #{}] 已更新 (间隔 {}ms)",
                pattern_counter, PATTERN_INTERVAL_MS
            );
        }

        // 更新并同步图案
        if let Some(pattern) = &current_pattern {
            bot.image_buffer().as_mut_data().copy_from_slice(pattern.as_data());
            match bot.sync_quick() {
                true => {}
                false => {
                    // 同步失败，可能是设备断开
                    eprintln!("同步失败，尝试重新连接...");
                    if bot.connect().is_err() {
                        eprintln!("重新连接失败");
                        break;
                    }
                }
            }
        }

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
                    println!("角度: [{:6.1}, {:6.1}, {:6.1}, {:6.1}, {:6.1}, {:6.1}]",
                        angles[0], angles[1], angles[2], angles[3], angles[4], angles[5]);
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
