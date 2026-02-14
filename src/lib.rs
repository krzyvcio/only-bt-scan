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
mod db;
mod db_frames;
mod device_events;
mod event_analyzer;
mod gatt_client;
mod hci_packet_parser;
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
use std::error::Error;
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
    let scan_interval_mins = env::var("SCAN_INTERVAL_MINUTES")
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

    // Initialize Telegram notifications
    if telegram_notifier::is_enabled() {
        if let Err(e) = telegram_notifier::init_telegram_notifications() {
            writeln!(
                stdout(),
                "{}",
                format!("⚠️  Telegram DB init error: {}", e).yellow()
            )?;
        }
    }

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
    let event_listener = unified_engine.get_event_listener();
    let mut event_rx: Option<
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
