//! CoreBluetooth Integration Module (macOS/iOS)
//! 
//! Provides native CoreBluetooth access for BLE scanning on Apple platforms.
//! This module uses the `core_bluetooth` crate for direct access to Apple's
//! CoreBluetooth framework.
//!
//! NOTE: This is optional - btleplug already uses CoreBluetooth internally on macOS.
//! Enable the `l2cap` feature in Cargo.toml to use this module.

use crate::data_models::DeviceModel;
use crate::l2cap_analyzer::L2CapChannel;
use log::{info, warn};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct CoreBluetoothConfig {
    pub enabled: bool,
    pub scan_duration: Duration,
    pub discover_services: bool,
    pub discover_characteristics: bool,
    pub extract_l2cap_info: bool,
    pub request_permissions: bool,
}

impl Default for CoreBluetoothConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            scan_duration: Duration::from_secs(30),
            discover_services: true,
            discover_characteristics: true,
            extract_l2cap_info: true,
            request_permissions: true,
        }
    }
}

pub struct CoreBluetoothScanner {
    config: CoreBluetoothConfig,
}

impl CoreBluetoothScanner {
    pub fn new(config: CoreBluetoothConfig) -> Self {
        Self { config }
    }

    /// Start scanning with native CoreBluetooth API
    pub async fn scan(&self) -> Result<Vec<DeviceModel>, Box<dyn std::error::Error + Send + Sync>> {
        info!("🍎 Starting native CoreBluetooth scan");

        #[cfg(feature = "l2cap")]
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            self.scan_native().await
        }

        #[cfg(not(feature = "l2cap"))]
        {
            warn!("CoreBluetooth not available - enable 'l2cap' feature");
            Ok(Vec::new())
        }

        #[cfg(feature = "l2cap")]
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        {
            warn!("CoreBluetooth is macOS/iOS only");
            Ok(Vec::new())
        }
    }

    #[cfg(feature = "l2cap")]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn scan_native(&self) -> Result<Vec<DeviceModel>, Box<dyn std::error::Error + Send + Sync>> {
        use core_bluetooth::central::{CentralEvent, CentralManager};
        
        info!("   Initializing CoreBluetooth CentralManager...");
        
        let (central, mut receiver) = CentralManager::new();
        
        info!("   CBCentralManager created, waiting for state changes...");
        
        let mut devices = Vec::new();
        let scan_start = std::time::Instant::now();
        
        while scan_start.elapsed() < self.config.scan_duration {
            if let Ok(event) = receiver.recv_timeout(Duration::from_millis(500)) {
                match event {
                    CentralEvent::ManagerStateChanged { new_state } => {
                        info!("   Manager state: {:?}", new_state);
                    }
                    CentralEvent::PeripheralDiscovered { peripheral, advertisement_data, rssi } => {
                        let uuid = peripheral.id();
                        let mac = uuid.to_string();
                        
                        let name = advertisement_data
                            .local_name()
                            .map(|s| s.to_string());
                        
                        let rssi_value = rssi as i8;
                        
                        let device = DeviceModel {
                            mac_address: mac.clone(),
                            device_name: name,
                            device_type: DeviceType::BleOnly,
                            rssi: rssi_value,
                            avg_rssi: rssi_value as f64,
                            rssi_variance: 0.0,
                            first_seen: chrono::Utc::now(),
                            last_seen: chrono::Utc::now(),
                            response_time_ms: 0,
                            manufacturer_id: None,
                            manufacturer_name: None,
                            advertised_services: Vec::new(),
                            appearance: None,
                            tx_power: None,
                            mac_type: Some("Public".to_string()),
                            is_rpa: false,
                            security_level: None,
                            pairing_method: None,
                            is_connectable: true,
                            detection_count: 1,
                            last_rssi_values: vec![rssi_value],
                            discovered_services: Vec::new(),
                            vendor_protocols: Vec::new(),
                        };
                        
                        info!("   Discovered: {} (RSSI: {})", mac, rssi_value);
                        devices.push(device);
                    }
                    CentralEvent::PeripheralConnected { peripheral } => {
                        info!("   Connected: {}", peripheral.id());
                    }
                    CentralEvent::PeripheralDisconnected { peripheral, .. } => {
                        info!("   Disconnected: {}", peripheral.id());
                    }
                    _ => {}
                }
            }
        }
        
        info!("   CoreBluetooth scan complete: {} devices found", devices.len());
        Ok(devices)
    }

    /// Extract L2CAP channel information for a device
    pub async fn extract_l2cap_channels(
        &self,
        mac_address: &str,
    ) -> Result<Vec<L2CapChannel>, Box<dyn std::error::Error + Send + Sync>> {
        info!("   L2CAP extraction for {} via native API", mac_address);
        
        #[cfg(feature = "l2cap")]
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            info!("   Note: L2CAP channels require active connection");
        }

        #[cfg(not(feature = "l2cap"))]
        {
            warn!("L2CAP not available - enable 'l2cap' feature");
        }

        Ok(Vec::new())
    }

    /// Get device connection info
    pub async fn get_device_connection_info(
        &self,
        mac_address: &str,
    ) -> Result<DeviceConnectionInfo, Box<dyn std::error::Error + Send + Sync>> {
        info!("   Getting connection info for {}", mac_address);

        #[cfg(feature = "l2cap")]
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            Ok(DeviceConnectionInfo {
                mac_address: mac_address.to_string(),
                is_connected: false,
                l2cap_channels: Vec::new(),
                connection_state: ConnectionState::Disconnected,
            })
        }

        #[cfg(not(feature = "l2cap"))]
        {
            Err("CoreBluetooth not available - enable 'l2cap' feature".into())
        }

        #[cfg(feature = "l2cap")]
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        {
            Err("CoreBluetooth not available".into())
        }
    }
}

