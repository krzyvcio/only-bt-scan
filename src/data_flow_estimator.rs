use serde::{Deserialize, Serialize};
/// Data Flow Estimation Module
///
/// Estimates potential data transfer between Bluetooth devices based on:
/// - Advertising payload analysis
/// - Protocol pattern recognition (Meshtastic, Eddystone, iBeacon, Custom)
/// - Packet frequency and RSSI stability
/// - Connection state inference
///
/// NOTE: This is passive analysis of advertising packets only.
/// Real point-to-point transfers occur in encrypted GATT channels (not visible).
use std::collections::HashMap;

/// Known Bluetooth protocol types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolType {
    Meshtastic,
    Eddystone,
    IBeacon,
    AltBeacon,
    CybertrackTag,
    CustomRaw,
    Unknown,
}

/// Estimated transmission throughput and characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlowEstimate {
    pub source_mac: String,
    pub dest_mac: Option<String>, // Some if device-to-device communication detected
    pub estimated_bytes_per_sec: f64,
    pub avg_payload_size: u16,
    pub packet_frequency_hz: f64,
    pub reliability_estimate: f32, // 0.0 - 1.0 based on RSSI stability
    pub protocol_type: ProtocolType,
    pub last_packet_timestamp_ms: u64,
    pub sample_count: u32,
    pub confidence: f32, // 0.0 - 1.0
}

/// Per-device data flow analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDataFlow {
    pub mac_address: String,
    pub total_payload_bytes_observed: u64,
    pub packet_count: u32,
    pub average_packet_interval_ms: u64,
    pub detected_protocol: ProtocolType,
    pub protocol_confidence: f32,
    pub estimated_connection_state: ConnectionState,
    pub data_flow_pairs: Vec<DataFlowEstimate>, // Potential transfers to other devices
}

/// Inferred connection state based on packet patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    Advertising,      // Regular advertising (legacy/extended)
    DisconnectedIdle, // Sparse advertising
    Connected,        // Dense packet stream suggests active connection
    DataTransfer,     // High-frequency packets suggest data movement
    Unknown,
}

/// Main data flow analysis engine
pub struct DataFlowEstimator {
    // Timeline of packets per device
    device_packets: HashMap<String, Vec<PacketRecord>>,

    // Known protocol signatures [first_bytes] -> ProtocolType
    protocol_signatures: HashMap<Vec<u8>, ProtocolType>,

    // Cached flow estimates
    flow_cache: HashMap<String, DeviceDataFlow>,

    // Configuration
    config: EstimatorConfig,
}

#[derive(Debug, Clone)]
struct PacketRecord {
    timestamp_ms: u64,
    payload_size: u16,
    rssi: i8,
    raw_data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct EstimatorConfig {
    pub min_packet_interval_to_detect_connection_ms: u64,
    pub high_frequency_threshold_hz: f64,
    pub rssi_stability_window_ms: u64,
}

impl Default for EstimatorConfig {
    fn default() -> Self {
        Self {
            min_packet_interval_to_detect_connection_ms: 100,
            high_frequency_threshold_hz: 10.0, // >10 pkts/sec = likely connected
            rssi_stability_window_ms: 5000,
        }
    }
}

impl DataFlowEstimator {
    pub fn new() -> Self {
        let mut estimator = Self {
            device_packets: HashMap::new(),
            protocol_signatures: HashMap::new(),
            flow_cache: HashMap::new(),
            config: EstimatorConfig::default(),
        };

        estimator.register_protocol_signatures();
        estimator
    }

    /// Register known protocol signatures (packet header patterns)
    fn register_protocol_signatures(&mut self) {
        // Meshtastic: typically starts with 0x94 (encrypted packet type marker) or specific service UUID
        // Service UUID: 6ba1b218-15a8-461f-9fa8-5dcb12cf92d7
        self.protocol_signatures.insert(
            vec![0x94, 0xfe], // Meshtastic-like encryption marker
            ProtocolType::Meshtastic,
        );

        // Eddystone: AD type 0x16 (Service Data) with UUID 0xAAFE
        self.protocol_signatures.insert(
            vec![0x16, 0xfe, 0xaa], // Service Data - Eddystone
            ProtocolType::Eddystone,
        );

        // iBeacon: Manufacturer Data 0x004C (Apple) with specific pattern
        self.protocol_signatures.insert(
            vec![0xff, 0x4c, 0x00, 0x02, 0x15], // iBeacon prefix
            ProtocolType::IBeacon,
        );

        // AltBeacon: Manufacturer data with specific layout
        self.protocol_signatures.insert(
            vec![0xff, 0xac, 0xbe], // AltBeacon marker
            ProtocolType::AltBeacon,
        );

        // Cybertrack TAG: Custom protocol marker
        self.protocol_signatures.insert(
            vec![0x03, 0x01, 0xcb], // Example Cybertrack signature
            ProtocolType::CybertrackTag,
        );
    }

