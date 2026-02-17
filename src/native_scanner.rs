/// Native Bluetooth Scanner Integration
///
/// Adapts platform-specific Bluetooth APIs to unified interface
/// Uses:
/// - Windows: winbluetooth + Windows Bluetooth API
/// - Linux: BlueZ + hci
/// - macOS: CoreBluetooth (via btleplug)

/// Multi-platform Bluetooth scanner with native API support.
///
/// Provides platform-specific scanning capabilities while maintaining
/// a unified interface. On Windows, combines WMI/Registry enumeration
/// with btleplug for BLE. On Linux, uses BlueZ. On macOS, uses CoreBluetooth.
///
/// # Fields
/// - `config`: Scan configuration
/// - `windows_manager`: Windows-specific Bluetooth manager (Windows only)
pub struct NativeBluetoothScanner {
    config: ScanConfig,
    #[cfg(target_os = "windows")]
    windows_manager: WindowsBluetoothManager,
}

impl NativeBluetoothScanner {
    /// Creates a new native Bluetooth scanner with the given configuration.
    ///
    /// Initializes platform-specific Bluetooth managers and displays
    /// capabilities for the current platform.
    ///
    /// # Arguments
    /// * `config` - Scan configuration parameters
    ///
    /// # Returns
    /// A new NativeBluetoothScanner instance
    pub fn new(config: ScanConfig) -> Self {
        info!("🚀 Initializing Native Bluetooth Scanner");

        #[cfg(target_os = "windows")]
        {
            let caps = WindowsBluetoothCapabilities::new();
            info!("{}", caps.summary());
        }

        Self {
            config,
            #[cfg(target_os = "windows")]
            windows_manager: WindowsBluetoothManager::new(),
        }
    }

    /// Runs a native platform scan using the best available method.
    ///
    /// Delegates to platform-specific scanning:
    /// - Windows: WMI enumeration + btleplug BLE scan
    /// - Linux: BlueZ via btleplug
    /// - macOS: CoreBluetooth via btleplug
    ///
    /// # Returns
    /// * `Ok(Vec<BluetoothDevice>)` - List of discovered devices
    /// * `Err(Box<dyn std::error::Error>)` - Error during scanning
    pub async fn run_native_scan(
        &mut self,
    ) -> Result<Vec<BluetoothDevice>, Box<dyn std::error::Error>> {
        #[cfg(target_os = "windows")]
        {
            return self.scan_windows().await;
        }

        #[cfg(target_os = "linux")]
        {
            return self.scan_linux().await;
        }

        #[cfg(target_os = "macos")]
        {
            return self.scan_macos().await;
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            Err("Native Bluetooth scanning not supported on this platform".into())
        }
    }

    #[cfg(target_os = "windows")]
    async fn scan_windows(&mut self) -> Result<Vec<BluetoothDevice>, Box<dyn std::error::Error>> {
        info!("🪟 Starting Windows scan: paired + dynamic BLE");

        let mut all_devices = std::collections::HashMap::new();

        // 1. Get paired devices via WMI/Registry
        match self.windows_manager.enumerate_devices().await {
            Ok(paired) => {
                info!("✅ Found {} paired devices via Windows API", paired.len());
                for d in paired {
                    all_devices.insert(d.mac_address.clone(), d);
                }
            }
            Err(e) => debug!("Windows API paired enumeration failed: {}", e),
        }

        // 2. Scan for BLE devices via btleplug
        match self.scan_via_btleplug().await {
            Ok(ble_devices) => {
                info!("✅ Found {} BLE devices via btleplug", ble_devices.len());
                for d in ble_devices {
                    all_devices.insert(d.mac_address.clone(), d);
                }
            }
            Err(e) => debug!("btleplug BLE scan failed: {}", e),
        }

        if all_devices.is_empty() {
            info!("ℹ️ No devices found from any scan method on Windows");
        }

        Ok(all_devices.into_values().collect())
    }

    #[cfg(target_os = "linux")]
    async fn scan_linux(&mut self) -> Result<Vec<BluetoothDevice>, Box<dyn std::error::Error>> {
        info!("🐧 Starting Linux native Bluetooth scan (BlueZ)");

        // Linux uses BlueZ via btleplug
        self.scan_via_btleplug().await
    }

    #[cfg(target_os = "macos")]
    async fn scan_macos(&mut self) -> Result<Vec<BluetoothDevice>, Box<dyn std::error::Error>> {
        info!("🍎 Starting macOS native Bluetooth scan");

        // macOS uses CoreBluetooth via btleplug
        self.scan_via_btleplug().await
    }

