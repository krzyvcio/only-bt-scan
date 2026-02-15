/// Unified Windows Bluetooth LE Integration
/// 
/// Combines HCI packet capture with device management and official Company ID references
/// Provides seamless integration between low-level packet analysis and high-level device operations
/// 
/// Architecture:
/// ```
/// Windows Unified BLE System
/// ├── HCI Scanner (Packet Capture)
/// │   ├── Raw advertising data
/// │   ├── RSSI monitoring
/// │   └── Channel tracking
/// ├── Device Manager (Device Operations)
/// │   ├── Device enumeration
/// │   ├── Pairing/connection
/// │   └── Service discovery
/// └── Manufacturer Recognition (Official SIG Data)
///     ├── Company ID lookup
///     ├── Manufacturer name resolution
///     └── Datasheet association
/// ```

#[cfg(target_os = "windows")]
pub mod unified {
    use crate::company_ids;
    use crate::windows_bluetooth::windows_bt::WindowsBluetoothManager;
    use crate::windows_hci::WindowsHciScanner;
    use log::{debug, info, warn};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use chrono::{DateTime, Utc};
    use colored::Colorize;

    /// Manufacturer device with full context (packets + metadata)
    #[derive(Debug, Clone)]
    pub struct ManagedDevice {
        pub mac_address: String,
        pub friendly_name: Option<String>,
        pub manufacturer_id: Option<u16>,
        pub manufacturer_name: String, // From official SIG
        pub rssi: i8,
        pub first_seen: DateTime<Utc>,
        pub last_seen: DateTime<Utc>,
        pub packet_count: u64,
        pub is_paired: bool,
        pub is_connected: bool,
        pub device_class: Option<String>,
        pub advertisement_types: Vec<String>,
    }

    impl ManagedDevice {
        pub fn new(mac_address: String) -> Self {
            Self {
                mac_address,
                friendly_name: None,
                manufacturer_id: None,
                manufacturer_name: "Unknown".to_string(),
                rssi: 0,
                first_seen: Utc::now(),
                last_seen: Utc::now(),
                packet_count: 0,
                is_paired: false,
                is_connected: false,
                device_class: None,
                advertisement_types: Vec::new(),
            }
        }

        /// Set manufacturer ID and look up official name from Bluetooth SIG
        pub fn set_manufacturer_id(&mut self, mfg_id: u16) {
            self.manufacturer_id = Some(mfg_id);
            self.manufacturer_name = company_ids::get_company_name(mfg_id);
        }

        /// Get manufacturer info as formatted string
        pub fn manufacturer_info(&self) -> String {
            if let Some(id) = self.manufacturer_id {
                format!("{}(0x{:04X}", self.manufacturer_name, id)
            } else {
                self.manufacturer_name.clone()
            }
        }
    }

    /// Unified Windows Bluetooth LE Scanner
    /// Combines HCI scanning with device management
    pub struct WindowsUnifiedBleScanner {
        hci_scanner: Option<WindowsHciScanner>,
        device_manager: WindowsBluetoothManager,
        devices: Arc<Mutex<HashMap<String, ManagedDevice>>>,
        packet_counter: Arc<Mutex<u64>>,
        is_scanning: bool,
    }

    impl WindowsUnifiedBleScanner {
        /// Create new unified scanner
        pub fn new() -> Self {
            Self {
                hci_scanner: None,
                device_manager: WindowsBluetoothManager::new(),
                devices: Arc::new(Mutex::new(HashMap::new())),
                packet_counter: Arc::new(Mutex::new(0)),
                is_scanning: false,
            }
        }

        /// Initialize HCI scanner
        pub fn with_hci_support(mut self) -> Self {
            self.hci_scanner = Some(WindowsHciScanner::new("default".to_string()));
            self
        }

        /// Start unified scanning (HCI + device enumeration)
        pub async fn start_scan(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            info!("🚀 Starting Windows Unified BLE Scanner");

            // Start HCI scanning if available
            if let Some(hci) = &mut self.hci_scanner {
                hci.start_scan().await?;
                info!("✅ HCI scanner started");
            } else {
                warn!("⚠️ HCI scanner not available, using device manager only");
            }

            self.is_scanning = true;
            info!("✅ Unified BLE Scanner active");

            Ok(())
        }

        /// Process next advertisement (HCI packet)
        pub async fn process_next_advertisement(
            &mut self,
        ) -> Result<Option<ManagedDevice>, Box<dyn std::error::Error>> {
            if let Some(hci) = &mut self.hci_scanner {
                if let Some(adv_report) = hci.receive_advertisement().await? {
                    // Increment packet counter
                    if let Ok(mut counter) = self.packet_counter.lock() {
                        *counter += 1;
                    }

                    // Lock devices map
                    let mut devices_lock = self.devices.lock().unwrap();

                    // Get or create device
                    let device = devices_lock
                        .entry(adv_report.address.clone())
                        .or_insert_with(|| ManagedDevice::new(adv_report.address.clone()));

                    // Update device info
                    device.rssi = adv_report.rssi;
                    device.last_seen = Utc::now();
                    device.packet_count += 1;

                    // Add advertisement type if not already present
                    let adv_type = match adv_report.event_type {
                        0x00 => "ADV_IND",
                        0x01 => "ADV_DIRECT_IND",
                        0x02 => "ADV_SCAN_IND",
                        0x03 => "ADV_NONCONN_IND",
                        0x04 => "SCAN_RSP",
                        _ => "UNKNOWN",
                    };

                    if !device.advertisement_types.contains(&adv_type.to_string()) {
                        device.advertisement_types.push(adv_type.to_string());
                    }

                    // Try to extract manufacturer data and set official name
                    if adv_report.advertising_data.len() >= 3 {
                        if adv_report.advertising_data[0] >= 3
                            && adv_report.advertising_data[1] == 0xFF
                        {
                            let mfg_id = u16::from_le_bytes([
                                adv_report.advertising_data[2],
                                adv_report.advertising_data[3],
                            ]);
                            device.set_manufacturer_id(mfg_id);
                        }
                    }

                    debug!(
                        "📱 Device: {} | Mfg: {} | RSSI: {} dBm | Packets: {}",
                        adv_report.address,
                        device.manufacturer_info(),
                        adv_report.rssi,
                        device.packet_count
                    );

                    let result = device.clone();
                    drop(devices_lock);

                    return Ok(Some(result));
                }
            }

            Ok(None)
        }

