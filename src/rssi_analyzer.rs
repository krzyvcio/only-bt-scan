/// RSSI Trend Analysis Engine
/// Real-time signal strength tracking for all BLE devices
///
/// Implements:
/// - Exponential Moving Average (EMA) for signal smoothing
/// - Linear regression for trend detection
/// - Motion classification (Still/Moving)
/// - Variance analysis for signal stability
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Configuration parameters for RSSI analysis
///
/// Tuning parameters for the RSSI trend detection algorithm:
/// - Window size: Number of samples to keep for analysis
/// - EMA alpha: Smoothing factor (higher = more responsive to changes)
/// - Slope epsilon: Threshold for trend detection
/// - Variance epsilon: Threshold for motion detection
/// - Min samples: Minimum samples before making predictions
#[derive(Clone, Copy, Debug)]
pub struct AnalysisConfig {
    /// Window size for trend analysis (number of samples)
    pub window_size: usize,
    /// EMA smoothing factor (0.0 - 1.0), higher = more responsive
    pub ema_alpha: f64,
    /// Slope threshold for trend detection (dB/sec)
    pub slope_epsilon: f64,
    /// Variance threshold for motion detection (dB²)
    pub variance_epsilon: f64,
    /// Minimum samples before making decision
    pub min_samples: usize,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            window_size: 20,
            ema_alpha: 0.3,
            slope_epsilon: 0.15,
            variance_epsilon: 2.0,
            min_samples: 6,
        }
    }
}

/// Single RSSI measurement
///
/// Represents a single point in the RSSI time series.
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    /// Timestamp in seconds (from start or Unix epoch)
    pub t: f64,
    /// Smoothed RSSI value in dBm
    pub rssi: f64,
}

/// Trend classification
///
/// Direction of device movement relative to scanner:
/// - `Approaching`: RSSI increasing (device getting closer)
/// - `Leaving`: RSSI decreasing (device moving away)
/// - `Stable`: RSSI relatively constant
/// - `Unknown`: Not enough data to determine
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Trend {
    #[serde(rename = "approaching")]
    Approaching,
    #[serde(rename = "leaving")]
    Leaving,
    #[serde(rename = "stable")]
    Stable,
    #[serde(rename = "unknown")]
    Unknown,
}

impl std::fmt::Display for Trend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Trend::Approaching => write!(f, "approaching"),
            Trend::Leaving => write!(f, "leaving"),
            Trend::Stable => write!(f, "stable"),
            Trend::Unknown => write!(f, "unknown"),
        }
    }
}

/// Motion classification
///
/// Whether device is stationary or in motion:
/// - `Still`: Low variance in RSSI (device stationary)
/// - `Moving`: High variance or trend changes (device moving)
/// - `Unknown`: Not enough data to determine
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Motion {
    #[serde(rename = "still")]
    Still,
    #[serde(rename = "moving")]
    Moving,
    #[serde(rename = "unknown")]
    Unknown,
}

impl std::fmt::Display for Motion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Motion::Still => write!(f, "still"),
            Motion::Moving => write!(f, "moving"),
            Motion::Unknown => write!(f, "unknown"),
        }
    }
}

/// Complete state of a device at a point in time
///
/// Computed analysis results from RSSI time series:
/// - `trend`: Movement direction (approaching/leaving/stable)
/// - `motion`: Whether device is still or moving
/// - `slope`: Rate of RSSI change in dB/sec
/// - `variance`: Signal stability in dB²
/// - `rssi`: Current smoothed RSSI in dBm
/// - `sample_count`: Number of samples used for analysis
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct DeviceState {
    pub trend: Trend,
    pub motion: Motion,
    pub slope: f64,    // dB/sec - rate of RSSI change
    pub variance: f64, // dB² - signal stability
    pub rssi: f64,     // Current smoothed RSSI in dBm
    pub sample_count: usize,
}

/// Real-time tracking for a single device
///
/// Maintains sliding window of RSSI samples and computes
/// trend/motion analysis on each update.
///
/// Fields:
/// - `id`: MAC address of tracked device
/// - `window`: Sliding window of recent RSSI samples
/// - `last_rssi_smooth`: Last EMA-smoothed RSSI value
/// - `config`: Analysis configuration parameters
/// - `last_update`: Timestamp of last sample
pub struct DeviceTracker {
    pub id: String, // MAC address
    pub window: VecDeque<Sample>,
    pub last_rssi_smooth: Option<f64>,
    pub config: AnalysisConfig,
    pub last_update: DateTime<Utc>,
}

impl DeviceTracker {
    /// Create new tracker for a device
    ///
    /// # Arguments
    /// * `id` - MAC address or unique identifier for device
    /// * `config` - Analysis configuration parameters
    ///
    /// # Returns
    /// New DeviceTracker instance
    pub fn new(id: String, config: AnalysisConfig) -> Self {
        Self {
            id,
            window: VecDeque::with_capacity(config.window_size),
            last_rssi_smooth: None,
            config,
            last_update: Utc::now(),
        }
    }

