use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use serde::{Deserialize, Serialize};

use crate::db_frames;

const DB_PATH: &str = "bluetooth_scan.db";

/// Parsed BLE Advertisement Data struktura
#[derive(Debug, Clone, Default)]
pub struct ParsedAdvertisementData {
    pub local_name: Option<String>,
    pub tx_power: Option<i8>,
    pub flags: Option<String>,
    pub appearance: Option<String>,
    pub service_uuids: Vec<String>,
    pub manufacturer_name: Option<String>,
    pub manufacturer_data: Option<String>,

    // Temporal metrics (1ms resolution)
    pub frame_interval_ms: Option<i32>, // Time since last frame for THIS device
    pub frames_per_second: Option<f32>, // Rate this device is transmitting
}

/// Get last advertisement data for a device and parse it
/// WITH frame interval timing (millisecond precision)
pub fn get_parsed_advertisement_with_timing(mac_address: &str) -> ParsedAdvertisementData {
    if let Ok(conn) = Connection::open(DB_PATH) {
        // Get last 2 frames for this device to calculate interval
        if let Ok(mut stmt) = conn.prepare(
            "SELECT advertising_data,
                    COALESCE(timestamp_ms, CAST(strftime('%s', timestamp) AS INTEGER) * 1000)
             FROM ble_advertisement_frames
             WHERE mac_address = ?
             ORDER BY timestamp DESC
             LIMIT 2",
        ) {
            let mut results: Vec<(String, i64)> = Vec::new();
            if let Ok(rows) = stmt.query_map([mac_address], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            }) {
                for row in rows {
                    if let Ok((ad_hex, ts_ms)) = row {
                        results.push((ad_hex, ts_ms));
                    }
                }
            }

            if !results.is_empty() {
                let mut ad_data = parse_advertisement_data(&results[0].0);

                // Calculate frame interval if we have 2 timestamps
                if results.len() >= 2 {
                    let interval_ms = (results[0].1 - results[1].1) as i32;
                    ad_data.frame_interval_ms = Some(interval_ms.abs());

                    // Calculate frames per second (if interval > 0)
                    if interval_ms > 0 {
                        ad_data.frames_per_second = Some(1000.0 / interval_ms as f32);
                    }
                }

                return ad_data;
            }
        }
    }

    ParsedAdvertisementData::default()
}

/// Parse BLE Advertisement Data from hex string
/// Format: Length-Type-Value (LTV) frames
/// 1eff060001092022... = 1e(len) ff(type=mfg) 0600(mfg_id=Microsoft) 01092022...(data)
pub fn parse_advertisement_data(hex_data: &str) -> ParsedAdvertisementData {
    let mut result = ParsedAdvertisementData::default();

    // Convert hex string to bytes
    let bytes = match hex_to_bytes(hex_data) {
        Some(b) => b,
        None => return result,
    };

    let mut pos = 0;
    while pos < bytes.len() {
        // Read length
        let length = bytes[pos] as usize;
        if length == 0 || pos + length + 1 > bytes.len() {
            break;
        }
        pos += 1;

        // Read type
        let ad_type = bytes[pos];
        pos += 1;

        let data_len = length - 1; // Subtract type byte

        match ad_type {
            0x01 => {
                // Flags
                if pos < bytes.len() {
                    result.flags = parse_flags(bytes[pos]);
                }
            }
            0x08 | 0x09 => {
                // Incomplete / Complete Local Name
                if let Ok(name) = std::str::from_utf8(&bytes[pos..pos.min(pos + data_len)]) {
                    result.local_name = Some(name.to_string());
                }
            }
            0x0A => {
                // TX Power Level
                if pos < bytes.len() {
                    result.tx_power = Some(bytes[pos] as i8);
                }
            }
            0x19 => {
                // Appearance (2 bytes, little-endian)
                if pos + 1 < bytes.len() {
                    let appearance = u16::from_le_bytes([bytes[pos], bytes[pos + 1]]) as u32;
                    result.appearance = Some(format!("0x{:04x}", appearance));
                }
            }
            0x06 | 0x07 => {
                // Incomplete / Complete 128-bit Service UUIDs
                let mut uuid_pos = pos;
                while uuid_pos + 16 <= pos + data_len && uuid_pos + 16 <= bytes.len() {
                    let uuid = format!(
                        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                        bytes[uuid_pos + 0], bytes[uuid_pos + 1], bytes[uuid_pos + 2], bytes[uuid_pos + 3],
                        bytes[uuid_pos + 4], bytes[uuid_pos + 5], bytes[uuid_pos + 6], bytes[uuid_pos + 7],
                        bytes[uuid_pos + 8], bytes[uuid_pos + 9], bytes[uuid_pos + 10], bytes[uuid_pos + 11],
                        bytes[uuid_pos + 12], bytes[uuid_pos + 13], bytes[uuid_pos + 14], bytes[uuid_pos + 15]
                    );
                    result.service_uuids.push(uuid);
                    uuid_pos += 16;
                }
            }
            0xFF => {
                // Manufacturer Specific Data (little-endian 16-bit company ID)
                if pos + 1 < bytes.len() {
                    let mfg_id = u16::from_le_bytes([bytes[pos], bytes[pos + 1]]);
                    result.manufacturer_name = Some(get_manufacturer_name(mfg_id));

                    // Format manufacturer data as hex
                    if data_len > 2 {
                        let mfg_data = &bytes[pos + 2..pos + data_len];
                        result.manufacturer_data = Some(format!("0x{}", bytes_to_hex(mfg_data)));
                    }
                }
            }
            _ => {} // Ignore other types for now
        }

        pos += data_len;
    }

    result
}

