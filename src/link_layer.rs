/// Link Layer Analysis - BLE Connection Parameters and Statistics
/// Analyzes connection interval, latency, PHY, channel usage, packet quality
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// BLE Link Layer connection parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkLayerParameters {
    pub connection_interval_ms: Option<f64>,
    pub peripheral_latency: Option<u16>,
    pub connection_timeout_ms: Option<u32>,
    pub phy_tx: Option<String>,
    pub phy_rx: Option<String>,
    pub mtu: Option<u16>,
}

impl Default for LinkLayerParameters {
    fn default() -> Self {
        Self {
            connection_interval_ms: None,
            peripheral_latency: None,
            connection_timeout_ms: None,
            phy_tx: None,
            phy_rx: None,
            mtu: None,
        }
    }
}

/// Channel map information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMap {
    pub channel_37: bool,
    pub channel_38: bool,
    pub channel_39: bool,
    pub data_channels: Vec<u8>,
}

impl ChannelMap {
    pub fn new() -> Self {
        Self {
            channel_37: true,
            channel_38: true,
            channel_39: true,
            data_channels: (0..37).collect(),
        }
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 5 {
            return None;
        }

        let mut map = Self::new();

        // First 37 bits are data channels
        let mut enabled_channels = Vec::new();
        for i in 0..37 {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            if (data[byte_idx] & (1 << bit_idx)) != 0 {
                enabled_channels.push(i as u8);
            }
        }

        map.data_channels = enabled_channels;
        Some(map)
    }

    pub fn enabled_count(&self) -> usize {
        let mut count = 0;
        if self.channel_37 {
            count += 1;
        }
        if self.channel_38 {
            count += 1;
        }
        if self.channel_39 {
            count += 1;
        }
        count += self.data_channels.len();
        count
    }

    pub fn is_healthy(&self) -> bool {
        // Healthy if at least 2 advertising channels are enabled
        let adv_enabled =
            (self.channel_37 as usize) + (self.channel_38 as usize) + (self.channel_39 as usize);
        adv_enabled >= 2 && !self.data_channels.is_empty()
    }
}

/// Packet statistics for a device or connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketStatistics {
    pub total_packets: u64,
    pub total_bytes: u64,
    pub average_rssi: f64,
    pub strongest_rssi: i8,
    pub weakest_rssi: i8,
    pub rssi_samples: Vec<i8>,
    pub packet_loss_rate: Option<f64>,
    pub retransmission_count: Option<u32>,
    pub crc_error_count: Option<u32>,
    pub channel_distribution: HashMap<u8, u32>,
}

impl Default for PacketStatistics {
    fn default() -> Self {
        Self {
            total_packets: 0,
            total_bytes: 0,
            average_rssi: -100.0,
            strongest_rssi: -100,
            weakest_rssi: -30,
            rssi_samples: Vec::new(),
            packet_loss_rate: None,
            retransmission_count: None,
            crc_error_count: None,
            channel_distribution: HashMap::new(),
        }
    }
}

impl PacketStatistics {
    pub fn add_packet(&mut self, rssi: i8, bytes: usize, channel: u8) {
        self.total_packets += 1;
        self.total_bytes += bytes as u64;
        self.rssi_samples.push(rssi);

        if rssi > self.strongest_rssi {
            self.strongest_rssi = rssi;
        }
        if rssi < self.weakest_rssi {
            self.weakest_rssi = rssi;
        }

        // Update average RSSI
        let sum: i32 = self.rssi_samples.iter().map(|&r| r as i32).sum();
        self.average_rssi = (sum as f64) / (self.rssi_samples.len() as f64);

        // Update channel distribution
        *self.channel_distribution.entry(channel).or_insert(0) += 1;

        // Keep only last 1000 RSSI samples to avoid memory bloat
        if self.rssi_samples.len() > 1000 {
            self.rssi_samples.remove(0);
        }
    }

    pub fn get_signal_quality(&self) -> SignalQuality {
        if self.average_rssi > -50.0 {
            SignalQuality::Excellent
        } else if self.average_rssi > -65.0 {
            SignalQuality::Good
        } else if self.average_rssi > -75.0 {
            SignalQuality::Fair
        } else if self.average_rssi > -85.0 {
            SignalQuality::Poor
        } else {
            SignalQuality::VeryPoor
        }
    }

    pub fn get_rssi_variance(&self) -> f64 {
        if self.rssi_samples.len() < 2 {
            return 0.0;
        }

        let avg_int = self.average_rssi as i32;
        let sum_sq: i64 = self
            .rssi_samples
            .iter()
            .map(|&r| {
                let diff = (r as i32) - avg_int;
                (diff * diff) as i64
            })
            .sum();

        let variance = (sum_sq as f64) / (self.rssi_samples.len() as f64);
        variance.sqrt()
    }

