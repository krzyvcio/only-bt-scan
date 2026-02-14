# 🛣️ IMPLEMENTATION ROADMAP - BLE Scanner v0.4.0+

**Last Updated:** 2024
**Total Estimated Work:** 75-120 hours
**Current Status:** v0.3.0 (95% complete, but with critical bugs)

---

## 📍 CURRENT STATE

### ✅ What Works (v0.3.0)
- Basic BLE scanning across platforms (Windows, Linux, macOS)
- Device detection and RSSI tracking
- Database persistence (SQLite)
- Web UI with device listing
- Telegram notifications
- Packet tracking and telemetry
- Data flow estimation
- Event analysis
- Windows native API integration

### ❌ What's Broken
- **Compilation errors** in main.rs, logger.rs, ui_renderer.rs
- Multiple placeholder implementations returning empty results
- Unused dependencies cluttering Cargo.toml
- Missing critical Web API endpoints
- Incomplete frontend UI (missing 4 tabs)
- No unit tests infrastructure

### ⚠️ What's Missing
- macOS CoreBluetooth proper integration
- RPA deduplication system
- Extended Advertising parsing
- BR/EDR scanning (Linux)
- L2CAP extraction (macOS/HCI)
- Database connection pooling
- Proper error handling in shutdown sequence
- Observability (metrics, structured logging, log rotation)
- Configuration validation

---

## 🚨 PHASE 0: CRITICAL FIXES (Days 1-2)

### 0.1 Compilation Error Fixes
**Status:** 🔴 TODO
**Effort:** ~1.5 hours
**Files:** `src/main.rs`, `src/logger.rs`, `src/ui_renderer.rs`

```rust
// main.rs - Change return type
- async fn main() -> Result<(), Box<dyn Error>> {
+ async fn main() -> Result<(), anyhow::Error> {

// logger.rs - Use consistent error handling
- pub fn clear_content_area() -> Result<(), Box<dyn std::error::Error>> {
+ pub fn clear_content_area() -> Result<(), anyhow::Error> {

// main.rs - Fix database connection in Ctrl+C handler
// Currently: 1 critical bracket mismatch + 3 type mismatches
// Fix: Proper Result handling, unified error types
```

**Checklist:**
- [ ] Change all error types to `anyhow::Error`
- [ ] Fix Ctrl+C handler bracket mismatch (line 279-336)
- [ ] Update database connection error handling
- [ ] Verify `cargo check` passes
- [ ] Run `cargo test --doc` if tests exist

**PR Title:** "Fix: Resolve compilation errors blocking v0.4.0 development"

---

### 0.2 Dependency Cleanup
**Status:** 🔴 TODO
**Effort:** ~2-4 hours per feature
**Files:** `Cargo.toml`, core modules

**Decision Tree:**

```
For each unused dependency (trouble, android-ble, btsnoop-extcap, bluest, hci):
├─ If removing:
│  ├─ Remove from Cargo.toml
│  ├─ Remove feature flag
│  ├─ Verify no references in code
│  └─ Test compilation
│
└─ If implementing:
   ├─ Create feature flag: [features] feature_name = ["dep:crate"]
   ├─ Add #[cfg(feature = "feature_name")] guards
   ├─ Implement actual functionality
   └─ Add tests
```

**Recommendation:** 
- ✂️ **REMOVE:** `trouble`, `android-ble`, `btsnoop-extcap` (no references, no time)
- 🔧 **KEEP & FIX:** `bluest` (L2CAP support), `hci` (HCI parsing)

**Checklist:**
- [ ] Document decision for each dependency
- [ ] Update Cargo.toml
- [ ] Update feature flags in code
- [ ] Test compilation
- [ ] Update docs/DEPENDENCIES.md

**PR Title:** "Refactor: Clean up unused dependencies and organize features"

---

## 📦 PHASE 1: FIX PLACEHOLDER IMPLEMENTATIONS (Days 3-5)

### 1.1 BLE Scanner Placeholders
**Status:** 🔴 TODO
**Effort:** ~20-30 hours
**Files:** Multiple core modules