/// Convert hex string to bytes
fn hex_to_bytes(hex_str: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let hex_str = hex_str.trim().replace(" ", "").replace("\n", "");

    for i in (0..hex_str.len()).step_by(2) {
        if i + 1 < hex_str.len() {
            if let Ok(byte) = u8::from_str_radix(&hex_str[i..i + 2], 16) {
                bytes.push(byte);
            } else {
                return None;
            }
        }
    }

    if bytes.is_empty() {
        None
    } else {
        Some(bytes)
    }
}

/// Convert bytes to hex string
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Parse flags byte
fn parse_flags(flags: u8) -> Option<String> {
    let mut flag_list = Vec::new();

    if flags & 0x01 != 0 {
        flag_list.push("LE Limited Discoverable");
    }
    if flags & 0x02 != 0 {
        flag_list.push("LE General Discoverable");
    }
    if flags & 0x04 != 0 {
        flag_list.push("BR/EDR Not Supported");
    }
    if flags & 0x08 != 0 {
        flag_list.push("Simultaneous LE+BR/EDR");
    }
    if flags & 0x10 != 0 {
        flag_list.push("LE BR/EDR Controller");
    }
    if flags & 0x20 != 0 {
        flag_list.push("LE BR/EDR Host");
    }

    if flag_list.is_empty() {
        None
    } else {
        Some(flag_list.join(", "))
    }
}

/// Get manufacturer name by ID (uses dynamic company_ids module)
fn get_manufacturer_name(mfg_id: u16) -> String {
    crate::company_ids::get_company_name(mfg_id)
}

/// Parse Device Class (0x005a420c format) to Service Classes and Device Type
pub fn parse_device_class(device_class_str: Option<&str>) -> (Option<String>, Option<String>) {
    if let Some(dc_str) = device_class_str {
        // Remove "0x" prefix if exists
        let hex_str = dc_str.trim_start_matches("0x").trim_start_matches("0X");

        if hex_str.len() >= 6 {
            // Parse three bytes: service classes, major, minor
            if let Ok(service_byte) = u8::from_str_radix(&hex_str[0..2], 16) {
                if let Ok(major_byte) = u8::from_str_radix(&hex_str[2..4], 16) {
                    if let Ok(minor_byte) = u8::from_str_radix(&hex_str[4..6], 16) {
                        let services = parse_service_classes(service_byte);
                        let device_type = parse_device_type(major_byte, minor_byte);
                        return (services, device_type);
                    }
                }
            }
        }
    }
    (None, None)
}

fn parse_service_classes(byte: u8) -> Option<String> {
    let mut classes = Vec::new();

    if byte & 0x80 != 0 {
        classes.push("LE Audio");
    }
    if byte & 0x40 != 0 {
        classes.push("Rendering");
    }
    if byte & 0x20 != 0 {
        classes.push("Capturing");
    }
    if byte & 0x10 != 0 {
        classes.push("Object Transfer");
    }
    if byte & 0x08 != 0 {
        classes.push("Audio");
    }
    if byte & 0x04 != 0 {
        classes.push("Telephony");
    }
    if byte & 0x02 != 0 {
        classes.push("Networking");
    }
    if byte & 0x01 != 0 {
        classes.push("Limited Discoverable");
    }

    if classes.is_empty() {
        None
    } else {
        Some(classes.join(", "))
    }
}

fn parse_device_type(major: u8, minor: u8) -> Option<String> {
    let major_class = major >> 2; // Top 6 bits
    let minor_class = (major & 0x03) << 4 | (minor >> 4); // Bottom 2 of major + top 4 of minor

    let major_name = match major_class {
        0 => "Miscellaneous",
        1 => "Computer",
        2 => "Phone",
        3 => "LAN/Network",
        4 => "Audio/Video",
        5 => "Peripheral",
        6 => "Imaging",
        7 => "Wearable",
        8 => "Toy",
        9 => "Health",
        _ => "Uncategorized",
    };

    let minor_name = match major_class {
        1 => match minor_class >> 2 {
            // Computer
            0 => "Unspecified",
            1 => "Desktop",
            2 => "Laptop",
            3 => "Handheld",
            4 => "Pad",
            5 => "Server",
            _ => "Other",
        },
        2 => match minor_class >> 2 {
            // Phone
            0 => "Unspecified",
            1 => "Cellular",
            2 => "Cordless",
            3 => "Smartphone",
            4 => "Wired",
            _ => "Other",
        },
        4 => match minor_class >> 2 {
            // Audio/Video
            0 => "Unspecified",
            1 => "Headset",
            2 => "Hands-Free",
            3 => "Microphone",
            4 => "Loudspeaker",
            5 => "Headphones",
            6 => "Portable Audio",
            7 => "Car Audio",
            8 => "Set-top Box",
            9 => "HiFi Audio",
            10 => "VCR",
            11 => "Video Camera",
            12 => "Camcorder",
            13 => "Video Monitor",
            14 => "Video Display",
            15 => "Video Conferencing",
            _ => "Other",
        },
        7 => match minor_class >> 2 {
            // Wearable
            1 => "Wristwatch",
            2 => "Pager",
            3 => "Jacket",
            4 => "Helmet",
            5 => "Glasses",
            _ => "Wearable Device",
        },
        8 => match minor_class >> 2 {
            // Toy
            1 => "Robot",
            2 => "Vehicle",
            3 => "Doll",
            4 => "Controller",
            5 => "Game",
            _ => "Toy",
        },
        9 => match minor_class >> 2 {
            // Health
            1 => "Blood Pressure Monitor",
            2 => "Thermometer",
            3 => "Weighing Scale",
            4 => "Glucose Meter",
            5 => "Pulse Oximeter",
            6 => "Heart/Pulse Rate Monitor",
            7 => "Health Data Display",
            8 => "Step Counter",
            9 => "Body Composition Scale",
            10 => "Peak Flow Monitor",
            11 => "Medication Dispenser",
            _ => "Health Device",
        },
        _ => "Device",
    };

    if minor_name == "Other" || minor_name == "Unspecified" || minor_name == "Device" {
        Some(major_name.to_string())
    } else {
        Some(format!("{}/{}", major_name, minor_name))
    }
}

