# 🔍 RAW PACKET CAPTURE STRATEGY - Complete Implementation Guide

**Version:** v0.4.0
**Status:** Implementation Plan
**Created:** 2024

---

## 📋 EXECUTIVE SUMMARY

Current state:
- ✅ Basic BLE advertising packets captured
- ❌ Only converts BluetoothDevice → RawPacketModel (loses raw data)
- ❌ No actual raw advertising data captured
- ❌ Missing scan response data
- ❌ No extended advertising support
- ❌ No BR/EDR packet capture
- ❌ No HCI event capture
- ❌ No L2CAP packet capture

**Goal:** Capture ALL possible Bluetooth packets at every layer and store with complete metadata.

---

## 🎯 PACKET TYPES TO CAPTURE

### Layer 1: BLE Advertising Channel Packets (Primary Focus)

#### 1.1 Advertising PDU Types (BLE 4.0-5.4)
- [ ] **ADV_IND** - Connectable undirected advertising
  - Captured: ✅ (basic)
  - Missing: Actual raw bytes, timing
  
- [ ] **ADV_DIRECT_IND** - Connectable directed advertising
  - Captured: ❌
  - Info: Targeted to specific device
  - Fields: InitA, AdvA
  
- [ ] **ADV_NONCONN_IND** - Non-connectable undirected advertising
  - Captured: ❌ (partially as ADV_IND)
  - Info: Beacons, broadcasts
  
- [ ] **ADV_SCAN_IND** - Scannable undirected advertising
  - Captured: ❌
  - Info: Allows scan response
  
- [ ] **SCAN_RSP** - Scan response data
  - Captured: ❌
  - Critical: Second part of advertising data
  
#### 1.2 Extended Advertising PDU Types (BT 5.0+)
- [ ] **ADV_EXT_IND** - Extended advertising indication
  - Captured: ❌
  - Features: Long advertising data, multiple PHYs, secondary channels
  
- [ ] **AUX_ADV_IND** - Auxiliary advertising indication
  - Captured: ❌
  - Info: Secondary channel advertising
  
- [ ] **AUX_SCAN_RSP** - Auxiliary scan response
  - Captured: ❌
  - Info: Extended scan response data
  
- [ ] **AUX_CHAIN_IND** - Chain indication
  - Captured: ❌
  - Info: Continuation of long advertising data

#### 1.3 Periodic Advertising (BT 5.1+)
- [ ] **AUX_SYNC_IND** - Periodic advertising sync
  - Captured: ❌
  - Features: Synchronized periodic data
  
- [ ] **BIGINFO** - BIG Info
  - Captured: ❌
  - Info: Broadcast isochronous group

### Layer 2: BR/EDR Packets (Linux/BlueZ only)

#### 2.1 BR/EDR Discovery & Connection
- [ ] **FHS** - Frequency Hopping Synchronization
  - Captured: ❌
  - Info: Device discovery beacon
  
- [ ] **EIR** - Extended Inquiry Response
  - Captured: ❌
  - Info: Device class, local name, services
  
- [ ] **Page response** - Connection response
  - Captured: ❌

#### 2.2 BR/EDR Data Packets
- [ ] **DM1-5** - Data packets various sizes
  - Captured: ❌
  
- [ ] **DH1-5** - High-speed data packets
  - Captured: ❌

### Layer 3: Link Manager Protocol (LMP)
- [ ] **LMP PDUs** - Link manager commands
  - Captured: ❌
  - Via HCI: Yes (HCI_LE_Meta_Event)

### Layer 4: L2CAP Packets
- [ ] **L2CAP frames** - Logical Link Control & Adaptation
  - Captured: ❌
  - Critical: Data channel communication
  
#### 4.1 L2CAP Connection Phases
- [ ] **LE Signaling Channel (0x0005)**
  - Connection requests/responses
  - Parameter update commands
  
- [ ] **LE Data Channels (0x0040-0x007F)**
  - Custom protocol data
  - Mesh, GATT, proprietary

### Layer 5: GATT Packets
- [ ] **ATT PDUs** - GATT layer
  - Captured: ❌
  - Types: Read, Write, Notify, Indicate