        /// Get all discovered devices
        pub fn get_devices(&self) -> Vec<ManagedDevice> {
            self.devices
                .lock()
                .unwrap()
                .values()
                .cloned()
                .collect()
        }

        /// Get device by MAC address
        pub fn get_device(&self, mac: &str) -> Option<ManagedDevice> {
            self.devices.lock().unwrap().get(mac).cloned()
        }

        /// Get device count
        pub fn device_count(&self) -> usize {
            self.devices.lock().unwrap().len()
        }

        /// Get total packets processed
        pub fn packet_count(&self) -> u64 {
            *self.packet_counter.lock().unwrap()
        }

        /// Get devices grouped by manufacturer
        pub fn devices_by_manufacturer(&self) -> HashMap<String, Vec<ManagedDevice>> {
            let devices = self.get_devices();
            let mut grouped: HashMap<String, Vec<ManagedDevice>> = HashMap::new();

            for device in devices {
                grouped
                    .entry(device.manufacturer_name.clone())
                    .or_insert_with(Vec::new)
                    .push(device);
            }

            grouped
        }

        /// Get manufacturer statistics
        pub fn manufacturer_stats(&self) -> Vec<(String, u16, usize)> {
            // (manufacturer_name, company_id, device_count)
            let grouped = self.devices_by_manufacturer();
            let mut stats: Vec<(String, u16, usize)> = grouped
                .into_iter()
                .map(|(name, devices)| {
                    let company_id = devices
                        .first()
                        .and_then(|d| d.manufacturer_id)
                        .unwrap_or(0);
                    (name, company_id, devices.len())
                })
                .collect();

            stats.sort_by(|a, b| b.2.cmp(&a.2)); // Sort by device count descending
            stats
        }

        /// Search devices by manufacturer name (case-insensitive)
        pub fn find_devices_by_manufacturer(&self, pattern: &str) -> Vec<ManagedDevice> {
            let pattern_lower = pattern.to_lowercase();
            self.get_devices()
                .into_iter()
                .filter(|d| d.manufacturer_name.to_lowercase().contains(&pattern_lower))
                .collect()
        }

        /// Search devices by company ID
        pub fn find_devices_by_company_id(&self, company_id: u16) -> Vec<ManagedDevice> {
            self.get_devices()
                .into_iter()
                .filter(|d| d.manufacturer_id == Some(company_id))
                .collect()
        }

        /// Display scan summary
        pub fn print_summary(&self) {
            let devices = self.get_devices();
            let stats = self.manufacturer_stats();

            println!("\n{}", "═══════════════════════════════════════════════════════".cyan());
            println!("📊 Windows Unified BLE Scan Summary");
            println!("{}", "═══════════════════════════════════════════════════════".cyan());

            println!(
                "📱 Total Devices: {} | Total Packets: {}",
                devices.len(),
                self.packet_count()
            );

            println!("\n{}", "Manufacturers Detected:".bold());
            println!("{:<40} | {:>6} | Count", "Manufacturer", "ID");
            println!("{}", "─".repeat(60));

            for (name, id, count) in stats.iter().take(20) {
                if *id == 0 {
                    println!("{:<40} | {:>6} | {}", name, "N/A", count);
                } else {
                    println!("{:<40} | 0x{:04X} | {}", name, id, count);
                }
            }

            if stats.len() > 20 {
                println!(
                    "{:<40} |        | ... and {} more",
                    "",
                    stats.len() - 20
                );
            }

            println!("{}", "═══════════════════════════════════════════════════════".cyan());
        }

        /// Stop scanning
        pub async fn stop_scan(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            if let Some(hci) = &mut self.hci_scanner {
                hci.stop_scan().await?;
                info!("✅ HCI scanner stopped");
            }

            self.is_scanning = false;
            self.print_summary();

            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_managed_device_creation() {
            let mut device = ManagedDevice::new("AA:BB:CC:DD:EE:FF".to_string());
            device.set_manufacturer_id(0x004C); // Apple

            assert_eq!(device.manufacturer_id, Some(0x004C));
            assert!(device
                .manufacturer_name
                .to_lowercase()
                .contains("apple"));
        }

        #[test]
        fn test_unified_scanner_creation() {
            let scanner = WindowsUnifiedBleScanner::new();
            assert_eq!(scanner.device_count(), 0);
            assert_eq!(scanner.packet_count(), 0);
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub mod unified {
    use log::warn;

    pub struct WindowsUnifiedBleScanner;

    impl WindowsUnifiedBleScanner {
        pub fn new() -> Self {
            warn!("Unified Windows BLE Scanner not available on non-Windows platforms");
            Self
        }
    }
}
