use chrono::{DateTime, Utc};

/// Core Data Models - Two fundamental data types
///
/// This module defines the two main data models in the system:
/// 1. DEVICE MODEL - High-level aggregated device information
/// 2. RAW PACKET MODEL - Low-level raw Bluetooth packet data
///
/// These models work together:
/// - Devices aggregate information from many raw packets
/// - Raw packets provide the detailed telemetry data
/// - Web API serves both independently and combined
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════════
// MODEL 1: DEVICE DATA - High-level aggregated device information
// ═══════════════════════════════════════════════════════════════════════════════

/// Complete device representation - aggregated from many packets.
///
/// This struct represents a high-level view of a Bluetooth device,
/// aggregating information from multiple raw advertisement packets.
///
/// # Fields
/// - `mac_address` - Unique MAC address (primary identifier)
/// - `device_name` - Friendly name from advertising data or GATT
/// - `device_type` - BLE, BR/EDR, or Dual Mode
/// - `rssi` - Most recent signal strength in dBm
/// - `avg_rssi` - Rolling average RSSI
/// - `rssi_variance` - Signal stability measure
/// - `first_seen` - Timestamp of first detection
/// - `last_seen` - Timestamp of most recent detection
/// - `response_time_ms` - Time gap between first and last detection
/// - `manufacturer_id` - Bluetooth SIG company identifier
/// - `manufacturer_name` - Human-readable company name
/// - `advertised_services` - List of advertised service UUIDs
/// - `appearance` - Device appearance category
/// - `tx_power` - Transmit power level from advertising
/// - `mac_type` - Public, Random, or Resolvable Private Address
/// - `is_rpa` - Flag for Random Private Address
/// - `security_level` - Security level if discovered
/// - `pairing_method` - Pairing method if discovered
/// - `is_connectable` - Whether device accepts connections
/// - `detection_count` - Total number of scan detections
/// - `last_rssi_values` - Recent RSSI values for charting
/// - `discovered_services` - GATT services (if connected)
/// - `vendor_protocols` - Vendor-specific protocols (iBeacon, Eddystone, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceModel {
    // === Core Identification ===
    pub mac_address: String,         // Primary key
    pub device_name: Option<String>, // Friendly name
    pub device_type: DeviceType,

    // === Signal Quality ===
    pub rssi: i8,           // Current signal strength
    pub avg_rssi: f64,      // Average over time
    pub rssi_variance: f64, // Signal stability

    // === Timing ===
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub response_time_ms: u64, // First to last detection gap

    // === Advertising Info ===
    pub manufacturer_id: Option<u16>,
    pub manufacturer_name: Option<String>,
    pub advertised_services: Vec<String>,
    pub appearance: Option<u16>,
    pub tx_power: Option<i8>,

    // === MAC Addressing ===
    pub mac_type: Option<String>, // Public, Random, RPA
    pub is_rpa: bool,             // Random Private Address flag

    // === Security & Connection ===
    pub security_level: Option<String>,
    pub pairing_method: Option<String>,
    pub is_connectable: bool,

    // === Statistics ===
    pub detection_count: u64,      // Total times scanned
    pub last_rssi_values: Vec<i8>, // Last N RSSI values for charts

    // === GATT Discovery ===
    pub discovered_services: Vec<GattServiceInfo>,

    // === Vendor Protocols ===
    pub vendor_protocols: Vec<VendorProtocolInfo>,
}

/// Bluetooth device type classification.
///
/// Based on Bluetooth Core Specification:
/// - BleOnly - LE (Low Energy) only
/// - BrEdrOnly - Classic Bluetooth only  
/// - DualModeBle - Dual mode, LE preferred
/// - DualModeBrEdr - Dual mode, BR/EDR preferred
/// - Unknown - Unable to determine
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DeviceType {
    BleOnly,
    BrEdrOnly,
    DualModeBle,
    DualModeBrEdr,
    Unknown,
}

