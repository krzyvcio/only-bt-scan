use crate::advertising_parser::ParsedAdvertisingPacket;
/// Vendor-specific BLE protocols parser
/// Implements: iBeacon, Eddystone, Apple Continuity, Google Fast Pair, Microsoft Swift Pair
use serde::{Deserialize, Serialize};

/// Detected beacon type and data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BeaconType {
    IBeacon(IBeaconData),
    Eddystone(EddystoneData),
    AltBeacon(AltBeaconData),
    Unknown,
}

/// iBeacon format (Apple proximity beacon)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IBeaconData {
    pub uuid: String,
    pub major: u16,
    pub minor: u16,
    pub tx_power: i8,
    pub measured_power: Option<i8>,
}

/// Eddystone beacon formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EddystoneData {
    UID {
        namespace_id: [u8; 10],
        instance_id: [u8; 6],
        tx_power: i8,
    },
    URL {
        url: String,
        tx_power: i8,
    },
    TLM {
        version: u8,
        battery_voltage: u16,
        temperature: i8,
        pdu_count: u32,
        uptime_millis: u32,
    },
    EID {
        eid: [u8; 8],
        tx_power: i8,
    },
}

/// AltBeacon format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AltBeaconData {
    pub manufacturer_id: u16,
    pub beacon_code: u16,
    pub uuid: String,
    pub major: u16,
    pub minor: u16,
    pub tx_power: i8,
    pub reserved: u8,
}

/// Apple Continuity protocols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppleContinuity {
    Handoff { sequence_number: u32, auth_tag: u32 },
    AirDrop { hash: Vec<u8> },
    Nearby { action: u8, hash: Vec<u8> },
    Unknown,
}

/// Google Fast Pair info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleFastPair {
    pub model_id: u32,
    pub flags: u8,
    pub battery_level: Option<u8>,
    pub is_show_ui_indication: bool,
}

/// Microsoft Swift Pair info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftSwiftPair {
    pub tlv_version: u8,
    pub tlv_data: Vec<(u8, Vec<u8>)>,
}