#### 1.1.1 Bluey Integration
**File:** `src/bluey_integration.rs` (Lines 84-102)
**Current:** Returns `Ok(Vec::new())`
**Fix:** 
```rust
// Real Bluey implementation with async/await
async fn scan_bluey_impl(&self) -> Result<Vec<DeviceModel>, Box<dyn Error>> {
    let manager = bluey::Manager::new().await?;
    let adapters = manager.adapters().await?;
    
    let mut devices = Vec::new();
    for adapter in adapters {
        let discovered = adapter.discover(self.config.scan_duration).await?;
        devices.extend(discovered.into_iter().map(to_device_model));
    }
    Ok(devices)
}
```
**Lines:** ~40-50 lines
**Estimated time:** 4-6 hours (+ testing)
**Blockers:** Need Bluey API docs

#### 1.1.2 GATT Client Discovery
**File:** `src/gatt_client.rs` (Line 129+)
**Current:** Says "simulate" in comments
**Fix:** Real GATT service discovery via btleplug
**Lines:** ~80-100 lines
**Estimated time:** 4-6 hours

#### 1.1.3 HCI Socket Implementation
**File:** `src/hci_sniffer_example.rs` (Lines 44-83)
**Current:** fd = -1 placeholder
**Fix:** Real socket creation with nix crate
**Lines:** ~60-80 lines
**Estimated time:** 6-8 hours
**Platform:** Linux only (Windows/macOS use different APIs)

#### 1.1.4 BR/EDR Scanning
**File:** `src/bluetooth_scanner.rs` (Line 490)
**Current:** Returns `Ok(Vec::new())` with warn
**Fix:** Use bluer crate for Linux BR/EDR
**Lines:** ~40-60 lines
**Estimated time:** 3-5 hours
**Platform:** Linux only

#### 1.1.5 L2CAP Extraction
**File:** `src/l2cap_analyzer.rs` (Lines 271+, 285+)
**Current:** Returns `Ok(Vec::new())`
**Fix:** Platform-specific L2CAP parsing
- macOS: Use Core Bluetooth L2CAP APIs
- Linux/HCI: Parse HCI packets
**Lines:** ~100-150 lines
**Estimated time:** 8-10 hours
**Platform:** macOS + Linux

#### 1.1.6 Tray Icon
**File:** `src/tray_manager.rs` (Line 117)
**Current:** `get_app_icon()` returns empty Vec
**Fix:** Embed actual icon asset
**Action:** Add `.ico` file to project, include in binary
**Lines:** ~5-10 lines (config change)
**Estimated time:** 1-2 hours

**Summary Table:**

| Module | Function | Time | Priority | Platform |
|--------|----------|------|----------|----------|
| bluey_integration | scan_bluey_impl | 4-6h | High | All |
| gatt_client | discover_services | 4-6h | High | All |
| hci_sniffer | HciSocket::open | 6-8h | Medium | Linux |
| bluetooth_scanner | scan_bredr | 3-5h | Medium | Linux |
| l2cap_analyzer | extract_l2cap | 8-10h | High | All |
| tray_manager | get_app_icon | 1-2h | Low | Windows |
| **TOTAL** | | **26-37h** | | |

---

## 🌐 PHASE 2: WEB API IMPLEMENTATION (Days 6-7)

### 2.1 Missing Endpoints
**Status:** 🟡 TODO
**Effort:** ~4-6 hours
**File:** `src/web_server.rs`

#### 2.1.1 GET /api/telemetry
**Response Schema:**
```json
{
  "devices": [
    {
      "mac": "AA:BB:CC:DD:EE:FF",
      "packet_count": 147,
      "avg_rssi": -65,
      "rssi_min": -75,
      "rssi_max": -55,
      "latencies": {
        "min_ms": 12,
        "max_ms": 8934,
        "avg_ms": 245,
        "stddev_ms": 342
      },
      "anomalies": ["rssi_dropout", "signal_degradation"],
      "last_updated": "2024-01-15T10:30:45Z"
    }
  ],
  "scan_duration_seconds": 300,
  "total_devices": 42,
  "total_packets": 5847
}
```
**Implementation:**
- Access `TelemetryCollector` singleton
- Iterate through tracked devices
- Aggregate packet/RSSI statistics
- Build response JSON

**Lines:** ~40-50
**Time:** 1-1.5 hours

#### 2.1.2 GET /api/data-flow
**Response Schema:**
```json
{
  "devices": [
    {
      "mac": "AA:BB:CC:DD:EE:FF",
      "detected_protocol": "Meshtastic",
      "connection_state": "DataTransfer",
      "estimated_throughput_bytes_sec": 1024,
      "reliability": 0.92,
      "confidence": 0.87,
      "last_packet_time": "2024-01-15T10:30:45Z"
    }
  ],
  "analysis_timestamp": "2024-01-15T10:31:00Z"
}
```
**Implementation:**
- Access `DataFlowEstimator` singleton
- Query analyzed flows by device MAC
- Return protocol detection + metrics