    /// Add a packet observation
    pub fn add_packet_observation(
        &mut self,
        mac_address: &str,
        timestamp_ms: u64,
        payload: &[u8],
        rssi: i8,
    ) {
        let device_key = mac_address.to_string();

        self.device_packets
            .entry(device_key)
            .or_insert_with(Vec::new)
            .push(PacketRecord {
                timestamp_ms,
                payload_size: payload.len() as u16,
                rssi,
                raw_data: payload.to_vec(),
            });

        // Invalidate cache for this device
        self.flow_cache.remove(mac_address);
    }

    /// Analyze data flow for a specific device
    pub fn analyze_device_flow(&mut self, mac_address: &str) -> Option<DeviceDataFlow> {
        // Return cached result if available
        if let Some(cached) = self.flow_cache.get(mac_address) {
            return Some(cached.clone());
        }

        let packets = self.device_packets.get(mac_address)?;
        if packets.is_empty() {
            return None;
        }

        let protocol = self.detect_protocol(packets);
        let protocol_confidence = self.calculate_protocol_confidence(packets, protocol);
        let connection_state = self.infer_connection_state(packets);

        // Calculate statistics
        let total_bytes: u64 = packets.iter().map(|p| p.payload_size as u64).sum();

        // Calculate packet intervals
        let intervals: Vec<u64> = packets
            .windows(2)
            .map(|w| w[1].timestamp_ms.saturating_sub(w[0].timestamp_ms))
            .collect();

        let avg_interval = if !intervals.is_empty() {
            intervals.iter().sum::<u64>() / intervals.len() as u64
        } else {
            0
        };

        // Detect device-to-device communication patterns
        let data_flow_pairs = self.detect_peer_communication(mac_address, packets);

        let flow = DeviceDataFlow {
            mac_address: mac_address.to_string(),
            total_payload_bytes_observed: total_bytes,
            packet_count: packets.len() as u32,
            average_packet_interval_ms: avg_interval,
            detected_protocol: protocol,
            protocol_confidence,
            estimated_connection_state: connection_state,
            data_flow_pairs,
        };

        self.flow_cache
            .insert(mac_address.to_string(), flow.clone());
        Some(flow)
    }

    /// Detect Bluetooth protocol type from packet signatures
    fn detect_protocol(&self, packets: &[PacketRecord]) -> ProtocolType {
        use std::cmp::Reverse;

        let mut type_scores: HashMap<ProtocolType, u32> = HashMap::new();

        for packet in packets {
            if packet.raw_data.len() < 2 {
                continue;
            }

            // Try exact signature matches (5, 3, 2 bytes)
            for sig_len in [5, 3, 2].iter() {
                if packet.raw_data.len() >= *sig_len {
                    let prefix = &packet.raw_data[..*sig_len];
                    if let Some(&proto) = self.protocol_signatures.get(prefix) {
                        *type_scores.entry(proto).or_insert(0) += 1;
                        break;
                    }
                }
            }
        }

        // Return most detected protocol, or Unknown
        type_scores
            .into_iter()
            .max_by_key(|(_, score)| Reverse(*score))
            .map(|(proto, _)| proto)
            .unwrap_or(ProtocolType::Unknown)
    }

    /// Calculate confidence score for detected protocol (0.0 - 1.0)
    fn calculate_protocol_confidence(
        &self,
        packets: &[PacketRecord],
        detected: ProtocolType,
    ) -> f32 {
        if detected == ProtocolType::Unknown {
            return 0.0;
        }

        // Count matching signatures
        let mut matches = 0u32;

        for packet in packets {
            for sig_len in [5, 3, 2] {
                if packet.raw_data.len() >= sig_len {
                    let prefix = &packet.raw_data[..sig_len];
                    if let Some(&proto) = self.protocol_signatures.get(prefix) {
                        if proto == detected {
                            matches += 1;
                            break;
                        }
                    }
                }
            }
        }

        // Confidence = % of packets matching this protocol
        matches as f32 / packets.len().max(1) as f32
    }

    /// Infer connection state from packet timing and frequency
    fn infer_connection_state(&self, packets: &[PacketRecord]) -> ConnectionState {
        if packets.len() < 2 {
            return ConnectionState::Advertising;
        }

        let intervals: Vec<u64> = packets
            .windows(2)
            .map(|w| w[1].timestamp_ms.saturating_sub(w[0].timestamp_ms))
            .collect();

        if intervals.is_empty() {
            return ConnectionState::Advertising;
        }

        // Calculate average frequency
        let avg_interval_ms = intervals.iter().sum::<u64>() / intervals.len() as u64;
        let freq_hz = if avg_interval_ms > 0 {
            1000.0 / avg_interval_ms as f64
        } else {
            0.0
        };

        // Classify based on frequency
        match freq_hz {
            f if f > self.config.high_frequency_threshold_hz => {
                // Very frequent packets suggest active data transfer
                if avg_interval_ms < 50 {
                    ConnectionState::DataTransfer
                } else {
                    ConnectionState::Connected
                }
            }
            f if f > 2.0 => {
                // Moderate frequency suggests connected state
                ConnectionState::Connected
            }
            _ => {
                // Low frequency = idle advertising
                ConnectionState::DisconnectedIdle
            }
        }
    }