    pub fn get_most_used_channel(&self) -> Option<(u8, u32)> {
        self.channel_distribution
            .iter()
            .max_by_key(|&(_, count)| count)
            .map(|(&ch, &count)| (ch, count))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SignalQuality {
    Excellent, // > -50 dBm
    Good,      // -50 to -65 dBm
    Fair,      // -65 to -75 dBm
    Poor,      // -75 to -85 dBm
    VeryPoor,  // < -85 dBm
}

impl std::fmt::Display for SignalQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignalQuality::Excellent => write!(f, "Excellent"),
            SignalQuality::Good => write!(f, "Good"),
            SignalQuality::Fair => write!(f, "Fair"),
            SignalQuality::Poor => write!(f, "Poor"),
            SignalQuality::VeryPoor => write!(f, "Very Poor"),
        }
    }
}

/// Link layer health assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkLayerHealth {
    pub signal_quality: SignalQuality,
    pub channel_health: ChannelHealth,
    pub packet_quality: PacketQuality,
    pub connection_stability: ConnectionStability,
    pub overall_health: OverallHealth,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChannelHealth {
    Excellent,
    Good,
    Fair,
    Poor,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PacketQuality {
    Excellent,
    Good,
    Fair,
    Poor,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectionStability {
    Stable,
    Unstable,
    Disconnecting,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OverallHealth {
    Healthy,
    Warning,
    Critical,
}

/// Analyze link layer health
pub fn assess_link_health(
    params: &LinkLayerParameters,
    stats: &PacketStatistics,
    channel_map: Option<&ChannelMap>,
) -> LinkLayerHealth {
    let signal_quality = stats.get_signal_quality();

    let channel_health = if let Some(map) = channel_map {
        if map.is_healthy() {
            ChannelHealth::Excellent
        } else if map.enabled_count() >= 20 {
            ChannelHealth::Good
        } else {
            ChannelHealth::Poor
        }
    } else {
        ChannelHealth::Fair
    };

    let packet_quality = if stats.average_rssi > -65.0 && stats.get_rssi_variance() < 5.0 {
        PacketQuality::Excellent
    } else if stats.average_rssi > -75.0 {
        PacketQuality::Good
    } else if stats.average_rssi > -85.0 {
        PacketQuality::Fair
    } else {
        PacketQuality::Poor
    };

    let rssi_variance = stats.get_rssi_variance();
    let connection_stability = if rssi_variance < 3.0 {
        ConnectionStability::Stable
    } else if rssi_variance < 8.0 {
        ConnectionStability::Unstable
    } else {
        ConnectionStability::Disconnecting
    };

    let overall_health = match (
        signal_quality,
        channel_health,
        packet_quality,
        connection_stability,
    ) {
        (SignalQuality::Excellent, _, PacketQuality::Excellent, ConnectionStability::Stable) => {
            OverallHealth::Healthy
        }
        (SignalQuality::VeryPoor, _, _, _) => OverallHealth::Critical,
        (_, ChannelHealth::Poor, _, _) => OverallHealth::Critical,
        (_, _, PacketQuality::Poor, _) => OverallHealth::Warning,
        (_, _, _, ConnectionStability::Disconnecting) => OverallHealth::Warning,
        _ => OverallHealth::Healthy,
    };

    LinkLayerHealth {
        signal_quality,
        channel_health,
        packet_quality,
        connection_stability,
        overall_health,
    }
}

/// Connection interval helper
pub fn calculate_connection_interval(interval_units: u16) -> f64 {
    (interval_units as f64) * 1.25 // Each unit is 1.25ms
}

/// Convert PHY value to string
pub fn phy_to_string(phy: u8) -> String {
    match phy {
        0x01 => "LE 1M".to_string(),
        0x02 => "LE 2M".to_string(),
        0x03 => "LE Coded (S=8)".to_string(),
        0x04 => "LE Coded (S=2)".to_string(),
        _ => format!("Unknown PHY ({})", phy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_statistics() {
        let mut stats = PacketStatistics::default();
        stats.add_packet(-60, 100, 37);
        stats.add_packet(-65, 120, 38);
        stats.add_packet(-55, 110, 37);

        assert_eq!(stats.total_packets, 3);
        assert_eq!(stats.total_bytes, 330);
        assert_eq!(stats.strongest_rssi, -55);
        assert_eq!(stats.weakest_rssi, -65);
    }

    #[test]
    fn test_signal_quality() {
        let mut stats = PacketStatistics::default();
        stats.average_rssi = -45.0;
        assert_eq!(stats.get_signal_quality(), SignalQuality::Excellent);

        stats.average_rssi = -70.0;
        assert_eq!(stats.get_signal_quality(), SignalQuality::Fair);

        stats.average_rssi = -90.0;
        assert_eq!(stats.get_signal_quality(), SignalQuality::VeryPoor);
    }

    #[test]
    fn test_connection_interval() {
        let interval_ms = calculate_connection_interval(40);
        assert_eq!(interval_ms, 50.0); // 40 * 1.25 = 50ms
    }

    #[test]
    fn test_channel_map() {
        let map = ChannelMap::new();
        assert!(map.is_healthy());
        assert_eq!(map.enabled_count(), 40); // 3 advertising + 37 data
    }

    #[test]
    fn test_phy_to_string() {
        assert_eq!(phy_to_string(0x01), "LE 1M");
        assert_eq!(phy_to_string(0x02), "LE 2M");
    }
}
