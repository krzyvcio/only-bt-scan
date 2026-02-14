# 📋 Manual Raw Packet Processing Guide

## Overview

This guide explains how to manually parse raw Bluetooth packets from text format and store them in the database.

## Quick Start

### 1. Parse Raw Packet Text

```rust
use raw_packet_parser::{RawPacketParser, RawPacketBatchProcessor};

// Single packet parsing
let parser = RawPacketParser::new();
let line = r#"14:0e:90:a4:b3:90 "" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)"#;

if let Some(packet) = parser.parse_packet(line) {
    println!("Parsed: {} - RSSI: {}", packet.mac_address, packet.rssi);
}
```

### 2. Process Batch of Packets

```rust
let mut processor = RawPacketBatchProcessor::new();

// Add multiple lines of raw packet data
let raw_data = r#"
14:0e:90:a4:b3:90 "" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)
AA:BB:CC:DD:EE:FF "TestDevice" -75dB tx=5 Connectable Paired company-id=0x004C manuf-data=020106 (Apple)
"#;

processor.add_raw_text(raw_data);
let models = processor.process_all();

// Get statistics
let stats = processor.get_statistics();
println!("Total packets: {}", stats.total_packets);
println!("Unique MACs: {}", stats.unique_macs);
println!("Avg RSSI: {:.1} dBm", stats.avg_rssi);
```

### 3. Save to Database

```rust
use db_frames;

let conn = rusqlite::Connection::open("bluetooth_scan.db")?;

// Save parsed packets
db_frames::insert_parsed_raw_packets(&conn, &processor.packets)?;

// Save statistics
let scan_id = uuid::Uuid::new_v4().to_string();
db_frames::store_packet_statistics(&conn, &processor.get_statistics(), &scan_id)?;

log::info!("✅ Saved {} packets to database", processor.packets.len());
```

---

## Raw Packet Format

### Standard Format
```
MAC_ADDRESS "DEVICE_NAME" RSSI_VALUE TX_POWER CONNECTABLE PAIRED company-id=0xXXXX manuf-data=HEXDATA (COMPANY_NAME)
```

### Field Descriptions

| Field | Type | Example | Description |
|-------|------|---------|-------------|
| MAC_ADDRESS | String | 14:0e:90:a4:b3:90 | Bluetooth MAC address (colon-separated) |
| DEVICE_NAME | String | "TestDevice" | Device name (empty string "" allowed) |
| RSSI_VALUE | Integer | -82dB | Signal strength in dBm |
| TX_POWER | Integer or n/a | tx=n/a | Transmit power level (optional) |
| CONNECTABLE | Flag | Connectable or Non-Connectable | Device connectivity |
| PAIRED | Flag | Paired or Non-Paired | Pairing status |
| company-id | Hex | 0x0006 | Manufacturer ID (IEEE format) |
| manuf-data | Hex String | 0109202231AF... | Manufacturer-specific data |
| COMPANY_NAME | String | (Microsoft) | Manufacturer name (optional) |

### Examples

#### Microsoft Swift Pair Device
```
14:0e:90:a4:b3:90 "" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)
```

#### Apple iBeacon
```
AA:BB:CC:DD:EE:FF "Apple iBeacon" -65dB tx=5 Non-Connectable Non-Paired company-id=0x004C manuf-data=020106030334A2A4 (Apple)
```

#### Generic BLE Device
```
11:22:33:44:55:66 "MyDevice" -70dB tx=10 Connectable Paired company-id=0x004C manuf-data=020106 (Apple)
```

---

## Supported Manufacturer IDs

Common manufacturer IDs (company-id values):

```
0x0006  - Microsoft
0x004C  - Apple
0x0059  - Google
0x0075  - Fitbit
0x0077  - SRAM
0x00E0  - Google LLC
0x00FF  - Beacon/Proprietary
```

---

## Integration Examples

### Example 1: Load from File

```rust
use std::fs;

pub fn load_raw_packets_from_file(path: &str) -> Result<Vec<RawPacketData>> {
    let content = fs::read_to_string(path)?;
    
    let parser = RawPacketParser::new();
    let packets = parser.parse_packets(&content);
    
    log::info!("Loaded {} packets from {}", packets.len(), path);
    Ok(packets)
}

// Usage
let packets = load_raw_packets_from_file("packets.txt")?;
db_frames::insert_parsed_raw_packets(&conn, &packets)?;
```

### Example 2: Real-time Processing

