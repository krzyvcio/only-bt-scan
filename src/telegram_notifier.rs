use chrono::{DateTime, Utc};
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
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    let chat_id = env::var("TELEGRAM_CHAT_ID").unwrap_or_default();
    
    eprintln!("[TELEGRAM] bot_token='{}', chat_id='{}'", 
        if bot_token.is_empty() { "EMPTY" } else { "SET" }, 
        if chat_id.is_empty() { "EMPTY" } else { "SET" }
    );
    
    let enabled = !bot_token.is_empty() && !chat_id.is_empty();

    if enabled {
        log::info!("[+] Telegram notifications enabled");
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
         VALUES (1, datetime('now', '-6 minutes'), 0, 0)",
        [],
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

pub async fn send_initial_device_report() -> Result<(), String> {
    let config = get_config();
    if !config.enabled {
        return Ok(());
    }

    let conn = rusqlite::Connection::open("bluetooth_scan.db").map_err(|e| e.to_string())?;
    
    let devices = get_all_current_devices(&conn).map_err(|e| e.to_string())?;
    
    if devices.is_empty() {
        let msg = "<i>Brak wykrytych urzadzen</i>";
        send_telegram_message(&config.bot_token, &config.chat_id, msg).await
    } else {
        let message = format_initial_devices_message(&devices);
        send_telegram_message(&config.bot_token, &config.chat_id, &message).await
    }
}

fn get_all_current_devices(conn: &rusqlite::Connection) -> Result<Vec<DeviceReport>, Box<dyn std::error::Error>> {
    let time_filter = "-10 minutes";
    
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
            (SELECT COUNT(*) FROM ble_advertisement_frames WHERE mac_address = d.mac_address AND timestamp > datetime('now', ?)) as packet_count
        FROM devices d
        LEFT JOIN scan_history sh ON d.id = sh.device_id AND sh.scan_timestamp > datetime('now', ?)
        WHERE d.last_seen > datetime('now', ?)
        GROUP BY d.id
        ORDER BY d.last_seen DESC, d.rssi DESC
        LIMIT 50"
    )?;

    let devices = stmt
        .query_map(params![time_filter, time_filter, time_filter], |row| {
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

    Ok(devices)
}

fn format_initial_devices_message(devices: &[DeviceReport]) -> String {
    let mut message = String::new();
    
    message.push_str("<b>📱 WYRYTE URZADZENIA</b>\n\n");
    message.push_str(&format!("Znaleziono <b>{}</b> urzadzen(i)\n\n", devices.len()));

    for (idx, device) in devices.iter().enumerate() {
        let name = device.device_name.as_deref().unwrap_or("Unknown");
        let manufacturer = device.manufacturer_name.as_deref().unwrap_or("Unknown");
        
        message.push_str(&format!("<b>{}. {}</b>\n", idx + 1, name));
        message.push_str(&format!("   MAC: <code>{}</code>\n", device.mac_address));
        
        if !manufacturer.is_empty() && manufacturer != "Unknown" {
            message.push_str(&format!("   Producent: {}\n", manufacturer));
        }
        
        message.push_str(&format!(
            "   RSSI: {} dBm | Pakiety: {}\n",
            device.current_rssi, device.packet_count
        ));
        
        if device.is_connectable {
            message.push_str("   [Connectable]\n");
        }
        
        message.push_str(&format!(
            "   Pierwsze: {} | Ostatnie: {}\n\n",
            device.first_seen, device.last_seen
        ));
    }

    message.push_str(&format!(
        "Czas: {}",
        chrono::Local::now().format("%H:%M:%S")
    ));

    message
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
    
    eprintln!("[TELEGRAM] URL: {}", url);
    
    let client = reqwest::Client::builder()
        .use_native_tls()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Client build failed: {}", e))?;

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
            let _device_id: i64 = row.get(0)?;
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

    let html_content = generate_raw_packets_html(&conn, DEVICES_HISTORY_WINDOW_SECS / 60)
        .map_err(|e| e.to_string())?;
    
    send_html_file(&html_content, "raw_packets.html").await?;

    // 📈 Send RSSI trend analysis report
    if let Err(e) = periodic_rssi_trends_report().await {
        log::warn!("Failed to send RSSI trend report: {}", e);
    }

    update_last_report_time(&conn).map_err(|e| e.to_string())?;

    log::info!("[+] Sent Telegram report with {} device(s)", devices.len());

    Ok(())
}

fn generate_raw_packets_html(conn: &rusqlite::Connection, minutes: i64) -> Result<String, Box<dyn std::error::Error>> {
    let time_filter = format!("-{} minutes", minutes);
    
    let mut stmt = conn.prepare(
        "SELECT 
            f.mac_address,
            f.timestamp,
            f.rssi,
            f.advertising_data,
            f.phy,
            f.channel,
            f.frame_type,
            d.device_name,
            d.first_seen as first_detected
        FROM ble_advertisement_frames f
        LEFT JOIN devices d ON f.mac_address = d.mac_address
        WHERE f.timestamp > datetime('now', ?)
        ORDER BY f.timestamp DESC
        LIMIT 500"
    )?;

    let packets: Vec<(String, String, i8, String, String, i32, String, Option<String>, String)> = stmt
        .query_map([&time_filter], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get::<_, Option<String>>(8)?.unwrap_or_else(|| "N/A".to_string()),
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut html = String::new();
    html.push_str(r#"<!DOCTYPE html>
<html lang="pl">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>BLE Raw Packets</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 16px; background: #1a1a2e; color: #eee; }
        h1 { color: #00d9ff; font-size: 20px; margin-bottom: 16px; }
        .stats { color: #888; font-size: 14px; margin-bottom: 16px; }
        .packet { background: #16213e; border-radius: 8px; padding: 12px; margin-bottom: 8px; border-left: 3px solid #00d9ff; }
        .packet-header { display: flex; justify-content: space-between; margin-bottom: 8px; }
        .mac { font-family: monospace; color: #00d9ff; font-size: 14px; }
        .name { color: #aaa; font-size: 13px; }
        .time { color: #666; font-size: 12px; }
        .rssi { font-weight: bold; }
        .rssi strong { color: #ff6b6b; }
        .data { font-family: monospace; font-size: 11px; color: #888; word-break: break-all; background: #0f0f23; padding: 8px; border-radius: 4px; margin-top: 8px; }
        .meta { font-size: 11px; color: #555; margin-top: 4px; }
        .first-seen { color: #4ade80; }
    </style>
</head>
<body>
"#);

    html.push_str(&format!("<h1>📡 BLE Raw Packets (ostatnie {} min)</h1>", minutes));
    html.push_str(&format!("<div class=\"stats\">{} pakietow | {}</div>", 
        packets.len(),
        chrono::Local::now().format("%H:%M:%S")
    ));

    for (mac, timestamp, rssi, ad_data, phy, channel, frame_type, name, first_seen) in packets {
        let time_str = parse_and_format_time(&timestamp);
        
        html.push_str("<div class=\"packet\">");
        html.push_str("<div class=\"packet-header\">");
        html.push_str(&format!("<span class=\"mac\">{}</span>", mac));
        html.push_str(&format!("<span class=\"rssi\"><strong>{} dBm</strong></span>", rssi));
        html.push_str("</div>");
        
        if let Some(n) = name {
            html.push_str(&format!("<div class=\"name\">{}</div>", n));
        }
        
        html.push_str(&format!("<div class=\"time\">{}</div>", time_str));
        
        if !first_seen.is_empty() && first_seen != "N/A" {
            html.push_str(&format!("<div class=\"first-seen\">Pierwsze wykrycie: {}</div>", parse_and_format_time(&first_seen)));
        }
        
        html.push_str(&format!("<div class=\"data\">{}</div>", ad_data));
        html.push_str(&format!("<div class=\"meta\">PHY: {} | CH: {} | Typ: {}</div>", phy, channel, frame_type));
        
        html.push_str("</div>");
    }

    html.push_str("</body></html>");

    Ok(html)
}

async fn send_html_file(html_content: &str, filename: &str) -> Result<(), String> {
    let config = get_config();
    if !config.enabled {
        return Ok(());
    }

    let url = format!("https://api.telegram.org/bot{}/sendDocument", config.bot_token);
    
    let client = reqwest::Client::builder()
        .use_native_tls()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Client build failed: {}", e))?;

    let part = reqwest::multipart::Part::text(html_content.to_string())
        .file_name(filename.to_string())
        .mime_str("text/html")
        .map_err(|e| e.to_string())?;

    let form = reqwest::multipart::Form::new()
        .text("chat_id", config.chat_id.clone())
        .text("caption", "<b>📡 Logi pakietow BLE</b>")
        .part("document", part);

    let response = client
        .post(&url)
        .multipart(form)
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

// ═══════════════════════════════════════════════════════════════════════
// 📈 RSSI TREND REPORTING (Real-time motion/distance analysis)
// ═══════════════════════════════════════════════════════════════════════

/// Format RSSI trend analysis for Telegram and terminal
fn format_rssi_trends_report() -> String {
    let mut message = String::new();

    message.push_str("<b>📈 RSSI TREND ANALYSIS</b>\n");
    message.push_str(&format!("🕐 {}\n\n", chrono::Local::now().format("%H:%M:%S")));

    // Get global snapshot
    let snapshot = crate::rssi_trend_manager::GLOBAL_RSSI_MANAGER.get_snapshot();
    
    if snapshot.devices.is_empty() {
        message.push_str("Brak danych o trendach (oczekiwanie na skanowanie)\n");
        return message;
    }

    // Approaching devices
    let approaching: Vec<_> = snapshot.devices.iter()
        .filter(|d| d.trend == "approaching")
        .collect();
    
    // Leaving devices
    let leaving: Vec<_> = snapshot.devices.iter()
        .filter(|d| d.trend == "leaving")
        .collect();
    
    // Still devices
    let still: Vec<_> = snapshot.devices.iter()
        .filter(|d| d.motion == "still")
        .collect();
    
    // Moving devices
    let moving: Vec<_> = snapshot.devices.iter()
        .filter(|d| d.motion == "moving")
        .collect();

    // Summary stats
    message.push_str(&format!("<b>📊 PODSUMOWANIE</b>\n"));
    message.push_str(&format!("  Urządzenia: {}\n", snapshot.devices.len()));
    message.push_str(&format!("  📶 Zbliżające się: {}\n", approaching.len()));
    message.push_str(&format!("  📉 Oddalające się: {}\n", leaving.len()));
    message.push_str(&format!("  🚶 Poruszające się: {}\n", moving.len()));
    message.push_str(&format!("  🧍 Stacjonarne: {}\n\n", still.len()));

    // Approaching vehicles (interesting!)
    if !approaching.is_empty() {
        message.push_str("<b>📶 ZBLIŻAJĄCE SIĘ (Getting closer)</b>\n");
        for device in approaching.iter().take(5) {
            let rssi_trend = format!("{:.3}", device.slope);
            message.push_str(&format!(
                "  {} <code>{}</code>\n    RSSI: {} dBm | Trend: {}/s | Próbek: {}\n",
                "→", device.mac, device.rssi as i32, rssi_trend, device.sample_count
            ));
        }
        message.push_str("\n");
    }

    // Leaving devices
    if !leaving.is_empty() {
        message.push_str("<b>📉 ODDALAJĄCE SIĘ (Moving away)</b>\n");
        for device in leaving.iter().take(5) {
            let rssi_trend = format!("{:.3}", device.slope);
            message.push_str(&format!(
                "  {} <code>{}</code>\n    RSSI: {} dBm | Trend: {}/s | Próbek: {}\n",
                "←", device.mac, device.rssi as i32, rssi_trend, device.sample_count
            ));
        }
        message.push_str("\n");
    }

    // Moving devices (unstable signal)
    if !moving.is_empty() && moving.len() <= 5 {
        message.push_str("<b>🚶 AKTYWNE (In motion - unstable signal)</b>\n");
        for device in moving.iter().take(5) {
            let variance = format!("{:.2}", device.variance);
            message.push_str(&format!(
                "  {} <code>{}</code>\n    RSSI: {} dBm | Wariancja: {} dB² | Próbek: {}\n",
                "◆", device.mac, device.rssi as i32, variance, device.sample_count
            ));
        }
        message.push_str("\n");
    }

    // Top devices by signal quality (best connected)
    let sorted: Vec<_> = snapshot.devices.iter()
        .map(|d| (d, d.rssi as i32))
        .collect();
    
    if !sorted.is_empty() {
        message.push_str("<b>🔝 TOP SYGNAŁY (Strongest signals)</b>\n");
        let mut top = sorted;
        top.sort_by_key(|(_, rssi)| -rssi);
        for (device, _) in top.iter().take(3) {
            let quality = if device.rssi >= -50.0 {
                "🟢 Doskonały"
            } else if device.rssi >= -60.0 {
                "🔵 Dobry"
            } else if device.rssi >= -70.0 {
                "🟡 Słaby"
            } else if device.rssi >= -85.0 {
                "🔴 Bardzo słaby"
            } else {
                "⚫ Krytyczny"
            };
            message.push_str(&format!(
                "  {}: {} dBm | {}\n",
                device.mac, device.rssi as i32, quality
            ));
        }
    }

    message
}

/// Print RSSI trends to terminal with colors
pub fn print_rssi_trends_terminal() {
    let snapshot = crate::rssi_trend_manager::GLOBAL_RSSI_MANAGER.get_snapshot();
    
    if snapshot.devices.is_empty() {
        return;
    }

    println!("\n{}", "═".repeat(80));
    println!(" 📈 RSSI TREND ANALYSIS - {} devices tracked", snapshot.devices.len());
    println!("{}", "═".repeat(80));

    let approaching: Vec<_> = snapshot.devices.iter()
        .filter(|d| d.trend == "approaching")
        .collect();
    
    let leaving: Vec<_> = snapshot.devices.iter()
        .filter(|d| d.trend == "leaving")
        .collect();

    if !approaching.is_empty() {
        println!("\n📶 APPROACHING ({} devices):", approaching.len());
        for d in approaching.iter().take(5) {
            println!("   {} {} {} dBm (slope: {:.3} dB/s)", 
                "→", d.mac, d.rssi as i32, d.slope);
        }
    }

    if !leaving.is_empty() {
        println!("\n📉 LEAVING ({} devices):", leaving.len());
        for d in leaving.iter().take(5) {
            println!("   {} {} {} dBm (slope: {:.3} dB/s)", 
                "←", d.mac, d.rssi as i32, d.slope);
        }
    }

    println!("{}\n", "═".repeat(80));
}

/// Send RSSI trend report to Telegram
pub async fn send_rssi_trends_report() -> Result<(), String> {
    let config = get_config();
    if !config.enabled {
        return Ok(());
    }

    let message = format_rssi_trends_report();
    send_telegram_message(&config.bot_token, &config.chat_id, &message).await
}

/// Send periodic RSSI trend reports (call this in main loop)
pub async fn periodic_rssi_trends_report() -> Result<(), String> {
    // Send both to terminal and Telegram
    print_rssi_trends_terminal();
    
    if is_enabled() {
        if let Err(e) = send_rssi_trends_report().await {
            log::warn!("Failed to send RSSI trends to Telegram: {}", e);
        }
    }

    Ok(())
}
