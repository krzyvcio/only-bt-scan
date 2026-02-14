# 📚 Raw Packet System - Quick Reference

## 🚀 Quick Start (30 seconds)

```rust
// 1. Parse raw packets
let parser = RawPacketParser::new();
let packets = parser.parse_packets(raw_text);

// 2. Process & get stats
let mut processor = RawPacketBatchProcessor::new();
processor.add_raw_text(raw_text);
let models = processor.process_all();
let stats = processor.get_statistics();

// 3. Store in database
let conn = rusqlite::Connection::open("bluetooth_scan.db")?;
db_frames::insert_raw_packets_from_scan(&conn, &models)?;
db_frames::store_packet_statistics(&conn, &stats, "session-1")?;
```

## 📋 Input Format

```
MAC_ADDR "NAME" RSSI TX STATUS company-id=0xID manuf-data=HEX (COMPANY)
```

**Example:**
```
14:0e:90:a4:b3:90 "" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)
```

## 🔧 Core Classes

### RawPacketParser
```rust
pub struct RawPacketParser { ... }

impl RawPacketParser {
    pub fn new() -> Self
    pub fn parse_packet(&self, line: &str) -> Option<RawPacketData>
    pub fn parse_packets(&self, input: &str) -> Vec<RawPacketData>
    pub fn to_raw_packet_model(&self, packet: &RawPacketData, id: u64) -> RawPacketModel
}
```

### RawPacketBatchProcessor
```rust
pub struct RawPacketBatchProcessor { ... }

impl RawPacketBatchProcessor {
    pub fn new() -> Self
    pub fn add_raw_text(&mut self, text: &str)
    pub fn process_all(&mut self) -> Vec<RawPacketModel>
    pub fn deduplicate_by_mac(&self) -> Vec<RawPacketModel>
    pub fn get_statistics(&self) -> RawPacketStatistics
    pub fn clear(&mut self)
}
```

### RawPacketData
```rust
pub struct RawPacketData {
    pub mac_address: String,
    pub device_name: Option<String>,
    pub rssi: i8,
    pub tx_power: Option<i8>,
    pub connectable: bool,
    pub paired: bool,
    pub company_id: Option<u16>,
    pub company_name: Option<String>,
    pub manufacturer_data: Vec<u8>,
    pub manufacturer_data_hex: String,
}
```

### RawPacketStatistics
```rust
pub struct RawPacketStatistics {
    pub total_packets: usize,
    pub unique_macs: usize,
    pub connectable_count: usize,
    pub non_connectable_count: usize,
    pub with_tx_power: usize,
    pub with_company_data: usize,
    pub min_rssi: i8,
    pub max_rssi: i8,
    pub avg_rssi: f64,
}
```

## 💾 Database Functions

### Insert Packets
```rust
db_frames::insert_parsed_raw_packets(&conn, &packets)?;
db_frames::insert_raw_packets_from_scan(&conn, &models)?;
db_frames::insert_raw_packet_batch(&conn, &processor)?;
```

### Store Statistics
```rust
db_frames::store_packet_statistics(&conn, &stats, "scan-id")?;
```

### Query Data
```rust
let packets = db_frames::get_raw_packets_by_mac(&conn, "AA:BB:CC:DD:EE:FF", 100)?;
let summary = db_frames::get_packet_statistics_summary(&conn)?;
```

## 🔍 Common Tasks

### Task 1: Parse Single Line
```rust
let parser = RawPacketParser::new();
if let Some(packet) = parser.parse_packet(line) {
    println!("MAC: {}", packet.mac_address);
    println!("RSSI: {}", packet.rssi);
    println!("Company: {:?}", packet.company_name);
}
```

### Task 2: Parse File
```rust
let content = std::fs::read_to_string("packets.txt")?;
let parser = RawPacketParser::new();
let packets = parser.parse_packets(&content);
println!("Parsed {} packets", packets.len());
```

### Task 3: Get Unique Devices
```rust
let mut processor = RawPacketBatchProcessor::new();
processor.add_raw_text(raw_data);
processor.process_all();
let unique = processor.deduplicate_by_mac();
println!("Found {} unique devices", unique.len());
```

### Task 4: Find Best Signal
```rust
let stats = processor.get_statistics();
println!("Best signal: {} dBm", stats.max_rssi);
println!("Weakest: {} dBm", stats.min_rssi);
println!("Average: {:.1} dBm", stats.avg_rssi);
```

### Task 5: Filter by Company
```rust
let microsoft_packets: Vec<_> = packets.iter()
    .filter(|p| p.company_name.as_deref() == Some("Microsoft"))
    .collect();
println!("Found {} Microsoft packets", microsoft_packets.len());
```

### Task 6: Save to Database
```rust
let conn = rusqlite::Connection::open("bluetooth_scan.db")?;
db_frames::init_frame_storage(&conn)?;
db_frames::insert_parsed_raw_packets(&conn, &packets)?;
db_frames::store_packet_statistics(&conn, &stats, "scan-1")?;
```

## 📊 Manufacturer IDs