```rust
use std::io::{self, BufRead};

pub async fn process_real_time_packets() -> Result<()> {
    let mut processor = RawPacketBatchProcessor::new();
    let stdin = io::stdin();
    
    log::info!("Waiting for raw packet input (Ctrl+D to finish)...");
    
    for line in stdin.lock().lines() {
        let line = line?;
        processor.add_raw_text(&line);
        
        // Process every 10 packets
        if processor.packets.len() >= 10 {
            let models = processor.process_all();
            db_frames::insert_raw_packets_from_scan(&conn, &models)?;
            processor.clear();
        }
    }
    
    // Process remaining packets
    let models = processor.process_all();
    db_frames::insert_raw_packets_from_scan(&conn, &models)?;
    
    Ok(())
}
```

### Example 3: Deduplication

```rust
pub fn deduplicate_and_save(
    raw_packets: &[RawPacketData],
    conn: &Connection,
) -> Result<()> {
    let mut processor = RawPacketBatchProcessor::new();
    
    for packet in raw_packets {
        processor.packets.push(packet.clone());
    }
    
    processor.process_all();
    
    // Get unique packets by MAC (keeps most recent RSSI)
    let dedup = processor.deduplicate_by_mac();
    
    log::info!("Deduplicated {} packets to {} unique devices",
               raw_packets.len(),
               dedup.len());
    
    db_frames::insert_raw_packets_from_scan(conn, &dedup)?;
    
    Ok(())
}
```

### Example 4: Filter by Company

```rust
pub fn filter_and_process(
    raw_packets: &[RawPacketData],
    target_company: &str,
) -> Vec<RawPacketData> {
    raw_packets.iter()
        .filter(|p| p.company_name.as_deref() == Some(target_company))
        .cloned()
        .collect()
}

// Usage
let microsoft_packets = filter_and_process(&packets, "Microsoft");
println!("Found {} Microsoft packets", microsoft_packets.len());
```

### Example 5: Statistics and Analysis

```rust
pub fn analyze_packets(packets: &[RawPacketData]) {
    let mut processor = RawPacketBatchProcessor::new();
    processor.packets = packets.to_vec();
    
    let stats = processor.get_statistics();
    
    println!("═══ RAW PACKET ANALYSIS ═══");
    println!("Total Packets: {}", stats.total_packets);
    println!("Unique Devices: {}", stats.unique_macs);
    println!("Signal Strength:");
    println!("  Min: {} dBm", stats.min_rssi);
    println!("  Max: {} dBm", stats.max_rssi);
    println!("  Avg: {:.1} dBm", stats.avg_rssi);
    println!("Device Status:");
    println!("  Connectable: {}", stats.connectable_count);
    println!("  Non-Connectable: {}", stats.non_connectable_count);
    println!("Advanced Features:");
    println!("  With TX Power: {}", stats.with_tx_power);
    println!("  With Company Data: {}", stats.with_company_data);
}
```

---

## Web API Integration

### Add Endpoint for Raw Packet Upload

```rust
// In web_server.rs

#[derive(Deserialize)]
pub struct RawPacketUpload {
    pub raw_text: String,
}

pub async fn upload_raw_packets(
    payload: web::Json<RawPacketUpload>,
) -> impl Responder {
    let parser = raw_packet_parser::RawPacketParser::new();
    let packets = parser.parse_packets(&payload.raw_text);
    
    if packets.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "error": "No packets parsed from input"
        }));
    }
    
    match rusqlite::Connection::open("bluetooth_scan.db") {
        Ok(conn) => {
            match db_frames::insert_parsed_raw_packets(&conn, &packets) {
                Ok(_) => {
                    log::info!("✅ Uploaded {} raw packets", packets.len());
                    HttpResponse::Ok().json(json!({
                        "success": true,
                        "packets_uploaded": packets.len(),
                        "unique_devices": packets.iter()
                            .map(|p| &p.mac_address)
                            .collect::<std::collections::HashSet<_>>()
                            .len()
                    }))
                },
                Err(e) => {
                    log::error!("Failed to insert packets: {}", e);
                    HttpResponse::InternalServerError().json(json!({
                        "error": format!("Database error: {}", e)
                    }))
                }
            }
        },
        Err(e) => {
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Database connection error: {}", e)
            }))
        }
    }
}

// In configure_services():
.route("/api/raw-packets/upload", web::post().to(upload_raw_packets))
```

### Frontend Form