/// GATT Service information discovered from a connectable device.
///
/// # Fields
/// - `uuid` - Service UUID (16-bit or 128-bit)
/// - `name` - Human-readable service name (if known)
/// - `characteristics_count` - Number of characteristics in this service
/// - `readable` - Whether service has readable characteristics
/// - `writable` - Whether service has writable characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GattServiceInfo {
    pub uuid: String,
    pub name: Option<String>,
    pub characteristics_count: usize,
    pub readable: bool,
    pub writable: bool,
}

/// Vendor-specific protocol information extracted from advertising data.
///
/// # Fields
/// - `protocol_name` - Protocol name (e.g., "iBeacon", "Eddystone", "Fast Pair")
/// - `protocol_type` - Protocol category (e.g., "beacon", "continuity", "fast_pair")
/// - `data` - Protocol-specific data as key-value pairs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorProtocolInfo {
    pub protocol_name: String, // e.g., "iBeacon", "Eddystone", "Fast Pair"
    pub protocol_type: String, // e.g., "beacon", "continuity", "fast_pair"
    pub data: HashMap<String, String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// MODEL 2: RAW PACKET DATA - Low-level raw Bluetooth packet information
// ═══════════════════════════════════════════════════════════════════════════════

/// Complete raw Bluetooth packet - as captured from the air.
///
/// Represents a single BLE advertisement packet with all metadata.
///
/// # Fields
/// - `packet_id` - Unique sequential ID
/// - `mac_address` - Device MAC address
/// - `timestamp` - Full timestamp with nanosecond precision
/// - `timestamp_ms` - Milliseconds since epoch (for temporal analysis)
/// - `latency_from_previous_ms` - Time since previous packet from same device
/// - `phy` - Physical layer (LE 1M, LE 2M, LE Coded)
/// - `channel` - BLE channel (37-39 for advertising, 0-36 for data)
/// - `rssi` - Signal strength in dBm
/// - `packet_type` - Advertisement type (ADV_IND, SCAN_RSP, etc.)
/// - `is_scan_response` - Whether this is a scan response
/// - `is_extended` - BT 5.0+ extended advertising
/// - `advertising_data` - Raw bytes from advertising data
/// - `advertising_data_hex` - Hex string representation
/// - `ad_structures` - Parsed AD structures
/// - `flags` - Parsed advertising flags
/// - `local_name` - Complete local name
/// - `short_name` - Shortened local name
/// - `advertised_services` - List of service UUIDs
/// - `manufacturer_data` - Manufacturer-specific data by company ID
/// - `service_data` - Service data by service UUID
/// - `total_length` - Total packet length
/// - `parsed_successfully` - Whether parsing succeeded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPacketModel {
    // === Packet Identification ===
    pub packet_id: u64,      // Unique ID
    pub mac_address: String, // Which device sent this
    pub timestamp: DateTime<Utc>,
    pub timestamp_ms: u64, // Milliseconds since epoch (for temporal analysis)
    pub latency_from_previous_ms: Option<u64>, // Time from previous packet for this device (real-time calculated)

    // === Physical Layer ===
    pub phy: String, // "LE 1M", "LE 2M", "LE Coded"
    pub channel: u8, // BLE: 37-39 adv, 0-36 data
    pub rssi: i8,    // Signal strength in dBm

    // === Packet Structure ===
    pub packet_type: String, // "ADV_IND", "SCAN_RSP", "ADV_NONCONN_IND"
    pub is_scan_response: bool,
    pub is_extended: bool, // BT 5.0+ extended advertising
    pub address_type: Option<String>, // "Public", "Random", "RPA", etc.

    // === Raw Advertising Data ===
    pub advertising_data: Vec<u8>,    // Complete raw bytes
    pub advertising_data_hex: String, // Hex string representation

    // === Parsed AD Structures ===
    pub ad_structures: Vec<AdStructureData>,

    // === Flags ===
    pub flags: Option<AdvertisingFlags>,
    pub local_name: Option<String>,
    pub short_name: Option<String>,
    pub advertised_services: Vec<String>,
    pub manufacturer_data: HashMap<u16, Vec<u8>>,
    pub service_data: HashMap<String, Vec<u8>>,

    // === Statistics ===
    pub total_length: usize,
    pub parsed_successfully: bool,
}

