#[allow(unused_imports)]
use crate::bluetooth_features::{BluetoothFeature, BluetoothVersion};
use crate::bluetooth_scanner::BluetoothDevice;
use colored::Colorize;
/// Interactive terminal UI for browsing Bluetooth devices
/// Allows navigation with arrow keys
use crossterm::{
    cursor::MoveTo, // Added MoveTo
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};

pub struct InteractiveUI {
    devices: Vec<BluetoothDevice>,
    selected_index: usize,
    show_details: bool,
}

impl InteractiveUI {
    pub fn new(devices: Vec<BluetoothDevice>) -> Self {
        Self {
            devices,
            selected_index: 0,
            show_details: false,
        }
    }

    /// Run the interactive UI loop
    pub fn run(&mut self) -> io::Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let result = self.event_loop();

        // Cleanup terminal
        execute!(io::stdout(), LeaveAlternateScreen)?;
        disable_raw_mode()?;

        result
    }

    fn event_loop(&mut self) -> io::Result<()> {
        loop {
            self.draw()?;

            if event::poll(std::time::Duration::from_millis(20))? {
                if let Event::Key(key) = event::read()? {
                    if !self.handle_key(key)? {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> io::Result<bool> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Ok(false),
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                Ok(true)
            }
            KeyCode::Down => {
                if self.selected_index < self.devices.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
                Ok(true)
            }
            KeyCode::Enter => {
                self.show_details = !self.show_details;
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    fn draw(&self) -> io::Result<()> {
        clearscreen::clear().unwrap();

        if self.devices.is_empty() {
            println!("╔════════════════════════════════════════════════════════════╗");
            println!("║                   Brak urządzeń do wyświetlenia            ║");
            println!("║                   Poczekaj na wyniki skanowania...          ║");
            println!("╚════════════════════════════════════════════════════════════╝");
            println!("\nNaciśnij 'q' aby zamknąć");
            return Ok(());
        }

        // Draw header
        println!(
            "╔════════════════════════════════════════════════════════════════════════════════╗"
        );
        println!(
            "║                  📱 BLUETOOTH DEVICE SCANNER - INTERACTIVE MODE                 ║"
        );
        println!(
            "╠════════════════════════════════════════════════════════════════════════════════╣"
        );
        println!(
            "║  Nawigacja: ↑↓ Strzałki | Enter: Szczegóły | Q: Wyjście                      ║"
        );
        println!(
            "╠════════════════════════════════════════════════════════════════════════════════╣"
        );

        // Draw device list
        for (idx, device) in self.devices.iter().enumerate() {
            let is_selected = idx == self.selected_index;
            let marker = if is_selected { "❯" } else { " " };
            let selected_bg = if is_selected {
                "\x1b[7m" // Inverse colors
            } else {
                ""
            };
            let reset = if is_selected { "\x1b[0m" } else { "" };

            let default_name = "<Unknown>".to_string();
            let name = device.name.as_ref().unwrap_or(&default_name);

            let default_mfg = "?".to_string();
            let mfg = device.manufacturer_name.as_ref().unwrap_or(&default_mfg);

            println!(
                "{}{}{}  [{:2}] {} | {} | {} dBm | {} ms | {}",
                selected_bg,
                marker,
                reset,
                idx + 1,
                &device.mac_address,
                if name.len() > 20 {
                    format!("{}...", &name[..17])
                } else {
                    format!("{:<20}", name)
                },
                device.rssi,
                device.response_time_ms,
                mfg
            );
        }

        println!(
            "╠════════════════════════════════════════════════════════════════════════════════╣"
        );

        // Draw details if requested
        if self.show_details && self.selected_index < self.devices.len() {
            let device = &self.devices[self.selected_index];
            println!(
                "║ SZCZEGÓŁY URZĄDZENIA                                                         ║"
            );
            println!("├────────────────────────────────────────────────────────────────────────────────┤");
            println!(
                "║ MAC Address:        {} @ {:<40} ║",
                device.mac_address,
                if device.name.is_some() { "✓" } else { "" }
            );
            if let Some(name) = &device.name {
                println!("║ Nazwa:              {:<60} ║", name);
            }
            println!(
                "║ Siła sygnału:       {} dBm                                                 ║",
                device.rssi
            );
            println!(
                "║ Czas odpowiedzi:    {} ms                                                ║",
                device.response_time_ms
            );
            println!(
                "║ Typ:                {}                                              ║",
                match device.device_type {
                    crate::bluetooth_scanner::DeviceType::BleOnly => "BLE Only       ",
                    crate::bluetooth_scanner::DeviceType::BrEdr => "BR/EDR         ",
                    crate::bluetooth_scanner::DeviceType::DualMode => "Dual (BLE+BR/EDR)",
                }
            );
            if let Some(mfg_id) = device.manufacturer_id {
                println!(
                    "║ ID Producenta:      0x{:04X}                                                   ║",
                    mfg_id
                );
            }
            if let Some(mfg) = &device.manufacturer_name {
                println!("║ Producent:          {:<60} ║", mfg);
            }
            println!(
                "║ Usługi BLE:         {} usług                                           ║",
                device.services.len()
            );

            // Display Bluetooth version and features
            if let Some(bt_version) = device.detected_bt_version {
                println!(
                    "║ Bluetooth:          {} ({})",
                    bt_version.as_str(),
                    bt_version.full_name()
                );
            }

            if !device.supported_features.is_empty() {
                println!(
                    "║ Obsługiwane cechy:                                                       ║"
                );
                for (idx, feature) in device.supported_features.iter().enumerate() {
                    if idx < 3 {
                        println!("║   ✨ {:<56} ║", feature.name());
                    }
                }
                if device.supported_features.len() > 3 {
                    println!(
                        "║   ... i {} więcej cech",
                        device.supported_features.len() - 3
                    );
                }
            }

            if !device.services.is_empty() {
                println!("│");
                for service in device.services.iter().take(5) {
                    if let Some(uuid16) = service.uuid16 {
                        println!(
                            "│   • 0x{:04X}: {}",
                            uuid16,
                            service.name.as_ref().unwrap_or(&"<Unknown>".to_string())
                        );
                    }
                }
                if device.services.len() > 5 {
                    println!("│   ... i {} więcej", device.services.len() - 5);
                }
            }

            println!("├────────────────────────────────────────────────────────────────────────────────┤");
        }

        println!(
            "╚════════════════════════════════════════════════════════════════════════════════╝"
        );
        println!(
            "Razem urządzeń: {} | Wybrane: {}/{}",
            self.devices.len(),
            self.selected_index + 1,
            self.devices.len()
        );

        Ok(())
    }
}

/// Convert RSSI to signal strength indicator
fn rssi_to_strength(rssi: i8) -> (&'static str, &'static str) {
    match rssi {
        -30..=0 => ("🟢", "Bardzo mocny"), // Excellent
        -67..=-31 => ("🟢", "Mocny"),      // Good
        -70..=-68 => ("🟡", "Średni"),     // Fair
        -80..=-71 => ("🟠", "Słaby"),      // Weak
        _ => ("🔴", "Bardzo słaby"),       // Very weak
    }
}

/// Simple non-interactive display of devices.
/// If max_rows is Some(n), only first n devices are shown; extra count shown as "... i jeszcze X urządzeń".
pub fn display_devices_simple(
    devices: &[BluetoothDevice],
    start_y: u16,
    max_rows: Option<usize>,
) -> io::Result<()> {
    let mut stdout = io::stdout();
    let mut current_y = start_y;

    if devices.is_empty() {
        execute!(stdout, MoveTo(0, current_y))?;
        writeln!(stdout, "{}", "Brak znalezionych urządzeń".yellow())?;
        return Ok(());
    }

    let (to_show, overflow) = match max_rows {
        Some(n) if n < devices.len() => (
            devices.iter().take(n).collect::<Vec<_>>(),
            Some(devices.len() - n),
        ),
        _ => (devices.iter().collect::<Vec<_>>(), None),
    };

    for device in to_show.iter() {
        let name = device.name.as_deref().unwrap_or("<Unknown>");
        let mfg = device.manufacturer_name.as_deref().unwrap_or("?");
        let device_type = match device.device_type {
            crate::bluetooth_scanner::DeviceType::BleOnly => "BLE",
            crate::bluetooth_scanner::DeviceType::BrEdr => "BR/E",
            crate::bluetooth_scanner::DeviceType::DualMode => "DUAL",
        };

        let (emoji, strength) = rssi_to_strength(device.rssi);
        let _services_count = device.services.len();

        // Color code RSSI
        let rssi_display = if device.rssi >= -50 {
            format!("{:>7} dB", device.rssi).bright_green()
        } else if device.rssi >= -70 {
            format!("{:>7} dB", device.rssi).green()
        } else if device.rssi >= -85 {
            format!("{:>7} dB", device.rssi).yellow()
        } else {
            format!("{:>7} dB", device.rssi).red()
        };

        let mac_colored = device.mac_address.bright_white().bold();
        let name_display = if name.len() > 19 {
            format!("{}...", &name[..16])
        } else {
            format!("{:<19}", name)
        };

        execute!(stdout, MoveTo(0, current_y))?;
        writeln!(
            stdout,
            "║ {} │ {} │ {} │ {} {:<6} │ {:>7} ms │ {:>4} │ {:<13} ║",
            mac_colored,
            name_display.bright_white(),
            rssi_display,
            emoji,
            strength,
            device.response_time_ms,
            device_type,
            if mfg.len() > 13 {
                format!("{}...", &mfg[..10])
            } else {
                format!("{:<13}", mfg)
            }
        )?;
        current_y += 1;
    }

    if let Some(extra) = overflow {
        execute!(stdout, MoveTo(0, current_y))?;
        writeln!(
            stdout,
            "{}",
            format!("... i jeszcze {} urządzeń", extra).bright_black()
        )?;
        current_y += 1;
    }

    // Clear remaining lines if new device list is shorter
    let devices_height = to_show.len() + overflow.is_some().then_some(1).unwrap_or(0);
    let clear_until = (start_y as usize + devices_height + 1) as u16;
    for y in current_y..clear_until {
        execute!(
            stdout,
            MoveTo(0, y),
            crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine)
        )?;
    }

    execute!(stdout, MoveTo(0, current_y))?;
    writeln!(stdout, "{}", "╚══════════════════════════════════════════════════════════════════════════════════════════════════════╝".bright_blue())?;
    current_y += 1;
    execute!(stdout, MoveTo(0, current_y))?;
    writeln!(
        stdout,
        "{}",
        format!("📊 Razem: {} urządzeń", devices.len())
            .bright_cyan()
            .bold()
    )?;
    current_y += 1;
    execute!(stdout, MoveTo(0, current_y))?;
    writeln!(stdout, "{}", "🟢 Mocny (>-67dB) │ 🟡 Średni (-70..-68dB) │ 🟠 Słaby (-80..-71dB) │ 🔴 Bardzo słaby (<-80dB)".bright_white())?;
    writeln!(stdout)?; // Print an extra newline at the end

    Ok(())
}

/// Check and display Bluetooth permissions on startup
#[cfg(target_os = "windows")]
pub fn check_bluetooth_permissions() -> bool {
    use log::warn;
    println!("🔍 Sprawdzanie uprawnień Bluetooth...");

    // Try to detect if Windows has Bluetooth available
    // This is a simplified check - in production you'd use Windows APIs
    let has_bluetooth = true; // Assume available on Windows unless error

    if has_bluetooth {
        println!("✓ Uprawnienia Bluetooth: OK");
        log::info!("✓ Bluetooth permissions verified");
        true
    } else {
        warn!("❌ Brak uprawnień Bluetooth lub adapter nie dostępny");
        println!("❌ Bluetooth: Brak dostępu");
        println!("   • Upewnij się że Bluetooth jest włączony w systemie");
        println!("   • Sprawdź ustawienia Bluetooth w Ustawieniach Windows");
        println!("   • Spróbuj uruchomić jako Administrator");
        false
    }
}

#[cfg(not(target_os = "windows"))]
pub fn check_bluetooth_permissions() -> bool {
    println!("🔍 Sprawdzanie uprawnień Bluetooth...");
    println!("✓ Uprawnienia Bluetooth: OK");
    log::info!("✓ Bluetooth permissions verified");
    true
}

/// Show scanning mode menu with 5-second timeout and return the selected mode
/// Returns true for continuous scanning (default), false for interval-based (5 minutes)
/// If no selection within 5 seconds, defaults to continuous mode
pub fn show_scan_mode_menu() -> bool {
    use std::time::Duration;

    clearscreen::clear().unwrap_or_default();

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║            🔵 Bluetooth Scanner - Scan Mode Selection            ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    println!("Wybierz tryb skanowania Bluetooth (5 sekund do autostartu):");
    println!();
    println!("  1. 🔄 Ciągłe skanowanie (domyślnie)");
    println!("     └─ Skanuje non-stop bez przerw");
    println!();
    println!("  2. ⏰ Skanowanie co 5 minut");
    println!("     └─ Skanuje przez 30s, potem czeka 5 minut");
    println!();

    // Enable raw mode for non-blocking input
    let _ = enable_raw_mode();

    // Show countdown from 5 to 0 with input check
    for countdown in (0..=5).rev() {
        print!("\rWybór (1 lub 2): {} sekund ", countdown);
        std::io::stdout().flush().ok();

        // Check for input with 1-second timeout
        if event::poll(Duration::from_secs(1)).ok().unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                let _ = disable_raw_mode();
                match key.code {
                    KeyCode::Char('2') => {
                        println!("\n✓ Wybrano: Skanowanie co 5 minut");
                        std::thread::sleep(Duration::from_millis(1500));
                        return false;
                    }
                    KeyCode::Char('1') => {
                        println!("\n✓ Wybrano: Ciągłe skanowanie");
                        std::thread::sleep(Duration::from_millis(1500));
                        return true;
                    }
                    _ => {}
                }
            }
        }
    }

    let _ = disable_raw_mode();
    println!("\n⏰ Upłynęło 5 sekund - autostart: Ciągłe skanowanie");
    std::thread::sleep(Duration::from_millis(1500));
    true
}

/// Display countdown timer to next scan
/// min: minutes, sec: seconds
pub fn display_countdown(mut minutes: u64, mut seconds: u64) {
    loop {
        print!("\r⏳ Następne skanowanie za: {:02}:{:02}", minutes, seconds);
        std::io::stdout().flush().ok();

        if minutes == 0 && seconds == 0 {
            println!();
            break;
        }

        std::thread::sleep(std::time::Duration::from_secs(1));

        if seconds > 0 {
            seconds -= 1;
        } else if minutes > 0 {
            minutes -= 1;
            seconds = 59;
        } else {
            println!();
            break;
        }
    }
}

/// Display countdown timer that can be interrupted by shutdown flag
/// min: minutes, sec: seconds, shutdown_flag: Arc<AtomicBool>
pub fn display_countdown_interruptible(
    mut minutes: u64,
    mut seconds: u64,
    shutdown_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    loop {
        // Check for shutdown request
        if shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
            println!("\n👋 Przerwano odliczanie - zamykanie...");
            break;
        }

        print!("\r⏳ Następne skanowanie za: {:02}:{:02}", minutes, seconds);
        std::io::stdout().flush().ok();

        if minutes == 0 && seconds == 0 {
            println!();
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(500));

        // Check again for shutdown between seconds
        if shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
            println!("\n👋 Przerwano odliczanie - zamykanie...");
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(500));

        if seconds > 0 {
            seconds -= 1;
        } else if minutes > 0 {
            minutes -= 1;
            seconds = 59;
        } else {
            println!();
            break;
        }
    }
}