### Layer 6: HCI Events (Windows/Linux)
- [ ] **LE Meta Events (0x3E)**
  - LE Advertising Report
  - LE Connection Complete
  - LE Read Remote Features
  - LE Connection Update
  
- [ ] **Meta Events (0xFF)**
  - Vendor-specific events
  
- [ ] **Standard Events**
  - Remote Name Request Complete
  - Authentication Complete
  - Encryption Change

---

## 📊 DATABASE SCHEMA ENHANCEMENTS

### Current Tables
```
✅ ble_advertisement_frames
   - id, device_id, mac_address, rssi, advertising_data, phy, channel, frame_type, timestamp

❌ Missing detailed packet metadata
```

### New Tables Needed

#### 1. ble_raw_packet_metadata
```sql
CREATE TABLE ble_raw_packet_metadata (
    packet_id INTEGER PRIMARY KEY,
    mac_address TEXT NOT NULL,
    
    -- Physical Layer
    phy TEXT NOT NULL,                    -- "LE 1M", "LE 2M", "LE Coded S=2", "LE Coded S=8"
    channel INTEGER NOT NULL,             -- 37-39 (advertising), 0-36 (data)
    tx_power_level INTEGER,               -- Transmit power in dBm
    rssi INTEGER NOT NULL,                -- RSSI in dBm
    
    -- Packet Structure
    packet_type TEXT NOT NULL,            -- "ADV_IND", "SCAN_RSP", "ADV_EXT_IND", etc.
    packet_length INTEGER NOT NULL,       -- Total packet length
    
    -- Advertising Specific
    advertising_data BLOB,                -- Raw bytes
    advertising_data_hex TEXT,            -- Hex representation
    advertising_data_length INTEGER,      -- AD data only, excluding header
    
    -- Scan Response (if SCAN_RSP)
    scan_response_data BLOB,
    scan_response_data_hex TEXT,
    is_scan_response BOOLEAN DEFAULT 0,
    
    -- Extended Advertising (BT 5.0+)
    is_extended BOOLEAN DEFAULT 0,
    extended_header_length INTEGER,
    aux_pointer_present BOOLEAN,
    aux_channel INTEGER,
    aux_phy TEXT,
    
    -- Link Layer Timing
    packet_timestamp_us INTEGER,          -- Microseconds since scan start
    inter_frame_gap_us INTEGER,           -- Time since last packet from this device
    
    -- Signal Quality
    rssi_min INTEGER,                     -- For averaged signals
    rssi_max INTEGER,
    rssi_avg REAL,
    snr_estimate REAL,                    -- Signal-to-noise ratio
    
    -- Periodic Advertising (BT 5.1+)
    is_periodic BOOLEAN DEFAULT 0,
    sync_handle INTEGER,
    periodic_offset_us INTEGER,
    
    -- Connection Info (if data channel packet)
    connection_handle INTEGER,
    ce_length_us INTEGER,                 -- Connection event length
    
    -- Flags
    crc_ok BOOLEAN,                       -- CRC validation
    phy_valid BOOLEAN,
    address_type TEXT,                    -- "Random", "Public", "Resolvable"
    
    -- Timestamps
    timestamp DATETIME NOT NULL,
    timestamp_ms INTEGER NOT NULL,
    timestamp_us INTEGER NOT NULL,        -- Microsecond precision
    
    -- Metadata
    packet_source TEXT,                   -- "btleplug", "windows_api", "hci", "bluer"
    scan_session_id TEXT,
    
    FOREIGN KEY(mac_address) REFERENCES devices(mac_address),
    UNIQUE(packet_id, mac_address, timestamp_us)
);

CREATE INDEX idx_raw_packet_mac_time ON ble_raw_packet_metadata(mac_address, timestamp_us DESC);
CREATE INDEX idx_raw_packet_type ON ble_raw_packet_metadata(packet_type);
CREATE INDEX idx_raw_packet_phy ON ble_raw_packet_metadata(phy);
CREATE INDEX idx_raw_packet_channel ON ble_raw_packet_metadata(channel);
```

