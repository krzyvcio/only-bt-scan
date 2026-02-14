/// Unified Scan Engine - Integrates all scanning methods and event handling
///
/// Coordinates:
/// - Native platform scanners (Windows, Linux, macOS)
/// - Packet ordering and deduplication
/// - Device event listening
/// - Raw HCI scanning
/// - Telemetry collection

use crate::bluetooth_scanner::{BluetoothScanner, BluetoothDevice, ScanConfig};
use crate::native_scanner::NativeBluetoothScanner;
use crate::packet_tracker::GlobalPacketTracker;
use crate::device_events::{DeviceEventListener, BluetoothDeviceEvent};
use crate::scanner_integration::ScannerWithTracking;
use crate::telemetry::TelemetryCollector;
use crate::data_models::RawPacketModel;
use log::{info, debug, warn};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main scanning engine combining all subsystems
pub struct UnifiedScanEngine {
    config: ScanConfig,
    native_scanner: NativeBluetoothScanner,
    tracker_system: ScannerWithTracking,
    event_listener: Arc<DeviceEventListener>,
    #[cfg(target_os = "windows")]
    hci_scanner: Option<crate::windows_hci::windows_hci::WindowsHciScanner>,
}

impl UnifiedScanEngine {
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

        Self {
            config,
            native_scanner,
            tracker_system,
            event_listener,
            #[cfg(target_os = "windows")]
            hci_scanner: None,
        }
    }

    /// Run integrated scan operation
    pub async fn run_scan(&mut self) -> Result<ScanEngineResults, Box<dyn std::error::Error>> {
        info!("🔄 Starting unified scan cycle");

        let start_time = std::time::Instant::now();

        // Phase 1: Run native platform scanner
        info!("📡 Phase 1: Native platform scanning");
        let native_devices = self.native_scanner.run_native_scan().await?;
        info!("✅ Phase 1 complete: {} devices found", native_devices.len());

        // Phase 2: Process devices through packet tracker
        info!("📊 Phase 2: Packet ordering and deduplication");
        self.tracker_system.process_scan_results(native_devices.clone());

        // Phase 3: Emit events for newly discovered devices
        info!("🎧 Phase 3: Device event emission");
        for device in &native_devices {
            self.event_listener.emit(BluetoothDeviceEvent::DeviceDiscovered {
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
                info!("✅ Phase 4 complete: {} devices from HCI", hci_devices.len());
            }
        }

        let duration = start_time.elapsed();

        // Collect results
        let stats = self.tracker_system.get_stats();
        let telemetry = self.tracker_system.export_telemetry();

        Ok(ScanEngineResults {
            devices: native_devices,
            scanner_stats: stats,
            packet_sequence: self.tracker_system.get_packet_ordering(),
            telemetry_json: telemetry,
            duration_ms: duration.as_millis() as u64,
            event_count: self.tracker_system.packet_tracker.packet_count,
        })
    }

    /// Run HCI-only scan on Windows
    #[cfg(target_os = "windows")]
    async fn scan_windows_hci(&mut self) -> Result<Vec<BluetoothDevice>, Box<dyn std::error::Error>> {
        use crate::windows_hci::windows_hci::WindowsHciScanner;

        info!("🪟 Initializing Windows HCI scanner");

        let mut hci_scanner = WindowsHciScanner::new("BT0".to_string());
        hci_scanner.start_scan().await?;

        let mut devices = Vec::new();

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
    async fn scan_windows_hci(&mut self) -> Result<Vec<BluetoothDevice>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }

    /// Get event listener
    pub fn get_event_listener(&self) -> Arc<DeviceEventListener> {
        self.event_listener.clone()
    }

    /// Export full telemetry
    pub fn export_telemetry(&self) -> String {
        self.tracker_system.export_telemetry()
    }

    /// Export device-specific telemetry
    pub fn export_device_telemetry(&self, mac: &str) -> Option<String> {
        self.tracker_system.export_device_telemetry(mac)
    }

    /// Get packet sequence for device
    pub fn get_device_packet_sequence(&self, mac: &str) -> Option<Vec<u64>> {
        self.tracker_system.get_device_sequence(mac)
    }

    /// Get global packet ordering
    pub fn get_global_packet_order(&self) -> Vec<(String, u64, u64)> {
        self.tracker_system.get_packet_ordering()
    }
}

/// Results from single scan operation
#[derive(Debug, Clone)]
pub struct ScanEngineResults {
    pub devices: Vec<BluetoothDevice>,
    pub scanner_stats: crate::scanner_integration::ScannerTrackingStats,
    pub packet_sequence: Vec<(String, u64, u64)>, // (mac, packet_id, timestamp_ms)
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
