# only-bt-scan - Specyfikacja Techniczna

## 1. Architektura Systemu

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Tokio Runtime (Multi-threaded)                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                  │
│  │  Scanner     │    │   Parser     │    │    DB        │                  │
│  │  Task(s)     │───▶│  Task(s)     │───▶│  Writer      │                  │
│  │              │    │              │    │  Task        │                  │
│  └──────────────┘    └──────────────┘    └──────────────┘                  │
│         │                   │                   │                             │
│         │                   │                   │                             │
│         ▼                   ▼                   ▼                             │
│  ┌────────────────────────────────────────────────────────────────┐         │
│  │                    Channel (mpsc/broadcast)                     │         │
│  │  • Packet channel (bounded, with backpressure)                 │         │
│  │  • Device channel (for state changes)                         │         │
│  │  • Control channel (for stop/pause/config)                   │         │
│  └────────────────────────────────────────────────────────────────┘         │
│                                    │                                        │
│                                    ▼                                        │
│                         ┌──────────────────┐                               │
│                         │  Terminal Output │                               │
│                         │  (non-blocking)  │                               │
│                         └──────────────────┘                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 2. Wykrywanie Adapterów

### 2.1 Adapter Manager

```rust
/// Represents a Bluetooth adapter with its capabilities
pub struct Adapter {
    pub id: String,              // System identifier
    pub name: String,            // Human-readable name
    pub address: String,         // MAC address
    pub capabilities: AdapterCapabilities,
    pub is_default: bool,
}

/// Adapter capabilities (bitfield)
pub struct AdapterCapabilities {
    pub bt_version: BluetoothVersion,      // 1.0 - 5.4
    pub supports_ble: bool,
    pub supports_bredr: bool,
    pub supports_le_audio: bool,
    pub supports_extended_advertising: bool,
    pub supports_periodic_advertising: bool,
    pub supports_iso_channels: bool,
    pub supports_2m_phy: bool,
    pub supports_coded_phy: bool,
    pub max_advertising_sets: u8,
    pub max_connection_interval: u16,
    pub supports_simultaneous_central_peripheral: bool,
}

/// Selection strategy
pub enum AdapterSelection {
    BestCapabilities,    // Wybierz z najlepszymi możliwościami
    ById(String),        // Wybierz konkretny adapter
    FirstAvailable,     // Wybierz pierwszy dostępny
}
```

### 2.2 Algorithm wyboru adaptera

1. Enumerate all adapters
2. Score each adapter:
   - BLE support: +10
   - BR/EDR support: +5
   - Extended advertising: +15
   - LE Audio: +10
   - Max BT version: + (version * 5)
   - 2M/Coded PHY: +10
3. Select highest score

## 3. Skanowanie

### 3.1 Typy skanowania

| Typ | Opis | API |
|-----|------|-----|
| BLE Passive | Nasłuchuj bez wysyłania | btleplug |
| BLE Active | Nasłuchuj + scan request | btleplug |
| BLE Extended | BLE 5.0+ extended | btleplug |
| BR/EDR Inquiry | Classic discovery | bluer (Linux) |
| BR/EDR Page | Connectable devices | bluer (Linux) |
| HCI Raw | Bezpośredni dostęp | hci crate |

### 3.2 Konfiguracja skanowania

```rust
#[derive(Debug, Clone)]
pub struct ScanConfig {
    /// Duration of each scan cycle
    pub scan_duration: Duration,
    /// Number of scan cycles (0 = infinite)
    pub num_cycles: usize,
    /// Enable BLE scanning
    pub ble_enabled: bool,
    /// Enable Classic scanning (Linux only)
    pub classic_enabled: bool,
    /// Use extended advertising (BLE 5.0+)
    pub use_extended: bool,
    /// Use all PHYs (1M, 2M, Coded)
    pub use_all_phys: bool,
    /// Filter duplicates
    pub filter_duplicates: bool,
    /// Active or passive scanning
    pub active_scanning: bool,
    /// Scan window/interval
    pub scan_window: Duration,
    pub scan_interval: Duration,
}
```