#### 2. ble_ad_structures_detailed
```sql
CREATE TABLE ble_ad_structures_detailed (
    id INTEGER PRIMARY KEY,
    packet_id INTEGER NOT NULL,
    mac_address TEXT NOT NULL,
    
    -- AD Structure Parsing
    ad_type INTEGER NOT NULL,            -- 0x00-0xFF (43 defined types)
    ad_type_name TEXT,                   -- "Flags", "Manufacturer Data", etc.
    data BLOB NOT NULL,
    data_hex TEXT NOT NULL,
    data_length INTEGER NOT NULL,
    
    -- Structured Parsing by Type
    flags_value INTEGER,                 -- For type 0x01
    tx_power_level INTEGER,              -- For type 0x0A
    uuid_type TEXT,                      -- "UUID16", "UUID32", "UUID128"
    uuid_value TEXT,                     -- Parsed UUID
    local_name TEXT,                     -- For type 0x08, 0x09
    manufacturer_id INTEGER,             -- For type 0xFF
    manufacturer_name TEXT,
    manufacturer_data BLOB,
    
    -- Service Data
    service_uuid TEXT,
    service_data BLOB,
    
    -- Appearance
    appearance_value INTEGER,
    appearance_name TEXT,
    
    -- OOB/Security
    oob_hash BLOB,
    oob_randomizer BLOB,
    security_flags INTEGER,
    
    -- Raw Interpretation
    interpretation TEXT,                 -- Human-readable explanation
    confidence REAL,                     -- Parsing confidence 0.0-1.0
    
    timestamp DATETIME NOT NULL,
    FOREIGN KEY(packet_id) REFERENCES ble_raw_packet_metadata(packet_id)
);

CREATE INDEX idx_ad_structures_packet ON ble_ad_structures_detailed(packet_id);
CREATE INDEX idx_ad_structures_type ON ble_ad_structures_detailed(ad_type);
CREATE INDEX idx_ad_structures_mac ON ble_ad_structures_detailed(mac_address, timestamp DESC);
```

#### 3. hci_events_captured
```sql
CREATE TABLE hci_events_captured (
    event_id INTEGER PRIMARY KEY,
    
    -- HCI Event Header
    event_code INTEGER NOT NULL,
    event_name TEXT,
    subevent_code INTEGER,              -- For Meta Events
    subevent_name TEXT,
    
    -- Raw Event Data
    event_data BLOB NOT NULL,
    event_data_hex TEXT,
    event_length INTEGER,
    
    -- Parsed Fields (varies by event type)
    num_reports INTEGER,                -- For LE Meta Events
    
    -- LE Advertising Report
    event_type INTEGER,
    address_type TEXT,
    mac_address TEXT,
    data_length INTEGER,
    data BLOB,
    rssi INTEGER,
    
    -- LE Connection Complete
    connection_handle INTEGER,
    role TEXT,                          -- "Master" / "Slave"
    peer_address_type TEXT,
    peer_address TEXT,
    
    -- LE Read Remote Features
    le_features BLOB,
    
    -- Connection Update
    conn_update_handle INTEGER,
    conn_update_interval INTEGER,
    conn_update_latency INTEGER,
    conn_update_timeout INTEGER,
    
    -- Timestamps
    timestamp DATETIME NOT NULL,
    timestamp_ms INTEGER NOT NULL,
    
    -- Metadata
    hci_source TEXT,                   -- "windows", "linux", "macos"
    scan_session_id TEXT,
    
    FOREIGN KEY(mac_address) REFERENCES devices(mac_address)
);

CREATE INDEX idx_hci_events_code ON hci_events_captured(event_code);
CREATE INDEX idx_hci_events_mac ON hci_events_captured(mac_address);
CREATE INDEX idx_hci_events_time ON hci_events_captured(timestamp DESC);
```

