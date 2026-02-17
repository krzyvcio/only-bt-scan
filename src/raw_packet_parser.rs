//! Raw Bluetooth Packet Parser
//!
//! Parses raw packet data from text format (as shown in logs) and stores to database
//! Format: MAC RSSI TX company-id manuf-data (Company Name)
//!
//! Example:
//! 14:0e:90:a4:b3:90 "" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)

use crate::data_models::{AdStructureData, RawPacketModel};
use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed raw packet from text format
///
/// Contains all extracted fields from a raw packet line including MAC address,
/// RSSI, device name, company information, and manufacturer data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPacketData {
    /// MAC address in uppercase colon-separated format (e.g., "AA:BB:CC:DD:EE:FF")
    pub mac_address: String,
    /// Device name advertised in the packet (if present)
    pub device_name: Option<String>,
    /// Received Signal Strength Indicator in dBm (negative value)
    pub rssi: i8,
    /// Transmit power level in dBm (if advertised)
    pub tx_power: Option<i8>,
    /// Whether the device accepts connections
    pub connectable: bool,
    /// Whether the device is paired
    pub paired: bool,
    /// Company identifier assigned by Bluetooth SIG (if present)
    pub company_id: Option<u16>,
    /// Human-readable company name (if available)
    pub company_name: Option<String>,
    /// Raw manufacturer-specific data as byte vector
    pub manufacturer_data: Vec<u8>,
    /// Hexadecimal string representation of manufacturer data
    pub manufacturer_data_hex: String,
}

/// Parser for raw packet text format
///
/// Uses regex patterns to extract fields from log-formatted packet data.
/// Each instance contains compiled regex patterns for performance.
pub struct RawPacketParser {
    /// Regex pattern for MAC address (XX:XX:XX:XX:XX:XX)
    mac_regex: Regex,
    /// Regex pattern for RSSI value (-XXdB)
    rssi_regex: Regex,
    /// Regex pattern for TX power (tx=XXdBm or tx=n/a)
    tx_regex: Regex,
    /// Regex pattern for company ID (company-id=0xXXXX)
    company_id_regex: Regex,
    /// Regex pattern for manufacturer data (manuf-data=HEXSTRING)
    manuf_data_regex: Regex,
    /// Regex pattern for company name in parentheses
    company_name_regex: Regex,
}

impl Default for RawPacketParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RawPacketParser {
    /// Creates a new RawPacketParser with compiled regex patterns.
    ///
    /// # Returns
    /// A new parser instance ready to process packet lines.
    pub fn new() -> Self {
        Self {
            // MAC address pattern: XX:XX:XX:XX:XX:XX
            mac_regex: Regex::new(r"([0-9A-Fa-f]{2}(?::[0-9A-Fa-f]{2}){5})").unwrap(),

            // RSSI pattern: -XXdB
            rssi_regex: Regex::new(r"-(\d+)dB").unwrap(),

            // TX Power pattern: tx=XXdBm or tx=n/a
            tx_regex: Regex::new(r"tx=([0-9\-]+|n/a)").unwrap(),

            // Company ID pattern: company-id=0xXXXX
            company_id_regex: Regex::new(r"company-id=(0x[0-9A-Fa-f]+)").unwrap(),

            // Manufacturer data pattern: manuf-data=HEXSTRING
            manuf_data_regex: Regex::new(r"manuf-data=([0-9A-Fa-f]+)").unwrap(),

            // Company name in parentheses: (Name)
            company_name_regex: Regex::new(r"\(([^)]+)\)$").unwrap(),
        }
    }

    /// Parse single raw packet line
    ///
    /// Extracts all fields from a single line of packet log data.
    ///
    /// # Arguments
    /// * `line` - A string slice containing the raw packet text
    ///
    /// # Returns
    /// `Some(RawPacketData)` if parsing succeeds, `None` if the line format is invalid
    pub fn parse_packet(&self, line: &str) -> Option<RawPacketData> {
        // Extract MAC address
        let mac_address = self
            .mac_regex
            .captures(line)?
            .get(1)?
            .as_str()
            .to_uppercase();

        // Extract RSSI
        let rssi = self
            .rssi_regex
            .captures(line)?
            .get(1)?
            .as_str()
            .parse::<i8>()
            .ok()
            .map(|v| -v)?;

        // Extract TX Power (optional)
        let tx_power = self
            .tx_regex
            .captures(line)
            .and_then(|cap| {
                let tx_str = cap.get(1)?.as_str();
                if tx_str == "n/a" {
                    Some(None)
                } else {
                    tx_str.parse::<i8>().ok().map(Some)
                }
            })
            .flatten();

        // Extract device name (between quotes)
        let device_name = if let Some(name_start) = line.find('"') {
            if let Some(name_end) = line[name_start + 1..].find('"') {
                let name = line[name_start + 1..name_start + 1 + name_end].to_string();
                if name.is_empty() {
                    None
                } else {
                    Some(name)
                }
            } else {
                None
            }
        } else {
            None
        };

        // Check connectable/non-connectable
        let connectable = !line.contains("Non-Connectable");

        // Check paired/non-paired
        let paired = !line.contains("Non-Paired") && line.contains("Paired");

        // Extract company ID
        let company_id = self.company_id_regex.captures(line).and_then(|cap| {
            let id_str = cap.get(1)?.as_str();
            u16::from_str_radix(&id_str[2..], 16).ok()
        });

        // Extract manufacturer data
        let (manufacturer_data, manufacturer_data_hex) = self
            .manuf_data_regex
            .captures(line)
            .and_then(|cap| {
                let hex_str = cap.get(1)?.as_str();
                let bytes = hex::decode(hex_str).ok()?;
                Some((bytes, hex_str.to_uppercase()))
            })
            .unwrap_or_default();

        // Extract company name
        let company_name = self
            .company_name_regex
            .captures(line)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string());

