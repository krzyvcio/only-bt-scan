# 🟦 Bluey Integration

## Overview
Integration with **Bluey** - Advanced Bluetooth library with better cross-platform support than btleplug.

**Bluey Repository:** https://github.com/rib/bluey

### Features:
- ✅ GATT service discovery & connection
- ✅ Characteristic reading/writing
- ✅ Descriptor analysis
- ✅ Better Windows/Android support
- 🔄 Planned: macOS, iOS, Linux, Web Bluetooth

### Comparison: btleplug vs Bluey

| Feature | btleplug | Bluey |
|---------|----------|-------|
| **Scanning** | ✓ All platforms | ✓ Windows, Android |
| **Connection** | Limited | ✓ Full GATT |
| **Services** | Discovery only | ✓ Read/Write |
| **Characteristics** | Limited | ✓ Full support |
| **Descriptors** | No | ✓ Yes |
| **Windows** | Good | ✓ Better |
| **Android** | No | ✓ Yes |
| **macOS/iOS** | ✓ Yes | 🔄 Planned |
| **Linux** | Via bluer | 🔄 Planned |

---

## Architecture: Hybrid Scanner

```
┌─────────────────────────────────────────────────────────────┐
│ HybridScanner                                               │
│                                                             │
│  ┌──────────────────┐         ┌──────────────────┐         │
│  │ btleplug Scan    │ ─────→  │ All Devices      │         │
│  │ (Always)         │         │ (Primary)        │         │
│  └──────────────────┘         └────────┬─────────┘         │
│                                        │                    │
│  ┌──────────────────┐                  │                    │
│  │ Bluey Scan       │ ─────────────────┼──→ Merge &        │
│  │ (If supported)   │                  │    Enrich         │
│  └──────────────────┘         ┌────────▼─────────┐         │
│                               │ Final Devices    │         │
│                               │ - btleplug data  │         │
│                               │ + Bluey GATT     │         │
│                               └──────────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

---

## Implementation

### Module: `bluey_integration.rs`

```rust
// Configuration
pub struct BlueyConfig {
    pub enabled: bool,
    pub scan_duration: Duration,
    pub discover_gatt: bool,
    pub max_concurrent_connections: usize,
}

// Scanner
pub struct BlueyScanner {
    config: BlueyConfig,
}

impl BlueyScanner {
    pub async fn scan_with_bluey(&self) -> Result<Vec<DeviceModel>, ...>
    pub async fn discover_gatt_services(&self, mac: &str) -> Result<Vec<GattServiceInfo>, ...>
}

// Hybrid combining btleplug + Bluey
pub struct HybridScanner {
    btleplug_enabled: bool,
    bluey_enabled: bool,
}

impl HybridScanner {
    pub async fn hybrid_scan(&self, btleplug_result: Vec<DeviceModel>) 
        -> Result<Vec<DeviceModel>, ...>
}

// Platform capabilities detection
pub struct BlueyCapabilities {
    pub platform: String,
    pub supported: bool,
    pub gatt_discovery: bool,
    pub connection_support: bool,
    pub descriptor_read: bool,
}
```

---

## Usage

### 1. Enable Bluey Feature (Optional)
```bash
# Compile with Bluey support
cargo build --features bluey

# Or without (just btleplug)
cargo build
```

### 2. Use Hybrid Scanner

```rust
use only_bt_scan::bluey_integration::{BlueyConfig, HybridScanner};

// Create Bluey config
let bluey_config = BlueyConfig {
    enabled: true,
    scan_duration: Duration::from_secs(30),
    discover_gatt: true,
    max_concurrent_connections: 5,
};

// Create hybrid scanner
let hybrid = HybridScanner::new(bluey_config);

// Run btleplug scan first
let btleplug_devices = scanner.run_scan().await?;

// Enhance with Bluey
let all_devices = hybrid.hybrid_scan(btleplug_devices).await?;
```

### 3. Check Platform Support

```rust
use only_bt_scan::bluey_integration::BlueyCapabilities;

let caps = BlueyCapabilities::current();
println!("{}", caps.info());

// Output example:
// "Bluey on Windows: ✅ Full support (GATT: ✓, Connection: ✓, Descriptors: ✓)"
// "Bluey on Linux: ⏳ Coming soon"
```

---

## Platform Support Matrix

### Current ✅
| Platform | Scanning | GATT | Connection | Descriptors |
|----------|----------|------|-----------|-------------|
| Windows | ✓ btleplug + Bluey | ✓ | ✓ | ✓ |
| Android | ✓ Bluey | ✓ | ✓ | ✓ |

### Planned 🔄
| Platform | Status | ETA |
|----------|--------|-----|
| macOS | Planned | TBD |
| iOS | Planned | TBD |
| Linux | Planned | TBD |
| Web | Planned | TBD |

---

## Integration Points

### 1. Scanner Coordinator
Update `concurrent_scan_all_methods()` to include Bluey:

```rust
pub async fn concurrent_scan_all_methods_v2(&self) -> Result<Vec<BluetoothDevice>> {
    let (btleplug, bluey, bredr, hci) = tokio::join!(
        self.scan_ble(),
        self.scan_with_bluey(),
        self.scan_bredr(),
        self.scan_hci_direct()
    );
    // Merge results...
}
```

### 2. GATT Discovery
Bluey provides native GATT discovery without needing separate HCI commands:

```rust
// Before: Manual GATT parsing
let services = gatt_client.discover_services().await?;

