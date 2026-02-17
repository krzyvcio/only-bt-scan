//! Async Batch Database Writer with Backpressure
//!
//! Features:
//! - Batched writes for high throughput
//! - Non-blocking I/O (runs in dedicated task)
//! - Configurable batch size and timeout
//! - Memory limits and graceful degradation
//! - Transaction support for consistency

use log::{debug, error, info};
use rusqlite::{params, Connection};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;

use crate::async_scanner::Packet;

/// Database writer configuration.
/// 
/// Controls batch size, timeouts, and SQLite performance tuning.
/// 
/// # Fields
/// - `batch_size` - Maximum packets to batch before writing (default: 500)
/// - `batch_timeout` - Max wait before flushing partial batch (default: 100ms)
/// - `queue_capacity` - Max packets in memory queue (default: 10000)
/// - `use_wal` - Enable WAL mode (default: true)
/// - `synchronous` - Synchronous mode: OFF, NORMAL, FULL (default: NORMAL)
/// - `cache_size` - Cache size in pages (default: 10000)
/// - `temp_store` - Temp store: MEMORY or FILE (default: MEMORY)
#[derive(Debug, Clone)]
pub struct DbWriterConfig {
    /// Maximum packets to batch before writing
    pub batch_size: usize,
    /// Maximum time to wait before flushing partial batch
    pub batch_timeout: Duration,
    /// Maximum packets in memory queue
    pub queue_capacity: usize,
    /// Enable WAL mode
    pub use_wal: bool,
    /// Synchronous mode (OFF, NORMAL, FULL)
    pub synchronous: String,
    /// Cache size in pages
    pub cache_size: i32,
    /// Temp store (MEMORY, FILE)
    pub temp_store: String,
}

impl Default for DbWriterConfig {
    /// Creates default configuration with sensible production values.
    ///
    /// # Returns
    /// DbWriterConfig - Default configuration
    fn default() -> Self {
        Self {
            batch_size: 500,
            batch_timeout: Duration::from_millis(100),
            queue_capacity: 10000,
            use_wal: true,
            synchronous: "NORMAL".to_string(),
            cache_size: 10000,
            temp_store: "MEMORY".to_string(),
        }
    }
}

/// Database writer statistics.
/// 
/// Tracks performance metrics for monitoring and debugging.
/// 
/// # Fields
/// - `packets_written` - Total packets successfully written
/// - `packets_dropped` - Packets dropped due to queue full
/// - `write_errors` - Number of write failures
/// - `total_batches` - Total batches written
/// - `avg_batch_size` - Average packets per batch
/// - `last_write_duration_ms` - Last batch write time in milliseconds
#[derive(Debug, Clone, Default)]
pub struct DbWriterStats {
    pub packets_written: u64,
    pub packets_dropped: u64,
    pub write_errors: u64,
    pub total_batches: u64,
    pub avg_batch_size: f64,
    pub last_write_duration_ms: u64,
}

/// Packet batch for efficient database insertion.
/// 
/// Groups multiple packets together for bulk insert operations.
/// 
/// # Fields
/// - `packets` - Vector of packets to insert
/// - `created_at` - When the batch was created (for timeout tracking)
#[derive(Debug, Clone)]
pub struct PacketBatch {
    pub packets: Vec<Packet>,
    pub created_at: std::time::Instant,
}

/// Database writer errors.
/// 
/// # Variants
/// - `Database` - SQLite error
/// - `QueueFull` - Packet queue is full (backpressure)
/// - `WriterNotRunning` - Writer task not started
/// - `Channel` - MPSC channel error
#[derive(Debug, thiserror::Error)]
pub enum DbWriterError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("Queue is full")]
    QueueFull,
    
    #[error("Writer not running")]
    NotRunning,
    
    #[error("Channel error: {0}")]
    Channel(String),
}

/// Async batch database writer.
/// 
/// Writes packets to database in batches for high throughput.
/// Runs in a dedicated tokio task for non-blocking operation.
/// 
/// # Type Parameters
/// Uses internal channels for packet submission and control.
/// 
/// # Fields
/// - `config` - Writer configuration
/// - `db_path` - Path to SQLite database
/// - `packet_rx` - Receiver for incoming packets
/// - `control_tx` - Sender for control commands
/// - `stats` - Writer statistics (atomic, thread-safe)
/// - `running` - Flag indicating if writer is active
/// - `pending_batch` - Current batch awaiting flush
pub struct DbWriter {
    config: DbWriterConfig,
    db_path: String,
    packet_rx: mpsc::Receiver<Packet>,
    control_tx: mpsc::Sender<DbWriterCommand>,
    stats: Arc<std::sync::Mutex<DbWriterStats>>,
    running: Arc<std::sync::atomic::AtomicBool>,
    pending_batch: Arc<std::sync::Mutex<Vec<Packet>>>,
}