    /// Update tracker with new RSSI measurement
    ///
    /// Processes new RSSI sample through the analysis pipeline:
    /// 1. Apply EMA smoothing
    /// 2. Convert timestamp to relative seconds
    /// 3. Add to sliding window (evict oldest if full)
    /// 4. Compute current device state
    ///
    /// # Arguments
    /// * `rssi` - Raw RSSI measurement in dBm
    /// * `timestamp` - When measurement was taken
    ///
    /// # Returns
    /// Computed DeviceState with trend, motion, and statistics
    pub fn update(&mut self, rssi: i8, timestamp: DateTime<Utc>) -> DeviceState {
        let rssi_f = rssi as f64;

        // Step 1: EMA smoothing
        let rssi_smooth = match self.last_rssi_smooth {
            None => rssi_f,
            Some(prev) => self.config.ema_alpha * rssi_f + (1.0 - self.config.ema_alpha) * prev,
        };
        self.last_rssi_smooth = Some(rssi_smooth);

        // Step 2: Convert timestamp to seconds from first sample
        let t_seconds = if let Some(first_sample) = self.window.front() {
            (timestamp.timestamp_millis() as f64 - first_sample.t * 1000.0) / 1000.0
        } else {
            timestamp.timestamp_millis() as f64 / 1000.0
        };

        // Step 3: Add sample to window
        let sample = Sample {
            t: t_seconds,
            rssi: rssi_smooth,
        };

        if self.window.len() == self.config.window_size {
            self.window.pop_front();
        }
        self.window.push_back(sample);
        self.last_update = timestamp;

        // Step 4: Compute state if enough samples
        self.compute_state()
    }

    /// Compute current device state
    ///
    /// Analyzes sliding window to determine trend and motion.
    /// Requires minimum samples configured in AnalysisConfig.
    ///
    /// # Returns
    /// DeviceState with computed trend, motion, slope, variance
    fn compute_state(&self) -> DeviceState {
        let sample_count = self.window.len();

        if sample_count < self.config.min_samples {
            return DeviceState {
                trend: Trend::Unknown,
                motion: Motion::Unknown,
                slope: 0.0,
                variance: 0.0,
                rssi: self.last_rssi_smooth.unwrap_or(0.0),
                sample_count,
            };
        }

        // Compute slope (linear regression)
        let slope = compute_slope(&self.window);

        // Compute variance
        let variance = compute_variance(&self.window);

        // Classify trend
        let trend = if slope > self.config.slope_epsilon {
            Trend::Approaching
        } else if slope < -self.config.slope_epsilon {
            Trend::Leaving
        } else {
            Trend::Stable
        };

        // Classify motion
        let motion =
            if variance < self.config.variance_epsilon && slope.abs() < self.config.slope_epsilon {
                Motion::Still
            } else {
                Motion::Moving
            };

        DeviceState {
            trend,
            motion,
            slope,
            variance,
            rssi: self.last_rssi_smooth.unwrap_or(0.0),
            sample_count,
        }
    }
}

/// Compute linear regression slope
///
/// Uses least squares to find rate of RSSI change over time.
/// Positive = approaching, Negative = leaving.
///
/// # Arguments
/// * `samples` - Deque of Sample points
///
/// # Returns
/// Slope in dB/sec
fn compute_slope(samples: &VecDeque<Sample>) -> f64 {
    let n = samples.len() as f64;

    let mut sum_t = 0.0;
    let mut sum_r = 0.0;
    let mut sum_tt = 0.0;
    let mut sum_tr = 0.0;

    for s in samples {
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

/// Compute variance of RSSI values
///
/// Measures signal stability - higher variance indicates
/// more fluctuation in signal strength.
///
/// # Arguments
/// * `samples` - Deque of Sample points
///
/// # Returns
/// Variance of RSSI values
fn compute_variance(samples: &VecDeque<Sample>) -> f64 {
    let n = samples.len() as f64;
    if n == 0.0 {
        return 0.0;
    }

    let mean = samples.iter().map(|s| s.rssi).sum::<f64>() / n;

    samples
        .iter()
        .map(|s| {
            let d = s.rssi - mean;
            d * d
        })
        .sum::<f64>()
        / n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema_smoothing() {
        let config = AnalysisConfig {
            ema_alpha: 0.5,
            ..Default::default()
        };
        let mut tracker = DeviceTracker::new("AA:BB:CC:DD:EE:FF".to_string(), config);

        let now = Utc::now();
        let _state1 = tracker.update(-50, now);
        let _state2 = tracker.update(-70, now);

        // Second smoothed RSSI should be 0.5 * -70 + 0.5 * -50 = -60
        assert!((tracker.last_rssi_smooth.unwrap() - (-60.0)).abs() < 0.01);
    }

    #[test]
    fn test_trend_approaching() {
        let config = AnalysisConfig {
            window_size: 20,
            ema_alpha: 0.3,
            slope_epsilon: 0.1,
            ..Default::default()
        };
        let mut tracker = DeviceTracker::new("AA:BB:CC:DD:EE:FF".to_string(), config);

        let mut now = Utc::now();
        // Simulate device getting closer (RSSI increasing)
        for i in 0..10 {
            let rssi = (-80 + i) as i8;
            tracker.update(rssi, now);
            now = now + chrono::Duration::seconds(1);
        }

        let state = tracker.compute_state();
        assert!(
            state.slope > 0.0,
            "Should have positive slope (approaching)"
        );
    }

    #[test]
    fn test_trend_leaving() {
        let config = AnalysisConfig {
            window_size: 20,
            ema_alpha: 0.3,
            slope_epsilon: 0.1,
            ..Default::default()
        };
        let mut tracker = DeviceTracker::new("AA:BB:CC:DD:EE:FF".to_string(), config);

        let mut now = Utc::now();
        // Simulate device moving away (RSSI decreasing)
        for i in (0..10).rev() {
            let rssi = (-80 + i) as i8;
            tracker.update(rssi, now);
            now = now + chrono::Duration::seconds(1);
        }

        let state = tracker.compute_state();
        assert!(state.slope < 0.0, "Should have negative slope (leaving)");
    }
}
