use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};

const DB_PATH: &str = "bluetooth_scan.db";

#[derive(Debug, Clone)]
pub struct ScannedDevice {
    pub mac_address: String,
    pub name: Option<String>,
    pub rssi: i8,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub manufacturer_id: Option<u16>,
    pub manufacturer_name: Option<String>,
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
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Add column if not exists (for existing databases)
    conn.execute(
        "ALTER TABLE devices ADD COLUMN number_of_scan INTEGER DEFAULT 1",
        [],
    )
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
        return Ok(device_id);
    }

    // If no update, insert new device
    conn.execute(
        "INSERT INTO devices (mac_address, device_name, rssi, first_seen, last_seen, manufacturer_id, manufacturer_name, number_of_scan)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
        params![
            &device.mac_address,
            &device.name,
            device.rssi,
            device.first_seen,
            now,
            device.manufacturer_id,
            &device.manufacturer_name,
        ],
    )?;

    let device_id = conn.last_insert_rowid() as i32;
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
        "SELECT mac_address, device_name, rssi, first_seen, last_seen, manufacturer_id, manufacturer_name
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
        })
    })?;

    devices.collect()
}

/// Get device by MAC address
pub fn get_device(mac_address: &str) -> SqliteResult<Option<ScannedDevice>> {
    let conn = Connection::open(DB_PATH)?;
    let mut stmt = conn.prepare(
        "SELECT mac_address, device_name, rssi, first_seen, last_seen, manufacturer_id, manufacturer_name
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
        "SELECT mac_address, device_name, rssi, first_seen, last_seen, manufacturer_id, manufacturer_name
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
