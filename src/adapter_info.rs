#![allow(dead_code)]

/// Bluetooth Adapter Information Discovery
/// Shows available adapters and their capabilities

use log::info;
use std::fmt;

#[derive(Debug, Clone)]
pub struct AdapterInfo {
    pub name: String,
    pub address: String,
    pub is_powered: bool,
    pub is_connectable: bool,
    pub is_discoverable: bool,
    pub supported_modes: Vec<BluetoothMode>,
    pub supported_phys: Vec<BluetoothPhy>,
    pub bt_version: Option<String>,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BluetoothMode {
    BLE,
    BrEdr,
    DualMode,
}

impl fmt::Display for BluetoothMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BluetoothMode::BLE => write!(f, "BLE (Bluetooth Low Energy)"),
            BluetoothMode::BrEdr => write!(f, "BR/EDR (Bluetooth Classic)"),
            BluetoothMode::DualMode => write!(f, "DUAL MODE (BLE + BR/EDR)"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BluetoothPhy {
    Le1M,
    Le2M,
    LeCoded,
    BrEdrBasic,
    BrEdrEdr,
}

impl fmt::Display for BluetoothPhy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BluetoothPhy::Le1M => write!(f, "LE 1M PHY"),
            BluetoothPhy::Le2M => write!(f, "LE 2M PHY (High Speed)"),
            BluetoothPhy::LeCoded => write!(f, "LE Coded PHY (Long Range)"),
            BluetoothPhy::BrEdrBasic => write!(f, "BR/EDR Basic Rate"),
            BluetoothPhy::BrEdrEdr => write!(f, "BR/EDR Enhanced Data Rate"),
        }
    }
}

impl AdapterInfo {
    /// Create a new adapter info with defaults
    pub fn new(name: String, address: String) -> Self {
        Self {
            name,
            address,
            is_powered: false,
            is_connectable: false,
            is_discoverable: false,
            supported_modes: vec![],
            supported_phys: vec![],
            bt_version: None,
            features: vec![],
        }
    }

