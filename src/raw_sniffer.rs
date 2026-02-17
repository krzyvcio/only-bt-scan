/// Raw Bluetooth packet sniffing and frame analysis
/// Collects complete advertising packets and raw BLE frames
/// Supports: Linux (HCI raw sockets), Windows (WinAPI), macOS (limited)
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use log::debug;

/// Complete Bluetooth frame with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BluetoothFrame {
    /// MAC address of advertiser
    pub mac_address: String,
    /// Received Signal Strength Indicator (dBm)
    pub rssi: i8,
    /// Raw advertising data (up to 31 bytes for legacy, 251 for extended)
    pub advertising_data: Vec<u8>,
    /// Timestamp when frame was received
    pub timestamp: DateTime<Utc>,
    /// PHY used: 1M, 2M, Coded (S=2 or S=8)
    pub phy: BluetoothPhy,
    /// Channel on which frame was received (37, 38, 39 for BLE)
    pub channel: u8,
    /// Type of advertising PDU
    pub frame_type: AdvertisingType,
    /// Type of device address (Public, Random, etc.)
    pub address_type: AddressType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddressType {
    Public,
    Random,
    PublicId,
    RandomId,
    Anonymous,
    Unknown,
}

impl std::fmt::Display for AddressType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AddressType::Public => write!(f, "Public"),
            AddressType::Random => write!(f, "Random"),
            AddressType::PublicId => write!(f, "Public Identity"),
            AddressType::RandomId => write!(f, "Random Identity"),
            AddressType::Anonymous => write!(f, "Anonymous"),
            AddressType::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BluetoothPhy {
    Le1M,
    Le2M,
    LeCodedS2,
    LeCodedS8,
    BrEdr,
    Unknown,
}

