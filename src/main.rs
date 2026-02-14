mod adapter_info;
mod advertising_parser;
mod background;
mod ble_security;
mod ble_uuids;
mod bluetooth_features;
mod bluetooth_scanner;
mod db;
mod db_frames;
mod gatt_client;
mod html_report;
mod interactive_ui;
mod link_layer;
mod raw_sniffer;
mod telegram_notifier;
mod vendor_protocols;

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

mod ui_renderer;
mod web_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load .env file
    dotenv().ok();

    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Draw initial static header
    ui_renderer::draw_static_header()?;

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
        _tray.setup_tray()?;
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
    db::init_database()?;
    execute!(
        stdout(),
        MoveTo(0, ui_renderer::get_device_list_start_line() - 8)
    )?; // Temporary Y coordinate
    writeln!(stdout(), "✓ Database initialized")?;

    // Initialize raw frame storage tables
    let conn = rusqlite::Connection::open("./bluetooth_scan.db")?;
    db_frames::init_frame_storage(&conn)?;
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

    // Initialize Telegram notifications
    if telegram_notifier::is_enabled() {
        if let Err(e) = telegram_notifier::init_telegram_notifications() {
            writeln!(
                stdout(),
                "{}",
                format!("⚠️  Telegram DB init error: {}", e).yellow()
            )?;
        }
        execute!(
            stdout(),
            MoveTo(0, ui_renderer::get_device_list_start_line() - 5)
        )?;
        writeln!(
            stdout(),
            "{}",
            "✅ Telegram notifications enabled (3h cooldown)".bright_green()
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
        .unwrap_or(8000);

    let browser_url = format!("http://localhost:{}", web_port);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let app_state = web_server::init_state();
        rt.block_on(async {
            if let Err(e) = web_server::start_server(web_port, app_state).await {
                log::error!("Web server error: {}", e);
            }
        });
    });

    // Open browser automatically after short delay
    execute!(
        stdout(),
        MoveTo(0, ui_renderer::get_device_list_start_line() - 4)
    )?;
    writeln!(stdout(), "🌐 Web panel: {}", browser_url.bright_cyan())?;

    std::thread::sleep(std::time::Duration::from_millis(500));

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
    // Configure scanner
    let config = ScanConfig {
        scan_duration: Duration::from_secs(scan_duration_secs),
        num_cycles: scan_cycles,
        use_ble: true,
        use_bredr: cfg!(target_os = "linux"),
    };
    let scanner = BluetoothScanner::new(config);

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
                // Use .ok()
                } else {
                    execute!(
                        stdout(),
                        MoveTo(0, ui_renderer::get_device_list_start_line() + 22)
                    )
                    .ok(); // Use .ok()
                    writeln!(stdout(), "✅ Baza danych zamknięta bezpiecznie").ok();
                    // Use .ok()
                }
            }

            execute!(
                stdout(),
                MoveTo(0, ui_renderer::get_device_list_start_line() + 23)
            )
            .ok(); // Use .ok()
            writeln!(stdout(), "✅ Aplikacja zakończyła pracę bezpiecznie").ok();
            // Use .ok()
        }
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
        ui_renderer::clear_content_area()?;
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

        match scanner.run_scan().await {
            Ok(devices) => {
                // Save to database
                if let Err(e) = scanner.save_devices_to_db(&devices).await {
                    writeln!(stdout(), "{}", format!("⚠️  DB: {}", e).yellow())?;
                }

                // Send Telegram notification for new devices
                if telegram_notifier::is_enabled() {
                    if let Err(e) = telegram_notifier::check_and_notify_new_devices(&devices).await
                    {
                        writeln!(stdout(), "{}", format!("⚠️  Telegram: {}", e).yellow())?;
                    }
                }

                // Show devices in simple clean table
                writeln!(
                    stdout(),
                    "{}",
                    format!("📱 Found: {} devices", devices.len()).bright_white()
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

    writeln!(stdout(), "")?; // Replaced stdout!() with stdout
    writeln!(stdout(), "{}", "🔌 Zamykanie zasobów...".bright_yellow())?; // Replaced stdout!() with stdout

    // Close database connection gracefully
    if let Ok(conn) = rusqlite::Connection::open("./bluetooth_scan.db") {
        if let Err(e) = conn.close() {
            writeln!(
                stdout(),
                "{}",
                format!("⚠️  Błąd zamykania bazy danych: {:?}", e).yellow()
            )?;
        } else {
            writeln!(
                stdout(),
                "{}",
                "✅ Baza danych zamknięta bezpiecznie".bright_green()
            )?;
        }
    }

    writeln!(stdout(), "")?;
    writeln!(stdout(), "{}", "👋 Do widzenia!".bright_green().bold())?;
    Ok(())
}
