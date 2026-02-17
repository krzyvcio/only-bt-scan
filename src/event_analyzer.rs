/// Event Analysis - Temporal correlation and event patterns
///
/// Analyzes:
/// - Latency between events
/// - Event patterns and correlations
/// - Device behavior over time
/// - Anomalies and signal degradation
///
/// # How it works:
/// 1. Events are collected from ScannerWithTracking during BLE scanning
/// 2. Each packet received creates a TimelineEvent with RSSI, timestamp, MAC
/// 3. analyze_device_behavior() splits events into thirds and compares avg RSSI
///    - If last third RSSI > first third + 5dBm → Improving (device approaching)
///    - If last third RSSI < first third - 5dBm → Degrading (device moving away)
///    - Otherwise → Stable
/// 4. detect_anomalies() finds gaps > 2.5x avg interval and RSSI drops > 20dBm
/// 5. find_correlations() finds devices with events within 100ms of each other
use crate::telemetry::{EventType, TimelineEvent};
use serde::{Deserialize, Serialize};

/// Global singleton storing all timeline events from BLE scanning
/// Uses LazyLock + Mutex for thread-safe access across async tasks
static EVENT_ANALYZER: std::sync::LazyLock<std::sync::Mutex<EventAnalyzerState>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(EventAnalyzerState::new()));

/// Internal state holding collected events with size limit (10k events max)
pub struct EventAnalyzerState {
    events: Vec<TimelineEvent>,
    max_events: usize,
}

impl EventAnalyzerState {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            max_events: 10000,
        }
    }

    /// Adds new events to the timeline, removes oldest if over limit
    pub fn add_events(&mut self, new_events: Vec<TimelineEvent>) {
        self.events.extend(new_events);
        if self.events.len() > self.max_events {
            let excess = self.events.len() - self.max_events;
            self.events.drain(0..excess);
        }
    }

    pub fn get_events(&self) -> &[TimelineEvent] {
        &self.events
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Analyzes behavior for specific device MAC
    pub fn analyze_device(&self, mac: &str) -> Option<DeviceBehavior> {
        let analyzer = EventAnalyzer::new(self.events.clone());
        analyzer.analyze_device_pattern(mac)
    }

    /// Detects anomalies for specific device
    pub fn detect_device_anomalies(&self, mac: &str) -> Vec<EventAnomaly> {
        let analyzer = EventAnalyzer::new(self.events.clone());
        analyzer.detect_anomalies(mac)
    }

    /// Finds temporal correlations between ALL devices
    pub fn find_all_correlations(&self) -> Vec<TemporalCorrelation> {
        let analyzer = EventAnalyzer::new(self.events.clone());
        analyzer.find_correlations()
    }
}

/// Adds new timeline events to the global analyzer
/// Called from ScannerWithTracking when packets are accepted
/// # Arguments
/// * `events` - Vector of TimelineEvent (usually 1 event per accepted packet)
pub fn add_timeline_events(events: Vec<TimelineEvent>) {
    if let Ok(mut state) = EVENT_ANALYZER.lock() {
        state.add_events(events);
    }
}

/// Analyzes device behavior pattern including RSSI trend
/// # Returns
/// Some(DeviceBehavior) with:
/// - `rssi_trend`: Improving/Degrading/Stable/Volatile
/// - `pattern_type`: Regular/Bursty/Random
/// - `stability_score`: 0-100 based on RSSI variance
/// # Algorithm
/// Splits device events into 3 time windows, compares average RSSI:
/// - Improving: last window avg > first window avg + 5 dBm
/// - Degrading: last window avg < first window avg - 5 dBm
/// - Stable: difference within ±5 dBm
/// - Volatile: variance > 15.0
pub fn analyze_device_behavior(mac: &str) -> Option<DeviceBehavior> {
    EVENT_ANALYZER
        .lock()
        .ok()
        .and_then(|s| s.analyze_device(mac))
}