#[derive(Debug, Clone)]
pub struct ScannedDevice {
    pub mac_address: String,
    pub name: Option<String>,
    pub rssi: i8,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub manufacturer_id: Option<u16>,
    pub manufacturer_name: Option<String>,
    pub mac_type: Option<String>,
    pub is_rpa: bool,
    pub security_level: Option<String>,
    pub pairing_method: Option<String>,
    pub is_authenticated: bool,
    pub device_class: Option<String>,
    pub service_classes: Option<String>,
    pub device_type: Option<String>,
    pub ad_flags: Option<String>,
    pub ad_local_name: Option<String>,
    pub ad_tx_power: Option<i8>,
    pub ad_appearance: Option<String>,
    pub ad_service_uuids: Option<String>,
    pub ad_manufacturer_data: Option<String>,
    pub ad_service_data: Option<String>,
}

impl Default for ScannedDevice {
    fn default() -> Self {
        Self {
            mac_address: String::new(),
            name: None,
            rssi: -100,
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            manufacturer_id: None,
            manufacturer_name: None,
            mac_type: None,
            is_rpa: false,
            security_level: None,
            pairing_method: None,
            is_authenticated: false,
            device_class: None,
            service_classes: None,
            device_type: None,
            ad_flags: None,
            ad_local_name: None,
            ad_tx_power: None,
            ad_appearance: None,
            ad_service_uuids: None,
            ad_manufacturer_data: None,
            ad_service_data: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BleService {
    pub device_id: i32,
    pub uuid16: Option<u16>,
    pub uuid128: Option<String>,
    pub name: Option<String>,
}

/// Initialize the database with required tables
pub fn init_database() -> SqliteResult<()> {
    let conn = Connection::open(DB_PATH)?;

    // Create devices table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS devices (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            mac_address TEXT UNIQUE NOT NULL,
            device_name TEXT,
            rssi INTEGER NOT NULL,
            first_seen TIMESTAMP NOT NULL,
            last_seen TIMESTAMP NOT NULL,
            manufacturer_id INTEGER,
            manufacturer_name TEXT,
            device_type TEXT,
            number_of_scan INTEGER DEFAULT 1,
            mac_type TEXT,
            is_rpa INTEGER DEFAULT 0,
            security_level TEXT,
            pairing_method TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Add columns if not exists (for existing databases)
    conn.execute(
        "ALTER TABLE devices ADD COLUMN number_of_scan INTEGER DEFAULT 1",
        [],
    )
    .ok();

    conn.execute("ALTER TABLE devices ADD COLUMN mac_type TEXT", [])
        .ok();

    conn.execute(
        "ALTER TABLE devices ADD COLUMN is_rpa INTEGER DEFAULT 0",
        [],
    )
    .ok();

    conn.execute("ALTER TABLE devices ADD COLUMN security_level TEXT", [])
        .ok();

    conn.execute("ALTER TABLE devices ADD COLUMN pairing_method TEXT", [])
        .ok();

    conn.execute(
        "ALTER TABLE devices ADD COLUMN is_authenticated INTEGER DEFAULT 0",
        [],
    )
    .ok();

    conn.execute("ALTER TABLE devices ADD COLUMN device_class TEXT", [])
        .ok();

    conn.execute("ALTER TABLE devices ADD COLUMN service_classes TEXT", [])
        .ok();

    conn.execute("ALTER TABLE devices ADD COLUMN device_type TEXT", [])
        .ok();

    // Add advertising data columns
    conn.execute("ALTER TABLE devices ADD COLUMN ad_flags TEXT", [])
        .ok();
    conn.execute("ALTER TABLE devices ADD COLUMN ad_local_name TEXT", [])
        .ok();
    conn.execute("ALTER TABLE devices ADD COLUMN ad_tx_power INTEGER", [])
        .ok();
    conn.execute("ALTER TABLE devices ADD COLUMN ad_appearance TEXT", [])
        .ok();
    conn.execute("ALTER TABLE devices ADD COLUMN ad_service_uuids TEXT", [])
        .ok();
    conn.execute(
        "ALTER TABLE devices ADD COLUMN ad_manufacturer_data TEXT",
        [],
    )
    .ok();
    conn.execute("ALTER TABLE devices ADD COLUMN ad_service_data TEXT", [])
        .ok();

    // Create BLE services table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ble_services (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id INTEGER NOT NULL,
            uuid16 INTEGER,
            uuid128 TEXT,
            service_name TEXT,
            characteristic_count INTEGER DEFAULT 0,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(device_id) REFERENCES devices(id) ON DELETE CASCADE,
            UNIQUE(device_id, uuid16, uuid128)
        )",
        [],
    )?;

    // Create scan history table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS scan_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id INTEGER NOT NULL,
            rssi INTEGER NOT NULL,
            scan_number INTEGER DEFAULT 1,
            scan_timestamp TIMESTAMP NOT NULL,
            FOREIGN KEY(device_id) REFERENCES devices(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Add column if not exists
    conn.execute(
        "ALTER TABLE scan_history ADD COLUMN scan_number INTEGER DEFAULT 1",
        [],
    )
    .ok();

    // Create indexes for better query performance
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_devices_mac ON devices(mac_address)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_devices_last_seen ON devices(last_seen)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_ble_services_device ON ble_services(device_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_scan_history_device ON scan_history(device_id)",
        [],
    )?;

    // ═══════════════════════════════════════════════════════════════
    // TELEMETRY HISTORY TABLES (for v0.4.0 persistence)
    // ═══════════════════════════════════════════════════════════════

    // Telemetry snapshots - overall stats every 5 minutes
    conn.execute(
        "CREATE TABLE IF NOT EXISTS telemetry_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            snapshot_timestamp DATETIME NOT NULL,
            total_packets INTEGER NOT NULL,
            total_devices INTEGER NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Per-device telemetry history
    conn.execute(
        "CREATE TABLE IF NOT EXISTS device_telemetry_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            snapshot_id INTEGER NOT NULL,
            device_mac TEXT NOT NULL,
            packet_count INTEGER NOT NULL,
            avg_rssi REAL NOT NULL,
            min_latency_ms INTEGER DEFAULT 0,
            max_latency_ms INTEGER DEFAULT 0,
            FOREIGN KEY(snapshot_id) REFERENCES telemetry_snapshots(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create indexes for telemetry queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_telemetry_snapshots_timestamp ON telemetry_snapshots(snapshot_timestamp)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_device_telemetry_snapshot ON device_telemetry_history(snapshot_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_device_telemetry_mac ON device_telemetry_history(device_mac)",
        [],
    )?;

    // Initialize frame storage tables (ble_advertisement_frames, frame_statistics)
    db_frames::init_frame_storage(&conn)?;

    Ok(())
}

/// Insert or update a scanned device
pub fn insert_or_update_device(device: &ScannedDevice) -> SqliteResult<i32> {
    let conn = Connection::open(DB_PATH)?;
    let now = Utc::now();

    // Try to update first (increment scan count)
    let updated = conn.execute(
        "UPDATE devices 
         SET rssi = ?1, last_seen = ?2, device_name = ?3, manufacturer_id = ?4, manufacturer_name = ?5, number_of_scan = number_of_scan + 1
         WHERE mac_address = ?6",
        params![
            device.rssi,
            now,
            &device.name,
            device.manufacturer_id,
            &device.manufacturer_name,
            &device.mac_address,
        ],
    )?;

    if updated > 0 {
        // Get the ID of updated device
        let mut stmt = conn.prepare("SELECT id FROM devices WHERE mac_address = ?1")?;
        let device_id: i32 = stmt.query_row(params![&device.mac_address], |row| row.get(0))?;

        // Update security info if provided
        if let Some(ref mac_type) = device.mac_type {
            if let Some(ref security) = device.security_level {
                if let Some(ref pairing) = device.pairing_method {
                    conn.execute(
                        "UPDATE devices SET mac_type = ?1, is_rpa = ?2, security_level = ?3, pairing_method = ?4, is_authenticated = ?5 WHERE id = ?6",
                        params![mac_type, device.is_rpa as i32, security, pairing, device.is_authenticated as i32, device_id],
                    ).ok();
                }
            }
        }

        // 📊 Update global RSSI trend analyzer
        if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::rssi_trend_manager::GLOBAL_RSSI_MANAGER.update_rssi(
                &device.mac_address,
                device.rssi,
                now,
            );
        })) {
            log::warn!("Failed to update RSSI trend: {:?}", e);
        }

        return Ok(device_id);
    }

    // If no update, insert new device
    conn.execute(
        "INSERT INTO devices (mac_address, device_name, rssi, first_seen, last_seen, manufacturer_id, manufacturer_name, number_of_scan, mac_type, is_rpa, security_level, pairing_method, is_authenticated)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9, ?10, ?11, ?12)",
        params![
            &device.mac_address,
            &device.name,
            device.rssi,
            device.first_seen,
            now,
            device.manufacturer_id,
            &device.manufacturer_name,
            &device.mac_type,
            device.is_rpa as i32,
            &device.security_level,
            &device.pairing_method,
            device.is_authenticated as i32,
        ],
    )?;

    let device_id = conn.last_insert_rowid() as i32;

    // 📊 Update global RSSI trend analyzer
    if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        crate::rssi_trend_manager::GLOBAL_RSSI_MANAGER.update_rssi(
            &device.mac_address,
            device.rssi,
            now,
        );
    })) {
        log::warn!("Failed to update RSSI trend: {:?}", e);
    }

    Ok(device_id)
}

