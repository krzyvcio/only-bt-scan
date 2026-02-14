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
mod config_params;
mod core_bluetooth_integration;
mod data_flow_estimator;
mod data_models;
pub mod db;
mod db_frames;
mod device_events;
mod event_analyzer;
mod gatt_client;
mod hci_packet_parser;
mod hci_realtime_capture;
mod hci_scanner;
mod html_report;
mod interactive_ui;
mod l2cap_analyzer;
mod link_layer;
mod mac_address_handler;
mod native_scanner;
mod packet_tracker;
mod pcap_exporter;
mod raw_packet_integration;
mod raw_packet_parser;
mod raw_sniffer;
mod scanner_integration;
mod telegram_notifier;
mod telemetry;
mod unified_scan;
mod vendor_protocols;
mod windows_bluetooth;
pub mod windows_hci;

#[cfg(target_os = "windows")]
mod tray_manager;

use colored::Colorize;
use crossterm::{cursor::MoveTo, execute};
use std::env;
use std::io::{stdout, Write};
use std::time::Duration;

use bluetooth_scanner::{BluetoothScanner, ScanConfig};
use dotenv::dotenv;
use unified_scan::UnifiedScanEngine;

mod ui_renderer;
mod web_server;

pub async fn run() -> Result<(), anyhow::Error> {
    // Load .env file
    dotenv().ok();

    // Initialize file logger
    // logger::init_logger is not available
    log::info!("Application starting...");
    log::info!("Starting Bluetooth Scanner application");

    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Environment logger initialized");

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

    // ═══════════════════════════════════════════════════════════════
    // LOAD INITIAL TELEMETRY FROM DATABASE
    // ═══════════════════════════════════════════════════════════════
    {
        use std::collections::HashMap;
        use crate::telemetry::{DeviceTelemetryQuick, LatencyStatsQuick, TelemetrySnapshot};
        use chrono::Utc;

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
                    let avg_rssi = rssi_values.iter().map(|&r| r as f64).sum::<f64>() / rssi_values.len() as f64;
                    let packet_count = device_packet_counts.get(mac).copied().unwrap_or(0);
                    
                    // Calculate latency as difference between min/max timestamps
                    let latency_ms = if let Some(timestamps) = device_timestamps.get(mac) {
                        if let (Some(&min_ts), Some(&max_ts)) = (timestamps.iter().min(), timestamps.iter().max()) {
                            max_ts.saturating_sub(min_ts)
                        } else {
                            0
                        }
                    } else {
                        0
                    };
                    
                    devices_map.insert(mac.clone(), DeviceTelemetryQuick {
                        mac: mac.clone(),
                        packet_count,
                        avg_rssi,
                        latencies: LatencyStatsQuick {
                            min_ms: 0,
                            max_ms: latency_ms,  // Total span = max - min
                            avg_ms: 0.0,
                        },
                    });
                }
            }

            // Sort top devices
            let mut top_devices: Vec<(String, u64)> = device_packet_counts.iter()
                .map(|(mac, count)| (mac.clone(), *count))
                .collect();
            top_devices.sort_by(|a, b| b.1.cmp(&a.1));
            top_devices.truncate(20);

            // Create and save snapshot (timestamp COMES FROM ACTUAL PACKET, not Utc::now())
            let snapshot = TelemetrySnapshot {
                timestamp: snapshot_timestamp,  // ← From packet data, not processing time
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
        if let Err(e) =
            telegram_notifier::send_startup_notification(&adapter.address, &adapter.name).await
        {
            log::warn!("Failed to send startup notification: {}", e);
        }

        // Spawn periodic Telegram report task
        tokio::spawn(async {
            if let Err(e) = telegram_notifier::run_periodic_report_task().await {
                log::warn!("Telegram periodic task failed: {}", e);
            }
        });

        execute!(
            stdout(),
            MoveTo(0, ui_renderer::get_device_list_start_line() - 5)
        )?;
        writeln!(
            stdout(),
            "{}",
            "✅ Telegram periodic reports enabled (every 5 minutes)".bright_green()
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
        let (hci_tx, hci_rx) = tokio::sync::mpsc::unbounded_channel::<crate::data_models::RawPacketModel>();
        let mut hci_sniffer = hci_realtime_capture::HciRealTimeSniffer::new();
        
        match hci_sniffer.start(hci_tx) {
            Ok(_) => {
                execute!(stdout(), MoveTo(0, ui_renderer::get_device_list_start_line() - 1))?;
                writeln!(stdout(), "✓ HCI Real-time Capture enabled (packets at 1ms resolution)")?;
                log::info!("HCI Real-time Sniffer started - capturing all BLE traffic");
                
                // Spawn HCI packet processing task
                tokio::spawn(async {
                    hci_realtime_capture::hci_capture_task(hci_rx).await;
                });
            }
            Err(e) => {
                execute!(stdout(), MoveTo(0, ui_renderer::get_device_list_start_line() - 1))?;
                writeln!(stdout(), "⚠️  HCI Capture: {} (requires admin elevation)", e)?;
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
    let shutdown_in_progress = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
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

    // Main event loop — w trybie ciągłym: odświeżanie w miejscu (bez przewijania), bez przerwy
    let start_line = ui_renderer::get_device_list_start_line();
    let mut scan_count = 0;

    while !shutdown_in_progress.load(std::sync::atomic::Ordering::Relaxed) {
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
                "🔄 Scan #{:03} | {}",
                scan_count,
                chrono::Local::now().format("%H:%M:%S")
            )
            .bold()
        )?;
        writeln!(stdout(), "{}", "─".repeat(60).blue())?;
        writeln!(stdout())?;
        stdout().flush()?;

        match unified_engine.run_scan().await {
            Ok(results) => {
                let devices = &results.devices;

                log::info!(
                    "📝 Scan complete: {} devices, {} raw packets",
                    devices.len(),
                    results.raw_packets.len()
                );

                // ═══════════════════════════════════════════════════════════════
                // UPDATE GLOBAL TELEMETRY
                // ═══════════════════════════════════════════════════════════════
                {
                    use std::collections::HashMap;
                    use crate::telemetry::{DeviceTelemetryQuick, LatencyStatsQuick, TelemetrySnapshot};
                    use chrono::Utc;

                    let mut device_stats: HashMap<String, Vec<i8>> = HashMap::new();
                    let mut device_packet_counts: HashMap<String, u64> = HashMap::new();
                    let mut device_timestamps: HashMap<String, Vec<u64>> = HashMap::new();

                    // Count packets, RSSI values, and timestamps per device
                    for packet in &results.raw_packets {
                        *device_packet_counts.entry(packet.mac_address.clone()).or_insert(0) += 1;
                        device_stats.entry(packet.mac_address.clone())
                            .or_insert_with(Vec::new)
                            .push(packet.rssi);
                        device_timestamps.entry(packet.mac_address.clone())
                            .or_insert_with(Vec::new)
                            .push(packet.timestamp_ms);
                    }

                    // Build device telemetry
                    let mut devices_map: HashMap<String, DeviceTelemetryQuick> = HashMap::new();
                    for (mac, rssi_values) in &device_stats {
                        if !rssi_values.is_empty() {
                            let avg_rssi = rssi_values.iter().map(|&r| r as f64).sum::<f64>() / rssi_values.len() as f64;
                            let packet_count = device_packet_counts.get(mac).copied().unwrap_or(0);
                            
                            // Calculate latency as difference between min/max packet timestamps
                            let latency_ms = if let Some(timestamps) = device_timestamps.get(mac) {
                                if let (Some(&min_ts), Some(&max_ts)) = (timestamps.iter().min(), timestamps.iter().max()) {
                                    max_ts.saturating_sub(min_ts)
                                } else {
                                    0
                                }
                            } else {
                                0
                            };
                            
                            devices_map.insert(mac.clone(), DeviceTelemetryQuick {
                                mac: mac.clone(),
                                packet_count,
                                avg_rssi,
                                latencies: LatencyStatsQuick {
                                    min_ms: 0,
                                    max_ms: latency_ms,  // Total latency span
                                    avg_ms: 0.0,
                                },
                            });
                        }
                    }

                    // Sort by packet count for top devices
                    let mut top_devices: Vec<(String, u64)> = device_packet_counts.iter()
                        .map(|(mac, count)| (mac.clone(), *count))
                        .collect();
                    top_devices.sort_by(|a, b| b.1.cmp(&a.1));
                    top_devices.truncate(20);

                    // Create snapshot - use the max timestamp from all packets
                    let snapshot_timestamp = if let Some(max_ts) = results.raw_packets.iter()
                        .map(|p| p.timestamp_ms)
                        .max() {
                        chrono::DateTime::<Utc>::from_timestamp_millis(max_ts as i64)
                            .unwrap_or_else(|| Utc::now())
                    } else {
                        Utc::now()
                    };

                    let snapshot = TelemetrySnapshot {
                        timestamp: snapshot_timestamp,  // Latest packet time from this scan
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

                // Save devices to database
                if let Err(e) = BluetoothScanner::new(config.clone())
                    .save_devices_to_db(devices)
                    .await
                {
                    writeln!(stdout(), "{}", format!("⚠️  DB Devices: {}", e).yellow())?;
                    log::error!("Failed to save devices: {}", e);
                } else {
                    log::info!("✅ Devices saved to database");
                }

                // Save raw packets to database
                if let Ok(conn) = rusqlite::Connection::open("./bluetooth_scan.db") {
                    if let Err(e) =
                        db_frames::insert_raw_packets_from_scan(&conn, &results.raw_packets)
                    {
                        writeln!(stdout(), "{}", format!("⚠️  DB Packets: {}", e).yellow())?;
                        log::error!("Failed to insert raw packets: {}", e);
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
                writeln!(stdout(), "{}", "┌─────┬───────────────────────┬──────────────┬───────────┬────────────────────────┐".blue())?;
                writeln!(stdout(), "{}", "│ #   │ Name                  │ MAC          │ RSSI     │ Manufacturer           │".blue())?;
                writeln!(stdout(), "{}", "├─────┼───────────────────────┼──────────────┼───────────┼────────────────────────┤".blue())?;

                for (i, device) in devices.iter().take(15).enumerate() {
                    let name = device.name.as_deref().unwrap_or("Unknown");
                    let mfg = device.manufacturer_name.as_deref().unwrap_or("-");
                    let name_trunc = if name.len() > 19 { &name[..19] } else { name };
                    let mfg_trunc = if mfg.len() > 20 { &mfg[..20] } else { mfg };
                    writeln!(
                        stdout(),
                        "│ {:3} │ {:19} │ {:12} │ {:5} dBm │ {:20} │",
                        i + 1,
                        name_trunc,
                        device.mac_address,
                        device.rssi,
                        mfg_trunc
                    )?;
                }

                writeln!(stdout(), "{}", "└─────┴───────────────────────┴──────────────┴───────────┴────────────────────────┘".blue())?;

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

/// Save current telemetry snapshot to database (called every 5 minutes)
async fn save_telemetry_snapshot() -> anyhow::Result<()> {
    use chrono::Utc;

    // Get current telemetry from global singleton
    if let Some(snapshot) = telemetry::get_global_telemetry() {
        // Save main snapshot
        let snapshot_id = db::save_telemetry_snapshot(
            snapshot.timestamp,
            snapshot.total_packets as i32,
            snapshot.total_devices as i32,
        ).map_err(|e| anyhow::anyhow!("Failed to save snapshot: {}", e))?;

        // Save per-device telemetry
        for (mac, device_telemetry) in &snapshot.devices {
            db::save_device_telemetry(
                snapshot_id,
                mac,
                device_telemetry.packet_count,
                device_telemetry.avg_rssi,
                device_telemetry.latencies.min_ms as u64,
                device_telemetry.latencies.max_ms as u64,
            ).map_err(|e| anyhow::anyhow!("Failed to save device telemetry: {}", e))?;
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