/// Detects anomalies in device event stream:
/// 1. Gap in transmission: interval > 2.5x average interval
/// 2. RSSI dropout: sudden signal drop > 20 dBm
/// # Returns
/// Vector of EventAnomaly with timestamp, type, severity (0-1), description
pub fn detect_anomalies(mac: &str) -> Vec<EventAnomaly> {
    EVENT_ANALYZER
        .lock()
        .ok()
        .map(|s| s.detect_device_anomalies(mac))
        .unwrap_or_default()
}

/// Finds pairs of devices with correlated event timing
/// # Algorithm
/// For each device pair, counts events within 100ms of each other
/// Returns correlation strength: None/Weak/Moderate/Strong/VeryStrong
/// Useful for finding devices that are used together or near each other
pub fn find_correlations() -> Vec<TemporalCorrelation> {
    EVENT_ANALYZER
        .lock()
        .ok()
        .map(|s| s.find_all_correlations())
        .unwrap_or_default()
}

/// Returns total number of events in the timeline
pub fn get_event_count() -> usize {
    EVENT_ANALYZER.lock().map(|s| s.events.len()).unwrap_or(0)
}

/// Clears all events from the analyzer (useful for starting fresh scan)
pub fn clear_events() {
    if let Ok(mut state) = EVENT_ANALYZER.lock() {
        state.clear();
    }
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// EVENT PATTERNS
/// ═══════════════════════════════════════════════════════════════════════════════

/// Event patterns
///
/// Classification of device advertising behavior:
/// - `Regular`: Consistent packet intervals
/// - `Bursty`: Clusters of packets with gaps
/// - `Random`: Unpredictable timing
/// - `Degrading`: Signal getting weaker over time
/// - `Improving`: Signal getting stronger over time
/// - `Intermittent`: Frequent gaps in transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceEventPattern {
    /// MAC address of device
    pub device_mac: String,
    /// Type of advertising pattern
    pub pattern_type: PatternType,
    /// Events per second
    pub frequency_hz: f64,
    /// Regularity score (0.0-1.0, 1.0 = perfectly regular)
    pub regularity: f64,
    /// Analysis confidence (0.0-1.0)
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PatternType {
    /// Consistent interval between events
    Regular,
    /// Clusters of events with gaps
    Bursty,
    /// Unpredictable timing
    Random,
    /// Signal getting weaker
    Degrading,
    /// Signal getting stronger
    Improving,
    /// Frequent gaps
    Intermittent,
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// DEVICE BEHAVIOR ANALYSIS
/// ═══════════════════════════════════════════════════════════════════════════════

/// Device behavior analysis results
///
/// Complete behavioral analysis for a device:
/// - `device_mac`: MAC address
/// - `total_events`: Count of events observed
/// - `event_duration_ms`: Time span of observations
/// - `pattern`: Detected advertising pattern
/// - `rssi_trend`: Signal strength trend
/// - `stability_score`: Signal stability (0-100)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceBehavior {
    pub device_mac: String,
    pub total_events: usize,
    pub event_duration_ms: u64,
    pub pattern: DeviceEventPattern,
    pub rssi_trend: RssiTrend,
    /// Signal stability score (0.0-100.0)
    pub stability_score: f64,
}

/// RSSI trend classification
///
/// Direction of signal strength change:
/// - `Stable`: No significant change
/// - `Improving`: Signal getting stronger
/// - `Degrading`: Signal getting weaker
/// - `Volatile`: Rapid fluctuations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RssiTrend {
    Stable,
    Improving,
    Degrading,
    Volatile,
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// TEMPORAL CORRELATIONS
/// ═══════════════════════════════════════════════════════════════════════════════

/// Temporal correlation between device event patterns
///
/// Represents devices that tend to be active at similar times:
/// - `device1`, `device2`: MAC addresses of correlated devices
/// - `correlation_coefficient`: Similarity measure (-1.0 to 1.0)
/// - `simultaneous_events`: Count of events within 100ms
/// - `correlation_strength`: Categorical strength rating
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalCorrelation {
    pub device1: String,
    pub device2: String,
    /// Correlation coefficient (-1.0 to 1.0)
    pub correlation_coefficient: f64,
    /// Events within 100ms of each other
    pub simultaneous_events: usize,
    pub correlation_strength: CorrelationStrength,
}

/// Correlation strength classification
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

/// Anomaly detected in device event stream
///
/// Represents unusual patterns:
/// - `timestamp_ms`: When anomaly occurred
/// - `device_mac`: Device with anomaly
/// - `anomaly_type`: Type of anomaly
/// - `severity`: How severe (0.0-1.0)
/// - `description`: Human-readable explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventAnomaly {
    pub timestamp_ms: u64,
    pub device_mac: String,
    pub anomaly_type: AnomalyType,
    /// Severity of anomaly (0.0-1.0)
    pub severity: f64,
    pub description: String,
}

