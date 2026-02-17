//! Async Bluetooth Scanner with Channels and Backpressure
//!
//! Architecture:
//! - Single-producer, multi-consumer channels
//! - Bounded channel with backpressure when full
//! - Separate tasks for scanning, parsing, and storage
//! - Non-blocking design

use chrono::Utc;
use log::{info, warn};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch};

use crate::adapter_manager::AdapterSelection;

/// Maximum channel capacity - controls memory usage
const DEFAULT_CHANNEL_CAPACITY: usize = 10000;

/// Batch size for DB writes
const DEFAULT_BATCH_SIZE: usize = 500;

/// Batch timeout in milliseconds
const DEFAULT_BATCH_TIMEOUT_MS: u64 = 100;

/// Raw Bluetooth packet with metadata for async processing.
///
/// Represents a single advertising packet received from a BLE device.
/// Contains timing, signal, and raw data information for processing
/// through the async scanner pipeline.
///
/// # Fields
/// - `id`: Global sequence number for ordering
/// - `adapter_id`: ID of the adapter that received the packet
/// - `timestamp_ns`: Timestamp in nanoseconds since epoch
/// - `mac_address`: MAC address of the advertising device
/// - `rssi`: Signal strength in dBm
/// - `phy`: Physical layer type (1M, 2M, Coded)
/// - `channel`: Advertising channel (37, 38, 39)
/// - `data`: Raw advertising data bytes
/// - `packet_type`: Type of packet received
/// - `is_connectable`: Whether device accepts connections
/// - `is_scannable`: Whether device sends scan responses
/// - `device_name`: Device name if available
/// - `manufacturer_id`: Manufacturer ID if available
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

/// Type of Bluetooth packet received.
///
/// Indicates the kind of advertising or inquiry packet that was captured.
///
/// - `BleAdvertisement`: Standard BLE advertising packet
/// - `BleScanResponse`: Response to an active scan
/// - `BleExtendedAdvertisement`: Extended BLE advertising (BT 5.0+)
/// - `BrEdrInquiry`: Classic Bluetooth inquiry message
/// - `BrEdrPage`: Classic Bluetooth page message
/// - `Unknown`: Unrecognized packet type
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

/// Configuration for the async scanner.
///
/// Controls scanning behavior, channel capacity, batch processing,
/// and memory limits for the async scanner pipeline.
///
/// # Fields
/// - `adapter_selection`: Strategy for selecting Bluetooth adapter
/// - `scan_duration`: Duration of each scan cycle
/// - `num_cycles`: Number of cycles (0 = infinite)
/// - `ble_enabled`: Enable BLE scanning
/// - `classic_enabled`: Enable Classic Bluetooth (Linux only)
/// - `use_extended`: Use extended advertising
/// - `use_all_phys`: Use all PHYs (1M, 2M, Coded)
/// - `active_scanning`: Request scan responses
/// - `filter_duplicates`: Filter duplicate packets
/// - `channel_capacity`: Maximum packets in channel
/// - `batch_size`: Number of packets per DB write batch
/// - `batch_timeout`: Timeout for batch writes
/// - `max_packets_in_memory`: Maximum packets to keep in memory
/// - `max_devices_tracked`: Maximum unique devices to track
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

/// Control commands for the scanner.
///
/// Commands sent through the control channel to modify scanner behavior.
///
/// - `Start`: Begin scanning
/// - `Stop`: Stop scanning
/// - `Pause`: Pause scanning temporarily
/// - `Resume`: Resume paused scanning
/// - `UpdateConfig`: Update scanner configuration
/// - `GetMetrics`: Request current metrics snapshot
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

/// Scanner metrics for monitoring performance.
///
/// Tracks packet counts, errors, and performance statistics
/// for the async scanner pipeline.
///
/// # Fields
/// - `packets_received`: Total packets received
/// - `packets_sent_to_db`: Packets written to database
/// - `packets_dropped`: Packets dropped due to backpressure
/// - `devices_discovered`: Unique devices found
/// - `db_write_errors`: Database write failures
/// - `channel_full_count`: Times channel was full
/// - `start_time`: When scanner started
/// - `uptime_seconds`: Time since start in seconds
/// - `packets_per_second`: Current throughput
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

/// Action to take when channel is full (backpressure).
///
/// Controls how the scanner handles the case when the packet
/// channel reaches capacity.
///
/// - `DropOldest`: Remove oldest packet to make room
/// - `DropNewest`: Reject newest packet
/// - `Block`: Wait until space available (not recommended)
/// - `DropWithWarning`: Drop packet and log warning
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

/// Current state of the scanner.
///
/// Represents the operational state of the async scanner.
///
/// - `Idle`: Scanner created but not started
/// - `Scanning`: Actively scanning for devices
/// - `Paused`: Scanning temporarily paused
/// - `Stopped`: Scanning stopped
/// - `Error`: Error occurred
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

