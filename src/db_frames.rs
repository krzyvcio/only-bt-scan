#![allow(dead_code)]

/// Database operations for raw Bluetooth frames and packets
/// Stores complete advertising packets with metadata for analysis

use chrono::{DateTime, Utc};

use rusqlite::{params, Connection, Result as SqliteResult};
use crate::raw_sniffer::{BluetoothFrame, BluetoothPhy, AdvertisingType};

/// Initialize frame storage tables in the database
pub fn init_frame_storage(conn: &Connection) -> SqliteResult<()> {
    // Raw Bluetooth advertisement frames
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ble_advertisement_frames (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id INTEGER NOT NULL,
            mac_address TEXT NOT NULL,
            rssi INTEGER NOT NULL,
            advertising_data BLOB NOT NULL,
            phy TEXT NOT NULL,
            channel INTEGER NOT NULL,
            frame_type TEXT NOT NULL,
            timestamp DATETIME NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(device_id) REFERENCES devices(id)
        )",
        [],
    )?;

    // Create index for fast queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_frames_mac_timestamp 
         ON ble_advertisement_frames(mac_address, timestamp DESC)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_frames_device_timestamp 
         ON ble_advertisement_frames(device_id, timestamp DESC)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_frames_timestamp 
         ON ble_advertisement_frames(timestamp DESC)",
        [],
    )?;

    // Frame statistics (updated periodically)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS frame_statistics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            mac_address TEXT NOT NULL UNIQUE,
            total_frames INTEGER DEFAULT 0,
            average_rssi REAL,
            strongest_signal INTEGER,
            weakest_signal INTEGER,
            phy_1m_count INTEGER DEFAULT 0,
            phy_2m_count INTEGER DEFAULT 0,
            phy_coded_count INTEGER DEFAULT 0,
            adv_ind_count INTEGER DEFAULT 0,
            scan_resp_count INTEGER DEFAULT 0,
            last_updated DATETIME NOT NULL
        )",
        [],
    )?;

    Ok(())
}

/// Store a single Bluetooth frame
pub fn insert_frame(
    conn: &Connection,
    device_id: i64,
    frame: &BluetoothFrame,
) -> SqliteResult<()> {
    let phy_str = format!("{}", frame.phy);
    let frame_type_str = format!("{}", frame.frame_type);
    let advertising_data_hex = hex::encode(&frame.advertising_data);

    conn.execute(
        "INSERT INTO ble_advertisement_frames 
         (device_id, mac_address, rssi, advertising_data, phy, channel, frame_type, timestamp)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            device_id,
            &frame.mac_address,
            frame.rssi,
            advertising_data_hex,
            phy_str,
            frame.channel,
            frame_type_str,
            frame.timestamp,
        ],
    )?;

    Ok(())
}

/// Bulk insert multiple frames
pub fn insert_frames_batch(
    conn: &Connection,
    device_id: i64,
    frames: &[BluetoothFrame],
) -> SqliteResult<()> {
    let mut stmt = conn.prepare(
        "INSERT INTO ble_advertisement_frames 
         (device_id, mac_address, rssi, advertising_data, phy, channel, frame_type, timestamp)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )?;

    for frame in frames {
        let phy_str = format!("{}", frame.phy);
        let frame_type_str = format!("{}", frame.frame_type);
        let advertising_data_hex = hex::encode(&frame.advertising_data);

        stmt.execute(params![
            device_id,
            &frame.mac_address,
            frame.rssi,
            advertising_data_hex,
            phy_str,
            frame.channel,
            frame_type_str,
            frame.timestamp,
        ])?;
    }

    Ok(())
}

