/// Device Discovery Tracker
///
/// Tracks all device discoveries with:
/// - First detection timestamp
/// - Last detection timestamp
/// - Detection count per MAC address
/// - Verbose terminal logging
/// - Database persistence
/// - RSSI telemetry (trend, motion detection)
use chrono::{DateTime, Utc};
use colored::Colorize;
use log::{debug, info};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::company_id_reference;
use crate::db::{self, ScannedDevice};

/// Telemetry constants
const TELEMETRY_WINDOW_SIZE: usize = 20;
const TELEMETRY_EMA_ALPHA: f64 = 0.3;
const TELEMETRY_SLOPE_EPS: f64 = 0.15;
const TELEMETRY_VAR_EPS: f64 = 2.0;
const TELEMETRY_MIN_SAMPLES: usize = 6;

/// Single RSSI sample with timestamp for telemetry
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub t: f64,
    pub rssi: f64,
}

/// Ring buffer for samples
#[derive(Debug, Clone)]
pub struct SampleWindow {
    samples: VecDeque<Sample>,
    max_size: usize,
}

impl SampleWindow {
    pub fn new(max_size: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    pub fn push(&mut self, s: Sample) {
        if self.samples.len() == self.max_size {
            self.samples.pop_front();
        }
        self.samples.push_back(s);
    }

    pub fn samples(&self) -> &VecDeque<Sample> {
        &self.samples
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

/// Movement trend relative to antenna
///
/// - `Approaching`: Device signal strength increasing (getting closer)
/// - `Leaving`: Device signal strength decreasing (moving away)
/// - `Stable`: Signal strength relatively constant
/// - `Unknown`: Not enough samples to determine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    Approaching,
    Leaving,
    Stable,
    Unknown,
}

impl Trend {
    pub fn as_str(&self) -> &'static str {
        match self {
            Trend::Approaching => "approaching",
            Trend::Leaving => "leaving",
            Trend::Stable => "stable",
            Trend::Unknown => "unknown",
        }
    }
}

/// Motion state
///
/// - `Still`: Device is stationary (low variance in RSSI)
/// - `Moving`: Device is in motion (high variance or changing trend)
/// - `Unknown`: Not enough samples to determine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Still,
    Moving,
    Unknown,
}

impl Motion {
    pub fn as_str(&self) -> &'static str {
        match self {
            Motion::Still => "still",
            Motion::Moving => "moving",
            Motion::Unknown => "unknown",
        }
    }
}

/// Device telemetry state output
///
/// Contains computed state from RSSI telemetry including:
/// - `trend`: Movement direction (approaching/leaving/stable)
/// - `motion`: Whether device is still or moving
/// - `slope`: Rate of RSSI change (dB/sec)
/// - `variance`: Signal stability measure (dB²)
/// - `rssi`: Current smoothed RSSI value
/// - `confidence`: How reliable the state estimate is (0-1)
#[derive(Debug, Clone)]
pub struct DeviceState {
    pub trend: Trend,
    pub motion: Motion,
    pub slope: f64,
    pub variance: f64,
    pub rssi: f64,
    pub confidence: f64,
}

impl Default for DeviceState {
    fn default() -> Self {
        Self {
            trend: Trend::Unknown,
            motion: Motion::Unknown,
            slope: 0.0,
            variance: 0.0,
            rssi: 0.0,
            confidence: 0.0,
        }
    }
}