#### 4. l2cap_packets
```sql
CREATE TABLE l2cap_packets (
    packet_id INTEGER PRIMARY KEY,
    
    -- L2CAP Header
    connection_handle INTEGER NOT NULL,
    mac_address TEXT NOT NULL,
    
    channel_id INTEGER NOT NULL,       -- PSM (0x0001-0x7FFF)
    channel_name TEXT,                 -- e.g., "ATT", "SMP", "L2CAP Signaling"
    
    packet_length INTEGER NOT NULL,
    packet_data BLOB NOT NULL,
    packet_data_hex TEXT,
    
    -- L2CAP Flags
    start_of_l2cap_sdu BOOLEAN,
    complete_l2cap_sdu BOOLEAN,
    
    -- Direction
    direction TEXT,                     -- "Mobile->Device" or "Device->Mobile"
    
    -- PDU Type (varies by channel)
    pdu_type INTEGER,
    pdu_name TEXT,
    
    -- Parsed Data (varies by channel)
    -- For SMP (Security Manager):
    smp_opcode INTEGER,
    smp_opcode_name TEXT,
    
    -- For ATT (GATT):
    att_opcode INTEGER,
    att_opcode_name TEXT,
    att_handle INTEGER,
    att_value BLOB,
    
    -- Timing
    timestamp DATETIME NOT NULL,
    timestamp_ms INTEGER NOT NULL,
    inter_packet_gap_us INTEGER,
    
    -- Metadata
    packet_source TEXT,                -- "hci", "pcap", "bluez"
    scan_session_id TEXT,
    
    FOREIGN KEY(mac_address) REFERENCES devices(mac_address)
);

CREATE INDEX idx_l2cap_mac_time ON l2cap_packets(mac_address, timestamp DESC);
CREATE INDEX idx_l2cap_channel ON l2cap_packets(channel_id);
```

#### 5. packet_sequence_analysis
```sql
CREATE TABLE packet_sequence_analysis (
    sequence_id INTEGER PRIMARY KEY,
    mac_address TEXT NOT NULL,
    
    -- Sequence Info
    first_packet_time DATETIME NOT NULL,
    last_packet_time DATETIME NOT NULL,
    sequence_duration_ms INTEGER,
    
    -- Packet Statistics
    total_packets INTEGER,
    packet_types JSON,                 -- {"ADV_IND": 45, "SCAN_RSP": 23, ...}
    
    -- Timing Analysis
    min_interval_ms REAL,
    max_interval_ms REAL,
    avg_interval_ms REAL,
    median_interval_ms REAL,
    stddev_interval_ms REAL,
    
    -- PHY Analysis
    phy_distribution JSON,             -- {"LE 1M": 0.95, "LE 2M": 0.05}
    
    -- Channel Distribution
    channel_distribution JSON,         -- {37: 0.33, 38: 0.33, 39: 0.34}
    
    -- RSSI Analysis
    rssi_min INTEGER,
    rssi_max INTEGER,
    rssi_avg REAL,
    rssi_median INTEGER,
    rssi_stddev REAL,
    
    -- Pattern Detection
    is_periodic BOOLEAN,
    periodicity_ms INTEGER,
    periodicity_confidence REAL,
    
    is_bursty BOOLEAN,
    burst_characteristics JSON,
    
    -- Anomalies
    has_gaps BOOLEAN,
    gap_count INTEGER,
    max_gap_ms INTEGER,
    
    -- Data Characteristics
    advertising_data_static BOOLEAN,   -- Does AD data change?
    data_change_events INTEGER,
    data_size_distribution JSON,
    
    -- Connection Likelihood
    has_connection_attempt BOOLEAN,
    connection_handle INTEGER,
    
    -- Analysis Metadata
    analysis_timestamp DATETIME,
    confidence REAL,                   -- 0.0-1.0
    
    FOREIGN KEY(mac_address) REFERENCES devices(mac_address)
);
```

---

## 🔧 IMPLEMENTATION PHASES

### PHASE 1: Enhance RawPacketModel & Database (2-3 days)

