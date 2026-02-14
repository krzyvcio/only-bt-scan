# 🔗 L2CAP Channel Analysis & Extraction

## What is L2CAP?

**L2CAP** (Logical Link Control and Adaptation Protocol) is a Bluetooth layer that sits between HCI (Host Controller Interface) and higher protocols (GATT, HID, etc).

Think of it like this:
```
Bluetooth Protocol Stack
├── Physical Layer
├── Link Layer
├── HCI (Host Controller Interface)
├── L2CAP ← We are here!
│   ├── PSM 0x001F (ATT - Attribute Protocol for GATT)
│   ├── PSM 0x0011 (HID-Control)
│   ├── PSM 0x0003 (RFCOMM for Classic BT)
│   └── PSM 0xXXXX (Many more services...)
└── GATT, HID, RFCOMM, etc.
```

## Key Concepts

### PSM (Protocol/Service Multiplexer)
- **Like TCP/UDP ports** for Bluetooth
- Unique identifier for each service type
- Range: 0x0000 - 0xFFFF (16-bit)

### Channel ID (CID)
- Specific channel instance number
- Different from PSM (multiple channels can use same PSM)
- Assigned by HCI stack

### L2CAP Frame
Raw data unit transmitted over L2CAP with:
- Source CID (SCID)
- Destination CID (DCID)
- PSM information
- Data payload

---

## Why Extract L2CAP Channels?

### 1. **Service Type Detection**
```
PSM 0x001F → ATT (Attribute Protocol)
PSM 0x0011 → HID-Control
PSM 0x0003 → RFCOMM (Serial)
PSM 0x0019 → AVDTP (Audio)
```

### 2. **Connection Profiling**
Understand what services a device is using:
```
Device: iPhone
├── Channel 1: ATT (GATT discovery)
├── Channel 2: HID-Interrupt (Apple Remote control)
├── Channel 3: SMP (Security/Pairing)
└── Channel 4: Dynamic PSM (Custom service)
```

### 3. **Data Transfer Tracking**
Monitor how much data flows through each service:
```
iPhone L2CAP Summary:
├── ATT: 50MB/month (lots of GATT reads)
├── HID: 2MB/month (remote input)
└── SMP: 0.1MB/month (pairing info)
```

### 4. **Connection Quality Analysis**
- MTU (Maximum Transmission Unit) per channel
- Flush timeouts
- Retry patterns
- Error rates

---

## Standard PSM Values

| PSM | Hex | Service | Purpose |
|-----|-----|---------|---------|
| 1 | 0x0001 | **SDP** | Service Discovery Protocol |
| 3 | 0x0003 | **RFCOMM** | Serial port emulation |
| 5 | 0x0005 | TCS-BIN | Telephony Control |
| 15 | 0x000F | **BNEP** | Bluetooth Network Encapsulation |
| 17 | 0x0011 | **HID-Control** | Human Interface Device |
| 19 | 0x0013 | **HID-Interrupt** | HID Interrupt Channel |
| 21 | 0x0015 | BLE L2CAP | Attribute Protocol |
| 23 | 0x0017 | **AVDTP** | Audio/Video Distribution |
| 25 | 0x0019 | **AVCTP** | Audio/Video Control |
| 27 | 0x001B | AVCTP-Browsing | AV Remote Browsing |
| 29 | 0x001D | UDI_C-Plane | UDI |
| 31 | 0x001F | **ATT** | Attribute Protocol (GATT) |
| 33 | 0x0021 | **EATT** | Enhanced ATT |
| 35 | 0x0023 | **SMP** | Security Manager Protocol |
| ≥4096 | ≥0x1000 | **Dynamic** | Custom services |

### Most Common in BLE:
- **0x001F (ATT)** - GATT attribute reads/writes
- **0x0021 (EATT)** - Enhanced ATT (BT 5.2+)
- **0x0023 (SMP)** - Pairing & security
- **0x1000+ (Dynamic)** - Custom vendor services

