//! MAC Address Handler - Parsing and filtering of Bluetooth MAC addresses

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MacAddress {
    address: String,
    bytes: [u8; 6],
}

impl MacAddress {
    pub fn from_string(s: &str) -> Result<Self, String> {
        let cleaned = s.replace(":", "").replace("-", "");

        if cleaned.len() != 12 {
            return Err("MAC address must be 6 octets".to_string());
        }

        let mut bytes = [0u8; 6];
        for i in 0..6 {
            let hex_str = &cleaned[i * 2..(i + 1) * 2];
            bytes[i] =
                u8::from_str_radix(hex_str, 16).map_err(|_| format!("Invalid hex: {}", hex_str))?;
        }

        let address = format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]
        );

        Ok(MacAddress { address, bytes })
    }

    pub fn from_bytes(bytes: &[u8; 6]) -> Self {
        let address = format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]
        );
        MacAddress {
            address,
            bytes: *bytes,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.address
    }

    pub fn as_bytes(&self) -> &[u8; 6] {
        &self.bytes
    }

    pub fn is_unicast(&self) -> bool {
        (self.bytes[0] & 0x01) == 0
    }

    pub fn is_multicast(&self) -> bool {
        (self.bytes[0] & 0x01) == 1
    }

    pub fn is_locally_administered(&self) -> bool {
        (self.bytes[0] & 0x02) == 2
    }

    pub fn is_universally_administered(&self) -> bool {
        (self.bytes[0] & 0x02) == 0
    }

    pub fn is_rpa(&self) -> bool {
        let first_byte = self.bytes[0];
        (first_byte & 0xC0) == 0x40
    }

    pub fn is_static_random(&self) -> bool {
        let first_byte = self.bytes[0];
        (first_byte & 0xC0) == 0xC0
    }

    pub fn is_nrpa(&self) -> bool {
        let first_byte = self.bytes[0];
        (first_byte & 0xC0) == 0x00 && self.is_randomly_generated()
    }

    pub fn is_randomly_generated(&self) -> bool {
        let all_zeros = self.bytes == [0, 0, 0, 0, 0, 0];
        let all_ones = self.bytes == [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        !all_zeros && !all_ones
    }

    pub fn manufacturer_id(&self) -> [u8; 3] {
        [self.bytes[0], self.bytes[1], self.bytes[2]]
    }

    pub fn device_id(&self) -> [u8; 3] {
        [self.bytes[3], self.bytes[4], self.bytes[5]]
    }

    pub fn matches_pattern(&self, pattern: &str) -> bool {
        let pattern_parts: Vec<&str> = pattern.split(':').collect();
        if pattern_parts.len() != 6 {
            return false;
        }

        for (i, part) in pattern_parts.iter().enumerate() {
            if *part == "*" || *part == "?" {
                continue;
            }

            if let Ok(pattern_byte) = u8::from_str_radix(part, 16) {
                if self.bytes[i] != pattern_byte {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

impl std::fmt::Display for MacAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.address)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacAddressFilter {
    whitelist: Option<HashSet<String>>,
    blacklist: HashSet<String>,
    patterns: Vec<String>,
    allow_rpa: bool,
    allow_static_random: bool,
    allow_public: bool,
}

impl Default for MacAddressFilter {
    fn default() -> Self {
        MacAddressFilter {
            whitelist: None,
            blacklist: HashSet::new(),
            patterns: Vec::new(),
            allow_rpa: true,
            allow_static_random: true,
            allow_public: true,
        }
    }
}

impl MacAddressFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_whitelist(&mut self, mac: &str) -> Result<(), String> {
        MacAddress::from_string(mac)?;
        if self.whitelist.is_none() {
            self.whitelist = Some(HashSet::new());
        }
        if let Some(ref mut wl) = self.whitelist {
            wl.insert(mac.to_uppercase().replace(":", ""));
        }
        Ok(())
    }

    pub fn add_blacklist(&mut self, mac: &str) -> Result<(), String> {
        MacAddress::from_string(mac)?;
        self.blacklist.insert(mac.to_uppercase().replace(":", ""));
        Ok(())
    }

    pub fn add_pattern(&mut self, pattern: &str) -> Result<(), String> {
        let parts: Vec<&str> = pattern.split(':').collect();
        if parts.len() != 6 {
            return Err("Pattern must have 6 parts".to_string());
        }
        self.patterns.push(pattern.to_uppercase());
        Ok(())
    }

    pub fn matches(&self, mac: &MacAddress) -> bool {
        let mac_str = mac.as_str().replace(":", "");

        if self.blacklist.contains(&mac_str) {
            return false;
        }

        if let Some(ref wl) = self.whitelist {
            if !wl.contains(&mac_str) {
                return false;
            }
        }

        if !self.patterns.is_empty() {
            let mut pattern_match = false;
            for pattern in &self.patterns {
                if mac.matches_pattern(pattern) {
                    pattern_match = true;
                    break;
                }
            }
            if !pattern_match {
                return false;
            }
        }

        if mac.is_rpa() && !self.allow_rpa {
            return false;
        }
        if mac.is_static_random() && !self.allow_static_random {
            return false;
        }
        if !mac.is_rpa() && !mac.is_static_random() && !self.allow_public {
            return false;
        }

        true
    }

    pub fn set_allow_rpa(&mut self, allow: bool) {
        self.allow_rpa = allow;
    }

    pub fn set_allow_static_random(&mut self, allow: bool) {
        self.allow_static_random = allow;
    }

    pub fn set_allow_public(&mut self, allow: bool) {
        self.allow_public = allow;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacAddressStats {
    pub total_addresses: u64,
    pub unique_addresses: u64,
    pub rpa_count: u64,
    pub static_random_count: u64,
    pub public_count: u64,
    pub multicast_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_from_string() {
        let mac = MacAddress::from_string("AA:BB:CC:DD:EE:FF").unwrap();
        assert_eq!(mac.as_str(), "AA:BB:CC:DD:EE:FF");
    }

    #[test]
    fn test_rpa_detection() {
        let rpa = MacAddress::from_string("4A:BB:CC:DD:EE:FF").unwrap();
        assert!(rpa.is_rpa());
    }

    #[test]
    fn test_static_random_detection() {
        let sr = MacAddress::from_string("CA:BB:CC:DD:EE:FF").unwrap();
        assert!(sr.is_static_random());
    }

    #[test]
    fn test_mac_filter_whitelist() {
        let mut filter = MacAddressFilter::new();
        filter.add_whitelist("AA:BB:CC:DD:EE:FF").unwrap();

        let mac1 = MacAddress::from_string("AA:BB:CC:DD:EE:FF").unwrap();
        let mac2 = MacAddress::from_string("11:22:33:44:55:66").unwrap();

        assert!(filter.matches(&mac1));
        assert!(!filter.matches(&mac2));
    }

    #[test]
    fn test_mac_pattern() {
        let mac = MacAddress::from_string("AA:BB:CC:DD:EE:FF").unwrap();
        assert!(mac.matches_pattern("AA:BB:CC:*:*:*"));
        assert!(!mac.matches_pattern("11:BB:CC:*:*:*"));
    }
}
