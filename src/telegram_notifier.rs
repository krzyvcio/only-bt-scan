use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct NewDeviceInfo {
    pub mac_address: String,
    pub device_name: Option<String>,
    pub rssi: i8,
    pub manufacturer_id: Option<u16>,
    pub manufacturer_name: Option<String>,
    pub first_seen: String,
    pub last_seen: String,
    pub is_connectable: bool,
    pub services_count: usize,
}

pub fn get_config() -> TelegramConfig {
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    let chat_id = env::var("TELEGRAM_CHAT_ID").unwrap_or_default();

    let enabled = !bot_token.is_empty() && !chat_id.is_empty();

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
    Ok(())
}

pub async fn send_new_device_notification(device: &NewDeviceInfo) -> Result<(), String> {
    let config = get_config();

    if !config.enabled {
        return Ok(());
    }

    let message = format_device_message(device);

    send_telegram_message(&config.bot_token, &config.chat_id, &message).await
}

fn format_device_message(device: &NewDeviceInfo) -> String {
    let name = device.device_name.as_deref().unwrap_or("Unknown");
    let manufacturer = device.manufacturer_name.as_deref().unwrap_or("Unknown");
    let connectable = if device.is_connectable { "Yes" } else { "No" };

    let mut message = String::new();

    message.push_str("🔵 <b>NEW DEVICE DETECTED</b>\n\n");
    message.push_str(&format!("📱 <b>Name:</b> {}\n", name));
    message.push_str(&format!(
        "🔢 <b>MAC:</b> <code>{}</code>\n",
        device.mac_address
    ));
    message.push_str(&format!("📶 <b>RSSI:</b> {} dBm\n", device.rssi));
    message.push_str(&format!("🏭 <b>Manufacturer:</b> {}\n", manufacturer));
    message.push_str(&format!("🔗 <b>Connectable:</b> {}\n", connectable));
    message.push_str(&format!("📅 <b>First Seen:</b> {}\n", device.first_seen));
    message.push_str(&format!("🕐 <b>Last Seen:</b> {}\n", device.last_seen));

    if device.services_count > 0 {
        message.push_str(&format!(
            "🔌 <b>Services:</b> {} found\n",
            device.services_count
        ));
    }

    message
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

/// Check if device is newly detected (first appearance in database)
pub fn is_device_new(mac_address: &str) -> bool {
    if let Ok(conn) = rusqlite::Connection::open("bluetooth_scan.db") {
        let is_new: bool = conn
            .query_row(
                "SELECT COUNT(*) = 0 FROM devices WHERE mac_address = ?",
                [mac_address],
                |row| row.get(0),
            )
            .unwrap_or(true);
        is_new
    } else {
        false
    }
}

/// Send notification only for newly detected devices
pub async fn check_and_notify_new_devices(
    devices: &[crate::bluetooth_scanner::BluetoothDevice],
) -> Result<(), String> {
    if !is_enabled() {
        return Ok(());
    }

    for device in devices {
        // Check if device is new (hasn't been in database before)
        if is_device_new(&device.mac_address) {
            let first_seen = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let last_seen = first_seen.clone();

            let new_device = NewDeviceInfo {
                mac_address: device.mac_address.clone(),
                device_name: device.name.clone(),
                rssi: device.rssi,
                manufacturer_id: device.manufacturer_id,
                manufacturer_name: device.manufacturer_name.clone(),
                first_seen,
                last_seen,
                is_connectable: device.is_connectable,
                services_count: device.services.len(),
            };

            if let Err(e) = send_new_device_notification(&new_device).await {
                log::warn!("Failed to send Telegram notification: {}", e);
            } else {
                log::info!(
                    "Sent Telegram notification for new device: {}",
                    device.mac_address
                );
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    Ok(())
}