---

## Platform-Specific Extraction

### 📱 macOS / iOS (CoreBluetooth)
```swift
// Apple's CBL2CAPChannel provides direct access
let peripheral = CBPeripheral(...)
let l2capChannels = peripheral.l2capChannels()

for channel in l2capChannels {
    let psm = channel.psm                    // 0x001F for ATT
    let sourceID = channel.sourceID          // Outgoing CID
    let destinationID = channel.destinationID // Incoming CID
    let mtu = channel.mtu                    // Max data size
}
```

### 🐧 Linux / 🪟 Windows (HCI)
```
HCI Commands to extract L2CAP info:
├── HCI_Read_Link_Supervision_Timeout
├── HCI_Read_Link_Quality
├── HCI_Get_Link_Key
└── Parse L2CAP frames from HCI packet capture
```

---

## Implementation in only-bt-scan

### Module: `l2cap_analyzer.rs`

#### Structures:

```rust
// L2CAP PSM identifier
pub struct L2CapPsm(pub u16);

// Individual channel information
pub struct L2CapChannel {
    pub channel_id: u16,              // CID
    pub psm: L2CapPsm,                // 0x001F, etc
    pub mac_address: String,
    pub state: ChannelState,          // Connected, etc
    pub tx_bytes: u32,                // Data sent
    pub rx_bytes: u32,                // Data received
    pub mtu: u16,                     // Max transmission unit
    pub flush_timeout_ms: u16,
    pub opened_at: DateTime,
    pub last_activity: DateTime,
}

// Device's complete L2CAP profile
pub struct L2CapDeviceProfile {
    pub mac_address: String,
    pub device_name: Option<String>,
    pub channels: Vec<L2CapChannel>,
    pub psm_usage: HashMap<u16, u32>, // PSM -> count
    pub total_tx_bytes: u64,
    pub total_rx_bytes: u64,
    pub supports_ble: bool,
    pub supports_bredr: bool,
    pub supports_eatt: bool,          // Enhanced ATT
}

// Main analyzer
pub struct L2CapAnalyzer {
    devices: HashMap<String, L2CapDeviceProfile>,
}
```

#### Usage:

```rust
use only_bt_scan::l2cap_analyzer::{L2CapAnalyzer, L2CapChannel, L2CapPsm, ChannelState};

// Create analyzer
let mut analyzer = L2CapAnalyzer::new();

// Register device
analyzer.register_device("AA:BB:CC:DD:EE:FF".to_string(), Some("iPhone".to_string()));

// Add L2CAP channel info
let channel = L2CapChannel {
    channel_id: 0x0040,
    psm: L2CapPsm(0x001F),  // ATT
    mac_address: "AA:BB:CC:DD:EE:FF".to_string(),
    state: ChannelState::Connected,
    tx_bytes: 50_000,
    rx_bytes: 100_000,
    mtu: 512,
    flush_timeout_ms: 3000,
    opened_at: chrono::Utc::now(),
    last_activity: chrono::Utc::now(),
};

analyzer.add_channel("AA:BB:CC:DD:EE:FF", channel)?;

// Get analysis
analyzer.print_summary();

// Output:
// 📊 L2CAP Channel Analysis Summary
// AA:BB:CC:DD:EE:FF: 1 total channels (1 active), 0MB tx, 0MB rx
//    Attribute Protocol (ATT) - 1 channel(s)
```

---

## Real-World Examples