    /// Fallback method using btleplug for cross-platform scanning.
    ///
    /// Used when native platform APIs are unavailable or fail.
    /// Provides standard BLE scanning via btleplug library.
    ///
    /// # Returns
    /// * `Ok(Vec<BluetoothDevice>)` - List of discovered BLE devices
    /// * `Err(Box<dyn std::error::Error>)` - Error during scanning
    async fn scan_via_btleplug(&self) -> Result<Vec<BluetoothDevice>, Box<dyn std::error::Error>> {
        debug!("Using btleplug for cross-platform scanning");

        let scanner = BluetoothScanner::new(self.config.clone());
        scanner.run_scan().await
    }

    /// Gets native Bluetooth capabilities for the current platform.
    ///
    /// Returns information about what features and APIs are supported
    /// on the current platform including BLE, BR/EDR, HCI, GATT, and pairing.
    ///
    /// # Returns
    /// PlatformCapabilities with supported features
    pub fn get_capabilities(&self) -> PlatformCapabilities {
        #[cfg(target_os = "windows")]
        {
            let caps = WindowsBluetoothCapabilities::new();
            return PlatformCapabilities {
                platform: "Windows".to_string(),
                supports_native_api: true,
                supports_ble: caps.supports_ble,
                supports_bredr: caps.supports_bredr,
                supports_dual_mode: caps.supports_dual_mode,
                supports_hci_raw: caps.supports_hci_raw,
                supports_gatt: caps.supports_gatt,
                supports_pairing: caps.supports_pairing,
                api_name: "Windows Bluetooth API + winbluetooth".to_string(),
            };
        }

        #[cfg(target_os = "linux")]
        {
            PlatformCapabilities {
                platform: "Linux".to_string(),
                supports_native_api: true,
                supports_ble: true,
                supports_bredr: true,
                supports_dual_mode: true,
                supports_hci_raw: true,
                supports_gatt: true,
                supports_pairing: true,
                api_name: "BlueZ + hci-dev".to_string(),
            }
        }

        #[cfg(target_os = "macos")]
        {
            PlatformCapabilities {
                platform: "macOS".to_string(),
                supports_native_api: true,
                supports_ble: true,
                supports_bredr: false,
                supports_dual_mode: false,
                supports_hci_raw: false,
                supports_gatt: true,
                supports_pairing: true,
                api_name: "CoreBluetooth + IOBluetooth".to_string(),
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            PlatformCapabilities {
                platform: "Unknown".to_string(),
                supports_native_api: false,
                supports_ble: false,
                supports_bredr: false,
                supports_dual_mode: false,
                supports_hci_raw: false,
                supports_gatt: false,
                supports_pairing: false,
                api_name: "None".to_string(),
            }
        }
    }
}

/// Platform detection and capabilities.
///
/// Contains information about what Bluetooth features and APIs
/// are supported on the current platform.
///
/// # Fields
/// - `platform`: Operating system name (Windows, Linux, macOS, etc.)
/// - `supports_native_api`: Whether platform has native Bluetooth API
/// - `supports_ble`: Whether BLE (Low Energy) is supported
/// - `supports_bredr`: Whether Classic Bluetooth (BR/EDR) is supported
/// - `supports_dual_mode`: Whether dual-mode devices are supported
/// - `supports_hci_raw`: Whether raw HCI access is available
/// - `supports_gatt`: Whether GATT server/client is supported
/// - `supports_pairing`: Whether device pairing is supported
/// - `api_name`: Name of the Bluetooth API being used
#[derive(Debug, Clone)]
pub struct PlatformCapabilities {
    pub platform: String,
    pub supports_native_api: bool,
    pub supports_ble: bool,
    pub supports_bredr: bool,
    pub supports_dual_mode: bool,
    pub supports_hci_raw: bool,
    pub supports_gatt: bool,
    pub supports_pairing: bool,
    pub api_name: String,
}

impl std::fmt::Display for PlatformCapabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "📱 {} Bluetooth Capabilities:\n  \
             Native API: {}\n  API: {}\n  \
             BLE: {}, BR/EDR: {}, Dual: {}\n  \
             HCI Raw: {}, GATT: {}, Pairing: {}",
            self.platform,
            if self.supports_native_api {
                "✓"
            } else {
                "✗"
            },
            self.api_name,
            if self.supports_ble { "✓" } else { "✗" },
            if self.supports_bredr { "✓" } else { "✗" },
            if self.supports_dual_mode {
                "✓"
            } else {
                "✗"
            },
            if self.supports_hci_raw { "✓" } else { "✗" },
            if self.supports_gatt { "✓" } else { "✗" },
            if self.supports_pairing { "✓" } else { "✗" },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let scanner = NativeBluetoothScanner::new(ScanConfig::default());
        let caps = scanner.get_capabilities();

        assert!(!caps.platform.is_empty());
        println!("{}", caps);
    }
}
