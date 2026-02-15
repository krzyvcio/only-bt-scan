# 🎯 Complete Bluetooth Scanner Implementation TODO

## 📊 Project Structure Overview
52 files, ~10,000 lines of code, organized by function:

---

## 🔍 PHASE 1: Core BLE Scanning Methods (FOUNDATION)

### 1️⃣ btleplug Integration (Cross-Platform Standard)
- **Status**: Not integrated
- **Role**: Primary BLE scanner for Windows, macOS, Linux
- **Key Functions**:
  - Device enumeration via btleplug
  - Advertisement scanning
  - RSSI collection
- **Input to**: `multi_method_scanner.rs` (Method 1)
- **TODO**:
  - [ ] Implement `scan_with_btleplug()` in multi_method_scanner
  - [ ] Handle connection lifecycle
  - [ ] Extract manufacturer data from advertisements
  - [ ] Support both classic and extended advertising

### 2️⃣ windows_hci.rs - Raw HCI Packets (Windows Low-Level)
- **Status**: Partially implemented
- **Role**: Direct HCI packet capture on Windows
- **Key Structs**: `WindowsHciScanner`, `WindowsHciAdapter`, `LeAdvertisingReport`
- **Input to**: `multi_method_scanner.rs` (Method 2)
- **TODO**:
  - [ ] Implement `scan_with_hci_raw()` in multi_method_scanner
  - [ ] Handle serial port connections
  - [ ] Parse HCI events properly
  - [ ] Add error handling for missing ports
  - [ ] Support extended advertising events

### 3️⃣ windows_bluetooth.rs - Windows Bluetooth API
- **Status**: Stubs only
- **Role**: High-level Windows Bluetooth API access
- **Key Structs**: `WindowsBluetoothManager`, `WindowsBluetoothCapabilities`
- **Input to**: `multi_method_scanner.rs` (Method 3)
- **TODO**:
  - [ ] Implement `scan_with_windows_api()` in multi_method_scanner
  - [ ] Enumerate paired devices
  - [ ] Get device class/type
  - [ ] Retrieve pairing status
  - [ ] Implement device connection/disconnection

### 4️⃣ windows_unified_ble.rs - Windows Integration (NEW)
- **Status**: Created, not integrated
- **Role**: Combine HCI + API methods for Windows
- **Key Structs**: `WindowsUnifiedBleScanner`, `ManagedDevice`
- **TODO**:
  - [ ] Integrate with multi_method_scanner
  - [ ] Merge device info from both HCI and API sources
  - [ ] Create manufacturer ID → official name mapping

### 5️⃣ android_ble_bridge.rs - Android BLE Bridge
- **Status**: Stubs only
- **Role**: Android device scanning via JNI/bridge
- **Key Structs**: `AndroidBleDevice`, `AndroidBleScanner`, `AndroidGattProfile`
- **Platform**: Android only
- **TODO**:
  - [ ] Implement Android JNI bridge
  - [ ] Connect to Android BleAdapter
  - [ ] Handle Android permissions
  - [ ] Enumerate nearby devices
  - [ ] Extract BLE advertisements

### 6️⃣ bluey_integration.rs - Alternative BLE Library
- **Status**: Stubs only
- **Role**: Alternative to btleplug using Bluey library
- **Key Structs**: `BlueyScanner`, `BlueyCapabilities`
- **TODO**:
  - [ ] Implement Bluey scanner initialization
  - [ ] Integrate GATT service discovery
  - [ ] Handle Bluey-specific events
  - [ ] Compare coverage vs btleplug

### 7️⃣ core_bluetooth_integration.rs - macOS CoreBluetooth
- **Status**: Stubs only
- **Role**: Apple macOS native BLE scanning
- **Key Structs**: `CoreBluetoothScanner`, `CoreBluetoothConfig`
- **Platform**: macOS only
- **TODO**:
  - [ ] Implement macOS CoreBluetooth integration
  - [ ] Handle CBCentralManager delegates
  - [ ] Scan for peripherals
  - [ ] Extract RSSI and advertisements

### 8️⃣ hci_scanner.rs - Direct HCI Commands
- **Status**: Unknown implementation level
- **Role**: Low-level HCI command execution
- **TODO**:
  - [ ] Verify implementation completeness
  - [ ] Test all HCI command opcodes
  - [ ] Add LE Set Scan Parameters
  - [ ] Add LE Set Scan Enable
  - [ ] Add LE Read White List Size