/// Compute linear regression slope (dRSSI/dt)
///
/// Uses least squares regression to find rate of RSSI change over time.
/// Positive slope = device approaching, Negative = device leaving.
///
/// # Arguments
/// * `samples` - Deque of RSSI samples with timestamps
///
/// # Returns
/// Slope in dB/sec (rate of RSSI change)
fn compute_slope(samples: &VecDeque<Sample>) -> f64 {
    let n = samples.len() as f64;
    if n < 2.0 {
        return 0.0;
    }

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
/// Measures signal stability - low variance indicates steady signal,
/// high variance indicates unstable/intermittent signal.
///
/// # Arguments
/// * `samples` - Deque of RSSI samples
///
/// # Returns
/// Variance of RSSI values in dB²
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

/// Single device tracking record
///
/// Tracks all discovery events for a single BLE device with:
/// - MAC address and name/manufacturer identification
/// - Temporal tracking (first/last detection, count, timestamps)
/// - Signal statistics (current/average/min/max RSSI)
/// - Detection methods used
/// - Database persistence status
/// - RSSI telemetry for trend/motion analysis
#[derive(Debug, Clone)]
pub struct DeviceTracker {
    pub mac_address: String,
    pub device_name: Option<String>,
    pub manufacturer_id: Option<u16>,
    pub manufacturer_name: Option<String>,

    // Temporal tracking
    pub first_detected: DateTime<Utc>,
    pub last_detected: DateTime<Utc>,
    pub detection_count: u64,
    pub detection_times: Vec<DateTime<Utc>>,

    // Signal tracking
    pub rssi_values: Vec<i8>,
    pub current_rssi: i8,
    pub avg_rssi: f64,
    pub min_rssi: i8,
    pub max_rssi: i8,

    // Detection metadata
    pub detected_by_methods: Vec<String>,
    pub detection_methods_count: usize,
    pub last_detection_method: Option<String>,

    // Database persistence
    pub db_device_id: Option<i32>,
    pub stored_in_db: bool,

    // Telemetry (RSSI trend/motion detection)
    pub telemetry_window: SampleWindow,
    pub last_rssi_smooth: Option<f64>,
    pub device_state: DeviceState,
}

impl DeviceTracker {
    /// Create new device tracker
    ///
    /// Initializes a new tracker for the given MAC address with default values.
    ///
    /// # Arguments
    /// * `mac` - MAC address of the device to track
    ///
    /// # Returns
    /// New DeviceTracker instance with first_detected set to current time
    pub fn new(mac: String) -> Self {
        let now = Utc::now();
        Self {
            mac_address: mac,
            device_name: None,
            manufacturer_id: None,
            manufacturer_name: None,
            first_detected: now,
            last_detected: now,
            detection_count: 0,
            detection_times: vec![now],
            rssi_values: Vec::new(),
            current_rssi: 0,
            avg_rssi: 0.0,
            min_rssi: i8::MAX,
            max_rssi: i8::MIN,
            detected_by_methods: Vec::new(),
            detection_methods_count: 0,
            last_detection_method: None,
            db_device_id: None,
            stored_in_db: false,
            telemetry_window: SampleWindow::new(TELEMETRY_WINDOW_SIZE),
            last_rssi_smooth: None,
            device_state: DeviceState::default(),
        }
    }

    /// Record a detection event
    ///
    /// Updates all tracking statistics for a device detection including:
    /// - Temporal tracking (last_detected, detection_count)
    /// - Signal tracking (current_rssi, avg_rssi, min_rssi, max_rssi)
    /// - Device info (name, manufacturer)
    /// - Telemetry for trend/motion analysis
    ///
    /// # Arguments
    /// * `rssi` - Signal strength in dBm
    /// * `method` - Detection method/source (e.g., "btleplug", "windows")
    /// * `device_name` - Optional device name from advertising data
    /// * `manufacturer_id` - Optional manufacturer ID from company identifier
    pub fn record_detection(
        &mut self,
        rssi: i8,
        method: &str,
        device_name: Option<String>,
        manufacturer_id: Option<u16>,
    ) {
        // Update temporal tracking
        self.last_detected = Utc::now();
        self.detection_count += 1;
        self.detection_times.push(self.last_detected);

        // Update method tracking
        if !self.detected_by_methods.contains(&method.to_string()) {
            self.detected_by_methods.push(method.to_string());
            self.detection_methods_count = self.detected_by_methods.len();
        }
        self.last_detection_method = Some(method.to_string());

        // Update signal tracking
        self.current_rssi = rssi;
        self.rssi_values.push(rssi);
        self.min_rssi = self.min_rssi.min(rssi);
        self.max_rssi = self.max_rssi.max(rssi);
        self.avg_rssi =
            self.rssi_values.iter().map(|&x| x as f64).sum::<f64>() / self.rssi_values.len() as f64;

        // Update device info if provided
        if let Some(name) = device_name {
            self.device_name = Some(name);
        }
        if let Some(mfg_id) = manufacturer_id {
            if self.manufacturer_id.is_none() {
                self.manufacturer_id = Some(mfg_id);
                if let Some(name) = company_id_reference::lookup_company_id(mfg_id) {
                    self.manufacturer_name = Some(name.to_string());
                }
            }
        }

        // Update telemetry (trend/motion detection)
        self.update_telemetry();
    }

    /// Update telemetry state based on RSSI samples
    fn update_telemetry(&mut self) {
        let rssi_f = self.current_rssi as f64;
        let t = self.last_detected.timestamp_subsec_millis() as f64 / 1000.0;

        let rssi_smooth = match self.last_rssi_smooth {
            None => rssi_f,
            Some(prev) => TELEMETRY_EMA_ALPHA * rssi_f + (1.0 - TELEMETRY_EMA_ALPHA) * prev,
        };
        self.last_rssi_smooth = Some(rssi_smooth);

        self.telemetry_window.push(Sample {
            t,
            rssi: rssi_smooth,
        });

        if self.telemetry_window.len() < TELEMETRY_MIN_SAMPLES {
            self.device_state = DeviceState {
                trend: Trend::Unknown,
                motion: Motion::Unknown,
                slope: 0.0,
                variance: 0.0,
                rssi: rssi_smooth,
                confidence: 0.0,
            };
            return;
        }

        let slope = compute_slope(self.telemetry_window.samples());
        let variance = compute_variance(self.telemetry_window.samples());

        let trend = if slope > TELEMETRY_SLOPE_EPS {
            Trend::Approaching
        } else if slope < -TELEMETRY_SLOPE_EPS {
            Trend::Leaving
        } else {
            Trend::Stable
        };

        let motion = if variance < TELEMETRY_VAR_EPS && slope.abs() < TELEMETRY_SLOPE_EPS {
            Motion::Still
        } else {
            Motion::Moving
        };

        let confidence = (self.telemetry_window.len() as f64 / TELEMETRY_WINDOW_SIZE as f64)
            .min(1.0)
            * (1.0 - (variance / 20.0).min(0.5));

        self.device_state = DeviceState {
            trend,
            motion,
            slope,
            variance,
            rssi: rssi_smooth,
            confidence: confidence.max(0.0),
        };
    }

    /// Get time since first detection
    ///
    /// Calculates the duration between first and last detection.
    ///
    /// # Returns
    /// Duration representing time span of device tracking
    pub fn duration_detected(&self) -> chrono::Duration {
        self.last_detected - self.first_detected
    }

    /// Print verbose device info to terminal
    ///
    /// Outputs comprehensive device information including:
    /// - Device identification (MAC, name, manufacturer)
    /// - Temporal statistics (first/last detection, span, count)
    /// - Detection methods used
    /// - Signal quality (current, average, min/max RSSI)
    /// - Recent detection timeline (last 10 events)
    pub fn print_verbose(&self) {
        let method_str = self.detected_by_methods.join(", ");
        let duration = self.duration_detected();
        let duration_str = format_duration(duration);

        println!("\n{}", "═".repeat(100));
        println!("📱 Device: {}", self.mac_address.bright_cyan().bold());

        if let Some(name) = &self.device_name {
            println!("  Name: {}", name.bright_white());
        }

        if let Some(mfg_name) = &self.manufacturer_name {
            println!(
                "  Manufacturer: {} {}",
                "🏭".bright_yellow(),
                mfg_name.bright_white()
            );
        }

        println!("\n⏰ Temporal Info:");
        println!(
            "  First detected:  {}",
            self.first_detected
                .format("%Y-%m-%d %H:%M:%S.%3f UTC")
                .to_string()
                .bright_green()
        );
        println!(
            "  Last detected:   {}",
            self.last_detected
                .format("%Y-%m-%d %H:%M:%S.%3f UTC")
                .to_string()
                .bright_green()
        );
        println!(
            "  Detection span:  {} ({})",
            duration_str.bright_yellow(),
            format!("{:.2}s", duration.num_milliseconds() as f64 / 1000.0)
        );

        println!("\n📊 Detection Stats:");
        println!(
            "  Total detections: {} times",
            self.detection_count.to_string().bright_cyan().bold()
        );
        println!(
            "  Detection rate: {:.2} per minute",
            (self.detection_count as f64 / (duration.num_seconds() as f64 / 60.0))
                .to_string()
                .bright_cyan()
        );
        println!(
            "  Methods used: {} {}",
            self.detection_methods_count.to_string().bright_magenta(),
            format!("[{}]", method_str).bright_magenta()
        );
        println!(
            "  Last method: {}",
            self.last_detection_method
                .as_deref()
                .unwrap_or("unknown")
                .bright_cyan()
        );

        println!("\n📡 Signal Quality:");
        println!(
            "  Current RSSI:     {} dBm",
            self.current_rssi.to_string().bright_white()
        );
        println!(
            "  Average RSSI:     {:.1} dBm",
            self.avg_rssi.to_string().bright_white()
        );
        println!(
            "  Min/Max RSSI:     {} / {} dBm",
            self.min_rssi.to_string().bright_white(),
            self.max_rssi.to_string().bright_white()
        );
        println!(
            "  Signal range:     {} dBm",
            (self.max_rssi - self.min_rssi).to_string().bright_white()
        );

        // Detection timeline (last 10)
        if !self.detection_times.is_empty() {
            println!("\n⌚ Recent Detection Timeline:");
            let recent = if self.detection_times.len() > 10 {
                &self.detection_times[self.detection_times.len() - 10..]
            } else {
                &self.detection_times[..]
            };

            for (idx, time) in recent.iter().enumerate() {
                println!(
                    "  #{:<3} {}",
                    idx + 1,
                    time.format("%H:%M:%S.%3f").to_string().bright_blue()
                );
            }
        }

        println!("{}", "═".repeat(100));
    }

    /// Insert or update device in database and return device ID
    ///
    /// Persists device information to SQLite database using insert_or_update.
    /// Sets db_device_id and stored_in_db flag on success.
    ///
    /// # Returns
    /// * `Ok(id)` - Database ID of persisted device
    /// * `Err(message)` - Error message if persistence failed
    pub fn persist_to_db(&mut self) -> Result<i32, String> {
        let device = ScannedDevice {
            mac_address: self.mac_address.clone(),
            name: self.device_name.clone(),
            rssi: self.current_rssi,
            first_seen: self.first_detected,
            last_seen: self.last_detected,
            manufacturer_id: self.manufacturer_id,
            manufacturer_name: self.manufacturer_name.clone(),
            mac_type: None,
            is_rpa: false,
            security_level: None,
            pairing_method: None,
            is_authenticated: false,
            device_class: None,
            service_classes: None,
            device_type: None,
            ad_flags: None,
            ad_local_name: None,
            ad_tx_power: None,
            ad_appearance: None,
            ad_service_uuids: None,
            ad_manufacturer_data: None,
            ad_service_data: None,
        };

        match db::insert_or_update_device(&device) {
            Ok(id) => {
                self.db_device_id = Some(id);
                self.stored_in_db = true;
                debug!("Device {} persisted to DB with ID {}", self.mac_address, id);
                Ok(id)
            }
            Err(e) => Err(format!(
                "Failed to persist device {}: {}",
                self.mac_address, e
            )),
        }
    }
}

/// Global device tracker manager
///
/// Manages tracking of multiple BLE devices with:
/// - Thread-safe access via Arc<Mutex<>>
/// - Device limit to prevent memory leaks (default 10,000)
/// - Automatic eviction of oldest devices when limit reached
/// - Database persistence for all tracked devices
pub struct DeviceTrackerManager {
    devices: Arc<Mutex<HashMap<String, DeviceTracker>>>,
    max_devices: usize,
}

impl DeviceTrackerManager {
    /// Create new manager with default limit (10000 devices)
    ///
    /// # Returns
    /// New DeviceTrackerManager with 10,000 device limit
    pub fn new() -> Self {
        Self {
            devices: Arc::new(Mutex::new(HashMap::new())),
            max_devices: 10000,
        }
    }

    /// Create new manager with custom device limit
    ///
    /// # Arguments
    /// * `max_devices` - Maximum number of devices to track
    ///
    /// # Returns
    /// New DeviceTrackerManager with specified limit
    pub fn with_limit(max_devices: usize) -> Self {
        Self {
            devices: Arc::new(Mutex::new(HashMap::new())),
            max_devices,
        }
    }

    /// Get or create tracker for device
    ///
    /// Returns Arc to the tracker. If device limit is reached, evicts oldest
    /// device before creating new tracker.
    ///
    /// # Arguments
    /// * `mac` - MAC address of device
    ///
    /// # Returns
    /// Arc<Mutex<DeviceTracker>> for the device
    /// Note: Caller should hold the lock briefly
    pub fn get_or_create(&self, mac: &str) -> Arc<Mutex<DeviceTracker>> {
        let mut devices = self.devices.lock().unwrap();

        // Evict oldest device if at capacity
        if devices.len() >= self.max_devices && !devices.contains_key(mac) {
            if let Some((oldest_mac, _)) = devices.iter().min_by_key(|(_, v)| v.first_detected) {
                let oldest = oldest_mac.clone();
                devices.remove(&oldest);
                log::warn!("Evicted oldest device {} due to capacity limit", oldest);
            }
        }

        devices
            .entry(mac.to_string())
            .or_insert_with(|| DeviceTracker::new(mac.to_string()));

        Arc::new(Mutex::new(devices.get(mac).unwrap().clone()))
    }

    /// Record detection on device - holds lock for entire operation to prevent race
    ///
    /// Creates device tracker if needed, records detection, updates telemetry,
    /// and logs detection event to console. Lock is held for entire operation.
    ///
    /// # Arguments
    /// * `mac` - MAC address of device
    /// * `rssi` - Signal strength in dBm
    /// * `method` - Detection method/source
    /// * `device_name` - Optional device name
    /// * `manufacturer_id` - Optional manufacturer ID
    ///
    /// # Returns
    /// Formatted log message string
    pub fn record_detection(
        &self,
        mac: &str,
        rssi: i8,
        method: &str,
        device_name: Option<String>,
        manufacturer_id: Option<u16>,
    ) -> String {
        // Get tracker and hold lock for entire operation
        let tracker = self.get_or_create(mac);
        let mut t = tracker.lock().unwrap();

        // Record detection
        t.record_detection(rssi, method, device_name, manufacturer_id);

        // Build log message while still holding lock
        let timestamp = Utc::now().format("%H:%M:%S.%3f").to_string();

        let name_str = t
            .device_name
            .as_deref()
            .unwrap_or("(unknown)")
            .bright_white();

        let mfg_str = t
            .manufacturer_name
            .as_deref()
            .unwrap_or("Unknown")
            .bright_yellow();

        let _method_str = format!("[{}]", method).bright_magenta();
        let rssi_str = format!("{:+4} dBm", rssi);

        let log_msg = format!(
            "{} 📡 {} | {} | {} | {:#6} | Count: {:#4} | Avg RSSI: {:.1} dBm",
            timestamp.bright_blue(),
            mac.bright_cyan().bold(),
            name_str,
            mfg_str,
            rssi_str.bright_green(),
            t.detection_count.to_string().bright_yellow(),
            t.avg_rssi
        );

        info!("{}", log_msg);
        log_msg
    }

    /// Get all tracked devices
    ///
    /// Returns vector of all device trackers, sorted by detection count (descending).
    ///
    /// # Returns
    /// Vector of DeviceTracker for all tracked devices
    pub fn get_all_devices(&self) -> Vec<DeviceTracker> {
        let devices = self.devices.lock().unwrap();
        let mut all: Vec<_> = devices.values().cloned().collect();
        all.sort_by(|a, b| b.detection_count.cmp(&a.detection_count));
        all
    }

    /// Get device by MAC
    ///
    /// # Arguments
    /// * `mac` - MAC address to look up
    ///
    /// # Returns
    /// Some(DeviceTracker) if found, None otherwise
    pub fn get_device(&self, mac: &str) -> Option<DeviceTracker> {
        let devices = self.devices.lock().unwrap();
        devices.get(mac).cloned()
    }

    /// Print summary of all tracked devices
    ///
    /// Outputs formatted table to terminal with:
    /// - MAC address, device name, manufacturer
    /// - Detection count and average RSSI
    /// - Detection methods used
    pub fn print_summary(&self) {
        let devices = self.get_all_devices();

        if devices.is_empty() {
            println!("❌ No devices tracked");
            return;
        }

        println!("\n{}", "═".repeat(120));
        println!("🎯 DEVICE TRACKING SUMMARY");
        println!("{}", "═".repeat(120));

        println!(
            "{:<20} | {:<35} | {:<20} | Count | Avg RSSI | Methods",
            "MAC Address".bright_cyan(),
            "Device Name".bright_white(),
            "Manufacturer".bright_yellow()
        );
        println!("{}", "─".repeat(120));

        for device in &devices {
            let name = device.device_name.as_deref().unwrap_or("(unknown)");
            let mfg = device.manufacturer_name.as_deref().unwrap_or("Unknown");
            let methods = device.detected_by_methods.join(", ");

            println!(
                "{:<20} | {:<35} | {:<20} | {:<5} | {:<8.1} | {}",
                device.mac_address.bright_cyan(),
                name.bright_white(),
                mfg.bright_yellow(),
                device.detection_count.to_string().bright_green(),
                format!("{:.1}", device.avg_rssi).bright_green(),
                methods.bright_magenta()
            );
        }

        println!("{}", "═".repeat(120));
        println!(
            "Total devices: {}",
            devices.len().to_string().bright_cyan().bold()
        );
    }

    /// Persist all devices to database
    ///
    /// Iterates through all tracked devices and persists any that haven't
    /// been stored yet. Updates stored_in_db flag on success.
    ///
    /// # Returns
    /// * `Ok(count)` - Number of devices persisted
    /// * `Err(message)` - Error message if critical failure
    pub fn persist_all(&self) -> Result<usize, String> {
        let mut devices = self.devices.lock().unwrap();
        let mut count = 0;

        for tracker in devices.values_mut() {
            if !tracker.stored_in_db {
                match tracker.persist_to_db() {
                    Ok(_) => count += 1,
                    Err(e) => {
                        log::warn!("Failed to persist device: {}", e);
                    }
                }
            }
        }

        info!("Persisted {} new devices to database", count);
        Ok(count)
    }

    /// Export devices to formatted table
    ///
    /// Generates detailed text report with all device information including:
    /// - MAC, name, manufacturer
    /// - First/last detection timestamps
    /// - Detection count and methods
    /// - RSSI statistics (min/avg/max)
    ///
    /// # Returns
    /// Formatted string report
    pub fn export_detailed_report(&self) -> String {
        let devices = self.get_all_devices();
        let mut report = String::new();

        report.push_str(&format!(
            "Device Discovery Report - {}\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));
        report.push_str(&"═".repeat(150));
        report.push_str("\n");

        for device in devices {
            report.push_str(&format!("MAC: {}\n", device.mac_address));
            report.push_str(&format!(
                "Name: {}\n",
                device.device_name.as_deref().unwrap_or("Unknown")
            ));
            report.push_str(&format!(
                "Manufacturer: {}\n",
                device.manufacturer_name.as_deref().unwrap_or("Unknown")
            ));
            report.push_str(&format!(
                "First Detected: {}\n",
                device.first_detected.format("%Y-%m-%d %H:%M:%S.%3f UTC")
            ));
            report.push_str(&format!(
                "Last Detected: {}\n",
                device.last_detected.format("%Y-%m-%d %H:%M:%S.%3f UTC")
            ));
            report.push_str(&format!("Detection Count: {}\n", device.detection_count));
            report.push_str(&format!(
                "Detected By: {}\n",
                device.detected_by_methods.join(", ")
            ));
            report.push_str(&format!(
                "RSSI: {} / {:.1} / {} dBm (min/avg/max)\n",
                device.min_rssi, device.avg_rssi, device.max_rssi
            ));
            report.push_str(&"─".repeat(150));
            report.push_str("\n");
        }

        report
    }
}

/// Format duration to readable string
///
/// Converts chrono::Duration to human-readable format (Xh Xm Xs).
///
/// # Arguments
/// * `duration` - Duration to format
///
/// # Returns
/// Formatted string like "1h 23m 45s" or "5m 30s"
fn format_duration(duration: chrono::Duration) -> String {
    let secs = duration.num_seconds();
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_tracker_creation() {
        let mut tracker = DeviceTracker::new("AA:BB:CC:DD:EE:FF".to_string());
        tracker.record_detection(-50, "btleplug", Some("MyDevice".to_string()), Some(0x004C));

        assert_eq!(tracker.mac_address, "AA:BB:CC:DD:EE:FF");
        assert_eq!(tracker.detection_count, 1);
        assert_eq!(tracker.current_rssi, -50);
        assert!(tracker
            .detected_by_methods
            .contains(&"btleplug".to_string()));
    }

    #[test]
    fn test_manager_tracking() {
        let manager = DeviceTrackerManager::new();
        manager.record_detection(
            "AA:BB:CC:DD:EE:FF",
            -50,
            "btleplug",
            Some("Test".to_string()),
            Some(0x004C),
        );
        manager.record_detection("AA:BB:CC:DD:EE:FF", -48, "btleplug", None, None);

        let devices = manager.get_all_devices();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].detection_count, 2);
    }
}
