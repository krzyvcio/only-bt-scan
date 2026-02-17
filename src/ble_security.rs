use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MacAddressType {
    Public,
    RandomStatic,
    RandomResolvable,
    RandomNonResolvable,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PairingMethod {
    JustWorks,
    PasskeyEntry,
    NumericComparison,
    Oob,
    None,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityLevel {
    Secured,
    Unsecured,
    Legacy,
    SecureConnections,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityInfo {
    pub mac_type: MacAddressType,
    pub is_rpa: bool,
    pub mac_randomization: bool,
    pub pairing_method: PairingMethod,
    pub security_level: SecurityLevel,
    pub is_connectable: bool,
    pub requires_bonding: bool,
}

impl Default for SecurityInfo {
    fn default() -> Self {
        Self {
            mac_type: MacAddressType::Unknown,
            is_rpa: false,
            mac_randomization: false,
            pairing_method: PairingMethod::Unknown,
            security_level: SecurityLevel::Unknown,
            is_connectable: false,
            requires_bonding: false,
        }
    }
}

pub fn detect_mac_type(mac: &str) -> MacAddressType {
    let mac_clean = mac.replace(":", "").replace("-", "").to_uppercase();

    if mac_clean.len() != 12 {
        return MacAddressType::Unknown;
    }

    let first_byte = u8::from_str_radix(&mac_clean[0..2], 16).unwrap_or(0);

    // W BLE typ adresu losowego (Random) jest określony przez dwa najbardziej znaczące bity pierwszego bajtu:
    // 11 - Static Random Address
    // 01 - Resolvable Private Address (RPA)
    // 00 - Non-Resolvable Private Address (NRPA)
    // Uwaga: Publiczne adresy nie mają tych bitów ustawionych w ten specyficzny sposób (zwykle).
    // Jednak formalnie typ Public vs Random jest przesyłany w nagłówku pakietu.
    // Tutaj stosujemy heurystykę opartą na samej zawartości adresu.

    let top_bits = first_byte & 0xC0;

    match top_bits {
        0xC0 => MacAddressType::RandomStatic,
        0x40 => MacAddressType::RandomResolvable,
        0x00 => {
            // Jeśli bity są 00, może to być Public (OUI) lub Non-Resolvable Private.
            // Zazwyczaj przyjmujemy że jeśli nie jest to Random, to jest Public.
            // Heurystyka: jeśli top bits są 00, sprawdzamy czy to adres lokalnie administrowany.
            if (first_byte & 0x02) != 0 {
                MacAddressType::RandomNonResolvable
            } else {
                MacAddressType::Public
            }
        }
        _ => MacAddressType::Unknown,
    }
}

pub fn is_rpa(mac: &str) -> bool {
    matches!(detect_mac_type(mac), MacAddressType::RandomResolvable)
}

pub fn has_mac_randomization(mac: &str) -> bool {
    matches!(
        detect_mac_type(mac),
        MacAddressType::RandomStatic
            | MacAddressType::RandomResolvable
            | MacAddressType::RandomNonResolvable
    )
}

pub fn analyze_security_from_advertising(
    mac: &str,
    services: &[String],
    service_data: &[(String, Vec<u8>)],
    is_connectable: bool,
) -> SecurityInfo {
    let mac_type = detect_mac_type(mac);
    let is_rpa = is_rpa(mac);
    let mac_randomization = has_mac_randomization(mac);

    let (pairing_method, security_level) =
        analyze_pairing_and_security(services, service_data, is_connectable);

    let requires_bonding = is_connectable
        && matches!(
            security_level,
            SecurityLevel::Secured | SecurityLevel::SecureConnections
        );

    SecurityInfo {
        mac_type,
        is_rpa,
        mac_randomization,
        pairing_method,
        security_level,
        is_connectable,
        requires_bonding,
    }
}

fn analyze_pairing_and_security(
    services: &[String],
    _service_data: &[(String, Vec<u8>)],
    is_connectable: bool,
) -> (PairingMethod, SecurityLevel) {
    let has_secure_services = services.iter().any(|s| {
        let s_upper = s.to_uppercase();
        s_upper.contains("SECURE") || s_upper.contains("AUTH") || s_upper.contains("KEY")
    });

    let has_standard_services = services.iter().any(|s| {
        let s_upper = s.to_uppercase();
        s_upper.contains("1800") || s_upper.contains("1801") || s_upper.contains("180A")
    });

    if is_connectable && has_secure_services {
        (
            PairingMethod::PasskeyEntry,
            SecurityLevel::SecureConnections,
        )
    } else if is_connectable && has_standard_services {
        (PairingMethod::JustWorks, SecurityLevel::Legacy)
    } else if is_connectable {
        (PairingMethod::JustWorks, SecurityLevel::Unsecured)
    } else {
        (PairingMethod::None, SecurityLevel::Unsecured)
    }
}

pub fn get_mac_type_name(mac_type: &MacAddressType) -> &'static str {
    match mac_type {
        MacAddressType::Public => "Public",
        MacAddressType::RandomStatic => "Random (Static)",
        MacAddressType::RandomResolvable => "RPA",
        MacAddressType::RandomNonResolvable => "Random (Non-Resolvable)",
        MacAddressType::Unknown => "Unknown",
    }
}

pub fn get_pairing_name(method: &PairingMethod) -> &'static str {
    match method {
        PairingMethod::JustWorks => "Just Works",
        PairingMethod::PasskeyEntry => "Passkey Entry",
        PairingMethod::NumericComparison => "Numeric Comparison",
        PairingMethod::Oob => "OOB (Out of Band)",
        PairingMethod::None => "None",
        PairingMethod::Unknown => "Unknown",
    }
}

pub fn get_security_name(level: &SecurityLevel) -> &'static str {
    match level {
        SecurityLevel::Secured => "Secured",
        SecurityLevel::Unsecured => "Unsecured",
        SecurityLevel::Legacy => "Legacy",
        SecurityLevel::SecureConnections => "Secure Connections",
        SecurityLevel::Unknown => "Unknown",
    }
}

pub fn format_security_summary(info: &SecurityInfo) -> String {
    let mut parts = vec![];

    parts.push(format!("MAC: {}", get_mac_type_name(&info.mac_type)));

    if info.is_rpa {
        parts.push("RPA Detected".to_string());
    }

    if info.mac_randomization {
        parts.push("Randomized MAC".to_string());
    }

    parts.push(format!(
        "Pairing: {}",
        get_pairing_name(&info.pairing_method)
    ));
    parts.push(format!(
        "Security: {}",
        get_security_name(&info.security_level)
    ));

    if info.is_connectable {
        parts.push("Connectable".to_string());
    } else {
        parts.push("Non-Connectable".to_string());
    }

    parts.join(" | ")
}
