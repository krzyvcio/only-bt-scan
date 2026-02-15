/// Comprehensive Example: btleplug Scanner with Device Tracking & Database Persistence
/// 
/// This example demonstrates:
/// 1. btleplug standard BLE scanning
/// 2. Device discovery tracking (first/last detection, count)
/// 3. Verbose terminal logging with timestamps
/// 4. Database persistence of all discovered devices
/// 5. Real-time statistics and reporting

use only_bt_scan::device_tracker::DeviceTrackerManager;
use chrono::Utc;
use colored::Colorize;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("\n{}", "═".repeat(120));
    println!("🎯 Bluetooth Device Tracker with btleplug Integration");
    println!("{}", "═".repeat(120));
    println!("Features:");
    println!("  ✓ Real-time device discovery via btleplug");
    println!("  ✓ First & last detection timestamps");
    println!("  ✓ Detection count per MAC address");
    println!("  ✓ Verbose terminal output");
    println!("  ✓ Automatic database persistence");
    println!("  ✓ Manufacturer identification");
    println!("{}", "═".repeat(120));

    // Initialize database
    println!("\n📦 Initializing database...");
    only_bt_scan::db::init_database()?;
    println!("   ✓ Database ready");

    // Create device tracker manager
    let tracker_manager = DeviceTrackerManager::new();

    // Simulate device discoveries (in real scenario, these come from btleplug scanner)
    println!("\n🔍 Simulating device discoveries for demonstration...");
    println!("   (In production, this would come from btleplug scanner)\n");

    // Simulate discovering some devices
    simulate_device_discovery(&tracker_manager).await;

    // Print verbose summary
    println!("\n\n");
    tracker_manager.print_summary();

    // Show detailed report for top device
    println!("\n\n");
    println!("{}", "═".repeat(120));
    println!("📋 DETAILED DEVICE REPORTS");
    println!("{}", "═".repeat(120));

    let devices = tracker_manager.get_all_devices();
    if !devices.is_empty() {
        // Show details for first device
        devices[0].print_verbose();
    }

    // Persist all devices to database
    println!("\n💾 Persisting devices to database...");
    match tracker_manager.persist_all() {
        Ok(count) => {
            println!("   ✓ Persisted {} devices", count.to_string().bright_green().bold());
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }

    // Retrieve devices from database
    println!("\n📂 Retrieving devices from database...");
    match only_bt_scan::db::get_all_devices() {
        Ok(db_devices) => {
            println!("   ✓ Retrieved {} devices from database", db_devices.len().to_string().bright_cyan());
            
            if !db_devices.is_empty() {
                println!("\n   Device List:");
                for device in db_devices.iter().take(5) {
                    println!("   ├─ {} - {} (RSSI: {} dBm, Seen: {} times)",
                        device.mac_address.bright_cyan(),
                        device.name.as_deref().unwrap_or("Unknown").bright_white(),
                        device.rssi.to_string().bright_green(),
                        device.pairing_method.as_deref().unwrap_or("0")
                    );
                }
                if db_devices.len() > 5 {
                    println!("   └─ ... and {} more", (db_devices.len() - 5).to_string().bright_yellow());
                }
            }
        }
        Err(e) => {
            println!("   ❌ Error retrieving devices: {}", e);
        }
    }

    // Export detailed report
    println!("\n📄 Generating detailed report...");
    let report = tracker_manager.export_detailed_report();
    println!("{}", report);

    println!("\n{}", "═".repeat(120));
    println!("✅ Scanner demonstration completed!");
    println!("{}", "═".repeat(120));

    Ok(())
}

/// Simulate btleplug device discoveries
async fn simulate_device_discovery(tracker: &DeviceTrackerManager) {
    // Device 1: Apple iPhone
    tracker.record_detection(
        "AA:BB:CC:DD:EE:01",
        -45,
        "btleplug",
        Some("iPhone 14 Pro".to_string()),
        Some(0x004C),
    );
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    tracker.record_detection(
        "AA:BB:CC:DD:EE:01",
        -48,
        "btleplug",
        Some("iPhone 14 Pro".to_string()),
        Some(0x004C),
    );
    tokio::time::sleep(tokio::time::Duration::from_millis(700)).await;

    tracker.record_detection(
        "AA:BB:CC:DD:EE:01",
        -46,
        "btleplug",
        Some("iPhone 14 Pro".to_string()),
        Some(0x004C),
    );
    
    println!("   ✓ iPhone 14 Pro detected 3 times");

    // Device 2: Xiaomi Mi Band
    tracker.record_detection(
        "11:22:33:44:55:66",
        -60,
        "btleplug",
        Some("Mi Band 7".to_string()),
        Some(0x05AD),
    );
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    tracker.record_detection(
        "11:22:33:44:55:66",
        -62,
        "btleplug",
        Some("Mi Band 7".to_string()),
        Some(0x05AD),
    );

    println!("   ✓ Xiaomi Mi Band 7 detected 2 times");

    // Device 3: Apple AirPods
    tracker.record_detection(
        "FF:EE:DD:CC:BB:AA",
        -52,
        "btleplug",
        Some("AirPods Pro".to_string()),
        Some(0x004C),
    );
    tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;

    tracker.record_detection(
        "FF:EE:DD:CC:BB:AA",
        -50,
        "btleplug",
        Some("AirPods Pro".to_string()),
        Some(0x004C),
    );
    tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;

    tracker.record_detection(
        "FF:EE:DD:CC:BB:AA",
        -54,
        "btleplug",
        Some("AirPods Pro".to_string()),
        Some(0x004C),
    );
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    tracker.record_detection(
        "FF:EE:DD:CC:BB:AA",
        -51,
        "btleplug",
        Some("AirPods Pro".to_string()),
        Some(0x004C),
    );

    println!("   ✓ Apple AirPods Pro detected 4 times");

    // Device 4: Samsung Galaxy Watch
    tracker.record_detection(
        "22:33:44:55:66:77",
        -58,
        "btleplug",
        Some("Galaxy Watch 5".to_string()),
        Some(0x0075),
    );
    tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

    tracker.record_detection(
        "22:33:44:55:66:77",
        -61,
        "btleplug",
        Some("Galaxy Watch 5".to_string()),
        Some(0x0075),
    );

    println!("   ✓ Samsung Galaxy Watch 5 detected 2 times");

    // Device 5: Unknown device
    tracker.record_detection(
        "99:88:77:66:55:44",
        -70,
        "btleplug",
        Some("UnknownDevice".to_string()),
        None,
    );

    println!("   ✓ Unknown device detected 1 time");
}

// After running for a few seconds, you should see:
//
// Output:
// ├─ Device tracking with verbose terminal output showing:
//    - MAC address, device name, manufacturer
//    - First detection time
//    - Last detection time
//    - Number of detections
//    - RSSI signal quality
//    - Detection methods used
//
// ├─ Summary table with devices sorted by detection count
// ├─ Detailed reports for top devices
// ├─ Database persistence status
// └─ Exported detailed report