```html
<div class="raw-packet-upload">
    <h3>📦 Upload Raw Packets</h3>
    <form id="packet-upload-form">
        <textarea id="packet-input" placeholder="Paste raw packet data here..." rows="10"></textarea>
        <button type="submit" class="btn">Upload Packets</button>
    </form>
    <div id="upload-result"></div>
</div>

<script>
document.getElementById('packet-upload-form').addEventListener('submit', async (e) => {
    e.preventDefault();
    
    const rawText = document.getElementById('packet-input').value;
    const result = document.getElementById('upload-result');
    
    try {
        const response = await fetch('/api/raw-packets/upload', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ raw_text: rawText })
        });
        
        const data = await response.json();
        
        if (response.ok) {
            result.innerHTML = `
                ✅ Successfully uploaded ${data.packets_uploaded} packets
                from ${data.unique_devices} unique devices
            `;
            document.getElementById('packet-input').value = '';
        } else {
            result.innerHTML = `❌ Error: ${data.error}`;
        }
    } catch (error) {
        result.innerHTML = `❌ Upload failed: ${error.message}`;
    }
});
</script>
```

---

## Database Queries

### View Stored Raw Packets

```sql
-- Most recent packets from all devices
SELECT mac_address, rssi, advertising_data, timestamp
FROM ble_advertisement_frames
ORDER BY timestamp DESC
LIMIT 50;

-- Packets from specific device
SELECT mac_address, rssi, advertising_data, timestamp
FROM ble_advertisement_frames
WHERE mac_address = 'AA:BB:CC:DD:EE:FF'
ORDER BY timestamp DESC
LIMIT 100;

-- Statistics by company
SELECT 
    COUNT(*) as packet_count,
    AVG(rssi) as avg_rssi,
    MIN(rssi) as min_rssi,
    MAX(rssi) as max_rssi
FROM ble_advertisement_frames
WHERE advertising_data LIKE '0006%'
GROUP BY mac_address;

-- Packet statistics summary
SELECT 
    total_packets,
    unique_macs,
    connectable_count,
    non_connectable_count,
    avg_rssi
FROM raw_packet_statistics
ORDER BY timestamp DESC
LIMIT 10;
```

---

## Logging and Debugging

### Enable Debug Logging

```rust
// In your code
log::debug!("Parsing packet: {}", line);
log::info!("Successfully parsed {} packets", count);
log::warn!("Failed to parse: {}", line);
log::error!("Database error: {}", error);
```

### Log Environment Variable

```bash
RUST_LOG=debug cargo run
RUST_LOG=raw_packet_parser=debug cargo run
RUST_LOG=db_frames=debug cargo run
```

---

## Performance Tips

1. **Batch Processing**: Process packets in batches of 100+ for better performance
2. **Deduplication**: Use `deduplicate_by_mac()` to reduce database size
3. **Compression**: Store hex data as BLOB for better compression
4. **Indexing**: Database has indices on MAC address and timestamp
5. **Cleanup**: Periodically delete old packets using `delete_old_frames()`

---

## Troubleshooting

### Parser returns None
- Check MAC address format (must be XX:XX:XX:XX:XX:XX)
- Verify RSSI is in format "-XXdB"
- Ensure company-id starts with "0x"

### Database insertion fails
- Check database file is writable
- Verify schema is initialized with `db_frames::init_frame_storage()`
- Check available disk space

### Statistics are incorrect
- Ensure all packets are processed with `process_all()`
- Verify no packets were filtered out
- Check for duplicate MACs

---

## Complete Example Program

```rust
use only_bt_scan::raw_packet_parser::*;
use only_bt_scan::db_frames;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    let raw_data = r#"
14:0e:90:a4:b3:90 "" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)
AA:BB:CC:DD:EE:FF "TestDevice" -75dB tx=5 Connectable Paired company-id=0x004C manuf-data=020106 (Apple)
11:22:33:44:55:66 "" -90dB tx=n/a Non-Connectable Non-Paired company-id=0x0059 manuf-data=0102030405 (Google)
    "#;
    
    // Parse packets
    let mut processor = RawPacketBatchProcessor::new();
    processor.add_raw_text(raw_data);
    let models = processor.process_all();
    
    // Show statistics
    let stats = processor.get_statistics();
    println!("Parsed {} packets from {} devices", stats.total_packets, stats.unique_macs);
    println!("RSSI range: {} to {} dBm", stats.min_rssi, stats.max_rssi);
    
    // Save to database
    let conn = rusqlite::Connection::open("bluetooth_scan.db")?;
    db_frames::insert_raw_packets_from_scan(&conn, &models)?;
    db_frames::store_packet_statistics(&conn, &stats, &uuid::Uuid::new_v4().to_string())?;
    
    println!("✅ Successfully stored packets in database");
    
    Ok(())
}
```

---

## See Also

- `raw_packet_parser.rs` - Parser implementation and unit tests
- `db_frames.rs` - Database storage functions
- `data_models.rs` - RawPacketModel structure
- Raw Packet Guide (RAW_PACKET_GUIDE.md)