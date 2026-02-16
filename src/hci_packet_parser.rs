//! HCI Packet Parser - Raw Bluetooth 5.0+ HCI event and L2CAP packet handling

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// L2CAP Packet structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2CapPacket {
    pub payload_length: u16,
    pub channel_id: u16,
    pub packet_type: L2CapPacketType,
    pub payload: Vec<u8>,
    pub sdu_length: Option<u16>,
    pub sar_flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum L2CapPacketType {
    Control,
    Data,
    FirstFragment,
    ContinuationFragment,
}

impl L2CapPacket {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 4 {
            return Err("L2CAP packet too short".to_string());
        }
        let payload_length = u16::from_le_bytes([data[0], data[1]]);
        let channel_id = u16::from_le_bytes([data[2], data[3]]);
        if data.len() < 4 + payload_length as usize {
            return Err("L2CAP payload incomplete".to_string());
        }
        let payload = data[4..4 + payload_length as usize].to_vec();
        Ok(L2CapPacket {
            payload_length,
            channel_id,
            packet_type: L2CapPacketType::Data,
            payload,
            sdu_length: None,
            sar_flags: 0,
        })
    }

    pub fn channel_name(&self) -> &'static str {
        match self.channel_id {
            0x0001 => "L2CAP Signaling",
            0x0003 => "RFCOMM",
            0x001F => "Attribute Protocol (ATT)",
            0x0021 => "Enhanced ATT (EATT)",
            0x0023 => "Security Manager (SMP)",
            _ => "Unknown",
        }
    }
}

/// HCI Event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HciEvent {
    pub event_code: u8,
    pub event_name: String,
    pub parameter_length: u8,
    pub parameters: Vec<u8>,
    pub decoded: Option<HciEventDecoded>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HciEventDecoded {
    LeAdvertisingReport {
        subevent_code: u8,
        num_reports: u8,
        reports: Vec<AdvertisingReport>,
    },
    LeExtendedAdvertisingReport {
        subevent_code: u8,
        num_reports: u8,
        reports: Vec<ExtendedAdvertisingReport>,
    },
    LeConnectionUpdateComplete {
        subevent_code: u8,
        status: u8,
        connection_handle: u16,
        connection_interval: u16,
        peripheral_latency: u16,
        supervision_timeout: u16,
    },
    DisconnectionComplete {
        status: u8,
        connection_handle: u16,
        reason: u8,
    },
    Other {
        data: Vec<u8>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvertisingReport {
    pub event_type: u8,
    pub address_type: u8,
    pub address: String,
    pub rssi: i8,
    pub advertising_data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedAdvertisingReport {
    pub event_type: u16,
    pub address_type: u8,
    pub address: String,
    pub primary_phy: u8,
    pub secondary_phy: u8,
    pub advertising_sid: u8,
    pub tx_power: i8,
    pub rssi: i8,
    pub periodic_advertising_interval: u16,
    pub data_length: u8,
    pub advertising_data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhyType {
    Le1m = 1,
    Le2m = 2,
    LeCoded = 3,
}

impl PhyType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(PhyType::Le1m),
            2 => Some(PhyType::Le2m),
            3 => Some(PhyType::LeCoded),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            PhyType::Le1m => "LE 1M",
            PhyType::Le2m => "LE 2M",
            PhyType::LeCoded => "LE Coded",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HciPacketStats {
    pub total_hci_events: u64,
    pub total_l2cap_packets: u64,
    pub event_type_counts: HashMap<String, u64>,
    pub cid_counts: HashMap<u16, u64>,
    pub phy_distribution: HashMap<String, u64>,
}

pub struct HciPacketParser {
    pub stats: HciPacketStats,
}

impl Default for HciPacketParser {
    fn default() -> Self {
        Self::new()
    }
}

impl HciPacketParser {
    pub fn new() -> Self {
        HciPacketParser {
            stats: HciPacketStats {
                total_hci_events: 0,
                total_l2cap_packets: 0,
                event_type_counts: HashMap::new(),
                cid_counts: HashMap::new(),
                phy_distribution: HashMap::new(),
            },
        }
    }

    pub fn parse_hci_event(&mut self, event_code: u8, parameters: &[u8]) -> HciEvent {
        self.stats.total_hci_events += 1;
        let event_name = match event_code {
            0x05 => "Disconnection Complete".to_string(),
            0x3E => "LE Meta Event".to_string(),
            0x13 => "Number of Completed Packets".to_string(),
            _ => format!("Unknown Event (0x{:02X})", event_code),
        };
        *self
            .stats
            .event_type_counts
            .entry(event_name.clone())
            .or_insert(0) += 1;
        HciEvent {
            event_code,
            event_name,
            parameter_length: parameters.len() as u8,
            parameters: parameters.to_vec(),
            decoded: None,
        }
    }

    pub fn parse_l2cap_packet(&mut self, data: &[u8]) -> Result<L2CapPacket, String> {
        let packet = L2CapPacket::parse(data)?;
        self.stats.total_l2cap_packets += 1;
        *self.stats.cid_counts.entry(packet.channel_id).or_insert(0) += 1;
        Ok(packet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_l2cap_packet_parsing() {
        let data = vec![0x04, 0x00, 0x1F, 0x00, 0x01, 0x02, 0x03, 0x04];
        let packet = L2CapPacket::parse(&data).unwrap();
        assert_eq!(packet.payload_length, 4);
        assert_eq!(packet.channel_id, 0x001F);
    }
    #[test]
    fn test_phy_type() {
        assert_eq!(PhyType::from_u8(1), Some(PhyType::Le1m));
        assert_eq!(PhyType::from_u8(2), Some(PhyType::Le2m));
    }
}
