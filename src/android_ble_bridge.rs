//! Android BLE Bridge - Android Bluetooth integration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidBleConfig {
    pub enabled: bool,
    pub scan_duration_ms: u32,
    pub scan_mode: u8,
    pub discover_ble: bool,
    pub discover_bredr: bool,
    pub discover_gatt: bool,
    pub rssi_threshold: i8,
    pub parse_ad_data: bool,
    pub max_connections: u8,
}

impl Default for AndroidBleConfig {
    fn default() -> Self {
        AndroidBleConfig {
            enabled: true,
            scan_duration_ms: 30000,
            scan_mode: 0,
            discover_ble: true,
            discover_bredr: true,
            discover_gatt: true,
            rssi_threshold: -100,
            parse_ad_data: true,
            max_connections: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidBleDevice {
    pub address: String,
    pub name: Option<String>,
    pub rssi: i8,
    pub tx_power: Option<i8>,
    pub bonded: bool,
    pub device_type: String,
    pub advertised_services: Vec<String>,
    pub manufacturer_data: HashMap<u16, Vec<u8>>,
    pub service_data: HashMap<String, Vec<u8>>,
    pub connection_state: String,
    pub last_seen: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidGattProfile {
    pub device_address: String,
    pub connected: bool,
    pub services: Vec<GattService>,
    pub connection_interval_ms: u16,
    pub mtu: u16,
    pub phy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GattService {
    pub uuid: String,
    pub is_primary: bool,
    pub characteristics: Vec<GattCharacteristic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GattCharacteristic {
    pub uuid: String,
    pub properties: Vec<String>,
    pub descriptors: Vec<String>,
    pub value: Option<Vec<u8>>,
    pub notifications_enabled: bool,
}

pub struct AndroidBleScanner {
    config: AndroidBleConfig,
    devices: HashMap<String, AndroidBleDevice>,
    connected_devices: HashMap<String, AndroidGattProfile>,
    scan_active: bool,
}

impl AndroidBleScanner {
    pub fn new(config: AndroidBleConfig) -> Self {
        AndroidBleScanner {
            config,
            devices: HashMap::new(),
            connected_devices: HashMap::new(),
            scan_active: false,
        }
    }

    pub fn default() -> Self {
        Self::new(AndroidBleConfig::default())
    }

    pub fn start_scan(&mut self) -> Result<(), String> {
        if self.scan_active {
            return Err("Scan already in progress".to_string());
        }
        self.scan_active = true;
        self.devices.clear();
        Ok(())
    }

    pub fn stop_scan(&mut self) -> Result<(), String> {
        if !self.scan_active {
            return Err("Scan not in progress".to_string());
        }
        self.scan_active = false;
        Ok(())
    }

    pub fn add_device(&mut self, device: AndroidBleDevice) {
        self.devices.insert(device.address.clone(), device);
    }

    pub fn get_devices(&self) -> Vec<AndroidBleDevice> {
        self.devices.values().cloned().collect()
    }

    pub fn connect_device(&mut self, address: &str) -> Result<(), String> {
        if self.connected_devices.len() >= self.config.max_connections as usize {
            return Err("Max connections reached".to_string());
        }
        let profile = AndroidGattProfile {
            device_address: address.to_string(),
            connected: true,
            services: Vec::new(),
            connection_interval_ms: 30,
            mtu: 517,
            phy: "LE 1M".to_string(),
        };
        self.connected_devices.insert(address.to_string(), profile);
        Ok(())
    }

    pub fn disconnect_device(&mut self, address: &str) -> Result<(), String> {
        if let Some(profile) = self.connected_devices.get_mut(address) {
            profile.connected = false;
            self.connected_devices.remove(address);
            Ok(())
        } else {
            Err("Device not connected".to_string())
        }
    }

    pub fn discover_services(&mut self, address: &str) -> Result<(), String> {
        if let Some(profile) = self.connected_devices.get_mut(address) {
            profile.services.push(GattService {
                uuid: "0000180A-0000-1000-8000-00805F9B34FB".to_string(),
                is_primary: true,
                characteristics: vec![],
            });
            Ok(())
        } else {
            Err("Device not connected".to_string())
        }
    }

    pub fn get_connected_devices(&self) -> Vec<AndroidGattProfile> {
        self.connected_devices.values().cloned().collect()
    }

    pub fn get_stats(&self) -> (u32, u32) {
        (self.devices.len() as u32, self.connected_devices.len() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_creation() {
        let scanner = AndroidBleScanner::default();
        assert!(scanner.config.enabled);
    }

    #[test]
    fn test_start_stop() {
        let mut scanner = AndroidBleScanner::default();
        assert!(scanner.start_scan().is_ok());
        assert!(scanner.stop_scan().is_ok());
    }
}