// After: Bluey handles it
let bluey = BlueyScanner::new(config);
let services = bluey.discover_gatt_services("AA:BB:CC:DD:EE:FF").await?;
```

### 3. Device Enrichment
When device found by btleplug, Bluey can discover GATT services:

```rust
for device in devices {
    // btleplug gives us basic info
    let mut dev = device;
    
    // Bluey gives us GATT details
    if let Ok(services) = bluey.discover_gatt_services(&dev.mac_address).await {
        dev.discovered_services = services;
    }
}
```

---

## Data Flow

```
┌─────────────────────┐
│ Start Scan          │
└──────────┬──────────┘
           │
           ├─────────────────────────────┐
           │                             │
    ┌──────▼─────┐             ┌────────▼──────┐
    │ btleplug    │             │ Bluey         │
    │ Scan BLE    │             │ Scan BLE      │
    │ (Always)    │             │ (If support)  │
    └──────┬─────┘             └────────┬──────┘
           │                            │
    ┌──────▼─────────────────────┬──────▼──────┐
    │ btleplug Results            │ Bluey       │
    │ - Devices found             │ Results     │
    │ - Basic info                │ - Devices   │
    │ - No services               │ - Services  │
    │                             │ - Chars     │
    └──────┬──────────────────────┴──────┬──────┘
           │                            │
           └──────────┬─────────────────┘
                      │
           ┌──────────▼────────────┐
           │ HybridScanner Merge   │
           │ - Deduplicate by MAC  │
           │ - Enrich with Bluey   │
           │ - Add GATT services   │
           └──────────┬────────────┘
                      │
           ┌──────────▼────────────┐
           │ Final Device List     │
           │ - All devices found   │
           │ - GATT info included  │
           └───────────────────────┘
```

---

## Configuration Examples

### Minimal (btleplug only)
```rust
// No Bluey, just standard scanning
let devices = scanner.run_scan().await?;
```

### With Bluey Support
```rust
let bluey_config = BlueyConfig::default(); // 30s scan, GATT enabled
let hybrid = HybridScanner::new(bluey_config);
let devices = hybrid.hybrid_scan(btleplug_result).await?;
```

### Custom Configuration
```rust
let bluey_config = BlueyConfig {
    enabled: true,
    scan_duration: Duration::from_secs(60),
    discover_gatt: true,
    max_concurrent_connections: 10,
};
let hybrid = HybridScanner::new(bluey_config);
```

---

## Future Enhancements

1. **Full Bluey Integration**
   - Connect to devices
   - Read/write characteristics
   - Subscribe to notifications

2. **Platform Expansion**
   - macOS/iOS support when available
   - Linux support when available

3. **Caching**
   - Cache GATT discovery results
   - Expire after configurable time

4. **Connection Pool**
   - Maintain connections to multiple devices
   - Subscribe to multiple notifications

5. **Web API Endpoints**
   - `/api/devices/{mac}/services` - GATT services
   - `/api/devices/{mac}/characteristics` - Characteristics
   - `/api/devices/{mac}/connect` - Connect and interact

---

## Testing

```rust
#[test]
fn test_bluey_capabilities() {
    let caps = BlueyCapabilities::current();
    println!("{}", caps.info());
}

#[test]
fn test_hybrid_scanner_creation() {
    let config = BlueyConfig::default();
    let _scanner = HybridScanner::new(config);
}

// Run with Bluey feature enabled
// cargo test --features bluey
```

---

## Troubleshooting

### Bluey Not Compiling
- Ensure you're on Windows or Android for full support
- Linux/macOS will skip Bluey in `#[cfg(feature = "bluey")]` blocks

### GATT Discovery Timeout
- Increase `scan_duration` in BlueyConfig
- Check device is in advertising range

### Services Not Found
- Device may not advertise services in BLE advertising data
- Need to connect and discover via GATT
- Bluey handles this automatically

---

## References

- **Bluey Repository:** https://github.com/rib/bluey
- **Bluey Documentation:** https://github.com/rib/bluey#readme
- **BLE Specification:** https://www.bluetooth.com/specifications/specs/
- **GATT:** https://www.bluetooth.com/specifications/specs/generic-attribute-profile-specification/