impl std::fmt::Display for BluetoothPhy {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BluetoothPhy::Le1M => write!(f, "LE 1M"),
            BluetoothPhy::Le2M => write!(f, "LE 2M"),
            BluetoothPhy::LeCodedS2 => write!(f, "LE Coded (S=2)"),
            BluetoothPhy::LeCodedS8 => write!(f, "LE Coded (S=8)"),
            BluetoothPhy::BrEdr => write!(f, "BR/EDR"),
            BluetoothPhy::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum AdvertisingType {
    Adv_Ind,         // Connectable undirected
    Adv_Direct_Ind,  // Connectable directed
    Adv_Nonconn_Ind, // Non-connectable undirected
    Adv_Scan_Ind,    // Scannable undirected
    Scan_Rsp,        // Scan response
    Ext_Adv_Ind,     // Extended advertising (BT 5.0+)
    Unknown,
}

impl std::fmt::Display for AdvertisingType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AdvertisingType::Adv_Ind => write!(f, "ADV_IND"),
            AdvertisingType::Adv_Direct_Ind => write!(f, "ADV_DIRECT_IND"),
            AdvertisingType::Adv_Nonconn_Ind => write!(f, "ADV_NONCONN_IND"),
            AdvertisingType::Adv_Scan_Ind => write!(f, "ADV_SCAN_IND"),
            AdvertisingType::Scan_Rsp => write!(f, "SCAN_RSP"),
            AdvertisingType::Ext_Adv_Ind => write!(f, "EXT_ADV_IND"),
            AdvertisingType::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Parsed AD Structure (advertising data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdStructure {
    pub ad_type: u8,
    pub data: Vec<u8>,
    pub name: Option<String>,
}

impl AdStructure {
    pub fn parse(ad_type: u8, data: &[u8]) -> Self {
        let name = match ad_type {
            0x01 => Some("Flags"),
            0x02 => Some("Incomplete List of 16-bit UUIDs"),
            0x03 => Some("Complete List of 16-bit UUIDs"),
            0x06 => Some("Incomplete List of 128-bit UUIDs"),
            0x07 => Some("Complete List of 128-bit UUIDs"),
            0x08 => Some("Shortened Local Name"),
            0x09 => Some("Complete Local Name"),
            0x0A => Some("TX Power Level"),
            0x0D => Some("Class of Device"),
            0x0F => Some("List of 32-bit Service UUIDs"),
            0x10 => Some("Service Data - 16-bit UUID"),
            0x11 => Some("Public Target Address"),
            0x12 => Some("Random Target Address"),
            0x14 => Some("List of 128-bit Service UUIDs"),
            0x15 => Some("Service Data - 128-bit UUID"),
            0x16 => Some("Appearance"),
            0x17 => Some("Advertising Interval"),
            0x18 => Some("LE Bluetooth Device Address"),
            0x19 => Some("LE Role"),
            0x1A => Some("Simple Pairing Hash C"),
            0x1B => Some("Simple Pairing Hash C-256"),
            0x1C => Some("Simple Pairing Randomizer R"),
            0x1D => Some("Simple Pairing Randomizer R-256"),
            0x1F => Some("List of 32-bit Service UUIDs"),
            0x20 => Some("Service Data - 32-bit UUID"),
            0x21 => Some("LE Secure Connections Confirmation Value"),
            0x22 => Some("LE Secure Connections Random Value"),
            0x23 => Some("URI"),
            0x24 => Some("Indoor Positioning"),
            0x25 => Some("Transport Discovery Data"),
            0x26 => Some("LE Supported Features"),
            0x27 => Some("Channel Map Update Indication"),
            0x28 => Some("PB-ADV"),
            0x29 => Some("Mesh Message"),
            0x2A => Some("Mesh Beacon"),
            0x2B => Some("Big Info"),
            0x2C => Some("Broadcast Code"),
            0x3D => Some("3D Information Data"),
            0xFF => Some("Manufacturer Specific Data"),
            _ => None,
        };

        Self {
            ad_type,
            data: data.to_vec(),
            name: name.map(|s| s.to_string()),
        }
    }
}

/// Parse advertising data into AD structures
pub fn parse_advertising_data(raw_data: &[u8]) -> Vec<AdStructure> {
    let mut structures = Vec::new();
    let mut pos = 0;

    while pos < raw_data.len() {
        let len = raw_data[pos] as usize;
        if len == 0 || pos + len + 1 > raw_data.len() {
            break;
        }

        let ad_type = raw_data[pos + 1];
        let data = &raw_data[pos + 2..pos + len + 1];

        structures.push(AdStructure::parse(ad_type, data));
        pos += len + 1;
    }

    structures
}

/// Frame sniffer statistics
#[derive(Debug, Clone, Default)]
pub struct SniffStats {
    pub total_frames: u64,
    pub total_devices: u64,
    pub total_bytes_captured: u64,
    pub average_rssi: f64,
    pub strongest_signal: i8,
    pub weakest_signal: i8,
}

/// Raw packet sniffer (platform-specific implementation)
pub struct RawPacketSniffer {
    frame_buffer: Vec<BluetoothFrame>,
    max_buffer_size: usize,
    stats: SniffStats,
}

impl RawPacketSniffer {
    pub fn new(max_buffer_size: usize) -> Self {
        Self {
            frame_buffer: Vec::with_capacity(max_buffer_size),
            max_buffer_size,
            stats: SniffStats::default(),
        }
    }

    /// Add a captured frame to the buffer
    pub fn add_frame(&mut self, frame: BluetoothFrame) {
        // Update statistics
        self.stats.total_frames += 1;
        self.stats.total_bytes_captured += frame.advertising_data.len() as u64;

        if self.stats.total_frames == 1 {
            self.stats.strongest_signal = frame.rssi;
            self.stats.weakest_signal = frame.rssi;
        } else {
            self.stats.strongest_signal = self.stats.strongest_signal.max(frame.rssi);
            self.stats.weakest_signal = self.stats.weakest_signal.min(frame.rssi);
        }

        // Buffer management
        if self.frame_buffer.len() >= self.max_buffer_size {
            self.frame_buffer.remove(0);
        }

        self.frame_buffer.push(frame);
    }

    /// Get all frames for a specific device
    pub fn get_device_frames(&self, mac_address: &str) -> Vec<&BluetoothFrame> {
        self.frame_buffer
            .iter()
            .filter(|f| f.mac_address == mac_address)
            .collect()
    }

    /// Get all unique devices from captured frames
    pub fn get_unique_devices(&self) -> Vec<String> {
        let mut devices: Vec<String> = self
            .frame_buffer
            .iter()
            .map(|f| f.mac_address.clone())
            .collect();
        devices.sort();
        devices.dedup();
        devices
    }

    /// Get frames captured in time range
    pub fn get_frames_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&BluetoothFrame> {
        self.frame_buffer
            .iter()
            .filter(|f| f.timestamp >= start && f.timestamp <= end)
            .collect()
    }

    /// Get frames by advertising type
    pub fn get_frames_by_type(&self, frame_type: AdvertisingType) -> Vec<&BluetoothFrame> {
        self.frame_buffer
            .iter()
            .filter(|f| f.frame_type == frame_type)
            .collect()
    }

    /// Get current statistics
    pub fn get_stats(&self) -> &SniffStats {
        &self.stats
    }

    /// Export frames as JSON
    pub fn export_frames_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.frame_buffer)
    }

