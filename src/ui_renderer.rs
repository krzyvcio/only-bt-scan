use colored::Colorize;
use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{size, Clear, ClearType},
};
use std::env;
use std::io::{stdout, Write};

pub fn draw_static_header() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = stdout();

    // Clear the screen
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;

    let web_port = env::var("WEB_SERVER_PORT").unwrap_or_else(|_| "8080".to_string());

    // Simple header
    writeln!(
        stdout,
        "{}",
        "╔════════════════════════════════════════════════════════════════════╗".bright_cyan()
    )?;
    writeln!(
        stdout,
        "{}",
        "║              🔵 BLUETOOTH SCANNER - DEVICES FOUND                 ║"
            .bright_cyan()
            .bold()
    )?;
    writeln!(
        stdout,
        "{}",
        "╚════════════════════════════════════════════════════════════════════╝".bright_cyan()
    )?;
    writeln!(stdout)?;
    writeln!(
        stdout,
        "🌐 Web Panel: http://localhost:{} | Press Ctrl+C to quit",
        web_port.bright_green()
    )?;
    writeln!(
        stdout,
        "{}",
        "──────────────────────────────────────────────────────────────────────────".blue()
    )?;

    Ok(())
}

pub fn get_device_list_start_line() -> u16 {
    // Calculate the line number where the device list should start
    // This needs to be precisely adjusted based on the `draw_static_header` output
    // Count the lines printed by `draw_static_header`
    // Header section (5 lines + 1 blank) = 6
    // Startup info (4 lines + 1 blank) = 5
    // Diagnostic section (3 lines) = 3
    // Scanner config (6 lines) = 6
    // Success message + separator (3 lines + 1 blank) = 4
    // Instructions (3 lines + 1 blank) = 4
    // Total = 6 + 5 + 3 + 6 + 4 + 4 = 28 lines
    28
}

/// Clears only the content area (from device list start to end of screen).
/// Use this instead of full redraw so the header stays fixed and there's no scroll.
pub fn clear_content_area() -> Result<(), anyhow::Error> {
    log::debug!("Clearing content area starting from line {}", get_device_list_start_line());
    let mut stdout = stdout();
    let start = get_device_list_start_line();
    execute!(stdout, MoveTo(0, start), Clear(ClearType::FromCursorDown))
        .map_err(|e| {
            log::error!("Failed to clear content area: {}", e);
            anyhow::anyhow!("Terminal clear error: {}", e)
        })?;
    log::debug!("Content area cleared successfully");
    Ok(())
}

/// Terminal height (rows). Falls back to 24 if size unavailable.
pub fn get_terminal_height() -> u16 {
    size().map(|(_w, h)| h).unwrap_or(24)
}

/// Max device rows that fit below the content header without scrolling.
/// Fixed: status(3) + table_header(3) + closing border + "Razem" + legend ≈ 8.
pub fn max_device_rows_for_display() -> usize {
    let start = get_device_list_start_line();
    let height = get_terminal_height();
    let fixed_lines = 8u16;
    height.saturating_sub(start).saturating_sub(fixed_lines) as usize
}

// Function to print the device table header
pub fn draw_device_table_header() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = stdout();
    writeln!(stdout, "{}", "╔═════════════════════╤═══════════════════════════╤═══════════╤═══════════╤═══════════╤══════╤═══════════╗".bright_blue())?;
    writeln!(stdout, "{}", "║ MAC Address         │ Nazwa                     │ RSSI (dB) │ Sygnał    │ Odpowiedź │ Typ  │ Producent ║".bright_blue())?;
    writeln!(stdout, "{}", "╠═════════════════════╪═══════════════════════════╪═══════════╪═══════════╪═══════════╪══════╪═══════════╣".bright_blue())?;
    Ok(())
}
