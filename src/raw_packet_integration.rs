//! Raw Packet Integration Module
//!
//! Integrates raw packet parsing and processing into the main application
//! Provides functions to:
//! - Load raw packets from files
//! - Process packets during scans
//! - Store results in database
//! - Generate reports

use crate::db_frames;
use crate::raw_packet_parser::{
    RawPacketBatchProcessor, RawPacketData, RawPacketParser, RawPacketStatistics,
};
use chrono::Utc;
use rusqlite::Connection;
use std::fs;
use std::path::Path;

/// Integration handler for raw packet processing
pub struct RawPacketIntegration {
    parser: RawPacketParser,
    processor: RawPacketBatchProcessor,
    session_id: String,
}

impl RawPacketIntegration {
    /// Create new integration handler
    pub fn new() -> Self {
        Self {
            parser: RawPacketParser::new(),
            processor: RawPacketBatchProcessor::new(),
            session_id: Utc::now().to_rfc3339(),
        }
    }

    /// Load raw packets from text file
    pub fn load_from_file<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        log::info!("📂 Loading raw packets from file: {:?}", path.as_ref());

        let content = fs::read_to_string(path)?;
        let packets = self.parser.parse_packets(&content);
        let count = packets.len();

        for packet in packets {
            self.processor.add_packet(packet);
        }

        log::info!("✅ Loaded {} packets from file", count);
        Ok(count)
    }

    /// Load raw packets from string
    pub fn load_from_string(&mut self, text: &str) -> usize {
        log::info!("📝 Loading raw packets from string input");

        let packets = self.parser.parse_packets(text);
        let count = packets.len();

        for packet in packets {
            self.processor.add_packet(packet);
        }

        log::info!("✅ Loaded {} packets from string", count);
        count
    }

    /// Add single raw packet line
    pub fn add_packet_line(&mut self, line: &str) -> bool {
        if let Some(packet) = self.parser.parse_packet(line) {
            self.processor.add_packet(packet);
            log::debug!("📦 Added packet from line");
            true
        } else {
            log::warn!("⚠️  Failed to parse packet line");
            false
        }
    }

    /// Process all loaded packets and get statistics
    pub fn process(&mut self) -> RawPacketStatistics {
        log::info!("🔄 Processing {} packets", self.processor.packets().len());

        self.processor.process_all();
        let stats = self.processor.get_statistics();

        log::info!(
            "✅ Processing complete: {} packets, {} unique devices, avg RSSI: {:.1} dBm",
            stats.total_packets,
            stats.unique_macs,
            stats.avg_rssi
        );

        stats
    }

    /// Save processed packets to database
    pub fn save_to_database(
        &mut self,
        conn: &Connection,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!(
            "💾 Saving {} packets to database",
            self.processor.packets().len()
        );

        // Process if not already done
        if self.processor.packet_models().is_empty() {
            self.processor.process_all();
        }

        // Insert packets
        db_frames::insert_raw_packets_from_scan(conn, self.processor.packet_models())?;

        // Get and store statistics
        let stats = self.processor.get_statistics();
        db_frames::store_packet_statistics(conn, &stats, &self.session_id)?;

        log::info!(
            "✅ Saved {} packets and statistics",
            self.processor.packet_models().len()
        );
        Ok(())
    }

    /// Get deduplicated packets (one per MAC address)
    pub fn get_deduplicated(&self) -> Vec<crate::data_models::RawPacketModel> {
        log::info!("🔄 Deduplicating packets by MAC address");

        let unique = self.processor.deduplicate_by_mac();
        log::info!("✅ Deduplicated to {} unique devices", unique.len());

        unique
    }

    /// Get statistics from current batch
    pub fn get_statistics(&self) -> RawPacketStatistics {
        self.processor.get_statistics()
    }

    /// Get statistics report as string
    pub fn get_report(&self) -> String {
        let stats = self.get_statistics();

        format!(
            r#"
╔════════════════════════════════════════════════════════════════╗
║               RAW PACKET PROCESSING REPORT                      ║
╚════════════════════════════════════════════════════════════════╝

📊 STATISTICS:
  Total Packets:           {}
  Unique Devices:          {}

📱 DEVICE STATUS:
  Connectable:             {}
  Non-Connectable:         {}

🔋 ADVANCED FEATURES:
  With TX Power:           {}
  With Company Data:       {}

📈 SIGNAL STRENGTH:
  Strongest:               {} dBm
  Weakest:                 {} dBm
  Average:                 {:.1} dBm

🎯 SESSION ID: {}

"#,
            stats.total_packets,
            stats.unique_macs,
            stats.connectable_count,
            stats.non_connectable_count,
            stats.with_tx_power,
            stats.with_company_data,
            stats.max_rssi,
            stats.min_rssi,
            stats.avg_rssi,
            self.session_id
        )
    }

    /// Clear all data for new batch
    pub fn clear(&mut self) {
        log::info!("🧹 Clearing processor");
        self.processor.clear();
        self.session_id = Utc::now().to_rfc3339();
    }

    /// Get raw packets (before processing)
    pub fn get_raw_packets(&self) -> &[RawPacketData] {
        self.processor.packets()
    }

    /// Get processed models
    pub fn get_models(&self) -> &[crate::data_models::RawPacketModel] {
        self.processor.packet_models()
    }
}

