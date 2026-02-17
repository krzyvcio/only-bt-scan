/// Unified Scan Engine - Integrates all scanning methods and event handling
///
/// Coordinates:
/// - Native platform scanners (Windows, Linux, macOS)
/// - Packet ordering and deduplication
/// - Device event listening
/// - Raw HCI scanning
/// - Telemetry collection
use crate::bluetooth_scanner::{BluetoothDevice, ScanConfig};
use crate::data_models::RawPacketModel;
use crate::device_events::{BluetoothDeviceEvent, DeviceEventListener};
use crate::native_scanner::NativeBluetoothScanner;
use crate::scanner_integration::ScannerWithTracking;
use crate::mac_address_handler::{MacAddress, MacAddressFilter};
use log::{debug, info};
use std::sync::Arc;
/// Main scanning engine combining all subsystems.
///
/// Coordinates native platform scanners, packet tracking, event handling,
/// and optional HCI scanning for comprehensive device discovery.
///
/// # Fields
/// - `config`: Scan configuration
/// - `native_scanner`: Platform-specific Bluetooth scanner
/// - `tracker_system`: Scanner with packet tracking
/// - `event_listener`: Device event listener
/// - `hci_scanner`: Windows HCI scanner (Windows only)
pub struct UnifiedScanEngine {
    config: ScanConfig,
    native_scanner: NativeBluetoothScanner,
    tracker_system: ScannerWithTracking,
    event_listener: Arc<DeviceEventListener>,
    #[cfg(target_os = "windows")]
    hci_scanner: Option<crate::windows_hci::WindowsHciScanner>,
    mac_filter: MacAddressFilter,
}

impl UnifiedScanEngine {
    /// Creates a new UnifiedScanEngine with the given configuration.
    ///
    /// Initializes all subsystems including native scanner, packet tracker,
    /// event listener, and platform-specific HCI scanner.
    ///
    /// # Arguments
    /// * `config` - Scan configuration
    ///
    /// # Returns
    /// A new UnifiedScanEngine instance
    pub fn new(config: ScanConfig) -> Self {
        info!("🚀 Initializing Unified Scan Engine");

        let native_scanner = NativeBluetoothScanner::new(config.clone());
        let tracker_system = ScannerWithTracking::new();
        let event_listener = Arc::new(DeviceEventListener::new());

        // Display platform capabilities
        let caps = native_scanner.get_capabilities();
        info!("{}", caps);

        #[cfg(target_os = "windows")]
        {
            info!("🪟 Windows platform detected - enabling HCI support");
        }

        // Initialize MAC filter from environment
        let whitelist = std::env::var("MAC_WHITELIST").ok();
        let blacklist = std::env::var("MAC_BLACKLIST").ok();
        let mac_filter = MacAddressFilter::from_config(whitelist.as_deref(), blacklist.as_deref());

        Self {
            config,
            native_scanner,
            tracker_system,
            event_listener,
            #[cfg(target_os = "windows")]
            hci_scanner: None,
            mac_filter,
        }
    }

    /// Runs an integrated scan operation through all phases.
    ///
    /// Executes scan in multiple phases:
    /// 1. Native platform scanning
    /// 2. Packet ordering and deduplication
    /// 3. Device event emission
    /// 4. Raw HCI scan (Windows only)
    ///
    /// # Returns
    /// * `Ok(ScanEngineResults)` - Scan results with devices, stats, and telemetry
    /// * `Err(Box<dyn std::error::Error>)` - Error during scanning
    pub async fn run_scan(&mut self) -> Result<ScanEngineResults, Box<dyn std::error::Error>> {
        info!("🔄 Starting unified scan cycle");

        let start_time = std::time::Instant::now();

        // Phase 1: Run native platform scanner
        info!("📡 Phase 1: Native platform scanning");
        let mut native_devices = self.native_scanner.run_native_scan().await?;
        
        // Filter devices
        native_devices.retain(|d| {
            if let Ok(mac) = MacAddress::from_string(&d.mac_address) {
                self.mac_filter.matches(&mac)
            } else {
                false
            }
        });

        info!(
            "✅ Phase 1 complete: {} devices found (after filtering)",
            native_devices.len()
        );

        // Phase 2: Process devices through packet tracker
        info!("📊 Phase 2: Packet ordering and deduplication");
        self.tracker_system
            .process_scan_results(native_devices.clone());

        // Phase 3: Emit events for newly discovered devices
        info!("🎧 Phase 3: Device event emission");
        for device in &native_devices {
            self.event_listener
                .emit(BluetoothDeviceEvent::DeviceDiscovered {
                    mac_address: device.mac_address.clone(),
                    name: device.name.clone(),
                    rssi: device.rssi,
                    is_ble: true,
                    is_bredr: matches!(
                        device.device_type,
                        crate::bluetooth_scanner::DeviceType::BrEdr
                            | crate::bluetooth_scanner::DeviceType::DualMode
                    ),
                });
        }

        // Phase 4: Raw HCI scan on Windows (optional, runs in parallel)
        #[cfg(target_os = "windows")]
        {
            info!("📡 Phase 4: Windows Raw HCI scan (optional)");
            if let Ok(hci_devices) = self.scan_windows_hci().await {
                info!(
                    "✅ Phase 4 complete: {} devices from HCI",
                    hci_devices.len()
                );
            }
        }

        let duration = start_time.elapsed();

        // Collect results
        let stats = self.tracker_system.get_stats();
        let telemetry = self.tracker_system.export_telemetry();
        let raw_packets = self.tracker_system.get_last_scan_packets().to_vec();

        Ok(ScanEngineResults {
            devices: native_devices,
            scanner_stats: stats,
            packet_sequence: self.tracker_system.get_packet_ordering(),
            raw_packets,
            telemetry_json: telemetry,
            duration_ms: duration.as_millis() as u64,
            event_count: self.tracker_system.packet_tracker.packet_count,
        })
    }

