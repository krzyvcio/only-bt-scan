//! HCI Scanner - Raw HCI event capture and L2CAP packet parsing

use crate::hci_packet_parser::{HciEvent, HciPacketParser, L2CapPacket};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HciScannerConfig {
    pub enabled: bool,
    pub device_id: u8,
    pub scan_timeout: u64,
    pub capture_l2cap: bool,
    pub capture_hci_events: bool,
    pub cid_filter: Option<Vec<u16>>,
}

impl Default for HciScannerConfig {
    fn default() -> Self {
        HciScannerConfig {
            enabled: true,
            device_id: 0,
            scan_timeout: 30,
            capture_l2cap: true,
            capture_hci_events: true,
            cid_filter: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HciScanResult {
    pub hci_events: Vec<HciEvent>,
    pub l2cap_packets: Vec<L2CapPacketInfo>,
    pub stats: HciScanStatistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2CapPacketInfo {
    pub packet: L2CapPacket,
    pub timestamp: u64,
    pub source_mac: Option<String>,
    pub dest_cid: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HciScanStatistics {
    pub total_events: u64,
    pub total_l2cap_packets: u64,
    pub events_by_type: HashMap<String, u64>,
    pub packets_by_cid: HashMap<u16, u64>,
    pub extended_advertising_reports: u64,
    pub connection_updates: u64,
    pub disconnections: u64,
    pub avg_packet_size: f64,
}

impl Default for HciScanStatistics {
    fn default() -> Self {
        HciScanStatistics {
            total_events: 0,
            total_l2cap_packets: 0,
            events_by_type: HashMap::new(),
            packets_by_cid: HashMap::new(),
            extended_advertising_reports: 0,
            connection_updates: 0,
            disconnections: 0,
            avg_packet_size: 0.0,
        }
    }
}

pub struct HciScanner {
    config: HciScannerConfig,
    parser: HciPacketParser,
    stats: HciScanStatistics,
    captured_events: Vec<HciEvent>,
    captured_packets: Vec<L2CapPacketInfo>,
}

impl HciScanner {
    pub fn new(config: HciScannerConfig) -> Self {
        HciScanner {
            config,
            parser: HciPacketParser::new(),
            stats: HciScanStatistics::default(),
            captured_events: Vec::new(),
            captured_packets: Vec::new(),
        }
    }

    pub fn default() -> Self {
        Self::new(HciScannerConfig::default())
    }

    pub fn simulate_hci_event(&mut self, event_code: u8, parameters: &[u8]) -> HciEvent {
        info!("HCI Event: 0x{:02X}", event_code);
        let event = self.parser.parse_hci_event(event_code, parameters);

        self.stats.total_events += 1;
        *self
            .stats
            .events_by_type
            .entry(event.event_name.clone())
            .or_insert(0) += 1;

        match event_code {
            0x3E => {
                if !parameters.is_empty() {
                    match parameters[0] {
                        0x13 => self.stats.extended_advertising_reports += 1,
                        0x03 => self.stats.connection_updates += 1,
                        _ => {}
                    }
                }
            }
            0x05 => self.stats.disconnections += 1,
            _ => {}
        }

        self.captured_events.push(event.clone());
        event
    }

    pub fn simulate_l2cap_packet(
        &mut self,
        data: &[u8],
        source_mac: Option<String>,
    ) -> Result<L2CapPacketInfo, String> {
        let packet = self.parser.parse_l2cap_packet(data)?;

        if let Some(ref filter) = self.config.cid_filter {
            if !filter.contains(&packet.channel_id) {
                return Err("CID filtered out".to_string());
            }
        }

        let packet_size = packet.payload.len() as f64;
        self.stats.avg_packet_size =
            (self.stats.avg_packet_size * self.stats.total_l2cap_packets as f64 + packet_size)
                / (self.stats.total_l2cap_packets as f64 + 1.0);

        let info = L2CapPacketInfo {
            packet,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            source_mac,
            dest_cid: 0,
        };

        self.stats.total_l2cap_packets += 1;
        *self
            .stats
            .packets_by_cid
            .entry(info.packet.channel_id)
            .or_insert(0) += 1;

        debug!(
            "L2CAP Packet: CID=0x{:04X} ({}), Size={} bytes",
            info.packet.channel_id,
            info.packet.channel_name(),
            info.packet.payload.len()
        );

        self.captured_packets.push(info.clone());
        Ok(info)
    }

    pub fn get_stats(&self) -> HciScanStatistics {
        self.stats.clone()
    }

    pub fn get_hci_events(&self) -> Vec<HciEvent> {
        self.captured_events.clone()
    }

    pub fn get_l2cap_packets(&self) -> Vec<L2CapPacketInfo> {
        self.captured_packets.clone()
    }

    pub fn get_results(&self) -> HciScanResult {
        HciScanResult {
            hci_events: self.captured_events.clone(),
            l2cap_packets: self.captured_packets.clone(),
            stats: self.stats.clone(),
        }
    }

    pub fn clear(&mut self) {
        self.captured_events.clear();
        self.captured_packets.clear();
        self.stats = HciScanStatistics::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hci_scanner_creation() {
        let scanner = HciScanner::default();
        assert_eq!(scanner.stats.total_events, 0);
    }

    #[test]
    fn test_hci_event_simulation() {
        let mut scanner = HciScanner::default();
        let event = scanner.simulate_hci_event(0x05, &[0x00, 0x01, 0x02, 0x13]);
        assert_eq!(event.event_code, 0x05);
        assert_eq!(scanner.stats.total_events, 1);
    }

    #[test]
    fn test_l2cap_packet_simulation() {
        let mut scanner = HciScanner::default();
        let data = vec![0x04, 0x00, 0x1F, 0x00, 0x01, 0x02, 0x03, 0x04];
        let result = scanner.simulate_l2cap_packet(&data, None);
        assert!(result.is_ok());
        assert_eq!(scanner.stats.total_l2cap_packets, 1);
    }

    #[test]
    fn test_cid_filter() {
        let config = HciScannerConfig {
            cid_filter: Some(vec![0x0003, 0x001F]),
            ..Default::default()
        };
        let mut scanner = HciScanner::new(config);
        let data = vec![0x04, 0x00, 0x1F, 0x00, 0x01, 0x02, 0x03, 0x04];
        assert!(scanner.simulate_l2cap_packet(&data, None).is_ok());
    }
}
