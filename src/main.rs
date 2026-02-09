fn main() {
    // Scan for devices
    println!("Scanning for USB devices...");
    let devices = electron_bot_rusb::ElectronBot::scan_devices();
    println!("Found {} devices:", devices.len());
    for (i, (vid, pid, name)) in devices.iter().enumerate() {
        println!("  [{}] {} - {}", i, name, if *vid == 0x1001 && *pid == 0x8023 { "<-- ElectronBot" } else { "" });
    }

    // Look for ElectronBot
    let electron_bot = devices.iter().find(|(vid, pid, _)| *vid == 0x1001 && *pid == 0x8023);

    if electron_bot.is_none() {
        println!("\nElectronBot not found! (VID=0x1001, PID=0x8023)");
        println!("Please connect ElectronBot via USB.");
        return;
    }

    println!("\nFound ElectronBot! Connecting...");

    let mut robot = electron_bot_rusb::ElectronBot::new();

    // Connect (tries interface 1 first, like USBInterface.cpp)
    if robot.connect().is_err() {
        println!("Failed to connect!");
        println!("\nNote: On Windows, you may need to:");
        println!("1. Install libusb driver using Zadig (https://zadig.akeo.ie/)");
        println!("2. Replace the USB driver with libusb-win32 for ElectronBot");
        return;
    }

    println!("Connected!");

    // Example: set red test image
    println!("Setting test image (red)...");
    if let Err(e) = robot.set_image_from_color(&[0, 255, 255]) {
        println!("Set color error: {:?}", e);
        return;
    }

    println!("Syncing...");
    match robot.sync() {
        Ok(true) => println!("Sync successful!"),
        Ok(false) => println!("Sync returned false"),
        Err(e) => {
            println!("Sync error: {:?}", e);
            robot.disconnect();
            return;
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(5000));

    // Set servo angles
    println!("Setting joint angles to [10, 20, 30, 40, 50, 60]...");
    if let Err(e) = robot.set_joint_angles(&[10.0, 20.0, 30.0, 40.0, 50.0, 60.0], true) {
        println!("Set angles error: {:?}", e);
        robot.disconnect();
        return;
    }

    match robot.sync() {
        Ok(true) => println!("Sync successful!"),
        Ok(false) => println!("Sync returned false"),
        Err(e) => println!("Sync error: {:?}", e),
    }

    // Print received extra data
    let extra = robot.get_extra_data();
    println!("Extra data received: {:?}", extra);

    robot.disconnect();
    println!("Disconnected!");
}
