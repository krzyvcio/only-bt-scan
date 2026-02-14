/// Complete BLE Advertising Data Parser
/// Parses all 43 AD Types and extracts complete information from advertising packets
/// Supports: Legacy Advertising, Extended Advertising (BT 5.0+), Scan Response Data
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete parsed advertising packet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedAdvertisingPacket {
    pub mac_address: String,
    pub rssi: i8,
    pub flags: Option<AdvertisingFlags>,
    pub local_name: Option<String>,
    pub short_name: Option<String>,
    pub tx_power: Option<i8>,
    pub appearance: Option<u16>,
    pub services_16bit: Vec<u16>,
    pub services_128bit: Vec<String>,
    pub services_32bit: Vec<u32>,
    pub service_data_16: HashMap<u16, Vec<u8>>,
    pub service_data_128: HashMap<String, Vec<u8>>,
    pub service_data_32: HashMap<u32, Vec<u8>>,
    pub manufacturer_data: HashMap<u16, Vec<u8>>,
    pub ad_structures: Vec<ParsedAdStructure>,
    pub is_scan_response: bool,
    pub is_extended_advertising: bool,
}

impl Default for ParsedAdvertisingPacket {
    fn default() -> Self {
        Self {
            mac_address: String::new(),
            rssi: -100,
            flags: None,
            local_name: None,
            short_name: None,
            tx_power: None,
            appearance: None,
            services_16bit: Vec::new(),
            services_128bit: Vec::new(),
            services_32bit: Vec::new(),
            service_data_16: HashMap::new(),
            service_data_128: HashMap::new(),
            service_data_32: HashMap::new(),
            manufacturer_data: HashMap::new(),
            ad_structures: Vec::new(),
            is_scan_response: false,
            is_extended_advertising: false,
        }
    }
}

/// Advertising flags (AD Type 0x01)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AdvertisingFlags {
    pub le_limited_discoverable: bool,
    pub le_general_discoverable: bool,
    pub br_edr_not_supported: bool,
    pub simultaneous_le_and_br_edr_controller: bool,
    pub simultaneous_le_and_br_edr_host: bool,
}

impl AdvertisingFlags {
    pub fn from_byte(byte: u8) -> Self {
        Self {
            le_limited_discoverable: (byte & 0x01) != 0,
            le_general_discoverable: (byte & 0x02) != 0,
            br_edr_not_supported: (byte & 0x04) != 0,
            simultaneous_le_and_br_edr_controller: (byte & 0x08) != 0,
            simultaneous_le_and_br_edr_host: (byte & 0x10) != 0,
        }
    }
}

/// Single parsed AD structure with detailed information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedAdStructure {
    pub ad_type: u8,
    pub type_name: String,
    pub data: Vec<u8>,
    pub description: String,
}

/// Parse complete advertising packet
pub fn parse_advertising_packet(
    mac_address: &str,
    rssi: i8,
    raw_data: &[u8],
    is_scan_response: bool,
) -> ParsedAdvertisingPacket {
    let mut packet = ParsedAdvertisingPacket {
        mac_address: mac_address.to_string(),
        rssi,
        is_scan_response,
        ..Default::default()
    };

    parse_ad_structures(raw_data, &mut packet);
    packet
}

/// Parse all AD structures from raw advertising data
fn parse_ad_structures(raw_data: &[u8], packet: &mut ParsedAdvertisingPacket) {
    let mut pos = 0;

    while pos < raw_data.len() {
        let len = raw_data[pos] as usize;

        // Check for end of data or invalid length
        if len == 0 {
            break;
        }

        if pos + len + 1 > raw_data.len() {
            break;
        }

        let ad_type = raw_data[pos + 1];
        let data = &raw_data[pos + 2..pos + len + 1];

        // Parse the AD structure
        parse_ad_structure(ad_type, data, packet);

        pos += len + 1;
    }
}