/// Single AD Structure from advertising data.
///
/// BLE advertising data is composed of AD Structures, each with:
/// - Length (1 byte)
/// - AD Type (1 byte)
/// - AD Data (length-1 bytes)
///
/// # Fields
/// - `ad_type` - AD type byte (e.g., 0xFF for Manufacturer Data)
/// - `type_name` - Human-readable AD type name
/// - `data` - Raw data bytes
/// - `data_hex` - Hex string representation
/// - `interpretation` - Human-readable meaning of the data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdStructureData {
    pub ad_type: u8,
    pub type_name: String,
    pub data: Vec<u8>,
    pub data_hex: String,
    pub interpretation: String, // Human-readable meaning
}

/// Advertising Flags from AD Type 0x01.
///
/// Standard Bluetooth LE flags indicating:
/// - Discovery mode (limited/general)
/// - BR/EDR support
/// - Dual mode controller/host capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvertisingFlags {
    pub le_limited_discoverable: bool,
    pub le_general_discoverable: bool,
    pub br_edr_not_supported: bool,
    pub simultaneous_le_and_br_edr_controller: bool,
    pub simultaneous_le_and_br_edr_host: bool,
}

// ═══════════════════════════════════════════════════════════════════════════════
// RELATIONSHIP MODEL - Connecting the two
// ═══════════════════════════════════════════════════════════════════════════════

/// Links device to its packets - relationship model for API responses.
///
/// Provides aggregated statistics about a device's packet history.
///
/// # Fields
/// - `mac_address` - Device MAC address
/// - `total_packets` - Total packet count
/// - `packets_by_channel` - Packet count per channel
/// - `packets_by_type` - Packet count per advertisement type
/// - `packets_by_phy` - Packet count per physical layer
/// - `last_packet_ids` - Recent packet IDs for quick lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePacketRelationship {
    pub mac_address: String,
    pub total_packets: u64,
    pub packets_by_channel: HashMap<u8, u64>,
    pub packets_by_type: HashMap<String, u64>,
    pub packets_by_phy: HashMap<String, u64>,
    pub last_packet_ids: Vec<u64>, // Last 100 packet IDs for quick lookup
}

// ═══════════════════════════════════════════════════════════════════════════════
// COMBINED API RESPONSES
// ═══════════════════════════════════════════════════════════════════════════════

/// Device with its recent packets - combined API response.
///
/// # Fields
/// - `device` - The device model
/// - `recent_packets` - Recent raw packets from this device
/// - `packet_count` - Total packets for this device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceWithPackets {
    pub device: DeviceModel,
    pub recent_packets: Vec<RawPacketModel>,
    pub packet_count: u64,
}

/// Paginated response for devices listing.
///
/// # Fields
/// - `devices` - Array of device models for current page
/// - `total` - Total count of all devices
/// - `page` - Current page number (1-indexed)
/// - `page_size` - Number of items per page
/// - `total_pages` - Total number of pages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedDevices {
    pub devices: Vec<DeviceModel>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// Paginated response for raw packets listing.
///
/// # Fields
/// - `packets` - Array of packet models for current page
/// - `total` - Total count of all packets
/// - `page` - Current page number (1-indexed)
/// - `page_size` - Number of items per page
/// - `total_pages` - Total number of pages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedPackets {
    pub packets: Vec<RawPacketModel>,
    pub total: u64,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// Combined scan results - all data from a single scan cycle.
///
/// # Fields
/// - `scan_timestamp` - When the scan started
/// - `total_devices` - Number of unique devices found
/// - `total_packets` - Total raw packets captured
/// - `devices` - List of all discovered devices
/// - `sample_packets` - Sample of raw packets for detail view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResultsModel {
    pub scan_timestamp: DateTime<Utc>,
    pub total_devices: usize,
    pub total_packets: u64,
    pub devices: Vec<DeviceModel>,
    pub sample_packets: Vec<RawPacketModel>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// DATABASE SCHEMA MAPPING
// ═══════════════════════════════════════════════════════════════════════════════

