# ElectronBot Rust USB 通信库

基于 [rusb](https://crates.io/crates/rusb) 库的 ElectronBot USB 通信库。

## 功能特性

- USB 设备扫描和连接
- 图片缓冲区操作（支持文件加载、纯色填充、像素操作）
- 舵机控制数据（6个关节角度）
- 数据同步（乒乓缓冲策略）
- 可选的日志功能（通过 `logging` feature 开启）

## 依赖

```toml
[dependencies]
electron-bot = { path = "path/to/electron-bot-rusb" }
```

### 启用日志功能

```toml
[dependencies]
electron-bot = { path = "path/to/electron-bot-rusb", features = ["logging"] }
env_logger = "0.10"
```

## 使用方法

### 基本连接和图片同步

```rust
use electron_bot::{ElectronBot, Color};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志（如果启用了 logging feature）
    #[cfg(feature = "logging")]
    env_logger::init();

    let mut bot = ElectronBot::new();

    // 连接设备
    bot.connect()?;

    // 设置红色测试图片
    bot.set_image_color(Color::Red);

    // 同步数据到设备
    bot.sync()?;

    // 断开连接（Drop 时自动断开）
    bot.disconnect();

    Ok(())
}
```

### 从文件加载图片

```rust
fn show_image() -> Result<(), Box<dyn std::error::Error>> {
    let mut bot = ElectronBot::new();
    bot.connect()?;

    // 从文件加载图片
    bot.set_image("path/to/image.png")?;

    bot.sync()?;
    Ok(())
}
```

### 控制舵机角度

```rust
fn move_servos() -> Result<(), Box<dyn std::error::Error>> {
    let mut bot = ElectronBot::new();
    bot.connect()?;

    // 设置 6 个舵机角度（度数）
    let angles = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0];
    bot.set_joint_angles_easy(&angles)?;

    bot.sync()?;
    Ok(())
}

// 获取当前角度
let angles = bot.get_joint_angles();
println!("当前角度: {:?}", angles.as_array());
```

### 设备扫描

```rust
// 列出所有 USB 设备
electron_bot::list_devices();

// 检查 ElectronBot 是否连接
if ElectronBot::is_device_present() {
    println!("ElectronBot 已连接！");
}

// 获取设备信息
if let Some(device) = ElectronBot::find_electron_bot() {
    println!("找到设备: VID={:04x}, PID={:04x}", device.vid, device.pid);
}
```

### 高级用法

#### 直接访问图片缓冲区

```rust
let mut bot = ElectronBot::new();
bot.connect()?;

// 获取图片缓冲区引用
let buffer = bot.image_buffer();

// 设置单个像素
buffer.set_pixel(100, 100, Color::Green);

// 填充矩形
buffer.fill_rect(50, 50, 100, 50, Color::Blue);

// 画圆
buffer.draw_circle(120, 120, 30, Color::Red);

// 同步图片
bot.sync()?;
```

#### 直接访问扩展数据

```rust
let mut bot = ElectronBot::new();
bot.connect()?;

// 获取扩展数据引用
let extra = bot.extra_data();

// 设置原始数据
extra.set_raw(&[1, 2, 3, 4]);

// 设置启用标志
extra.set_enable(true);

bot.sync()?;
```

## API 文档

### ElectronBot 结构体

| 方法 | 描述 |
|------|------|
| `new()` | 创建新实例（不连接） |
| `connect()` | 连接到设备 |
| `disconnect()` | 断开连接 |
| `is_connected()` | 检查是否已连接 |
| `sync()` | 同步数据 |
| `sync_quick()` | 快速同步（忽略错误） |

### 图片操作

| 方法 | 描述 |
|------|------|
| `set_image(path)` | 从文件加载图片 |
| `set_image_from_image(img)` | 从 DynamicImage 加载 |
| `set_image_from_data(data, w, h)` | 从原始数据加载 |
| `set_image_color(color)` | 设置纯色 |

### 舵机控制

| 方法 | 描述 |
|------|------|
| `set_joint_angles(angles, enable)` | 设置舵机角度 |
| `set_joint_angles_easy(angles)` | 设置舵机角度（默认启用） |
| `get_joint_angles()` | 获取当前角度 |

## 构建

```bash
# 不带日志（默认）
cargo build

# 启用日志
cargo build --features logging

# 运行测试
cargo test

# 运行 clippy
cargo clippy
```

## USB 参数

- **VID**: `0x1001`
- **PID**: `0x8023`
- **接口**: 批量传输 (Bulk)
- **帧大小**: 240 x 240 RGB565
- **数据包大小**: 512 字节
- **包数量**: 84 + 1 尾包

## 注意事项

1. 需要 USB 设备连接到电脑
2. Linux/WSL 可能需要配置 udev 规则
    - Ubuntu下配置usb, 让普通用户也能读写usb设备, 再虚拟机Ubuntu下数据写入速度很慢, Windows11下会快很多.
    ```shell
    # 创建以下文件, `99-`确保规则不会被覆盖
    sudo vim /etc/udev/rules.d/99-electronbot.rules
    # 在内部输入
    SUBSYSTEM=="usb", ATTR{idVendor}=="1001", ATTR{idProduct}=="8023", MODE="0666", GROUP="plugdev"
    
    # 保存后重新加载规则
    sudo udevadm control --reload-rules
    sudo udevadm trigger
    ```
3. Windows 上可能需要安装 libusb 驱动
