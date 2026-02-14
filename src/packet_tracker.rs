/// Packet Tracking & Ordering Module
///
/// Handles:
/// - Deduplication of packets within time window
/// - Packet ordering by timestamp
/// - Sequence tracking across multiple devices
/// - RSSI-based filtering
use crate::config_params::*;
use crate::data_models::RawPacketModel;
use std::collections::HashMap;

/// Tracks packet order and deduplication for a single device
#[derive(Debug, Clone)]
pub struct DevicePacketTracker {
    pub mac_address: String,
    pub packet_sequence: Vec<u64>,         // Ordered packet IDs
    pub last_packet_time_ms: u64,          // Last seen timestamp (ms)
    pub packet_rssi_map: HashMap<u64, i8>, // packet_id -> RSSI
    pub total_packets: u64,
    pub total_filtered: u64,
    pub total_duplicates: u64,
}

impl DevicePacketTracker {
    pub fn new(mac_address: String) -> Self {
        Self {
            mac_address,
            packet_sequence: Vec::new(),
            last_packet_time_ms: 0,
            packet_rssi_map: HashMap::new(),
            total_packets: 0,
            total_filtered: 0,
            total_duplicates: 0,
        }
    }

    /// Add packet if it passes filters and deduplication
    pub fn add_packet(&mut self, packet: &RawPacketModel) -> bool {
        self.total_packets += 1;

        // Filter 1: RSSI threshold
        if !should_accept_rssi(packet.rssi) {
            self.total_filtered += 1;
            return false;
        }

        // Filter 2: Deduplication
        if self.is_duplicate(&packet) {
            self.total_duplicates += 1;
            return false; // Reject duplicate
        }

        // Packet accepted
        self.packet_sequence.push(packet.packet_id);
        self.packet_rssi_map.insert(packet.packet_id, packet.rssi);
        self.last_packet_time_ms = packet.timestamp_ms;

        true
    }

    /// Check if packet is a duplicate (same device, within dedup window, weaker signal)
    fn is_duplicate(&self, packet: &RawPacketModel) -> bool {
        if self.last_packet_time_ms == 0 {
            return false; // First packet
        }

        let time_diff = calculate_latency_ms(self.last_packet_time_ms, packet.timestamp_ms);

        if time_diff <= PACKET_DEDUP_WINDOW_MS {
            // Within dedup window - compare RSSI
            if let Some(last_rssi) = self.packet_rssi_map.iter().last().map(|(_, v)| *v) {
                // Keep only stronger signal
                return packet.rssi < last_rssi;
            }
        }

        false
    }

    /// Get packet sequence as ordered vector of packet IDs
    pub fn get_sequence(&self) -> &[u64] {
        &self.packet_sequence
    }

    /// Get statistics
    pub fn get_stats(&self) -> PacketStats {
        PacketStats {
            total_received: self.total_packets,
            total_accepted: self.packet_sequence.len() as u64,
            total_filtered: self.total_filtered,
            total_duplicates: self.total_duplicates,
            acceptance_rate: if self.total_packets > 0 {
                (self.packet_sequence.len() as f64 / self.total_packets as f64) * 100.0
            } else {
                0.0
            },
        }
    }
}

/// Global packet ordering tracker (across all devices)
#[derive(Debug)]
pub struct GlobalPacketTracker {
    pub device_trackers: HashMap<String, DevicePacketTracker>,
    pub global_sequence: Vec<(String, u64, u64)>, // (mac_address, packet_id, timestamp_ms)
    pub packet_count: u64,
}

impl GlobalPacketTracker {
    pub fn new() -> Self {
        Self {
            device_trackers: HashMap::new(),
            global_sequence: Vec::new(),
            packet_count: 0,
        }
    }

    /// Add packet globally
    pub fn add_packet(&mut self, packet: RawPacketModel) -> PacketAddResult {
        let mac_address = packet.mac_address.clone();
        let packet_id = packet.packet_id;
        let timestamp_ms = packet.timestamp_ms;

        // Get or create device tracker
        let tracker = self
            .device_trackers
            .entry(mac_address.clone())
            .or_insert_with(|| DevicePacketTracker::new(mac_address.clone()));

        // Try to add packet
        let accepted = tracker.add_packet(&packet);

        if accepted {
            // Add to global sequence
            self.global_sequence
                .push((mac_address, packet_id, timestamp_ms));
            self.packet_count += 1;

            PacketAddResult::Accepted {
                packet_id,
                device_mac: tracker.mac_address.clone(),
                sequence_position: tracker.packet_sequence.len(),
            }
        } else {
            PacketAddResult::Rejected {
                packet_id,
                device_mac: mac_address,
                reason: "Failed RSSI or deduplication checks".to_string(),
            }
        }
    }