**Lines:** ~40-50
**Time:** 1-1.5 hours

#### 2.1.3 GET /api/packet-sequence/{mac}
**Response Schema:**
```json
{
  "device_mac": "AA:BB:CC:DD:EE:FF",
  "total_packets": 147,
  "time_span_seconds": 245,
  "packets": [
    {
      "packet_id": 1,
      "timestamp_ms": 1000,
      "rssi": -65,
      "delta_ms": 0,
      "channel": 37,
      "protocol": "BLE"
    },
    {
      "packet_id": 2,
      "timestamp_ms": 1050,
      "rssi": -64,
      "delta_ms": 50,
      "channel": 38,
      "protocol": "BLE"
    }
  ],
  "pattern": "regular",
  "avg_interval_ms": 50,
  "gaps": []
}
```
**Implementation:**
- Query `GlobalPacketTracker` by MAC
- Fetch packet timeline data
- Calculate deltas and patterns
- Identify gaps

**Lines:** ~50-60
**Time:** 1.5-2 hours

#### 2.1.4 GET /api/rpa-history (Future)
**Note:** Will be implemented after device_fingerprinting module
**Dependency:** Phase 3 (RPA Deduplication)

**Summary:**
```rust
// In web_server.rs

pub async fn get_telemetry() -> impl Responder {
    // ~40 lines: Access telemetry singleton, aggregate, return JSON
}

pub async fn get_data_flow() -> impl Responder {
    // ~40 lines: Access data flow estimator, return analysis
}

pub async fn get_packet_sequence(path: web::Path<String>) -> impl Responder {
    // ~50 lines: Query packet tracker by MAC, build timeline
}

// Update configure_services():
pub fn configure_services(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/telemetry", web::get().to(get_telemetry))
            .route("/data-flow", web::get().to(get_data_flow))
            .route("/packet-sequence/{mac}", web::get().to(get_packet_sequence))
            // ... existing routes
    )
}
```

**Checklist:**
- [ ] Create `get_telemetry()` function (~40 lines)
- [ ] Create `get_data_flow()` function (~40 lines)
- [ ] Create `get_packet_sequence()` function (~50 lines)
- [ ] Update `configure_services()` with new routes
- [ ] Test all endpoints with curl/Postman
- [ ] Add error handling for missing data
- [ ] Document API in README

**PR Title:** "Feature: Add missing Web API endpoints (telemetry, data-flow, packet-sequence)"

---

## 🎨 PHASE 3: FRONTEND UI TABS (Days 8-9)

### 3.1 New Tabs Implementation
**Status:** 🟡 TODO
**Effort:** ~10-12 hours
**Files:** `frontend/index.html`, `frontend/app.js`, `frontend/styles.css`

#### 3.1.1 HTML Structure (index.html)
**Current:** Only "Devices" and "Logs" tabs
**Add 4 new tabs:**