#[derive(Debug)]
pub struct DeviceConnectionInfo {
    pub mac_address: String,
    pub is_connected: bool,
    pub l2cap_channels: Vec<L2CapChannel>,
    pub connection_state: ConnectionState,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
}

#[derive(Debug, Clone)]
pub struct CoreBluetoothCapabilities {
    pub platform: &'static str,
    pub available: bool,
    pub l2cap_support: bool,
    pub gatt_support: bool,
}

impl CoreBluetoothCapabilities {
    pub fn current() -> Self {
        #[cfg(feature = "l2cap")]
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            Self {
                platform: if cfg!(target_os = "macos") { "macOS" } else { "iOS" },
                available: true,
                l2cap_support: true,
                gatt_support: true,
            }
        }

        #[cfg(not(feature = "l2cap"))]
        {
            Self {
                platform: "Other",
                available: false,
                l2cap_support: false,
                gatt_support: false,
            }
        }

        #[cfg(feature = "l2cap")]
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        {
            Self {
                platform: "Other",
                available: false,
                l2cap_support: false,
                gatt_support: false,
            }
        }
    }
}

pub struct EnhancedCoreBluetoothScanner {
    config: CoreBluetoothConfig,
    capabilities: CoreBluetoothCapabilities,
}

impl EnhancedCoreBluetoothScanner {
    pub fn new(config: CoreBluetoothConfig) -> Self {
        let capabilities = CoreBluetoothCapabilities::current();
        
        info!("🍎 Enhanced CoreBluetooth Scanner");
        info!("   Platform: {}", capabilities.platform);
        info!("   Available: {}", capabilities.available);

        Self {
            config,
            capabilities,
        }
    }

    pub async fn scan(&self) -> Result<Vec<DeviceModel>, Box<dyn std::error::Error + Send + Sync>> {
        if !self.capabilities.available {
            warn!("CoreBluetooth not available on this platform");
            return Ok(Vec::new());
        }

        let scanner = CoreBluetoothScanner::new(self.config.clone());
        scanner.scan().await
    }

    pub fn is_available(&self) -> bool {
        self.capabilities.available
    }

    pub fn platform(&self) -> &'static str {
        self.capabilities.platform
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_bluetooth_config_default() {
        let config = CoreBluetoothConfig::default();
        assert!(config.enabled);
        assert!(config.discover_services);
    }

    #[test]
    fn test_capabilities() {
        let caps = CoreBluetoothCapabilities::current();
        println!("Platform: {}, Available: {}", caps.platform, caps.available);
    }
}
