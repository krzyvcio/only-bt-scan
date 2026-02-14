/// Scanner Integration - Bridges BluetoothDevice to PacketTracker
///
/// Adapts BluetoothScanner results for packet ordering and temporal analysis

use crate::bluetooth_scanner::BluetoothDevice;
use crate::data_models::RawPacketModel;
use crate::packet_tracker::{GlobalPacketTracker, PacketAddResult};
use crate::telemetry::TelemetryCollector;
use chrono::Utc;

/// Wrapper for unified scanning + tracking
pub struct ScannerWithTracking {
    pub packet_tracker: GlobalPacketTracker,
    pub telemetry_collector: TelemetryCollector,
}

impl ScannerWithTracking {
    pub fn new() -> Self {
        Self {
            packet_tracker: GlobalPacketTracker::new(),
            telemetry_collector: TelemetryCollector::new(),
        }
    }

    /// Process Bluetooth devices from scan and add to tracker
    pub fn process_scan_results(&mut self, devices: Vec<BluetoothDevice>) {
        let mut packet_counter = self.packet_tracker.packet_count;

        for device in devices {
            // Convert BluetoothDevice to RawPacketModel
            let mut packet = create_raw_packet_from_device(&device, packet_counter);
            packet_counter = packet_counter.wrapping_add(1);

            // Add to global tracker
            let result = self.packet_tracker.add_packet(packet.clone());

            // Record in telemetry
            self.telemetry_collector
                .record_packet_result(&result, packet.rssi);

            match result {
                PacketAddResult::Accepted { packet_id, .. } => {
                    log::debug!(
                        "✓ Packet {} from {} added to sequence",
                        packet_id,
                        device.mac_address
                    );
                }
                PacketAddResult::Rejected { reason, .. } => {
                    log::debug!(
                        "✗ Packet from {} rejected: {}",
                        device.mac_address, reason
                    );
                }
            }
        }
    }

    /// Get global packet ordering
    pub fn get_packet_ordering(&self) -> Vec<(String, u64, u64)> {
        self.packet_tracker.get_global_sequence()
    }

    /// Get device packet sequence
    pub fn get_device_sequence(&self, mac: &str) -> Option<Vec<u64>> {
        self.packet_tracker.get_device_sequence(mac)
    }

    /// Get tracking statistics
    pub fn get_stats(&self) -> ScannerTrackingStats {
        let global_stats = self.packet_tracker.get_global_stats();

        let mut device_sequences = std::collections::HashMap::new();
        for (mac, tracker) in &self.packet_tracker.device_trackers {
            device_sequences.insert(mac.clone(), tracker.packet_sequence.len());
        }

        ScannerTrackingStats {
            unique_devices: global_stats.unique_devices,
            total_packets_received: global_stats.total_packets_received,
            total_packets_tracked: global_stats.total_packets_accepted,
            acceptance_rate_percent: global_stats.acceptance_rate,
            total_filtered: global_stats.total_filtered,
            total_duplicates: global_stats.total_duplicates,
            device_sequence_lengths: device_sequences,
        }
    }

    /// Export telemetry
    pub fn export_telemetry(&self) -> String {
        let telemetry = self
            .telemetry_collector
            .generate_global_telemetry(&self.packet_tracker);

        match crate::telemetry::telemetry_to_json(&telemetry) {
            Ok(json) => json,
            Err(e) => {
                log::error!("Failed to serialize telemetry: {}", e);
                "{}".to_string()
            }
        }
    }

    /// Export device-specific telemetry
    pub fn export_device_telemetry(&self, mac: &str) -> Option<String> {
        let telemetry = self
            .telemetry_collector
            .generate_device_telemetry(&self.packet_tracker, mac)?;

        match crate::telemetry::device_telemetry_to_json(&telemetry) {
            Ok(json) => Some(json),
            Err(e) => {
                log::error!("Failed to serialize device telemetry: {}", e);
                None
            }
        }
    }
}

/// Convert BluetoothDevice to RawPacketModel for packet tracking
fn create_raw_packet_from_device(device: &BluetoothDevice, packet_id: u64) -> RawPacketModel {
    // Use last_detected_ns as timestamp
    let timestamp = Utc::now();
    let timestamp_ms = (device.last_detected_ns / 1_000_000) as u64;

    let mut packet = RawPacketModel {
        packet_id,
        mac_address: device.mac_address.clone(),
        timestamp,
        timestamp_ms,
        phy: "LE 1M".to_string(),
        channel: 37, // Default advertising channel
        rssi: device.rssi,
        packet_type: match device.device_type {
            crate::bluetooth_scanner::DeviceType::DualMode => "ADV_IND".to_string(),
            _ => "ADV_IND".to_string(),
        },
        is_scan_response: false,
        is_extended: false,
        advertising_data: Vec::new(),
        advertising_data_hex: String::new(),
        ad_structures: Vec::new(),
        flags: None,
        local_name: device.name.clone(),
        short_name: device.name.clone(),
        advertised_services: device
            .services
            .iter()
            .filter_map(|s| s.uuid128.clone())
            .collect(),
        manufacturer_data: {
            let mut map = std::collections::HashMap::new();
            if let Some(mfg_id) = device.manufacturer_id {
                map.insert(mfg_id, Vec::new());
            }
            map
        },
        service_data: std::collections::HashMap::new(),
        total_length: 0,
        parsed_successfully: true,
    };

    packet
}

/// Statistics from scanner with tracking
#[derive(Debug, Clone)]
pub struct ScannerTrackingStats {
    pub unique_devices: usize,
    pub total_packets_received: u64,
    pub total_packets_tracked: u64,
    pub acceptance_rate_percent: f64,
    pub total_filtered: u64,
    pub total_duplicates: u64,
    pub device_sequence_lengths: std::collections::HashMap<String, usize>,
}

impl std::fmt::Display for ScannerTrackingStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "📊 SCANNING STATS:\n  Devices: {}\n  Packets Received: {}\n  Packets Tracked: {}\n  Acceptance Rate: {:.1}%\n  Filtered: {}\n  Duplicates: {}",
            self.unique_devices,
            self.total_packets_received,
            self.total_packets_tracked,
            self.acceptance_rate_percent,
            self.total_filtered,
            self.total_duplicates
        )
    }
}
