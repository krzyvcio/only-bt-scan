btleplug                  → Standard BLE (cross-platform)
windows_hci.rs           → Raw HCI packets (low-level hardware)
windows_bluetooth.rs     → Windows Bluetooth API (device info)
android_ble_bridge.rs    → Android BLE (bridge, dalsze rozszerzenie)
bluey_integration.rs     → Bluey library (alternatywny stack)
core_bluetooth_integration → macOS CoreBluetooth (Apple devices)
hci_scanner.rs           → Direct HCI commands
hci_realtime_capture.rs  → Real-time HCI sniffing
advertising_parser.rs    → Deep packet analysis
ble_security.rs          → Security/pairing info
ble_uuids.rs             → Service classification
vendor_protocols.rs      → Vendor-specific detection
l2cap_analyzer.rs        → L2CAP channel detection
raw_packet_parser.rs     → Raw frame analysis
advertising_parser.rs        ← Parsuje surowe pakiety
hci_realtime_capture.rs      ← Przechwytuje HCI real-time
vendor_protocols.rs          ← Detektuje specjalne frame'i
ble_uuids.rs                 ← Klasyfikuje usługi
ble_security.rs              ← Analizuje bezpieczeństwo
l2cap_analyzer.rs            ← Wydobywa L2CAP kanały
raw_packet_parser.rs         ← Analizuje raw data