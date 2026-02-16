use crate::device_tracker::DeviceTrackerManager;
use btleplug::api::{Central, Manager as ManagerTrait, Peripheral as PeripheralTrait, ScanFilter};
use btleplug::platform::Manager;
use chrono::{DateTime, Utc};
use colored::Colorize;
use log::{debug, info, warn};
/// Multi-Method Unified Bluetooth Scanner
///
/// Combines ALL available scanning methods simultaneously to maximize device detection
/// - btleplug standard scanning
/// - Windows HCI raw packets
/// - Windows Bluetooth API
/// - Platform-native scanners
/// - Real-time HCI capture
/// - Deep packet analysis
/// - Security & vendor-specific detection
/// - Android bridge scanning
/// - CoreBluetooth (macOS/iOS)
///
/// A device might only be visible through ONE method, so we use ALL of them!
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::task::JoinSet;

/// Complete device info merged from all detection methods
#[derive(Debug, Clone)]
pub struct UnifiedDevice {
    // === Identification ===
    pub mac_address: String,
    pub device_name: Option<String>,

    // === Detection Methods Used ===
    pub detected_by_btleplug: bool,
    pub detected_by_hci_raw: bool,
    pub detected_by_windows_api: bool,
    pub detected_by_hci_realtime: bool,
    pub detected_by_vendor_protocol: bool,
    pub detected_by_android_bridge: bool,
    pub detected_by_corebluetooth: bool,
    pub detection_methods_count: usize,

    // === Signal & Physical ===
    pub rssi: i8,
    pub tx_power: Option<i8>,
    pub phy: Option<String>,
    pub channel: Option<u8>,

    // === Device Info ===
    pub manufacturer_id: Option<u16>,
    pub manufacturer_name: String,
    pub device_type: Option<String>,
    pub device_class: Option<String>,

    // === Services & Features ===
    pub advertised_services: Vec<String>,
    pub security_flags: Option<String>,
    pub pairing_capable: bool,
    pub le_audio_capable: bool,

    // === Vendor-Special ===
    pub vendor_protocol: Option<String>,
    pub l2cap_channels: Vec<u16>,
    pub raw_manufacturing_data: Vec<u8>,

    // === Parsed Advertising Data ===
    pub ad_flags: Option<String>,
    pub ad_local_name: Option<String>,
    pub ad_tx_power: Option<i8>,
    pub ad_appearance: Option<String>,
    pub ad_service_uuids: Vec<String>,
    pub ad_manufacturer_data: Option<String>,
    pub ad_service_data: Vec<String>,

    // === Temporal ===
    pub first_detected: DateTime<Utc>,
    pub last_detected: DateTime<Utc>,
    pub detection_count: u64,
    pub packet_count: u64,
}

impl UnifiedDevice {
    pub fn new(mac: String) -> Self {
        Self {
            mac_address: mac,
            device_name: None,
            detected_by_btleplug: false,
            detected_by_hci_raw: false,
            detected_by_windows_api: false,
            detected_by_hci_realtime: false,
            detected_by_vendor_protocol: false,
            detected_by_android_bridge: false,
            detected_by_corebluetooth: false,
            detection_methods_count: 0,
            rssi: 0,
            tx_power: None,
            phy: None,
            channel: None,
            manufacturer_id: None,
            manufacturer_name: "Unknown".to_string(),
            device_type: None,
            device_class: None,
            advertised_services: Vec::new(),
            security_flags: None,
            pairing_capable: false,
            le_audio_capable: false,
            vendor_protocol: None,
            l2cap_channels: Vec::new(),
            raw_manufacturing_data: Vec::new(),
            ad_flags: None,
            ad_local_name: None,
            ad_tx_power: None,
            ad_appearance: None,
            ad_service_uuids: Vec::new(),
            ad_manufacturer_data: None,
            ad_service_data: Vec::new(),
            first_detected: Utc::now(),
            last_detected: Utc::now(),
            detection_count: 0,
            packet_count: 0,
        }
    }

    fn count_detection_methods(&mut self) {
        self.detection_methods_count = [
            self.detected_by_btleplug,
            self.detected_by_hci_raw,
            self.detected_by_windows_api,
            self.detected_by_hci_realtime,
            self.detected_by_vendor_protocol,
            self.detected_by_android_bridge,
            self.detected_by_corebluetooth,
        ]
        .iter()
        .filter(|&&x| x)
        .count();
    }

    /// Get detection confidence (0-7, # of methods that detected it)
    pub fn detection_confidence(&self) -> usize {
        self.detection_methods_count
    }