        Some(RawPacketData {
            mac_address,
            device_name,
            rssi,
            tx_power,
            connectable,
            paired,
            company_id,
            company_name,
            manufacturer_data,
            manufacturer_data_hex,
        })
    }

    /// Parse multiple packet lines
    ///
    /// Parses all lines from the input string, filtering out empty lines.
    ///
    /// # Arguments
    /// * `input` - A string slice containing multiple packet lines separated by newlines
    ///
    /// # Returns
    /// A vector of successfully parsed `RawPacketData` structs
    pub fn parse_packets(&self, input: &str) -> Vec<RawPacketData> {
        input
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return None;
                }
                self.parse_packet(trimmed)
            })
            .collect()
    }

    /// Convert parsed packet to RawPacketModel for database storage
    ///
    /// Transforms a RawPacketData into a RawPacketModel suitable for database insertion.
    /// This includes creating AD structures and generating timestamps.
    ///
    /// # Arguments
    /// * `packet` - A reference to the parsed `RawPacketData`
    /// * `packet_id` - Unique identifier for this packet
    ///
    /// # Returns
    /// A `RawPacketModel` ready for database storage
    pub fn to_raw_packet_model(&self, packet: &RawPacketData, packet_id: u64) -> RawPacketModel {
        let timestamp = Utc::now();
        let timestamp_ms = (chrono::Local::now().timestamp_millis()) as u64;

        // Determine packet type based on connectable flag
        let packet_type = if packet.connectable {
            "ADV_IND".to_string()
        } else {
            "ADV_NONCONN_IND".to_string()
        };

        // Build AD structures from manufacturer data
        let mut ad_structures = Vec::new();

        // Add manufacturer data AD structure (type 0xFF)
        if !packet.manufacturer_data.is_empty() {
            if let Some(company_id) = packet.company_id {
                let mut mfg_data = Vec::new();
                mfg_data.extend_from_slice(&company_id.to_le_bytes());
                mfg_data.extend_from_slice(&packet.manufacturer_data);

                ad_structures.push(AdStructureData {
                    ad_type: 0xFF,
                    type_name: "Manufacturer Specific Data".to_string(),
                    data: mfg_data,
                    data_hex: format!("{:04x}{}", company_id, packet.manufacturer_data_hex),
                    interpretation: format!(
                        "Company ID: 0x{:04X} ({})",
                        company_id,
                        packet.company_name.as_deref().unwrap_or("Unknown")
                    ),
                });
            }
        }

        // Add TX Power if available (type 0x0A)
        if let Some(tx_power) = packet.tx_power {
            ad_structures.push(AdStructureData {
                ad_type: 0x0A,
                type_name: "TX Power Level".to_string(),
                data: vec![tx_power as u8],
                data_hex: format!("{:02X}", tx_power as u8),
                interpretation: format!("{} dBm", tx_power),
            });
        }

        // Add device name if available (type 0x09 - Complete Local Name)
        if let Some(ref name) = packet.device_name {
            ad_structures.push(AdStructureData {
                ad_type: 0x09,
                type_name: "Complete Local Name".to_string(),
                data: name.as_bytes().to_vec(),
                data_hex: hex::encode(name.as_bytes()),
                interpretation: name.clone(),
            });
        }

        // Build manufacturer data map
        let mut manufacturer_data = HashMap::new();
        if let Some(company_id) = packet.company_id {
            manufacturer_data.insert(company_id, packet.manufacturer_data.clone());
        }

        RawPacketModel {
            packet_id,
            mac_address: packet.mac_address.clone(),
            timestamp,
            timestamp_ms,
            latency_from_previous_ms: None,
            phy: "LE 1M".to_string(),
            channel: 37, // Default advertising channel
            rssi: packet.rssi,
            packet_type,
            is_scan_response: false,
            is_extended: false,
            address_type: None,
            advertising_data: packet.manufacturer_data.clone(),
            advertising_data_hex: packet.manufacturer_data_hex.clone(),
            ad_structures,
            flags: None,
            local_name: packet.device_name.clone(),
            short_name: packet.device_name.clone(),
            advertised_services: Vec::new(),
            manufacturer_data,
            service_data: HashMap::new(),
            total_length: packet.manufacturer_data.len(),
            parsed_successfully: true,
        }
    }
}

