use chrono::{DateTime, Utc};
use dotenv::dotenv;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::env;

const PERIODIC_REPORT_INTERVAL_SECS: u64 = 300;
const DEVICES_HISTORY_WINDOW_SECS: i64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
    pub enabled: bool,
}

pub fn get_config() -> TelegramConfig {
    dotenv().ok();
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    let chat_id = env::var("TELEGRAM_CHAT_ID").unwrap_or_default();
    let enabled = !bot_token.is_empty() && !chat_id.is_empty();

    if enabled {
        log::info!("[+] Telegram notifications loaded from .env");
    } else {
        log::warn!("[!] Telegram notifications not configured");
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
    let conn = rusqlite::Connection::open("bluetooth_scan.db").map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS telegram_reports (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            last_report_time DATETIME,
            report_count INTEGER DEFAULT 0,
            scan_session_number INTEGER DEFAULT 0
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR IGNORE INTO telegram_reports (id, last_report_time, report_count, scan_session_number)
         VALUES (1, datetime(''now'', ''-6 minutes''), 0, 0)", [],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE telegram_reports SET scan_session_number = scan_session_number + 1 WHERE id = 1",
        [],
    )
    .map_err(|e| e.to_string())?;

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
    let session_number = get_scan_session_number().unwrap_or(1);
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
        std::env::var("HOSTNAME").unwrap_or_else(|_| {
            hostname::get()
                .ok()
                .and_then(|s| s.into_string().ok())
                .unwrap_or_else(|| "Unknown".to_string())
        })
    }
}