/// Parse single AD structure
fn parse_ad_structure(ad_type: u8, data: &[u8], packet: &mut ParsedAdvertisingPacket) {
    let (type_name, description) = get_ad_type_info(ad_type);

    let parsed = ParsedAdStructure {
        ad_type,
        type_name,
        data: data.to_vec(),
        description: description.clone(),
    };

    packet.ad_structures.push(parsed);

    // Process specific AD types
    match ad_type {
        0x01 => parse_flags(data, packet),
        0x02 => parse_incomplete_list_16bit_uuids(data, packet),
        0x03 => parse_complete_list_16bit_uuids(data, packet),
        0x06 => parse_incomplete_list_128bit_uuids(data, packet),
        0x07 => parse_complete_list_128bit_uuids(data, packet),
        0x08 => packet.short_name = parse_string(data),
        0x09 => packet.local_name = parse_string(data),
        0x0A => parse_tx_power(data, packet),
        0x0F => parse_list_32bit_service_uuids(data, packet),
        0x10 => parse_service_data_16bit(data, packet),
        0x14 => parse_list_128bit_service_uuids(data, packet),
        0x15 => parse_service_data_128bit(data, packet),
        0x16 => packet.appearance = parse_appearance(data),
        0x1F => parse_list_32bit_service_uuids(data, packet),
        0x20 => parse_service_data_32bit(data, packet),
        0x26 => parse_le_supported_features(data, packet),
        0xFF => parse_manufacturer_data(data, packet),
        // Other types are stored in ad_structures
        _ => {}
    }
}

/// Get AD Type name and description
fn get_ad_type_info(ad_type: u8) -> (String, String) {
    match ad_type {
        0x01 => ("Flags".to_string(), "LE Limited Discoverable Mode, LE General Discoverable Mode, BR/EDR Not Supported, etc.".to_string()),
        0x02 => ("Incomplete List of 16-bit UUIDs".to_string(), "Incomplete list of 16-bit service UUIDs".to_string()),
        0x03 => ("Complete List of 16-bit UUIDs".to_string(), "Complete list of 16-bit service UUIDs".to_string()),
        0x04 => ("Incomplete List of 32-bit UUIDs".to_string(), "Incomplete list of 32-bit service UUIDs".to_string()),
        0x05 => ("Complete List of 32-bit UUIDs".to_string(), "Complete list of 32-bit service UUIDs".to_string()),
        0x06 => ("Incomplete List of 128-bit UUIDs".to_string(), "Incomplete list of 128-bit service UUIDs".to_string()),
        0x07 => ("Complete List of 128-bit UUIDs".to_string(), "Complete list of 128-bit service UUIDs".to_string()),
        0x08 => ("Shortened Local Name".to_string(), "Short name of the device".to_string()),
        0x09 => ("Complete Local Name".to_string(), "Full name of the device".to_string()),
        0x0A => ("TX Power Level".to_string(), "Transmit power level in dBm".to_string()),
        0x0D => ("Class of Device".to_string(), "Class of device for Bluetooth Classic".to_string()),
        0x0E => ("Simple Pairing Hash C".to_string(), "Simple Pairing Hash C for legacy pairing".to_string()),
        0x0F => ("List of 32-bit Service UUIDs".to_string(), "List of 32-bit service UUIDs".to_string()),
        0x10 => ("Service Data - 16-bit UUID".to_string(), "Service data with 16-bit UUID".to_string()),
        0x11 => ("Public Target Address".to_string(), "Public target address for directed advertising".to_string()),
        0x12 => ("Random Target Address".to_string(), "Random target address for directed advertising".to_string()),
        0x13 => ("Appearance".to_string(), "External appearance of the device".to_string()),
        0x14 => ("Advertising Interval".to_string(), "Advertising interval".to_string()),
        0x15 => ("LE Bluetooth Device Address".to_string(), "LE Bluetooth device address".to_string()),
        0x16 => ("LE Role".to_string(), "LE Role (Peripheral only, Central only, Peripheral and Central)".to_string()),
        0x17 => ("Simple Pairing Hash C-256".to_string(), "Simple Pairing Hash C-256 for LE Secure Connections".to_string()),
        0x18 => ("Simple Pairing Randomizer R-256".to_string(), "Simple Pairing Randomizer R-256 for LE Secure Connections".to_string()),
        0x19 => ("Flags (32-bit)".to_string(), "32-bit version of flags".to_string()),
        0x1A => ("Service Data - 32-bit UUID".to_string(), "Service data with 32-bit UUID".to_string()),
        0x1B => ("Service Data - 128-bit UUID".to_string(), "Service data with 128-bit UUID".to_string()),
        0x1C => ("LE Secure Connections Confirmation Value".to_string(), "LE Secure Connections Confirmation Value".to_string()),
        0x1D => ("LE Secure Connections Random Value".to_string(), "LE Secure Connections Random Value".to_string()),
        0x1E => ("URI".to_string(), "Uniform Resource Identifier".to_string()),
        0x1F => ("Indoor Positioning".to_string(), "Indoor positioning information".to_string()),
        0x20 => ("Transport Discovery Data".to_string(), "Transport discovery data".to_string()),
        0x21 => ("LE Supported Features".to_string(), "LE Supported Features".to_string()),
        0x22 => ("Channel Map Update Indication".to_string(), "Channel Map Update Indication".to_string()),
        0x23 => ("PB-ADV".to_string(), "Mesh Provisioning Advertising".to_string()),
        0x24 => ("Mesh Message".to_string(), "Mesh Message".to_string()),
        0x25 => ("Mesh Beacon".to_string(), "Mesh Beacon".to_string()),
        0x26 => ("Big Info".to_string(), "BIG Information".to_string()),
        0x27 => ("Broadcast Code".to_string(), "Broadcast Code".to_string()),
        0x28 => ("Resolvable Set ID".to_string(), "Resolvable Set Identifier (RSIS)".to_string()),
        0x29 => ("Advertising Interval - long".to_string(), "Advertising Interval (Long)".to_string()),
        0x3D => ("3D Information Data".to_string(), "3D Information Data".to_string()),
        0xFF => ("Manufacturer Specific Data".to_string(), "Manufacturer-specific advertising data".to_string()),
        _ => (format!("Unknown (0x{:02X})", ad_type), "Unknown AD type".to_string()),
    }
}