/// Batch processor for raw packets
///
/// Provides utilities for processing multiple raw packets including
/// deduplication, statistics generation, and conversion to database models.
pub struct RawPacketBatchProcessor {
    /// Internal parser instance
    parser: RawPacketParser,
    /// Collected parsed packets
    packets: Vec<RawPacketData>,
    /// Processed packet models ready for database
    packet_models: Vec<RawPacketModel>,
}

impl Default for RawPacketBatchProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl RawPacketBatchProcessor {
    /// Creates a new empty batch processor.
    ///
    /// # Returns
    /// A new `RawPacketBatchProcessor` instance
    pub fn new() -> Self {
        Self {
            parser: RawPacketParser::new(),
            packets: Vec::new(),
            packet_models: Vec::new(),
        }
    }

    /// Add raw packet text for processing
    ///
    /// Parses the text and adds all valid packets to the batch.
    ///
    /// # Arguments
    /// * `text` - Raw packet text containing one or more packet lines
    pub fn add_raw_text(&mut self, text: &str) {
        let parsed = self.parser.parse_packets(text);
        self.packets.extend(parsed);
    }

    /// Add a single pre-parsed packet
    ///
    /// Adds an already-parsed RawPacketData to the batch queue.
    ///
    /// # Arguments
    /// * `packet` - The parsed packet data to add
    pub fn add_packet(&mut self, packet: RawPacketData) {
        self.packets.push(packet);
    }

    /// Process all packets and convert to models
    ///
    /// Converts all queued packets into RawPacketModel instances suitable
    /// for database storage. Clears any previously processed models.
    ///
    /// # Returns
    /// A vector of processed `RawPacketModel` structs
    pub fn process_all(&mut self) -> Vec<RawPacketModel> {
        self.packet_models.clear();

        for (id, packet) in self.packets.iter().enumerate() {
            let model = self.parser.to_raw_packet_model(packet, id as u64);
            self.packet_models.push(model);
        }

        self.packet_models.clone()
    }

    /// Get deduplicated packets by MAC address (keeps most recent)
    ///
    /// Removes duplicate packets based on MAC address, keeping only the most recent
    /// occurrence of each unique device.
    ///
    /// # Returns
    /// A vector of unique `RawPacketModel` structs (one per MAC address)
    pub fn deduplicate_by_mac(&self) -> Vec<RawPacketModel> {
        let mut dedup: HashMap<String, RawPacketModel> = HashMap::new();

        for packet in &self.packet_models {
            dedup.insert(packet.mac_address.clone(), packet.clone());
        }

        dedup.into_values().collect()
    }

    /// Get statistics about parsed packets
    ///
    /// Calculates aggregate statistics including counts, RSSI values,
    /// and connectability information.
    ///
    /// # Returns
    /// A `RawPacketStatistics` struct containing aggregated data
    pub fn get_statistics(&self) -> RawPacketStatistics {
        let total_packets = self.packets.len();
        let unique_macs = self
            .packets
            .iter()
            .map(|p| &p.mac_address)
            .collect::<std::collections::HashSet<_>>()
            .len();

        let mut rssi_values: Vec<i8> = self.packets.iter().map(|p| p.rssi).collect();
        rssi_values.sort();

        let connectable_count = self.packets.iter().filter(|p| p.connectable).count();
        let with_tx_power = self.packets.iter().filter(|p| p.tx_power.is_some()).count();
        let with_company_data = self
            .packets
            .iter()
            .filter(|p| p.company_id.is_some())
            .count();

        let min_rssi = rssi_values.first().copied().unwrap_or(0);
        let max_rssi = rssi_values.last().copied().unwrap_or(0);
        let avg_rssi = if !rssi_values.is_empty() {
            rssi_values.iter().sum::<i8>() as f64 / rssi_values.len() as f64
        } else {
            0.0
        };

        RawPacketStatistics {
            total_packets,
            unique_macs,
            connectable_count,
            non_connectable_count: total_packets - connectable_count,
            with_tx_power,
            with_company_data,
            min_rssi,
            max_rssi,
            avg_rssi,
        }
    }

    /// Clear all data
    ///
    /// Removes all packets and processed models from the batch processor.
    pub fn clear(&mut self) {
        self.packets.clear();
        self.packet_models.clear();
    }

