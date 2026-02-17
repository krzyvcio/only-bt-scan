/// Legacy company IDs module - compatibility wrapper
///
/// Delegates to company_id_reference for official Bluetooth SIG lookups.
/// This module provides a simplified interface for company name lookups.
use crate::company_id_reference;
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    /// Cache statistics: (company_count, last_updated_timestamp)
    static ref CACHE_STATS: Mutex<(u32, i64)> = Mutex::new((0, 0));
}

/// Get company name by manufacturer ID (16-bit)
///
/// Returns "Unknown" if not found in official SIG assignments.
///
/// # Arguments
/// * `mfg_id` - 16-bit manufacturer identifier
///
/// # Returns
/// Company name string (or "Unknown (0xXXXX)" if not found)
pub fn get_company_name(mfg_id: u16) -> String {
    company_id_reference::lookup_company_id(mfg_id)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("Unknown (0x{:04X})", mfg_id))
}

/// Get company name by manufacturer ID (32-bit)
///
/// Looks up a 32-bit manufacturer identifier (used for extended company IDs).
///
/// # Arguments
/// * `mfg_id` - 32-bit manufacturer identifier
///
/// # Returns
/// `Some(String)` with company name if found, `None` otherwise
pub fn get_company_name_u32(mfg_id: u32) -> Option<String> {
    company_id_reference::lookup_company_id_u32(mfg_id).map(|s| s.to_string())
}

/// Initialize company IDs database
///
/// Loads official Bluetooth SIG company identifiers and initializes
/// the cache statistics. Now integrated with company_id_reference.
pub fn init_company_ids() {
    let count = company_id_reference::total_companies();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    if let Ok(mut stats) = CACHE_STATS.lock() {
        *stats = (count as u32, timestamp);
    }

    log::info!("Initialized {} official Bluetooth SIG Company IDs", count);
}

/// Get cache statistics (count, last_updated timestamp)
///
/// Returns the number of registered company IDs and when they were loaded.
///
/// # Returns
/// `Some((count, timestamp))` if cache is populated, `None` otherwise
pub fn get_cache_stats() -> Option<(u32, i64)> {
    if let Ok(stats) = CACHE_STATS.lock() {
        if stats.0 > 0 {
            return Some(*stats);
        }
    }
    None
}

/// Check and update cache
///
/// Re-initializes the company IDs cache. Now always uses official SIG data.
///
/// # Returns
/// `Ok(())` on success, or an error if update fails
pub async fn check_and_update_cache() -> Result<(), anyhow::Error> {
    init_company_ids();
    Ok(())
}

/// Update from Bluetooth SIG
///
/// Re-initializes with official SIG data. Now integrated - no remote fetch needed.
///
/// # Returns
/// `Ok(String)` with status message, or an error if update fails
pub async fn update_from_bluetooth_sig() -> Result<String, anyhow::Error> {
    init_company_ids();
    let count = company_id_reference::total_companies();
    Ok(format!(
        "Updated with {} official Bluetooth SIG Company IDs",
        count
    ))
}

/// Search for company by name pattern
///
/// Performs a case-insensitive search for companies matching the pattern.
///
/// # Arguments
/// * `pattern` - Search string to match against company names
///
/// # Returns
/// Vector of tuples (company_id, company_name) for matching companies
pub fn search_company(pattern: &str) -> Vec<(u16, String)> {
    company_id_reference::search_company_by_name(pattern)
        .into_iter()
        .map(|(id, name)| (id, name.to_string()))
        .collect()
}

/// Verify if manufacturer ID is registered
///
/// Checks whether a company ID exists in the official Bluetooth SIG registry.
///
/// # Arguments
/// * `mfg_id` - 16-bit manufacturer identifier to check
///
/// # Returns
/// `true` if the ID is registered, `false` otherwise
pub fn is_registered(mfg_id: u16) -> bool {
    company_id_reference::is_registered_company_id(mfg_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_company_name() {
        assert_eq!(get_company_name(0x004C), "Apple, Inc.".to_string());
        assert_eq!(get_company_name(0xFFFF), format!("Unknown (0x{:04X})", 0xFFFF));
    }

    #[test]
    fn test_is_registered() {
        assert!(is_registered(0x004C));
        assert!(!is_registered(0xFFFF));
    }
}
