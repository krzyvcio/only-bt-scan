# 📊 Core Data Models Architecture

## Overview
The only-bt-scan system is built on **TWO fundamental data models** that work together:

1. **DEVICE MODEL** - High-level aggregated information
2. **RAW PACKET MODEL** - Low-level packet bytes and structures

---

## MODEL 1: DEVICE DATA 🔵

### Purpose
Aggregated, high-level information about a discovered Bluetooth device. This is what you see when you list "devices found".

### Structure
```rust
DeviceModel {
    // Core ID
    mac_address: "AA:BB:CC:DD:EE:FF"
    device_name: Option<String>
    device_type: BleOnly | BrEdrOnly | DualMode
    
    // Signal Quality
    rssi: -65              // Current
    avg_rssi: -62.5        // Average over time
    rssi_variance: 3.2     // Stability metric
    
    // Timing
    first_seen: DateTime   // When first detected
    last_seen: DateTime    // Most recent detection
    response_time_ms: 1250 // Gap between first and last
    
    // Advertising Info
    manufacturer_id: 0x004C (Apple)
    advertised_services: ["180D", "180A"]  // Heart Rate, Device Info
    appearance: 0x0840     // Fitness Band
    tx_power: -5 dBm
    
    // MAC Addressing
    mac_type: "Random"
    is_rpa: true           // Random Private Address
    
    // Security
    security_level: "Encrypted"
    pairing_method: "LE Secure Connections"
    is_connectable: true
    
    // Statistics
    detection_count: 127   // Times scanned
    last_rssi_values: [-65, -63, -62]  // For charts
    
    // Discovered Services (from GATT)
    discovered_services: [
        {uuid: "180D", name: "Heart Rate", ...}
    ]
    
    // Vendor Protocols Found
    vendor_protocols: [
        {name: "Apple Continuity", type: "handoff"}
    ]
}
```

### Database Table: `devices`
```sql
CREATE TABLE devices (
    id INTEGER PRIMARY KEY,
    mac_address TEXT UNIQUE,
    device_name TEXT,
    rssi INTEGER,
    first_seen DATETIME,
    last_seen DATETIME,
    manufacturer_id INTEGER,
    manufacturer_name TEXT,
    device_type TEXT,
    number_of_scan INTEGER,
    mac_type TEXT,
    is_rpa BOOLEAN,
    security_level TEXT,
    pairing_method TEXT
);
```

### Example Data
```json
{
    "mac_address": "AA:BB:CC:DD:EE:FF",
    "device_name": "Apple Watch",
    "device_type": "BleOnly",
    "rssi": -65,
    "avg_rssi": -62.3,
    "manufacturer_id": 76,
    "manufacturer_name": "Apple Inc.",
    "detection_count": 45,
    "advertised_services": ["180D", "180A"],
    "is_rpa": false,
    "first_seen": "2026-02-14T10:30:00Z",
    "last_seen": "2026-02-14T10:45:30Z"
}
```

---

## MODEL 2: RAW PACKET DATA 📦

### Purpose
Complete, low-level information about individual Bluetooth advertisement packets. This is the **telemetry data** - every packet captured.

### Structure
```rust
RawPacketModel {
    // Identification
    packet_id: 12345
    mac_address: "AA:BB:CC:DD:EE:FF"
    timestamp: DateTime
    
    // Physical Layer
    phy: "LE 1M"           // 1M, 2M, Coded S=2, Coded S=8
    channel: 37            // Advertising channel
    rssi: -65 dBm          // Signal strength for THIS packet
    
    // Packet Structure
    packet_type: "ADV_IND"
    is_scan_response: false
    is_extended: false     // BT 5.0+ extended advertising
    
    // Raw Bytes
    advertising_data: [0x02, 0x01, 0x06, ...]
    advertising_data_hex: "020106..."
    
    // Parsed AD Structures (from advertising_data)
    ad_structures: [
        {
            ad_type: 0x01,
            type_name: "Flags",
            data: [0x06],
            interpretation: "LE General Discoverable Mode"
        },
        {
            ad_type: 0xFF,
            type_name: "Manufacturer Specific Data",
            data: [0x4C, 0x00, ...],
            interpretation: "Apple Inc."
        }
    ]
    
    // Extracted Fields
    flags: AdvertisingFlags {
        le_general_discoverable: true,
        br_edr_not_supported: true,
        ...
    }
    local_name: "Apple Watch"
    advertised_services: ["180D", "180A"]
    manufacturer_data: {0x004C: [...]}
    service_data: {"180D": [...]}
    
    // Quality Metrics
    total_length: 31
    parsed_successfully: true
}
```