/// Parse vendor-specific protocols from advertising packet
pub fn parse_vendor_protocols(packet: &ParsedAdvertisingPacket) -> Vec<VendorProtocol> {
    let mut protocols = Vec::new();

    // Check for iBeacon
    if let Some(beacon) = detect_ibeacon(packet) {
        protocols.push(VendorProtocol::IBeacon(beacon));
    }

    // Check for Eddystone
    if let Some(eddystone) = detect_eddystone(packet) {
        protocols.push(VendorProtocol::Eddystone(eddystone));
    }

    // Check for AltBeacon
    if let Some(altbeacon) = detect_altbeacon(packet) {
        protocols.push(VendorProtocol::AltBeacon(altbeacon));
    }

    // Check for Apple Continuity
    if let Some(continuity) = detect_apple_continuity(packet) {
        protocols.push(VendorProtocol::AppleContinuity(continuity));
    }

    // Check for Google Fast Pair
    if let Some(fast_pair) = detect_google_fast_pair(packet) {
        protocols.push(VendorProtocol::GoogleFastPair(fast_pair));
    }

    // Check for Microsoft Swift Pair
    if let Some(swift_pair) = detect_microsoft_swift_pair(packet) {
        protocols.push(VendorProtocol::MicrosoftSwiftPair(swift_pair));
    }

    protocols
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VendorProtocol {
    IBeacon(IBeaconData),
    Eddystone(EddystoneData),
    AltBeacon(AltBeaconData),
    AppleContinuity(AppleContinuity),
    GoogleFastPair(GoogleFastPair),
    MicrosoftSwiftPair(MicrosoftSwiftPair),
}

/// Detect iBeacon (Apple manufacturer ID 0x004C with iBeacon prefix)
fn detect_ibeacon(packet: &ParsedAdvertisingPacket) -> Option<IBeaconData> {
    // Apple manufacturer ID
    if let Some(mfg_data) = packet.manufacturer_data.get(&0x004C) {
        // iBeacon format: 0x4C 0x02 0x15 [16 bytes UUID] [2 bytes major] [2 bytes minor] [1 byte TX power]
        if mfg_data.len() >= 23 && mfg_data[0] == 0x02 && mfg_data[1] == 0x15 {
            let uuid = format!(
                "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                mfg_data[2], mfg_data[3], mfg_data[4], mfg_data[5],
                mfg_data[6], mfg_data[7],
                mfg_data[8], mfg_data[9],
                mfg_data[10], mfg_data[11],
                mfg_data[12], mfg_data[13], mfg_data[14], mfg_data[15], mfg_data[16], mfg_data[17]
            );
            let major = u16::from_be_bytes([mfg_data[18], mfg_data[19]]);
            let minor = u16::from_be_bytes([mfg_data[20], mfg_data[21]]);
            let tx_power = mfg_data[22] as i8;

            return Some(IBeaconData {
                uuid,
                major,
                minor,
                tx_power,
                measured_power: packet.tx_power,
            });
        }
    }
    None
}

/// Detect Eddystone beacons (Google manufacturer protocol)
fn detect_eddystone(packet: &ParsedAdvertisingPacket) -> Option<EddystoneData> {
    // Eddystone uses manufacturer ID 0xFFFF or service data with 0xAAFE UUID

    // Check service data with 16-bit UUID 0xAAFE (Eddystone)
    if let Some(svc_data) = packet.service_data_16.get(&0xAAFE) {
        if !svc_data.is_empty() {
            let frame_type = svc_data[0];

            match frame_type {
                0x00 => {
                    // UID frame
                    if svc_data.len() >= 19 {
                        let tx_power = svc_data[1] as i8;
                        let mut namespace_id = [0u8; 10];
                        let mut instance_id = [0u8; 6];
                        namespace_id.copy_from_slice(&svc_data[2..12]);
                        instance_id.copy_from_slice(&svc_data[12..18]);

                        return Some(EddystoneData::UID {
                            namespace_id,
                            instance_id,
                            tx_power,
                        });
                    }
                }
                0x10 => {
                    // URL frame
                    if svc_data.len() >= 4 {
                        let tx_power = svc_data[1] as i8;
                        let url_scheme = svc_data[2];
                        let url_data = &svc_data[3..];

                        let url = decode_eddystone_url(url_scheme, url_data);
                        return Some(EddystoneData::URL { url, tx_power });
                    }
                }
                0x20 => {
                    // TLM frame
                    if svc_data.len() >= 14 {
                        let version = svc_data[1];
                        let battery_voltage = u16::from_be_bytes([svc_data[2], svc_data[3]]);
                        let temperature = svc_data[4] as i8;
                        let pdu_count = u32::from_be_bytes([
                            svc_data[5],
                            svc_data[6],
                            svc_data[7],
                            svc_data[8],
                        ]);
                        let uptime_millis = u32::from_be_bytes([
                            svc_data[9],
                            svc_data[10],
                            svc_data[11],
                            svc_data[12],
                        ]);

                        return Some(EddystoneData::TLM {
                            version,
                            battery_voltage,
                            temperature,
                            pdu_count,
                            uptime_millis,
                        });
                    }
                }
                0x30 => {
                    // EID frame
                    if svc_data.len() >= 10 {
                        let tx_power = svc_data[1] as i8;
                        let mut eid = [0u8; 8];
                        eid.copy_from_slice(&svc_data[2..10]);

                        return Some(EddystoneData::EID { eid, tx_power });
                    }
                }
                _ => {}
            }
        }
    }

    None
}

/// Detect AltBeacon format
fn detect_altbeacon(packet: &ParsedAdvertisingPacket) -> Option<AltBeaconData> {
    // AltBeacon uses manufacturer specific data with specific format
    // Check if we have a pattern that looks like AltBeacon
    for (mfg_id, data) in &packet.manufacturer_data {
        // AltBeacon format: 0x4C 0x00 0x02 0x01 [16 bytes UUID] [2 bytes major] [2 bytes minor] [1 byte TX] [1 byte reserved]
        if data.len() >= 26 && data[0] == 0xBE && data[1] == 0xAC {
            let uuid = format!(
                "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                data[2], data[3], data[4], data[5],
                data[6], data[7],
                data[8], data[9],
                data[10], data[11],
                data[12], data[13], data[14], data[15], data[16], data[17]
            );
            let major = u16::from_be_bytes([data[18], data[19]]);
            let minor = u16::from_be_bytes([data[20], data[21]]);
            let tx_power = data[22] as i8;
            let reserved = data[23];

            return Some(AltBeaconData {
                manufacturer_id: *mfg_id,
                beacon_code: u16::from_be_bytes([data[0], data[1]]),
                uuid,
                major,
                minor,
                tx_power,
                reserved,
            });
        }
    }
    None
}

/// Detect Apple Continuity handoff, AirDrop, Nearby
fn detect_apple_continuity(packet: &ParsedAdvertisingPacket) -> Option<AppleContinuity> {
    // Apple Continuity uses manufacturer ID 0x004C with specific prefixes
    if let Some(mfg_data) = packet.manufacturer_data.get(&0x004C) {
        if mfg_data.len() >= 2 {
            match mfg_data[0] {
                0x00 | 0x01 => {
                    // Handoff
                    if mfg_data.len() >= 10 {
                        let sequence_number = u32::from_be_bytes([
                            mfg_data[1],
                            mfg_data[2],
                            mfg_data[3],
                            mfg_data[4],
                        ]);
                        let auth_tag = u32::from_be_bytes([
                            mfg_data[5],
                            mfg_data[6],
                            mfg_data[7],
                            mfg_data[8],
                        ]);
                        return Some(AppleContinuity::Handoff {
                            sequence_number,
                            auth_tag,
                        });
                    }
                }
                0x05 => {
                    // AirDrop
                    if mfg_data.len() > 1 {
                        return Some(AppleContinuity::AirDrop {
                            hash: mfg_data[1..].to_vec(),
                        });
                    }
                }
                0x08 | 0x0C => {
                    // Nearby
                    if mfg_data.len() > 2 {
                        return Some(AppleContinuity::Nearby {
                            action: mfg_data[1],
                            hash: mfg_data[2..].to_vec(),
                        });
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Detect Google Fast Pair
fn detect_google_fast_pair(packet: &ParsedAdvertisingPacket) -> Option<GoogleFastPair> {
    // Google Fast Pair uses service data with 16-bit UUID 0xFE2C
    if let Some(svc_data) = packet.service_data_16.get(&0xFE2C) {
        if svc_data.len() >= 3 {
            let flags = svc_data[0];
            let model_id = u32::from_le_bytes([
                svc_data[1],
                svc_data[2],
                if svc_data.len() > 3 { svc_data[3] } else { 0 },
                0,
            ]);

            let battery_level = if svc_data.len() > 4 {
                Some(svc_data[4])
            } else {
                None
            };

            let is_show_ui_indication = (flags & 0x01) != 0;

            return Some(GoogleFastPair {
                model_id,
                flags,
                battery_level,
                is_show_ui_indication,
            });
        }
    }
    None
}

/// Detect Microsoft Swift Pair
fn detect_microsoft_swift_pair(packet: &ParsedAdvertisingPacket) -> Option<MicrosoftSwiftPair> {
    // Microsoft Swift Pair uses manufacturer ID 0x006F
    if let Some(mfg_data) = packet.manufacturer_data.get(&0x006F) {
        if mfg_data.len() >= 2 {
            let tlv_version = mfg_data[0];
            let mut tlv_data = Vec::new();

            let mut pos = 1;
            while pos < mfg_data.len() {
                let tlv_type = mfg_data[pos];
                if pos + 1 >= mfg_data.len() {
                    break;
                }
                let tlv_len = mfg_data[pos + 1] as usize;
                if pos + 2 + tlv_len > mfg_data.len() {
                    break;
                }

                let tlv_value = mfg_data[pos + 2..pos + 2 + tlv_len].to_vec();
                tlv_data.push((tlv_type, tlv_value));

                pos += 2 + tlv_len;
            }

            return Some(MicrosoftSwiftPair {
                tlv_version,
                tlv_data,
            });
        }
    }
    None
}

/// Decode Eddystone URL scheme and path
fn decode_eddystone_url(scheme: u8, data: &[u8]) -> String {
    let scheme_str = match scheme {
        0x00 => "http://www.",
        0x01 => "https://www.",
        0x02 => "http://",
        0x03 => "https://",
        _ => "",
    };

    let mut url = scheme_str.to_string();

    // Expand compressed characters
    for &byte in data {
        match byte {
            0x00 => url.push_str(".com/"),
            0x01 => url.push_str(".org/"),
            0x02 => url.push_str(".edu/"),
            0x03 => url.push_str(".net/"),
            0x04 => url.push_str(".info/"),
            0x05 => url.push_str(".biz/"),
            0x06 => url.push_str(".gov/"),
            0x07 => url.push_str(".com"),
            0x08 => url.push_str(".org"),
            0x09 => url.push_str(".edu"),
            0x0A => url.push_str(".net"),
            0x0B => url.push_str(".info"),
            0x0C => url.push_str(".biz"),
            0x0D => url.push_str(".gov"),
            b if b >= 32 && b <= 126 => url.push(b as char),
            _ => {}
        }
    }

    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_eddystone_url() {
        let data = [0x67, 0x6F, 0x6F, 0x67]; // "goog"
        let url = decode_eddystone_url(0x00, &data);
        assert!(url.contains("http://www."));
    }
}