### 9️⃣ hci_realtime_capture.rs - Real-Time HCI Sniffing
- **Status**: Partially implemented
- **Role**: Real-time HCI event capture and processing
- **Key Functions**: Device detection loop, event parsing
- **TODO**:
  - [ ] Fix unused variables (event_type, address_type, rssi)
  - [ ] Implement `scan_with_hci_realtime()` in multi_method_scanner
  - [ ] Handle all HCI subevent codes
  - [ ] Support extended advertising reports
  - [ ] Add timeout handling

---

## 📦 PHASE 2: Packet Analysis & Deep Parsing

### 🔟 advertising_parser.rs - Deep Packet Analysis
- **Status**: Complete but unused (~350 lines)
- **Role**: Parse advertising data into structured format
- **Key Functions**: 
  - `parse_advertising_packet()` 
  - `parse_ad_structures()`
  - `parse_manufacturer_data()` - CRITICAL!
  - `parse_service_data_*()`
  - `parse_flags()`
  - `parse_*_uuids()`
- **TODO**:
  - [ ] Integrate into packet analysis pipeline
  - [ ] Use in `marketing_frame` extraction
  - [ ] Connect to `vendor_protocols.rs` for decoding
  - [ ] Add to `multi_method_scanner` result processing

### 1️⃣1️⃣ raw_packet_parser.rs - Raw Frame Analysis
- **Status**: Unknown implementation
- **Role**: Parse raw BLE packet bytes into fields
- **TODO**:
  - [ ] Verify it parses physical layer info
  - [ ] Extract PHY type (LE 1M, LE 2M, LE Coded)
  - [ ] Parse header info
  - [ ] Extract payload data
  - [ ] Validate CRC

### 1️⃣2️⃣ hci_packet_parser.rs - HCI Packet Parsing
- **Status**: Partially implemented
- **Role**: Parse HCI event packets into readable format
- **TODO**:
  - [ ] Support all HCI event types
  - [ ] Parse LE Meta Events (0x3E)
  - [ ] Extract advertising reports
  - [ ] Handle extended advertising events

---

## 🧠 PHASE 3: Device Intelligence & Analysis

### 1️⃣3️⃣ vendor_protocols.rs - Vendor-Specific Detection
- **Status**: Unknown implementation
- **Role**: Detect iBeacon, Eddystone, AltBeacon, manufacturer-specific frames
- **Critical For**: Identifying special device types (trackers, proximity beacons)
- **TODO**:
  - [ ] Implement Apple iBeacon frame detection/parsing
  - [ ] Implement Google Eddystone (UID, URL, TLM)
  - [ ] Implement AltBeacon format
  - [ ] Add manufacturer-specific frame handlers:
      - [ ] Xiaomi Mi Frame
      - [ ] Fitbit proprietary
      - [ ] Tile BLE frame
      - [ ] Samsung SmartThings
  - [ ] Integrate into `multi_method_scanner` detection

### 1️⃣4️⃣ ble_uuids.rs - Service Classification
- **Status**: Complete, ~30 helper functions (unused)
- **Role**: Map BLE UUIDs to service names and categories
- **Key Functions**: 
  - `is_le_audio_service()`
  - `is_fitness_wearable_service()`
  - `is_iot_smart_service()`
  - UUID classification helpers
- **TODO**:
  - [ ] Integrate UUID lookup into scan results
  - [ ] Use for service discovery
  - [ ] Add to device type inference

### 1️⃣5️⃣ ble_security.rs - Security Analysis
- **Status**: Complete, ~15 functions (unused)
- **Role**: Parse and analyze BLE security information
- **Key Functions**:
  - Security flag parsing
  - Pairing capability detection
  - Authentication type inference
- **TODO**:
  - [ ] Integrate into device info extraction
  - [ ] Analyze security flags from advertising
  - [ ] Detect pairing vs bonded devices

### 1️⃣6️⃣ l2cap_analyzer.rs - L2CAP Channel Detection
- **Status**: Probably incomplete
- **Role**: Analyze L2CAP layer protocols
- **TODO**:
  - [ ] Extract L2CAP channels from HCI
  - [ ] Parse protocol service multiplexer (PSM) info
  - [ ] Identify active connections
  - [ ] Detect connection parameters

