# HCI Support - Bluetooth 5.0+ Raw Packet Capture

## Overview

Implementacja obsługi HCI (Host Controller Interface) do czytania raw Bluetooth 5.0+ eventów i L2CAP pakietów.

## Moduły

### hci_packet_parser.rs (170 linii)
Parser dla raw HCI packets i L2CAP struktur

**Struktury:**
- `L2CapPacket` - L2CAP packet z payload
- `HciEvent` - Raw HCI event
- `HciEventDecoded` - Decoded event types
- `AdvertisingReport` - BLE advertising report
- `ExtendedAdvertisingReport` - BT 5.0+ extended advertising
- `PhyType` - PHY identifiers (1M, 2M, Coded)
- `HciPacketParser` - Parser główny
- `HciPacketStats` - Statystyki

**CID Mapping (L2CAP Channel IDs):**
```
0x0001 - L2CAP Signaling
0x0003 - RFCOMM (Serial Port)
0x001F - Attribute Protocol (ATT)
0x0021 - Enhanced ATT (EATT) - BT 5.0+
0x0023 - Security Manager (SMP)
0x0024 - BR/EDR Security Manager
```

**HCI Event Codes:**
```
0x05   - Disconnection Complete
0x3E   - LE Meta Event (sub-events)
  0x02 - LE Advertising Report
  0x03 - LE Connection Update Complete
  0x13 - LE Extended Advertising Report (BT 5.0+)
0x13   - Number of Completed Packets
```

### hci_scanner.rs (185 linii)
High-level HCI scanning interface

**Struktury:**
- `HciScannerConfig` - Konfiguracja scannera
- `HciScanner` - Scanner główny
- `HciScanResult` - Wyniki skanowania
- `L2CapPacketInfo` - L2CAP packet z metadata
- `HciScanStatistics` - Statystyki skanowania

**Metody:**
- `simulate_hci_event()` - Symulacja HCI event
- `simulate_l2cap_packet()` - Symulacja L2CAP packet
- `get_stats()` - Pobierz statystyki
- `get_results()` - Pobierz wszystkie wyniki

## Dependencies

```toml
hci = { version = "0.1", optional = true }
```

Dodaj feature do Cargo.toml:
```toml
[features]
hci_support = ["dep:hci"]
```

## API Endpoints

### GET /api/hci-scan
Zwraca HCI events i L2CAP packets

**Response:**
```json
{
  "hci_events": [
    {
      "event_code": 5,
      "event_name": "Disconnection Complete",
      "parameter_length": 4,
      "parameters": [0, 1, 2, 19],
      "decoded": null
    }
  ],
  "l2cap_packets": [
    {
      "packet": {
        "payload_length": 4,
        "channel_id": 31,
        "packet_type": "Data",
        "payload": [1, 2, 3, 4],
        "sdu_length": null,
        "sar_flags": 0
      },
      "timestamp": 1707856000000,
      "source_mac": "AA:BB:CC:DD:EE:FF",
      "dest_cid": 0
    }
  ],
  "stats": {
    "total_events": 2,
    "total_l2cap_packets": 2,
    "events_by_type": {
      "Disconnection Complete": 1,
      "LE Meta Event": 1
    },
    "packets_by_cid": {
      "31": 1,
      "35": 1
    },
    "extended_advertising_reports": 0,
    "connection_updates": 0,
    "disconnections": 1,
    "avg_packet_size": 5.0
  }
}
```

## Bluetooth 5.0+ Features

### Extended Advertising
- `LeExtendedAdvertisingReport` - Event code 0x3E, sub-event 0x13
- Primary PHY (1M, 2M, Coded)
- Secondary PHY
- TX Power
- Periodic Advertising Interval

### Enhanced ATT (EATT)
- CID 0x0021
- Multiple simultaneous L2CAP channels per connection
- Improved throughput

### LE Coded PHY
- Long range (250 Kbit/s or 125 Kbit/s)
- PHY type value: 3

### Connection Parameters
- Connection interval (1.25ms - 4s)
- Peripheral latency
- Supervision timeout

## Usage Examples

### Czytanie HCI events
```rust
use crate::hci_scanner::{HciScanner, HciScannerConfig};

let mut scanner = HciScanner::default();

// Simulate/capture HCI event
let event = scanner.simulate_hci_event(0x05, &[0x00, 0x01, 0x02, 0x13]);
println!("Event: {:?}", event);
```

### Parsowanie L2CAP packets
```rust
let mut scanner = HciScanner::default();

// L2CAP packet: Length=4, CID=0x001F (ATT), Payload=[0x01, 0x02, 0x03, 0x04]
let data = vec![0x04, 0x00, 0x1F, 0x00, 0x01, 0x02, 0x03, 0x04];
let packet_info = scanner.simulate_l2cap_packet(&data, Some("AA:BB:CC:DD:EE:FF".to_string()))?;

println!("L2CAP Packet: CID=0x{:04X} ({})", 
         packet_info.packet.channel_id, 
         packet_info.packet.channel_name());
```

### CID Filtering
```rust
let config = HciScannerConfig {
    cid_filter: Some(vec![0x001F, 0x0023]), // Only ATT and SMP
    ..Default::default()
};
let mut scanner = HciScanner::new(config);
```

## Platform Support

### Linux
- Direct HCI socket access (requires CAP_NET_RAW)
- Raw packet sniffing
- Full HCI command support

### macOS/iOS
- CoreBluetooth framework integration
- Limited direct HCI access (via system APIs)

### Windows
- WinSock integration (planned)
- Native Bluetooth API

## Statistics Tracked

- Total HCI events
- Total L2CAP packets
- Event type distribution
- Packets by CID
- Extended advertising report count
- Connection updates count
- Disconnections count
- Average packet size

## Testing

Moduł zawiera unit tests:
```bash
cargo test hci_scanner
cargo test hci_packet_parser
```

## Future Enhancements

1. Real socket-based HCI capture (not just simulation)
2. Packet reassembly dla fragmented L2CAP data
3. Connection state tracking
4. RSSI quality metrics per L2CAP channel
5. Periodic advertising parsing
6. Mesh network support
7. Isochronous channels (LE audio)

## References

- Bluetooth 5.0 Specification
- https://crates.io/crates/hci
- BLE L2CAP Protocol (Core Spec Vol 3, Part A)
- HCI Packet Format (Core Spec Vol 4, Part E)