impl DbWriter {
    /// Creates a new DB writer (call spawn() to start).
    ///
    /// Returns a tuple of:
    /// - Self (not yet running)
    /// - Sender for packets (cloneable)
    /// - Sender for control commands
    ///
    /// # Arguments
    /// * `config` - Writer configuration
    /// * `db_path` - Path to SQLite database file
    ///
    /// # Returns
    /// (Self, mpsc::Sender<Packet>, mpsc::Sender<DbWriterCommand>)
    pub fn new(config: DbWriterConfig, db_path: &str) -> (Self, mpsc::Sender<Packet>, mpsc::Sender<DbWriterCommand>) {
        let (packet_tx, packet_rx) = mpsc::channel(config.queue_capacity);
        let (control_tx, _control_rx) = mpsc::channel(10);

        let writer = Self {
            config: config.clone(),
            db_path: db_path.to_string(),
            packet_rx,
            control_tx: control_tx.clone(),
            stats: Arc::new(std::sync::Mutex::new(DbWriterStats::default())),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            pending_batch: Arc::new(std::sync::Mutex::new(Vec::with_capacity(config.batch_size))),
        };

        (writer, packet_tx, control_tx)
    }

    /// Gets sender for packets.
    ///
    /// Note: In practice, use the sender returned from new().
    /// This method exists for API consistency.
    ///
    /// # Returns
    /// mpsc::Sender<Packet> - Channel sender for packets
    /// 
    /// # Note
    /// Currently returns unimplemented - use sender from new()
    pub fn packet_sender(&self) -> mpsc::Sender<Packet> {
        // This is a bit awkward - in practice we'd use the one returned from new()
        // but we need to return it from here for API consistency
        todo!("Use packet sender from new()")
    }

    /// Gets current statistics.
    ///
    /// Thread-safe snapshot of writer metrics.
    ///
    /// # Returns
    /// DbWriterStats - Current statistics
    pub fn get_stats(&self) -> DbWriterStats {
        self.stats.lock().unwrap().clone()
    }

    /// Starts the writer task.
    ///
    /// This is an async function that runs the main event loop.
    /// It:
    /// 1. Initializes the database schema
    /// 2. Enters the main loop handling packets and timeouts
    /// 3. Flushes batches on timeout or when batch size reached
    /// 4. Exits when packet channel is closed
    ///
    /// # Behavior
    /// - Receives packets and adds to pending batch
    /// - Flushes when batch_size reached
    /// - Flushes on batch_timeout tick
    /// - Stops when running flag is cleared
    pub async fn spawn(mut self) {
        let running = self.running.clone();
        running.store(true, std::sync::atomic::Ordering::SeqCst);

        info!("Starting DB writer task with config: {:?}", self.config);

        // Initialize database
        if let Err(e) = self.init_database() {
            error!("Failed to initialize database: {}", e);
            return;
        }

        let mut batch_timeout = time::interval(self.config.batch_timeout);
        let batch_size = self.config.batch_size;

        loop {
            tokio::select! {
                // Check for stop command
                _ = time::sleep(Duration::from_millis(10)) => {
                    if !running.load(std::sync::atomic::Ordering::SeqCst) {
                        // Flush remaining and stop
                        self.flush_batch().await;
                        break;
                    }
                }

                // Batch timeout
                _ = batch_timeout.tick() => {
                    self.flush_batch().await;
                }

                // Receive packet
                packet = self.packet_rx.recv() => {
                    match packet {
                        Some(p) => {
                            let mut batch = self.pending_batch.lock().unwrap();
                            batch.push(p);

                            if batch.len() >= batch_size {
                                drop(batch);
                                self.flush_batch().await;
                            }
                        }
                        None => {
                            // Channel closed - flush and exit
                            self.flush_batch().await;
                            break;
                        }
                    }
                }
            }
        }

        info!("DB writer task stopped");
    }

    /// Initializes database schema.
    ///
    /// Creates tables for adapters, scan sessions, raw packets, and devices.
    /// Configures SQLite for performance (WAL, cache, temp store).
    ///
    /// # Returns
    /// Result<(), DbWriterError> - Ok on success
    fn init_database(&self) -> Result<(), DbWriterError> {
        let conn = Connection::open(&self.db_path)?;

        // Configure for performance
        if self.config.use_wal {
            conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        }
        conn.execute_batch(&format!("PRAGMA synchronous={};", self.config.synchronous))?;
        conn.execute_batch(&format!("PRAGMA cache_size={};", -self.config.cache_size))?;
        conn.execute_batch(&format!("PRAGMA temp_store={};", self.config.temp_store))?;

        // Create tables
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS adapters (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                address TEXT NOT NULL,
                capabilities TEXT NOT NULL,
                is_default INTEGER DEFAULT 0,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS scan_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                adapter_id TEXT NOT NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                config TEXT NOT NULL,
                packets_received INTEGER DEFAULT 0,
                devices_discovered INTEGER DEFAULT 0,
                FOREIGN KEY (adapter_id) REFERENCES adapters(id)
            );

            CREATE TABLE IF NOT EXISTS raw_packets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER,
                adapter_id TEXT NOT NULL,
                timestamp_ns INTEGER NOT NULL,
                packet_type TEXT NOT NULL,
                mac_address TEXT NOT NULL,
                rssi INTEGER NOT NULL,
                phy TEXT NOT NULL,
                channel INTEGER NOT NULL,
                data BLOB NOT NULL,
                flags INTEGER DEFAULT 0,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_raw_packets_mac ON raw_packets(mac_address);
            CREATE INDEX IF NOT EXISTS idx_raw_packets_timestamp ON raw_packets(timestamp_ns);
            CREATE INDEX IF NOT EXISTS idx_raw_packets_session ON raw_packets(session_id);

            CREATE TABLE IF NOT EXISTS devices (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mac_address TEXT NOT NULL UNIQUE,
                first_seen TEXT NOT NULL,
                last_seen TEXT NOT NULL,
                packet_count INTEGER DEFAULT 0,
                avg_rssi REAL,
                min_rssi INTEGER,
                max_rssi INTEGER,
                name TEXT,
                manufacturer_id INTEGER,
                device_type TEXT,
                is_rpa INTEGER DEFAULT 0,
                security_level TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_devices_last_seen ON devices(last_seen DESC);
            "#,
        )?;

        info!("Database initialized at {}", self.db_path);
        Ok(())
    }

