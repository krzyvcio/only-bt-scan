use std::error::Error;
use std::fmt;

#[cfg(target_os = "windows")]
use btleplug::api::{Central, Manager as BtManager, Peripheral};
#[cfg(target_os = "windows")]
use btleplug::platform::Manager as PlatformManager;

#[cfg(target_os = "macos")]
use btleplug::api::{Central, Manager as BtManager, Peripheral};
#[cfg(target_os = "macos")]
use btleplug::platform::Manager as PlatformManager;

#[cfg(target_os = "linux")]
use bluer::{Adapter, Device, Session};

#[derive(Debug, Clone)]
pub struct PairedDevice {
    pub name: Option<String>,
    pub address: String,
    pub connected: bool,
    pub device_type: DeviceType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceType {
    Ble,
    Classic,
    DualMode,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceType::Ble => write!(f, "BLE"),
            DeviceType::Classic => write!(f, "Classic"),
            DeviceType::DualMode => write!(f, "Dual"),
        }
    }
}

#[derive(Debug)]
pub enum BluetoothManagerError {
    NoAdapter,
    DeviceNotFound,
    ConnectionFailed(String),
    DisconnectionFailed(String),
    NotSupported(String),
    Other(String),
}

impl fmt::Display for BluetoothManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BluetoothManagerError::NoAdapter => write!(f, "No Bluetooth adapter found"),
            BluetoothManagerError::DeviceNotFound => write!(f, "Device not found"),
            BluetoothManagerError::ConnectionFailed(e) => write!(f, "Connection failed: {}", e),
            BluetoothManagerError::DisconnectionFailed(e) => write!(f, "Disconnection failed: {}", e),
            BluetoothManagerError::NotSupported(e) => write!(f, "Not supported: {}", e),
            BluetoothManagerError::Other(e) => write!(f, "Error: {}", e),
        }
    }
}

impl Error for BluetoothManagerError {}

pub struct BluetoothManager;

