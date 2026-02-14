mod ble_uuids;
mod db;
mod db_frames;
mod bluetooth_scanner;
mod background;
mod raw_sniffer;
mod interactive_ui;
mod adapter_info;
mod bluetooth_features;

#[cfg(target_os = "windows")]
mod tray_manager;

use std::error::Error;
use log::info;
use std::time::Duration;
use std::env;

use bluetooth_scanner::{BluetoothScanner, ScanConfig, BluetoothDevice};
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load .env file
    dotenv().ok();
    
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    // Clear screen
    clearscreen::clear().unwrap_or_default();
    
    info!("╔════════════════════════════════════════════════════════════════╗");
    info!("║                  🔵 Bluetooth Scanner v0.1.0                    ║");
    info!("║                   Raw Packet Capture Enabled                    ║");
    info!("╚════════════════════════════════════════════════════════════════╝");
    
    // Load configuration from .env
    let web_port = env::var("WEB_SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let scan_duration_secs = env::var("SCAN_DURATION")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(30);
    let scan_cycles = env::var("SCAN_CYCLES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(3);
    let scan_interval_mins = env::var("SCAN_INTERVAL_MINUTES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(5);
    
    // Display startup info
    info!("📡 Web Server:     http://localhost:{}", web_port);
    info!("⏱️  Scan Duration:  {} seconds", scan_duration_secs);
    info!("🔄 Scan Cycles:    {}", scan_cycles);
    info!("⏰ Interval:        {} minutes", scan_interval_mins);
    info!("");
    
    // Setup Windows features
    #[cfg(target_os = "windows")]
    {
        let _tray = tray_manager::TrayManager::new();
        _tray.setup_tray()?;
        // Note: tray_manager::prevent_console_close()?;
        info!("✓ System Tray support activated");
        info!("  ℹ️  Close window to minimize to tray (right-click to exit)");
    }
    
    // Initialize database
    db::init_database()?;
    info!("✓ Database initialized");
    
    // Initialize raw frame storage tables
    let conn = rusqlite::Connection::open("./bluetooth_scan.db")?;
    db_frames::init_frame_storage(&conn)?;
    info!("✓ Raw frame storage initialized");
    drop(conn);
    
    info!("");
    info!("Starting Bluetooth scan...");
    info!("Navigation: ↑↓ = Move | Enter = Details | Q = Quit");
    info!("");
    
    // Display adapter information
    let adapter = adapter_info::AdapterInfo::get_default_adapter();
    adapter_info::display_adapter_info(&adapter);
    adapter_info::log_adapter_info(&adapter);
    
    // Configure scanner
    let config = ScanConfig {
        scan_duration: Duration::from_secs(scan_duration_secs),
        num_cycles: scan_cycles,
        use_ble: true,
        use_bredr: cfg!(target_os = "linux"),
    };
    
    let scanner = BluetoothScanner::new(config);
    
    // Shared devices state for interactive UI
    let mut all_devices = Vec::new();
    
    // Setup Ctrl+C handler with graceful and forced shutdown
    let shutdown_in_progress = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shutdown_in_progress_clone = shutdown_in_progress.clone();
    
    ctrlc::set_handler(move || {
        if shutdown_in_progress_clone.swap(true, std::sync::atomic::Ordering::Relaxed) {
            // Shutdown already in progress - force exit
            eprintln!("\n❌ Wymuszone zamknięcie aplikacji!");
            std::process::exit(1);
        } else {
            // First Ctrl+C - graceful shutdown
            eprintln!("\n⚠️  Zamykanie aplikacji... (naciśnij Ctrl+C jeszcze raz aby wymusić)");
        }
    })
    .expect("Error setting Ctrl-C handler");
    
    // Show scan mode selection menu
    let continuous_mode = interactive_ui::show_scan_mode_menu();
    
    info!("");
    if continuous_mode {
        info!("🔄 Tryb: CIĄGŁE SKANOWANIE");
    } else {
        info!("⏰ Tryb: SKANOWANIE CO 5 MINUT");
    }
    info!("");
    
    // Main event loop
    while !shutdown_in_progress.load(std::sync::atomic::Ordering::Relaxed) {
        info!("▶️  Starting scan cycle...");
        
        match scanner.run_scan().await {
            Ok(mut devices) => {
                all_devices = devices.clone();
                info!("✅ Scan completed. Found {} devices", devices.len());
                
                // Save to database
                if let Err(e) = scanner.save_devices_to_db(&devices).await {
                    eprintln!("Failed to save devices: {}", e);
                }
                
                // Display simple list
                interactive_ui::display_devices_simple(&devices);
                
                // Show interactive UI option
                println!("\n💡 Press ENTER to browse devices interactively, or wait for next scan");
                
                // Display database stats
                match db::get_device_count() {
                    Ok(count) => info!("📊 Total devices in database: {}", count),
                    Err(e) => eprintln!("Failed to get device count: {}", e),
                }
                
                // Wait for next scan - interval depends on mode
                if continuous_mode {
                    // Continuous: show countdown for 5 seconds before next scan
                    println!();
                    interactive_ui::display_countdown_interruptible(0, 5, shutdown_in_progress.clone());
                } else {
                    // Interval-based: show countdown to next scan (5 minutes)
                    println!();
                    interactive_ui::display_countdown_interruptible(5, 0, shutdown_in_progress.clone());
                }
            }
            Err(e) => {
                eprintln!("❌ Scan failed: {}", e);
                info!("⏳ Retrying in 10 seconds...");
                interactive_ui::display_countdown_interruptible(0, 10, shutdown_in_progress.clone());
            }
        }
        
        if continuous_mode {
            info!("🔄 Continuous scanning mode active. Press Ctrl+C to stop.");
        } else {
            info!("⏰ Interval scanning mode active (5 min intervals). Press Ctrl+C to stop.");
        }
        info!("");
    }
    
    println!();
    info!("👋 Aplikacja zamknięta. Do widzenia!");
    Ok(())
}