#### 1.1 Update data_models.rs
```rust
// Expand RawPacketModel with complete metadata
pub struct RawPacketModel {
    // Existing fields...
    
    // Add new fields:
    pub tx_power_level: Option<i8>,
    pub inter_frame_gap_us: Option<u32>,
    pub rssi_min: Option<i8>,
    pub rssi_max: Option<i8>,
    pub rssi_avg: Option<f64>,
    pub snr_estimate: Option<f64>,
    
    pub is_extended: bool,
    pub extended_header_length: Option<u8>,
    pub aux_pointer_present: bool,
    pub aux_channel: Option<u8>,
    pub aux_phy: Option<String>,
    
    pub is_periodic: bool,
    pub periodic_offset_us: Option<u32>,
    
    pub connection_handle: Option<u16>,
    pub crc_ok: bool,
    pub address_type: String,              // "Random", "Public", "Resolvable"
    
    pub timestamp_us: u64,                 // Microsecond precision
    pub packet_source: String,             // "btleplug", "windows_api", "hci"
    pub scan_session_id: String,
    
    // Scan Response Data (separated)
    pub scan_response_data: Option<Vec<u8>>,
    pub scan_response_data_hex: Option<String>,
    pub is_scan_response: bool,
    
    pub ad_structures_detailed: Vec<DetailedAdStructure>,
}

pub struct DetailedAdStructure {
    pub ad_type: u8,
    pub ad_type_name: String,
    pub data: Vec<u8>,
    pub data_hex: String,
    pub parsed_value: AdStructureValue,
    pub confidence: f64,
}

pub enum AdStructureValue {
    Flags(AdvertisingFlags),
    TxPowerLevel(i8),
    CompleteLocalName(String),
    ShortenedLocalName(String),
    Uuid16(Vec<u16>),
    Uuid32(Vec<u32>),
    Uuid128(Vec<String>),
    ManufacturerData(u16, Vec<u8>),
    ServiceData16(u16, Vec<u8>),
    ServiceData32(u32, Vec<u8>),
    ServiceData128(String, Vec<u8>),
    Appearance(u16),
    AdvertisingInterval(u16),
    LeBluetoothDeviceAddress(String, String), // Address, Address Type
    Flags3d(u8),
    SimpleServiceSolicitation16(Vec<u16>),
    // ... others
}
```

#### 1.2 Update db.rs & db_frames.rs
- [ ] Create all 5 new tables above
- [ ] Add migration system for schema changes
- [ ] Create indices for performance
- [ ] Add helper functions for bulk insertion

#### 1.3 Add logging
- [ ] Log packet type distribution per scan
- [ ] Log raw data hex for debugging
- [ ] Track extraction success rates by type

**Estimated Effort:** 8-10 hours

---

### PHASE 2: Capture Advertising Packets with Full Data (3-4 days)

#### 2.1 Enhance btleplug Integration
```rust
// In bluetooth_scanner.rs - enhance packet capture

async fn capture_advertising_data(&self, peripheral: &Peripheral) -> Result<RawPacketModel> {
    let properties = peripheral.properties().await?;
    
    // Get manufacturer data
    let mut raw_packet = RawPacketModel::new(...);
    
    // Extract TX Power from advertisement
    if let Some(tx_power) = properties.tx_power_level {
        raw_packet.tx_power_level = Some(tx_power);
    }
    
    // Extract all manufacturer data
    if let Some(mfg_data) = properties.manufacturer_data {
        for (mfg_id, data) in mfg_data {
            raw_packet.manufacturer_data.insert(mfg_id, data);
        }
    }
    
    // Extract service data
    if let Some(svc_data) = properties.service_data {
        for (uuid, data) in svc_data {
            raw_packet.service_data.insert(uuid, data);
        }
    }
    
    // Capture raw advertising data bytes
    raw_packet.advertising_data = extract_raw_ad_bytes(&properties)?;
    raw_packet.advertising_data_hex = hex::encode(&raw_packet.advertising_data);
    
    // Parse AD structures
    raw_packet.ad_structures_detailed = parse_all_ad_structures(&raw_packet.advertising_data);
    
    Ok(raw_packet)
}

fn parse_all_ad_structures(data: &[u8]) -> Vec<DetailedAdStructure> {
    let mut structures = Vec::new();
    let mut pos = 0;
    
    while pos < data.len() {
        let len = data[pos] as usize;
        if len == 0 || pos + len + 1 > data.len() {
            break;
        }
        
        let ad_type = data[pos + 1];
        let ad_data = &data[pos + 2..pos + 1 + len];
        
        let (type_name, parsed_value, confidence) = parse_ad_type(ad_type, ad_data);
        
        structures.push(DetailedAdStructure {
            ad_type,
            ad_type_name: type_name,
            data: ad_data.to_vec(),
            data_hex: hex::encode(ad_data),
            parsed_value,
            confidence,
        });
        
        pos += len + 1;
    }
    
    structures
}
```