```html
<!-- Tab Navigation -->
<div class="tabs-navigation">
    <button class="tab-btn active" data-tab="devices">📱 Devices</button>
    <button class="tab-btn" data-tab="telemetry">📊 Telemetry</button>
    <button class="tab-btn" data-tab="data-flow">📈 Data Flow</button>
    <button class="tab-btn" data-tab="packet-sequence">📋 Packets</button>
    <button class="tab-btn" data-tab="logs">📜 Logs</button>
</div>

<!-- Telemetry Tab -->
<div class="tab-content" id="telemetry">
    <div class="telemetry-controls">
        <select id="telemetry-device-select">
            <option>Select device...</option>
        </select>
        <button id="export-telemetry-btn">Export JSON</button>
    </div>
    <div class="telemetry-metrics">
        <div class="metric-card">
            <span class="metric-label">Packets</span>
            <span class="metric-value" id="telemetry-packets">0</span>
        </div>
        <div class="metric-card">
            <span class="metric-label">Avg RSSI</span>
            <span class="metric-value" id="telemetry-rssi">0 dBm</span>
        </div>
        <div class="metric-card">
            <span class="metric-label">Min/Max Latency</span>
            <span class="metric-value" id="telemetry-latency">0/0 ms</span>
        </div>
    </div>
    <div class="anomalies-list" id="anomalies-container"></div>
</div>

<!-- Data Flow Tab -->
<div class="tab-content" id="data-flow">
    <div class="data-flow-grid" id="data-flow-devices">
        <!-- Populated by JS: protocol badges, throughput, reliability -->
    </div>
    <div class="top-devices">
        <h3>Top Devices by Throughput</h3>
        <table id="throughput-table">
            <thead>
                <tr>
                    <th>Device</th>
                    <th>Protocol</th>
                    <th>Throughput</th>
                    <th>Reliability</th>
                </tr>
            </thead>
            <tbody id="throughput-list"></tbody>
        </table>
    </div>
</div>

<!-- Packet Sequence Tab -->
<div class="tab-content" id="packet-sequence">
    <div class="packet-controls">
        <select id="packet-device-select">
            <option>Select device...</option>
        </select>
        <label>
            <input type="checkbox" id="auto-refresh-packets"> Auto-refresh
        </label>
    </div>
    <div class="packet-timeline">
        <svg id="rssi-timeline" width="800" height="300"></svg>
    </div>
    <table class="packet-table" id="packet-list">
        <thead>
            <tr>
                <th>ID</th>
                <th>Time</th>
                <th>RSSI</th>
                <th>Delta</th>
                <th>Channel</th>
            </tr>
        </thead>
        <tbody id="packet-rows"></tbody>
    </table>
</div>

<!-- RPA History Tab (optional, for future) -->
<div class="tab-content" id="rpa-history">
    <p class="notice">RPA deduplication feature coming soon</p>
</div>
```

**Lines:** ~200-250 lines

#### 3.1.2 JavaScript Logic (app.js)
**Add functions:**

```javascript
// Tab switching
function setupTabNavigation() {
    document.querySelectorAll('.tab-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            const tab = btn.dataset.tab;
            showTab(tab);
            loadTabContent(tab);
        });
    });
}

// Telemetry Tab
async function loadTelemetry() {
    const response = await fetch('/api/telemetry');
    const data = await response.json();
    
    // Populate device selector
    const select = document.getElementById('telemetry-device-select');
    data.devices.forEach(device => {
        const option = document.createElement('option');
        option.value = device.mac;
        option.textContent = device.mac;
        select.appendChild(option);
    });
    
    // Display metrics
    select.addEventListener('change', (e) => {
        const selected = data.devices.find(d => d.mac === e.target.value);
        if (selected) displayTelemetryMetrics(selected);
    });
}

// Data Flow Tab
async function loadDataFlow() {
    const response = await fetch('/api/data-flow');
    const data = await response.json();
    
    const container = document.getElementById('data-flow-devices');
    data.devices.forEach(device => {
        const card = createDataFlowCard(device);
        container.appendChild(card);
    });
    
    // Populate throughput table
    const sorted = data.devices.sort((a, b) => 
        b.estimated_throughput_bytes_sec - a.estimated_throughput_bytes_sec
    ).slice(0, 5);
    
    const tbody = document.getElementById('throughput-list');
    sorted.forEach(device => {
        const row = document.createElement('tr');
        row.innerHTML = `
            <td>${device.mac}</td>
            <td><span class="protocol-badge">${device.detected_protocol}</span></td>
            <td>${(device.estimated_throughput_bytes_sec / 1024).toFixed(2)} KB/s</td>
            <td>${(device.reliability * 100).toFixed(0)}%</td>
        `;
        tbody.appendChild(row);
    });
}

// Packet Sequence Tab
async function loadPacketSequence(mac) {
    const response = await fetch(`/api/packet-sequence/${mac}`);
    const data = await response.json();
    
    // Draw RSSI timeline
    drawRssiTimeline(data.packets);
    
    // Populate packet table
    const tbody = document.getElementById('packet-rows');
    data.packets.forEach(packet => {
        const row = document.createElement('tr');
        row.innerHTML = `
            <td>${packet.packet_id}</td>
            <td>${new Date(packet.timestamp_ms).toLocaleTimeString()}</td>
            <td>${packet.rssi} dBm</td>
            <td>${packet.delta_ms} ms</td>
            <td>${packet.channel}</td>
        `;
        tbody.appendChild(row);
    });
}

// Simple RSSI timeline with SVG
function drawRssiTimeline(packets) {
    const svg = document.getElementById('rssi-timeline');
    // Simple line chart implementation
    // Or use Chart.js for more features
}
```