/// Insert BLE service for a device
pub fn insert_ble_service(
    device_id: i32,
    uuid16: Option<u16>,
    uuid128: Option<&str>,
    name: Option<&str>,
) -> SqliteResult<()> {
    let conn = Connection::open(DB_PATH)?;

    conn.execute(
        "INSERT OR IGNORE INTO ble_services (device_id, uuid16, uuid128, service_name)
         VALUES (?1, ?2, ?3, ?4)",
        params![device_id, uuid16, uuid128, name],
    )?;

    Ok(())
}

/// Get or create global scan counter
pub fn get_next_scan_number() -> SqliteResult<i32> {
    let conn = Connection::open(DB_PATH)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS scan_counter (
            id INTEGER PRIMARY KEY,
            counter INTEGER DEFAULT 0
        )",
        [],
    )
    .ok();

    let count: i32 = conn
        .query_row("SELECT counter FROM scan_counter WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    let new_count = count + 1;

    conn.execute(
        "INSERT OR REPLACE INTO scan_counter (id, counter) VALUES (1, ?1)",
        params![new_count],
    )?;

    Ok(new_count)
}

/// Record RSSI value in scan history with scan number
pub fn record_scan_rssi(device_id: i32, rssi: i8, scan_number: i32) -> SqliteResult<()> {
    let conn = Connection::open(DB_PATH)?;
    let now = Utc::now();

    conn.execute(
        "INSERT INTO scan_history (device_id, rssi, scan_number, scan_timestamp)
         VALUES (?1, ?2, ?3, ?4)",
        params![device_id, rssi, scan_number, now],
    )?;

    Ok(())
}