    /// Get a slice of the raw packet data
    ///
    /// # Returns
    /// A slice of all collected `RawPacketData` structs
    pub fn packets(&self) -> &[RawPacketData] {
        &self.packets
    }

    /// Get a slice of the processed packet models
    ///
    /// # Returns
    /// A slice of all processed `RawPacketModel` structs
    pub fn packet_models(&self) -> &[RawPacketModel] {
        &self.packet_models
    }
}

/// Statistics about parsed packets
///
/// Contains aggregate statistics calculated from a batch of parsed packets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPacketStatistics {
    /// Total number of packets parsed
    pub total_packets: usize,
    /// Number of unique MAC addresses
    pub unique_macs: usize,
    /// Count of connectable devices
    pub connectable_count: usize,
    /// Count of non-connectable devices
    pub non_connectable_count: usize,
    /// Count of packets with TX power information
    pub with_tx_power: usize,
    /// Count of packets with company data
    pub with_company_data: usize,
    /// Minimum RSSI value observed
    pub min_rssi: i8,
    /// Maximum RSSI value observed
    pub max_rssi: i8,
    /// Average RSSI value
    pub avg_rssi: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_packet() {
        let parser = RawPacketParser::new();
        let line = r#"14:0e:90:a4:b3:90 "" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)"#;

        let packet = parser.parse_packet(line).unwrap();

        assert_eq!(packet.mac_address, "14:0E:90:A4:B3:90");
        assert_eq!(packet.rssi, -82);
        assert_eq!(packet.tx_power, None);
        assert_eq!(packet.connectable, false);
        assert_eq!(packet.paired, false);
        assert_eq!(packet.company_id, Some(0x0006));
        assert_eq!(packet.company_name, Some("Microsoft".to_string()));
        assert_eq!(
            packet.manufacturer_data_hex,
            "0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7"
        );
        assert_eq!(packet.device_name, Some("".to_string()));
    }

    #[test]
    fn test_parse_multiple_packets() {
        let parser = RawPacketParser::new();
        let input = r#"14:0e:90:a4:b3:90 "" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)
14:0e:90:a4:b3:90 "" -84dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)"#;

        let packets = parser.parse_packets(input);

        assert_eq!(packets.len(), 2);
        assert_eq!(packets[0].rssi, -82);
        assert_eq!(packets[1].rssi, -84);
    }

    #[test]
    fn test_convert_to_raw_packet_model() {
        let parser = RawPacketParser::new();
        let line = r#"14:0e:90:a4:b3:90 "TestDevice" -82dB tx=10 Connectable Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)"#;

        let packet = parser.parse_packet(line).unwrap();
        let model = parser.to_raw_packet_model(&packet, 1);

        assert_eq!(model.mac_address, "14:0E:90:A4:B3:90");
        assert_eq!(model.rssi, -82);
        assert_eq!(model.local_name, Some("TestDevice".to_string()));
        assert_eq!(model.packet_type, "ADV_IND");
        assert_eq!(model.ad_structures.len(), 3); // Manufacturer data, TX Power, Local Name
    }

    #[test]
    fn test_batch_processor() {
        let mut processor = RawPacketBatchProcessor::new();
        let input = r#"14:0e:90:a4:b3:90 "" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)
14:0e:90:a4:b3:90 "" -84dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)"#;

        processor.add_raw_text(input);
        let models = processor.process_all();

        assert_eq!(models.len(), 2);
        assert_eq!(models[0].packet_id, 0);
        assert_eq!(models[1].packet_id, 1);
    }

    #[test]
    fn test_deduplication() {
        let mut processor = RawPacketBatchProcessor::new();
        let input = r#"14:0e:90:a4:b3:90 "" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)
AA:BB:CC:DD:EE:FF "Device2" -75dB tx=5 Connectable Paired company-id=0x004C manuf-data=020106 (Apple)
14:0e:90:a4:b3:90 "" -84dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)"#;

        processor.add_raw_text(input);
        processor.process_all();
        let dedup = processor.deduplicate_by_mac();

        // Should have 2 unique MACs, with most recent RSSI values
        assert_eq!(dedup.len(), 2);
    }

    #[test]
    fn test_statistics() {
        let mut processor = RawPacketBatchProcessor::new();
        let input = r#"14:0e:90:a4:b3:90 "" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)
AA:BB:CC:DD:EE:FF "Device2" -75dB tx=5 Connectable Paired company-id=0x004C manuf-data=020106 (Apple)"#;

        processor.add_raw_text(input);
        let stats = processor.get_statistics();

        assert_eq!(stats.total_packets, 2);
        assert_eq!(stats.unique_macs, 2);
        assert_eq!(stats.connectable_count, 1);
        assert_eq!(stats.non_connectable_count, 1);
        assert_eq!(stats.min_rssi, -82);
        assert_eq!(stats.max_rssi, -75);
    }
}