impl Default for RawPacketIntegration {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function: Load and process raw packets from file
pub async fn process_raw_packets_from_file<P: AsRef<Path>>(
    path: P,
    conn: &Connection,
) -> Result<RawPacketStatistics, Box<dyn std::error::Error>> {
    let mut integration = RawPacketIntegration::new();

    integration.load_from_file(path)?;
    let stats = integration.process();
    integration.save_to_database(conn)?;

    println!("{}", integration.get_report());

    Ok(stats)
}

/// Helper function: Process raw packet text
pub async fn process_raw_packets_text(
    text: &str,
    conn: &Connection,
) -> Result<RawPacketStatistics, Box<dyn std::error::Error>> {
    let mut integration = RawPacketIntegration::new();

    integration.load_from_string(text);
    let stats = integration.process();
    integration.save_to_database(conn)?;

    println!("{}", integration.get_report());

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integration_creation() {
        let integration = RawPacketIntegration::new();
        assert!(!integration.session_id.is_empty());
    }

    #[test]
    fn test_add_packet_line() {
        let mut integration = RawPacketIntegration::new();
        let line = r#"14:0e:90:a4:b3:90 "" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)"#;

        let result = integration.add_packet_line(line);
        assert!(result);
        assert_eq!(integration.processor.packets().len(), 1);
    }

    #[test]
    fn test_process() {
        let mut integration = RawPacketIntegration::new();
        let line = r#"14:0e:90:a4:b3:90 "" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)"#;

        integration.add_packet_line(line);
        let stats = integration.process();

        assert_eq!(stats.total_packets, 1);
        assert_eq!(stats.unique_macs, 1);
        assert_eq!(stats.min_rssi, -82);
        assert_eq!(stats.max_rssi, -82);
    }

    #[test]
    fn test_clear() {
        let mut integration = RawPacketIntegration::new();
        let line = r#"14:0e:90:a4:b3:90 "" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)"#;

        integration.add_packet_line(line);
        assert_eq!(integration.processor.packets().len(), 1);

        integration.clear();
        assert_eq!(integration.processor.packets().len(), 0);
    }

    #[test]
    fn test_get_report() {
        let mut integration = RawPacketIntegration::new();
        let line = r#"14:0e:90:a4:b3:90 "" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)"#;

        integration.add_packet_line(line);
        integration.process();
        let report = integration.get_report();

        assert!(report.contains("RAW PACKET PROCESSING REPORT"));
        assert!(report.contains("Total Packets"));
    }
}