/// Get all devices
pub fn get_all_devices() -> SqliteResult<Vec<ScannedDevice>> {
    let conn = Connection::open(DB_PATH)?;
    let mut stmt = conn.prepare(
        "SELECT mac_address, device_name, rssi, first_seen, last_seen, manufacturer_id, manufacturer_name, is_authenticated, device_class
         FROM devices
         ORDER BY last_seen DESC"
    )?;

    let devices = stmt.query_map([], |row| {
        Ok(ScannedDevice {
            mac_address: row.get(0)?,
            name: row.get(1)?,
            rssi: row.get(2)?,
            first_seen: row.get(3)?,
            last_seen: row.get(4)?,
            manufacturer_id: row.get(5)?,
            manufacturer_name: row.get(6)?,
            mac_type: None,
            is_rpa: false,
            security_level: None,
            pairing_method: None,
            is_authenticated: row.get::<_, i32>(7).unwrap_or(0) != 0,
            device_class: row.get(8).ok(),
            service_classes: None,
            device_type: None,
            ..Default::default()
        })
    })?;

    devices.collect()
}

/// Get device by MAC address
pub fn get_device(mac_address: &str) -> SqliteResult<Option<ScannedDevice>> {
    let conn = Connection::open(DB_PATH)?;
    let mut stmt = conn.prepare(
        "SELECT mac_address, device_name, rssi, first_seen, last_seen, manufacturer_id, manufacturer_name, is_authenticated, device_class
         FROM devices
         WHERE mac_address = ?1"
    )?;

    let device: Option<ScannedDevice> = stmt
        .query_row(params![mac_address], |row| {
            Ok(ScannedDevice {
                mac_address: row.get(0)?,
                name: row.get(1)?,
                rssi: row.get(2)?,
                first_seen: row.get(3)?,
                last_seen: row.get(4)?,
                manufacturer_id: row.get(5)?,
                manufacturer_name: row.get(6)?,
                mac_type: None,
                is_rpa: false,
                security_level: None,
                pairing_method: None,
                is_authenticated: row.get::<_, i32>(7).unwrap_or(0) != 0,
                device_class: row.get(8).ok(),
                service_classes: None,
                device_type: None,
                ..Default::default()
            })
        })
        .optional()?;

    Ok(device)
}

/// Get BLE services for a device
pub fn get_device_services(device_id: i32) -> SqliteResult<Vec<BleService>> {
    let conn = Connection::open(DB_PATH)?;
    let mut stmt = conn.prepare(
        "SELECT device_id, uuid16, uuid128, service_name
         FROM ble_services
         WHERE device_id = ?1
         ORDER BY service_name",
    )?;

    let services = stmt.query_map(params![device_id], |row| {
        Ok(BleService {
            device_id: row.get(0)?,
            uuid16: row.get(1)?,
            uuid128: row.get(2)?,
            name: row.get(3)?,
        })
    })?;

    services.collect()
}

/// Get devices found in the last N minutes
pub fn get_recent_devices(minutes: u32) -> SqliteResult<Vec<ScannedDevice>> {
    let conn = Connection::open(DB_PATH)?;
    let mut stmt = conn.prepare(
        "SELECT mac_address, device_name, rssi, first_seen, last_seen, manufacturer_id, manufacturer_name, is_authenticated, device_class
         FROM devices
         WHERE last_seen > datetime('now', ?1)
         ORDER BY last_seen DESC"
    )?;

    let time_filter = format!("-{} minutes", minutes);
    let devices = stmt.query_map(params![&time_filter], |row| {
        Ok(ScannedDevice {
            mac_address: row.get(0)?,
            name: row.get(1)?,
            rssi: row.get(2)?,
            first_seen: row.get(3)?,
            last_seen: row.get(4)?,
            manufacturer_id: row.get(5)?,
            manufacturer_name: row.get(6)?,
            mac_type: None,
            is_rpa: false,
            security_level: None,
            pairing_method: None,
            is_authenticated: row.get::<_, i32>(7).unwrap_or(0) != 0,
            device_class: row.get(8).ok(),
            service_classes: None,
            device_type: None,
            ..Default::default()
        })
    })?;

    devices.collect()
}

/// Get device count
pub fn get_device_count() -> SqliteResult<i32> {
    let conn = Connection::open(DB_PATH)?;
    conn.query_row("SELECT COUNT(*) FROM devices", [], |row| row.get(0))
}

/// Clear old scan history (older than X days)
pub fn cleanup_old_scans(days: u32) -> SqliteResult<usize> {
    let conn = Connection::open(DB_PATH)?;
    let time_filter = format!("-{} days", days);

    conn.execute(
        "DELETE FROM scan_history WHERE scan_timestamp < datetime('now', ?1)",
        params![&time_filter],
    )
}

// ═══════════════════════════════════════════════════════════════
// TELEMETRY PERSISTENCE FUNCTIONS (v0.4.0+)
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct TelemetrySnapshot {
    pub id: i32,
    pub snapshot_timestamp: DateTime<Utc>,
    pub total_packets: i32,
    pub total_devices: i32,
}

#[derive(Debug, Clone)]
pub struct DeviceTelemetryRecord {
    pub id: i32,
    pub snapshot_id: i32,
    pub device_mac: String,
    pub packet_count: i32,
    pub avg_rssi: f64,
    pub min_latency_ms: i32,
    pub max_latency_ms: i32,
}

/// RSSI Trend point - for signal quality trend visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssiTrendPoint {
    pub timestamp: DateTime<Utc>,
    pub avg_rssi: f64,
    pub packet_count: i32,
    pub signal_quality: String, // "excellent", "good", "fair", "poor", "very_poor"
}

/// Raw RSSI measurement from advertisement frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssiMeasurement {
    pub timestamp: DateTime<Utc>,
    pub rssi: i32,
    pub signal_quality: String,
}