/// Async scanner with channels and backpressure.
///
/// Builder-style struct that creates channels and tasks for async scanning.
/// Use `spawn()` to create the actual runtime components, or use the
/// packet/control channels directly with `take_packet_sender()` etc.
///
/// # Fields
/// - `config`: Scanner configuration
/// - `packet_tx`: Sender for packet channel (optional after take)
/// - `packet_rx`: Receiver for packet channel (optional after take)
/// - `control_tx`: Sender for control channel (optional after take)
/// - `control_rx`: Receiver for control channel (optional after take)
/// - `metrics`: Scanner metrics (atomic, thread-safe)
/// - `state`: Current scanner state
/// - `running`: Atomic flag indicating if running
/// - `packet_counter`: Global packet counter
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
    /// Creates a new async scanner with channels.
    ///
    /// Initializes channels with configured capacity and sets up
    /// metrics and state tracking.
    ///
    /// # Arguments
    /// * `config` - Scanner configuration
    ///
    /// # Returns
    /// A new AsyncScanner instance
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

    /// Takes ownership of the packet sender channel.
    ///
    /// Can only be called once. After this, the scanner no longer
    /// has access to send packets.
    ///
    /// # Returns
    /// The packet sender, or None if already taken
    pub fn take_packet_sender(&mut self) -> Option<mpsc::Sender<Packet>> {
        self.packet_tx.take()
    }

    /// Takes ownership of the packet receiver channel.
    ///
    /// Can only be called once. After this, the scanner no longer
    /// has access to receive packets.
    ///
    /// # Returns
    /// The packet receiver, or None if already taken
    pub fn take_packet_receiver(&mut self) -> Option<mpsc::Receiver<Packet>> {
        self.packet_rx.take()
    }

    /// Takes ownership of the control sender channel.
    ///
    /// Can only be called once.
    ///
    /// # Returns
    /// The control sender, or None if already taken
    pub fn take_control_sender(&mut self) -> Option<watch::Sender<ControlCommand>> {
        self.control_tx.take()
    }

    /// Takes ownership of the control receiver channel.
    ///
    /// Can only be called once.
    ///
    /// # Returns
    /// The control receiver, or None if already taken
    pub fn take_control_receiver(&mut self) -> Option<watch::Receiver<ControlCommand>> {
        self.control_rx.take()
    }

    /// Gets a clone of the packet sender for sending from multiple tasks.
    ///
    /// Unlike take_packet_sender(), this can be called multiple times.
    ///
    /// # Returns
    /// Clone of packet sender, or None if already taken
    pub fn packet_sender(&self) -> Option<mpsc::Sender<Packet>> {
        self.packet_tx.as_ref().map(|tx| tx.clone())
    }

    /// Gets current scanner metrics.
    ///
    /// Returns a snapshot of current performance statistics including
    /// packet counts, error counts, and throughput.
    ///
    /// # Returns
    /// Current ScannerMetrics
    pub fn get_metrics(&self) -> ScannerMetrics {
        let mut m = self.metrics.lock().unwrap();
        m.packets_per_second = m.packets_per_second();
        m.clone()
    }

    /// Gets the current scanner state.
    ///
    /// # Returns
    /// Current ScannerState (Idle, Scanning, Paused, Stopped, or Error)
    pub fn get_state(&self) -> ScannerState {
        *self.state.lock().unwrap()
    }

    /// Checks if the scanner is currently running.
    ///
    /// # Returns
    /// True if scanner is in Scanning state
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Starts the scanner, transitioning to Scanning state.
    ///
    /// Initializes metrics start time and sets state to Scanning.
    /// After calling start(), packets can be sent via send_packet().
    ///
    /// # Returns
    /// * `Ok(())` - Scanner started successfully
    /// * `Err(ScannerError::AlreadyRunning)` - Scanner already running
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

    /// Stops the scanner, transitioning to Stopped state.
    ///
    /// Sends a Stop control command and updates state.
    /// After calling stop(), packets cannot be sent.
    ///
    /// # Returns
    /// * `Ok(())` - Scanner stopped successfully
    /// * `Err(ScannerError::NotRunning)` - Scanner not running
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

    /// Sends a packet through the channel with backpressure handling.
    ///
    /// If the channel is full, applies the configured backpressure
    /// action (drops oldest by default) and updates metrics.
    ///
    /// # Arguments
    /// * `packet` - Packet to send
    ///
    /// # Returns
    /// * `Ok(())` - Packet sent (or dropped due to backpressure)
    /// * `Err(ScannerError::NotRunning)` - Scanner not started
    /// * `Err(ScannerError::ChannelClosed)` - Channel closed
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

/// Scanner errors.
///
/// Errors that can occur during async scanner operations.
///
/// - `AlreadyRunning`: Scanner already started
/// - `NotRunning`: Scanner not started
/// - `ChannelFull`: Channel at capacity
/// - `ChannelClosed`: Channel closed
/// - `Adapter`: Adapter-specific error
/// - `Scan`: Scanning error
/// - `Config`: Configuration error
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
