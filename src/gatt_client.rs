use btleplug::api::{Central, Manager as ManagerTrait, Peripheral as PeripheralTrait};
use btleplug::platform::Manager;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Duration;
use tokio::time::timeout;

/// GATT Service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GattService {
    pub uuid16: Option<u16>,
    pub uuid128: Option<String>,
    pub name: Option<String>,
    pub is_primary: bool,
    pub characteristics: Vec<GattCharacteristic>,
}

/// GATT Characteristic information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GattCharacteristic {
    pub uuid16: Option<u16>,
    pub uuid128: Option<String>,
    pub name: Option<String>,
    pub properties: CharacteristicProperties,
    pub value: Option<Vec<u8>>,
    pub descriptors: Vec<GattDescriptor>,
}

/// Characteristic property flags
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CharacteristicProperties {
    pub broadcast: bool,
    pub read: bool,
    pub write_without_response: bool,
    pub write: bool,
    pub notify: bool,
    pub indicate: bool,
    pub authenticated_signed_writes: bool,
    pub extended_properties: bool,
}

impl CharacteristicProperties {
    pub fn from_byte(byte: u8) -> Self {
        Self {
            broadcast: (byte & 0x01) != 0,
            read: (byte & 0x02) != 0,
            write_without_response: (byte & 0x04) != 0,
            write: (byte & 0x08) != 0,
            notify: (byte & 0x10) != 0,
            indicate: (byte & 0x20) != 0,
            authenticated_signed_writes: (byte & 0x40) != 0,
            extended_properties: (byte & 0x80) != 0,
        }
    }

    pub fn to_byte(&self) -> u8 {
        let mut byte = 0u8;
        if self.broadcast {
            byte |= 0x01;
        }
        if self.read {
            byte |= 0x02;
        }
        if self.write_without_response {
            byte |= 0x04;
        }
        if self.write {
            byte |= 0x08;
        }
        if self.notify {
            byte |= 0x10;
        }
        if self.indicate {
            byte |= 0x20;
        }
        if self.authenticated_signed_writes {
            byte |= 0x40;
        }
        if self.extended_properties {
            byte |= 0x80;
        }
        byte
    }

    pub fn properties_list(&self) -> Vec<String> {
        let mut props = Vec::new();
        if self.broadcast {
            props.push("Broadcast".to_string());
        }
        if self.read {
            props.push("Read".to_string());
        }
        if self.write_without_response {
            props.push("Write Without Response".to_string());
        }
        if self.write {
            props.push("Write".to_string());
        }
        if self.notify {
            props.push("Notify".to_string());
        }
        if self.indicate {
            props.push("Indicate".to_string());
        }
        if self.authenticated_signed_writes {
            props.push("Authenticated Signed Writes".to_string());
        }
        if self.extended_properties {
            props.push("Extended Properties".to_string());
        }
        props
    }
}

/// GATT Descriptor information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GattDescriptor {
    pub uuid16: Option<u16>,
    pub uuid128: Option<String>,
    pub name: Option<String>,
    pub value: Option<Vec<u8>>,
}

/// GATT Client for discovering device services
pub struct GattClient {
    pub mac_address: String,
    pub services: Vec<GattService>,
}

impl GattClient {
    pub fn new(mac_address: String) -> Self {
        Self {
            mac_address,
            services: Vec::new(),
        }
    }

    /// Simulate service discovery (in real implementation would use btleplug)
    pub async fn discover_services(&mut self) -> Result<(), String> {
        info!("Discovering services for {}", self.mac_address);

        // Note: In a real implementation, this would:
        // 1. Connect to the device
        // 2. Execute discover_services() via btleplug
        // 3. Parse the returned services and characteristics
        // 4. For each characteristic, query descriptors

        // For now, return empty services
        Ok(())
    }