/// Save telemetry snapshot to database
pub fn save_telemetry_snapshot(
    snapshot_timestamp: DateTime<Utc>,
    total_packets: i32,
    total_devices: i32,
) -> SqliteResult<i32> {
    let conn = Connection::open(DB_PATH)?;

    conn.execute(
        "INSERT INTO telemetry_snapshots (snapshot_timestamp, total_packets, total_devices)
         VALUES (?1, ?2, ?3)",
        params![snapshot_timestamp, total_packets, total_devices],
    )?;

    let snapshot_id = conn.last_insert_rowid() as i32;
    Ok(snapshot_id)
}

/// Save device telemetry for a snapshot
pub fn save_device_telemetry(
    snapshot_id: i32,
    device_mac: &str,
    packet_count: u64,
    avg_rssi: f64,
    min_latency_ms: u64,
    max_latency_ms: u64,
) -> SqliteResult<()> {
    let conn = Connection::open(DB_PATH)?;

    conn.execute(
        "INSERT INTO device_telemetry_history 
         (snapshot_id, device_mac, packet_count, avg_rssi, min_latency_ms, max_latency_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            snapshot_id,
            device_mac,
            packet_count as i32,
            avg_rssi,
            min_latency_ms as i32,
            max_latency_ms as i32
        ],
    )?;

    Ok(())
}

/// Get telemetry snapshots from last N hours
pub fn get_telemetry_snapshots(hours: u32) -> SqliteResult<Vec<TelemetrySnapshot>> {
    let conn = Connection::open(DB_PATH)?;
    let time_filter = format!("-{} hours", hours);

    let mut stmt = conn.prepare(
        "SELECT id, snapshot_timestamp, total_packets, total_devices
         FROM telemetry_snapshots
         WHERE snapshot_timestamp > datetime('now', ?1)
         ORDER BY snapshot_timestamp DESC",
    )?;

    let snapshots = stmt.query_map(params![&time_filter], |row| {
        let ts_str: String = row.get(1)?;
        let timestamp = DateTime::parse_from_rfc3339(&ts_str)
            .unwrap_or_else(|_| {
                chrono::Local::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
            })
            .with_timezone(&Utc);

        Ok(TelemetrySnapshot {
            id: row.get(0)?,
            snapshot_timestamp: timestamp,
            total_packets: row.get(2)?,
            total_devices: row.get(3)?,
        })
    })?;

    snapshots.collect()
}

/// Get device telemetry for a snapshot
pub fn get_snapshot_device_telemetry(snapshot_id: i32) -> SqliteResult<Vec<DeviceTelemetryRecord>> {
    let conn = Connection::open(DB_PATH)?;

    let mut stmt = conn.prepare(
        "SELECT id, snapshot_id, device_mac, packet_count, avg_rssi, min_latency_ms, max_latency_ms
         FROM device_telemetry_history
         WHERE snapshot_id = ?1
         ORDER BY packet_count DESC",
    )?;

    let records = stmt.query_map(params![snapshot_id], |row| {
        Ok(DeviceTelemetryRecord {
            id: row.get(0)?,
            snapshot_id: row.get(1)?,
            device_mac: row.get(2)?,
            packet_count: row.get(3)?,
            avg_rssi: row.get(4)?,
            min_latency_ms: row.get(5)?,
            max_latency_ms: row.get(6)?,
        })
    })?;

    records.collect()
}

/// Delete telemetry snapshots older than X days
pub fn cleanup_old_telemetry(days: u32) -> SqliteResult<usize> {
    let conn = Connection::open(DB_PATH)?;
    let time_filter = format!("-{} days", days);

    conn.execute(
        "DELETE FROM telemetry_snapshots WHERE snapshot_timestamp < datetime('now', ?1)",
        params![&time_filter],
    )
}