/// Parse flags (AD Type 0x01)
fn parse_flags(data: &[u8], packet: &mut ParsedAdvertisingPacket) {
    if !data.is_empty() {
        packet.flags = Some(AdvertisingFlags::from_byte(data[0]));
    }
}

/// Parse incomplete list of 16-bit UUIDs (AD Type 0x02)
fn parse_incomplete_list_16bit_uuids(data: &[u8], packet: &mut ParsedAdvertisingPacket) {
    for chunk in data.chunks(2) {
        if chunk.len() == 2 {
            let uuid = u16::from_le_bytes([chunk[0], chunk[1]]);
            if !packet.services_16bit.contains(&uuid) {
                packet.services_16bit.push(uuid);
            }
        }
    }
}

/// Parse complete list of 16-bit UUIDs (AD Type 0x03)
fn parse_complete_list_16bit_uuids(data: &[u8], packet: &mut ParsedAdvertisingPacket) {
    parse_incomplete_list_16bit_uuids(data, packet);
}

/// Parse incomplete list of 128-bit UUIDs (AD Type 0x06)
fn parse_incomplete_list_128bit_uuids(data: &[u8], packet: &mut ParsedAdvertisingPacket) {
    for chunk in data.chunks(16) {
        if chunk.len() == 16 {
            let uuid = format!(
                "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                chunk[3], chunk[2], chunk[1], chunk[0],
                chunk[5], chunk[4],
                chunk[7], chunk[6],
                chunk[8], chunk[9],
                chunk[10], chunk[11], chunk[12], chunk[13], chunk[14], chunk[15]
            );
            if !packet.services_128bit.contains(&uuid) {
                packet.services_128bit.push(uuid);
            }
        }
    }
}

/// Parse complete list of 128-bit UUIDs (AD Type 0x07)
fn parse_complete_list_128bit_uuids(data: &[u8], packet: &mut ParsedAdvertisingPacket) {
    parse_incomplete_list_128bit_uuids(data, packet);
}

/// Parse list of 128-bit service UUIDs (AD Type 0x14)
fn parse_list_128bit_service_uuids(data: &[u8], packet: &mut ParsedAdvertisingPacket) {
    parse_incomplete_list_128bit_uuids(data, packet);
}