/// Maps to 'devices' table in SQLite database.
///
/// Internal struct for database row mapping.
///
/// # Fields
/// - `id` - Primary key
/// - `mac_address` - Unique MAC address
/// - `device_name` - Device name (optional)
/// - `rssi` - Current RSSI value
/// - `first_seen` - First detection timestamp
/// - `last_seen` - Last detection timestamp
/// - `manufacturer_id` - Bluetooth company ID
/// - `manufacturer_name` - Company name
/// - `device_type` - Device type string
/// - `number_of_scan` - Number of scan cycles detected in
/// - `mac_type` - MAC address type
/// - `is_rpa` - Is Random Private Address
/// - `security_level` - Security level
/// - `pairing_method` - Pairing method used
#[derive(Debug, Clone)]
pub struct DeviceRow {
    pub id: i32,
    pub mac_address: String,
    pub device_name: Option<String>,
    pub rssi: i8,
    pub first_seen: String,
    pub last_seen: String,
    pub manufacturer_id: Option<u16>,
    pub manufacturer_name: Option<String>,
    pub device_type: String,
    pub number_of_scan: i32,
    pub mac_type: Option<String>,
    pub is_rpa: bool,
    pub security_level: Option<String>,
    pub pairing_method: Option<String>,
}

/// Maps to 'ble_advertisement_frames' table in SQLite database.
///
/// Internal struct for database row mapping.
///
/// # Fields
/// - `id` - Primary key
/// - `device_id` - Foreign key to devices table
/// - `mac_address` - Device MAC address
/// - `rssi` - Signal strength
/// - `advertising_data` - Raw bytes (BLOB in DB)
/// - `phy` - Physical layer
/// - `channel` - BLE channel
/// - `frame_type` - Advertisement type
/// - `timestamp` - Timestamp string
#[derive(Debug, Clone)]
pub struct PacketRow {
    pub id: i64,
    pub device_id: i32,
    pub mac_address: String,
    pub rssi: i8,
    pub advertising_data: Vec<u8>, // BLOB in DB
    pub phy: String,
    pub channel: i32,
    pub frame_type: String,
    pub timestamp: String,
    pub address_type: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// CONVERSION FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

impl DeviceModel {
    /// Creates a new DeviceModel with default values.
    ///
    /// # Arguments
    /// * `mac_address` - The MAC address of the device
    ///
    /// # Returns
    /// DeviceModel - New instance with default/empty values
    pub fn new(mac_address: String) -> Self {
        let now = Utc::now();
        Self {
            mac_address,
            device_name: None,
            device_type: DeviceType::Unknown,
            rssi: -100,
            avg_rssi: -100.0,
            rssi_variance: 0.0,
            first_seen: now,
            last_seen: now,
            response_time_ms: 0,
            manufacturer_id: None,
            manufacturer_name: None,
            advertised_services: Vec::new(),
            appearance: None,
            tx_power: None,
            mac_type: None,
            is_rpa: false,
            security_level: None,
            pairing_method: None,
            is_connectable: true,
            detection_count: 0,
            last_rssi_values: Vec::new(),
            discovered_services: Vec::new(),
            vendor_protocols: Vec::new(),
        }
    }