/// Insert advertisement frame (for real-time HCI capture)
/// Called directly from HCI sniffer task
pub fn insert_advertisement_frame(
    mac_address: &str,
    rssi: i8,
    advertising_data_hex: &str,
    phy: &str,
    channel: u8,
    frame_type: &str,
    timestamp_ms: u64,
) -> SqliteResult<()> {
    let conn = Connection::open(DB_PATH)?;

    // Convert milliseconds to ISO 8601 datetime string
    let timestamp_str = if let Some(dt) =
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(timestamp_ms as i64)
    {
        dt.to_rfc3339()
    } else {
        chrono::Utc::now().to_rfc3339()
    };

    // Get or create device
    let device_id: i32 = conn
        .query_row(
            "SELECT id FROM devices WHERE mac_address = ?",
            [mac_address],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or_else(|| {
            // Create new device if doesn't exist
            conn.execute(
                "INSERT INTO devices (mac_address, device_name, rssi, first_seen, last_seen)
             VALUES (?, NULL, ?, datetime('now'), datetime('now'))",
                [mac_address, &rssi.to_string(), ""],
            )
            .ok();

            conn.query_row(
                "SELECT id FROM devices WHERE mac_address = ?",
                [mac_address],
                |row| row.get(0),
            )
            .unwrap_or(0)
        });

    // Insert advertisement frame
    conn.execute(
        "INSERT INTO ble_advertisement_frames 
         (device_id, mac_address, rssi, advertising_data, phy, channel, frame_type, timestamp, timestamp_ms)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![device_id, mac_address, rssi, advertising_data_hex, phy, channel as i32, frame_type, timestamp_str, timestamp_ms as i64],
    )?;

    // Update device last_seen
    conn.execute(
        "UPDATE devices SET last_seen = ?, rssi = ? WHERE mac_address = ?",
        params![timestamp_str, rssi, mac_address],
    )?;

    Ok(())
}

/// Insert advertisement frame using pooled connection
/// This is more efficient than opening a new connection each time
pub fn insert_advertisement_frame_pooled(
    mac_address: &str,
    rssi: i8,
    advertising_data_hex: &str,
    phy: &str,
    channel: u8,
    frame_type: &str,
    timestamp_ms: u64,
) -> SqliteResult<()> {
    let pool = crate::db_pool::get_pool();
    if let Some(pool) = pool {
        pool.execute(|conn| {
            insert_advertisement_frame_inner(
                conn,
                mac_address,
                rssi,
                advertising_data_hex,
                phy,
                channel,
                frame_type,
                timestamp_ms,
            )
        })
    } else {
        insert_advertisement_frame(
            mac_address,
            rssi,
            advertising_data_hex,
            phy,
            channel,
            frame_type,
            timestamp_ms,
        )
    }
}

/// Inner function for inserting advertisement frame (used by pooled version)
fn insert_advertisement_frame_inner(
    conn: &Connection,
    mac_address: &str,
    rssi: i8,
    advertising_data_hex: &str,
    phy: &str,
    channel: u8,
    frame_type: &str,
    timestamp_ms: u64,
) -> SqliteResult<()> {
    let timestamp_str = if let Some(dt) =
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(timestamp_ms as i64)
    {
        dt.to_rfc3339()
    } else {
        chrono::Utc::now().to_rfc3339()
    };

    let device_id: i32 = conn
        .query_row(
            "SELECT id FROM devices WHERE mac_address = ?",
            [mac_address],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or_else(|| {
            conn.execute(
                "INSERT INTO devices (mac_address, device_name, rssi, first_seen, last_seen)
              VALUES (?, NULL, ?, datetime('now'), datetime('now'))",
                [mac_address, &rssi.to_string()],
            )
            .ok();

            conn.query_row(
                "SELECT id FROM devices WHERE mac_address = ?",
                [mac_address],
                |row| row.get(0),
            )
            .unwrap_or(0)
        });

    conn.execute(
        "INSERT INTO ble_advertisement_frames 
         (device_id, mac_address, rssi, advertising_data, phy, channel, frame_type, timestamp, timestamp_ms)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![device_id, mac_address, rssi, advertising_data_hex, phy, channel as i32, frame_type, timestamp_str, timestamp_ms as i64],
    )?;

    conn.execute(
        "UPDATE devices SET last_seen = ?, rssi = ? WHERE mac_address = ?",
        params![timestamp_str, rssi, mac_address],
    )?;

    // 📊 Update global RSSI trend analyzer for real-time trend tracking
    let timestamp = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(timestamp_ms as i64)
        .unwrap_or_else(|| chrono::Utc::now());

    if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        crate::rssi_trend_manager::GLOBAL_RSSI_MANAGER.update_rssi(mac_address, rssi, timestamp);
    })) {
        log::warn!("Failed to update RSSI trend: {:?}", e);
    }

    Ok(())
}

/// Get device by MAC using pooled connection
pub fn get_device_pooled(mac_address: &str) -> SqliteResult<Option<ScannedDevice>> {
    let pool = crate::db_pool::get_pool();
    if let Some(pool) = pool {
        pool.execute(|conn| get_device_inner(conn, mac_address))
    } else {
        get_device(mac_address)
    }
}

/// Inner function for get_device (used by pooled version)
fn get_device_inner(conn: &Connection, mac_address: &str) -> SqliteResult<Option<ScannedDevice>> {
    let mut stmt = conn.prepare(
        "SELECT mac_address, device_name, rssi, first_seen, last_seen, manufacturer_id, manufacturer_name, is_authenticated, device_class
         FROM devices
         WHERE mac_address = ?1"
    )?;

    stmt.query_row(params![mac_address], |row| {
        Ok(ScannedDevice {
            mac_address: row.get(0)?,
            name: row.get(1)?,
            rssi: row.get(2)?,
            first_seen: row.get(3)?,
            last_seen: row.get(4)?,
            manufacturer_id: row.get(5)?,
            manufacturer_name: row.get(6)?,
            mac_type: None,
            is_rpa: false,
            security_level: None,
            pairing_method: None,
            is_authenticated: row.get::<_, i32>(7).unwrap_or(0) != 0,
            device_class: row.get(8).ok(),
            service_classes: None,
            device_type: None,
            ..Default::default()
        })
    })
    .optional()
}

/// Get all devices using pooled connection
pub fn get_all_devices_pooled() -> SqliteResult<Vec<ScannedDevice>> {
    let pool = crate::db_pool::get_pool();
    if let Some(pool) = pool {
        pool.execute(|conn| get_all_devices_inner(conn))
    } else {
        get_all_devices()
    }
}

/// Inner function for get_all_devices (used by pooled version)
fn get_all_devices_inner(conn: &Connection) -> SqliteResult<Vec<ScannedDevice>> {
    let mut stmt = conn.prepare(
        "SELECT mac_address, device_name, rssi, first_seen, last_seen, manufacturer_id, manufacturer_name, is_authenticated, device_class
         FROM devices
         ORDER BY last_seen DESC"
    )?;

    let devices = stmt.query_map([], |row| {
        Ok(ScannedDevice {
            mac_address: row.get(0)?,
            name: row.get(1)?,
            rssi: row.get(2)?,
            first_seen: row.get(3)?,
            last_seen: row.get(4)?,
            manufacturer_id: row.get(5)?,
            manufacturer_name: row.get(6)?,
            mac_type: None,
            is_rpa: false,
            security_level: None,
            pairing_method: None,
            is_authenticated: row.get::<_, i32>(7).unwrap_or(0) != 0,
            device_class: row.get(8).ok(),
            service_classes: None,
            device_type: None,
            ..Default::default()
        })
    })?;

    devices.collect()
}

/// Get device count using pooled connection
pub fn get_device_count_pooled() -> SqliteResult<i32> {
    let pool = crate::db_pool::get_pool();
    if let Some(pool) = pool {
        pool.execute(|conn| conn.query_row("SELECT COUNT(*) FROM devices", [], |row| row.get(0)))
    } else {
        get_device_count()
    }
}