    /// Pretty-print detection methods used
    pub fn detection_methods_str(&self) -> String {
        let mut methods = Vec::new();
        if self.detected_by_btleplug {
            methods.push("btleplug")
        }
        if self.detected_by_hci_raw {
            methods.push("HCI-raw")
        }
        if self.detected_by_windows_api {
            methods.push("API")
        }
        if self.detected_by_hci_realtime {
            methods.push("HCI-rt")
        }
        if self.detected_by_vendor_protocol {
            methods.push("Vendor")
        }
        if self.detected_by_android_bridge {
            methods.push("Android")
        }
        if self.detected_by_corebluetooth {
            methods.push("CoreBT")
        }

        format!("[{}]", methods.join("|"))
    }

    /// Parse and store advertising data
    pub fn parse_advertising_data(&mut self) {
        if self.raw_manufacturing_data.is_empty() {
            return;
        }

        use crate::advertising_parser::parse_advertising_packet;
        
        let parsed = parse_advertising_packet(
            &self.mac_address,
            self.rssi,
            &self.raw_manufacturing_data,
            false,
        );

        // Store flags
        if let Some(flags) = parsed.flags {
            let flags_str = format!(
                "LE:{}, LE_Gen:{}, BR_EDR:{}, LE+BR:{}, BR+LE_Host:{}",
                flags.le_limited_discoverable,
                flags.le_general_discoverable,
                flags.br_edr_not_supported,
                flags.simultaneous_le_and_br_edr_controller,
                flags.simultaneous_le_and_br_edr_host
            );
            self.ad_flags = Some(flags_str);
        }

        // Store local name
        if let Some(name) = parsed.local_name {
            self.ad_local_name = Some(name);
        }

        // Store TX Power
        if let Some(tx) = parsed.tx_power {
            self.ad_tx_power = Some(tx);
        }

        // Store appearance
        if let Some(app) = parsed.appearance {
            self.ad_appearance = Some(format!("0x{:04X}", app));
        }

        // Store 16-bit service UUIDs
        for uuid in &parsed.services_16bit {
            self.ad_service_uuids.push(format!("0x{:04X}", uuid));
        }

        // Store manufacturer data as hex string
        for (mfg_id, data) in &parsed.manufacturer_data {
            let hex_data = data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
            self.ad_manufacturer_data = Some(format!("{:04X}: {}", mfg_id, hex_data));
        }

        // Store service data
        for (uuid, data) in &parsed.service_data_16 {
            let hex_data = data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
            self.ad_service_data.push(format!("0x{:04X}: {}", uuid, hex_data));
        }
    }
}

/// Multi-Method Scanner Coordinator
pub struct MultiMethodScanner {
    devices: Arc<Mutex<HashMap<String, UnifiedDevice>>>,
    is_scanning: bool,
}