    /// Discover GATT services using btleplug - connects to device and reads services/characteristics
    pub async fn discover_services_btleplug(&mut self) -> Result<Vec<GattService>, String> {
        info!("GATT Discovery: Starting for {}", self.mac_address);

        // Parse MAC address
        let mac = btleplug::api::BDAddr::from_str(&self.mac_address.replace('-', ":").to_uppercase())
            .map_err(|e| format!("Invalid MAC address: {}", e))?;

        // Get platform manager
        let manager = Manager::new().await.map_err(|e| format!("Failed to create manager: {}", e))?;
        
        // Get adapters
        let adapters = manager.adapters().await.map_err(|e| format!("No adapters found: {}", e))?;
        if adapters.is_empty() {
            return Err("No Bluetooth adapters available".to_string());
        }

        // Find the peripheral - get properties for each peripheral to match MAC
        let adapter = &adapters[0];
        let peripherals = adapter.peripherals().await
            .map_err(|e| format!("Failed to list peripherals: {}", e))?;
        
        let mut peripheral = None;
        for p in peripherals {
            if let Ok(Some(props)) = p.properties().await {
                if props.address == mac {
                    peripheral = Some(p);
                    break;
                }
            }
        }

        let peripheral = peripheral.ok_or_else(|| format!("Device {} not found - device must be advertising", self.mac_address))?;

        // Connect with timeout
        info!("GATT Discovery: Connecting to {}", self.mac_address);
        match timeout(Duration::from_secs(10), peripheral.connect()).await {
            Ok(Ok(_)) => {
                info!("GATT Discovery: Connected to {}", self.mac_address);
            }
            Ok(Err(e)) => {
                return Err(format!("Failed to connect: {}", e));
            }
            Err(_) => {
                return Err("Connection timeout".to_string());
            }
        }

        // Discover services with timeout
        match timeout(Duration::from_secs(10), peripheral.discover_services()).await {
            Ok(Ok(_)) => {
                info!("GATT Discovery: Services discovered");
            }
            Ok(Err(e)) => {
                let _ = peripheral.disconnect().await;
                return Err(format!("Service discovery failed: {}", e));
            }
            Err(_) => {
                let _ = peripheral.disconnect().await;
                return Err("Service discovery timeout".to_string());
            }
        };

        // Get services from peripheral after discovery - services() returns a BTreeSet directly
        let discovered_services = peripheral.services();

        info!("GATT Discovery: Found {} services", discovered_services.len());

        // Parse services and characteristics
        let mut gatt_services = Vec::new();

        for service in &discovered_services {
            let uuid = service.uuid.clone();
            
            // Try to parse UUID
            let uuid_str = uuid.to_string();
            let uuid128 = if uuid_str.len() == 4 {
                // 16-bit UUID
                None
            } else {
                Some(uuid_str.clone())
            };
            
            let uuid16 = if uuid_str.len() == 4 {
                u16::from_str_radix(&uuid_str[0..4], 16).ok()
            } else {
                None
            };

            // Get service name from standard UUIDs
            let name = uuid16
                .and_then(|id| get_gatt_service_name(id))
                .map(|s| s.to_string());

            // Get characteristics
            let mut characteristics = Vec::new();
            
            for char in &service.characteristics {
                let char_uuid = char.uuid.clone();
                let char_uuid_str = char_uuid.to_string();
                
                let char_uuid128 = if char_uuid_str.len() == 4 {
                    None
                } else {
                    Some(char_uuid_str.clone())
                };
                
                let char_uuid16 = if char_uuid_str.len() == 4 {
                    u16::from_str_radix(&char_uuid_str[0..4], 16).ok()
                } else {
                    None
                };

                let char_name = char_uuid16
                    .and_then(|id| get_gatt_characteristic_name(id))
                    .map(|s| s.to_string());

                let properties = CharacteristicProperties::from_byte(char.properties.bits() as u8);

                characteristics.push(GattCharacteristic {
                    uuid16: char_uuid16,
                    uuid128: char_uuid128,
                    name: char_name,
                    properties,
                    value: None,
                    descriptors: Vec::new(),
                });
            }

            gatt_services.push(GattService {
                uuid16,
                uuid128,
                name,
                is_primary: true,
                characteristics,
            });
        }

        self.services = gatt_services.clone();

        // Disconnect
        let _ = peripheral.disconnect().await;

        info!("GATT Discovery: Completed for {} - {} services", self.mac_address, gatt_services.len());
        
        Ok(gatt_services)
    }

