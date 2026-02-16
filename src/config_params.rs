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

/// Get signal quality as percentage (0-100)
/// -30 dBm = excellent (near device)
/// -70 dBm = good
/// -90 dBm = fair/weak
pub fn rssi_to_signal_quality(rssi: i8) -> u8 {
    if rssi >= -30 {
        100
    } else if rssi >= -50 {
        100 - ((rssi + 30) / 2) as u8
    } else if rssi >= -70 {
        80 - ((rssi + 50) / 2) as u8
    } else if rssi >= -90 {
        60 - ((rssi + 70) / 2) as u8
    } else {
        std::cmp::max(10, (rssi + 100) as u8 / 2)
    }
}

/// Check if signal is stable (low variance)
pub fn is_signal_stable(variance: f64) -> bool {
    variance < RSSI_VARIANCE_LIMIT
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// Time Analysis Helpers
/// ═══════════════════════════════════════════════════════════════════════════════

/// Check if two timestamps are within deduplication window
pub fn is_duplicate_packet(timestamp_ms_1: u64, timestamp_ms_2: u64) -> bool {
    let diff = if timestamp_ms_1 > timestamp_ms_2 {
        timestamp_ms_1 - timestamp_ms_2
    } else {
        timestamp_ms_2 - timestamp_ms_1
    };
    diff <= PACKET_DEDUP_WINDOW_MS
}

/// Check if enough time has passed since last packet
pub fn should_process_packet(last_packet_time_ms: u64, current_time_ms: u64) -> bool {
    (current_time_ms - last_packet_time_ms) >= MIN_PACKET_INTERVAL_MS
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
    fn test_signal_quality() {
        let excellent = rssi_to_signal_quality(-30);
        let good = rssi_to_signal_quality(-60);
        let weak = rssi_to_signal_quality(-85);

        assert!(excellent > good);
        assert!(good > weak);
    }

    #[test]
    fn test_duplicate_detection() {
        let t1 = 1000000;
        let t2 = 1000050; // 50ms later

        assert!(is_duplicate_packet(t1, t2)); // Within 100ms window
        assert!(!is_duplicate_packet(t1, t1 + 200)); // Outside 100ms window
    }

    #[test]
    fn test_latency_calculation() {
        assert_eq!(calculate_latency_ms(1000, 1500), 500);
        assert_eq!(calculate_latency_ms(1500, 1000), 0); // Edge case
    }
}