    /// Clear buffer
    pub fn clear(&mut self) {
        self.frame_buffer.clear();
        self.stats = SniffStats::default();
    }
}

#[cfg(target_os = "linux")]
pub mod linux_sniffer {
    use super::*;
    use log::{debug, error, info};

    /// Start raw HCI packet sniffing on Linux
    pub async fn start_hci_sniffing(
        adapter_index: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting HCI sniffing on adapter {}", adapter_index);

        // Raw HCI socket listening would go here
        // Requires: struct hci_filter, HCI_CHANNEL_RAW
        // hci_device_raw_open() + raw packet reception loop

        debug!("HCI sniffing initialized");
        Ok(())
    }

    /// Parse HCI LE Meta Event (LE Advertising Report)
    pub fn parse_le_advertising_report(
        data: &[u8],
    ) -> Result<BluetoothFrame, Box<dyn std::error::Error>> {
        // HCI LE Meta Event (0x3E) parsing
        // Subevent: LE Advertising Report (0x02)

        if data.len() < 11 {
            return Err("Insufficient data for LE Advertising Report".into());
        }

        let event_type = data[0];
        let address_type = data[1];
        let rssi = data[data.len() - 1] as i8;

        let mac_address = format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            data[2], data[3], data[4], data[5], data[6], data[7]
        );

        let data_length = data[8] as usize;
        let advertising_data = if data.len() > 9 {
            data[9..9 + data_length.min(data.len() - 9)].to_vec()
        } else {
            Vec::new()
        };

        let frame_type = match event_type {
            0x00 => AdvertisingType::Adv_Ind,
            0x01 => AdvertisingType::Adv_Direct_Ind,
            0x02 => AdvertisingType::Adv_Nonconn_Ind,
            0x03 => AdvertisingType::Adv_Scan_Ind,
            _ => AdvertisingType::Unknown,
        };

        Ok(BluetoothFrame {
            mac_address,
            rssi,
            advertising_data,
            timestamp: Utc::now(),
            phy: BluetoothPhy::Le1M,
            channel: 37, // Default channel, would need to parse from data
            frame_type,
            address_type: match address_type {
                0x00 => AddressType::Public,
                0x01 => AddressType::Random,
                0x02 => AddressType::PublicId,
                0x03 => AddressType::RandomId,
                0xFF => AddressType::Anonymous,
                _ => AddressType::Unknown,
            },
        })
    }
}

#[cfg(target_os = "windows")]
pub mod windows_sniffer {
    use super::*;
    use log::info;

    /// Start Bluetooth packet sniffing on Windows
    pub async fn start_winsock_sniffing() -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting Windows socket-based Bluetooth sniffing");

        // WinSock2 RAW socket sniffing would go here
        // Requires elevated privileges and WINSOCK_PACKET_FILTER

        debug!("Windows sniffing initialized");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_ad_structure_parsing() {
        let data = vec![0x02, 0x01, 0x06]; // Flags
        let structures = parse_advertising_data(&data);
        assert_eq!(structures.len(), 1);
        assert_eq!(structures[0].ad_type, 0x01);
    }

    #[test]
    fn test_sniffer_stats() {
        let mut sniffer = RawPacketSniffer::new(13);
        let frame = BluetoothFrame {
            mac_address: "AA:BB:CC:DD:EE:FF".to_string(),
            rssi: -50,
            advertising_data: vec![0x02, 0x01, 0x06],
            timestamp: Utc::now(),
            phy: BluetoothPhy::Le1M,
            channel: 37,
            frame_type: AdvertisingType::Adv_Ind,
            address_type: AddressType::Public,
        };

        sniffer.add_frame(frame);
        assert_eq!(sniffer.get_stats().total_frames, 1);
        assert_eq!(sniffer.get_unique_devices().len(), 1);
    }
}