### Database Table: `ble_advertisement_frames`
```sql
CREATE TABLE ble_advertisement_frames (
    id INTEGER PRIMARY KEY,
    device_id INTEGER,
    mac_address TEXT,
    rssi INTEGER,
    advertising_data BLOB,      -- Raw bytes
    phy TEXT,
    channel INTEGER,
    frame_type TEXT,
    timestamp DATETIME,
    FOREIGN KEY(device_id) REFERENCES devices(id)
);
```

### Example Data
```json
{
    "packet_id": 98765,
    "mac_address": "AA:BB:CC:DD:EE:FF",
    "timestamp": "2026-02-14T10:45:23.123Z",
    "phy": "LE 1M",
    "channel": 37,
    "rssi": -65,
    "packet_type": "ADV_IND",
    "advertising_data_hex": "020106030D1810FF4C000402150C8CABA12D3D4AEBA8C9E2E0200C5",
    "ad_structures": [
        {
            "ad_type": 1,
            "type_name": "Flags",
            "data_hex": "06",
            "interpretation": "LE General Discoverable Mode, BR/EDR Not Supported"
        }
    ],
    "local_name": "Apple Watch",
    "advertised_services": ["180D"]
}
```

---

## 🔗 RELATIONSHIP: How They Connect

```
DeviceModel (High-level)
    └── mac_address: "AA:BB:CC:DD:EE:FF"
        └── aggregates data from many packets
            
RawPacketModel (Low-level) x N
    ├── packet_id: 1, mac_address: "AA:BB:CC:DD:EE:FF", rssi: -65
    ├── packet_id: 2, mac_address: "AA:BB:CC:DD:EE:FF", rssi: -62
    ├── packet_id: 3, mac_address: "AA:BB:CC:DD:EE:FF", rssi: -68
    └── ...
```

### Data Flow:
```
Raw Bluetooth Packets
    ↓
Parse Individual Packets (RawPacketModel)
    ↓
Aggregate by MAC Address (DeviceModel)
    ↓
Store both in Database
    ↓
Serve via API (Separate or Combined)
```

---

## 📡 API Endpoints Using These Models

### Device-Level Endpoints (MODEL 1)
```
GET /api/devices
  → List of DeviceModel (paginated)
  
GET /api/devices/{mac}
  → Single DeviceModel with full details

GET /api/devices/{mac}/summary
  → Quick DeviceModel summary
```

### Packet-Level Endpoints (MODEL 2)
```
GET /api/raw-packets
  → List of RawPacketModel (paginated)
  
GET /api/raw-packets?mac={mac}
  → Packets for specific device
  
GET /api/raw-packets/latest
  → Most recent packets

GET /api/raw-packets/{id}
  → Single RawPacketModel
```

### Combined Endpoints
```
GET /api/devices/{mac}/packets
  → DeviceWithPackets (DeviceModel + recent RawPacketModel[])
  
GET /api/scan-results
  → ScanResultsModel (all devices + sample packets)
```

---

## 💾 Storage Strategy

### Two Tables, Two Purposes:

**`devices` table**
- One row per unique MAC address
- Updated frequently (RSSI, timestamps)
- ~100KB per 1000 devices
- Used for quick device lookups