    /// Detect potential device-to-device communication patterns
    fn detect_peer_communication(
        &self,
        mac_address: &str,
        packets: &[PacketRecord],
    ) -> Vec<DataFlowEstimate> {
        let mut estimates = Vec::new();

        // For each unique packet pattern, estimate if it's talking to another device
        // This is speculative based on packet structure analysis

        let total_bytes: u64 = packets.iter().map(|p| p.payload_size as u64).sum();
        let avg_payload = total_bytes as f32 / packets.len().max(1) as f32;

        // Calculate frequency
        let time_span_ms = if packets.len() > 1 {
            packets.last().unwrap().timestamp_ms - packets.first().unwrap().timestamp_ms
        } else {
            0
        };

        let freq_hz = if time_span_ms > 0 {
            (packets.len() as f64 * 1000.0) / time_span_ms as f64
        } else {
            0.0
        };

        // Calculate RSSI stability (lower variance = better signal)
        let rssi_values: Vec<i8> = packets.iter().map(|p| p.rssi).collect();
        let rssi_variance = calculate_variance(&rssi_values);
        let stability = (50.0 - rssi_variance.min(50.0)) / 50.0; // Normalize to 0.0-1.0

        let throughput = avg_payload * freq_hz as f32;

        estimates.push(DataFlowEstimate {
            source_mac: mac_address.to_string(),
            dest_mac: None, // Would require correlation analysis
            estimated_bytes_per_sec: throughput as f64,
            avg_payload_size: avg_payload as u16,
            packet_frequency_hz: freq_hz,
            reliability_estimate: stability as f32,
            protocol_type: ProtocolType::CustomRaw,
            last_packet_timestamp_ms: packets.last().map(|p| p.timestamp_ms).unwrap_or(0),
            sample_count: packets.len() as u32,
            confidence: 0.6, // Will be updated after cross-device analysis
        });

        estimates
    }

    /// Generate summary statistics
    pub fn generate_summary(&self) -> DataFlowSummary {
        let mut total_bytes = 0u64;
        let mut device_count = 0;
        let mut busiest_device = String::new();
        let mut max_bytes = 0u64;

        for (mac, packets) in &self.device_packets {
            let bytes: u64 = packets.iter().map(|p| p.payload_size as u64).sum();
            total_bytes += bytes;
            device_count += 1;

            if bytes > max_bytes {
                max_bytes = bytes;
                busiest_device = mac.clone();
            }
        }

        DataFlowSummary {
            total_devices_observed: device_count,
            total_payload_bytes_observed: total_bytes,
            busiest_device,
            busiest_device_bytes: max_bytes,
            average_bytes_per_device: if device_count > 0 {
                total_bytes / device_count as u64
            } else {
                0
            },
        }
    }

    /// Export all estimates as JSON
    pub fn export_estimates(&mut self) -> Result<String, serde_json::Error> {
        let mut estimates = Vec::new();

        for mac in self.device_packets.keys().cloned().collect::<Vec<_>>() {
            if let Some(flow) = self.analyze_device_flow(&mac) {
                estimates.push(flow);
            }
        }

        serde_json::to_string_pretty(&estimates)
    }
}

/// Summary statistics for all observed data flows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlowSummary {
    pub total_devices_observed: usize,
    pub total_payload_bytes_observed: u64,
    pub busiest_device: String,
    pub busiest_device_bytes: u64,
    pub average_bytes_per_device: u64,
}

/// Helper: calculate variance of a numeric slice
fn calculate_variance(values: &[i8]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }

    let mean = values.iter().map(|&v| v as f32).sum::<f32>() / values.len() as f32;
    let variance = values
        .iter()
        .map(|&v| {
            let diff = v as f32 - mean;
            diff * diff
        })
        .sum::<f32>()
        / values.len() as f32;

    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_detection() {
        let mut estimator = DataFlowEstimator::new();

        // Simulate Eddystone packets
        let eddystone_packet = vec![0x16, 0xfe, 0xaa, 0x10, 0xec, 0x00, 0x3c];
        estimator.add_packet_observation("AA:BB:CC:DD:EE:FF", 1000, &eddystone_packet, -50);

        if let Some(flow) = estimator.analyze_device_flow("AA:BB:CC:DD:EE:FF") {
            assert_eq!(flow.detected_protocol, ProtocolType::Eddystone);
        }
    }

    #[test]
    fn test_connection_state_inference() {
        let mut estimator = DataFlowEstimator::new();

        // Add dense packets (high frequency)
        for i in 0..20 {
            let packet = vec![0x02, 0x01, 0x06]; // BLE flags
            estimator.add_packet_observation(
                "AA:BB:CC:DD:EE:FF",
                (i * 50) as u64, // 50ms intervals = 20Hz
                &packet,
                -60,
            );
        }

        if let Some(flow) = estimator.analyze_device_flow("AA:BB:CC:DD:EE:FF") {
            assert_eq!(
                flow.estimated_connection_state,
                ConnectionState::DataTransfer
            );
        }
    }
}