/// Get recent devices using pooled connection
pub fn get_recent_devices_pooled(minutes: u32) -> SqliteResult<Vec<ScannedDevice>> {
    let pool = crate::db_pool::get_pool();
    if let Some(pool) = pool {
        pool.execute(|conn| get_recent_devices_inner(conn, minutes))
    } else {
        get_recent_devices(minutes)
    }
}

/// Inner function for get_recent_devices (used by pooled version)
fn get_recent_devices_inner(conn: &Connection, minutes: u32) -> SqliteResult<Vec<ScannedDevice>> {
    let mut stmt = conn.prepare(
        "SELECT mac_address, device_name, rssi, first_seen, last_seen, manufacturer_id, manufacturer_name, is_authenticated, device_class
         FROM devices
         WHERE last_seen > datetime('now', ?1)
         ORDER BY last_seen DESC"
    )?;

    let time_filter = format!("-{} minutes", minutes);
    let devices = stmt.query_map(params![&time_filter], |row| {
        Ok(ScannedDevice {
            mac_address: row.get(0)?,
            name: row.get(1)?,
            rssi: row.get(2)?,
            first_seen: row.get(3)?,
            last_seen: row.get(4)?,
            manufacturer_id: row.get(5)?,
            manufacturer_name: row.get(6)?,
            mac_type: None,
            is_rpa: false,
            security_level: None,
            pairing_method: None,
            is_authenticated: row.get::<_, i32>(7).unwrap_or(0) != 0,
            device_class: row.get(8).ok(),
            service_classes: None,
            device_type: None,
            ..Default::default()
        })
    })?;

    devices.collect()
}

/// Determine signal quality based on RSSI value
/// dBm scale: -30 to -70 and beyond
/// excellent: >= -50 dBm
/// good: -50 to -60 dBm
/// fair: -60 to -70 dBm
/// poor: -70 to -85 dBm
/// very_poor: < -85 dBm
fn get_signal_quality(rssi: f64) -> String {
    match rssi as i32 {
        r if r >= -50 => "excellent".to_string(),
        r if r >= -60 => "good".to_string(),
        r if r >= -70 => "fair".to_string(),
        r if r >= -85 => "poor".to_string(),
        _ => "very_poor".to_string(),
    }
}

/// Get RSSI trend for a specific device over time
/// Returns all telemetry snapshots for the device, showing how signal quality changes
pub fn get_device_rssi_trend(device_mac: &str, hours: u32) -> SqliteResult<Vec<RssiTrendPoint>> {
    let conn = Connection::open(DB_PATH)?;
    let time_filter = format!("-{} hours", hours);

    let mut stmt = conn.prepare(
        "SELECT ts.snapshot_timestamp, dth.avg_rssi, dth.packet_count
         FROM device_telemetry_history dth
         JOIN telemetry_snapshots ts ON dth.snapshot_id = ts.id
         WHERE dth.device_mac = ?1 AND ts.snapshot_timestamp > datetime('now', ?2)
         ORDER BY ts.snapshot_timestamp ASC",
    )?;

    let trend_points = stmt.query_map(params![device_mac, &time_filter], |row| {
        let ts_str: String = row.get(0)?;
        let timestamp = DateTime::parse_from_rfc3339(&ts_str)
            .unwrap_or_else(|_| {
                chrono::Local::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
            })
            .with_timezone(&Utc);

        let avg_rssi: f64 = row.get(1)?;
        let signal_quality = get_signal_quality(avg_rssi);

        Ok(RssiTrendPoint {
            timestamp,
            avg_rssi,
            packet_count: row.get(2)?,
            signal_quality,
        })
    })?;

    trend_points.collect()
}

/// Get RSSI trend using pooled connection
pub fn get_device_rssi_trend_pooled(
    device_mac: &str,
    hours: u32,
) -> SqliteResult<Vec<RssiTrendPoint>> {
    let pool = crate::db_pool::get_pool();
    if let Some(pool) = pool {
        pool.execute(|_conn| get_device_rssi_trend(device_mac, hours))
    } else {
        get_device_rssi_trend(device_mac, hours)
    }
}

/// Get last N raw RSSI measurements from advertisement frames
/// Returns up to `limit` most recent RSSI readings for a device
pub fn get_raw_rssi_measurements(
    device_mac: &str,
    limit: u32,
) -> SqliteResult<Vec<RssiMeasurement>> {
    let conn = Connection::open(DB_PATH)?;

    let mut stmt = conn.prepare(
        "SELECT timestamp, rssi
         FROM ble_advertisement_frames
         WHERE mac_address = ?1
         ORDER BY timestamp DESC
         LIMIT ?2",
    )?;

    let measurements = stmt.query_map(params![device_mac, limit], |row| {
        let ts_str: String = row.get(0)?;
        let timestamp = DateTime::parse_from_rfc3339(&ts_str)
            .unwrap_or_else(|_| {
                chrono::Local::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
            })
            .with_timezone(&Utc);

        let rssi: i32 = row.get(1)?;
        let signal_quality = get_signal_quality(rssi as f64);

        Ok(RssiMeasurement {
            timestamp,
            rssi,
            signal_quality,
        })
    })?;

    let mut result: Vec<RssiMeasurement> = measurements.collect::<Result<Vec<_>, _>>()?;
    // Reverse to get chronological order (oldest first)
    result.reverse();
    Ok(result)
}

/// Get last N raw RSSI measurements using pooled connection
pub fn get_raw_rssi_measurements_pooled(
    device_mac: &str,
    limit: u32,
) -> SqliteResult<Vec<RssiMeasurement>> {
    let pool = crate::db_pool::get_pool();
    if let Some(pool) = pool {
        pool.execute(|_conn| get_raw_rssi_measurements(device_mac, limit))
    } else {
        get_raw_rssi_measurements(device_mac, limit)
    }
}