    /// Get default system adapter (simulated for cross-platform compatibility)
    pub fn get_default_adapter() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self::get_windows_adapter()
        }

        #[cfg(target_os = "linux")]
        {
            Self::get_linux_adapter()
        }

        #[cfg(target_os = "macos")]
        {
            Self::get_macos_adapter()
        }
    }

    #[cfg(target_os = "windows")]
    fn get_windows_adapter() -> Self {
        let mut adapter = Self::new(
            "Windows Bluetooth Adapter".to_string(),
            Self::get_windows_mac_address().unwrap_or_else(|| "XX:XX:XX:XX:XX:XX".to_string()),
        );

        adapter.is_powered = true;
        adapter.is_connectable = true;
        adapter.is_discoverable = true;
        adapter.supported_modes = vec![BluetoothMode::BLE, BluetoothMode::DualMode];
        adapter.supported_phys = vec![BluetoothPhy::Le1M, BluetoothPhy::Le2M];
        adapter.bt_version = Some("Bluetooth 5.2+".to_string());
        adapter.features = vec![
            "Classic Pairing".to_string(),
            "LE Secure Connections".to_string(),
            "Extended Advertising".to_string(),
            "LE Coded PHY".to_string(),
            "LE 2M PHY".to_string(),
            "Simultaneous LE/BR-EDR".to_string(),
        ];

        adapter
    }

    #[cfg(target_os = "linux")]
    fn get_linux_adapter() -> Self {
        let mut adapter = Self::new(
            "hci0 (Linux BlueZ)".to_string(),
            Self::get_linux_mac_address().unwrap_or_else(|| "XX:XX:XX:XX:XX:XX".to_string()),
        );

        adapter.is_powered = true;
        adapter.is_connectable = true;
        adapter.is_discoverable = true;
        adapter.supported_modes = vec![
            BluetoothMode::BLE,
            BluetoothMode::BrEdr,
            BluetoothMode::DualMode,
        ];
        adapter.supported_phys = vec![
            BluetoothPhy::Le1M,
            BluetoothPhy::Le2M,
            BluetoothPhy::LeCoded,
            BluetoothPhy::BrEdrBasic,
            BluetoothPhy::BrEdrEdr,
        ];
        adapter.bt_version = Some("Bluetooth 5.3".to_string());
        adapter.features = vec![
            "Full BR/EDR Support".to_string(),
            "LE Secure Connections".to_string(),
            "Extended Advertising".to_string(),
            "LE Coded PHY (Long Range)".to_string(),
            "LE 2M PHY (High Speed)".to_string(),
            "Simultaneous LE/BR-EDR".to_string(),
            "LE isochronous channels".to_string(),
            "HCI Raw Socket Access".to_string(),
            "Raw Packet Sniffing".to_string(),
        ];

        adapter
    }

    #[cfg(target_os = "macos")]
    fn get_macos_adapter() -> Self {
        let mut adapter = Self::new(
            "macOS Built-in Bluetooth".to_string(),
            Self::get_macos_mac_address().unwrap_or_else(|| "XX:XX:XX:XX:XX:XX".to_string()),
        );

        adapter.is_powered = true;
        adapter.is_connectable = true;
        adapter.is_discoverable = true;
        adapter.supported_modes = vec![BluetoothMode::BLE, BluetoothMode::BrEdr, BluetoothMode::DualMode];
        adapter.supported_phys = vec![
            BluetoothPhy::Le1M,
            BluetoothPhy::Le2M,
            BluetoothPhy::LeCoded,
            BluetoothPhy::BrEdrBasic,
            BluetoothPhy::BrEdrEdr,
        ];
        adapter.bt_version = Some("Bluetooth 5.2+".to_string());
        adapter.features = vec![
            "BR/EDR with LE".to_string(),
            "LE Secure Connections".to_string(),
            "Extended Advertising".to_string(),
            "LE Coded PHY".to_string(),
            "LE 2M PHY".to_string(),
            "Simultaneous LE/BR-EDR".to_string(),
            "Continuity".to_string(),
        ];

        adapter
    }

    #[cfg(target_os = "windows")]
    fn get_windows_mac_address() -> Option<String> {
        use std::process::Command;

        let output = Command::new("powershell")
            .arg("-Command")
            .arg("(Get-NetAdapter -Physical | Select-Object -First 1 | Get-NetAdapterAdvancedProperty -RegistryKeyword hw_address).DisplayValue")
            .output();

        match output {
            Ok(out) => {
                let result = String::from_utf8_lossy(&out.stdout);
                if !result.trim().is_empty() {
                    Some(result.trim().to_string())
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    #[cfg(target_os = "linux")]
    fn get_linux_mac_address() -> Option<String> {
        use std::process::Command;

        let output = Command::new("hciconfig")
            .arg("hci0")
            .output();

        match output {
            Ok(out) => {
                let result = String::from_utf8_lossy(&out.stdout);
                // Parse MAC from hciconfig output
                for line in result.lines() {
                    if let Some(start) = line.find("BD Address: ") {
                        let mac = &line[start + 12..];
                        if let Some(end) = mac.find(' ') {
                            return Some(mac[..end].to_string());
                        }
                        return Some(mac.to_string());
                    }
                }
                None
            }
            Err(_) => None,
        }
    }

    #[cfg(target_os = "macos")]
    fn get_macos_mac_address() -> Option<String> {
        use std::process::Command;

        let output = Command::new("system_profiler")
            .arg("SPBluetoothDataType")
            .output();

        match output {
            Ok(out) => {
                let result = String::from_utf8_lossy(&out.stdout);
                for line in result.lines() {
                    if line.contains("Address:") {
                        if let Some(addr) = line.split("Address:").nth(1) {
                            return Some(addr.trim().to_string());
                        }
                    }
                }
                None
            }
            Err(_) => None,
        }
    }
}

/// Display adapter information in formatted output
pub fn display_adapter_info(adapter: &AdapterInfo) {
    println!("╔════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                      📱 BLUETOOTH ADAPTER INFORMATION                          ║");
    println!("╠════════════════════════════════════════════════════════════════════════════════╣");
    println!("║                                                                                ║");
    println!("║  Adapter Name:     {} {}", adapter.name, 
        if adapter.is_powered { "✅ ACTIVE" } else { "❌ INACTIVE" }
    );
    println!("║  MAC Address:      {:<62} ║", adapter.address);
    
    if let Some(version) = &adapter.bt_version {
        println!("║  BT Version:       {:<62} ║", version);
    }
    
    println!("║  Status:           Powered={} Connectable={} Discoverable={}",
        if adapter.is_powered { "✓" } else { "✗" },
        if adapter.is_connectable { "✓" } else { "✗" },
        if adapter.is_discoverable { "✓" } else { "✗" }
    );
    println!("║");
    
    println!("║  SUPPORTED MODES:                                                              ║");
    for mode in &adapter.supported_modes {
        println!("║    • {:<77} ║", mode.to_string());
    }
    
    println!("║                                                                                ║");
    println!("║  SUPPORTED PHY (Physical Layers):                                              ║");
    for phy in &adapter.supported_phys {
        println!("║    • {:<77} ║", phy.to_string());
    }
    
    println!("║                                                                                ║");
    println!("║  ADVANCED FEATURES:                                                            ║");
    for feature in &adapter.features {
        println!("║    ✨ {:<75} ║", feature);
    }
    
    println!("║                                                                                ║");
    println!("╚════════════════════════════════════════════════════════════════════════════════╝");
}

/// Log adapter information
pub fn log_adapter_info(adapter: &AdapterInfo) {
    info!("");
    info!("📱 BLUETOOTH ADAPTER INFORMATION");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("  Adapter:       {} {}", adapter.name, 
        if adapter.is_powered { "✅" } else { "❌" }
    );
    info!("  Address:       {}", adapter.address);
    
    if let Some(version) = &adapter.bt_version {
        info!("  Version:       {}", version);
    }
    
    info!("  Status:        Powered={} Connectable={} Discoverable={}",
        if adapter.is_powered { "✓" } else { "✗" },
        if adapter.is_connectable { "✓" } else { "✗" },
        if adapter.is_discoverable { "✓" } else { "✗" }
    );
    
    info!("");
    info!("  📡 Supported Modes:");
    for mode in &adapter.supported_modes {
        info!("     • {}", mode);
    }
    
    info!("");
    info!("  📶 Supported PHY:");
    for phy in &adapter.supported_phys {
        info!("     • {}", phy);
    }
    
    info!("");
    info!("  ✨ Features:");
    for feature in &adapter.features {
        info!("     ✓ {}", feature);
    }
    
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = AdapterInfo::new("Test".to_string(), "AA:BB:CC:DD:EE:FF".to_string());
        assert_eq!(adapter.name, "Test");
        assert_eq!(adapter.address, "AA:BB:CC:DD:EE:FF");
    }

    #[test]
    fn test_default_adapter() {
        let adapter = AdapterInfo::get_default_adapter();
        assert!(!adapter.name.is_empty());
        assert!(!adapter.supported_modes.is_empty());
    }

    #[test]
    fn test_mode_display() {
        assert_eq!(BluetoothMode::BLE.to_string(), "BLE (Bluetooth Low Energy)");
        assert_eq!(
            BluetoothMode::DualMode.to_string(),
            "DUAL MODE (BLE + BR/EDR)"
        );
    }
}