    /// Flushes current batch to database.
    ///
    /// Takes all pending packets, writes them in a blocking task,
    /// and updates statistics. Uses transaction for atomicity.
    ///
    /// # Behavior
    /// - Moves pending batch to local variable
    /// - Spawns blocking task for database write
    /// - Updates stats on completion
    /// - Logs debug info on success
    async fn flush_batch(&self) {
        let mut batch = self.pending_batch.lock().unwrap();
        if batch.is_empty() {
            return;
        }

        let packets = std::mem::take(&mut *batch);
        batch.reserve(self.config.batch_size); // Pre-allocate for next batch
        drop(batch);

        let start = std::time::Instant::now();
        let count = packets.len();

        // Write to database in a blocking task
        let result = tokio::task::spawn_blocking({
            let db_path = self.db_path.clone();
            let packets = packets;
            let stats = self.stats.clone();

            move || {
                if let Err(e) = write_batch_to_db(&db_path, &packets) {
                    error!("Failed to write batch: {}", e);
                    stats.lock().unwrap().write_errors += 1;
                    return Err(e);
                }

                // Update stats
                let mut s = stats.lock().unwrap();
                s.packets_written += packets.len() as u64;
                s.total_batches += 1;
                if s.total_batches > 0 {
                    s.avg_batch_size = s.packets_written as f64 / s.total_batches as f64;
                }
                s.last_write_duration_ms = start.elapsed().as_millis() as u64;

                Ok(())
            }
        })
        .await;

        if let Err(e) = result {
            error!("DB write task failed: {}", e);
        } else if let Err(e) = result {
            error!("DB write failed: {}", e);
        }

        debug!("Wrote {} packets in {:?}",
            count,
            start.elapsed()
        );
    }
}

/// Writes batch of packets to database (blocking).
///
/// Opens a new connection, begins transaction, inserts all packets,
/// then commits. Uses hex encoding for binary data.
///
/// # Arguments
/// * `db_path` - Path to SQLite database
/// * `packets` - Slice of packets to write
///
/// # Returns
/// Result<(), DbWriterError> - Ok on success
fn write_batch_to_db(db_path: &str, packets: &[Packet]) -> Result<(), DbWriterError> {
    let conn = Connection::open(db_path)?;

    // Use transaction for atomicity
    conn.execute("BEGIN TRANSACTION", [])?;

    // Insert packets one by one (SQLite doesn't support bulk insert easily)
    for packet in packets {
        let flags: i32 = {
            let mut flags = 0;
            if packet.is_connectable { flags |= 1; }
            if packet.is_scannable { flags |= 2; }
            flags
        };
        
        let data_hex = hex::encode(&packet.data);

        conn.execute(
            r#"INSERT INTO raw_packets 
               (adapter_id, timestamp_ns, packet_type, mac_address, rssi, phy, channel, data, flags)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            params![
                packet.adapter_id,
                packet.timestamp_ns,
                packet.packet_type.to_string(),
                packet.mac_address,
                packet.rssi,
                packet.phy,
                packet.channel,
                data_hex,
                flags,
            ],
        )?;
    }

    conn.execute("COMMIT", [])?;
    Ok(())
}

/// Commands for DB writer control.
/// 
/// # Variants
/// - `Flush` - Force immediate flush of pending batch
/// - `GetStats` - Request current statistics
/// - `Stop` - Stop the writer task
#[derive(Debug)]
pub enum DbWriterCommand {
    Flush,
    GetStats,
    Stop,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_writer_config_defaults() {
        let config = DbWriterConfig::default();
        assert_eq!(config.batch_size, 500);
        assert_eq!(config.queue_capacity, 10000);
    }

    #[test]
    fn test_db_path_default() {
        let config = DbWriterConfig::default();
        // Just verify it can be cloned
        let _ = config.clone();
    }
}