**Lines:** ~200-300 lines

#### 3.1.3 CSS Styling (styles.css)
**Add new classes:**

```css
/* Tab Navigation */
.tabs-navigation {
    display: flex;
    gap: 10px;
    border-bottom: 2px solid #e0e0e0;
    margin-bottom: 20px;
}

.tab-btn {
    padding: 10px 20px;
    background: none;
    border: none;
    border-bottom: 3px solid transparent;
    cursor: pointer;
    font-weight: 500;
    transition: all 0.3s;
}

.tab-btn.active {
    color: #0066cc;
    border-bottom-color: #0066cc;
}

.tab-btn:hover {
    background: #f5f5f5;
}

/* Tab Content */
.tab-content {
    display: none;
    animation: fadeIn 0.3s;
}

.tab-content.active {
    display: block;
}

@keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
}

/* Telemetry Tab */
.telemetry-metrics {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 15px;
    margin: 20px 0;
}

.metric-card {
    background: #f9f9f9;
    padding: 15px;
    border-radius: 8px;
    border-left: 4px solid #0066cc;
}

.metric-label {
    display: block;
    font-size: 12px;
    color: #666;
    margin-bottom: 5px;
}

.metric-value {
    display: block;
    font-size: 24px;
    font-weight: bold;
    color: #0066cc;
}

.anomalies-list {
    margin-top: 20px;
}

.anomaly-badge {
    display: inline-block;
    background: #ff4444;
    color: white;
    padding: 5px 10px;
    border-radius: 4px;
    margin: 5px;
    font-size: 12px;
}

/* Data Flow Tab */
.data-flow-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 20px;
    margin: 20px 0;
}

.data-flow-card {
    background: white;
    border: 1px solid #ddd;
    border-radius: 8px;
    padding: 15px;
}

.protocol-badge {
    display: inline-block;
    padding: 4px 12px;
    background: #e3f2fd;
    color: #0066cc;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 500;
    margin: 5px 5px 5px 0;
}

.reliability-gauge {
    width: 100%;
    height: 10px;
    background: #f0f0f0;
    border-radius: 5px;
    overflow: hidden;
    margin: 10px 0;
}

.reliability-gauge-fill {
    height: 100%;
    background: linear-gradient(90deg, #ff4444, #ffaa00, #44ff44);
}

/* Packet Sequence Tab */
.packet-timeline {
    background: white;
    border: 1px solid #ddd;
    border-radius: 8px;
    padding: 15px;
    margin: 15px 0;
    overflow-x: auto;
}

.packet-table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 20px;
}

.packet-table th {
    background: #f5f5f5;
    padding: 10px;
    text-align: left;
    border-bottom: 2px solid #ddd;
    font-weight: 600;
}

.packet-table td {
    padding: 10px;
    border-bottom: 1px solid #eee;
}

.packet-table tr:hover {
    background: #f9f9f9;
}
```

**Lines:** ~150-200 lines

**Checklist:**
- [ ] Add HTML for 4 new tabs (~200-250 lines)
- [ ] Implement tab switching logic
- [ ] Add telemetry endpoint fetch & display
- [ ] Add data-flow endpoint fetch & display
- [ ] Add packet-sequence endpoint fetch & display
- [ ] Create RSSI timeline visualization (SVG or Chart.js)
- [ ] Add CSS for all new elements (~150-200 lines)
- [ ] Test all tabs in browser
- [ ] Verify responsive design on mobile

**PR Title:** "Feature: Add Telemetry, Data Flow, and Packet Sequence UI tabs"

---

## 🎯 PHASE 4: RPA DEDUPLICATION (Week 2)

### 4.1 Create device_fingerprinting.rs
**Status:** 🔴 TODO
**Effort:** ~6-8 hours
**New File:** `src/device_fingerprinting.rs` (~250 lines)