#### 2.2 Add Scan Response Capture
```rust
// Enhanced packet model to separate scan response

pub struct AdvertisingExchange {
    pub advertisement: RawPacketModel,    // ADV_IND or ADV_SCAN_IND
    pub scan_response: Option<RawPacketModel>,  // SCAN_RSP (if received)
    pub correlation_time_ms: u32,         // Time between ADV and SCAN_RSP
}

// Modify scanner to track ADV -> SCAN_RSP pairs
pub struct AdvertisingTracker {
    pending_advertisements: HashMap<String, (RawPacketModel, Instant)>,
}

impl AdvertisingTracker {
    pub fn add_advertisement(&mut self, packet: RawPacketModel) {
        self.pending_advertisements.insert(
            packet.mac_address.clone(),
            (packet, Instant::now())
        );
    }
    
    pub fn add_scan_response(&mut self, mac: &str, response: RawPacketModel) -> Option<AdvertisingExchange> {
        if let Some((mut adv, time)) = self.pending_advertisements.remove(mac) {
            let correlation_time_ms = time.elapsed().as_millis() as u32;
            
            Some(AdvertisingExchange {
                advertisement: adv,
                scan_response: Some(response),
                correlation_time_ms,
            })
        } else {
            None
        }
    }
}
```

#### 2.3 Extended Advertising Support (BT 5.0+)
```rust
// Detect and parse extended advertising packets

pub fn is_extended_advertising(packet: &RawPacketModel) -> bool {
    packet.packet_type == "ADV_EXT_IND" || 
    packet.is_extended
}

pub fn parse_extended_advertising_header(data: &[u8]) -> ExtendedAdHeader {
    // Parse AuxPtr field
    // Parse SecondaryPHY
    // Parse AdvDataLength
    // Parse ExtHeaderLength
    // Parse ExtHeaderFlags
    
    ExtendedAdHeader {
        aux_pointer: None,
        aux_phy: None,
        secondary_phy: None,
        adv_data_length: 0,
        tx_power_included: false,
    }
}
```

**Estimated Effort:** 10-12 hours

---

### PHASE 3: Platform-Specific Raw Packet Capture (4-5 days)

#### 3.1 Windows Native API Enhancement
```rust
// In windows_bluetooth.rs - capture raw HCI packets

pub async fn capture_hci_packets(&self) -> Result<Vec<RawPacketModel>> {
    // Use Windows.Devices.Bluetooth HCI passthrough
    // Capture LE Meta Events (0x3E)
    // Parse LE Advertising Reports
    
    let mut packets = Vec::new();
    
    // Open HCI device
    let hci_device = open_hci_device()?;
    
    // Request LE event mask to receive all events
    hci_device.send_command(
        HCI_CMD_SET_EVENT_MASK_PAGE2,
        &[0xFF; 8]
    )?;
    
    // Read advertising events
    loop {
        if let Ok(event) = hci_device.read_event(Duration::from_millis(100)) {
            if event.event_code == 0x3E {  // Meta event
                let packet = parse_le_meta_event(&event)?;
                packets.push(packet);
            }
        }
    }
    
    Ok(packets)
}

fn parse_le_meta_event(event: &HciEvent) -> Result<RawPacketModel> {
    match event.subevent_code {
        0x02 => parse_le_advertising_report(event),   // LE Advertising Report
        0x01 => parse_le_connection_complete(event),  // Connection Complete
        0x05 => parse_le_read_remote_features(event), // Read Remote Features
        0x04 => parse_le_connection_update(event),    // Connection Update
        _ => Err("Unknown LE meta event".into())
    }
}
```