    /// Read characteristic value
    pub async fn read_characteristic(
        &self,
        service_uuid: &str,
        char_uuid: &str,
    ) -> Result<Vec<u8>, String> {
        debug!(
            "Reading characteristic {} from service {}",
            char_uuid, service_uuid
        );

        // Find the service and characteristic
        for service in &self.services {
            let service_match = service
                .uuid128
                .as_ref()
                .map(|u| u == service_uuid)
                .unwrap_or(false);

            if service_match {
                for characteristic in &service.characteristics {
                    let char_match = characteristic
                        .uuid128
                        .as_ref()
                        .map(|u| u == char_uuid)
                        .unwrap_or(false);

                    if char_match {
                        if !characteristic.properties.read {
                            return Err("Characteristic not readable".to_string());
                        }

                        if let Some(value) = &characteristic.value {
                            return Ok(value.clone());
                        }
                    }
                }
            }
        }

        Err("Service or characteristic not found".to_string())
    }

    /// Write characteristic value
    pub async fn write_characteristic(
        &mut self,
        service_uuid: &str,
        char_uuid: &str,
        value: Vec<u8>,
    ) -> Result<(), String> {
        debug!(
            "Writing to characteristic {} in service {}",
            char_uuid, service_uuid
        );

        // Find and update the characteristic
        for service in &mut self.services {
            let service_match = service
                .uuid128
                .as_ref()
                .map(|u| u == service_uuid)
                .unwrap_or(false);

            if service_match {
                for characteristic in &mut service.characteristics {
                    let char_match = characteristic
                        .uuid128
                        .as_ref()
                        .map(|u| u == char_uuid)
                        .unwrap_or(false);

                    if char_match {
                        if !characteristic.properties.write
                            && !characteristic.properties.write_without_response
                        {
                            return Err("Characteristic not writable".to_string());
                        }

                        characteristic.value = Some(value);
                        return Ok(());
                    }
                }
            }
        }

        Err("Service or characteristic not found".to_string())
    }

    /// Get summary of all discovered services
    pub fn get_summary(&self) -> GattSummary {
        let mut summary = GattSummary {
            mac_address: self.mac_address.clone(),
            service_count: self.services.len(),
            characteristic_count: 0,
            descriptor_count: 0,
            readable_characteristics: 0,
            writable_characteristics: 0,
            notify_characteristics: 0,
            indicate_characteristics: 0,
        };

        for service in &self.services {
            for characteristic in &service.characteristics {
                summary.characteristic_count += 1;
                summary.descriptor_count += characteristic.descriptors.len();

                if characteristic.properties.read {
                    summary.readable_characteristics += 1;
                }
                if characteristic.properties.write
                    || characteristic.properties.write_without_response
                {
                    summary.writable_characteristics += 1;
                }
                if characteristic.properties.notify {
                    summary.notify_characteristics += 1;
                }
                if characteristic.properties.indicate {
                    summary.indicate_characteristics += 1;
                }
            }
        }

        summary
    }
}

/// GATT discovery summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GattSummary {
    pub mac_address: String,
    pub service_count: usize,
    pub characteristic_count: usize,
    pub descriptor_count: usize,
    pub readable_characteristics: usize,
    pub writable_characteristics: usize,
    pub notify_characteristics: usize,
    pub indicate_characteristics: usize,
}

/// Parse GATT characteristic properties
pub fn parse_characteristic_properties(byte: u8) -> CharacteristicProperties {
    CharacteristicProperties::from_byte(byte)
}