### 3.3 Skanowanie agresywne

- Skanowanie ciągłe (nie przerywaj między cyklami)
- Wszystkie kanały reklamowe (37, 38, 39 + secondary)
- Wszystkie PHY (1M, 2M, Coded S2, Coded S8)
- Extended advertising chains
- Scan response processing

## 4. Parsowanie Pakietów

### 4.1 Struktury danych

```rust
/// Raw Bluetooth packet (immutable after creation)
#[derive(Clone)]
pub struct RawPacket {
    pub id: u64,                           // Global sequence number
    pub adapter_id: String,               // Which adapter received
    pub timestamp_ns: i64,                 // Nanoseconds since epoch
    pub packet_type: PacketType,
    pub mac_address: String,
    pub rssi: i8,
    pub phy: Phy,
    pub channel: u8,
    pub data: Vec<u8>,                     // Raw payload
    pub is_connectable: bool,
    pub is_scannable: bool,                // Has scan response
    pub is_directed: bool,
    pub is_legacy: bool,                   // vs Extended
}

/// Decoded packet with parsed fields
#[derive(Clone, Debug)]
pub struct ParsedPacket {
    pub raw: RawPacket,
    pub advertising_data: AdvertisingData,
    pub security: Option<SecurityInfo>,
    pub device_info: Option<DeviceInfo>,
}

/// BLE Advertising Data (parsed AD structures)
#[derive(Clone, Debug, Default)]
pub struct AdvertisingData {
    pub flags: Option<AdFlags>,
    pub local_name: Option<String>,
    pub tx_power: Option<i8>,
    pub appearance: Option<u16>,
    pub service_uuids: Vec<Uuid>,
    pub service_data: HashMap<Uuid, Vec<u8>>,
    pub manufacturer_data: HashMap<u16, Vec<u8>>,
    pub uri: Option<String>,
    // ... all 43 AD types
}
```

## 5. Kanały i Backpressure

### 5.1 Channel Configuration

```rust
// Main packet channel - bounded with backpressure
const PACKET_CHANNEL_SIZE: usize = 10000;
const PACKET_CHANNEL_FULL_ACTION: BackpressureAction = BackpressureAction::DropOldest;

// Batch queue for DB writes
const DB_BATCH_SIZE: usize = 500;
const DB_BATCH_TIMEOUT: Duration = Duration::from_millis(100);
```

### 5.2 Backpressure Strategies

```rust
pub enum BackpressureAction {
    /// Drop oldest packet (FIFO)
    DropOldest,
    /// Drop newest packet (LIFO)
    DropNewest,
    /// Block until space available (NOT recommended for scanners)
    Block,
    /// Log warning and drop
    DropWithWarning,
}
```

### 5.3 Memory Limits

```rust
pub struct MemoryConfig {
    pub max_packets_in_memory: usize = 50000,
    pub max_devices_tracked: usize = 10000,
    pub max_packet_queue: usize = 10000,
    pub db_write_buffer: usize = 500,
}
```

## 6. Baza Danych

### 6.1 Schemat