```rust
// device_fingerprinting.rs

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceFingerprint {
    pub manufacturer_id: Option<u16>,
    pub service_uuids: Vec<String>,
    pub tx_power: Option<i8>,
    pub device_name_hash: String,
    pub appearance: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct DeviceGroup {
    pub group_id: String,
    pub fingerprint: DeviceFingerprint,
    pub mac_addresses: Vec<(String, i64)>, // (MAC, timestamp)
    pub first_seen: i64,
    pub last_seen: i64,
}

#[derive(Debug, Clone)]
pub struct RpaRotationEvent {
    pub group_id: String,
    pub old_mac: String,
    pub new_mac: String,
    pub timestamp: i64,
    pub confidence: f64,
}

pub struct FingerprintMatcher {
    groups: HashMap<String, DeviceGroup>,
    rotation_history: Vec<RpaRotationEvent>,
}

impl FingerprintMatcher {
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
            rotation_history: Vec::new(),
        }
    }

    /// Extract fingerprint from device
    pub fn extract_fingerprint(
        manufacturer_id: Option<u16>,
        service_uuids: Vec<String>,
        tx_power: Option<i8>,
        device_name: Option<&str>,
        appearance: Option<u16>,
    ) -> DeviceFingerprint {
        let device_name_hash = device_name
            .map(|name| hash_string(name))
            .unwrap_or_else(|| "unknown".to_string());

        DeviceFingerprint {
            manufacturer_id,
            service_uuids,
            tx_power,
            device_name_hash,
            appearance,
        }
    }

    /// Find matching group by fuzzy fingerprint matching
    pub fn find_matching_group(&self, fingerprint: &DeviceFingerprint) -> Option<String> {
        for (group_id, group) in &self.groups {
            if self.fingerprints_match(fingerprint, &group.fingerprint) {
                return Some(group_id.clone());
            }
        }
        None
    }

    /// Fuzzy matching algorithm
    fn fingerprints_match(&self, fp1: &DeviceFingerprint, fp2: &DeviceFingerprint) -> bool {
        // Match score: 0-1 (1.0 = identical)
        let mut matches = 0;
        let mut total_checks = 0;

        // Manufacturer ID match (strong signal)
        if fp1.manufacturer_id.is_some() && fp2.manufacturer_id.is_some() {
            total_checks += 1;
            if fp1.manufacturer_id == fp2.manufacturer_id {
                matches += 1;
            }
        }

        // Service UUIDs overlap (strong signal)
        if !fp1.service_uuids.is_empty() && !fp2.service_uuids.is_empty() {
            total_checks += 1;
            let overlap = fp1.service_uuids.iter()
                .filter(|uuid| fp2.service_uuids.contains(uuid))
                .count();
            if overlap > 0 {
                matches += 1;
            }
        }

        // TX Power (weak signal, can vary)
        if fp1.tx_power.is_some() && fp2.tx_power.is_some() {
            total_checks += 1;
            if (fp1.tx_power.unwrap() - fp2.tx_power.unwrap()).abs() <= 2 {
                matches += 1;
            }
        }

        // Device name hash (strong signal)
        if fp1.device_name_hash != "unknown" && fp2.device_name_hash != "unknown" {
            total_checks += 1;
            if fp1.device_name_hash == fp2.device_name_hash {
                matches += 1;
            }
        }

        // Appearance (weak signal)
        if fp1.appearance.is_some() && fp2.appearance.is_some() {
            total_checks += 1;
            if fp1.appearance == fp2.appearance {
                matches += 1;
            }
        }

        if total_checks == 0 {
            return false;
        }

        (matches as f64 / total_checks as f64) >= 0.6 // 60% match threshold
    }

    /// Register new device or add MAC to existing group
    pub fn register_device(
        &mut self,
        mac_address: &str,
        fingerprint: DeviceFingerprint,
    ) -> (String, bool) {
        // Check if MAC already registered
        for (group_id, group) in &self.groups {
            if group.mac_addresses.iter().any(|(m, _)| m == mac_address) {
                return (group_id.clone(), false); // Already registered
            }
        }

        // Check for matching fingerprint
        if let Some(group_id) = self.find_matching_group(&fingerprint) {
            // Add to existing group
            let now = chrono::Utc::now().timestamp();
            self.groups.get_mut(&group_id).unwrap().mac_addresses.push((mac_address.to_string(), now));
            self.groups.get_mut(&group_id).unwrap().last_seen = now;

            // Log rotation event if MAC is new
            let is_new_rpa = true;
            if is_new_rpa {
                self.rotation_history.push(RpaRotationEvent {
                    group_id: group_id.clone(),
                    old_mac: mac_address.to_string(), // Simplified
                    new_mac: mac_address.to_string(),
                    timestamp: now,
                    confidence: 0.85,
                });
            }

            (group_id, true)
        } else {
            // Create new group
            let group_id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().timestamp();
            
            self.groups.insert(group_id.clone(), DeviceGroup {