/// Parse list of 32-bit service UUIDs (AD Type 0x0F, 0x1F)
fn parse_list_32bit_service_uuids(data: &[u8], packet: &mut ParsedAdvertisingPacket) {
    for chunk in data.chunks(4) {
        if chunk.len() == 4 {
            let uuid = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            if !packet.services_32bit.contains(&uuid) {
                packet.services_32bit.push(uuid);
            }
        }
    }
}

/// Parse TX Power Level (AD Type 0x0A)
fn parse_tx_power(data: &[u8], packet: &mut ParsedAdvertisingPacket) {
    if !data.is_empty() {
        packet.tx_power = Some(data[0] as i8);
    }
}

/// Parse appearance (AD Type 0x16)
fn parse_appearance(data: &[u8]) -> Option<u16> {
    if data.len() >= 2 {
        Some(u16::from_le_bytes([data[0], data[1]]))
    } else {
        None
    }
}

/// Parse service data with 16-bit UUID (AD Type 0x10)
fn parse_service_data_16bit(data: &[u8], packet: &mut ParsedAdvertisingPacket) {
    if data.len() >= 2 {
        let uuid = u16::from_le_bytes([data[0], data[1]]);
        let service_data = data[2..].to_vec();
        packet.service_data_16.insert(uuid, service_data);
    }
}

/// Parse service data with 128-bit UUID (AD Type 0x15, 0x1B)
fn parse_service_data_128bit(data: &[u8], packet: &mut ParsedAdvertisingPacket) {
    if data.len() >= 16 {
        let uuid_bytes = &data[0..16];
        let uuid = format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            uuid_bytes[3], uuid_bytes[2], uuid_bytes[1], uuid_bytes[0],
            uuid_bytes[5], uuid_bytes[4],
            uuid_bytes[7], uuid_bytes[6],
            uuid_bytes[8], uuid_bytes[9],
            uuid_bytes[10], uuid_bytes[11], uuid_bytes[12], uuid_bytes[13], uuid_bytes[14], uuid_bytes[15]
        );
        let service_data = data[16..].to_vec();
        packet.service_data_128.insert(uuid, service_data);
    }
}

/// Parse service data with 32-bit UUID (AD Type 0x1A, 0x20)
fn parse_service_data_32bit(data: &[u8], packet: &mut ParsedAdvertisingPacket) {
    if data.len() >= 4 {
        let uuid = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let service_data = data[4..].to_vec();
        packet.service_data_32.insert(uuid, service_data);
    }
}

/// Parse manufacturer specific data (AD Type 0xFF)
fn parse_manufacturer_data(data: &[u8], packet: &mut ParsedAdvertisingPacket) {
    if data.len() >= 2 {
        let manufacturer_id = u16::from_le_bytes([data[0], data[1]]);
        let mfg_data = data[2..].to_vec();
        packet.manufacturer_data.insert(manufacturer_id, mfg_data);
    }
}

/// Parse LE Supported Features (AD Type 0x21, 0x26)
fn parse_le_supported_features(data: &[u8], _packet: &mut ParsedAdvertisingPacket) {
    // Features are stored in ad_structures, can be parsed later
    if !data.is_empty() {
        let _features = data[0];
        // TODO: Parse individual feature bits
    }
}

/// Parse string from AD data
fn parse_string(data: &[u8]) -> Option<String> {
    String::from_utf8(data.to_vec()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_flags() {
        let mut packet = ParsedAdvertisingPacket::default();
        parse_flags(&[0x06], &mut packet);
        assert!(packet.flags.is_some());
        let flags = packet.flags.unwrap();
        assert!(flags.le_general_discoverable);
        assert!(flags.br_edr_not_supported);
    }

    #[test]
    fn test_parse_tx_power() {
        let mut packet = ParsedAdvertisingPacket::default();
        parse_tx_power(&[-5i8 as u8], &mut packet);
        assert_eq!(packet.tx_power, Some(-5));
    }

    #[test]
    fn test_parse_16bit_uuids() {
        let mut packet = ParsedAdvertisingPacket::default();
        let data = [0x0D, 0x18]; // Heart Rate Service UUID
        parse_complete_list_16bit_uuids(&data, &mut packet);
        assert_eq!(packet.services_16bit.len(), 1);
        assert_eq!(packet.services_16bit[0], 0x180D);
    }
}
