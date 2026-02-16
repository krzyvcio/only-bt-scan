//! Async Bluetooth Scanner with Channels and Backpressure
//!
//! Architecture:
//! - Single-producer, multi-consumer channels
//! - Bounded channel with backpressure when full
//! - Separate tasks for scanning, parsing, and storage
//! - Non-blocking design

use chrono::Utc;
use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::time;

use crate::adapter_manager::{Adapter, AdapterSelection};

/// Maximum channel capacity - controls memory usage
const DEFAULT_CHANNEL_CAPACITY: usize = 10000;

/// Batch size for DB writes
const DEFAULT_BATCH_SIZE: usize = 500;

/// Batch timeout in milliseconds
const DEFAULT_BATCH_TIMEOUT_MS: u64 = 100;

/// Raw Bluetooth packet with metadata
#[derive(Debug, Clone)]
pub struct Packet {
    /// Global sequence number
    pub id: u64,
    /// Adapter ID that received this packet
    pub adapter_id: String,
    /// Timestamp in nanoseconds since epoch
    pub timestamp_ns: i64,
    /// MAC address of device
    pub mac_address: String,
    /// RSSI in dBm
    pub rssi: i8,
    /// PHY type (1M, 2M, Coded)
    pub phy: String,
    /// Advertising channel (37, 38, 39 or secondary)
    pub channel: u8,
    /// Raw advertising data
    pub data: Vec<u8>,
    /// Packet type
    pub packet_type: PacketType,
    /// Is connectable
    pub is_connectable: bool,
    /// Has scan response
    pub is_scannable: bool,
    /// Device name if available
    pub device_name: Option<String>,
    /// Manufacturer ID if available
    pub manufacturer_id: Option<u16>,
}

/// Type of Bluetooth packet
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    BleAdvertisement,
    BleScanResponse,
    BleExtendedAdvertisement,
    BrEdrInquiry,
    BrEdrPage,
    Unknown,
}

impl Default for PacketType {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for PacketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PacketType::BleAdvertisement => write!(f, "BLE_ADV"),
            PacketType::BleScanResponse => write!(f, "BLE_SCAN_RSP"),
            PacketType::BleExtendedAdvertisement => write!(f, "BLE_EXT_ADV"),
            PacketType::BrEdrInquiry => write!(f, "BR_EDR_INQUIRY"),
            PacketType::BrEdrPage => write!(f, "BR_EDR_PAGE"),
            PacketType::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Scanner configuration
#[derive(Debug, Clone)]
pub struct ScannerConfig {
    /// Adapter selection strategy
    pub adapter_selection: AdapterSelection,
    /// Scan duration per cycle
    pub scan_duration: Duration,
    /// Number of cycles (0 = infinite)
    pub num_cycles: usize,
    /// Enable BLE scanning
    pub ble_enabled: bool,
    /// Enable Classic scanning (Linux only)
    pub classic_enabled: bool,
    /// Use extended advertising
    pub use_extended: bool,
    /// Use all PHYs
    pub use_all_phys: bool,
    /// Active scanning (request scan responses)
    pub active_scanning: bool,
    /// Filter duplicates
    pub filter_duplicates: bool,
    /// Channel capacity for packets
    pub channel_capacity: usize,
    /// Batch size for DB writes
    pub batch_size: usize,
    /// Batch timeout
    pub batch_timeout: Duration,
    /// Maximum packets in memory
    pub max_packets_in_memory: usize,
    /// Maximum devices to track
    pub max_devices_tracked: usize,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            adapter_selection: AdapterSelection::BestCapabilities,
            scan_duration: Duration::from_secs(10),
            num_cycles: 0, // Infinite
            ble_enabled: true,
            classic_enabled: false,
            use_extended: true,
            use_all_phys: true,
            active_scanning: true,
            filter_duplicates: true,
            channel_capacity: DEFAULT_CHANNEL_CAPACITY,
            batch_size: DEFAULT_BATCH_SIZE,
            batch_timeout: Duration::from_millis(DEFAULT_BATCH_TIMEOUT_MS),
            max_packets_in_memory: 50000,
            max_devices_tracked: 10000,
        }
    }
}

/// Control commands for scanner
#[derive(Debug, Clone)]
pub enum ControlCommand {
    /// Start scanning
    Start,
    /// Stop scanning
    Stop,
    /// Pause scanning
    Pause,
    /// Resume scanning
    Resume,
    /// Update configuration
    UpdateConfig(ScannerConfig),
    /// Request metrics snapshot
    GetMetrics,
}