impl BluetoothManager {
    pub fn new() -> Self {
        BluetoothManager
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub async fn list_paired_devices(&self) -> Result<Vec<PairedDevice>, Box<dyn Error>> {
        let manager = PlatformManager::new().await?;
        let adapters = manager.adapters().await?;
        
        if adapters.is_empty() {
            return Err(Box::new(BluetoothManagerError::NoAdapter));
        }

        let central = &adapters[0];
        let peripherals = central.peripherals().await?;
        
        let mut devices = Vec::new();
        for peripheral in peripherals {
            let props_result = peripheral.properties().await;
            if let Ok(Some(props)) = props_result {
                let name = props.local_name;
                let address = props.address.to_string();
                let connected = peripheral.is_connected().await.unwrap_or(false);
                
                let device_type = DeviceType::Ble;
                
                devices.push(PairedDevice {
                    name,
                    address,
                    connected,
                    device_type,
                });
            }
        }
        
        Ok(devices)
    }

    #[cfg(target_os = "linux")]
    pub async fn list_paired_devices(&self) -> Result<Vec<PairedDevice>, Box<dyn Error>> {
        let session = Session::new().await?;
        let adapter_names = session.adapter_names().await?;
        
        if adapter_names.is_empty() {
            return Err(Box::new(BluetoothManagerError::NoAdapter));
        }

        let adapter = session.adapter(&adapter_names[0])?;
        let devices = adapter.paired_devices().await?;
        
        let mut paired_devices = Vec::new();
        for device in devices {
            let address = device.address.to_string();
            let name = device.name().await.ok().flatten();
            let connected = device.is_connected().await.unwrap_or(false);
            
            let device_type = if device.is_le().await.unwrap_or(false) && device.is_bredr().await.unwrap_or(false) {
                DeviceType::DualMode
            } else if device.is_le().await.unwrap_or(false) {
                DeviceType::Ble
            } else {
                DeviceType::Classic
            };
            
            paired_devices.push(PairedDevice {
                name,
                address,
                connected,
                device_type,
            });
        }
        
        Ok(paired_devices)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub async fn list_paired_devices(&self) -> Result<Vec<PairedDevice>, Box<dyn Error>> {
        Err(Box::new(BluetoothManagerError::NotSupported(
            "Bluetooth management not supported on this platform".to_string(),
        )))
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub async fn connect_device(&self, identifier: &str) -> Result<(), Box<dyn Error>> {
        let manager = PlatformManager::new().await?;
        let adapters = manager.adapters().await?;
        
        if adapters.is_empty() {
            return Err(Box::new(BluetoothManagerError::NoAdapter));
        }

        let central = &adapters[0];
        let peripherals = central.peripherals().await?;
        
        let mut target = None;
        for p in peripherals {
            if let Ok(Some(props)) = p.properties().await {
                let matches_name = props.local_name
                    .as_ref()
                    .map(|n| n.to_lowercase().contains(&identifier.to_lowercase()))
                    .unwrap_or(false);
                let matches_addr = props.address.to_string().to_lowercase() == identifier.to_lowercase();
                if matches_name || matches_addr {
                    target = Some(p);
                    break;
                }
            }
        }

        let target = target.ok_or_else(|| Box::new(BluetoothManagerError::DeviceNotFound))?;

        if !target.is_connected().await? {
            target.connect().await?;
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub async fn connect_device(&self, identifier: &str) -> Result<(), Box<dyn Error>> {
        let session = Session::new().await?;
        let adapter_names = session.adapter_names().await?;
        
        if adapter_names.is_empty() {
            return Err(Box::new(BluetoothManagerError::NoAdapter));
        }

        let adapter = session.adapter(&adapter_names[0])?;
        let devices = adapter.paired_devices().await?;
        
        let target = devices
            .into_iter()
            .find(|d| {
                let addr = d.address.to_string().to_lowercase();
                let name_match = d.name().await.ok().flatten()
                    .map(|n| n.to_lowercase().contains(&identifier.to_lowercase()))
                    .unwrap_or(false);
                let addr_match = addr == identifier.to_lowercase();
                name_match || addr_match
            })
            .ok_or_else(|| Box::new(BluetoothManagerError::DeviceNotFound))?;

        if !target.is_connected().await? {
            target.connect().await?;
        }
        
        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub async fn connect_device(&self, _identifier: &str) -> Result<(), Box<dyn Error>> {
        Err(Box::new(BluetoothManagerError::NotSupported(
            "Bluetooth connection not supported on this platform".to_string(),
        )))
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub async fn disconnect_device(&self, identifier: &str) -> Result<(), Box<dyn Error>> {
        let manager = PlatformManager::new().await?;
        let adapters = manager.adapters().await?;
        
        if adapters.is_empty() {
            return Err(Box::new(BluetoothManagerError::NoAdapter));
        }

        let central = &adapters[0];
        let peripherals = central.peripherals().await?;
        
        let mut target = None;
        for p in peripherals {
            if let Ok(Some(props)) = p.properties().await {
                let matches_name = props.local_name
                    .as_ref()
                    .map(|n| n.to_lowercase().contains(&identifier.to_lowercase()))
                    .unwrap_or(false);
                let matches_addr = props.address.to_string().to_lowercase() == identifier.to_lowercase();
                if matches_name || matches_addr {
                    target = Some(p);
                    break;
                }
            }
        }

        let target = target.ok_or_else(|| Box::new(BluetoothManagerError::DeviceNotFound))?;

        if target.is_connected().await? {
            target.disconnect().await?;
        }
        
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub async fn disconnect_device(&self, identifier: &str) -> Result<(), Box<dyn Error>> {
        let session = Session::new().await?;
        let adapter_names = session.adapter_names().await?;
        
        if adapter_names.is_empty() {
            return Err(Box::new(BluetoothManagerError::NoAdapter));
        }

        let adapter = session.adapter(&adapter_names[0])?;
        let devices = adapter.paired_devices().await?;
        
        let target = devices
            .into_iter()
            .find(|d| {
                let addr = d.address.to_string().to_lowercase();
                let name_match = d.name().await.ok().flatten()
                    .map(|n| n.to_lowercase().contains(&identifier.to_lowercase()))
                    .unwrap_or(false);
                let addr_match = addr == identifier.to_lowercase();
                name_match || addr_match
            })
            .ok_or_else(|| Box::new(BluetoothManagerError::DeviceNotFound))?;

        if target.is_connected().await? {
            target.disconnect().await?;
        }
        
        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub async fn disconnect_device(&self, _identifier: &str) -> Result<(), Box<dyn Error>> {
        Err(Box::new(BluetoothManagerError::NotSupported(
            "Bluetooth disconnection not supported on this platform".to_string(),
        )))
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub async fn is_device_connected(&self, identifier: &str) -> Result<bool, Box<dyn Error>> {
        let manager = PlatformManager::new().await?;
        let adapters = manager.adapters().await?;
        
        if adapters.is_empty() {
            return Err(Box::new(BluetoothManagerError::NoAdapter));
        }

        let central = &adapters[0];
        let peripherals = central.peripherals().await?;
        
        for peripheral in peripherals {
            if let Ok(Some(props)) = peripheral.properties().await {
                let matches_name = props.local_name
                    .as_ref()
                    .map(|n| n.to_lowercase().contains(&identifier.to_lowercase()))
                    .unwrap_or(false);
                let matches_addr = props.address.to_string().to_lowercase() == identifier.to_lowercase();
                
                if matches_name || matches_addr {
                    return peripheral.is_connected().await.map_err(|e| {
                        Box::new(BluetoothManagerError::Other(e.to_string())) as Box<dyn Error>
                    });
                }
            }
        }
        
        Ok(false)
    }

    #[cfg(target_os = "linux")]
    pub async fn is_device_connected(&self, identifier: &str) -> Result<bool, Box<dyn Error>> {
        let session = Session::new().await?;
        let adapter_names = session.adapter_names().await?;
        
        if adapter_names.is_empty() {
            return Err(Box::new(BluetoothManagerError::NoAdapter));
        }

        let adapter = session.adapter(&adapter_names[0])?;
        let devices = adapter.paired_devices().await?;
        
        for device in devices {
            let addr = device.address.to_string().to_lowercase();
            let name_match = device.name().await.ok().flatten()
                .map(|n| n.to_lowercase().contains(&identifier.to_lowercase()))
                .unwrap_or(false);
            let addr_match = addr == identifier.to_lowercase();
            
            if name_match || addr_match {
                return device.is_connected().await.map_err(|e| 
                    Box::new(BluetoothManagerError::Other(e.to_string())) as Box<dyn Error>
                );
            }
        }
        
        Ok(false)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub async fn is_device_connected(&self, _identifier: &str) -> Result<bool, Box<dyn Error>> {
        Err(Box::new(BluetoothManagerError::NotSupported(
            "Bluetooth status not supported on this platform".to_string(),
        )))
    }
}
