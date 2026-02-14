/// L2CAP Channel Analyzer - Extract L2CAP PSM (Protocol/Service Multiplexer)
/// and Channel Numbers from Bluetooth connections
///
/// L2CAP (Logical Link Control and Adaptation Protocol) is the layer above HCI
/// that manages channels for different services. Each channel has a unique PSM number.
///
/// Features:
/// - Extract L2CAP channel numbers from connections
/// - Map PSM values to service types
/// - Profile channel usage per device
/// - Detect connection types by L2CAP PSM

use log::{info, warn};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// L2CAP Protocol/Service Multiplexer (PSM)
/// PSM is the L2CAP equivalent of TCP/UDP ports
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct L2CapPsm(pub u16);

impl L2CapPsm {
    /// Get the service name for this PSM
    pub fn service_name(&self) -> &'static str {
        match self.0 {
            // Reserved PSMs
            0x0001 => "SDP (Service Discovery Protocol)",
            0x0003 => "RFCOMM",
            0x0005 => "TCS-BIN",
            0x0007 => "TCS-BIN-CORDLESS",
            0x000F => "BNEP (Bluetooth Network Encapsulation Protocol)",
            0x0011 => "HID-Control (Human Interface Device)",
            0x0013 => "HID-Interrupt",
            0x0015 => "BLE L2CAP (Attribute Protocol)",
            0x0017 => "AVDTP (Audio/Video Distribution Protocol)",
            0x0019 => "AVCTP (Audio/Video Control Transport Protocol)",
            0x001B => "AVCTP-Browsing",
            0x001D => "UDI_C-Plane",
            0x001F => "ATT (Attribute Protocol)",
            0x0021 => "EATT (Enhanced ATT)",
            0x0023 => "LE-SMP (Security Manager Protocol)",
            0x0025 => "SMP Br/EDR",

            // Dynamic PSMs (0x1000-0xFFFF)
            n if n >= 0x1000 => "Dynamic PSM",

            _ => "Unknown PSM",
        }
    }

    /// Is this a dynamic PSM (service-specific)?
    pub fn is_dynamic(&self) -> bool {
        self.0 >= 0x1000
    }

    /// Is this a fixed/reserved PSM?
    pub fn is_reserved(&self) -> bool {
        !self.is_dynamic()
    }
}

impl std::fmt::Display for L2CapPsm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PSM 0x{:04X} ({})", self.0, self.service_name())
    }
}

/// L2CAP Channel Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2CapChannel {
    /// L2CAP Channel ID (CID)
    pub channel_id: u16,

    /// Protocol/Service Multiplexer
    pub psm: L2CapPsm,

    /// Associated device MAC address
    pub mac_address: String,

    /// Channel state
    pub state: ChannelState,

    /// Data transferred in this channel
    pub tx_bytes: u32,
    pub rx_bytes: u32,

    /// Channel configuration
    pub mtu: u16,
    pub flush_timeout_ms: u16,

    /// Timestamps
    pub opened_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelState {
    Connecting,
    Connected,
    Disconnecting,
    Disconnected,
    Error,
}

impl std::fmt::Display for ChannelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelState::Connecting => write!(f, "Connecting"),
            ChannelState::Connected => write!(f, "Connected"),
            ChannelState::Disconnecting => write!(f, "Disconnecting"),
            ChannelState::Disconnected => write!(f, "Disconnected"),
            ChannelState::Error => write!(f, "Error"),
        }
    }
}

/// L2CAP Connection Profile for a device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2CapDeviceProfile {
    pub mac_address: String,
    pub device_name: Option<String>,

    /// Active L2CAP channels
    pub channels: Vec<L2CapChannel>,

    /// PSM usage summary
    pub psm_usage: HashMap<u16, u32>, // PSM -> count

    /// Total data transferred
    pub total_tx_bytes: u64,
    pub total_rx_bytes: u64,

    /// Connection characteristics
    pub supports_ble: bool,
    pub supports_bredr: bool,
    pub supports_eatt: bool, // Enhanced ATT
}

impl L2CapDeviceProfile {
    pub fn new(mac_address: String) -> Self {
        Self {
            mac_address,
            device_name: None,
            channels: Vec::new(),
            psm_usage: HashMap::new(),
            total_tx_bytes: 0,
            total_rx_bytes: 0,
            supports_ble: false,
            supports_bredr: false,
            supports_eatt: false,
        }
    }

    pub fn add_channel(&mut self, channel: L2CapChannel) {
        *self.psm_usage.entry(channel.psm.0).or_insert(0) += 1;
        self.total_tx_bytes += channel.tx_bytes as u64;
        self.total_rx_bytes += channel.rx_bytes as u64;
        self.channels.push(channel);
    }

    pub fn get_active_channels(&self) -> Vec<&L2CapChannel> {
        self.channels
            .iter()
            .filter(|ch| ch.state == ChannelState::Connected)
            .collect()
    }

    pub fn get_channel_summary(&self) -> String {
        let total = self.channels.len();
        let active = self.get_active_channels().len();
        format!(
            "{}: {} total channels ({} active), {}MB tx, {}MB rx",
            self.mac_address,
            total,
            active,
            self.total_tx_bytes / 1_000_000,
            self.total_rx_bytes / 1_000_000
        )
    }
}

/// L2CAP Analyzer
pub struct L2CapAnalyzer {
    devices: HashMap<String, L2CapDeviceProfile>,
}

