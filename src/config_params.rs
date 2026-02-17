/// Configuration Parameters - Hardcoded values for signal analysis
///
/// These parameters control RSSI filtering, temporal analysis, and packet deduplication
/// ═══════════════════════════════════════════════════════════════════════════════
/// RSSI Configuration
/// ═══════════════════════════════════════════════════════════════════════════════
/// Minimum RSSI threshold (dBm) - packets below this are filtered out
/// Range: -100 to -20 dBm (more negative = weaker signal)
/// Default: -75 dBm (recommended for most Bluetooth scanning)
pub const RSSI_THRESHOLD: i8 = -75;

/// RSSI smoothing factor for exponential moving average
/// Range: 0.0 to 1.0 (0.0 = not smoothed, 1.0 = full average)
pub const RSSI_SMOOTHING_FACTOR: f64 = 0.3;

/// Maximum acceptable RSSI variance for "stable" signal
/// If variance > this, signal is considered unstable
pub const RSSI_VARIANCE_LIMIT: f64 = 15.0;

/// Signal loss detection: if device not seen for N milliseconds
/// (Only relevant for multi-packet monitoring)
pub const SIGNAL_LOSS_TIMEOUT_MS: u64 = 5000; // 5 seconds
/// ═══════════════════════════════════════════════════════════════════════════════
/// Temporal/Timestamp Configuration
/// ═══════════════════════════════════════════════════════════════════════════════
/// Packet deduplication window in milliseconds
/// If 2+ packets from same device arrive within this window,
/// keep only the strongest signal (highest RSSI)
pub const PACKET_DEDUP_WINDOW_MS: u64 = 100; // 100 ms

/// Minimum time between packets from same device (anti-spam)
/// Used to avoid processing too many packets in quick succession
pub const MIN_PACKET_INTERVAL_MS: u64 = 50;

/// Timestamp resolution preference
/// Some analyzers may need microsecond precision
pub const TIMESTAMP_PRECISION_MS: bool = true; // Use milliseconds, not microseconds
/// ═══════════════════════════════════════════════════════════════════════════════
/// Filter Helpers
/// ═══════════════════════════════════════════════════════════════════════════════
/// Check if RSSI value passes the minimum threshold
#[inline]
pub fn should_accept_rssi(rssi: i8) -> bool {
    rssi >= RSSI_THRESHOLD
}

/// Calculate latency between two timestamps (in ms)
pub fn calculate_latency_ms(start_ms: u64, end_ms: u64) -> u64 {
    if end_ms > start_ms {
        end_ms - start_ms
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rssi_threshold() {
        assert!(should_accept_rssi(-70)); // Good signal
        assert!(!should_accept_rssi(-80)); // Below threshold
    }

    #[test]
    fn test_latency_calculation() {
        assert_eq!(calculate_latency_ms(1000, 1500), 500);
        assert_eq!(calculate_latency_ms(1500, 1000), 0); // Edge case
    }
}
