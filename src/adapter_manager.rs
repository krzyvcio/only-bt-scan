//! Bluetooth Adapter Management Module
//!
//! Handles:
//! - Adapter enumeration and detection
//! - Capability assessment
//! - Best adapter selection
//! - Adapter configuration

use log::{info, warn};
use serde::{Deserialize, Serialize};

/// Represents a Bluetooth adapter with its capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adapter {
    /// System identifier for this adapter
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// MAC address of the adapter
    pub address: String,
    /// Adapter capabilities
    pub capabilities: AdapterCapabilities,
    /// Is this the system default adapter
    pub is_default: bool,
}

impl std::fmt::Display for Adapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Adapter {} ({}): {} [{}]",
            self.name,
            self.address,
            self.capabilities.bt_version,
            self.capabilities.score()
        )
    }
}

/// Bluetooth adapter capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdapterCapabilities {
    /// Bluetooth version as integer (40 = 4.0, 52 = 5.2, etc.)
    pub bt_version: u8,
    /// Supports Bluetooth Low Energy
    pub supports_ble: bool,
    /// Supports Classic BR/EDR
    pub supports_bredr: bool,
    /// Supports LE Audio
    pub supports_le_audio: bool,
    /// Supports Extended Advertising (BLE 5.0+)
    pub supports_extended_advertising: bool,
    /// Supports Periodic Advertising
    pub supports_periodic_advertising: bool,
    /// Supports ISO Channels (LE Audio)
    pub supports_iso_channels: bool,
    /// Supports 2M PHY
    pub supports_2m_phy: bool,
    /// Supports Coded PHY (Long Range)
    pub supports_coded_phy: bool,
    /// Maximum number of advertising sets
    pub max_advertising_sets: u8,
    /// Maximum connection interval (in 1.25ms units)
    pub max_connection_interval: u16,
    /// Supports simultaneous central + peripheral
    pub supports_simultaneous_central_peripheral: bool,
}

impl AdapterCapabilities {
    /// Calculate capability score for adapter selection
    /// Higher score = better adapter
    pub fn score(&self) -> u32 {
        let mut score: u32 = 0;

        // Base score from BT version (version number)
        score += self.bt_version as u32;

        // Core features
        if self.supports_ble {
            score += 20;
        }
        if self.supports_bredr {
            score += 10;
        }

        // Advanced BLE features
        if self.supports_extended_advertising {
            score += 15;
        }
        if self.supports_periodic_advertising {
            score += 10;
        }
        if self.supports_le_audio {
            score += 10;
        }
        if self.supports_iso_channels {
            score += 10;
        }

        // PHY support
        if self.supports_2m_phy {
            score += 10;
        }
        if self.supports_coded_phy {
            score += 15;
        }

        // Concurrent roles
        if self.supports_simultaneous_central_peripheral {
            score += 5;
        }

        // Advertising capacity
        score += (self.max_advertising_sets as u32) * 2;

        score
    }

    /// Check if adapter supports a specific feature
    pub fn supports(&self, feature: AdapterFeature) -> bool {
        match feature {
            AdapterFeature::Ble => self.supports_ble,
            AdapterFeature::Classic => self.supports_bredr,
            AdapterFeature::LeAudio => self.supports_le_audio,
            AdapterFeature::ExtendedAdvertising => self.supports_extended_advertising,
            AdapterFeature::PeriodicAdvertising => self.supports_periodic_advertising,
            AdapterFeature::IsoChannels => self.supports_iso_channels,
            AdapterFeature::Phy2M => self.supports_2m_phy,
            AdapterFeature::PhyCoded => self.supports_coded_phy,
        }
    }
}

/// Features that can be queried on an adapter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdapterFeature {
    Ble,
    Classic,
    LeAudio,
    ExtendedAdvertising,
    PeriodicAdvertising,
    IsoChannels,
    Phy2M,
    PhyCoded,
}

/// How to select an adapter
#[derive(Debug, Clone)]
pub enum AdapterSelection {
    /// Select adapter with best capabilities (default)
    BestCapabilities,
    /// Select specific adapter by ID
    ById(String),
    /// Select specific adapter by MAC address
    ByAddress(String),
    /// Select first available adapter
    FirstAvailable,
}

impl Default for AdapterSelection {
    fn default() -> Self {
        Self::BestCapabilities
    }
}

/// Result of adapter enumeration
#[derive(Debug)]
pub struct AdapterList {
    pub adapters: Vec<Adapter>,
    pub selected_id: Option<String>,
}

impl AdapterList {
    /// Select adapter based on selection strategy
    pub fn select(&self, strategy: &AdapterSelection) -> Option<&Adapter> {
        match strategy {
            AdapterSelection::BestCapabilities => {
                self.adapters.iter().max_by_key(|a| a.capabilities.score())
            }
            AdapterSelection::ById(id) => self.adapters.iter().find(|a| &a.id == id),
            AdapterSelection::ByAddress(addr) => {
                self.adapters.iter().find(|a| a.address.to_uppercase() == addr.to_uppercase())
            }
            AdapterSelection::FirstAvailable => self.adapters.first(),
        }
    }

    /// Get the selected adapter
    pub fn selected(&self) -> Option<&Adapter> {
        self.selected_id
            .as_ref()
            .and_then(|id| self.adapters.iter().find(|a| &a.id == id))
            .or_else(|| self.adapters.first())
    }
}

/// Adapter manager trait - platform specific implementations
pub trait AdapterManagerTrait: Send + Sync {
    /// List all available adapters
    fn list_adapters(&self) -> Result<Vec<Adapter>, AdapterError>;