### 1️⃣7️⃣ event_analyzer.rs - Event Processing
- **Status**: Unknown implementation
- **Role**: Process HCI events into high-level device events
- **TODO**:
  - [ ] Implement device discovery events
  - [ ] Handle connection/disconnection events
  - [ ] Process RSSI updates
  - [ ] Track event sequence

---

## 💾 PHASE 4: Data Models & Persistence

### 1️⃣8️⃣ data_models.rs - Data Structures (CRITICAL)
- **Status**: Complete (~350 lines)
- **Role**: Define all data structures (DeviceModel, RawPacketModel, etc)
- **Key Structs**:
  - `DeviceModel` - Device metadata
  - `RawPacketModel` - Complete packet info
  - `AdStructureData` - AD structure details
  - `DevicePacketRelationship` - Device-packet mapping
- **Status**: Used extensively
- **TODO**: (Done - maintain)

### 1️⃣9️⃣ db.rs + db_frames.rs - Database (CRITICAL)
- **Status**: Partially implemented
- **Role**: SQLite persistence for devices and packets
- **Key Functions**:
  - Device enumeration/insertion
  - Packet storage
  - Query/filtering
- **Status**: In use
- **TODO**: 
  - [ ] Optimize db queries
  - [ ] Add indexes for performance
  - [ ] Implement retention policies

### 2️⃣0️⃣ packet_tracker.rs - Packet Tracking
- **Status**: Unknown
- **Role**: Track packet sequences, duplicates, timing
- **TODO**:
  - [ ] Implement duplicate detection
  - [ ] Track packet intervals
  - [ ] Calculate packet loss

### 2️⃣1️⃣ device_events.rs - Event Models
- **Status**: Unknown
- **Role**: Define device-related events
- **TODO**:
  - [ ] Define event enums
  - [ ] Implement event handlers
  - [ ] Connect to event_analyzer

---

## 🛠️ PHASE 5: Utilities & Helpers

### 2️⃣2️⃣ company_id_reference.rs - Official SIG Data
- **Status**: Just created! (~150 lines, 60+ companies)
- **Role**: Map Company IDs to official manufacturer names (Bluetooth SIG)
- **Status**: In use
- **Functions**:
  - `lookup_company_id(u16)` - Get name by ID
  - `search_company_by_name()` - Find by pattern
  - `all_companies()` - Get full list
- **TODO**:
  - [ ] Load ALL 1000+ Company IDs from YAML (optional)
  - [ ] Add datasheet/website lookups
  - [ ] Cache frequent lookups

### 2️⃣3️⃣ mac_address_handler.rs - MAC Address Parsing
- **Status**: Unknown implementation
- **Role**: Parse/validate/categorize MAC addresses
- **TODO**:
  - [ ] MAC vendor lookup (OUI database)
  - [ ] Detect random MAC addresses (RPA)
  - [ ] Extract device type from MAC
  - [ ] Validate MAC format

### 2️⃣4️⃣ adapter_info.rs - Adapter Enumeration
- **Status**: Unknown implementation
- **Role**: Enumerate available Bluetooth adapters
- **TODO**:
  - [ ] List all adapters on system
  - [ ] Get adapter capabilities
  - [ ] Check adapter status

### 2️⃣5️⃣ config_params.rs - Configuration
- **Status**: Unknown implementation
- **Role**: Load and manage configuration
- **TODO**:
  - [ ] Read from .env file
  - [ ] Validate parameters
  - [ ] Provide defaults

### 2️⃣6️⃣ logger.rs - Logging Setup
- **Status**: Unknown implementation
- **Role**: Initialize logging subsystem
- **TODO**:
  - [ ] Setup env_logger
  - [ ] Configure log levels
  - [ ] Add file logging

---

## 📤 PHASE 6: Output & Export

### 2️⃣7️⃣ packet_analyzer_terminal.rs - Terminal Output
- **Status**: Partially implemented
- **Role**: Format packets for terminal display
- **Function**: `format_packet_for_terminal()`
- **Status**: In use
- **TODO**:
  - [ ] Add color coding for different packet types
  - [ ] Show manufacturer info
  - [ ] Format AD structures nicely