impl L2CapAnalyzer {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }

    /// Register a device for L2CAP tracking
    pub fn register_device(&mut self, mac_address: String, device_name: Option<String>) {
        let mut profile = L2CapDeviceProfile::new(mac_address);
        profile.device_name = device_name;
        self.devices.insert(profile.mac_address.clone(), profile);
    }

    /// Add an L2CAP channel for a device
    pub fn add_channel(
        &mut self,
        mac_address: &str,
        channel: L2CapChannel,
    ) -> Result<(), String> {
        if let Some(profile) = self.devices.get_mut(mac_address) {
            profile.add_channel(channel);
            Ok(())
        } else {
            Err(format!("Device {} not registered", mac_address))
        }
    }

    /// Get device profile
    pub fn get_device(&self, mac_address: &str) -> Option<&L2CapDeviceProfile> {
        self.devices.get(mac_address)
    }

    /// Get all devices
    pub fn get_all_devices(&self) -> Vec<&L2CapDeviceProfile> {
        self.devices.values().collect()
    }

    /// Print summary of all L2CAP activity
    pub fn print_summary(&self) {
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!("📊 L2CAP Channel Analysis Summary");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        for profile in self.get_all_devices() {
            info!("{}", profile.get_channel_summary());

            for (psm, count) in &profile.psm_usage {
                let psm_obj = L2CapPsm(*psm);
                info!(
                    "   {} - {} channel(s)",
                    psm_obj.service_name(),
                    count
                );
            }
        }

        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }
}

/// Extract L2CAP channels from connection info
#[cfg(target_os = "macos")]
pub mod macos {
    use super::*;

    /// Get L2CAP channels using CoreBluetooth (macOS)
    pub async fn extract_l2cap_channels(
        mac_address: &str,
    ) -> Result<Vec<L2CapChannel>, String> {
        info!("🍎 Extracting L2CAP channels from macOS CoreBluetooth");

        // Note: Real implementation would use CoreBluetooth APIs
        // CBL2CAPChannel provides access to L2CAP connections

        // Example structure from CoreBluetooth:
        // let peripheral = CBPeripheral(address: mac_address)?;
        // let l2cap_channels = peripheral.l2capChannels()?;
        // for channel in l2cap_channels {
        //     channels.push(L2CapChannel {
        //         channel_id: channel.sourceID,
        //         psm: L2CapPsm(channel.psm),
        //         ...
        //     });
        // }

        Ok(Vec::new()) // Placeholder
    }
}

/// Extract L2CAP channels from HCI (Linux/Windows)
#[cfg(not(target_os = "macos"))]
pub mod hci {
    use super::*;

    /// Get L2CAP channels using HCI commands
    pub async fn extract_l2cap_channels(
        mac_address: &str,
    ) -> Result<Vec<L2CapChannel>, String> {
        info!("🐧 Extracting L2CAP channels via HCI interface");

        // HCI Read Information command would be used here:
        // HCI_Read_Link_Supervision_Timeout
        // HCI_Read_Link_Supervision_Timeout
        // HCI_Get_Link_Quality

        // Parse L2CAP frames from packet capture

        Ok(Vec::new()) // Placeholder
    }
}

/// Common L2CAP PSM constants
pub mod psm {
    use super::L2CapPsm;

    pub const SDP: L2CapPsm = L2CapPsm(0x0001);
    pub const RFCOMM: L2CapPsm = L2CapPsm(0x0003);
    pub const HID_CONTROL: L2CapPsm = L2CapPsm(0x0011);
    pub const HID_INTERRUPT: L2CapPsm = L2CapPsm(0x0013);
    pub const ATT: L2CapPsm = L2CapPsm(0x001F);
    pub const EATT: L2CapPsm = L2CapPsm(0x0021);
    pub const SMP: L2CapPsm = L2CapPsm(0x0023);
    pub const AVDTP: L2CapPsm = L2CapPsm(0x0019);
    pub const AVCTP: L2CapPsm = L2CapPsm(0x0019);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l2cap_psm_display() {
        let psm = L2CapPsm(0x0001);
        assert_eq!(psm.service_name(), "SDP (Service Discovery Protocol)");
    }

    #[test]
    fn test_psm_dynamic_detection() {
        assert!(!L2CapPsm(0x0001).is_dynamic()); // SDP is reserved
        assert!(L2CapPsm(0x1000).is_dynamic()); // Dynamic range
        assert!(L2CapPsm(0xFFFF).is_dynamic()); // Dynamic range
    }

    #[test]
    fn test_device_profile() {
        let mut profile = L2CapDeviceProfile::new("AA:BB:CC:DD:EE:FF".to_string());
        assert_eq!(profile.channels.len(), 0);

        let channel = L2CapChannel {
            channel_id: 0x0040,
            psm: L2CapPsm(0x001F), // ATT
            mac_address: "AA:BB:CC:DD:EE:FF".to_string(),
            state: ChannelState::Connected,
            tx_bytes: 1000,
            rx_bytes: 2000,
            mtu: 512,
            flush_timeout_ms: 3000,
            opened_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
        };

        profile.add_channel(channel);
        assert_eq!(profile.channels.len(), 1);
        assert_eq!(profile.total_tx_bytes, 1000);
    }

    #[test]
    fn test_l2cap_analyzer() {
        let mut analyzer = L2CapAnalyzer::new();
        analyzer.register_device("AA:BB:CC:DD:EE:FF".to_string(), Some("Test Device".to_string()));

        let channel = L2CapChannel {
            channel_id: 0x0040,
            psm: L2CapPsm(0x001F),
            mac_address: "AA:BB:CC:DD:EE:FF".to_string(),
            state: ChannelState::Connected,
            tx_bytes: 500,
            rx_bytes: 1000,
            mtu: 512,
            flush_timeout_ms: 3000,
            opened_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
        };

        analyzer
            .add_channel("AA:BB:CC:DD:EE:FF", channel)
            .expect("Failed to add channel");

        let device = analyzer.get_device("AA:BB:CC:DD:EE:FF");
        assert!(device.is_some());
        assert_eq!(device.unwrap().channels.len(), 1);
    }
}