    /// Adds an RSSI measurement and updates statistics.
    ///
    /// Updates current RSSI, maintains rolling average (last 100 values),
    /// and calculates signal variance for stability analysis.
    ///
    /// # Arguments
    /// * `rssi` - New RSSI value in dBm
    pub fn add_rssi(&mut self, rssi: i8) {
        self.rssi = rssi;
        self.last_rssi_values.push(rssi);

        // Keep only last 100 values
        if self.last_rssi_values.len() > 100 {
            self.last_rssi_values.remove(0);
        }

        // Update average
        let sum: i32 = self.last_rssi_values.iter().map(|&r| r as i32).sum();
        self.avg_rssi = (sum as f64) / (self.last_rssi_values.len() as f64);

        // Update variance
        let avg_int = self.avg_rssi as i32;
        let var_sum: i64 = self
            .last_rssi_values
            .iter()
            .map(|&r| {
                let diff = (r as i32) - avg_int;
                (diff * diff) as i64
            })
            .sum();
        self.rssi_variance = ((var_sum as f64) / (self.last_rssi_values.len() as f64)).sqrt();
    }
}

impl RawPacketModel {
    /// Creates a new RawPacketModel with default values.
    ///
    /// # Arguments
    /// * `mac_address` - The MAC address of the sending device
    /// * `timestamp` - Packet timestamp
    /// * `advertising_data` - Raw advertising data bytes
    ///
    /// # Returns
    /// RawPacketModel - New instance with calculated fields
    pub fn new(mac_address: String, timestamp: DateTime<Utc>, advertising_data: Vec<u8>) -> Self {
        let advertising_data_hex = hex::encode(&advertising_data);
        let timestamp_ms =
            (timestamp.timestamp() as u64) * 1000 + (timestamp.timestamp_subsec_millis() as u64);

        Self {
            packet_id: 0,
            mac_address,
            timestamp,
            timestamp_ms,
            latency_from_previous_ms: None,
            phy: "LE 1M".to_string(),
            channel: 37,
            rssi: -70,
            packet_type: "ADV_IND".to_string(),
            is_scan_response: false,
            is_extended: false,
            address_type: None,
            advertising_data,
            advertising_data_hex,
            ad_structures: Vec::new(),
            flags: None,
            local_name: None,
            short_name: None,
            advertised_services: Vec::new(),
            manufacturer_data: HashMap::new(),
            service_data: HashMap::new(),
            total_length: 0,
            parsed_successfully: false,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DATABASE SCHEMA DOCUMENTATION
// ═══════════════════════════════════════════════════════════════════════════════

/// SQL Schema for devices table
///
/// CREATE TABLE devices (
///     id INTEGER PRIMARY KEY AUTOINCREMENT,
///     mac_address TEXT UNIQUE NOT NULL,
///     device_name TEXT,
///     rssi INTEGER,
///     first_seen DATETIME,
///     last_seen DATETIME,
///     manufacturer_id INTEGER,
///     manufacturer_name TEXT,
///     device_type TEXT,
///     number_of_scan INTEGER,
///     mac_type TEXT,
///     is_rpa BOOLEAN,
///     security_level TEXT,
///     pairing_method TEXT,
///     created_at DATETIME DEFAULT CURRENT_TIMESTAMP
/// );

/// SQL Schema for packets table
///
/// CREATE TABLE ble_advertisement_frames (
///     id INTEGER PRIMARY KEY AUTOINCREMENT,
///     device_id INTEGER NOT NULL,
///     mac_address TEXT NOT NULL,
///     rssi INTEGER NOT NULL,
///     advertising_data BLOB NOT NULL,    # Raw bytes
///     phy TEXT NOT NULL,
///     channel INTEGER NOT NULL,
///     frame_type TEXT NOT NULL,
///     timestamp DATETIME NOT NULL,
///     created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
///     FOREIGN KEY(device_id) REFERENCES devices(id)
/// );

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_model_creation() {
        let device = DeviceModel::new("AA:BB:CC:DD:EE:FF".to_string());
        assert_eq!(device.mac_address, "AA:BB:CC:DD:EE:FF");
        assert_eq!(device.rssi, -100);
    }

    #[test]
    fn test_device_rssi_tracking() {
        let mut device = DeviceModel::new("AA:BB:CC:DD:EE:FF".to_string());

        device.add_rssi(-60);
        device.add_rssi(-65);
        device.add_rssi(-55);

        assert_eq!(device.rssi, -55);
        assert_eq!(device.last_rssi_values.len(), 3);
        assert!(device.avg_rssi > -65.0 && device.avg_rssi < -55.0);
    }

    #[test]
    fn test_raw_packet_creation() {
        let packet = RawPacketModel::new(
            "AA:BB:CC:DD:EE:FF".to_string(),
            Utc::now(),
            vec![0x02, 0x01, 0x06],
        );

        assert_eq!(packet.mac_address, "AA:BB:CC:DD:EE:FF");
        assert_eq!(packet.advertising_data.len(), 3);
        assert_eq!(packet.advertising_data_hex, "020106");
    }
}