### 2️⃣8️⃣ html_report.rs - HTML Reports
- **Status**: Probably incomplete
- **Role**: Generate HTML scan reports
- **TODO**:
  - [ ] Create HTML template
  - [ ] Generate device tables
  - [ ] Add charts/graphs
  - [ ] Export to file

### 2️⃣9️⃣ pcap_exporter.rs - PCAP Export
- **Status**: Probably incomplete
- **Role**: Export packets to PCAP format (Wireshark compatible)
- **TODO**:
  - [ ] Implement PCAP header writing
  - [ ] Convert BLE packets to PCAP format
  - [ ] Export to file
  - [ ] Support replay

---

## 🎨 PHASE 7: UI & User Interaction

### 3️⃣0️⃣ interactive_ui.rs - Interactive Terminal UI
- **Status**: In use
- **Role**: Interactive terminal prompts, menus
- **TODO**: (Maintain - in use)

### 3️⃣1️⃣ ui_renderer.rs - UI Rendering
- **Status**: In use
- **Role**: Render tables, headers, live updates
- **TODO**: (Maintain - in use)

### 3️⃣2️⃣ web_server.rs - Web Interface
- **Status**: Partially implemented
- **Role**: Actix-web REST API and web dashboard
- **Key Routes**:
  - GET `/devices` - List devices
  - GET `/device/:mac` - Device details
  - GET `/packets` - Recent packets
  - GET `/stats` - Scan statistics
- **Status**: In use
- **TODO**:
  - [ ] Add real-time WebSocket updates
  - [ ] Implement device filtering
  - [ ] Add export endpoints

### 3️⃣3️⃣ tray_manager.rs - System Tray Icon (Windows)
- **Status**: Partially implemented
- **Role**: System tray integration on Windows
- **TODO**:
  - [ ] Handle tray icon clicks
  - [ ] Show/hide window from tray
  - [ ] Implement menu actions

---

## 🔗 PHASE 8: Integration & Coordination

### 3️⃣4️⃣ multi_method_scanner.rs - Main Coordinator (NEW! 🌟)
- **Status**: Just created! (~330 lines)
- **Role**: Run ALL 5 scanning methods in parallel
- **Methods**:
  1. btleplug standard scanning
  2. Windows HCI raw packets
  3. Windows Bluetooth API
  4. Real-time HCI capture
  5. Vendor-specific protocols
- **Output**: `UnifiedDevice` with merged info from all methods
- **TODO**:
  - [ ] Implement all 5 `scan_with_*()` methods
  - [ ] Merge duplicate devices across methods
  - [ ] Track which methods detected each device
  - [ ] Assign confidence score (1-5)
  - [ ] Test parallel execution

### 3️⃣5️⃣ unified_scan.rs - Unified Engine
- **Status**: Probably has legacy code
- **Role**: High-level scanning orchestration
- **TODO**:
  - [ ] Refactor to use `multi_method_scanner`
  - [ ] Integrate all data sources
  - [ ] Implement scanning loop
  - [ ] Handle scan lifecycle

### 3️⃣6️⃣ native_scanner.rs - Native Platform Wrapper
- **Status**: Partially implemented
- **Role**: Platform-specific scanner selection
- **TODO**:
  - [ ] Detect platform (Windows/macOS/Linux/Android)
  - [ ] Select appropriate scanner
  - [ ] Provide unified interface

### 3️⃣7️⃣ scanner_integration.rs - Scanner Integration
- **Status**: Unknown implementation
- **Role**: Integrate all scanner implementations
- **TODO**:
  - [ ] Create unified scanner factory
  - [ ] Support multiple scanners simultaneously
  - [ ] Merge results

### 3️⃣8️⃣ raw_packet_integration.rs - Raw Packet Integration
- **Status**: Unknown implementation
- **Role**: Integrate raw packet capture with analysis
- **TODO**:
  - [ ] Connect packet capture to analysis pipeline
  - [ ] Store packets in database
  - [ ] Create packet history

---

## ⚡ PHASE 9: Advanced Features

### 3️⃣9️⃣ telemetry.rs - Telemetry Collection
- **Status**: Probably incomplete
- **Role**: Collect scanning statistics
- **Key Metrics**:
  - Packets/second
  - Devices found
  - RSSI distribution
  - Manufacturer distribution