    /// Get all packets in global order (by timestamp)
    pub fn get_global_sequence(&self) -> Vec<(String, u64, u64)> {
        let mut sorted = self.global_sequence.clone();
        sorted.sort_by_key(|&(_, _, ts)| ts);
        sorted
    }

    /// Get device-specific packet sequence
    pub fn get_device_sequence(&self, mac_address: &str) -> Option<Vec<u64>> {
        self.device_trackers
            .get(mac_address)
            .map(|t| t.packet_sequence.clone())
    }

    /// Get overall statistics
    pub fn get_global_stats(&self) -> GlobalPacketStats {
        let total_received: u64 = self.device_trackers.values().map(|t| t.total_packets).sum();
        let total_filtered: u64 = self
            .device_trackers
            .values()
            .map(|t| t.total_filtered)
            .sum();
        let total_duplicates: u64 = self
            .device_trackers
            .values()
            .map(|t| t.total_duplicates)
            .sum();

        GlobalPacketStats {
            unique_devices: self.device_trackers.len(),
            total_packets_received: total_received,
            total_packets_accepted: self.packet_count,
            total_filtered: total_filtered,
            total_duplicates: total_duplicates,
            acceptance_rate: if total_received > 0 {
                (self.packet_count as f64 / total_received as f64) * 100.0
            } else {
                0.0
            },
        }
    }
}

/// Result of adding a packet
#[derive(Debug, Clone)]
pub enum PacketAddResult {
    Accepted {
        packet_id: u64,
        device_mac: String,
        sequence_position: usize,
    },
    Rejected {
        packet_id: u64,
        device_mac: String,
        reason: String,
    },
}

/// Statistics for a device
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PacketStats {
    pub total_received: u64,
    pub total_accepted: u64,
    pub total_filtered: u64,
    pub total_duplicates: u64,
    pub acceptance_rate: f64,
}

/// Global statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GlobalPacketStats {
    pub unique_devices: usize,
    pub total_packets_received: u64,
    pub total_packets_accepted: u64,
    pub total_filtered: u64,
    pub total_duplicates: u64,
    pub acceptance_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_packet(mac: &str, id: u64, rssi: i8, timestamp_ms: u64) -> RawPacketModel {
        let mut packet = RawPacketModel::new(mac.to_string(), Utc::now(), vec![0x02, 0x01, 0x06]);
        packet.packet_id = id;
        packet.rssi = rssi;
        packet.timestamp_ms = timestamp_ms;
        packet
    }

    #[test]
    fn test_device_tracker_accepts_good_rssi() {
        let mut tracker = DevicePacketTracker::new("AA:BB:CC:DD:EE:FF".to_string());
        let packet = create_test_packet("AA:BB:CC:DD:EE:FF", 1, -60, 1000);

        assert!(tracker.add_packet(&packet));
        assert_eq!(tracker.total_packets, 1);
        assert_eq!(tracker.packet_sequence.len(), 1);
    }

    #[test]
    fn test_device_tracker_rejects_weak_rssi() {
        let mut tracker = DevicePacketTracker::new("AA:BB:CC:DD:EE:FF".to_string());
        let packet = create_test_packet("AA:BB:CC:DD:EE:FF", 1, -85, 1000); // Below threshold

        assert!(!tracker.add_packet(&packet));
        assert_eq!(tracker.total_filtered, 1);
    }

    #[test]
    fn test_global_tracker_ordering() {
        let mut tracker = GlobalPacketTracker::new();

        let p1 = create_test_packet("AA:BB:CC:DD:EE:FF", 1, -60, 1000);
        let p2 = create_test_packet("AA:BB:CC:DD:EE:FF", 2, -65, 2000);
        let p3 = create_test_packet("11:22:33:44:55:66", 3, -70, 1500);

        tracker.add_packet(p1);
        tracker.add_packet(p2);
        tracker.add_packet(p3);

        let seq = tracker.get_global_sequence();
        assert_eq!(seq.len(), 3);
        assert_eq!(seq[0].2, 1000); // First by timestamp
        assert_eq!(seq[1].2, 1500); // Second
        assert_eq!(seq[2].2, 2000); // Third
    }
}
