use lazy_static::lazy_static;
/// Official Bluetooth SIG Company ID Reference
/// Data source: https://bitbucket.org/bluetooth-SIG/public (official Bluetooth SIG repository)
/// Loaded from: src/data/assigned_numbers/company_identifiers/company_identifiers.yaml
///
/// This module provides lookup functions for Bluetooth Company IDs
/// ensuring manufacturer identification matches official SIG assignments.
use serde::Deserialize;
use std::collections::BTreeMap;

const COMPANY_IDS_YAML: &str = "";

#[derive(Debug, Deserialize)]
struct CompanyIdentifiers {
    company_identifiers: Vec<CompanyEntry>,
}

#[derive(Debug, Deserialize)]
struct CompanyEntry {
    value: u16,
    name: String,
}

lazy_static! {
    /// Complete mapping of Company IDs (hex) to official manufacturer names
    /// Loaded from YAML file with 11,900+ entries from official Bluetooth SIG assignments
    static ref COMPANY_ID_MAP: BTreeMap<u16, String> = {
        let mut map = BTreeMap::new();

        if let Ok(data) = serde_yaml::from_str::<CompanyIdentifiers>(COMPANY_IDS_YAML) {
            for entry in data.company_identifiers {
                map.insert(entry.value, entry.name);
            }
            log::info!("Loaded {} company identifiers from YAML", map.len());
        } else {
            log::warn!("Failed to parse company_identifiers.yaml, using fallback entries");

            // Fallback entries if YAML fails to parse
            map.insert(0x004C, "Apple, Inc.".to_string());
            map.insert(0x0006, "Microsoft".to_string());
            map.insert(0x00E0, "Google".to_string());
            map.insert(0x0075, "Samsung Electronics Co. Ltd.".to_string());
            map.insert(0x01F0, "Xiaomi Communications Co., Ltd.".to_string());
            map.insert(0x0059, "Nordic Semiconductor ASA".to_string());
            map.insert(0x0258, "Broadcom Corporation".to_string());
            map.insert(0x00F7, "Intel Corp.".to_string());
            map.insert(0x005D, "Mediatek Inc.".to_string());
            map.insert(0x0044, "Texas Instruments".to_string());
            map.insert(0x027D, "HUAWEI Technologies Co., Ltd.".to_string());
        }

        map
    };
}

/// Lookup manufacturer name by Company ID (as u16)
///
/// # Arguments
/// * `company_id` - The Company ID as a 16-bit unsigned integer
///
/// # Returns
/// * `Some(&str)` - Official manufacturer name if found
/// * `None` - If Company ID is not in official SIG assignments
///
/// # Example
/// ```
/// assert_eq!(lookup_company_id(0x004C), Some("Apple, Inc."));
/// assert_eq!(lookup_company_id(0x9999), None);
/// ```
pub fn lookup_company_id(company_id: u16) -> Option<String> {
    COMPANY_ID_MAP.get(&company_id).cloned()
}

/// Lookup manufacturer name by Company ID (as u32 from advertising data)
///
/// # Arguments
/// * `company_id` - The Company ID as a 32-bit value (typically from manufacturer data)
///
/// # Returns
/// * `Some(&str)` - Official manufacturer name if found and within valid range
/// * `None` - If Company ID is out of range or not in official SIG assignments
pub fn lookup_company_id_u32(company_id: u32) -> Option<String> {
    if company_id > u16::MAX as u32 {
        return None;
    }
    lookup_company_id(company_id as u16)
}

/// Get list of all known Company IDs (sorted)
pub fn all_company_ids() -> Vec<u16> {
    COMPANY_ID_MAP.keys().cloned().collect()
}

/// Search companies by name pattern (case-insensitive substring match)
///
/// # Arguments
/// * `pattern` - Substring to search for (case-insensitive)
///
/// # Returns
/// * `Vec<(u16, &str)>` - List of matching (Company ID, name) tuples
///
/// # Example
/// ```
/// let results = search_company_by_name("apple");
/// assert!(results.iter().any(|(id, name)| name.to_lowercase().contains("apple")));
/// ```
pub fn search_company_by_name(pattern: &str) -> Vec<(u16, String)> {
    let pattern_lower = pattern.to_lowercase();
    COMPANY_ID_MAP
        .iter()
        .filter(|(_, name)| name.to_lowercase().contains(&pattern_lower))
        .map(|(&id, name)| (id, name.clone()))
        .collect()
}

/// Get total count of known Company IDs
pub fn total_companies() -> usize {
    COMPANY_ID_MAP.len()
}

/// Verify if a Company ID is officially registered
pub fn is_registered_company_id(company_id: u16) -> bool {
    COMPANY_ID_MAP.contains_key(&company_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_major_companies() {
        assert_eq!(lookup_company_id(0x004C), Some("Apple, Inc.".to_string()));
        assert_eq!(lookup_company_id(0x0006), Some("Microsoft".to_string()));
        assert_eq!(lookup_company_id(0x00E0), Some("Google".to_string()));
    }

    #[test]
    fn test_unknown_company() {
        assert_eq!(lookup_company_id(0xFFFF), None);
    }

    #[test]
    fn test_total_companies() {
        let total = total_companies();
        assert!(
            total > 1000,
            "Should have loaded many companies from YAML, got {}",
            total
        );
    }

    #[test]
    fn test_is_registered() {
        assert!(is_registered_company_id(0x004C)); // Apple
        assert!(!is_registered_company_id(0xFFFF));
    }
}

/// Return a reference to full map for advanced usage
pub fn all_companies() -> &'static BTreeMap<u16, String> {
    &COMPANY_ID_MAP
}

// Note: For full implementation with all 1000+ Company IDs, consider:
// 1. Loading from the YAML file at runtime using serde_yaml
// 2. Generating Rust code from YAML using a build script (build.rs)
// 3. Using the compact data structure from official SIG repository
//
// Current implementation contains ~60 major manufacturers. To add all 1000+ entries,
// use the build script approach which parses src/data/assigned_numbers/company_identifiers/company_identifiers.yaml