/// Get standard GATT Service UUID names
pub fn get_gatt_service_name(uuid16: u16) -> Option<&'static str> {
    match uuid16 {
        0x1800 => Some("Generic Access"),
        0x1801 => Some("Generic Attribute"),
        0x1802 => Some("Immediate Alert"),
        0x1803 => Some("Link Loss"),
        0x1804 => Some("Tx Power"),
        0x1805 => Some("Current Time"),
        0x1806 => Some("Reference Time Update"),
        0x1807 => Some("Next DST Change"),
        0x1808 => Some("Glucose"),
        0x1809 => Some("Health Thermometer"),
        0x180A => Some("Device Information"),
        0x180B => Some("Network Availability"),
        0x180C => Some("Watchdog"),
        0x180D => Some("Heart Rate"),
        0x180E => Some("Phone Alert Status"),
        0x180F => Some("Battery"),
        0x1810 => Some("Blood Pressure"),
        0x1811 => Some("Alert Notification"),
        0x1812 => Some("Human Interface Device"),
        0x1813 => Some("Scan Parameters"),
        0x1814 => Some("Running Speed and Cadence"),
        0x1815 => Some("Automation IO"),
        0x1816 => Some("Cycling Speed and Cadence"),
        0x1817 => Some("Cycling Power"),
        0x1818 => Some("Location and Navigation"),
        0x1819 => Some("Environmental Sensing"),
        0x181A => Some("Body Composition"),
        0x181B => Some("User Data"),
        0x181C => Some("Weight Scale"),
        0x181D => Some("Bond Management"),
        0x181E => Some("Continuous Glucose Monitoring"),
        0x181F => Some("Internet Protocol Support"),
        0x1820 => Some("Indoor Positioning"),
        0x1821 => Some("Pulse Oximeter"),
        0x1822 => Some("HTTP Proxy"),
        0x1823 => Some("Transport Discovery"),
        0x1824 => Some("Object Transfer"),
        0x1825 => Some("Mesh Provisioning"),
        0x1826 => Some("Mesh Proxy"),
        0x1827 => Some("Reconnection"),
        0x1828 => Some("Insulin Delivery"),
        0x1829 => Some("Binary Sensor"),
        0x182A => Some("Emergency Configuration"),
        0x182B => Some("Authorization and Authentication"),
        0x182C => Some("Fitness Machine"),
        0x182D => Some("Mesh Beacon"),
        0x182E => Some("Big Data Transfer"),
        0x182F => Some("Lighting and Control"),
        0x1830 => Some("QALE"),
        0x1831 => Some("Air and Water Quality"),
        0x1832 => Some("Personal Mobility"),
        0x1833 => Some("Electronic Shelf Label"),
        0x1834 => Some("Microphone Control"),
        _ => None,
    }
}

/// Get standard GATT Characteristic UUID names
pub fn get_gatt_characteristic_name(uuid16: u16) -> Option<&'static str> {
    match uuid16 {
        0x2A00 => Some("Device Name"),
        0x2A01 => Some("Appearance"),
        0x2A02 => Some("Peripheral Privacy Flag"),
        0x2A03 => Some("Reconnection Address"),
        0x2A04 => Some("Peripheral Preferred Connection Parameters"),
        0x2A05 => Some("Service Changed"),
        0x2A37 => Some("Heart Rate Measurement"),
        0x2A38 => Some("Body Sensor Location"),
        0x2A39 => Some("Heart Rate Control Point"),
        0x2A47 => Some("IEEE 11073-20601 Regulatory Certification Data List"),
        0x2A50 => Some("Glucose Measurement"),
        0x2A51 => Some("Glucose Measurement Context"),
        0x2A52 => Some("Glucose Features"),
        0x2A53 => Some("Record Access Control Point"),
        0x2A19 => Some("Battery Level"),
        0x2A49 => Some("Blood Pressure Feature"),
        0x2A35 => Some("Blood Pressure Measurement"),
        0x2A5C => Some("CSC Feature"),
        0x2A5B => Some("CSC Measurement"),
        0x2A2B => Some("Current Time"),
        0x2A0D => Some("Date of Birth"),
        0x2A0E => Some("Date of Death"),
        0x2A29 => Some("Manufacturer Name String"),
        0x2A24 => Some("Model Number String"),
        0x2A25 => Some("Serial Number String"),
        0x2A27 => Some("Hardware Revision String"),
        0x2A26 => Some("Firmware Revision String"),
        0x2A28 => Some("Software Revision String"),
        0x2A23 => Some("System ID"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_characteristic_properties() {
        let props = CharacteristicProperties::from_byte(0x12);
        assert!(props.read);
        assert!(props.notify);
        assert!(!props.write);
    }

    #[test]
    fn test_properties_list() {
        let props = CharacteristicProperties::from_byte(0x02);
        let list = props.properties_list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], "Read");
    }

    #[test]
    fn test_gatt_service_names() {
        assert_eq!(get_gatt_service_name(0x180D), Some("Heart Rate"));
        assert_eq!(get_gatt_service_name(0x180A), Some("Device Information"));
    }

    #[test]
    fn test_gatt_characteristic_names() {
        assert_eq!(
            get_gatt_characteristic_name(0x2A37),
            Some("Heart Rate Measurement")
        );
        assert_eq!(get_gatt_characteristic_name(0x2A19), Some("Battery Level"));
    }

    #[test]
    fn test_gatt_summary() {
        let client = GattClient::new("AA:BB:CC:DD:EE:FF".to_string());
        let summary = client.get_summary();
        assert_eq!(summary.service_count, 0);
        assert_eq!(summary.characteristic_count, 0);
    }
}