**`ble_advertisement_frames` table**
- One row per packet captured
- Grows continuously (millions of rows)
- Indexed by MAC and timestamp
- Partitioned by date for performance
- Used for detailed analysis and charts

### Size Estimation:
```
Devices Table:
  - 1,000 devices × 1 KB = 1 MB
  
Packets Table (1 month retention):
  - 10 packets/device/min × 1,000 devices × 60 min × 24 hours × 30 days
  = 432 million packets
  ≈ 200 GB (with 500 bytes per packet on average)
```

---

## 🎯 When to Use Each Model

### Use DEVICE MODEL When:
- Listing devices found
- Checking last seen time
- Getting average signal quality
- Discovering services (GATT)
- Security/pairing info
- Quick reference by MAC

### Use RAW PACKET MODEL When:
- Analyzing detailed packet structure
- Debugging advertising data
- Creating RSSI charts
- Packet loss analysis
- Interference detection
- Protocol dissection
- Statistical analysis

---

## 📊 Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────┐
│ Bluetooth Air                                               │
│ Individual Advertisement Packets                            │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ↓
┌─────────────────────────────────────────────────────────────┐
│ RawPacketModel (Multiple per MAC)                           │
│ - Physical layer (PHY, channel, RSSI)                       │
│ - Raw bytes (advertising_data)                              │
│ - Parsed AD structures                                      │
│ - Timestamp                                                 │
│                                                             │
│ Storage: ble_advertisement_frames table (millions of rows)  │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ↓ (Aggregate by MAC)
┌─────────────────────────────────────────────────────────────┐
│ DeviceModel (One per MAC)                                   │
│ - High-level info (name, manufacturer)                      │
│ - Aggregated statistics (avg_rssi, variance)                │
│ - Timing (first_seen, last_seen)                            │
│ - Discovered services (GATT)                                │
│ - Vendor protocols                                          │
│                                                             │
│ Storage: devices table (~1000s of rows)                     │
└─────────────────┬───────────────────────────────────────────┘
                  │
        ┌─────────┴──────────┐
        ↓                    ↓
    Web API            Analytics
  /api/devices      RSSI Charts
  /api/raw-packets  Trends
  /api/...          Reports
```

---

## 🔄 Example: Complete Data Journey

### 1. Device Sends Packets (Air)
```
00:11:22:33:44:55 → ADV_IND packet with iBeacon data
```

### 2. Packet Captured and Parsed (RawPacketModel)
```json
{
    "packet_id": 12345,
    "mac_address": "00:11:22:33:44:55",
    "advertising_data_hex": "020106...",
    "rssi": -65,
    "channel": 37
}
```

### 3. Stored in Database
```sql
INSERT INTO ble_advertisement_frames 
VALUES (12345, 1, '00:11:22:33:44:55', -65, 0x020106..., 'LE 1M', 37, ...)
```

### 4. Aggregated into Device (DeviceModel)
```json
{
    "mac_address": "00:11:22:33:44:55",
    "rssi": -65,
    "avg_rssi": -63.2,
    "detection_count": 45
}
```

### 5. Served via API
```
GET /api/devices/00:11:22:33:44:55
  → DeviceModel
  
GET /api/raw-packets?mac=00:11:22:33:44:55
  → [RawPacketModel, RawPacketModel, ...]
  
GET /api/devices/00:11:22:33:44:55/packets
  → DeviceWithPackets {device, recent_packets}
```

---

## ✨ Key Advantages

✅ **Separation of Concerns**
- Devices handle high-level logic
- Packets handle low-level telemetry

✅ **Scalability**
- Devices table stays small (fast queries)
- Packets table can grow (time-series optimized)

✅ **Flexibility**
- Query by MAC (device-centric)
- Query by time (packet-centric)
- Combine both for complete picture

✅ **Analysis**
- Devices: "What devices am I seeing?"
- Packets: "What's happening in detail?"

✅ **Storage Efficiency**
- Devices: Aggregated summary (small)
- Packets: Detailed history (large, indexed)