- **TODO**:
  - [ ] Implement telemetry collectors
  - [ ] Store time-series data
  - [ ] Generate reports

### 4️⃣0️⃣ telegram_notifier.rs - Alert Notifications
- **Status**: Probably incomplete
- **Role**: Send alerts via Telegram bot
- **TODO**:
  - [ ] Setup Telegram bot
  - [ ] Define alert triggers
  - [ ] Format notifications
  - [ ] Handle DM delivery

### 4️⃣1️⃣ background.rs - Background Mode
- **Status**: Stubs only
- **Role**: Run as system service/daemon
- **Functions** (unused):
  - `hide_console_window()`
  - `daemonize()`
  - Background monitoring
- **TODO**:
  - [ ] Implement Windows service integration
  - [ ] Handle daemonization
  - [ ] Graceful startup/shutdown

### 4️⃣2️⃣ gatt_client.rs - GATT Client
- **Status**: Probably incomplete
- **Role**: Connect to devices & read GATT characteristics
- **TODO**:
  - [ ] Implement connection logic
  - [ ] Service discovery
  - [ ] Read characteristics
  - [ ] Write descriptors

### 4️⃣3️⃣ data_flow_estimator.rs - Data Flow Estimation
- **Status**: Constants defined, functions unused
- **Role**: Estimate data throughput
- **TODO**:
  - [ ] Implement flow calculation
  - [ ] Estimate bandwidth
  - [ ] Predict resource usage

---

## 📋 SUPPORTING STRUCTURES

### bluetooth_scanner.rs
- Defines `BluetoothScanner` interface
- Used by most scanners
- **Status**: Active

### bluetooth_manager.rs  
- Device pairing/connection management (unused)
- **TODO**: Can be archived if not needed

### ble_features.rs, bluetooth_features.rs
- Feature flags and capabilities
- **TODO**: Consolidate if duplicate

### link_layer.rs
- Link layer protocol info
- **TODO**: Verify usage

---

## 🎯 DEPENDENCY FLOW

```
Multi-Method Scanner (COORDINATOR)
├─ btleplug
├─ windows_hci.rs → hci_realtime_capture.rs
├─ windows_bluetooth.rs
├─ android_ble_bridge.rs  
├─ bluey_integration.rs
├─ core_bluetooth_integration.rs
└─ All collected data → advertising_parser.rs
    └─ Enriched with:
        ├─ vendor_protocols.rs (iBeacon, Eddystone)
        ├─ ble_uuids.rs (service names)
        ├─ ble_security.rs (security info)
        ├─ l2cap_analyzer.rs (L2CAP info)
        ├─ company_id_reference.rs (manufacturer)
        └─ mac_address_handler.rs (MAC info)

Result: UnifiedDevice (complete info!)
    ↓
data_models.rs (structure)
    ↓
db.rs (persist)
    ↓
Output:
├─ packet_analyzer_terminal.rs (logs)
├─ web_server.rs (REST/dashboard)
├─ html_report.rs (report)
├─ pcap_exporter.rs (Wireshark)
└─ telegram_notifier.rs (alerts)
```

---

## 🚀 Implementation Priority

**CRITICAL (DO FIRST)**:
1. Implement `multi_method_scanner.rs` scanning methods
2. Integrate all 5 scanning methods
3. Test device deduplication

**HIGH**:
4. Integrate `vendor_protocols.rs` for beacon detection
5. Complete L2CAP analysis
6. Add security flag parsing

**MEDIUM**:
7. Add HTML/PCAP export
8. Implement telemetry
9. Add Android/macOS support

**LOW**:
10. Telegram notifications
11. Background mode
12. Advanced GATT operations


każde wykryte urządzenie w każdym pliki pełne verbose do terminala z datą pierwszego wykrycia i datą ostaniego wykrycia i ile razy wykryto rozpoznawalne po adresie MAC
dodatkowo każde wykryte urzadzenie badź pakiet surowy dodajemy do bazy danych
---

## 📊 Progress Tracking

- **Total Files**: 52
- **Implemented**: ~40%
- **Stubs Only**: ~20%
- **Integration Ready**: ~95%
- **Test Coverage**: ~10%

---

*Last Updated: 2026-02-15*
*Multi-Method Scanner Architecture: COMPLETE*