    /// Get default adapter
    fn get_default_adapter(&self) -> Result<Option<Adapter>, AdapterError>;

    /// Enable/disable adapter
    fn set_powered(&self, adapter_id: &str, powered: bool) -> Result<(), AdapterError>;
}

/// Errors related to adapter operations
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("No adapters found")]
    NoAdapters,

    #[error("Adapter not found: {0}")]
    NotFound(String),

    #[error("Adapter error: {0}")]
    Platform(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Adapter not powered: {0}")]
    NotPowered(String),

    #[error("Unsupported platform: {0}")]
    Unsupported(String),
}

impl From<AdapterError> for String {
    fn from(e: AdapterError) -> Self {
        e.to_string()
    }
}

/// Create adapter from btleplug
#[allow(unused_variables)]
pub mod btleplug_adapter {
    use super::*;
    
    /// List adapters using btleplug
    /// Note: This requires btleplug feature to be enabled
    pub async fn list_adapters_btleplug() -> Result<Vec<Adapter>, AdapterError> {
        // btleplug is always available in this project (via Cargo.toml)
        // This is a placeholder - actual implementation uses the scanner modules
        info!("Listing adapters via btleplug");
        
        // Return empty for now - actual enumeration happens in bluetooth_scanner
        Ok(Vec::new())
    }
}

/// Create adapter from Windows API
#[cfg(target_os = "windows")]
pub mod windows_adapter {
    use super::*;

    /// List adapters using Windows Bluetooth API
    pub fn list_adapters_windows() -> Result<Vec<Adapter>, AdapterError> {
        // TODO: Implement using winbluetooth or Windows API
        // For now, use btleplug
        Ok(Vec::new())
    }
}

/// Create adapter from Linux (BlueZ)
#[cfg(target_os = "linux")]
pub mod linux_adapter {
    use super::*;

    /// List adapters using BlueZ
    pub fn list_adapters_linux() -> Result<Vec<Adapter>, AdapterError> {
        // TODO: Implement using bluer
        Ok(Vec::new())
    }
}

/// Unified adapter listing - tries all available methods
pub async fn list_all_adapters() -> Result<Vec<Adapter>, AdapterError> {
    let mut all_adapters = Vec::new();

    // Try btleplug first (cross-platform)
    match btleplug_adapter::list_adapters_btleplug().await {
        Ok(adapters) => {
            info!("Found {} adapters via btleplug", adapters.len());
            all_adapters.extend(adapters);
        }
        Err(e) => {
            warn!("btleplug adapter enumeration failed: {}", e);
        }
    }

    // Try platform-specific methods
    #[cfg(target_os = "windows")]
    {
        match windows_adapter::list_adapters_windows() {
            Ok(adapters) => {
                info!("Found {} adapters via Windows API", adapters.len());
                all_adapters.extend(adapters);
            }
            Err(e) => {
                warn!("Windows adapter enumeration failed: {}", e);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        match linux_adapter::list_adapters_linux() {
            Ok(adapters) => {
                info!("Found {} adapters via BlueZ", adapters.len());
                all_adapters.extend(adapters);
            }
            Err(e) => {
                warn!("Linux adapter enumeration failed: {}", e);
            }
        }
    }

    if all_adapters.is_empty() {
        return Err(AdapterError::NoAdapters);
    }

    Ok(all_adapters)
}

/// Select best adapter from list
pub fn select_best_adapter(adapters: &[Adapter]) -> Option<&Adapter> {
    adapters.iter().max_by_key(|a| a.capabilities.score())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_scoring() {
        let caps_basic = AdapterCapabilities {
            bt_version: BluetoothVersion::V4_2,
            supports_ble: true,
            ..Default::default()
        };

        let caps_advanced = AdapterCapabilities {
            bt_version: BluetoothVersion::V5_2,
            supports_ble: true,
            supports_extended_advertising: true,
            supports_2m_phy: true,
            supports_coded_phy: true,
            ..Default::default()
        };

        assert!(caps_advanced.score() > caps_basic.score());
    }

    #[test]
    fn test_adapter_selection() {
        let adapters = vec![
            Adapter {
                id: "adapter1".to_string(),
                name: "Basic".to_string(),
                address: "AA:BB:CC:DD:EE:01".to_string(),
                capabilities: AdapterCapabilities {
                    bt_version: BluetoothVersion::V4_2,
                    supports_ble: true,
                    ..Default::default()
                },
                is_default: true,
            },
            Adapter {
                id: "adapter2".to_string(),
                name: "Advanced".to_string(),
                address: "AA:BB:CC:DD:EE:02".to_string(),
                capabilities: AdapterCapabilities {
                    bt_version: BluetoothVersion::V5_2,
                    supports_ble: true,
                    supports_extended_advertising: true,
                    supports_coded_phy: true,
                    ..Default::default()
                },
                is_default: false,
            },
        ];

        let list = AdapterList {
            adapters: adapters.clone(),
            selected_id: None,
        };

        // Best capabilities should select adapter2
        let selected = list.select(&AdapterSelection::BestCapabilities).unwrap();
        assert_eq!(selected.id, "adapter2");

        // First available should select adapter1
        let selected = list.select(&AdapterSelection::FirstAvailable).unwrap();
        assert_eq!(selected.id, "adapter1");

        // By ID
        let selected = list.select(&AdapterSelection::ById("adapter1".to_string())).unwrap();
        assert_eq!(selected.id, "adapter1");
    }
}
