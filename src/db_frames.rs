#![allow(dead_code)]

/// Database operations for raw Bluetooth frames and packets
/// Stores complete advertising packets with metadata for analysis

use chrono::{DateTime, Utc};

use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use std::collections::HashMap;
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

/// Save raw packets from unified scan to database
pub fn insert_raw_packets_from_scan(
    conn: &Connection,
    raw_packets: &[crate::data_models::RawPacketModel],
) -> SqliteResult<()> {
    if raw_packets.is_empty() {
        log::debug!("No raw packets to insert");
        return Ok(());
    }

    log::info!("💾 Inserting {} raw packets to database", raw_packets.len());

    let mut device_id_cache: HashMap<String, i64> = HashMap::new();
    let mut lookup_stmt = conn.prepare("SELECT id FROM devices WHERE mac_address = ?1")?;
    let mut insert_stmt = conn.prepare(
        "INSERT INTO ble_advertisement_frames
         (device_id, mac_address, rssi, advertising_data, phy, channel, frame_type, timestamp)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )?;
    let mut missing_device_count = 0;
    let mut inserted_count = 0;

    for packet in raw_packets {
        let device_id = if let Some(id) = device_id_cache.get(&packet.mac_address) {
            *id
        } else {
            let id_opt: Option<i64> = lookup_stmt
                .query_row(params![&packet.mac_address], |row| row.get(0))
                .optional()?;
            match id_opt {
                Some(id) => {
                    device_id_cache.insert(packet.mac_address.clone(), id);
                    id
                }
                None => {
                    missing_device_count += 1;
                    continue;
                }
            }
        };

        let advertising_data_hex = hex::encode(&packet.advertising_data);

        insert_stmt.execute(params![
            device_id,
            &packet.mac_address,
            packet.rssi,
            advertising_data_hex,
            &packet.phy,
            packet.channel as i32,
            &packet.packet_type,
            packet.timestamp,
        ])?;
        inserted_count += 1;
    }

    if missing_device_count > 0 {
        log::warn!(
            "Skipped {} raw packets without a matching device row",
            missing_device_count
        );
    }

    log::info!("✅ Successfully inserted {} raw packets", inserted_count);
    Ok(())
}

/// Insert raw packets from parser output (text format)
pub fn insert_parsed_raw_packets(
    conn: &Connection,
    raw_packets: &[crate::raw_packet_parser::RawPacketData],
) -> SqliteResult<()> {
    if raw_packets.is_empty() {
        log::debug!("No parsed raw packets to insert");
        return Ok(());
    }

    log::info!("💾 Inserting {} parsed raw packets to database", raw_packets.len());

    let mut stmt = conn.prepare(
        "INSERT INTO ble_advertisement_frames
         (mac_address, rssi, advertising_data, phy, channel, frame_type, timestamp)
         VALUES (?, ?, ?, ?, ?, ?, ?)"
    )?;

    let now = chrono::Local::now().to_rfc3339();

    for packet in raw_packets {
        let advertising_data_hex = &packet.manufacturer_data_hex;
        let frame_type = if packet.connectable { "ADV_IND" } else { "ADV_NONCONN_IND" };

        stmt.execute(params![
            &packet.mac_address,
            packet.rssi as i32,
            advertising_data_hex,
            "LE 1M",
            37i32,
            frame_type,
            now,
        ])?;

        log::debug!("📦 Inserted packet: {} RSSI:{} Company:{:?}",
                   packet.mac_address,
                   packet.rssi,
                   packet.company_name);
    }

    log::info!("✅ Successfully inserted {} parsed raw packets", raw_packets.len());
    Ok(())
}

/// Insert raw packets from batch processor
pub fn insert_raw_packet_batch(
    conn: &Connection,
    batch: &mut crate::raw_packet_parser::RawPacketBatchProcessor,
) -> SqliteResult<()> {
    let raw_packets = batch.process_all();
    insert_raw_packets_from_scan(conn, &raw_packets)
}

/// Store raw packet statistics
pub fn store_packet_statistics(
    conn: &Connection,
    stats: &crate::raw_packet_parser::RawPacketStatistics,
    scan_session_id: &str,
) -> SqliteResult<()> {
    // Create table if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS raw_packet_statistics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scan_session_id TEXT NOT NULL,
            total_packets INTEGER,
            unique_macs INTEGER,
            connectable_count INTEGER,
            non_connectable_count INTEGER,
            with_tx_power INTEGER,
            with_company_data INTEGER,
            min_rssi INTEGER,
            max_rssi INTEGER,
            avg_rssi REAL,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    let mut stmt = conn.prepare(
        "INSERT INTO raw_packet_statistics
         (scan_session_id, total_packets, unique_macs, connectable_count,
          non_connectable_count, with_tx_power, with_company_data, min_rssi, max_rssi, avg_rssi)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )?;

    stmt.execute(params![
        scan_session_id,
        stats.total_packets as i32,
        stats.unique_macs as i32,
        stats.connectable_count as i32,
        stats.non_connectable_count as i32,
        stats.with_tx_power as i32,
        stats.with_company_data as i32,
        stats.min_rssi as i32,
        stats.max_rssi as i32,
        stats.avg_rssi,
    ])?;

    log::info!("📊 Stored statistics for scan session: {}", scan_session_id);
    Ok(())
}

/// Get raw packets by MAC address from database
pub fn get_raw_packets_by_mac(
    conn: &Connection,
    mac_address: &str,
    limit: usize,
) -> SqliteResult<Vec<(String, i32, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT mac_address, rssi, advertising_data, timestamp
         FROM ble_advertisement_frames
         WHERE mac_address = ?
         ORDER BY timestamp DESC
         LIMIT ?"
    )?;

    let packets = stmt.query_map(params![mac_address, limit as i32], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i32>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    let mut result = Vec::new();
    for packet in packets {
        result.push(packet?);
    }

    Ok(result)
}

/// Get packet statistics summary
pub fn get_packet_statistics_summary(
    conn: &Connection,
) -> SqliteResult<Option<(i32, i32, f64)>> {
    let mut stmt = conn.prepare(
        "SELECT SUM(total_packets), COUNT(DISTINCT scan_session_id), AVG(avg_rssi)
         FROM raw_packet_statistics"
    )?;

    let result = stmt.query_row([], |row| {
        Ok((
            row.get::<_, i32>(0)?,
            row.get::<_, i32>(1)?,
            row.get::<_, f64>(2)?,
        ))
    });

    match result {
        Ok(data) => Ok(Some(data)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
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
