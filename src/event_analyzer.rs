/// Event Analysis - Temporal correlation and event patterns
///
/// Analyzes:
/// - Latency between events
/// - Event patterns and correlations
/// - Device behavior over time
/// - Anomalies and signal degradation

use crate::telemetry::{TimelineEvent, EventType};
use serde::{Serialize, Deserialize};

/// ═══════════════════════════════════════════════════════════════════════════════
/// EVENT PATTERNS
/// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceEventPattern {
    pub device_mac: String,
    pub pattern_type: PatternType,
    pub frequency_hz: f64,          // Events per second
    pub regularity: f64,            // 0.0-1.0 (1.0 = perfectly regular)
    pub confidence: f64,            // 0.0-1.0
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PatternType {
    Regular,           // Consistent interval
    Bursty,           // Clusters of events
    Random,           // Unpredictable
    Degrading,        // Signal getting weaker
    Improving,        // Signal getting stronger
    Intermittent,     // Frequent gaps
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// DEVICE BEHAVIOR ANALYSIS
/// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceBehavior {
    pub device_mac: String,
    pub total_events: usize,
    pub event_duration_ms: u64,
    pub pattern: DeviceEventPattern,
    pub rssi_trend: RssiTrend,
    pub stability_score: f64,      // 0.0-100.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RssiTrend {
    Stable,      // No significant change
    Improving,   // Getting stronger
    Degrading,   // Getting weaker
    Volatile,    // Rapid fluctuations
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// TEMPORAL CORRELATIONS
/// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalCorrelation {
    pub device1: String,
    pub device2: String,
    pub correlation_coefficient: f64,  // -1.0 to 1.0
    pub simultaneous_events: usize,    // Events within 100ms
    pub correlation_strength: CorrelationStrength,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CorrelationStrength {
    None,
    Weak,
    Moderate,
    Strong,
    VeryStrong,
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// EVENT ANOMALIES
/// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventAnomaly {
    pub timestamp_ms: u64,
    pub device_mac: String,
    pub anomaly_type: AnomalyType,
    pub severity: f64,              // 0.0-1.0
    pub description: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AnomalyType {
    GapInTransmission,   // Unexpected silence
    RssiDropout,         // Sudden signal loss
    BurstyBehavior,      // Unusual clustering
    FrequencyChange,     // Different pattern
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// EVENT ANALYZER
/// ═══════════════════════════════════════════════════════════════════════════════

pub struct EventAnalyzer {
    events: Vec<TimelineEvent>,
}

impl EventAnalyzer {
    pub fn new(events: Vec<TimelineEvent>) -> Self {
        Self { events }
    }

    /// Analyze device event patterns
    pub fn analyze_device_pattern(&self, mac_address: &str) -> Option<DeviceBehavior> {
        let device_events: Vec<_> = self
            .events
            .iter()
            .filter(|e| e.device_mac == mac_address && matches!(e.event_type, EventType::PacketReceived))
            .collect();

        if device_events.len() < 2 {
            return None;
        }

        let timestamps: Vec<u64> = device_events.iter().map(|e| e.timestamp_ms).collect();
        let rssi_values: Vec<i8> = device_events.iter().map(|e| e.rssi).collect();

        // Calculate inter-event times
        let mut intervals = Vec::new();
        for i in 1..timestamps.len() {
            if let Some(interval) = timestamps[i].checked_sub(timestamps[i - 1]) {
                intervals.push(interval);
            }
        }

        if intervals.is_empty() {
            return None;
        }

        // Pattern detection
        let avg_interval = intervals.iter().sum::<u64>() as f64 / intervals.len() as f64;
        let variance: f64 = intervals
            .iter()
            .map(|&i| {
                let diff = i as f64 - avg_interval;
                diff * diff
            })
            .sum::<f64>()
            / intervals.len() as f64;
        let std_dev = variance.sqrt();

        // Calculate regularity (1.0 = perfectly regular, 0.0 = random)
        let coefficient_of_variation = if avg_interval > 0.0 {
            std_dev / avg_interval
        } else {
            1.0
        };
        let regularity = (1.0 - coefficient_of_variation.min(1.0)).max(0.0);

        // Determine pattern type
        let pattern_type = if regularity > 0.8 {
            PatternType::Regular
        } else if coefficient_of_variation > 2.0 {
            PatternType::Bursty
        } else {
            PatternType::Random
        };

        // RSSI trend analysis
        let rssi_trend = analyze_rssi_trend(&rssi_values);

        // Stability score based on RSSI variance
        let rssi_values_f64: Vec<f64> = rssi_values.iter().map(|&r| r as f64).collect();
        let rssi_variance = calculate_variance(&rssi_values_f64);
        let stability_score = (100.0 - (rssi_variance / 100.0).min(100.0)).max(0.0);

        let frequency_hz = if avg_interval > 0.0 {
            1000.0 / avg_interval
        } else {
            0.0
        };

        let event_duration_ms = if let (Some(&first), Some(&last)) = (timestamps.first(), timestamps.last()) {
            last - first
        } else {
            0
        };

        Some(DeviceBehavior {
            device_mac: mac_address.to_string(),
            total_events: device_events.len(),
            event_duration_ms,
            pattern: DeviceEventPattern {
                device_mac: mac_address.to_string(),
                pattern_type,
                frequency_hz,
                regularity,
                confidence: 0.85,
            },
            rssi_trend,
            stability_score,
        })
    }

    /// Detect event anomalies
    pub fn detect_anomalies(&self, mac_address: &str) -> Vec<EventAnomaly> {
        let mut anomalies = Vec::new();
        let device_events: Vec<_> = self
            .events
            .iter()
            .filter(|e| e.device_mac == mac_address)
            .collect();

        if device_events.len() < 2 {
            return anomalies;
        }

        let timestamps: Vec<u64> = device_events.iter().map(|e| e.timestamp_ms).collect();
        let rssi_values: Vec<i8> = device_events.iter().map(|e| e.rssi).collect();

        // Detect transmission gaps
        let mut intervals = Vec::new();
        for i in 1..timestamps.len() {
            if let Some(interval) = timestamps[i].checked_sub(timestamps[i - 1]) {
                intervals.push((i, interval));
            }
        }

        let avg_interval = intervals.iter().map(|(_, i)| i).sum::<u64>() as f64 / intervals.len() as f64;
        let expected_gap = avg_interval * 2.5; // Anomaly if 2.5x longer than average

        for (idx, interval) in intervals {
            if interval as f64 > expected_gap {
                anomalies.push(EventAnomaly {
                    timestamp_ms: timestamps[idx],
                    device_mac: mac_address.to_string(),
                    anomaly_type: AnomalyType::GapInTransmission,
                    severity: ((interval as f64 - expected_gap) / expected_gap).min(1.0),
                    description: format!("Gap of {}ms (expected ~{}ms)", interval, avg_interval as u64),
                });
            }
        }

        // Detect RSSI dropouts
        for i in 1..rssi_values.len() {
            let rssi_drop = (rssi_values[i - 1] - rssi_values[i]).abs() as f64;
            if rssi_drop > 20.0 {
                anomalies.push(EventAnomaly {
                    timestamp_ms: timestamps[i],
                    device_mac: mac_address.to_string(),
                    anomaly_type: AnomalyType::RssiDropout,
                    severity: (rssi_drop / 60.0).min(1.0),
                    description: format!("RSSI drop of {}dBm", rssi_drop as i8),
                });
            }
        }

        anomalies
    }

    /// Find correlations between device event patterns
    pub fn find_correlations(&self) -> Vec<TemporalCorrelation> {
        let mut correlations = Vec::new();
        let devices: std::collections::HashSet<String> =
            self.events.iter().map(|e| e.device_mac.clone()).collect();
        let devices: Vec<_> = devices.iter().cloned().collect();

        for i in 0..devices.len() {
            for j in (i + 1)..devices.len() {
                let mac1 = &devices[i];
                let mac2 = &devices[j];

                let events1: Vec<_> = self
                    .events
                    .iter()
                    .filter(|e| &e.device_mac == mac1)
                    .collect();
                let events2: Vec<_> = self
                    .events
                    .iter()
                    .filter(|e| &e.device_mac == mac2)
                    .collect();

                // Count simultaneous events (within 100ms)
                let mut simultaneous = 0;
                for e1 in &events1 {
                    for e2 in &events2 {
                        let diff = if e1.timestamp_ms > e2.timestamp_ms {
                            e1.timestamp_ms - e2.timestamp_ms
                        } else {
                            e2.timestamp_ms - e1.timestamp_ms
                        };
                        if diff <= 100 {
                            simultaneous += 1;
                        }
                    }
                }

                if simultaneous > 0 {
                    let correlation_strength = match simultaneous {
                        0 => CorrelationStrength::None,
                        1..=2 => CorrelationStrength::Weak,
                        3..=5 => CorrelationStrength::Moderate,
                        6..=10 => CorrelationStrength::Strong,
                        _ => CorrelationStrength::VeryStrong,
                    };

                    correlations.push(TemporalCorrelation {
                        device1: mac1.clone(),
                        device2: mac2.clone(),
                        correlation_coefficient: simultaneous as f64 / events1.len().max(events2.len()) as f64,
                        simultaneous_events: simultaneous,
                        correlation_strength,
                    });
                }
            }
        }

        correlations
    }
}

/// Helper: Analyze RSSI trend
fn analyze_rssi_trend(rssi_values: &[i8]) -> RssiTrend {
    if rssi_values.len() < 3 {
        return RssiTrend::Stable;
    }

    let first_third = &rssi_values[..rssi_values.len() / 3];
    let last_third = &rssi_values[rssi_values.len() * 2 / 3..];

    let avg_first = first_third.iter().map(|&r| r as i32).sum::<i32>() / first_third.len() as i32;
    let avg_last = last_third.iter().map(|&r| r as i32).sum::<i32>() / last_third.len() as i32;

    let rssi_values_f64: Vec<f64> = rssi_values.iter().map(|&r| r as f64).collect();
    let rssi_variance = calculate_variance(&rssi_values_f64);

    if rssi_variance > 15.0 {
        RssiTrend::Volatile
    } else if avg_last > avg_first + 5 {
        RssiTrend::Improving
    } else if avg_last < avg_first - 5 {
        RssiTrend::Degrading
    } else {
        RssiTrend::Stable
    }
}

/// Helper: Calculate variance
fn calculate_variance(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|&v| (v - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rssi_trend_improving() {
        let rssi = vec![-80, -75, -70, -65, -60];
        assert!(matches!(analyze_rssi_trend(&rssi), RssiTrend::Improving));
    }

    #[test]
    fn test_rssi_trend_degrading() {
        let rssi = vec![-40, -50, -60, -70, -80];
        assert!(matches!(analyze_rssi_trend(&rssi), RssiTrend::Degrading));
    }
}