/// Scanner metrics
#[derive(Debug, Clone, Default)]
pub struct ScannerMetrics {
    pub packets_received: u64,
    pub packets_sent_to_db: u64,
    pub packets_dropped: u64,
    pub devices_discovered: u64,
    pub db_write_errors: u64,
    pub channel_full_count: u64,
    pub start_time: Option<chrono::DateTime<Utc>>,
    pub uptime_seconds: u64,
    pub packets_per_second: f64,
}

impl ScannerMetrics {
    pub fn packets_per_second(&self) -> f64 {
        if let Some(start) = self.start_time {
            let elapsed = Utc::now().signed_duration_since(start).num_seconds() as f64;
            if elapsed > 0.0 {
                return self.packets_received as f64 / elapsed;
            }
        }
        0.0
    }
}

/// Backpressure action when channel is full
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressureAction {
    /// Drop oldest packet
    DropOldest,
    /// Drop newest packet
    DropNewest,
    /// Block (not recommended for scanners)
    Block,
    /// Log warning and drop
    DropWithWarning,
}

impl Default for BackpressureAction {
    fn default() -> Self {
        Self::DropOldest
    }
}

/// Scanner state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScannerState {
    Idle,
    Scanning,
    Paused,
    Stopped,
    Error,
}

impl Default for ScannerState {
    fn default() -> Self {
        Self::Idle
    }
}

/// Async scanner with channels
/// 
/// This is a builder-style struct that creates the channels and tasks.
/// Use `spawn()` to create the actual runtime components.
pub struct AsyncScanner {
    config: ScannerConfig,
    packet_tx: Option<mpsc::Sender<Packet>>,
    packet_rx: Option<mpsc::Receiver<Packet>>,
    control_tx: Option<watch::Sender<ControlCommand>>,
    control_rx: Option<watch::Receiver<ControlCommand>>,
    metrics: Arc<std::sync::Mutex<ScannerMetrics>>,
    state: Arc<std::sync::Mutex<ScannerState>>,
    running: Arc<AtomicBool>,
    packet_counter: Arc<AtomicU64>,
}

impl AsyncScanner {
    /// Create new async scanner with channels
    pub fn new(config: ScannerConfig) -> Self {
        let (packet_tx, packet_rx) = mpsc::channel(config.channel_capacity);
        let (control_tx, control_rx) = watch::channel(ControlCommand::Start);

        Self {
            config,
            packet_tx: Some(packet_tx),
            packet_rx: Some(packet_rx),
            control_tx: Some(control_tx),
            control_rx: Some(control_rx),
            metrics: Arc::new(std::sync::Mutex::new(ScannerMetrics::default())),
            state: Arc::new(std::sync::Mutex::new(ScannerState::Idle)),
            running: Arc::new(AtomicBool::new(false)),
            packet_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Take the packet sender - can only be called once
    pub fn take_packet_sender(&mut self) -> Option<mpsc::Sender<Packet>> {
        self.packet_tx.take()
    }

    /// Take the packet receiver - can only be called once
    pub fn take_packet_receiver(&mut self) -> Option<mpsc::Receiver<Packet>> {
        self.packet_rx.take()
    }

    /// Take the control sender - can only be called once
    pub fn take_control_sender(&mut self) -> Option<watch::Sender<ControlCommand>> {
        self.control_tx.take()
    }

    /// Take the control receiver - can only be called once
    pub fn take_control_receiver(&mut self) -> Option<watch::Receiver<ControlCommand>> {
        self.control_rx.take()
    }

    /// Get packet sender clone
    pub fn packet_sender(&self) -> Option<mpsc::Sender<Packet>> {
        self.packet_tx.as_ref().map(|tx| tx.clone())
    }

    /// Get current metrics
    pub fn get_metrics(&self) -> ScannerMetrics {
        let mut m = self.metrics.lock().unwrap();
        m.packets_per_second = m.packets_per_second();
        m.clone()
    }

    /// Get current state
    pub fn get_state(&self) -> ScannerState {
        *self.state.lock().unwrap()
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Start the scanner (sets state to Running)
    pub fn start(&self) -> Result<(), ScannerError> {
        if self.running.load(Ordering::SeqCst) {
            return Err(ScannerError::AlreadyRunning);
        }

        self.running.store(true, Ordering::SeqCst);

        // Initialize metrics
        {
            let mut m = self.metrics.lock().unwrap();
            m.start_time = Some(Utc::now());
        }

        {
            let mut state = self.state.lock().unwrap();
            *state = ScannerState::Scanning;
        }

        info!("Scanner started with config: channel_capacity={}, batch_size={}", 
            self.config.channel_capacity, self.config.batch_size);
        
        Ok(())
    }

    /// Stop the scanner
    pub fn stop(&self) -> Result<(), ScannerError> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(ScannerError::NotRunning);
        }

        self.running.store(false, Ordering::SeqCst);
        
        // Send stop command
        if let Some(tx) = &self.control_tx {
            let _ = tx.send(ControlCommand::Stop);
        }

        {
            let mut state = self.state.lock().unwrap();
            *state = ScannerState::Stopped;
        }

        info!("Scanner stopped");
        Ok(())
    }

    /// Send a packet (with backpressure handling)
    pub async fn send_packet(&self, packet: Packet) -> Result<(), ScannerError> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(ScannerError::NotRunning);
        }