    /// Runs HCI-only scan on Windows for raw packet capture.
    ///
    /// Uses Windows HCI scanner to capture raw advertising packets
    /// at a low level, providing access to packets that may not
    /// be visible through standard scanning APIs.
    ///
    /// # Returns
    /// * `Ok(Vec<BluetoothDevice>)` - Devices found via HCI
    /// * `Err(Box<dyn std::error::Error>)` - Error during HCI scan
    #[cfg(target_os = "windows")]
    async fn scan_windows_hci(
        &mut self,
    ) -> Result<Vec<BluetoothDevice>, Box<dyn std::error::Error>> {
        use crate::windows_hci::WindowsHciScanner;

        info!("🪟 Initializing Windows HCI scanner (Passive: {})", self.config.passive);

        let mut hci_scanner = WindowsHciScanner::new("BT0".to_string());
        hci_scanner.start_scan(self.config.passive).await?;

        let devices = Vec::new();

        // Collect advertisements for 100ms
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < 100 {
            if let Ok(Some(_report)) = hci_scanner.receive_advertisement().await {
                // Process HCI advertising report
                // For now, just log that we received something
                debug!("📡 Received HCI advertisement");
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        hci_scanner.stop_scan().await?;

        Ok(devices)
    }

    #[cfg(not(target_os = "windows"))]
    async fn scan_windows_hci(
        &mut self,
    ) -> Result<Vec<BluetoothDevice>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }

    /// Gets the device event listener for subscribing to events.
    ///
    /// # Returns
    /// Arc-wrapped DeviceEventListener
    pub fn get_event_listener(&self) -> Arc<DeviceEventListener> {
        self.event_listener.clone()
    }

    /// Exports full telemetry as JSON.
    ///
    /// # Returns
    /// JSON string with global telemetry data
    pub fn export_telemetry(&self) -> String {
        self.tracker_system.export_telemetry()
    }

    /// Exports device-specific telemetry as JSON.
    ///
    /// # Arguments
    /// * `mac` - MAC address of the device
    ///
    /// # Returns
    /// JSON string with device telemetry, or None if not found
    pub fn export_device_telemetry(&self, mac: &str) -> Option<String> {
        self.tracker_system.export_device_telemetry(mac)
    }

    /// Gets the packet sequence for a specific device.
    ///
    /// # Arguments
    /// * `mac` - MAC address of the device
    ///
    /// # Returns
    /// Vector of packet IDs, or None if device not tracked
    pub fn get_device_packet_sequence(&self, mac: &str) -> Option<Vec<u64>> {
        self.tracker_system.get_device_sequence(mac)
    }

    /// Gets global packet ordering across all devices.
    ///
    /// # Returns
    /// Vector of tuples: (mac_address, packet_id, timestamp_ms)
    pub fn get_global_packet_order(&self) -> Vec<(String, u64, u64)> {
        self.tracker_system.get_packet_ordering()
    }
}

/// Results from a single scan operation.
///
/// Contains all data collected during a scan cycle including
/// discovered devices, statistics, telemetry, and timing information.
///
/// # Fields
/// - `devices`: Vector of discovered BluetoothDevice
/// - `scanner_stats`: Scanner tracking statistics
/// - `packet_sequence`: Global packet ordering tuples
/// - `raw_packets`: Raw packets for database persistence
/// - `telemetry_json`: Telemetry data as JSON string
/// - `duration_ms`: Scan duration in milliseconds
/// - `event_count`: Number of events emitted
#[derive(Debug, Clone)]
pub struct ScanEngineResults {
    pub devices: Vec<BluetoothDevice>,
    pub scanner_stats: crate::scanner_integration::ScannerTrackingStats,
    pub packet_sequence: Vec<(String, u64, u64)>, // (mac, packet_id, timestamp_ms)
    pub raw_packets: Vec<RawPacketModel>,         // Raw packets for database persistence
    pub telemetry_json: String,
    pub duration_ms: u64,
    pub event_count: u64,
}

impl std::fmt::Display for ScanEngineResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "📊 SCAN RESULTS:\n  \
             Devices: {}\n  \
             Packets Ordered: {}\n  \
             Events: {}\n  \
             Duration: {}ms\n  {}",
            self.devices.len(),
            self.packet_sequence.len(),
            self.event_count,
            self.duration_ms,
            self.scanner_stats
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = UnifiedScanEngine::new(ScanConfig::default());
        assert!(!engine.native_scanner.get_capabilities().platform.is_empty());
    }
}