/// Get frames for a specific MAC address
pub fn get_frames_by_mac(
    conn: &Connection,
    mac_address: &str,
    limit: Option<i64>,
) -> SqliteResult<Vec<BluetoothFrame>> {
    let query = if let Some(l) = limit {
        format!(
            "SELECT mac_address, rssi, advertising_data, phy, channel, \
             frame_type, timestamp FROM ble_advertisement_frames \
             WHERE mac_address = ?1 ORDER BY timestamp DESC LIMIT {}",
            l
        )
    } else {
        "SELECT mac_address, rssi, advertising_data, phy, channel, \
         frame_type, timestamp FROM ble_advertisement_frames \
         WHERE mac_address = ? ORDER BY timestamp DESC"
            .to_string()
    };

    let mut stmt = conn.prepare(&query)?;
    let frames = stmt
        .query_map([mac_address], |row| {
            Ok(BluetoothFrame {
                mac_address: row.get(0)?,
                rssi: row.get(1)?,
                advertising_data: hex::decode(row.get::<_, String>(2)?)
                    .unwrap_or_default(),
                phy: parse_phy(&row.get::<_, String>(3)?),
                channel: row.get(4)?,
                frame_type: parse_frame_type(&row.get::<_, String>(5)?),
                timestamp: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(frames)
}

/// Get frames in a time range
pub fn get_frames_by_time_range(
    conn: &Connection,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> SqliteResult<Vec<BluetoothFrame>> {
    let mut stmt = conn.prepare(
        "SELECT mac_address, rssi, advertising_data, phy, channel, frame_type, timestamp 
         FROM ble_advertisement_frames 
         WHERE timestamp >= ? AND timestamp <= ? 
         ORDER BY timestamp DESC"
    )?;

    let frames = stmt
        .query_map(params![start, end], |row| {
            Ok(BluetoothFrame {
                mac_address: row.get(0)?,
                rssi: row.get(1)?,
                advertising_data: hex::decode(row.get::<_, String>(2)?)
                    .unwrap_or_default(),
                phy: parse_phy(&row.get::<_, String>(3)?),
                channel: row.get(4)?,
                frame_type: parse_frame_type(&row.get::<_, String>(5)?),
                timestamp: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(frames)
}

/// Get frames by advertising type
pub fn get_frames_by_type(
    conn: &Connection,
    frame_type: AdvertisingType,
) -> SqliteResult<Vec<BluetoothFrame>> {
    let frame_type_str = format!("{}", frame_type);
    let mut stmt = conn.prepare(
        "SELECT mac_address, rssi, advertising_data, phy, channel, frame_type, timestamp 
         FROM ble_advertisement_frames 
         WHERE frame_type = ? 
         ORDER BY timestamp DESC LIMIT 1000"
    )?;

    let frames = stmt
        .query_map([frame_type_str], |row| {
            Ok(BluetoothFrame {
                mac_address: row.get(0)?,
                rssi: row.get(1)?,
                advertising_data: hex::decode(row.get::<_, String>(2)?)
                    .unwrap_or_default(),
                phy: parse_phy(&row.get::<_, String>(3)?),
                channel: row.get(4)?,
                frame_type: parse_frame_type(&row.get::<_, String>(5)?),
                timestamp: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(frames)
}

/// Count frames for a device
pub fn count_frames_for_device(conn: &Connection, device_id: i64) -> SqliteResult<i64> {
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM ble_advertisement_frames WHERE device_id = ?"
    )?;

    stmt.query_row([device_id], |row| row.get(0))
}

/// Delete old frames (cleanup)
pub fn delete_old_frames(conn: &Connection, days: i64) -> SqliteResult<usize> {
    conn.execute(
        "DELETE FROM ble_advertisement_frames 
         WHERE timestamp < datetime('now', '-' || ? || ' days')",
        [days],
    )
}

/// Helper: Parse PHY from string
fn parse_phy(phy_str: &str) -> BluetoothPhy {
    match phy_str {
        "LE 1M" => BluetoothPhy::Le1M,
        "LE 2M" => BluetoothPhy::Le2M,
        "LE Coded (S=2)" => BluetoothPhy::LeCodedS2,
        "LE Coded (S=8)" => BluetoothPhy::LeCodedS8,
        "BR/EDR" => BluetoothPhy::BrEdr,
        _ => BluetoothPhy::Unknown,
    }
}

/// Helper: Parse frame type from string
fn parse_frame_type(frame_type_str: &str) -> AdvertisingType {
    match frame_type_str {
        "ADV_IND" => AdvertisingType::Adv_Ind,
        "ADV_DIRECT_IND" => AdvertisingType::Adv_Direct_Ind,
        "ADV_NONCONN_IND" => AdvertisingType::Adv_Nonconn_Ind,
        "ADV_SCAN_IND" => AdvertisingType::Adv_Scan_Ind,
        "SCAN_RSP" => AdvertisingType::Scan_Rsp,
        "EXT_ADV_IND" => AdvertisingType::Ext_Adv_Ind,
        _ => AdvertisingType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_phy() {
        assert_eq!(parse_phy("LE 1M"), BluetoothPhy::Le1M);
        assert_eq!(parse_phy("Unknown"), BluetoothPhy::Unknown);
    }

    #[test]
    fn test_parse_frame_type() {
        assert_eq!(parse_frame_type("ADV_IND"), AdvertisingType::Adv_Ind);
        assert_eq!(parse_frame_type("UNKNOWN"), AdvertisingType::Unknown);
    }
}