        if let Some(tx) = &self.packet_tx {
            let packet_id = self.packet_counter.fetch_add(1, Ordering::Relaxed);
            let mut packet = packet;
            packet.id = packet_id;

            // Try to send with backpressure handling
            match tx.try_send(packet) {
                Ok(()) => {
                    // Update metrics
                    let mut m = self.metrics.lock().unwrap();
                    m.packets_received += 1;
                    Ok(())
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    // Channel full - apply backpressure
                    let mut m = self.metrics.lock().unwrap();
                    m.packets_dropped += 1;
                    m.channel_full_count += 1;
                    warn!("Packet dropped - channel full");
                    Ok(())
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    Err(ScannerError::ChannelClosed)
                }
            }
        } else {
            Err(ScannerError::NotRunning)
        }
    }
}

/// Scanner errors
#[derive(Debug, thiserror::Error)]
pub enum ScannerError {
    #[error("Scanner is already running")]
    AlreadyRunning,
    
    #[error("Scanner is not running")]
    NotRunning,
    
    #[error("Channel is full")]
    ChannelFull,
    
    #[error("Channel is closed")]
    ChannelClosed,
    
    #[error("Adapter error: {0}")]
    Adapter(String),
    
    #[error("Scan error: {0}")]
    Scan(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_config_defaults() {
        let config = ScannerConfig::default();
        assert_eq!(config.channel_capacity, DEFAULT_CHANNEL_CAPACITY);
        assert_eq!(config.batch_size, DEFAULT_BATCH_SIZE);
    }

    #[test]
    fn test_metrics() {
        let metrics = ScannerMetrics::default();
        assert_eq!(metrics.packets_received, 0);
    }

    #[tokio::test]
    async fn test_scanner_creation() {
        let scanner = AsyncScanner::new(ScannerConfig::default());
        assert_eq!(scanner.get_state(), ScannerState::Idle);
        assert!(!scanner.is_running());
    }

    #[tokio::test]
    async fn test_scanner_start_stop() {
        let scanner = AsyncScanner::new(ScannerConfig::default());
        
        scanner.start().unwrap();
        assert!(scanner.is_running());
        assert_eq!(scanner.get_state(), ScannerState::Scanning);
        
        scanner.stop().unwrap();
        assert!(!scanner.is_running());
        assert_eq!(scanner.get_state(), ScannerState::Stopped);
    }

    #[tokio::test]
    async fn test_send_packet() {
        let config = ScannerConfig {
            channel_capacity: 10,
            ..Default::default()
        };
        let scanner = AsyncScanner::new(config);
        
        scanner.start().unwrap();
        
        let packet = Packet {
            id: 0,
            adapter_id: "test".to_string(),
            timestamp_ns: 1234567890,
            mac_address: "AA:BB:CC:DD:EE:FF".to_string(),
            rssi: -50,
            phy: "LE 1M".to_string(),
            channel: 37,
            data: vec![0x02, 0x01, 0x06],
            packet_type: PacketType::BleAdvertisement,
            is_connectable: true,
            is_scannable: false,
            device_name: Some("Test Device".to_string()),
            manufacturer_id: Some(0x004C),
        };
        
        let result = scanner.send_packet(packet).await;
        assert!(result.is_ok());
        
        let metrics = scanner.get_metrics();
        assert_eq!(metrics.packets_received, 1);
        
        scanner.stop().unwrap();
    }
}