### Example 1: Apple Watch
```
Device: Apple Watch (AA:BB:CC:DD:EE:FF)

L2CAP Channels:
├── Channel 0x0040
│   ├── PSM: 0x001F (ATT - Attribute Protocol)
│   ├── State: Connected
│   ├── TX: 2.5 MB (watch → phone)
│   ├── RX: 12.8 MB (phone → watch)
│   ├── MTU: 512 bytes
│   └── Duration: 8 hours 23 minutes
│
├── Channel 0x0041
│   ├── PSM: 0x0021 (EATT - Enhanced ATT)
│   ├── State: Connected
│   ├── TX: 5.2 MB
│   ├── RX: 8.1 MB
│   └── MTU: 251 bytes
│
└── Channel 0x0042
    ├── PSM: 0x0023 (SMP - Security Manager)
    ├── State: Connected
    ├── TX: 0.05 MB
    ├── RX: 0.03 MB
    └── Duration: Initial pairing only
```

### Example 2: Bluetooth Speaker
```
Device: JBL Speaker (11:22:33:44:55:66)

L2CAP Channels:
├── Channel 0x0043
│   ├── PSM: 0x0019 (AVDTP - Audio Distribution)
│   ├── State: Connected
│   ├── TX: 150 MB (audio streamed out)
│   └── Duration: 2 hours
│
└── Channel 0x0044
    ├── PSM: 0x000B (AVCTP - Audio Control)
    ├── State: Connected
    ├── TX: 0.5 MB (play/pause commands)
    └── Duration: 2 hours
```

### Example 3: Wireless Mouse
```
Device: Logitech Mouse (AA:AA:AA:AA:AA:AA)

L2CAP Channels:
├── Channel 0x0045
│   ├── PSM: 0x0011 (HID-Control)
│   ├── State: Connected
│   ├── TX: 0 MB
│   ├── RX: 0.5 MB (mouse movements from device)
│   └── Duration: 6 hours
│
└── Channel 0x0046
    ├── PSM: 0x0013 (HID-Interrupt)
    ├── State: Connected
    ├── RX: 15 MB (button clicks, movement)
    └── Duration: 6 hours
```

---

## Integration with only-bt-scan

### 1. Add to DeviceModel
```rust
// In data_models.rs
pub struct DeviceModel {
    // ... existing fields ...
    pub l2cap_channels: Vec<L2CapChannel>,
    pub l2cap_profile: Option<L2CapDeviceProfile>,
}
```

### 2. Add API Endpoint
```
GET /api/devices/{mac}/l2cap
  → L2CapDeviceProfile
  
GET /api/l2cap/summary
  → All L2CAP channels across all devices
```

### 3. Integration Flow
```
BLE Scan
  ↓
Detect L2CAP Channels (if supported)
  ↓
Analyze PSM values
  ↓
Profile device connections
  ↓
Store in database
  ↓
Display in API
```

---

## Database Schema

```sql
CREATE TABLE l2cap_channels (
    id INTEGER PRIMARY KEY,
    device_id INTEGER NOT NULL,
    channel_id INTEGER NOT NULL,
    psm INTEGER NOT NULL,
    state TEXT,
    tx_bytes INTEGER,
    rx_bytes INTEGER,
    mtu INTEGER,
    opened_at DATETIME,
    last_activity DATETIME,
    FOREIGN KEY(device_id) REFERENCES devices(id)
);

CREATE TABLE l2cap_psm_map (
    psm INTEGER PRIMARY KEY,
    service_name TEXT,
    is_dynamic BOOLEAN
);
```

---

## Benefits

✅ **Service Discovery** - Know exactly what services are active
✅ **Bandwidth Profiling** - Track data per service type
✅ **Connection Analysis** - Understand connection configuration
✅ **Troubleshooting** - Identify which service is causing issues
✅ **Security** - Detect unusual channel activity
✅ **Performance** - Optimize MTU per channel type

---

## References

- **L2CAP Spec:** https://www.bluetooth.com/specifications/specs/logical-link-control-and-adaptation-protocol/
- **PSM Registry:** https://www.bluetooth.com/specifications/assigned-numbers/logical-link-control/
- **CoreBluetooth (macOS):** https://developer.apple.com/documentation/corebluetooth/cbl2capchannel
- **Linux HCI:** https://www.kernel.org/doc/html/latest/networking/bluetooth/index.html