#### 3.2 Linux HCI Socket Integration
```rust
// In hci_sniffer_example.rs or new module

pub struct HciPacketCapture {
    socket: RawSocket,
    buffer: Vec<u8>,
}

impl HciPacketCapture {
    pub fn new(device_id: u16) -> Result<Self> {
        // Create AF_BLUETOOTH socket with BTPROTO_HCI
        let socket = socket(AF_BLUETOOTH, SOCK_RAW, BTPROTO_HCI)?;
        
        // Bind to device
        bind_hci_device(socket, device_id)?;
        
        // Set socket options for all packets
        setsockopt(socket, SOL_HCI, HCI_FILTER, &filter)?;
        
        Ok(Self {
            socket,
            buffer: vec![0u8; 2048],
        })
    }
    
    pub async fn capture_packets(&mut self) -> Result<Vec<RawPacketModel>> {
        let mut packets = Vec::new();
        
        loop {
            match self.read_hci_packet() {
                Ok(packet) => packets.push(packet),
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }
        
        Ok(packets)
    }
    
    fn read_hci_packet(&mut self) -> Result<RawPacketModel> {
        let n = read(self.socket, &mut self.buffer)?;
        
        // Parse HCI packet header
        let packet_type = self.buffer[0];
        let event_code = self.buffer[1];
        let param_length = self.buffer[2] as usize;
        
        match packet_type {
            0x04 => self.parse_hci_event(&self.buffer[..n]),
            0x02 => self.parse_acl_data(&self.buffer[..n]),
            _ => Err("Unknown packet type".into())
        }
    }
}
```

#### 3.3 macOS CoreBluetooth (if available)
```rust
// Create macos_raw_capture.rs

pub struct MacOSRawCapture {
    // Would use CBCentralManagerDelegate to intercept raw packets
    // Note: CoreBluetooth doesn't expose raw HCI on macOS
    // Alternative: Use system-wide packet capture via libpcap
}

pub async fn capture_via_libpcap() -> Result<Vec<RawPacketModel>> {
    // Use libpcap to capture Bluetooth packets
    // Filter for Bluetooth device interface
    // Parse link layer packets
}
```

**Estimated Effort:** 12-16 hours

---

### PHASE 4: L2CAP & GATT Packet Capture (3-4 days)

#### 4.1 L2CAP Channel Monitoring
```rust
// New module: l2cap_packet_capture.rs

pub struct L2CapMonitor {
    connection_handles: HashMap<u16, String>,  // handle -> MAC
}

impl L2CapMonitor {
    pub fn capture_l2cap_packets(&self, hci_events: &[HciEvent]) -> Vec<L2CapPacket> {
        let mut l2cap_packets = Vec::new();
        
        for event in hci_events {
            if let HciEvent::AclDataPacket(acl) = event {
                // Parse ACL header
                let connection_handle = acl.connection_handle;
                let pbf = acl.packet_boundary_flag;
                let bcf = acl.broadcast_flag;
                
                // L2CAP packet starts after ACL header
                let l2cap_length = u16::from_le_bytes([
                    acl.data[0],
                    acl.data[1]
                ]) as usize;
                
                let channel_id = u16::from_le_bytes([
                    acl.data[2],
                    acl.data[3]
                ]);
                
                let l2cap_payload = &acl.data[4..4 + l2cap_length];
                
                l2cap_packets.push(L2CapPacket {
                    connection_handle,
                    channel_id,
                    data: l2cap_payload.to_vec(),
                    is_start_of_sdu: pbf == 0,
                    timestamp: event.timestamp,
                });
            }
        }
        
        l2cap_packets
    }
}

pub struct L2CapPacket {
    pub connection_handle: u16,
    pub channel_id: u16,  // PSM
    pub data: Vec<u8>,
    pub is_start_of_sdu: bool,
    pub timestamp: u64,
}

impl L2CapPacket {
    pub fn channel_name(&self) -> &'static str {
        match self.channel_id {
            0x0001 => "L2CAP Signaling",
            0x0003 => "AMP Manager",
            0x0005 => "LE Signaling",
            0x0007 => "SMP",
            0x000F => "LE SMP",
            0x0011 => "LE Attribute",
            0x001F => "LE Data Channel (0x001F)",
            id if id >= 0x0040 && id <= 0x007F => "LE Dynamic Channel",
            _ => "Unknown"
        }
    }
    
    pub fn parse_pdu(&self) -> Option<L2CapPdu> {
        match self.channel_id {
            0x0005 => self.parse_le_signaling(),
            0x0007 => self.parse_smp(),
            0