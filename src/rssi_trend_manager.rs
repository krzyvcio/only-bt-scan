/// Global RSSI Trend Manager
/// Manages DeviceTracker instances for all discovered BLE devices
/// Thread-safe using Arc<Mutex<>>
use crate::rssi_analyzer::{AnalysisConfig, DeviceState, DeviceTracker, Motion, Trend};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Aggregated snapshot of all device states
///
/// Contains timestamp and list of all device RSSI information
/// at a point in time. Useful for JSON serialization/APIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalRssiSnapshot {
    /// Timestamp when snapshot was taken
    pub timestamp: DateTime<Utc>,
    /// List of per-device RSSI information
    pub devices: Vec<DeviceRssiInfo>,
}

/// Per-device RSSI trend information
///
/// Compact representation of device state for API/serialization:
/// - `mac`: MAC address
/// - `rssi`: Current smoothed RSSI
/// - `trend`: Movement direction (string)
/// - `motion`: Still/moving (string)
/// - `slope`: Rate of RSSI change
/// - `variance`: Signal stability measure
/// - `sample_count`: Number of samples analyzed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRssiInfo {
    pub mac: String,
    pub rssi: f64,
    pub trend: String,
    pub motion: String,
    pub slope: f64,
    pub variance: f64,
    pub sample_count: usize,
}

/// Global manager for all device RSSI trackers
///
/// Thread-safe manager that maintains DeviceTracker instances
/// for all discovered BLE devices. Provides filtering by trend/motion.
///
/// Uses Arc<Mutex<>> for safe concurrent access from async tasks.
pub struct GlobalRssiManager {
    trackers: Arc<Mutex<HashMap<String, DeviceTracker>>>,
    config: AnalysisConfig,
}

impl GlobalRssiManager {
    /// Create a new global manager
    ///
    /// # Arguments
    /// * `config` - Analysis configuration parameters
    ///
    /// # Returns
    /// Arc-wrapped GlobalRssiManager
    pub fn new(config: AnalysisConfig) -> Arc<Self> {
        Arc::new(Self {
            trackers: Arc::new(Mutex::new(HashMap::new())),
            config,
        })
    }

    /// Create with default config
    ///
    /// # Returns
    /// Arc-wrapped GlobalRssiManager with default AnalysisConfig
    pub fn default() -> Arc<Self> {
        Self::new(AnalysisConfig::default())
    }

    /// Update RSSI for a device (creates tracker if needed)
    ///
    /// Adds new RSSI measurement for device, creating tracker if first observation.
    ///
    /// # Arguments
    /// * `mac` - MAC address of device
    /// * `rssi` - RSSI measurement in dBm
    /// * `timestamp` - When measurement was taken
    ///
    /// # Returns
    /// Computed DeviceState after adding sample
    pub fn update_rssi(&self, mac: &str, rssi: i8, timestamp: DateTime<Utc>) -> DeviceState {
        let mut trackers = self.trackers.lock().unwrap();

        let tracker = trackers
            .entry(mac.to_string())
            .or_insert_with(|| DeviceTracker::new(mac.to_string(), self.config));

        tracker.update(rssi, timestamp)
    }

    /// Get current state for a specific device
    ///
    /// # Arguments
    /// * `mac` - MAC address to look up
    ///
    /// # Returns
    /// Some(DeviceState) if device tracked, None otherwise
    pub fn get_device_state(&self, mac: &str) -> Option<DeviceState> {
        let trackers = self.trackers.lock().unwrap();
        trackers.get(mac).map(|t| {
            // Call compute_state on immutable reference
            // We need to recompute from window
            compute_device_state_from_tracker(t, &self.config)
        })
    }

    /// Get all device states
    ///
    /// # Returns
    /// Vector of (MAC, DeviceState) tuples for all tracked devices
    pub fn get_all_states(&self) -> Vec<(String, DeviceState)> {
        let trackers = self.trackers.lock().unwrap();
        trackers
            .iter()
            .map(|(mac, tracker)| {
                let state = compute_device_state_from_tracker(tracker, &self.config);
                (mac.clone(), state)
            })
            .collect()
    }

    /// Get snapshot of all devices
    ///
    /// Creates aggregated snapshot suitable for JSON serialization.
    ///
    /// # Returns
    /// GlobalRssiSnapshot with all device RSSI info
    pub fn get_snapshot(&self) -> GlobalRssiSnapshot {
        let devices = self
            .get_all_states()
            .into_iter()
            .map(|(mac, state)| DeviceRssiInfo {
                mac,
                rssi: state.rssi,
                trend: format!("{}", state.trend),
                motion: format!("{}", state.motion),
                slope: state.slope,
                variance: state.variance,
                sample_count: state.sample_count,
            })
            .collect();

        GlobalRssiSnapshot {
            timestamp: Utc::now(),
            devices,
        }
    }

    /// Get devices filtering by trend
    ///
    /// # Arguments
    /// * `trend` - Trend to filter by
    ///
    /// # Returns
    /// Vector of (MAC, DeviceState) matching the trend
    pub fn get_by_trend(&self, trend: Trend) -> Vec<(String, DeviceState)> {
        self.get_all_states()
            .into_iter()
            .filter(|(_, state)| state.trend == trend)
            .collect()
    }

