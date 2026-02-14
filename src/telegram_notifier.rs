use serde::{Deserialize, Serialize};
use std::env;
use rusqlite::params;
use chrono::{DateTime, Utc};
use dotenv::dotenv;

const PERIODIC_REPORT_INTERVAL_SECS: u64 = 300; // 5 minutes
const DEVICES_HISTORY_WINDOW_SECS: i64 = 300;   // 5 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
    pub enabled: bool,
}

pub fn get_config() -> TelegramConfig {
    // Load .env file (safe to call multiple times)
    dotenv().ok();
    
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    let chat_id = env::var("TELEGRAM_CHAT_ID").unwrap_or_default();
    
    let enabled = !bot_token.is_empty() && !chat_id.is_empty();
    
    if enabled {
        log::info!("✅ Telegram notifications loaded from .env");
    } else {
        log::warn!("⚠️  Telegram notifications not configured - set TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID in .env");
    }
    
    TelegramConfig {
        bot_token,
        chat_id,
        enabled,
    }
}

pub fn is_enabled() -> bool {
    get_config().enabled
}

pub fn init_telegram_notifications() -> Result<(), String> {
    let conn = rusqlite::Connection::open("bluetooth_scan.db")
        .map_err(|e| e.to_string())?;
    
    // Table for tracking periodic reports (single row with last report time)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS telegram_reports (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            last_report_time DATETIME,
            report_count INTEGER DEFAULT 0,
            scan_session_number INTEGER DEFAULT 0
        )",
        [],
    ).map_err(|e| e.to_string())?;
    
    // Initialize if empty
    conn.execute(
        "INSERT OR IGNORE INTO telegram_reports (id, last_report_time, report_count, scan_session_number)
         VALUES (1, datetime('now', '-6 minutes'), 0, 0)",
        [],
    ).map_err(|e| e.to_string())?;
    
    // Increment scan session number
    conn.execute(
        "UPDATE telegram_reports SET scan_session_number = scan_session_number + 1 WHERE id = 1",
        [],
    ).map_err(|e| e.to_string())?;
    
    Ok(())
}

pub async fn send_startup_notification(
    adapter_mac: &str,
    adapter_name: &str,
) -> Result<(), String> {
    let config = get_config();
    
    if !config.enabled {
        return Ok(());
    }
    
    let hostname = get_hostname();
    
    // Get scan session number from database
    let session_number = get_scan_session_number()
        .unwrap_or(1);
    
    let message = format_startup_message(&hostname, adapter_mac, adapter_name, session_number);
    
    send_telegram_message(&config.bot_token, &config.chat_id, &message).await
}

fn get_hostname() -> String {
    #[cfg(target_os = "windows")]
    {
        std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "Unknown".to_string())
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOSTNAME")
            .unwrap_or_else(|_| {
                hostname::get()
                    .ok()
                    .and_then(|s| s.into_string().ok())
                    .unwrap_or_else(|| "Unknown".to_string())
            })
    }
}

fn get_scan_session_number() -> Result<u32, String> {
    let conn = rusqlite::Connection::open("bluetooth_scan.db")
        .map_err(|e| e.to_string())?;
    
    let session_number: u32 = conn
        .query_row(
            "SELECT scan_session_number FROM telegram_reports WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    
    Ok(session_number)
}

fn format_startup_message(hostname: &str, adapter_mac: &str, adapter_name: &str, session_number: u32) -> String {
    let mut message = String::new();
    
    // Main heading in Polish
    message.push_str(&format!("🚀 <b>włączono skanowanie na {}</b>\n\n", hostname));
    
    message.push_str(&format!("📍 <b>Sesja skanowania:</b> #{}\n", session_number));
    message.push_str(&format!("📱 <b>Adapter:</b> {}\n", adapter_name));
    message.push_str(&format!("🔗 <b>MAC:</b> <code>{}</code>\n", adapter_mac));
    message.push_str(&format!("🕐 <b>Czas startu:</b> {}\n", chrono::Local::now().format("%H:%M:%S")));
    message.push_str("\n✅ Skanowanie w toku...\n");
    
    message
}

fn format_devices_report(devices: &[DeviceReport], duration_minutes: i64) -> String {
    let mut message = String::new();
    
    message.push_str(&format!("📊 <b>RAPORT URZĄDZEŃ BLE</b>\n"));
    message.push_str(&format!("🕐 Urządzenia z ostatnich {} minut\n\n", duration_minutes));
    
    if devices.is_empty() {
        message.push_str("❌ Nie wykryto urządzeń\n");
        return message;
    }
    
    message.push_str(&format!("✅ Znaleziono <b>{}</b> urządzenie(ń):\n\n", devices.len()));
    
    for (idx, device) in devices.iter().enumerate() {
        let name = device.device_name.as_deref().unwrap_or("Unknown");
        let manufacturer = device.manufacturer_name.as_deref().unwrap_or("Unknown");
        
        message.push_str(&format!("<b>{}. {}</b> ({})\n", idx + 1, name, manufacturer));
        message.push_str(&format!("   📱 MAC: <code>{}</code>\n", device.mac_address));
        message.push_str(&format!("   📶 RSSI: {} dBm | ", device.current_rssi));
        message.push_str(&format!("Średni: {} dBm\n", device.avg_rssi));
        message.push_str(&format!("   🆕 Pierwsze wykrycie: {}\n", device.first_seen));
        message.push_str(&format!("   🕐 Ostatnie: {}\n", device.last_seen));
        
        if device.services_count > 0 {
            message.push_str(&format!("   🔌 Serwisy: {}\n", device.services_count));
        }
        
        message.push_str("\n");
    }
    
    message.push_str("━━━━━━━━━━━━━━━━━━━━━\n");
    message.push_str(&format!("⏰ Raport wygenerowany: {}\n", chrono::Local::now().format("%H:%M:%S")));
    
    message
}

#[derive(Debug, Clone)]
pub struct DeviceReport {
    pub mac_address: String,
    pub device_name: Option<String>,
    pub current_rssi: i8,
    pub avg_rssi: i8,
    pub manufacturer_name: Option<String>,
    pub is_connectable: bool,
    pub services_count: usize,
    pub first_seen: String,
    pub last_seen: String,
}

pub async fn send_devices_report(devices: &[DeviceReport]) -> Result<(), String> {
    let config = get_config();
    
    if !config.enabled {
        return Ok(());
    }
    
    let message = format_devices_report(devices, DEVICES_HISTORY_WINDOW_SECS / 60);
    send_telegram_message(&config.bot_token, &config.chat_id, &message).await
}

async fn send_telegram_message(token: &str, chat_id: &str, message: &str) -> Result<(), String> {
    let url = format!(
        "https://api.telegram.org/bot{}/sendMessage",
        token
    );
    
    let client = reqwest::Client::new();
    
    let params = serde_json::json!({
        "chat_id": chat_id,
        "text": message,
        "parse_mode": "HTML"
    });
    
    let response = client
        .post(&url)
        .json(&params)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Telegram API error: {} - {}", status, body))
    }
}