| ID | Company | Example |
|----|---------|---------|
| 0x0006 | Microsoft | Swift Pair |
| 0x004C | Apple | iBeacon |
| 0x0059 | Google | Fast Pair |
| 0x0075 | Fitbit | Tracker |
| 0x0077 | SRAM | AirTag |
| 0x00E0 | Google LLC | Nearby |

## 🌐 Web API

### POST /api/raw-packets/upload
```json
{
  "raw_text": "14:0e:90:a4:b3:90 \"\" -82dB tx=n/a ..."
}
```

**Response:**
```json
{
  "success": true,
  "packets_uploaded": 5,
  "unique_devices": 3
}
```

### GET /api/raw-packets/by-mac/{mac}
Returns array of packets for given MAC address.

### GET /api/raw-packets/statistics
Returns summary statistics from database.

## 🧪 Testing

### Run Unit Tests
```bash
cargo test --lib raw_packet_parser
```

### Run Example
```bash
cargo run --example parse_raw_packets
```

### Available Tests
- `test_parse_single_packet`
- `test_parse_multiple_packets`
- `test_convert_to_raw_packet_model`
- `test_batch_processor`
- `test_deduplication`
- `test_statistics`

## 📈 Performance Tips

1. **Batch Processing**: Process 100+ packets per transaction
2. **Deduplication**: Use `deduplicate_by_mac()` to reduce storage
3. **Indexing**: Database automatically indexed on MAC + timestamp
4. **Cleanup**: Delete old packets with `delete_old_frames()`
5. **Compression**: BLOB format saves ~30% vs hex strings

## ⚠️ Common Errors

### Parser returns None
- Check MAC format: `XX:XX:XX:XX:XX:XX`
- Check RSSI format: `-XXdB`
- Check company ID: `0xXXXX`

### Database insertion fails
- Call `db_frames::init_frame_storage(&conn)` first
- Check file permissions
- Verify disk space

### Wrong statistics
- Call `process_all()` before `get_statistics()`
- Ensure packets weren't filtered

## 🔗 Module Imports

```rust
use crate::raw_packet_parser::{
    RawPacketParser,
    RawPacketData,
    RawPacketBatchProcessor,
    RawPacketStatistics,
};
use crate::db_frames;
use crate::data_models::RawPacketModel;
```

## 📚 Documentation

- **User Guide**: `MANUAL_RAW_PACKET_PROCESSING.md`
- **System Architecture**: `RAW_PACKET_SYSTEM.md`
- **Implementation**: `RAW_PACKET_IMPLEMENTATION_SUMMARY.md`
- **Examples**: `examples/parse_raw_packets.rs`
- **Source Code**: `src/raw_packet_parser.rs`

## 🎯 Complete Workflow

```
Raw Text Input
    ↓
RawPacketParser.parse_packets()
    ↓
RawPacketData Vec
    ↓
RawPacketBatchProcessor.add_raw_text()
    ↓
RawPacketBatchProcessor.process_all()
    ↓
RawPacketModel Vec
    ↓
db_frames::insert_raw_packets_from_scan()
    ↓
SQLite Database
    ↓
Query via API/Web
```

## 💡 Tips & Tricks

### Deduplicate and Save
```rust
processor.process_all();
let unique = processor.deduplicate_by_mac();
db_frames::insert_raw_packets_from_scan(&conn, &unique)?;
```

### Export Statistics
```rust
let stats = processor.get_statistics();
println!("Stats: {:?}", serde_json::to_string(&stats)?);
```

### Real-time Processing
```rust
for line in io::stdin().lock().lines() {
    if let Some(packet) = parser.parse_packet(&line?) {
        // Process immediately
        let model = parser.to_raw_packet_model(&packet, id);
    }
}
```

### Batch with Limit
```rust
for chunk in raw_data.lines().collect::<Vec<_>>().chunks(100) {
    processor.add_raw_text(&chunk.join("\n"));
    let models = processor.process_all();
    db_frames::insert_raw_packets_from_scan(&conn, &models)?;
    processor.clear();
}
```

## 🚀 Ready-to-Use Code Snippets

### Snippet 1: Parse and Store
```rust
let parser = RawPacketParser::new();
let packets = parser.parse_packets(raw_text);
let conn = rusqlite::Connection::open("bluetooth_scan.db")?;
db_frames::insert_parsed_raw_packets(&conn, &packets)?;
```

### Snippet 2: Analyze and Report
```rust
let mut processor = RawPacketBatchProcessor::new();
processor.add_raw_text(raw_text);
let stats = processor.get_statistics();
println!("Total: {}, Unique: {}, Avg RSSI: {:.1}", 
         stats.total_packets, stats.unique_macs, stats.avg_rssi);
```

### Snippet 3: Filter by Signal
```rust
let packets: Vec<_> = parser.parse_packets(raw_text)
    .into_iter()
    .filter(|p| p.rssi > -80)  // Stronger than -80 dBm
    .collect();
```

### Snippet 4: Find Devices by Company
```rust
let apple_devices: Vec<_> = packets.iter()
    .filter(|p| p.company_name.as_deref() == Some("Apple"))
    .map(|p| &p.mac_address)
    .collect();
```

---

**Last Updated**: 2024
**Status**: ✅ Production Ready
**Version**: 1.0