/// Types of anomalies that can be detected
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AnomalyType {
    /// Unexpected gap in transmission (>2.5x avg interval)
    GapInTransmission,
    /// Sudden RSSI drop (>20dBm)
    RssiDropout,
    /// Unusual clustering of events
    BurstyBehavior,
    /// Change in advertising pattern
    FrequencyChange,
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// EVENT ANALYZER
/// ═══════════════════════════════════════════════════════════════════════════════

/// Event analyzer for temporal pattern analysis
///
/// Performs device behavior analysis, anomaly detection,
/// and correlation finding on timeline events.
pub struct EventAnalyzer {
    events: Vec<TimelineEvent>,
}

impl EventAnalyzer {
    /// Create new analyzer with events
    ///
    /// # Arguments
    /// * `events` - Timeline events to analyze
    ///
    /// # Returns
    /// New EventAnalyzer instance
    pub fn new(events: Vec<TimelineEvent>) -> Self {
        Self { events }
    }

    /// Analyze device event patterns
    ///
    /// Computes behavioral analysis including pattern type, frequency,
    /// regularity, RSSI trend, and stability score.
    ///
    /// # Arguments
    /// * `mac_address` - Device MAC to analyze
    ///
    /// # Returns
    /// Some(DeviceBehavior) if enough events, None otherwise
    pub fn analyze_device_pattern(&self, mac_address: &str) -> Option<DeviceBehavior> {
        let device_events: Vec<_> = self
            .events
            .iter()
            .filter(|e| {
                e.device_mac == mac_address && matches!(e.event_type, EventType::PacketReceived)
            })
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

        let event_duration_ms =
            if let (Some(&first), Some(&last)) = (timestamps.first(), timestamps.last()) {
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
    ///
    /// Finds unusual patterns in device event stream:
    /// - Gap in transmission: interval > 2.5x average
    /// - RSSI dropout: sudden drop > 20dBm
    ///
    /// # Arguments
    /// * `mac_address` - Device MAC to analyze
    ///
    /// # Returns
    /// Vector of detected anomalies
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

        let avg_interval =
            intervals.iter().map(|(_, i)| i).sum::<u64>() as f64 / intervals.len() as f64;
        let expected_gap = avg_interval * 2.5; // Anomaly if 2.5x longer than average

        for (idx, interval) in intervals {
            if interval as f64 > expected_gap {
                anomalies.push(EventAnomaly {
                    timestamp_ms: timestamps[idx],
                    device_mac: mac_address.to_string(),
                    anomaly_type: AnomalyType::GapInTransmission,
                    severity: ((interval as f64 - expected_gap) / expected_gap).min(1.0),
                    description: format!(
                        "Gap of {}ms (expected ~{}ms)",
                        interval, avg_interval as u64
                    ),
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
    ///
    /// Identifies devices that tend to be active at similar times.
    /// Considers events within 100ms of each other as "simultaneous".
    ///
    /// # Returns
    /// Vector of temporal correlations between device pairs
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
                        correlation_coefficient: simultaneous as f64
                            / events1.len().max(events2.len()) as f64,
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
///
/// Compares average RSSI in first vs last third of samples.
/// High variance indicates volatile signal.
///
/// # Arguments
/// * `rssi_values` - Slice of RSSI measurements
///
/// # Returns
/// RssiTrend classification
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
///
/// Computes standard deviation variance for a slice of values.
///
/// # Arguments
/// * `values` - Slice of numeric values
///
/// # Returns
/// Standard deviation variance
fn calculate_variance(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
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