fn get_scan_session_number() -> Result<u32, String> {
    let conn = rusqlite::Connection::open("bluetooth_scan.db").map_err(|e| e.to_string())?;
    let session_number: u32 = conn
        .query_row(
            "SELECT scan_session_number FROM telegram_reports WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(session_number)
}

fn format_startup_message(
    hostname: &str,
    adapter_mac: &str,
    adapter_name: &str,
    session_number: u32,
) -> String {
    let mut message = String::new();
    message.push_str(&format!(
        "[*] <b>Wlacono skanowanie na {}</b>\n\n",
        hostname
    ));
    message.push_str(&format!("[#] <b>Sesja:</b> #{}\n", session_number));
    message.push_str(&format!("[*] <b>Adapter:</b> {}\n", adapter_name));
    message.push_str(&format!("[*] <b>MAC:</b> <code>{}</code>\n", adapter_mac));
    message.push_str(&format!(
        "[*] <b>Czas:</b> {}\n",
        chrono::Local::now().format("%H:%M:%S")
    ));
    message.push_str("\n[+] Skanowanie w toku...\n");
    message
}

#[derive(Debug, Clone)]
pub struct RawPacketInfo {
    pub timestamp: String,
    pub rssi: i8,
    pub advertising_data: String,
    pub phy: String,
    pub channel: i32,
    pub frame_type: String,
}

#[derive(Debug, Clone)]
pub struct DeviceReport {
    pub mac_address: String,
    pub device_name: Option<String>,
    pub current_rssi: i8,
    pub avg_rssi: i8,
    pub manufacturer_name: Option<String>,
    pub manufacturer_id: Option<i32>,
    pub is_connectable: bool,
    pub services_count: usize,
    pub services: Vec<String>,
    pub first_seen: String,
    pub last_seen: String,
    pub packet_count: i32,
    pub raw_packets: Vec<RawPacketInfo>,
}

fn format_devices_report(devices: &[DeviceReport], duration_minutes: i64) -> String {
    let mut message = String::new();

    message.push_str(&format!("<b>RAPORT URZADZEN BLE</b>\n"));
    message.push_str(&format!(
        "Urzadzenia z ostatnich {} minut\n\n",
        duration_minutes
    ));

    if devices.is_empty() {
        message.push_str("Nie wykryto urzadzen\n");
        return message;
    }

    message.push_str(&format!(
        "[+] Znaleziono <b>{}</b> urzadzenie(n):\n\n",
        devices.len()
    ));

    for (idx, device) in devices.iter().enumerate() {
        let name = device.device_name.as_deref().unwrap_or("Unknown");
        let manufacturer = device.manufacturer_name.as_deref().unwrap_or("Unknown");

        message.push_str(&format!(
            "<b>{}. {}</b> ({})\n",
            idx + 1,
            name,
            manufacturer
        ));
        message.push_str(&format!("   MAC: <code>{}</code>", device.mac_address));
        if let Some(mfg_id) = device.manufacturer_id {
            message.push_str(&format!(" | ID: {}", mfg_id));
        }
        message.push_str("\n");

        message.push_str(&format!(
            "   RSSI: {} dBm | Sredni: {} dBm\n",
            device.current_rssi, device.avg_rssi
        ));
        message.push_str(&format!(
            "   Pierwsze: {} | Ostatnie: {}\n",
            device.first_seen, device.last_seen
        ));

        if device.is_connectable {
            message.push_str("   [Connectable]\n");
        }

        if device.packet_count > 0 {
            message.push_str(&format!(
                "   Pakietow: {} | Serwisy: {}",
                device.packet_count, device.services_count
            ));
            if !device.services.is_empty() {
                message.push_str(&format!(
                    " | {:?}",
                    &device.services.iter().take(3).collect::<Vec<_>>()
                ));
            }
            message.push_str("\n");
        }

        if !device.raw_packets.is_empty() {
            message.push_str("   Ostatnie pakiety:\n");
            for (pidx, pkt) in device.raw_packets.iter().take(3).enumerate() {
                let data_short = if pkt.advertising_data.len() > 30 {
                    format!("{}...", &pkt.advertising_data[..30])
                } else {
                    pkt.advertising_data.clone()
                };
                message.push_str(&format!(
                    "      {}. [{}] {}dBm | {} | {}\n",
                    pidx + 1,
                    pkt.timestamp,
                    pkt.rssi,
                    pkt.phy,
                    data_short
                ));
            }
        }

        message.push_str("\n");
    }

    message.push_str("----------------------\n");
    message.push_str(&format!(
        "Raport: {}\n",
        chrono::Local::now().format("%H:%M:%S")
    ));

    message
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
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
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

fn should_send_report(conn: &rusqlite::Connection) -> Result<bool, rusqlite::Error> {
    let last_report: String = conn
        .query_row(
            "SELECT last_report_time FROM telegram_reports WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| chrono::Local::now().to_rfc3339());

    let last_report_time = DateTime::parse_from_rfc3339(&last_report)
        .unwrap_or_else(|_| {
            chrono::Local::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
        })
        .with_timezone(&Utc);

    let now = Utc::now();
    let duration = now.signed_duration_since(last_report_time);

    Ok(duration.num_seconds() >= PERIODIC_REPORT_INTERVAL_SECS as i64)
}

fn get_raw_packets_for_device(
    conn: &rusqlite::Connection,
    mac_address: &str,
    minutes: i64,
) -> Result<Vec<RawPacketInfo>, Box<dyn std::error::Error>> {
    let time_filter = format!("-{} minutes", minutes);

    let mut stmt = conn.prepare(
        "SELECT timestamp, rssi, advertising_data, phy, channel, frame_type
         FROM ble_advertisement_frames
         WHERE mac_address = ? AND timestamp > datetime(''now'', ?)
         ORDER BY timestamp DESC
         LIMIT 10",
    )?;

    let packets = stmt
        .query_map(params![mac_address, time_filter], |row| {
            let timestamp: String = row.get(0)?;
            let timestamp_formatted = parse_and_format_time(&timestamp);

            Ok(RawPacketInfo {
                timestamp: timestamp_formatted,
                rssi: row.get(1)?,
                advertising_data: row.get(2)?,
                phy: row.get(3)?,
                channel: row.get(4)?,
                frame_type: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(packets)
}

fn get_services_for_device(
    conn: &rusqlite::Connection,
    device_id: i64,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare("SELECT service_name FROM ble_services WHERE device_id = ?")?;

    let services = stmt
        .query_map([device_id], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(services)
}

fn get_devices_from_last_minutes(
    conn: &rusqlite::Connection,
    minutes: i64,
) -> Result<Vec<DeviceReport>, Box<dyn std::error::Error>> {
    let time_filter = format!("-{} minutes", minutes);

    let mut stmt = conn.prepare(
        "SELECT 
            d.id,
            d.mac_address, 
            d.device_name, 
            d.rssi,
            COALESCE(AVG(sh.rssi), d.rssi) as avg_rssi,
            d.manufacturer_name,
            d.manufacturer_id,
            d.mac_type,
            d.first_seen,
            d.last_seen,
            (SELECT COUNT(*) FROM ble_services WHERE device_id = d.id) as services_count,
            (SELECT COUNT(*) FROM ble_advertisement_frames WHERE mac_address = d.mac_address AND timestamp > datetime(''now'', ?)) as packet_count
        FROM devices d
        LEFT JOIN scan_history sh ON d.id = sh.device_id AND sh.scan_timestamp > datetime(''now'', ?)
        WHERE d.last_seen > datetime(''now'', ?)
        GROUP BY d.id
        ORDER BY d.last_seen DESC, d.rssi DESC"
    )?;

    let devices = stmt
        .query_map(params![time_filter, time_filter, time_filter], |row| {
            let device_id: i64 = row.get(0)?;
            let first_seen: String = row.get(8)?;
            let last_seen: String = row.get(9)?;
            let mac_type: Option<String> = row.get(7)?;

            let is_connectable = mac_type
                .as_deref()
                .map(|t| t.to_lowercase().contains("public") || t.to_lowercase().contains("random"))
                .unwrap_or(false);

            Ok(DeviceReport {
                mac_address: row.get(1)?,
                device_name: row.get(2)?,
                current_rssi: row.get(3)?,
                avg_rssi: row.get::<_, f64>(4)? as i8,
                manufacturer_name: row.get(5)?,
                manufacturer_id: row.get(6)?,
                is_connectable,
                services_count: row.get::<_, i32>(10)? as usize,
                services: vec![],
                first_seen: parse_and_format_time(&first_seen),
                last_seen: parse_and_format_time(&last_seen),
                packet_count: row.get::<_, i32>(11)?,
                raw_packets: vec![],
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut enriched_devices = Vec::new();
    for mut device in devices {
        if let Ok(services) = get_services_for_device(conn, {
            let mut stmt = conn.prepare("SELECT id FROM devices WHERE mac_address = ?")?;
            stmt.query_row([&device.mac_address], |row| row.get::<_, i64>(0))
                .unwrap_or(0)
        }) {
            device.services = services;
        }

        if let Ok(packets) = get_raw_packets_for_device(conn, &device.mac_address, minutes) {
            device.raw_packets = packets;
        }

        enriched_devices.push(device);
    }

    Ok(enriched_devices)
}

fn parse_and_format_time(timestamp: &str) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        dt.with_timezone(&chrono::Local)
            .format("%H:%M:%S")
            .to_string()
    } else {
        timestamp.split(' ').nth(1).unwrap_or(timestamp).to_string()
    }
}

fn update_last_report_time(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute("UPDATE telegram_reports SET last_report_time = datetime(''now''), report_count = report_count + 1 WHERE id = 1", [])?;
    Ok(())
}

pub async fn run_periodic_report_task() -> Result<(), String> {
    if !is_enabled() {
        return Ok(());
    }

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(
            PERIODIC_REPORT_INTERVAL_SECS,
        ))
        .await;

        if let Err(e) = send_periodic_report().await {
            log::warn!("Failed to send periodic Telegram report: {}", e);
        }
    }
}

async fn send_periodic_report() -> Result<(), String> {
    let conn = rusqlite::Connection::open("bluetooth_scan.db").map_err(|e| e.to_string())?;

    match should_send_report(&conn) {
        Ok(true) => {}
        Ok(false) => return Ok(()),
        Err(_) => return Ok(()),
    }

    let devices = get_devices_from_last_minutes(&conn, DEVICES_HISTORY_WINDOW_SECS / 60)
        .map_err(|e| e.to_string())?;

    send_devices_report(&devices).await?;

    update_last_report_time(&conn).map_err(|e| e.to_string())?;

    log::info!("[+] Sent Telegram report with {} device(s)", devices.len());

    Ok(())
}
