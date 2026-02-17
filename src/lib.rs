//! # only-bt-scan - Bluetooth LE/Bluetooth Scanner Application
//!
//! Main library for the BLE/Bluetooth scanner application.
//! Supports BLE scanning, database storage, Web API, and Telegram notifications.

mod adapter_info;
mod advertising_parser;
mod android_ble_bridge;
mod background;
mod ble_security;
mod ble_uuids;
mod bluetooth_features;
mod bluetooth_manager;
mod bluetooth_scanner;
mod bluey_integration;
mod class_of_device;
mod company_id_reference;
mod company_ids;
mod config_params;
mod core_bluetooth_integration;
mod data_flow_estimator;
mod data_models;
mod db;
mod db_frames;
mod db_pool;
mod device_events;
mod device_tracker;
mod env_config;
mod event_analyzer;
mod gatt_client;
mod hci_packet_parser;
mod hci_realtime_capture;
mod hci_scanner;
mod html_report;
mod interactive_ui;
mod l2cap_analyzer;
mod link_layer;
mod logger;
mod mac_address_handler;
mod multi_method_scanner;
mod native_scanner;
mod packet_analyzer_terminal;
mod packet_tracker;
mod passive_scanner;
mod pcap_exporter;
mod raw_packet_integration;
mod raw_packet_parser;
mod raw_sniffer;
mod rssi_analyzer;
mod rssi_trend_manager;
mod scanner_integration;
mod telegram_notifier;
#[cfg(test)]
mod telegram_simulator;
mod telemetry;
mod unified_scan;
mod vendor_protocols;
mod windows_bluetooth;
pub mod windows_hci;
mod windows_unified_ble;

#[cfg(target_os = "windows")]
mod tray_manager;

use colored::Colorize;
use crossterm::{cursor::MoveTo, execute};
use std::env;
use std::io::{stdout, Write};
use std::time::Duration;

use bluetooth_scanner::{BluetoothScanner, ScanConfig};
use unified_scan::UnifiedScanEngine;

mod ui_renderer;
mod web_server;

use crate::rssi_trend_manager::GlobalRssiManager;
use std::sync::{Arc, OnceLock};

/// Global RSSI manager for tracking signal strength trends across all devices.
/// 
/// This singleton manages real-time RSSI data for trend analysis and visualization.
static RSSI_MANAGER: OnceLock<Arc<GlobalRssiManager>> = OnceLock::new();

/// Returns the global RSSI manager instance (singleton pattern).
/// 
/// # Returns
/// Arc<GlobalRssiManager> - Shared manager for RSSI trend tracking
/// 
/// # Example
/// ```rust
/// let manager = get_rssi_manager();
/// ```
pub fn get_rssi_manager() -> Arc<GlobalRssiManager> {
    RSSI_MANAGER
        .get_or_init(|| GlobalRssiManager::default())
        .clone()
}

/// Creates a backup of the database before application startup.
/// 
/// The backup is saved as bluetooth_scan.db.bak in the current directory.
/// If no database exists, this function does nothing.
/// 
/// # Side Effects
/// Creates a .bak file if the database exists
fn backup_database() {
    const DB_PATH: &str = "bluetooth_scan.db";
    const DB_BAK: &str = "bluetooth_scan.db.bak";

    // Check if database exists
    if !std::path::Path::new(DB_PATH).exists() {
        return;
    }

    // Try to create backup
    match std::fs::copy(DB_PATH, DB_BAK) {
        Ok(bytes) => {
            println!("📦 Database backup created ({} bytes)", bytes);
            log::info!("Database backup created: {} bytes", bytes);
        }
        Err(e) => {
            println!("⚠️  Backup failed: {}", e);
            log::warn!("Database backup failed: {}", e);
        }
    }
}

/// Restores the database from a .bak backup file.
/// 
/// Removes the corrupted database file and copies from backup.
/// 
/// # Returns
/// bool - true if restore was successful, false otherwise
/// 
/// # Side Effects
/// - Deletes existing bluetooth_scan.db if present
/// - Creates new database from backup file
pub fn restore_database() -> bool {
    const DB_PATH: &str = "bluetooth_scan.db";
    const DB_BAK: &str = "bluetooth_scan.db.bak";

    // Check if backup exists
    if !std::path::Path::new(DB_BAK).exists() {
        println!("❌ No backup file found");
        return false;
    }

    // Remove corrupted database
    if std::path::Path::new(DB_PATH).exists() {
        if let Err(e) = std::fs::remove_file(DB_PATH) {
            println!("❌ Failed to remove corrupted database: {}", e);
            return false;
        }
    }

    // Restore from backup
    match std::fs::copy(DB_BAK, DB_PATH) {
        Ok(bytes) => {
            println!("✅ Database restored from backup ({} bytes)", bytes);
            log::info!("Database restored from backup: {} bytes", bytes);
            true
        }
        Err(e) => {
            println!("❌ Failed to restore database: {}", e);
            log::error!("Database restore failed: {}", e);
            false
        }
    }
}

