/// Telemetry & Analytics - Packet Tracking Insights
///
/// Exports:
/// - Packet sequences (JSON)
/// - Latency analysis (inter-packet times)
/// - Device statistics
/// - Timeline events
use crate::packet_tracker::{GlobalPacketStats, GlobalPacketTracker, PacketAddResult, PacketStats};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ═══════════════════════════════════════════════════════════════════════════════
/// LATENCY & TIMING ANALYSIS
/// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyAnalysis {
    pub device_mac: String,
    pub inter_packet_latencies_ms: Vec<u64>, // Time between consecutive packets
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub avg_latency_ms: f64,
    pub median_latency_ms: u64,
}

impl LatencyAnalysis {
    pub fn new(device_mac: String, timestamps_ms: Vec<u64>) -> Self {
        let mut latencies = Vec::new();

        // Calculate inter-packet times
        for i in 1..timestamps_ms.len() {
            if let Some(diff) = timestamps_ms[i].checked_sub(timestamps_ms[i - 1]) {
                if diff > 0 {
                    latencies.push(diff);
                }
            }
        }

        let (min, max, avg, median) = if !latencies.is_empty() {
            let min = *latencies.iter().min().unwrap_or(&0);
            let max = *latencies.iter().max().unwrap_or(&0);
            let avg = latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;

            let mut sorted = latencies.clone();
            sorted.sort();
            let median = sorted[sorted.len() / 2];

            (min, max, avg, median)
        } else {
            (0, 0, 0.0, 0)
        };

        Self {
            device_mac,
            inter_packet_latencies_ms: latencies,
            min_latency_ms: min,
            max_latency_ms: max,
            avg_latency_ms: avg,
            median_latency_ms: median,
        }
    }
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// EVENT TIMELINE
/// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub timestamp_ms: u64,
    pub device_mac: String,
    pub packet_id: u64,
    pub event_type: EventType,
    pub rssi: i8,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    PacketReceived,
    PacketFiltered,
    PacketDuplicate,
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// TELEMETRY EXPORTS
/// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketSequenceTelemetry {
    pub export_timestamp: DateTime<Utc>,
    pub device_mac: String,
    pub packet_ids: Vec<u64>,
    pub timestamps_ms: Vec<u64>,
    pub rssi_values: Vec<i8>,
    pub sequence_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalTelemetry {
    pub export_timestamp: DateTime<Utc>,
    pub global_stats: GlobalPacketStats,
    pub device_stats: HashMap<String, PacketStats>,
    pub device_latencies: Vec<LatencyAnalysis>,
    pub timeline: Vec<TimelineEvent>,
    pub top_devices_by_packet_count: Vec<(String, u64)>,
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// TELEMETRY GENERATOR
/// ═══════════════════════════════════════════════════════════════════════════════
pub struct TelemetryCollector {
    events: Vec<TimelineEvent>,
}

impl TelemetryCollector {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn get_events(&self) -> &[TimelineEvent] {
        &self.events
    }

    pub fn get_events_clone(&self) -> Vec<TimelineEvent> {
        self.events.clone()
    }

    /// Record packet addition result
    pub fn record_packet_result(&mut self, result: &PacketAddResult, rssi: i8) {
        match result {
            PacketAddResult::Accepted {
                packet_id,
                device_mac,
                ..
            } => {
                self.events.push(TimelineEvent {
                    timestamp_ms: Utc::now().timestamp_millis() as u64,
                    device_mac: device_mac.clone(),
                    packet_id: *packet_id,
                    event_type: EventType::PacketReceived,
                    rssi,
                    details: "Packet accepted and added to sequence".to_string(),
                });
            }
            PacketAddResult::Rejected {
                packet_id,
                device_mac,
                reason,
            } => {
                let event_type = if reason.contains("duplicate") {
                    EventType::PacketDuplicate
                } else {
                    EventType::PacketFiltered
                };

                self.events.push(TimelineEvent {
                    timestamp_ms: Utc::now().timestamp_millis() as u64,
                    device_mac: device_mac.clone(),
                    packet_id: *packet_id,
                    event_type,
                    rssi,
                    details: reason.clone(),
                });
            }
        }
    }

    /// Generate latency analysis for device
    pub fn analyze_device_latency(
        &self,
        tracker: &GlobalPacketTracker,
        mac_address: &str,
    ) -> Option<LatencyAnalysis> {
        let packet_ids = tracker.get_device_sequence(mac_address)?;

        // Collect timestamps for these packets from events
        let timestamps: Vec<u64> = self
            .events
            .iter()
            .filter(|e| e.device_mac == mac_address && packet_ids.contains(&e.packet_id))
            .map(|e| e.timestamp_ms)
            .collect();

        if timestamps.is_empty() {
            return None;
        }

        Some(LatencyAnalysis::new(mac_address.to_string(), timestamps))
    }

    /// Generate complete global telemetry
    pub fn generate_global_telemetry(&self, tracker: &GlobalPacketTracker) -> GlobalTelemetry {
        let global_stats = tracker.get_global_stats();

        // Device stats
        let mut device_stats = HashMap::new();
        for (mac, device_tracker) in &tracker.device_trackers {
            device_stats.insert(mac.clone(), device_tracker.get_stats());
        }

        // Device latencies
        let mut device_latencies = Vec::new();
        for mac_address in tracker.device_trackers.keys() {
            if let Some(latency) = self.analyze_device_latency(tracker, mac_address) {
                device_latencies.push(latency);
            }
        }

        // Top devices
        let mut top_devices: Vec<_> = tracker
            .device_trackers
            .iter()
            .map(|(mac, tracker)| (mac.clone(), tracker.packet_sequence.len() as u64))
            .collect();
        top_devices.sort_by(|a, b| b.1.cmp(&a.1));
        top_devices.truncate(10);

        GlobalTelemetry {
            export_timestamp: Utc::now(),
            global_stats,
            device_stats,
            device_latencies,
            timeline: self.events.clone(),
            top_devices_by_packet_count: top_devices,
        }
    }

    /// Generate packet sequence for single device
    pub fn generate_device_telemetry(
        &self,
        tracker: &GlobalPacketTracker,
        mac_address: &str,
    ) -> Option<PacketSequenceTelemetry> {
        let packet_ids = tracker.get_device_sequence(mac_address)?;

        // Collect timing and RSSI data
        let mut timestamps = Vec::new();
        let mut rssi_values = Vec::new();

        for packet_id in &packet_ids {
            if let Some(event) = self.events.iter().find(|e| e.packet_id == *packet_id) {
                timestamps.push(event.timestamp_ms);
                rssi_values.push(event.rssi);
            }
        }

        let sequence_length = packet_ids.len();

        Some(PacketSequenceTelemetry {
            export_timestamp: Utc::now(),
            device_mac: mac_address.to_string(),
            packet_ids,
            timestamps_ms: timestamps,
            rssi_values,
            sequence_length,
        })
    }
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// JSON EXPORT HELPERS
/// ═══════════════════════════════════════════════════════════════════════════════
pub fn telemetry_to_json(telemetry: &GlobalTelemetry) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(telemetry)
}

pub fn device_telemetry_to_json(
    telemetry: &PacketSequenceTelemetry,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(telemetry)
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// GLOBAL TELEMETRY SINGLETON
/// ═══════════════════════════════════════════════════════════════════════════════
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub timestamp: DateTime<Utc>,
    pub total_packets: u64,
    pub total_devices: usize,
    pub devices: HashMap<String, DeviceTelemetryQuick>,
    pub top_devices: Vec<(String, u64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTelemetryQuick {
    pub mac: String,
    pub packet_count: u64,
    pub avg_rssi: f64,
    pub latencies: LatencyStatsQuick,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStatsQuick {
    pub min_ms: u64,
    pub max_ms: u64,
    pub avg_ms: f64,
}

pub static GLOBAL_TELEMETRY: LazyLock<Mutex<TelemetrySnapshot>> = LazyLock::new(|| {
    Mutex::new(TelemetrySnapshot {
        timestamp: Utc::now(),
        total_packets: 0,
        total_devices: 0,
        devices: HashMap::new(),
        top_devices: Vec::new(),
    })
});

/// Update global telemetry snapshot
pub fn update_global_telemetry(snapshot: TelemetrySnapshot) {
    if let Ok(mut global) = GLOBAL_TELEMETRY.lock() {
        *global = snapshot;
    }
}

/// Get current global telemetry snapshot
pub fn get_global_telemetry() -> Option<TelemetrySnapshot> {
    GLOBAL_TELEMETRY.lock().ok().map(|g| g.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_analysis() {
        let timestamps = vec![1000, 1050, 1100, 1150, 1200];
        let analysis = LatencyAnalysis::new("AA:BB:CC:DD:EE:FF".to_string(), timestamps);

        assert_eq!(analysis.inter_packet_latencies_ms, vec![50, 50, 50, 50]);
        assert_eq!(analysis.min_latency_ms, 50);
        assert_eq!(analysis.max_latency_ms, 50);
        assert_eq!(analysis.avg_latency_ms, 50.0);
    }

    #[test]
    fn test_event_categorization() {
        let result = PacketAddResult::Rejected {
            packet_id: 1,
            device_mac: "AA:BB:CC:DD:EE:FF".to_string(),
            reason: "duplicate packet".to_string(),
        };

        let mut collector = TelemetryCollector::new();
        collector.record_packet_result(&result, -75);

        assert_eq!(collector.events.len(), 1);
        assert!(matches!(
            collector.events[0].event_type,
            EventType::PacketDuplicate
        ));
    }
}