```sql
-- Adapters (static)
CREATE TABLE adapters (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    address TEXT NOT NULL,
    capabilities JSON NOT NULL,
    is_default BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Scan sessions
CREATE TABLE scan_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    adapter_id TEXT NOT NULL,
    started_at DATETIME NOT NULL,
    ended_at DATETIME,
    config JSON NOT NULL,
    packets_received INTEGER DEFAULT 0,
    devices_discovered INTEGER DEFAULT 0,
    FOREIGN KEY (adapter_id) REFERENCES adapters(id)
);

-- Raw packets (high throughput)
CREATE TABLE raw_packets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    adapter_id TEXT NOT NULL,
    timestamp_ns INTEGER NOT NULL,
    packet_type TEXT NOT NULL,
    mac_address TEXT NOT NULL,
    rssi INTEGER NOT NULL,
    phy TEXT NOT NULL,
    channel INTEGER NOT NULL,
    data BLOB NOT NULL,              -- Raw bytes
    flags INTEGER NOT NULL,           -- Bitfield
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES scan_sessions(id),
    FOREIGN KEY (adapter_id) REFERENCES adapters(id)
);

-- Parsed advertising data
CREATE TABLE advertising_data (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    packet_id INTEGER NOT NULL UNIQUE,
    flags INTEGER,
    local_name TEXT,
    tx_power INTEGER,
    appearance INTEGER,
    service_uuids JSON,
    service_data JSON,
    manufacturer_id INTEGER,
    manufacturer_data BLOB,
    FOREIGN KEY (packet_id) REFERENCES raw_packets(id)
);

-- Devices (deduplicated)
CREATE TABLE devices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mac_address TEXT NOT NULL UNIQUE,
    first_seen DATETIME NOT NULL,
    last_seen DATETIME NOT NULL,
    packet_count INTEGER DEFAULT 0,
    avg_rssi REAL,
    min_rssi INTEGER,
    max_rssi INTEGER,
    name TEXT,
    manufacturer_id INTEGER,
    device_type TEXT,
    is_rpa BOOLEAN DEFAULT FALSE,
    security_level TEXT,
    FOREIGN KEY (manufacturer_id) REFERENCES manufacturers(id)
);

-- Indexes for performance
CREATE INDEX idx_raw_packets_mac ON raw_packets(mac_address);
CREATE INDEX idx_raw_packets_timestamp ON raw_packets(timestamp_ns);
CREATE INDEX idx_raw_packets_session ON raw_packets(session_id);
CREATE INDEX idx_devices_last_seen ON devices(last_seen DESC);
```

### 6.2 Batch Insert

```rust
/// Batch insert packets - optimized for high throughput
/// Uses transaction + prepared statements for speed
pub async fn insert_packets_batch(
    pool: &DbPool,
    packets: Vec<RawPacket>,
) -> Result<usize, DbError>;
```

## 7. Error Handling

### 7.1 Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Adapter error: {0}")]
    Adapter(String),
    
    #[error("Scan error: {0}")]
    Scan(String),
    
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Channel error: {0}")]
    Channel(String),
    
    #[error("Memory limit exceeded: {0}")]
    MemoryLimit(String),
}
```

## 8. CLI Interface

```bash
# Basic scan
only-bt-scan scan

# Specify adapter
only-bt-scan scan --adapter "hci0"

# Continuous mode
only-bt-scan scan --continuous --output.db

# With filters
only-bt-scan scan --filter-mac "AA:BB:CC:DD:EE:FF" --filter-rssi -70

# Export options
only-bt-scan scan --export-pcap output.pcap
only-bt-scan scan --export-json output.json

# Performance tuning
only-bt-scan scan --batch-size 1000 --queue-size 50000

# Show adapters
only-bt-scan adapters
```

## 9. Ograniczenia Systemowe

| Platform | Ograniczenie | Rozwiązanie |
|----------|--------------|-------------|
| Windows | Brak BR/EDR API | Tylko BLE |
| Windows | Ograniczone HCI | btleplug + winbluetooth |
| Linux | Wymaga root | sudo lub setcap |
| macOS | Ograniczone API | CoreBluetooth |
| BLE | Max 4 kanały | Extended advertising |
| DB | SQLite write lock | WAL mode + batch |

## 10. Metryki i Monitoring

```rust
pub struct ScannerMetrics {
    pub packets_received: u64,
    pub packets_dropped: u64,
    pub devices_discovered: u64,
    pub db_writes: u64,
    pub db_errors: u64,
    pub avg_rssi: f64,
    pub packets_per_second: f64,
    pub memory_used_mb: usize,
}
```

---

*Last updated: 2026-02-16*
