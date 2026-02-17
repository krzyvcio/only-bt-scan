use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodServices {
    pub cod_services: Vec<CodService>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodService {
    pub bit: u8,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodDeviceClass {
    pub cod_device_class: Vec<DeviceClassMajor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceClassMajor {
    pub major: u8,
    pub name: String,
    #[serde(default)]
    pub minor: Option<Vec<DeviceClassMinor>>,
    #[serde(default)]
    pub subminor: Option<Vec<DeviceClassMinor>>,
    #[serde(default)]
    pub subsplit: Option<u8>,
    #[serde(default)]
    pub minor_bits: Option<Vec<DeviceClassMinor>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceClassMinor {
    pub value: u8,
    pub name: String,
}

static COD_DATA: std::sync::LazyLock<CodDatabase> = std::sync::LazyLock::new(|| load_cod_data());

fn load_cod_data() -> CodDatabase {
    let yaml_content = include_str!("data/assigned_numbers/core/class_of_device.yaml");

    let mut services: HashMap<u8, String> = HashMap::new();
    let mut major_classes: HashMap<u8, DeviceClassMajorInfo> = HashMap::new();

    if let Ok(data) = serde_yaml::from_str::<CodServices>(yaml_content) {
        for service in data.cod_services {
            services.insert(service.bit, service.name);
        }
    }

    if let Ok(data) = serde_yaml::from_str::<CodDeviceClass>(yaml_content) {
        for class in data.cod_device_class {
            let minor_map: HashMap<u8, String> = class
                .minor
                .unwrap_or_default()
                .into_iter()
                .map(|m| (m.value, m.name))
                .collect();

            let subminor_map: HashMap<u8, String> = class
                .subminor
                .unwrap_or_default()
                .into_iter()
                .map(|m| (m.value, m.name))
                .collect();

            let minor_bits_map: HashMap<u8, String> = class
                .minor_bits
                .unwrap_or_default()
                .into_iter()
                .map(|m| (m.value, m.name))
                .collect();

            major_classes.insert(
                class.major,
                DeviceClassMajorInfo {
                    name: class.name,
                    minor: minor_map,
                    subminor: subminor_map,
                    minor_bits: minor_bits_map,
                },
            );
        }
    }

    CodDatabase {
        services,
        major_classes,
    }
}

#[derive(Debug, Clone)]
pub struct CodDatabase {
    services: HashMap<u8, String>,
    major_classes: HashMap<u8, DeviceClassMajorInfo>,
}

#[derive(Debug, Clone)]
pub struct DeviceClassMajorInfo {
    name: String,
    minor: HashMap<u8, String>,
    subminor: HashMap<u8, String>,
    minor_bits: HashMap<u8, String>,
}

impl CodDatabase {
    pub fn get_service_name(&self, bit: u8) -> Option<&String> {
        self.services.get(&bit)
    }

    pub fn get_major_class(&self, major: u8) -> Option<&str> {
        self.major_classes.get(&major).map(|m| m.name.as_str())
    }

    pub fn get_minor_class(&self, major: u8, minor: u8) -> Option<&str> {
        self.major_classes
            .get(&major)
            .and_then(|m| m.minor.get(&minor).map(|s| s.as_str()))
    }

    pub fn get_subminor_class(&self, major: u8, subminor: u8) -> Option<&str> {
        self.major_classes
            .get(&major)
            .and_then(|m| m.subminor.get(&subminor).map(|s| s.as_str()))
    }

    pub fn get_minor_bits_class(&self, major: u8, minor_bits: u8) -> Option<&str> {
        self.major_classes
            .get(&major)
            .and_then(|m| m.minor_bits.get(&minor_bits).map(|s| s.as_str()))
    }

    pub fn format_device_class(&self, cod: u32) -> String {
        let major = ((cod >> 8) & 0x1F) as u8;
        let minor = ((cod >> 2) & 0x3F) as u8;
        let subminor = (cod & 0x03) as u8;

        let mut parts = Vec::new();

        if let Some(major_name) = self.get_major_class(major) {
            parts.push(major_name.to_string());

            if let Some(minor_name) = self.get_minor_class(major, minor) {
                parts.push(minor_name.to_string());
            }

            if let Some(subminor_name) = self.get_subminor_class(major, subminor) {
                parts.push(subminor_name.to_string());
            }

            if let Some(minor_bits_name) = self.get_minor_bits_class(major, minor) {
                if parts.len() == 1 {
                    parts.push(minor_bits_name.to_string());
                }
            }
        } else {
            parts.push("Unknown".to_string());
        }

        parts.join(" > ")
    }

    pub fn get_services(&self, cod: u32) -> Vec<String> {
        let mut result = Vec::new();

        for bit in 13..=23 {
            if ((cod >> bit) & 1) == 1 {
                if let Some(name) = self.get_service_name(bit) {
                    result.push(name.clone());
                }
            }
        }

        result
    }
}

pub fn format_cod(cod: u32) -> String {
    COD_DATA.format_device_class(cod)
}

pub fn get_cod_services(cod: u32) -> Vec<String> {
    COD_DATA.get_services(cod)
}

pub fn get_major_class_name(major: u8) -> Option<String> {
    COD_DATA.get_major_class(major).map(|s| s.to_string())
}

pub fn get_minor_class_name(major: u8, minor: u8) -> Option<String> {
    COD_DATA
        .get_minor_class(major, minor)
        .map(|s| s.to_string())
}
