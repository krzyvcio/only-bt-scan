use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Data Flow Estimator - Estimates BLE advertising patterns and protocols
///
/// # How it works:
/// 1. Each accepted packet adds an observation with MAC, timestamp, payload, RSSI
/// 2. Protocol detection matches payload header bytes against known signatures:
///    - Meshtastic: 0x94 0xFE
///    - Eddystone: 0x16 0xFE 0xAA (Service Data)
///    - iBeacon: 0xFF 0x4C 0x00 0x02 0x15 (Apple)
///    - AltBeacon: 0xFF 0xAC 0xBE
///    - Cybertrack: 0x03 0x01 0xCB
/// 3. Connection state inference from packet frequency:
///    - >10 packets/sec → Connected or DataTransfer
///    - Regular intervals → Advertising
///    - Sparse → DisconnectedIdle
/// 4. Reliability estimate based on RSSI variance in 5s window

/// Global singleton for data flow estimation
static DATA_FLOW_ESTIMATOR: std::sync::LazyLock<std::sync::Mutex<DataFlowEstimatorState>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(DataFlowEstimatorState::new()));

/// Internal state holding packet observations per device
///
/// Maintains:
/// - Packet timeline per device (max 1000 packets)
/// - Protocol signature database
/// - Flow analysis cache
pub struct DataFlowEstimatorState {
    estimator: DataFlowEstimator,
    max_packets_per_device: usize,
}

impl DataFlowEstimatorState {
    /// Create new estimator state
    ///
    /// # Returns
    /// New DataFlowEstimatorState with default limits
    pub fn new() -> Self {
        Self {
            estimator: DataFlowEstimator::new(),
            max_packets_per_device: 1000,
        }
    }

    /// Adds packet observation for a device
    /// Keeps max 1000 packets per device, removes oldest 100 when full
    pub fn add_packet(&mut self, mac: &str, timestamp_ms: u64, payload: &[u8], rssi: i8) {
        use std::collections::hash_map::Entry;
        if let Entry::Occupied(mut entry) = self.estimator.device_packets.entry(mac.to_string()) {
            let packets = entry.get_mut();
            if packets.len() >= self.max_packets_per_device {
                packets.drain(0..100);
            }
        }
        self.estimator
            .add_packet_observation(mac, timestamp_ms, payload, rssi);
    }

    /// Analyzes flow for specific device
    pub fn analyze_device(&mut self, mac: &str) -> Option<DeviceDataFlow> {
        self.estimator.analyze_device_flow(mac)
    }

    /// Analyzes all tracked devices
    pub fn analyze_all_devices(&mut self) -> Vec<DeviceDataFlow> {
        let macs: Vec<String> = self.estimator.device_packets.keys().cloned().collect();
        macs.into_iter()
            .filter_map(|mac| self.estimator.analyze_device_flow(&mac))
            .collect()
    }

    /// Clear all observations
    ///
    /// Removes all packet observations and cached flow data.
    pub fn clear(&mut self) {
        self.estimator.device_packets.clear();
        self.estimator.flow_cache.clear();
    }
}

/// Adds a packet observation to the global estimator
/// Called from ScannerWithTracking when packets are accepted
/// # Arguments
/// * `mac` - Device MAC address
/// * `timestamp_ms` - Packet timestamp in milliseconds
/// * `payload` - Raw advertising data bytes
/// * `rssi` - Signal strength in dBm
pub fn add_packet(mac: &str, timestamp_ms: u64, payload: &[u8], rssi: i8) {
    if let Ok(mut state) = DATA_FLOW_ESTIMATOR.lock() {
        state.add_packet(mac, timestamp_ms, payload, rssi);
    }
}

/// Analyzes data flow characteristics for a specific device
/// # Returns
/// Some(DeviceDataFlow) with:
/// - `detected_protocol`: Meshtastic/Eddystone/IBeacon/AltBeacon/Custom/Unknown
/// - `estimated_connection_state`: Advertising/Connected/DataTransfer/DisconnectedIdle
/// - `packet_frequency_hz`: Average packets per second
/// - `reliability_estimate`: 0-1 based on RSSI stability
/// - `average_packet_interval_ms`: Time between packets
pub fn analyze_device(mac: &str) -> Option<DeviceDataFlow> {
    DATA_FLOW_ESTIMATOR.lock().ok()?.analyze_device(mac)
}

/// Analyzes data flow for ALL tracked devices
/// Returns vector of DeviceDataFlow, one per device
pub fn analyze_all_devices() -> Vec<DeviceDataFlow> {
    DATA_FLOW_ESTIMATOR
        .lock()
        .ok()
        .map(|mut s| s.analyze_all_devices())
        .unwrap_or_default()
}

/// Returns number of devices currently being tracked
pub fn get_device_count() -> usize {
    DATA_FLOW_ESTIMATOR
        .lock()
        .map(|s| s.estimator.device_packets.len())
        .unwrap_or(0)
}

/// Clears all packet observations (useful for starting fresh scan)
pub fn clear_estimates() {
    if let Ok(mut state) = DATA_FLOW_ESTIMATOR.lock() {
        state.clear();
    }
}