impl MultiMethodScanner {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(Mutex::new(HashMap::new())),
            is_scanning: false,
        }
    }

    /// Start ALL scanning methods in parallel
    pub async fn start_comprehensive_scan(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🚀 Starting Multi-Method Comprehensive BLE Scanner");
        info!("   Using ALL available detection methods in parallel...");

        self.is_scanning = true;
        let devices = self.devices.clone();

        // Create task set for parallel scanning
        let mut tasks = JoinSet::new();

        // Method 1: btleplug standard scanning
        let devices_1 = devices.clone();
        tasks.spawn(async move {
            info!("  ▶ Method 1/7: btleplug standard scanner");
            if let Err(e) = Self::scan_with_btleplug(devices_1).await {
                warn!("⚠️  btleplug scan failed: {}", e);
            }
        });

        // Method 2: Windows HCI Raw packets
        #[cfg(target_os = "windows")]
        {
            let devices_2 = devices.clone();
            tasks.spawn(async move {
                info!("  ▶ Method 2/7: Windows HCI raw packets");
                if let Err(e) = Self::scan_with_hci_raw(devices_2).await {
                    warn!("⚠️  HCI raw scan failed: {}", e);
                }
            });
        }

        // Method 3: Windows Bluetooth API
        #[cfg(target_os = "windows")]
        {
            let devices_3 = devices.clone();
            tasks.spawn(async move {
                info!("  ▶ Method 3/7: Windows Bluetooth API");
                if let Err(e) = Self::scan_with_windows_api(devices_3).await {
                    warn!("⚠️  Windows API scan failed: {}", e);
                }
            });
        }

        // Method 4: Real-time HCI capture
        let devices_4 = devices.clone();
        tasks.spawn(async move {
            info!("  ▶ Method 4/7: Real-time HCI capture");
            if let Err(e) = Self::scan_with_hci_realtime(devices_4).await {
                warn!("⚠️  HCI realtime scan failed: {}", e);
            }
        });

        // Method 5: Vendor-specific protocol detection
        let devices_5 = devices.clone();
        tasks.spawn(async move {
            info!("  ▶ Method 5/7: Vendor-specific protocols");
            if let Err(e) = Self::scan_with_vendor_protocols(devices_5).await {
                warn!("⚠️  Vendor protocol scan failed: {}", e);
            }
        });

        // Method 6: Android BLE bridge
        #[cfg(target_os = "android")]
        {
            let devices_6 = devices.clone();
            tasks.spawn(async move {
                info!("  ▶ Method 6/7: Android BLE bridge");
                if let Err(e) = Self::scan_with_android_bridge(devices_6).await {
                    warn!("⚠️  Android bridge scan failed: {}", e);
                }
            });
        }

        // Method 7: CoreBluetooth (macOS/iOS)
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            let devices_7 = devices.clone();
            tasks.spawn(async move {
                info!("  ▶ Method 7/7: CoreBluetooth (macOS/iOS)");
                if let Err(e) = Self::scan_with_corebluetooth(devices_7).await {
                    warn!("⚠️  CoreBluetooth scan failed: {}", e);
                }
            });
        }

        // Wait for all methods to complete (with timeout)
        let timeout = Duration::from_secs(30);
        let start = std::time::Instant::now();

        while let Some(result) = tasks.join_next().await {
            if start.elapsed() > timeout {
                warn!("⏱️  Scan timeout reached");
                break;
            }
            if let Err(e) = result {
                warn!("Task error: {}", e);
            }
        }

        info!("✅ All scanning methods completed");
        self.print_discovery_summary();

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // SCANNING METHODS
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Method 1: btleplug (standard cross-platform)
    async fn scan_with_btleplug(
        devices: Arc<Mutex<HashMap<String, UnifiedDevice>>>,
    ) -> Result<(), String> {
        debug!("btleplug scan: Using standard BLE scanning");

        // Create platform manager
        let manager = Manager::new().await.map_err(|e| e.to_string())?;
        let adapters = manager.adapters().await.map_err(|e| e.to_string())?;

        if adapters.is_empty() {
            warn!("❌ No Bluetooth adapters found for btleplug");
            return Ok(());
        }

        for adapter in adapters {
            info!("  ▶ btleplug using adapter");

            // Start scanning on this adapter
            if let Err(e) = adapter.start_scan(ScanFilter::default()).await {
                warn!("Failed to start scan on adapter: {}", e);
                continue;
            }

            // Scan for specified duration
            tokio::time::sleep(Duration::from_secs(10)).await;

            // Get discovered peripherals
            let peripherals = adapter.peripherals().await.map_err(|e| e.to_string())?;
            info!("  ✓ btleplug discovered {} devices", peripherals.len());

            for peripheral in peripherals {
                // Get device properties
                let properties = match peripheral.properties().await {
                    Ok(Some(props)) => props,
                    _ => continue,
                };

                let mac = properties.address.to_string();

                // Update or create device entry
                {
                    let mut devices_lock = devices.lock().unwrap();
                    let device = devices_lock
                        .entry(mac.clone())
                        .or_insert_with(|| UnifiedDevice::new(mac.clone()));

                    // Mark as detected by btleplug
                    device.detected_by_btleplug = true;
                    device.count_detection_methods();

                    // Update device info from properties
                    if let Some(name) = &properties.local_name {
                        device.device_name = Some(name.clone());
                    }

                    if let Some(rssi) = properties.rssi {
                        device.rssi = rssi as i8;
                    }

                    if let Some(tx_power) = properties.tx_power_level {
                        device.tx_power = Some(tx_power as i8);
                    }

                    device.last_detected = Utc::now();
                    device.detection_count += 1;
                }

                debug!(
                    "btleplug: {} - {} (RSSI: {})",
                    mac,
                    properties.local_name.as_deref().unwrap_or("Unknown"),
                    properties.rssi.unwrap_or(0)
                );
            }

            // Stop scanning
            if let Err(e) = adapter.stop_scan().await {
                warn!("Failed to stop scan: {}", e);
            }
        }

        Ok(())
    }

    /// Method 2: Windows HCI Raw (low-level packets)
    #[cfg(target_os = "windows")]
    async fn scan_with_hci_raw(
        devices: Arc<Mutex<HashMap<String, UnifiedDevice>>>,
    ) -> Result<(), String> {
        use crate::windows_hci::WindowsHciScanner;

        debug!("HCI raw scan: Capturing low-level HCI packets");

        let tracker = DeviceTrackerManager::new();
        let mut scanner = WindowsHciScanner::new("primary".to_string());

        // Open connection and start scanning
        match scanner.start_scan().await {
            Ok(_) => {
                info!("  ▶ HCI raw scanner started");
            }
            Err(e) => {
                warn!("❌ Failed to start HCI raw scanner: {}", e);
                return Ok(());
            }
        }

        // Capture HCI events for 10 seconds
        let scan_start = std::time::Instant::now();
        let scan_duration = Duration::from_secs(10);
        let mut device_count = 0;

        while scan_start.elapsed() < scan_duration {
            match scanner.receive_advertisement().await {
                Ok(Some(report)) => {
                    let mac = report.address.clone();
                    let rssi = report.rssi;

                    // Update UnifiedDevice
                    {
                        let mut devices_lock = devices.lock().unwrap();
                        let device = devices_lock
                            .entry(mac.clone())
                            .or_insert_with(|| UnifiedDevice::new(mac.clone()));

                        device.detected_by_hci_raw = true;
                        device.count_detection_methods();
                        device.rssi = rssi;
                        device.last_detected = Utc::now();
                        device.detection_count += 1;
                        device.packet_count += 1;
                        device.raw_manufacturing_data = report.advertising_data.clone();
                        device.parse_advertising_data();
                    }

                    // Log to device tracker
                    tracker.record_detection(&mac, rssi, "hci_raw", None, None);

                    device_count += 1;
                    debug!("HCI raw: {} - RSSI: {} dBm", mac, rssi);
                }
                Ok(None) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(_e) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }

        // Stop scanning
        match scanner.stop_scan().await {
            Ok(_) => {
                info!("  ✓ HCI raw scan completed ({} devices)", device_count);
            }
            Err(e) => {
                warn!("Failed to stop HCI scan: {}", e);
            }
        }

        // Persist all tracked devices to database
        if let Err(e) = tracker.persist_all() {
            warn!("Failed to persist HCI raw devices to database: {}", e);
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    async fn scan_with_hci_raw(
        _devices: Arc<Mutex<HashMap<String, UnifiedDevice>>>,
    ) -> Result<(), String> {
        Ok(())
    }

    /// Method 3: Windows Bluetooth API
    #[cfg(target_os = "windows")]
    async fn scan_with_windows_api(
        devices: Arc<Mutex<HashMap<String, UnifiedDevice>>>,
    ) -> Result<(), String> {
        use crate::windows_bluetooth::windows_bt::WindowsBluetoothManager;

        debug!("Windows API scan: Using native Bluetooth API");

        let tracker = DeviceTrackerManager::new();
        let mut manager = WindowsBluetoothManager::new();

        // Enumerate paired devices via Windows Bluetooth API
        match manager.enumerate_devices().await {
            Ok(bt_devices) => {
                let device_count = bt_devices.len();
                info!("  ▶ Windows API enumerated {} paired devices", device_count);

                for bt_device in bt_devices {
                    let mac = bt_device.mac_address.clone();
                    let name = bt_device.name.clone();
                    let rssi = bt_device.rssi;

                    // Update UnifiedDevice
                    {
                        let mut devices_lock = devices.lock().unwrap();
                        let device = devices_lock
                            .entry(mac.clone())
                            .or_insert_with(|| UnifiedDevice::new(mac.clone()));

                        device.detected_by_windows_api = true;
                        device.count_detection_methods();
                        device.rssi = rssi;
                        device.last_detected = Utc::now();
                        device.detection_count += 1;

                        if let Some(n) = &name {
                            device.device_name = Some(n.clone());
                        }
                    }

                    // Log to device tracker
                    let name_for_logging = name.clone();
                    tracker.record_detection(&mac, rssi, "windows_api", name, None);

                    debug!(
                        "Windows API: {} - {} (RSSI: {} dBm)",
                        mac,
                        name_for_logging.as_deref().unwrap_or("Unknown"),
                        rssi
                    );
                }

                info!("  ✓ Windows API scan completed ({} devices)", device_count);
            }
            Err(e) => {
                warn!("❌ Failed to enumerate devices via Windows API: {}", e);
            }
        }

        // Persist all tracked devices to database
        if let Err(e) = tracker.persist_all() {
            warn!("Failed to persist Windows API devices to database: {}", e);
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    async fn scan_with_windows_api(
        _devices: Arc<Mutex<HashMap<String, UnifiedDevice>>>,
    ) -> Result<(), String> {
        Ok(())
    }

    /// Method 4: Real-time HCI capture
    async fn scan_with_hci_realtime(
        devices: Arc<Mutex<HashMap<String, UnifiedDevice>>>,
    ) -> Result<(), String> {
        use crate::hci_realtime_capture::HciRealTimeSniffer;
        use tokio::sync::mpsc;

        debug!("HCI realtime: Capturing real-time HCI events");

        let tracker = DeviceTrackerManager::new();
        let mut sniffer = HciRealTimeSniffer::new();

        // Create channel for packet communication
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Start HCI sniffer
        match sniffer.start(tx) {
            Ok(_) => {
                info!("  ▶ Real-time HCI sniffer started");
            }
            Err(e) => {
                warn!(
                    "❌ Failed to start HCI real-time sniffer: {} (requires admin)",
                    e
                );
                return Ok(());
            }
        }

        // Capture HCI events for 10 seconds in parallel
        let capture_duration = Duration::from_secs(10);
        let capture_start = std::time::Instant::now();
        let mut device_count = 0;

        // Run packet reception in timeout
        let timeout_future = async {
            while let Some(packet) = rx.recv().await {
                if capture_start.elapsed() > capture_duration {
                    break;
                }

                let mac = packet.mac_address.clone();
                let rssi = packet.rssi;

                // Update UnifiedDevice
                {
                    let mut devices_lock = devices.lock().unwrap();
                    let device = devices_lock
                        .entry(mac.clone())
                        .or_insert_with(|| UnifiedDevice::new(mac.clone()));

                    device.detected_by_hci_realtime = true;
                    device.count_detection_methods();
                    device.rssi = rssi;
                    device.last_detected = Utc::now();
                    device.detection_count += 1;
                    device.packet_count += 1;
                    device.raw_manufacturing_data = packet.advertising_data.clone();
                    device.parse_advertising_data();
                }

                // Log to device tracker
                tracker.record_detection(&mac, rssi, "hci_realtime", None, None);

                device_count += 1;
                debug!("HCI realtime: {} - RSSI: {} dBm", mac, rssi);
            }
        };

        // Run with timeout
        match tokio::time::timeout(capture_duration, timeout_future).await {
            Ok(_) => {
                info!("  ✓ HCI realtime scan completed ({} devices)", device_count);
            }
            Err(_) => {
                info!(
                    "  ✓ HCI realtime scan timeout reached ({} devices)",
                    device_count
                );
            }
        }

        // Stop sniffer
        sniffer.stop();

        // Persist all tracked devices to database
        if let Err(e) = tracker.persist_all() {
            warn!("Failed to persist HCI realtime devices to database: {}", e);
        }

        Ok(())
    }

    /// Method 5: Vendor-specific protocols
    async fn scan_with_vendor_protocols(
        devices: Arc<Mutex<HashMap<String, UnifiedDevice>>>,
    ) -> Result<(), String> {
        use crate::advertising_parser::parse_advertising_packet;
        use crate::vendor_protocols::{parse_vendor_protocols, VendorProtocol};

        debug!("Vendor protocols: Detecting vendor-specific marketing frames");

        let tracker = DeviceTrackerManager::new();

        let device_snapshots: Vec<(String, i8, Vec<u8>)> = {
            let devices_lock = devices.lock().unwrap();
            devices_lock
                .values()
                .filter(|d| !d.raw_manufacturing_data.is_empty())
                .map(|d| {
                    (
                        d.mac_address.clone(),
                        d.rssi,
                        d.raw_manufacturing_data.clone(),
                    )
                })
                .collect()
        };

        let mut detected_count = 0;

        for (mac, rssi, raw_data) in device_snapshots {
            let parsed = parse_advertising_packet(&mac, rssi, &raw_data, false);
            let protocols = parse_vendor_protocols(&parsed);

            if protocols.is_empty() {
                continue;
            }

            let mut protocol_names = Vec::new();
            for protocol in &protocols {
                let name = match protocol {
                    VendorProtocol::IBeacon(_) => "iBeacon",
                    VendorProtocol::Eddystone(_) => "Eddystone",
                    VendorProtocol::AltBeacon(_) => "AltBeacon",
                    VendorProtocol::AppleContinuity(_) => "AppleContinuity",
                    VendorProtocol::GoogleFastPair(_) => "GoogleFastPair",
                    VendorProtocol::MicrosoftSwiftPair(_) => "MicrosoftSwiftPair",
                };
                protocol_names.push(name);
            }

            let protocol_summary = protocol_names.join("|");

            {
                let mut devices_lock = devices.lock().unwrap();
                if let Some(device) = devices_lock.get_mut(&mac) {
                    device.detected_by_vendor_protocol = true;
                    device.count_detection_methods();
                    device.vendor_protocol = Some(protocol_summary.clone());
                    device.last_detected = Utc::now();
                    device.detection_count += 1;
                }
            }

            tracker.record_detection(&mac, rssi, "vendor_protocol", None, None);

            detected_count += 1;
            info!(
                "  ✓ Vendor protocols detected for {} [{}]",
                mac, protocol_summary
            );
        }

        if let Err(e) = tracker.persist_all() {
            warn!(
                "Failed to persist vendor protocol devices to database: {}",
                e
            );
        }

        info!(
            "  ✓ Vendor protocol scan completed ({} devices)",
            detected_count
        );
        Ok(())
    }

    /// Method 6: Android BLE bridge
    #[cfg(target_os = "android")]
    async fn scan_with_android_bridge(
        devices: Arc<Mutex<HashMap<String, UnifiedDevice>>>,
    ) -> Result<(), String> {
        use crate::android_ble_bridge::AndroidBleScanner;
        use crate::db;

        debug!("Android bridge: Using Android BLE scanner");

        let tracker = DeviceTrackerManager::new();
        let mut scanner = AndroidBleScanner::default();

        if let Err(e) = scanner.start_scan() {
            warn!("❌ Failed to start Android BLE scan: {}", e);
            return Ok(());
        }

        let duration_ms = scanner.scan_duration_ms() as u64;
        tokio::time::sleep(Duration::from_millis(duration_ms)).await;
        let _ = scanner.stop_scan();

        let android_devices = scanner.get_devices();

        for android_device in android_devices.iter() {
            let mac = android_device.address.clone();
            let rssi = android_device.rssi;
            let timestamp_ms = if android_device.last_seen > 0 {
                android_device.last_seen
            } else {
                chrono::Utc::now().timestamp_millis() as u64
            };

            let mut ad_structures: Vec<u8> = Vec::new();

            if let Some(name) = android_device.name.as_ref() {
                let name_bytes = name.as_bytes();
                let len = (name_bytes.len() + 1) as u8;
                ad_structures.push(len);
                ad_structures.push(0x09);
                ad_structures.extend_from_slice(name_bytes);
            }

            if let Some(tx_power) = android_device.tx_power {
                ad_structures.push(2);
                ad_structures.push(0x0A);
                ad_structures.push(tx_power as u8);
            }

            if let Some(flags) = android_device.flags {
                ad_structures.push(2);
                ad_structures.push(0x01);
                ad_structures.push(flags);
            }

            if let Some(appearance) = android_device.appearance {
                let bytes = appearance.to_le_bytes();
                ad_structures.push(3);
                ad_structures.push(0x19);
                ad_structures.extend_from_slice(&bytes);
            }

            let mut svc_16: Vec<u8> = Vec::new();
            let mut svc_128: Vec<u8> = Vec::new();

            for uuid in &android_device.advertised_services {
                let cleaned = uuid.trim_start_matches("0x").replace('-', "");
                if cleaned.len() == 4 {
                    if let Ok(uuid16) = u16::from_str_radix(&cleaned, 16) {
                        svc_16.extend_from_slice(&uuid16.to_le_bytes());
                    }
                } else if cleaned.len() == 32 {
                    if let Ok(bytes) = hex::decode(&cleaned) {
                        for b in bytes.iter().rev() {
                            svc_128.push(*b);
                        }
                    }
                }
            }

            if !svc_16.is_empty() {
                let len = (svc_16.len() + 1) as u8;
                ad_structures.push(len);
                ad_structures.push(0x03);
                ad_structures.extend_from_slice(&svc_16);
            }

            if !svc_128.is_empty() {
                let len = (svc_128.len() + 1) as u8;
                ad_structures.push(len);
                ad_structures.push(0x07);
                ad_structures.extend_from_slice(&svc_128);
            }

            for (mfg_id, mfg_data) in &android_device.manufacturer_data {
                let mut payload = Vec::with_capacity(2 + mfg_data.len());
                payload.extend_from_slice(&mfg_id.to_le_bytes());
                payload.extend_from_slice(mfg_data);
                let len = (payload.len() + 1) as u8;
                ad_structures.push(len);
                ad_structures.push(0xFF);
                ad_structures.extend_from_slice(&payload);
            }

            for (uuid, svc_data) in &android_device.service_data {
                let cleaned = uuid.trim_start_matches("0x").replace('-', "");
                if cleaned.len() == 4 {
                    if let Ok(uuid16) = u16::from_str_radix(&cleaned, 16) {
                        let mut payload = Vec::with_capacity(2 + svc_data.len());
                        payload.extend_from_slice(&uuid16.to_le_bytes());
                        payload.extend_from_slice(svc_data);
                        let len = (payload.len() + 1) as u8;
                        ad_structures.push(len);
                        ad_structures.push(0x16);
                        ad_structures.extend_from_slice(&payload);
                    }
                } else if cleaned.len() == 32 {
                    if let Ok(bytes) = hex::decode(&cleaned) {
                        let mut payload = Vec::with_capacity(16 + svc_data.len());
                        for b in bytes.iter().rev() {
                            payload.push(*b);
                        }
                        payload.extend_from_slice(svc_data);
                        let len = (payload.len() + 1) as u8;
                        ad_structures.push(len);
                        ad_structures.push(0x21);
                        ad_structures.extend_from_slice(&payload);
                    }
                }
            }

            let advertising_data_hex = hex::encode(&ad_structures);

            let (manufacturer_id, manufacturer_name) = android_device
                .manufacturer_data
                .iter()
                .next()
                .map(|(id, _)| {
                    let name = company_id_reference::lookup_company_id(*id)
                        .unwrap_or("Unknown")
                        .to_string();
                    (Some(*id), name)
                })
                .unwrap_or((None, "Unknown".to_string()));

            {
                let mut devices_lock = devices.lock().unwrap();
                let device = devices_lock
                    .entry(mac.clone())
                    .or_insert_with(|| UnifiedDevice::new(mac.clone()));

                device.detected_by_android_bridge = true;
                device.count_detection_methods();
                device.rssi = rssi;
                device.last_detected = Utc::now();
                device.detection_count += 1;
                device.manufacturer_id = manufacturer_id;
                device.manufacturer_name = manufacturer_name;
                device.advertised_services = android_device.advertised_services.clone();

                if let Some(name) = android_device.name.as_ref() {
                    device.device_name = Some(name.clone());
                }
            }

            tracker.record_detection(
                &mac,
                rssi,
                "android_bridge",
                android_device.name.clone(),
                manufacturer_id,
            );

            if !advertising_data_hex.is_empty() {
                let _ = db::insert_advertisement_frame(
                    &mac,
                    rssi,
                    &advertising_data_hex,
                    "Android",
                    0,
                    "ADV_IND",
                    timestamp_ms,
                );
            }
        }

        if let Err(e) = tracker.persist_all() {
            warn!(
                "Failed to persist Android bridge devices to database: {}",
                e
            );
        }

        info!(
            "  ✓ Android bridge scan completed ({} devices)",
            android_devices.len()
        );
        Ok(())
    }

    #[cfg(not(target_os = "android"))]
    async fn scan_with_android_bridge(
        _devices: Arc<Mutex<HashMap<String, UnifiedDevice>>>,
    ) -> Result<(), String> {
        Ok(())
    }

    /// Method 7: CoreBluetooth (macOS/iOS)
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn scan_with_corebluetooth(
        devices: Arc<Mutex<HashMap<String, UnifiedDevice>>>,
    ) -> Result<(), String> {
        use crate::core_bluetooth_integration::{CoreBluetoothConfig, CoreBluetoothScanner};

        debug!("CoreBluetooth: Using native macOS/iOS scanner");

        let tracker = DeviceTrackerManager::new();
        let scanner = CoreBluetoothScanner::new(CoreBluetoothConfig::default());

        let results = scanner.scan().await.map_err(|e| e.to_string())?;

        for device_model in results.iter() {
            let mac = device_model.mac_address.clone();
            let rssi = device_model.rssi;
            let manufacturer_id = device_model.manufacturer_id;
            let manufacturer_name = device_model
                .manufacturer_name
                .clone()
                .unwrap_or_else(|| "Unknown".to_string());

            {
                let mut devices_lock = devices.lock().unwrap();
                let device = devices_lock
                    .entry(mac.clone())
                    .or_insert_with(|| UnifiedDevice::new(mac.clone()));

                device.detected_by_corebluetooth = true;
                device.count_detection_methods();
                device.rssi = rssi;
                device.last_detected = Utc::now();
                device.detection_count += 1;
                device.manufacturer_id = manufacturer_id;
                device.manufacturer_name = manufacturer_name;
                device.advertised_services = device_model.advertised_services.clone();
                device.tx_power = device_model.tx_power;
                device.device_type = Some(format!("{:?}", device_model.device_type));

                if let Some(name) = device_model.device_name.as_ref() {
                    device.device_name = Some(name.clone());
                }
            }

            tracker.record_detection(
                &mac,
                rssi,
                "corebluetooth",
                device_model.device_name.clone(),
                manufacturer_id,
            );
        }

        if let Err(e) = tracker.persist_all() {
            warn!("Failed to persist CoreBluetooth devices to database: {}", e);
        }

        info!(
            "  ✓ CoreBluetooth scan completed ({} devices)",
            results.len()
        );
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    async fn scan_with_corebluetooth(
        _devices: Arc<Mutex<HashMap<String, UnifiedDevice>>>,
    ) -> Result<(), String> {
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // DEVICE MANAGEMENT
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Get or create device entry
    pub fn get_or_create_device(&self, mac: &str) -> Arc<Mutex<UnifiedDevice>> {
        let mut devices_lock = self.devices.lock().unwrap();
        devices_lock
            .entry(mac.to_string())
            .or_insert_with(|| UnifiedDevice::new(mac.to_string()));

        Arc::new(Mutex::new(devices_lock.get(mac).unwrap().clone()))
    }

    /// Get all discovered devices sorted by confidence
    pub fn get_devices_by_confidence(&self) -> Vec<UnifiedDevice> {
        let devices_lock = self.devices.lock().unwrap();
        let mut devices: Vec<_> = devices_lock.values().cloned().collect();
        devices.sort_by(|a, b| {
            b.detection_methods_count
                .cmp(&a.detection_methods_count)
                .then_with(|| b.packet_count.cmp(&a.packet_count))
        });
        devices
    }

    /// Get devices detected by specific method
    pub fn get_devices_by_method(&self, method: &str) -> Vec<UnifiedDevice> {
        let devices_lock = self.devices.lock().unwrap();
        devices_lock
            .values()
            .filter(|d| match method {
                "btleplug" => d.detected_by_btleplug,
                "hci_raw" => d.detected_by_hci_raw,
                "windows_api" => d.detected_by_windows_api,
                "hci_realtime" => d.detected_by_hci_realtime,
                "vendor" => d.detected_by_vendor_protocol,
                "android" => d.detected_by_android_bridge,
                "corebluetooth" => d.detected_by_corebluetooth,
                _ => false,
            })
            .cloned()
            .collect()
    }

    /// Get devices detected by ONLY one method (unique finds)
    pub fn get_unique_devices(&self) -> Vec<UnifiedDevice> {
        self.devices
            .lock()
            .unwrap()
            .values()
            .filter(|d| d.detection_methods_count == 1)
            .cloned()
            .collect()
    }

    /// Print comprehensive discovery summary
    pub fn print_discovery_summary(&self) {
        let devices = self.get_devices_by_confidence();

        if devices.is_empty() {
            println!("❌ No devices discovered");
            return;
        }

        println!(
            "\n{}",
            "═══════════════════════════════════════════════════════════════".cyan()
        );
        println!("📊 Multi-Method Discovery Summary");
        println!(
            "{}",
            "═══════════════════════════════════════════════════════════════".cyan()
        );

        // Overall stats
        let total_devices = devices.len();
        let _multi_method = devices
            .iter()
            .filter(|d| d.detection_methods_count > 1)
            .count();
        let unique = devices
            .iter()
            .filter(|d| d.detection_methods_count == 1)
            .count();

        println!("📱 Total Devices: {}", total_devices);
        println!(
            "  ├─ Found by 7 methods: {}",
            devices
                .iter()
                .filter(|d| d.detection_methods_count == 7)
                .count()
        );
        println!(
            "  ├─ Found by 3-6 methods: {}",
            devices
                .iter()
                .filter(|d| d.detection_methods_count >= 3 && d.detection_methods_count < 7)
                .count()
        );
        println!(
            "  ├─ Found by 2 methods: {}",
            devices
                .iter()
                .filter(|d| d.detection_methods_count == 2)
                .count()
        );
        println!("  └─ Found by 1 method (UNIQUE): {}", unique);

        // Method coverage
        println!("\n🔍 Method Coverage:");
        println!(
            "  ├─ btleplug: {} devices",
            self.get_devices_by_method("btleplug").len()
        );
        println!(
            "  ├─ HCI Raw: {} devices",
            self.get_devices_by_method("hci_raw").len()
        );
        println!(
            "  ├─ Windows API: {} devices",
            self.get_devices_by_method("windows_api").len()
        );
        println!(
            "  ├─ HCI Realtime: {} devices",
            self.get_devices_by_method("hci_realtime").len()
        );
        println!(
            "  ├─ Vendor Protocols: {} devices",
            self.get_devices_by_method("vendor").len()
        );
        println!(
            "  ├─ Android Bridge: {} devices",
            self.get_devices_by_method("android").len()
        );
        println!(
            "  └─ CoreBluetooth: {} devices",
            self.get_devices_by_method("corebluetooth").len()
        );

        // Top devices
        println!("\n🏆 Top Devices (by confidence):");
        println!("{:<20} | {} | Count | RSSI", "MAC Address", "Confidence");
        println!("{}", "─".repeat(70));

        for device in devices.iter().take(15) {
            let confidence = "█".repeat(device.detection_methods_count)
                + &"░".repeat(7 - device.detection_methods_count);
            println!(
                "{:<20} | {} | {:4} | {:4}",
                device.mac_address, confidence, device.detection_count, device.rssi
            );
        }

        if devices.len() > 15 {
            println!("... and {} more devices", devices.len() - 15);
        }

        println!(
            "{}",
            "═══════════════════════════════════════════════════════════════".cyan()
        );
    }

    /// Stop scanning
    pub async fn stop_scan(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🛑 Stopping comprehensive scan");
        self.is_scanning = false;
        self.print_discovery_summary();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_device_creation() {
        let mut device = UnifiedDevice::new("AA:BB:CC:DD:EE:FF".to_string());
        device.detected_by_btleplug = true;
        device.detected_by_hci_raw = true;
        device.count_detection_methods();

        assert_eq!(device.detection_methods_count, 2);
        assert_eq!(device.detection_confidence(), 2);
    }

    #[test]
    fn test_scanner_creation() {
        let scanner = MultiMethodScanner::new();
        assert_eq!(scanner.get_devices_by_confidence().len(), 0);
    }
}