    /// Get devices filtering by motion
    ///
    /// # Arguments
    /// * `motion` - Motion state to filter by
    ///
    /// # Returns
    /// Vector of (MAC, DeviceState) matching the motion state
    pub fn get_by_motion(&self, motion: Motion) -> Vec<(String, DeviceState)> {
        self.get_all_states()
            .into_iter()
            .filter(|(_, state)| state.motion == motion)
            .collect()
    }

    /// Get devices that are approaching
    ///
    /// # Returns
    /// Vector of (MAC, DeviceState) with Approaching trend
    pub fn get_approaching(&self) -> Vec<(String, DeviceState)> {
        self.get_by_trend(Trend::Approaching)
    }

    /// Get devices that are leaving
    ///
    /// # Returns
    /// Vector of (MAC, DeviceState) with Leaving trend
    pub fn get_leaving(&self) -> Vec<(String, DeviceState)> {
        self.get_by_trend(Trend::Leaving)
    }

    /// Get devices that are still
    ///
    /// # Returns
    /// Vector of (MAC, DeviceState) with Still motion
    pub fn get_still(&self) -> Vec<(String, DeviceState)> {
        self.get_by_motion(Motion::Still)
    }

    /// Get devices that are moving
    ///
    /// # Returns
    /// Vector of (MAC, DeviceState) with Moving motion
    pub fn get_moving(&self) -> Vec<(String, DeviceState)> {
        self.get_by_motion(Motion::Moving)
    }

    /// Clear all trackers (reset)
    ///
    /// Removes all tracked devices and their RSSI history.
    pub fn clear(&self) {
        self.trackers.lock().unwrap().clear();
    }

    /// Get number of tracked devices
    ///
    /// # Returns
    /// Count of devices currently being tracked
    pub fn device_count(&self) -> usize {
        self.trackers.lock().unwrap().len()
    }
}

/// Helper to compute state from tracker (immutable access)
///
/// Recomputes device state from tracker's window without mutation.
/// Used for read-only state queries.
///
/// # Arguments
/// * `tracker` - DeviceTracker to analyze
/// * `config` - Analysis configuration
///
/// # Returns
/// Computed DeviceState
fn compute_device_state_from_tracker(
    tracker: &DeviceTracker,
    config: &AnalysisConfig,
) -> DeviceState {
    let sample_count = tracker.window.len();

    if sample_count < config.min_samples {
        return DeviceState {
            trend: Trend::Unknown,
            motion: Motion::Unknown,
            slope: 0.0,
            variance: 0.0,
            rssi: tracker.last_rssi_smooth.unwrap_or(0.0),
            sample_count,
        };
    }

    // Compute slope
    let slope = compute_slope_from_window(&tracker.window);

    // Compute variance
    let variance = compute_variance_from_window(&tracker.window);

    // Classify trend
    let trend = if slope > config.slope_epsilon {
        Trend::Approaching
    } else if slope < -config.slope_epsilon {
        Trend::Leaving
    } else {
        Trend::Stable
    };

    // Classify motion
    let motion = if variance < config.variance_epsilon && slope.abs() < config.slope_epsilon {
        Motion::Still
    } else {
        Motion::Moving
    };

    DeviceState {
        trend,
        motion,
        slope,
        variance,
        rssi: tracker.last_rssi_smooth.unwrap_or(0.0),
        sample_count,
    }
}

/// Compute slope from window (helper)
///
/// Linear regression to find RSSI rate of change.
///
/// # Arguments
/// * `window` - Deque of RSSI samples
///
/// # Returns
/// Slope in dB/sec
fn compute_slope_from_window(
    window: &std::collections::VecDeque<crate::rssi_analyzer::Sample>,
) -> f64 {
    let n = window.len() as f64;

    let mut sum_t = 0.0;
    let mut sum_r = 0.0;
    let mut sum_tt = 0.0;
    let mut sum_tr = 0.0;

    for s in window {
        sum_t += s.t;
        sum_r += s.rssi;
        sum_tt += s.t * s.t;
        sum_tr += s.t * s.rssi;
    }

    let denom = n * sum_tt - sum_t * sum_t;
    if denom.abs() < 1e-9 {
        return 0.0;
    }

    (n * sum_tr - sum_t * sum_r) / denom
}

/// Compute variance from window (helper)
///
/// Measures signal stability in RSSI time series.
///
/// # Arguments
/// * `window` - Deque of RSSI samples
///
/// # Returns
/// Variance of RSSI values
fn compute_variance_from_window(
    window: &std::collections::VecDeque<crate::rssi_analyzer::Sample>,
) -> f64 {
    let n = window.len() as f64;
    if n == 0.0 {
        return 0.0;
    }

    let mean = window.iter().map(|s| s.rssi).sum::<f64>() / n;

    window
        .iter()
        .map(|s| {
            let d = s.rssi - mean;
            d * d
        })
        .sum::<f64>()
        / n
}

// Global instance (lazy-initialized)
lazy_static::lazy_static! {
    pub static ref GLOBAL_RSSI_MANAGER: Arc<GlobalRssiManager> = GlobalRssiManager::default();
}