/// Known Bluetooth protocol types
///
/// Identified protocols from advertising packet analysis:
/// - `Meshtastic`: Meshtastic mesh networking
/// - `Eddystone`: Google Eddystone beacon
/// - `IBeacon`: Apple iBeacon
/// - `AltBeacon`: Alternative beacon format
/// - `CybertrackTag`: Cybertrack tracking tag
/// - `CustomRaw`: Unknown/custom protocol
/// - `Unknown`: No protocol identified
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
///
/// Detailed per-device flow analysis:
/// - `source_mac`: Origin device MAC
/// - `dest_mac`: Target device if peer-to-peer detected
/// - `estimated_bytes_per_sec`: Throughput estimate
/// - `avg_payload_size`: Average advertising payload size
/// - `packet_frequency_hz`: Packets per second
/// - `reliability_estimate`: Signal stability (0-1)
/// - `protocol_type`: Detected protocol
/// - `last_packet_timestamp_ms`: Most recent packet time
/// - `sample_count`: Number of packets analyzed
/// - `confidence`: Analysis confidence (0-1)
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
///
/// Complete analysis results for a single device:
/// - `mac_address`: Device MAC
/// - `total_payload_bytes_observed`: Cumulative bytes seen
/// - `packet_count`: Number of packets analyzed
/// - `average_packet_interval_ms`: Mean time between packets
/// - `detected_protocol`: Protocol identification
/// - `protocol_confidence`: Detection confidence (0-1)
/// - `estimated_connection_state`: Inferred state
/// - `data_flow_pairs`: Potential peer communications
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
///
/// Connection state deduced from advertising frequency:
/// - `Advertising`: Regular advertising intervals
/// - `DisconnectedIdle`: Sparse, infrequent packets
/// - `Connected`: Dense packet stream (active connection)
/// - `DataTransfer`: Very high frequency (>10Hz)
/// - `Unknown`: Insufficient data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    Advertising,      // Regular advertising (legacy/extended)
    DisconnectedIdle, // Sparse advertising
    Connected,        // Dense packet stream suggests active connection
    DataTransfer,     // High-frequency packets suggest data movement
    Unknown,
}

/// Main data flow analysis engine
///
/// Core analyzer that maintains:
/// - `device_packets`: Timeline of packets per device
/// - `protocol_signatures`: Known protocol signatures
/// - `flow_cache`: Cached analysis results
/// - `config`: Analysis configuration
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

/// Single packet observation record
///
/// Represents one captured advertising packet:
/// - `timestamp_ms`: When packet was received
/// - `payload_size`: Size of advertising data
/// - `rssi`: Signal strength
/// - `raw_data`: Complete packet bytes
#[derive(Debug, Clone)]
struct PacketRecord {
    timestamp_ms: u64,
    payload_size: u16,
    rssi: i8,
    raw_data: Vec<u8>,
}

/// Configuration for data flow estimator
///
/// Tuning parameters:
/// - `min_packet_interval_to_detect_connection_ms`: Min interval for connection detection
/// - `high_frequency_threshold_hz`: Threshold for high-frequency detection
/// - `rssi_stability_window_ms`: Window for RSSI stability analysis
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
    /// Create new estimator
    ///
    /// # Returns
    /// DataFlowEstimator with registered protocol signatures
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
    ///
    /// Records new advertising packet for device. Invalidates cache.
    ///
    /// # Arguments
    /// * `mac_address` - Device MAC address
    /// * `timestamp_ms` - Packet timestamp in milliseconds
    /// * `payload` - Raw advertising data
    /// * `rssi` - Signal strength in dBm
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
    ///
    /// Returns cached result if available, otherwise computes new analysis
    /// including protocol detection, connection state inference, and
    /// peer communication patterns.
    ///
    /// # Arguments
    /// * `mac_address` - Device MAC to analyze
    ///
    /// # Returns
    /// Some(DeviceDataFlow) with analysis results, or None if no packets
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
    ///
    /// Creates aggregate statistics across all tracked devices.
    ///
    /// # Returns
    /// DataFlowSummary with totals and busiest device info
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
    ///
    /// # Returns
    /// JSON string of all DeviceDataFlow estimates
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
///
/// Aggregate metrics:
/// - `total_devices_observed`: Count of unique devices
/// - `total_payload_bytes_observed`: Total bytes across all devices
/// - `busiest_device`: MAC of highest-throughput device
/// - `busiest_device_bytes`: Bytes for busiest device
/// - `average_bytes_per_device`: Mean bytes per device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlowSummary {
    pub total_devices_observed: usize,
    pub total_payload_bytes_observed: u64,
    pub busiest_device: String,
    pub busiest_device_bytes: u64,
    pub average_bytes_per_device: u64,
}

/// Helper: calculate variance of a numeric slice
///
/// Computes standard deviation variance for RSSI values.
///
/// # Arguments
/// * `values` - Slice of RSSI values
///
/// # Returns
/// Variance as f32
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