/// Formats a timestamp for display in the UI.
/// 
/// If the date is today, shows only time (HH:MM:SS).
/// Otherwise shows full date with time (YYYY-MM-DD HH:MM).
/// 
/// # Arguments
/// * `dt` - Reference to a UTC DateTime to format
/// 
/// # Returns
/// String - Formatted timestamp string
fn format_timestamp(dt: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let today = now.date_naive();
    let dt_date = dt.date_naive();

    if dt_date == today {
        // Today - show only time
        dt.format("%H:%M:%S").to_string()
    } else {
        // Not today - show full date with time
        dt.format("%Y-%m-%d %H:%M").to_string()
    }
}

/// Formats a duration as uptime string.
/// 
/// Examples: "1h 23m 45s", "45s", "5m 30s"
/// 
/// # Arguments
/// * `duration` - Duration to format
/// 
/// # Returns
/// String - Human-readable duration (e.g., "1h 23m 45s")
fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();

    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Main application entry point - initializes and runs the Bluetooth scanner.
/// 
/// This async function performs the following initialization steps:
/// 1. Loads configuration from .env file
/// 2. Initializes file logger
/// 3. Creates database backup (if exists)
/// 4. Initializes SQLite database and connection pool
/// 5. Loads company ID reference data
/// 6. Initializes HCI real-time capture (if available)
/// 7. Sets up Telegram notifications (if configured)
/// 8. Starts web server on configured port
/// 9. Launches BLE scanning in continuous mode
/// 
/// # Returns
/// Result<(), anyhow::Error> - Ok on successful shutdown, Error on failure
/// 
/// # Environment Variables
/// - SCAN_DURATION - Duration of each scan cycle in seconds (default: 30)
/// - SCAN_CYCLES - Number of scan cycles to run (default: 3)
/// - WEB_SERVER_PORT - Port for web server (default: 8080)
/// - TELEGRAM_BOT_TOKEN - Bot token for notifications
/// - TELEGRAM_CHAT_ID - Chat ID for notifications
pub async fn run() -> Result<(), anyhow::Error> {
    // Load .env file
    env_config::init();

    // Initialize file logger ONLY - no env_logger stdout output
    // All logs go to file via logger module
    if let Err(e) = logger::init_logger(&logger::get_log_path()) {
        eprintln!("Failed to initialize file logger: {}", e);
    }

    // Draw initial static header
    if let Err(e) = ui_renderer::draw_static_header() {
        log::error!("Failed to draw header: {}", e);
    }

    // Load configuration from .env
    let scan_duration_secs = env::var("SCAN_DURATION")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(30);
    let scan_cycles = env::var("SCAN_CYCLES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(3);
    let _scan_interval_mins = env::var("SCAN_INTERVAL_MINUTES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(5);

    // Setup Windows features
    #[cfg(target_os = "windows")]
    {
        let _tray = tray_manager::TrayManager::new();
        if let Err(e) = _tray.setup_tray() {
            log::warn!("Failed to setup tray: {}", e);
        }
        // Note: tray_manager::prevent_console_close()?;
        execute!(
            stdout(),
            MoveTo(0, ui_renderer::get_device_list_start_line() - 10)
        )?; // Temporary Y coordinate
        writeln!(stdout(), "✓ System Tray support activated")?;
        execute!(
            stdout(),
            MoveTo(0, ui_renderer::get_device_list_start_line() - 9)
        )?; // Temporary Y coordinate
        writeln!(
            stdout(),
            "  ℹ️  Close window to minimize to tray (right-click to exit)"
        )?;
    }

    // Backup database before starting (if exists)
    backup_database();

    // Initialize database
    match db::init_database() {
        Ok(_) => {
            log::info!("Database initialized successfully");
        }
        Err(e) => {
            log::error!("Failed to initialize database: {}", e);
            return Err(anyhow::anyhow!("Database initialization failed: {}", e));
        }
    }

    // Initialize database connection pool
    if let Err(e) = db_pool::init_pool() {
        log::error!("Failed to initialize database pool: {}", e);
        return Err(anyhow::anyhow!(
            "Database pool initialization failed: {}",
            e
        ));
    }
    log::info!("Database connection pool initialized");
    execute!(
        stdout(),
        MoveTo(0, ui_renderer::get_device_list_start_line() - 8)
    )?; // Temporary Y coordinate
    writeln!(stdout(), "✓ Database initialized")?;

    // Initialize raw frame storage tables
    let conn = rusqlite::Connection::open("./bluetooth_scan.db")
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
    log::info!("Initializing frame storage tables");
    db_frames::init_frame_storage(&conn).map_err(|e| {
        log::error!("Frame storage initialization failed: {}", e);
        anyhow::anyhow!("Frame storage init error: {}", e)
    })?;
    log::info!("Frame storage tables initialized successfully");
    execute!(
        stdout(),
        MoveTo(0, ui_renderer::get_device_list_start_line() - 7)
    )?; // Temporary Y coordinate
    writeln!(stdout(), "✓ Raw frame storage initialized")?;
    drop(conn);

    // Initialize company IDs (Bluetooth manufacturers)
    company_ids::init_company_ids();
    execute!(
        stdout(),
        MoveTo(0, ui_renderer::get_device_list_start_line() - 6)
    )?;
    if let Some((count, _)) = company_ids::get_cache_stats() {
        writeln!(stdout(), "✓ Loaded {} Bluetooth manufacturers", count)?;
    }

    // Start background task to update company IDs if needed
    tokio::spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        if let Err(e) = company_ids::check_and_update_cache().await {
            log::warn!("Failed to update company IDs: {}", e);
        }
    });

    // ═══════════════════════════════════════════════════════════════
    // LOAD INITIAL TELEMETRY FROM DATABASE
    // ═══════════════════════════════════════════════════════════════
    {
        use crate::telemetry::{DeviceTelemetryQuick, LatencyStatsQuick, TelemetrySnapshot};
        use chrono::Utc;
        use std::collections::HashMap;

        if let Ok(conn) = rusqlite::Connection::open("./bluetooth_scan.db") {
            // Count packets per device
            let mut device_packet_counts: HashMap<String, u64> = HashMap::new();
            let mut device_rssi_values: HashMap<String, Vec<i8>> = HashMap::new();
            let mut device_timestamps: HashMap<String, Vec<u64>> = HashMap::new();
            let mut latest_timestamp_ms: u64 = 0; // Fallback

            // Query all raw packets from last scan - USE TIMESTAMP_MS (milliseconds)
            if let Ok(mut stmt) = conn.prepare(
                "SELECT mac_address, rssi, timestamp_ms FROM ble_advertisement_frames ORDER BY id DESC LIMIT 5000"
            ) {
                if let Ok(mut rows) = stmt.query([]) {
                    let mut is_first = true;
                    while let Ok(Some(row)) = rows.next() {
                        if let (Ok(mac), Ok(rssi), Ok(ts_ms)) = (
                            row.get::<usize, String>(0),
                            row.get::<usize, i8>(1),
                            row.get::<usize, u64>(2)
                        ) {
                            // Capture latest timestamp from the most recent packet
                            if is_first {
                                latest_timestamp_ms = ts_ms;
                                is_first = false;
                            }
                            
                            *device_packet_counts.entry(mac.clone()).or_insert(0) += 1;
                            device_rssi_values.entry(mac.clone()).or_insert_with(Vec::new).push(rssi);
                            device_timestamps.entry(mac).or_insert_with(Vec::new).push(ts_ms);
                        }
                    }
                }
            }

            // Convert milliseconds to DateTime
            let snapshot_timestamp = if latest_timestamp_ms > 0 {
                chrono::DateTime::<Utc>::from_timestamp_millis(latest_timestamp_ms as i64)
                    .unwrap_or_else(|| Utc::now())
            } else {
                Utc::now()
            };

            // Build telemetry snapshot
            let mut devices_map: HashMap<String, DeviceTelemetryQuick> = HashMap::new();
            for (mac, rssi_values) in &device_rssi_values {
                if !rssi_values.is_empty() {
                    let avg_rssi = rssi_values.iter().map(|&r| r as f64).sum::<f64>()
                        / rssi_values.len() as f64;
                    let packet_count = device_packet_counts.get(mac).copied().unwrap_or(0);

                    // Calculate latency as difference between min/max timestamps
                    let latency_ms = if let Some(timestamps) = device_timestamps.get(mac) {
                        if let (Some(&min_ts), Some(&max_ts)) =
                            (timestamps.iter().min(), timestamps.iter().max())
                        {
                            max_ts.saturating_sub(min_ts)
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                    devices_map.insert(
                        mac.clone(),
                        DeviceTelemetryQuick {
                            mac: mac.clone(),
                            packet_count,
                            avg_rssi,
                            latencies: LatencyStatsQuick {
                                min_ms: 0,
                                max_ms: latency_ms, // Total span = max - min
                                avg_ms: 0.0,
                            },
                        },
                    );
                }
            }

            // Sort top devices
            let mut top_devices: Vec<(String, u64)> = device_packet_counts
                .iter()
                .map(|(mac, count)| (mac.clone(), *count))
                .collect();
            top_devices.sort_by(|a, b| b.1.cmp(&a.1));
            top_devices.truncate(20);

            // Create and save snapshot (timestamp COMES FROM ACTUAL PACKET, not Utc::now())
            let snapshot = TelemetrySnapshot {
                timestamp: snapshot_timestamp, // ← From packet data, not processing time
                total_packets: device_packet_counts.values().sum(),
                total_devices: devices_map.len(),
                devices: devices_map,
                top_devices,
            };

            telemetry::update_global_telemetry(snapshot);
            log::info!("✅ Initial telemetry loaded from database");
        }
    }
    // ═══════════════════════════════════════════════════════════════

    // Display adapter information
    let adapter = adapter_info::AdapterInfo::get_default_adapter();
    adapter_info::display_adapter_info(&adapter);
    adapter_info::log_adapter_info(&adapter);
    log::info!("Using adapter: {} ({})", adapter.name, adapter.address);

    // Setup shutdown flag early (before any async tasks)
    let shutdown_in_progress = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    // Initialize Telegram notifications
    if telegram_notifier::is_enabled() {
        if let Err(e) = telegram_notifier::init_telegram_notifications() {
            writeln!(
                stdout(),
                "{}",
                format!("⚠️  Telegram DB init error: {}", e).yellow()
            )?;
        }

        // Send startup notification
        eprintln!("[TELEGRAM] Sending startup notification...");
        if let Err(e) =
            telegram_notifier::send_startup_notification(&adapter.address, &adapter.name).await
        {
            eprintln!("[TELEGRAM] Startup notification failed: {}", e);
        } else {
            eprintln!("[TELEGRAM] Startup notification sent!");
        }

        // Send initial device report after 5 seconds
        eprintln!("[TELEGRAM] Scheduling initial device report in 5 seconds...");
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            eprintln!("[TELEGRAM] Sending initial device report now...");
            if let Err(e) = telegram_notifier::send_initial_device_report().await {
                eprintln!("[TELEGRAM] Error: {}", e);
            } else {
                eprintln!("[TELEGRAM] Initial device report sent!");
            }
        });

        // Spawn telegram periodic report task in separate thread with own Tokio runtime
        log::info!("[Telegram] Spawning periodic report task (every 15 minutes)");
        let shutdown_telegram = shutdown_in_progress.clone();
        std::thread::spawn(move || {
            eprintln!("[TELEGRAM] Thread started, creating Tokio runtime...");
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for Telegram");
            eprintln!("[TELEGRAM] Runtime created, starting periodic task...");
            rt.block_on(async {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(900)); // 15 minutes
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if shutdown_telegram.load(std::sync::atomic::Ordering::Relaxed) {
                                eprintln!("[TELEGRAM] Shutdown signal received, exiting");
                                break;
                            }
                            if let Err(e) = telegram_notifier::send_periodic_report().await {
                                eprintln!("[TELEGRAM] Failed to send periodic report: {}", e);
                                log::warn!("Failed to send periodic Telegram report: {}", e);
                            }
                        }
                        _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                            if shutdown_telegram.load(std::sync::atomic::Ordering::Relaxed) {
                                break;
                            }
                        }
                    }
                }
            });
            eprintln!("[TELEGRAM] Thread ending");
        });

        execute!(
            stdout(),
            MoveTo(0, ui_renderer::get_device_list_start_line() - 5)
        )?;
        writeln!(
            stdout(),
            "{}",
            "✅ Telegram enabled | Co 1 min: raport + HTML".bright_green()
        )?;
    } else {
        execute!(
            stdout(),
            MoveTo(0, ui_renderer::get_device_list_start_line() - 5)
        )?;
        writeln!(stdout(), "{}", "ℹ️  Telegram notifications not configured (set TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID)".bright_yellow())?;
    }

    // ═══════════════════════════════════════════════════════════════
    // Spawn periodic telemetry persistence task (every 5 minutes)
    // ═══════════════════════════════════════════════════════════════
    tokio::spawn(async {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await; // 5 minutes
            if let Err(e) = save_telemetry_snapshot().await {
                log::warn!("Failed to save telemetry snapshot: {}", e);
            }
        }
    });

    // ═══════════════════════════════════════════════════════════════
    // HCI REAL-TIME CAPTURE (Option B: WinDivert style intercept)
    // Captures ALL Bluetooth traffic at HCI level with ~1ms precision
    // ═══════════════════════════════════════════════════════════════
    {
        let (hci_tx, hci_rx) =
            tokio::sync::mpsc::unbounded_channel::<crate::data_models::RawPacketModel>();
        let mut hci_sniffer = hci_realtime_capture::HciRealTimeSniffer::new();

        match hci_sniffer.start(hci_tx) {
            Ok(_) => {
                execute!(
                    stdout(),
                    MoveTo(0, ui_renderer::get_device_list_start_line() - 1)
                )?;
                writeln!(
                    stdout(),
                    "✓ HCI Real-time Capture enabled (packets at 1ms resolution)"
                )?;
                log::info!("HCI Real-time Sniffer started - capturing all BLE traffic");

                // Spawn HCI packet processing task
                tokio::spawn(async {
                    hci_realtime_capture::hci_capture_task(hci_rx).await;
                });
            }
            Err(e) => {
                execute!(
                    stdout(),
                    MoveTo(0, ui_renderer::get_device_list_start_line() - 1)
                )?;
                writeln!(
                    stdout(),
                    "⚠️  HCI Capture: {} (requires admin elevation)",
                    e
                )?;
                log::warn!("HCI Real-time Capture: {}", e);
            }
        }
    }

    // Start web server in background thread
    let web_port = env::var("WEB_SERVER_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(8080);

    let browser_url = format!("http://localhost:{}", web_port);
    let app_state = web_server::init_state();

    // Spawn web server in a separate thread with its own runtime
    let web_port_clone = web_port;
    std::thread::spawn(move || match tokio::runtime::Runtime::new() {
        Ok(rt) => {
            rt.block_on(async {
                eprintln!("🚀 Web server starting on port {}", web_port_clone);
                match web_server::start_server(web_port_clone, app_state).await {
                    Ok(_) => eprintln!("✅ Web server started successfully"),
                    Err(e) => {
                        eprintln!("❌ Web server error: {}", e);
                        log::error!("Web server error: {}", e);
                    }
                }
            });
        }
        Err(e) => {
            eprintln!("❌ Failed to create tokio runtime: {}", e);
            log::error!("Failed to create tokio runtime: {}", e);
        }
    });

    // Open browser automatically after short delay
    execute!(
        stdout(),
        MoveTo(0, ui_renderer::get_device_list_start_line() - 4)
    )?;
    writeln!(stdout(), "🌐 Web panel: {}", browser_url.bright_cyan())?;

    std::thread::sleep(std::time::Duration::from_millis(1000));

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let _ = Command::new("cmd")
            .args(["/C", "start", &browser_url])
            .spawn();
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        let _ = Command::new("xdg-open").arg(&browser_url).spawn();
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let _ = Command::new("open").arg(&browser_url).spawn();
    }

    execute!(
        stdout(),
        MoveTo(0, ui_renderer::get_device_list_start_line() - 3)
    )?; // Temporary Y coordinate
    writeln!(stdout(), "")?;
    // Configure unified scan engine
    let config = ScanConfig {
        scan_duration: Duration::from_secs(scan_duration_secs),
        num_cycles: scan_cycles,
        use_ble: true,
        use_bredr: cfg!(target_os = "linux"),
    };
    let mut unified_engine = UnifiedScanEngine::new(config.clone());

    // Start device event listener
    let _event_listener = unified_engine.get_event_listener();
    let _event_rx: Option<
        tokio::sync::mpsc::UnboundedReceiver<device_events::DeviceEventNotification>,
    > = None;

    // Shared devices state for interactive UI
    let mut _all_devices: Vec<bluetooth_scanner::BluetoothDevice> = Vec::new();

    // Setup Ctrl+C handler with graceful and forced shutdown
    let shutdown_in_progress_clone = shutdown_in_progress.clone();

    ctrlc::set_handler(move || {
        if shutdown_in_progress_clone.swap(true, std::sync::atomic::Ordering::Relaxed) {
            // Shutdown already in progress - force exit
            execute!(
                stdout(),
                MoveTo(0, ui_renderer::get_device_list_start_line() + 20)
            )
            .ok(); // Use .ok() for non-critical errors
            writeln!(stdout(), "\n❌ Wymuszone zamknięcie aplikacji!").ok(); // Use .ok()
            std::process::exit(1);
        } else {
            // First Ctrl+C - graceful shutdown
            execute!(
                stdout(),
                MoveTo(0, ui_renderer::get_device_list_start_line() + 20)
            )
            .ok(); // Use .ok() for non-critical errors
            writeln!(
                stdout(),
                "\n⚠️  Zamykanie aplikacji... (naciśnij Ctrl+C jeszcze raz aby wymusić)"
            )
            .ok(); // Use .ok()
            execute!(
                stdout(),
                MoveTo(0, ui_renderer::get_device_list_start_line() + 21)
            )
            .ok(); // Use .ok()
            writeln!(stdout(), "📦 Zamykanie połączenia z bazą danych...").ok(); // Use .ok()

            // Close database connection gracefully
            if let Ok(conn) = rusqlite::Connection::open("./bluetooth_scan.db") {
                if let Err(e) = conn.close() {
                    execute!(
                        stdout(),
                        MoveTo(0, ui_renderer::get_device_list_start_line() + 22)
                    )
                    .ok(); // Use .ok()
                    writeln!(stdout(), "⚠️  Błąd zamykania bazy danych: {:?}", e).ok();
                } else {
                    execute!(
                        stdout(),
                        MoveTo(0, ui_renderer::get_device_list_start_line() + 22)
                    )
                    .ok(); // Use .ok()
                    writeln!(stdout(), "✅ Baza danych zamknięta bezpiecznie").ok();
                }
            }
        }

        execute!(
            stdout(),
            MoveTo(0, ui_renderer::get_device_list_start_line() + 23)
        )
        .ok(); // Use .ok()
        writeln!(stdout(), "✅ Aplikacja zakończyła pracę bezpiecznie").ok();
        // Use .ok()
    })
    .expect("Error setting Ctrl-C handler");

    let continuous_mode = true; // Temporarily hardcode for UI integration

    if continuous_mode {
        execute!(
            stdout(),
            MoveTo(0, ui_renderer::get_device_list_start_line() + 2)
        )?; // Position after static header
        writeln!(
            stdout(),
            "{}",
            "🔄 Tryb: CIĄGŁE SKANOWANIE".bright_blue().bold()
        )?;
    } else {
        execute!(
            stdout(),
            MoveTo(0, ui_renderer::get_device_list_start_line() + 2)
        )?; // Position after static header
        writeln!(
            stdout(),
            "{}",
            "⏰ Tryb: SKANOWANIE CO 5 MINUT".bright_blue().bold()
        )?;
    }
    writeln!(stdout())?; // Replaced stdout!() with stdout
    // Main event loop
    let start_line = ui_renderer::get_device_list_start_line();
    let mut scan_count = 0;
    let app_start_time = std::time::Instant::now();

    while !shutdown_in_progress.load(std::sync::atomic::Ordering::Relaxed) {
        // Increment global scan counter in DB
        let _ = db::get_next_scan_number();

        // Clear only the content area and show scan status
        log::debug!("Clearing content area");
        ui_renderer::clear_content_area().map_err(|e| {
            log::error!("Failed to clear content area: {}", e);
            anyhow::anyhow!("UI clear error: {}", e)
        })?;
        execute!(stdout(), MoveTo(0, start_line))?;
        scan_count += 1;

        writeln!(
            stdout(),
            "{}",
            format!(
                "🔄 Scan #{:03} | {} | Uptime: {}",
                scan_count,
                chrono::Local::now().format("%H:%M:%S"),
                format_duration(app_start_time.elapsed())
            )
            .bold()
        )?;
        writeln!(stdout(), "{}", "─".repeat(60).blue())?;
        writeln!(stdout())?;
        stdout().flush()?;

        match unified_engine.run_scan().await {
            Ok(results) => {
                let devices = &results.devices;
                let raw_packets = &results.raw_packets;

                // Debug output to stdout
                println!(
                    "[DEBUG] Scan complete: {} devices, {} raw packets",
                    devices.len(),
                    raw_packets.len()
                );

                log::info!(
                    "📝 Scan complete: {} devices, {} raw packets",
                    devices.len(),
                    results.raw_packets.len()
                );

                // ═══════════════════════════════════════════════════════════════
                // UPDATE GLOBAL TELEMETRY
                // ═══════════════════════════════════════════════════════════════
                {
                    use crate::telemetry::{
                        DeviceTelemetryQuick, LatencyStatsQuick, TelemetrySnapshot,
                    };
                    use chrono::Utc;
                    use std::collections::HashMap;

                    let mut device_stats: HashMap<String, Vec<i8>> = HashMap::new();
                    let mut device_packet_counts: HashMap<String, u64> = HashMap::new();
                    let mut device_timestamps: HashMap<String, Vec<u64>> = HashMap::new();

                    // Count packets, RSSI values, and timestamps per device
                    for packet in &results.raw_packets {
                        *device_packet_counts
                            .entry(packet.mac_address.clone())
                            .or_insert(0) += 1;
                        device_stats
                            .entry(packet.mac_address.clone())
                            .or_insert_with(Vec::new)
                            .push(packet.rssi);
                        device_timestamps
                            .entry(packet.mac_address.clone())
                            .or_insert_with(Vec::new)
                            .push(packet.timestamp_ms);
                    }

                    // Build device telemetry
                    let mut devices_map: HashMap<String, DeviceTelemetryQuick> = HashMap::new();
                    for (mac, rssi_values) in &device_stats {
                        if !rssi_values.is_empty() {
                            let avg_rssi = rssi_values.iter().map(|&r| r as f64).sum::<f64>()
                                / rssi_values.len() as f64;
                            let packet_count = device_packet_counts.get(mac).copied().unwrap_or(0);

                            // Calculate latency as difference between min/max packet timestamps
                            let latency_ms = if let Some(timestamps) = device_timestamps.get(mac) {
                                if let (Some(&min_ts), Some(&max_ts)) =
                                    (timestamps.iter().min(), timestamps.iter().max())
                                {
                                    max_ts.saturating_sub(min_ts)
                                } else {
                                    0
                                }
                            } else {
                                0
                            };

                            devices_map.insert(
                                mac.clone(),
                                DeviceTelemetryQuick {
                                    mac: mac.clone(),
                                    packet_count,
                                    avg_rssi,
                                    latencies: LatencyStatsQuick {
                                        min_ms: 0,
                                        max_ms: latency_ms, // Total latency span
                                        avg_ms: 0.0,
                                    },
                                },
                            );
                        }
                    }

                    // Sort by packet count for top devices
                    let mut top_devices: Vec<(String, u64)> = device_packet_counts
                        .iter()
                        .map(|(mac, count)| (mac.clone(), *count))
                        .collect();
                    top_devices.sort_by(|a, b| b.1.cmp(&a.1));
                    top_devices.truncate(20);

                    // Create snapshot - use the max timestamp from all packets
                    let snapshot_timestamp = if let Some(max_ts) =
                        results.raw_packets.iter().map(|p| p.timestamp_ms).max()
                    {
                        chrono::DateTime::<Utc>::from_timestamp_millis(max_ts as i64)
                            .unwrap_or_else(|| Utc::now())
                    } else {
                        Utc::now()
                    };

                    let snapshot = TelemetrySnapshot {
                        timestamp: snapshot_timestamp, // Latest packet time from this scan
                        total_packets: results.raw_packets.len() as u64,
                        total_devices: devices.len(),
                        devices: devices_map,
                        top_devices,
                    };

                    // Update global
                    crate::telemetry::update_global_telemetry(snapshot);
                    log::info!("✅ Telemetry snapshot updated");
                }
                // ═══════════════════════════════════════════════════════════════
                // EMIT NEW DEVICE ALERTS WITH PACKET ANALYSIS
                {
                    // Show ALL devices (not just new ones)
                    if !devices.is_empty() {
                        writeln!(stdout())?;
                        writeln!(
                            stdout(),
                            "{}",
                            "═══════════════════════════════════════════════════════".bright_cyan()
                        )?;
                        writeln!(stdout(), "{}", "📡 DETECTED DEVICES".bright_green().bold())?;
                        writeln!(
                            stdout(),
                            "{}",
                            "═══════════════════════════════════════════════════════".bright_cyan()
                        )?;

                        // Show ALL packets (no filtering)
                        for packet in &results.raw_packets {
                            let formatted =
                                crate::packet_analyzer_terminal::format_packet_for_terminal(packet);
                            writeln!(stdout(), "{}", formatted)?;
                        }
                        writeln!(
                            stdout(),
                            "{}",
                            "═══════════════════════════════════════════════════════".bright_cyan()
                        )?;
                        writeln!(stdout())?;
                    }
                }
                // ═══════════════════════════════════════════════════════════════

                // Save devices to database and broadcast to WebSocket clients
                if let Err(e) = BluetoothScanner::new(config.clone())
                    .save_devices_to_db(devices)
                    .await
                {
                    writeln!(stdout(), "{}", format!("⚠️  DB Devices: {}", e).yellow())?;
                    log::error!("Failed to save devices: {}", e);
                } else {
                    log::info!("✅ Devices saved to database");
                    // Broadcast devices to WebSocket subscribers
                    for device in devices {
                        let api_device = web_server::convert_to_api_device(device);
                        web_server::broadcast_new_device(&api_device);
                    }
                }

                // Save raw packets to database and broadcast
                if let Ok(conn) = rusqlite::Connection::open("./bluetooth_scan.db") {
                    if let Err(e) =
                        db_frames::insert_raw_packets_from_scan(&conn, &results.raw_packets)
                    {
                        writeln!(stdout(), "{}", format!("⚠️  DB Packets: {}", e).yellow())?;
                        log::error!("Failed to insert raw packets: {}", e);
                    } else {
                        // Broadcast raw packets to WebSocket subscribers
                        for packet in &results.raw_packets {
                            let raw_packet = web_server::convert_to_raw_packet(packet);
                            web_server::broadcast_raw_packet(&raw_packet);
                        }
                    }
                } else {
                    writeln!(
                        stdout(),
                        "{}",
                        "⚠️  Could not connect to DB for packet storage".yellow()
                    )?;
                    log::error!("Could not connect to database for packet storage");
                }

                // Show scan stats
                writeln!(
                    stdout(),
                    "{}",
                    format!(
                        "📱 Found: {} devices | Packets: {} | Time: {}ms",
                        devices.len(),
                        results.packet_sequence.len(),
                        results.duration_ms
                    )
                    .bright_white()
                )?;
                writeln!(stdout(), "{}", "┌─────┬──────────────────┬──────────────┬───────────┬──────────────┬──────────────┬───────┐".blue())?;
                writeln!(stdout(), "{}", "│ #   │ Name             │ MAC          │ RSSI     │ First Seen   │ Last Seen    │ Resp.T│".blue())?;
                writeln!(stdout(), "{}", "├─────┼──────────────────┼──────────────┼───────────┼──────────────┼──────────────┼───────┤".blue())?;

                for (i, device) in devices.iter().take(15).enumerate() {
                    let name = device.name.as_deref().unwrap_or("Unknown");
                    let name_trunc = if name.len() > 16 { &name[..16] } else { name };

                    // Convert nanosecond timestamps to DateTime for formatting
                    let first_seen = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                        device.first_detected_ns / 1_000_000,
                    )
                    .unwrap_or_else(|| chrono::Utc::now());
                    let last_seen = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                        device.last_detected_ns / 1_000_000,
                    )
                    .unwrap_or_else(|| chrono::Utc::now());

                    // Format: show full date if not today, otherwise just time
                    let first_seen_str = format_timestamp(&first_seen);
                    let last_seen_str = format_timestamp(&last_seen);
                    let resp_time_ms = device.response_time_ms;

                    writeln!(
                        stdout(),
                        "│ {:3} │ {:16} │ {:12} │ {:5} dBm │ {:<12} │ {:<12} │ {:5}ms │",
                        i + 1,
                        name_trunc,
                        device.mac_address,
                        device.rssi,
                        first_seen_str,
                        last_seen_str,
                        resp_time_ms
                    )?;
                }

                writeln!(stdout(), "{}", "└─────┴──────────────────┴──────────────┴───────────┴──────────────┴──────────────┴───────┘".blue())?;

                if devices.len() > 15 {
                    writeln!(
                        stdout(),
                        "{}",
                        format!(
                            "... and {} more devices (see web panel)",
                            devices.len() - 15
                        )
                        .yellow()
                    )?;
                }
            }
            Err(e) => {
                let err_msg = format!("Scan error: {}", e);
                log::error!("{}", err_msg);
                execute!(stdout(), MoveTo(0, start_line))?;
                writeln!(stdout(), "{}", format!("❌ Błąd skanu: {}", e).red().bold())?;
                writeln!(stdout(), "{}", "⏳ Ponowienie za 10 s...".yellow())?;
                interactive_ui::display_countdown_interruptible(
                    0,
                    10,
                    shutdown_in_progress.clone(),
                );
            }
        }
    }

    writeln!(stdout(), "")?;
    writeln!(stdout(), "{}", "🔌 Zamykanie zasobów...".bright_yellow())?;

    // Close database connection gracefully
    if let Ok(conn) = rusqlite::Connection::open("./bluetooth_scan.db") {
        if let Err(e) = conn.close() {
            let err_msg = format!("Database close error: {:?}", e);
            log::error!("{}", err_msg);
            writeln!(
                stdout(),
                "{}",
                format!("⚠️  Błąd zamykania bazy danych: {:?}", e).yellow()
            )?;
        } else {
            log::info!("Database closed successfully");
            writeln!(
                stdout(),
                "{}",
                "✅ Baza danych zamknięta bezpiecznie".bright_green()
            )?;
        }
    }

    log::info!("Application shutdown complete");
    log::info!("Application shutdown sequence completed successfully");
    writeln!(stdout(), "")?;
    writeln!(stdout(), "{}", "👋 Do widzenia!".bright_green().bold())?;
    log::info!("Application terminated gracefully");
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// TELEMETRY PERSISTENCE HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Saves the current telemetry snapshot to the database (called every 5 minutes).
/// 
/// Persists global telemetry data including:
/// - Total packets and devices count
/// - Per-device telemetry (packet count, avg RSSI, latency)
/// 
/// Also cleans up old telemetry records older than 30 days.
/// 
/// # Returns
/// Result<(), anyhow::Error> - Ok on success, Error on failure
async fn save_telemetry_snapshot() -> anyhow::Result<()> {
    // Get current telemetry from global singleton
    if let Some(snapshot) = telemetry::get_global_telemetry() {
        // Save main snapshot
        let snapshot_id = db::save_telemetry_snapshot(
            snapshot.timestamp,
            snapshot.total_packets as i32,
            snapshot.total_devices as i32,
        )
        .map_err(|e| anyhow::anyhow!("Failed to save snapshot: {}", e))?;

        // Save per-device telemetry
        for (mac, device_telemetry) in &snapshot.devices {
            db::save_device_telemetry(
                snapshot_id,
                mac,
                device_telemetry.packet_count,
                device_telemetry.avg_rssi,
                device_telemetry.latencies.min_ms as u64,
                device_telemetry.latencies.max_ms as u64,
            )
            .map_err(|e| anyhow::anyhow!("Failed to save device telemetry: {}", e))?;
        }

        log::info!(
            "📊 Saved telemetry snapshot: {} packets, {} devices",
            snapshot.total_packets,
            snapshot.total_devices
        );
    }

    // Cleanup old telemetry (older than 30 days)
    if let Ok(deleted) = db::cleanup_old_telemetry(30) {
        if deleted > 0 {
            log::info!("🧹 Cleaned up {} old telemetry records", deleted);
        }
    }

    Ok(())
}