/// Check if periodic report should be sent (every 5 minutes)
fn should_send_report(conn: &rusqlite::Connection) -> Result<bool, rusqlite::Error> {
    let last_report: String = conn
        .query_row(
            "SELECT last_report_time FROM telegram_reports WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| chrono::Local::now().to_rfc3339());
    
    let last_report_time = DateTime::parse_from_rfc3339(&last_report)
        .unwrap_or_else(|_| chrono::Local::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()))
        .with_timezone(&Utc);
    
    let now = Utc::now();
    let duration = now.signed_duration_since(last_report_time);
    
    Ok(duration.num_seconds() >= PERIODIC_REPORT_INTERVAL_SECS as i64)
}

/// Fetch devices visible in last N minutes
fn get_devices_from_last_minutes(
    conn: &rusqlite::Connection,
    minutes: i64,
) -> Result<Vec<DeviceReport>, Box<dyn std::error::Error>> {
    let time_filter = format!("-{} minutes", minutes);
    
    let mut stmt = conn.prepare(
        "SELECT 
            mac_address, 
            device_name, 
            rssi,
            rssi as avg_rssi,
            manufacturer_name,
            first_seen,
            last_seen,
            (SELECT COUNT(*) FROM ble_services WHERE device_id = devices.id) as services_count
        FROM devices 
        WHERE last_seen > datetime('now', ?)
        ORDER BY first_seen DESC, rssi DESC"
    )?;
    
    let devices = stmt.query_map(params![time_filter], |row| {
        // Parse timestamps and format them nicely
        let first_seen: String = row.get(5)?;
        let last_seen: String = row.get(6)?;
        
        // Format timestamps to HH:MM:SS
        let first_seen_formatted = parse_and_format_time(&first_seen);
        let last_seen_formatted = parse_and_format_time(&last_seen);
        
        Ok(DeviceReport {
            mac_address: row.get(0)?,
            device_name: row.get(1)?,
            current_rssi: row.get(2)?,
            avg_rssi: row.get(3)?,
            manufacturer_name: row.get(4)?,
            is_connectable: false,
            services_count: row.get::<_, i32>(7).unwrap_or(0) as usize,
            first_seen: first_seen_formatted,
            last_seen: last_seen_formatted,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;
    
    Ok(devices)
}

/// Parse ISO 8601 timestamp and format to HH:MM:SS
fn parse_and_format_time(timestamp: &str) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        dt.with_timezone(&chrono::Local).format("%H:%M:%S").to_string()
    } else {
        // Fallback: try SQLite format (YYYY-MM-DD HH:MM:SS)
        timestamp.split(' ').nth(1).unwrap_or(timestamp).to_string()
    }
}

/// Update last report timestamp
fn update_last_report_time(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE telegram_reports 
         SET last_report_time = datetime('now'),
             report_count = report_count + 1
         WHERE id = 1",
        [],
    )?;
    Ok(())
}

/// Periodic telegram report task (runs every 5 minutes)
pub async fn run_periodic_report_task() -> Result<(), String> {
    if !is_enabled() {
        return Ok(());
    }
    
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(PERIODIC_REPORT_INTERVAL_SECS)).await;
        
        if let Err(e) = send_periodic_report().await {
            log::warn!("Failed to send periodic Telegram report: {}", e);
        }
    }
}

/// Send periodic report of devices visible in last 5 minutes
async fn send_periodic_report() -> Result<(), String> {
    let conn = rusqlite::Connection::open("bluetooth_scan.db")
        .map_err(|e| e.to_string())?;
    
    // Check if enough time has passed
    match should_send_report(&conn) {
        Ok(true) => {},
        Ok(false) => return Ok(()), // Too soon
        Err(_) => return Ok(()), // DB error, skip this cycle
    }
    
    // Fetch devices from last 5 minutes
    let devices = get_devices_from_last_minutes(&conn, DEVICES_HISTORY_WINDOW_SECS / 60)
        .map_err(|e| e.to_string())?;
    
    // Send report
    send_devices_report(&devices).await?;
    
    // Update timestamp
    update_last_report_time(&conn)
        .map_err(|e| e.to_string())?;
    
    log::info!("✅ Sent Telegram report with {} device(s)", devices.len());
    
    Ok(())
}
