use chrono::Local;
use colored::Colorize;

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::Mutex;

static LOG_FILE: Mutex<Option<BufWriter<File>>> = Mutex::new(None);

pub fn init_logger(log_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(log_path);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = OpenOptions::new().create(true).append(true).open(&path)?;

    let writer = BufWriter::new(file);

    let mut guard = LOG_FILE.lock().unwrap();
    *guard = Some(writer);

    // Don't call log::info! here - would create infinite loop
    Ok(())
}

pub fn log_to_file(level: &str, message: &str) {
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut writer) = *guard {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let colored_msg = format!("[{}] [{}] {}\n", timestamp, level, message);
            let _ = writer.write_all(colored_msg.as_bytes());
            let _ = writer.flush();
        }
    }
}

pub fn log_error(message: &str) {
    let colored = format!("{}", message.red());
    log_to_file("ERROR", &colored);
    // Don't call log::error! - only write to file
}

pub fn log_warn(message: &str) {
    let colored = format!("{}", message.yellow());
    log_to_file("WARN", &colored);
    // Don't call log::warn! - only write to file
}

pub fn log_warn_with_context(context: &str, message: &str) {
    let full_msg = format!("[{}] {}", context, message);
    log_warn(&full_msg);
}

pub fn log_info(message: &str) {
    let colored = format!("{}", message.green());
    log_to_file("INFO", &colored);
    // Don't call log::info! - only write to file
}

pub fn log_debug(message: &str) {
    let colored = format!("{}", message.cyan());
    log_to_file("DEBUG", &colored);
    // Don't call log::debug! - only write to file
}

pub fn log_panic(message: &str) {
    let colored = format!("{}", message.red().bold());
    log_to_file("PANIC", &colored);
    eprintln!("{}", colored);
}

pub fn log_with_context(context: &str, message: &str) {
    let full_msg = format!("[{}] {}", context, message);
    log_info(&full_msg);
}

pub fn log_error_with_context(context: &str, message: &str) {
    let full_msg = format!("[{}] {}", context, message);
    log_error(&full_msg);
}

pub fn log_result<T, E: std::fmt::Display>(
    context: &str,
    result: Result<T, E>,
) -> Result<T, String> {
    match result {
        Ok(value) => {
            log_with_context(context, "OK");
            Ok(value)
        }
        Err(e) => {
            log_error_with_context(context, &format!("{}", e));
            Err(format!("{}", e))
        }
    }
}

pub fn get_log_path() -> String {
    "logs/bluetooth_scanner.log".to_string()
}

pub fn log_critical(message: &str) {
    let colored = format!("{}", message.red().bold());
    log_to_file("CRITICAL", &colored);
    eprintln!("{}", colored);
}

pub fn log_success(message: &str) {
    let colored = format!("{}", message.green().bold());
    log_to_file("SUCCESS", &colored);
    // Don't call log::info! - only write to file
}

pub fn log_operation_start(operation: &str) {
    let msg = format!("Starting operation: {}", operation);
    log_info(&msg);
}

pub fn log_operation_end(operation: &str, success: bool) {
    if success {
        let msg = format!("Operation completed successfully: {}", operation);
        log_success(&msg);
    } else {
        let msg = format!("Operation failed: {}", operation);
        log_error(&msg);
    }
}

pub fn log_database_operation(operation: &str, table: &str, result: Result<usize, String>) {
    match result {
        Ok(count) => {
            let msg = format!(
                "Database {}: {} rows affected in table '{}'",
                operation, count, table
            );
            log_info(&msg);
        }
        Err(e) => {
            let msg = format!("Database {} failed on table '{}': {}", operation, table, e);
            log_error(&msg);
        }
    }
}

pub fn log_scan_metrics(devices_found: usize, packets_captured: usize, scan_duration_ms: u64) {
    let msg = format!(
        "Scan metrics: {} devices, {} packets, {}ms duration",
        devices_found, packets_captured, scan_duration_ms
    );
    log_info(&msg);
}

pub fn log_bluetooth_event(event_type: &str, device_mac: &str, details: &str) {
    let msg = format!(
        "BT Event [{}] Device {}: {}",
        event_type, device_mac, details
    );
    log_info(&msg);
}

pub fn log_system_resource(resource_type: &str, usage: &str) {
    let msg = format!("System resource [{}]: {}", resource_type, usage);
    log_debug(&msg);
}
