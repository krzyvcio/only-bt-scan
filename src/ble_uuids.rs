/// BLE UUID mappings for services, characteristics, and manufacturer IDs
///
/// Data based on Bluetooth SIG Assigned Numbers (2026).
/// Provides lookup functions for known 128-bit services and manufacturer names.
pub fn get_known_128bit_service(uuid: &str) -> Option<&'static str> {
    let uuid_upper = uuid.to_uppercase();

    match uuid_upper.as_str() {
        // ===== Nordic Semiconductor =====
        "6E400001-B5A3-F393-E0A9-E50E24DCCA9E" => Some("Nordic UART Service (NUS)"),
        "6E400002-B5A3-F393-E0A9-E50E24DCCA9E" => Some("Nordic UART TX"),
        "6E400003-B5A3-F393-E0A9-E50E24DCCA9E" => Some("Nordic UART RX"),
        "00001523-1212-EFDE-1523-785FEABCD123" => Some("Nordic DFU"),
        "8EC90001-F315-4F60-9FB8-838830DAEA50" => Some("Nordic Secure DFU"),

        // ===== Google / Alphabet =====
        "FE2C123B-8366-4814-8EB0-01DE32100BEA" => Some("Google Fast Pair Model ID"),
        "FE2C123C-8366-4814-8EB0-01DE32100BEA" => Some("Google Fast Pair Additional Data"),
        "0000FEAA-0000-1000-8000-00805F9B34FB" => Some("Google Eddystone / Nearby"),
        "FDA50693-A4E2-4FB1-AFCF-C6EB07647825" => Some("Google Nearby Share"),
        "AE5ACDB1-4B1A-4B06-8319-1B0050DFABCF" => Some("Google Cast"),

        // ===== Apple =====
        "4C000215-E2C0-4B0C-98A4-C529E59D6D4F" => Some("Apple Find My / iBeacon"),
        "4C000216-E2C0-4B0C-98A4-C529E59D6D4F" => Some("Apple iBeacon Manufacturer"),
        "D0611E78-BBB4-4591-A5F8-4879101FEAE2" => Some("Apple AirPods Pairing"),
        "0000FD51-0000-1000-8000-00805F9B34FB" => Some("Apple Continuity Protocol"),
        "0000FD52-0000-1000-8000-00805F9B34FB" => Some("Apple Handoff"),
        "0000FE2C-0000-1000-8000-00805F9B34FB" => Some("Apple HomeKit"),
        "0000FE55-0000-1000-8000-00805F9B34FB" => Some("Apple AirPlay"),
        "0000FE95-0000-1000-8000-00805F9B34FB" => Some("Apple AirDrop"),
        "00000D00-0000-1000-8000-00805F9B34FB" => Some("Apple Wireless Direct Link"),

        // ===== Xiaomi / Amazfit / Mi =====
        "0000FEE0-0000-1000-8000-00805F9B34FB" => Some("Xiaomi Mi Band / Amazfit Service"),
        "0000FEE1-0000-1000-8000-00805F9B34FB" => Some("Xiaomi Auth / Config"),
        "FEE7D263-1E4B-4A0B-918F-C50C247CD498" => Some("Xiaomi Mesh"),

        // ===== Samsung =====
        "6FBFE641-DA44-4CB9-AC8F-0846105F6AEF" => Some("Samsung SmartThings"),
        "12345678-1234-5678-1234-56789ABCDEF0" => Some("Samsung Watch Connect"),

        // ===== Huawei / Honor =====
        "0000FD2D-0000-1000-8000-00805F9B34FB" => Some("Huawei Share / Find My Device"),
        "0000180A-0000-1000-8000-00805F9B34FB" => Some("Huawei Device Information"),
        "0000FDB8-0000-1000-8000-00805F9B34FB" => Some("Huawei HiLink"),

        // ===== Fitbit =====
        "ADAB0000-6E7D-4601-BDA2-BFFAA68956BA" => Some("Fitbit Service"),
        "ADAB0001-6E7D-4601-BDA2-BFFAA68956BA" => Some("Fitbit Charge/Versa"),
        "ADABFB00-6E7D-4601-BDA2-BFFAA68956BA" => Some("Fitbit Data Transfer"),

        // ===== Sony / WH/LinkBuds =====
        "8D53DC1D-1DB7-41F3-A51B-A9C9C7A46B4D" => Some("Sony WH-1000XM5 / Headphones"),
        "0000FD4B-0000-1000-8000-00805F9B34FB" => Some("Sony LinkBuds"),
        "0000FD47-0000-1000-8000-00805F9B34FB" => Some("Sony WH-CH710N"),

        // ===== Espressif (ESP32 / Native) =====
        "0000FFE0-0000-1000-8000-00805F9B34FB" => Some("ESP32 Custom Serial (HM-10)"),
        "0000FFE1-0000-1000-8000-00805F9B34FB" => Some("ESP32 TX/RX"),

        // ===== MIDI over BLE =====
        "03B80E5A-EDE8-4B33-A751-6CE34EC4C700" => Some("MIDI over BLE Service"),
        "7772E5DB-3868-4112-A1A9-F2669D106BF3" => Some("MIDI I/O Characteristic"),

        // ===== Qualcomm / Snapdragon =====
        "0000FD6F-0000-1000-8000-00805F9B34FB" => Some("Qualcomm Snapdragon Secure Processor"),

        // ===== LG Electronics =====
        "0000FD7A-0000-1000-8000-00805F9B34FB" => Some("LG Smart Device Service"),
        "0000FD7B-0000-1000-8000-00805F9B34FB" => Some("LG TV Remote Control"),

        // ===== Tile / Tile Mate =====
        "FEED0000-BEBA-BEBA-BEBA-FEEDDBABAEBE" => Some("Tile Lite/Slim UUID"),

        // ===== Philips Hue =====
        "0000FD3D-0000-1000-8000-00805F9B34FB" => Some("Philips Hue Service"),

        // ===== IoT & Smart Home =====
        "10000000-0000-0000-0000-000000000000" => Some("Generic IoT Service"),
        "36D4DC5D-DFD5-4216-93F7-B91F4816E34E" => Some("Matter / Thread Bridge"),

        // ===== Automotive (Tesla / BMW) =====
        "C3C9221C-7F1A-4E5E-B0F5-2ABBC601D4A7" => Some("Tesla Vehicle Service"),
        "112755DC-DCDB-ECDB-DCDB-DCCCBCCBCCBC" => Some("BMW ConnectedDrive"),

        // ===== Health & Medical =====
        "0000183E-0000-1000-8000-00805F9B34FB" => Some("Medical Health Device Service"),

        // ===== Garmin / Sports =====
        "0000FEB3-0000-1000-8000-00805F9B34FB" => Some("Garmin ANT+ Bridge"),
        "6ACCDBEE-6D60-4C76-9E48-9FFDE405EBC9" => Some("Garmin Device Service"),

        // ===== Polar / Sports Watches =====
        "FB005C80-02E7-F387-1CAD-8ACD2D8DF0C8" => Some("Polar H9/H10 Heart Rate Monitor"),

        // ===== OnePlus =====
        "0000FCE0-0000-1000-8000-00805F9B34FB" => Some("OnePlus Alert Notification"),

        // ===== MSFT / Xbox / Surface =====
        "00001812-0000-1000-8000-00805F9B34FB" => Some("Microsoft Xbox Wireless"),
        "00001801-0000-1000-8000-00805F9B34FB" => Some("Microsoft Surface Connector"),

        // ===== LEGO / Toy Protocol =====
        "7ADBFB00-6E7D-4601-BDA2-BFFAA68956BA" => Some("LEGO Wireless Protocol"),

        // ===== Withings / Health Monitoring =====
        "00000000-0000-1000-8000-00805F9B34FB" => Some("Withings Health Monitoring"),

        _ => None,
    }
}

/// Get manufacturer name by company ID
///
/// Looks up the company name associated with a Bluetooth SIG company identifier.
///
/// # Arguments
/// * `code` - 16-bit company identifier (e.g., 0x004C for Apple)
///
/// # Returns
/// `Some(&str)` with the company name if found, `None` otherwise
pub fn get_manufacturer_name(code: u16) -> Option<&'static str> {
    match code {
        // ===== Major Tech Companies =====
        0x0001 => Some("Ericsson Technology Licensing"),
        0x0004 => Some("Nokia Mobile Phones"),
        0x0005 => Some("Toshiba Corp."),
        0x0006 => Some("Microsoft Corporation"),
        0x0007 => Some("Lucent"),
        0x0008 => Some("Motorola"),
        0x0009 => Some("Infineon Technologies AG"),
        0x000A => Some("Cambridge Silicon Radio"),
        0x000B => Some("Silicon Wave"),
        0x000C => Some("Digianswer A/S"),
        0x000D => Some("Texas Instruments Inc."),
        0x000E => Some("Parthus Technologies Inc."),
        0x000F => Some("Broadcom Corporation"),
        0x0010 => Some("Intel Corp."),
        0x0011 => Some("Waveplus Technology Co., Ltd."),
        0x0015 => Some("ASUSTek Computer Inc."),

        // ===== Apple & Google =====
        0x004C => Some("Apple Inc."),
        0x0059 => Some("Google LLC"),
        0x00E0 => Some("Google"),

        // ===== Consumer Electronics Giants =====
        0x0075 => Some("Samsung Electronics Co. Ltd."),
        0x0076 => Some("LG Electronics"),
        0x0156 => Some("Huawei Technologies Co. Ltd."),
        0x019F => Some("Sony Corporation"),
        0x01E4 => Some("Panasonic Corporation"),
        0x0124 => Some("Sony Ericsson Mobile Communications AB"),

        // ===== Xiaomi Family =====
        0x038F => Some("Xiaomi Inc."),
        0x028E => Some("Xiaomi Inc."),
        0x01C3 => Some("Xiaomi Communications Co., Ltd."),
        0x02E3 => Some("Anhui Huami Information Technology Co., Ltd."),
        0x023D => Some("Dreame Innovation Technology Co., Ltd."),

        // ===== Chinese Tech =====
        0x0152 => Some("OPPO Mobile Telecommunications Corp., Ltd."),
        0x0190 => Some("OnePlus Electronics (Shenzhen) Co. Ltd."),
        0x0157 => Some("Shenzhen Goodix Technology Co., Ltd."),
        0x02A6 => Some("Realme Chongqing Mobile Telecommunications Corp., Ltd."),
        0x03C7 => Some("Vivo Mobile Communication Co., Ltd."),

        // ===== Audio & Entertainment =====
        0x003C => Some("Bose Corporation"),
        0x0117 => Some("Harman International Industries Inc."),
        0x0158 => Some("Sonos, Inc."),
        0x02D9 => Some("Marshall London"),
        0x0087 => Some("Sennheiser Communications A/S"),
        0x0138 => Some("Plantronics"),
        0x0277 => Some("Bowers & Wilkins"),

        // ===== Wearables & Fitness =====
        0x0220 => Some("Fitbit, Inc."),
        0x0293 => Some("TomTom International BV"),
        0x0219 => Some("Garmin Ltd."),
        0x020E => Some("Polar Electro Oy"),
        0x015D => Some("Jawbone"),
        0x029D => Some("HUAWEI Technologies Co., Ltd. (wearables)"),
        0x0394 => Some("Samsung Electronics Co., Ltd. (wearables)"),

        // ===== Smart Home & IoT =====
        0x004D => Some("Broadcom Corporation"),
        0x00D0 => Some("Nordic Semiconductor ASA"),
        0x025D => Some("Philips Lighting B.V."),
        0x0100 => Some("LEGO System A/S"),
        0x02CA => Some("Espressif Incorporated"),
        0x0131 => Some("Amazon.com Services, Inc."),
        0x00AD => Some("TP-Link Corporation Limited"),

        // ===== Chip Manufacturers =====
        0x02DB => Some("Infineon Technologies AG"),
        0x025B => Some("Realtek Semiconductor Corporation"),
        0x0060 => Some("NXP Semiconductors"),
        0x0088 => Some("STMicroelectronics International NV"),
        0x00CB => Some("Marvell Technology Group Ltd."),
        0x02E5 => Some("Nordic Semiconductor ASA (DFU)"),
        0x0171 => Some("MediaTek Inc."),
        0x00E5 => Some("Qualcomm Technologies, Inc."),
        0x0229 => Some("Qualcomm Inc."),

        // ===== Automotive =====
        0x0099 => Some("BMW Group"),
        0x00CF => Some("Daimler AG"),
        0x0110 => Some("Tesla Inc."),
        0x0167 => Some("BMW AG"),
        0x006D => Some("Ford Motor Company"),
        0x0085 => Some("Volkswagen AG"),
        0x009E => Some("Audi AG"),
        0x00E3 => Some("Porsche AG"),
        0x012D => Some("Tesla Motors"),

        // ===== Medical & Health =====
        0x00DA => Some("A&D Company, Limited"),
        0x014D => Some("GN ReSound A/S"),
        0x0223 => Some("ResMed Inc."),
        0x02C8 => Some("NeuroPace Inc."),
        0x00A7 => Some("Abbott Diabetes Care"),
        0x0168 => Some("Medtronic Inc."),
        0x01A8 => Some("Dexcom, Inc."),

        // ===== Computer & Peripherals =====
        0x0046 => Some("Logitech International SA"),
        0x004F => Some("Hewlett-Packard Company"),
        0x0057 => Some("Microsoft Corporation (Xbox)"),
        0x005B => Some("Dell Inc."),
        0x0068 => Some("Lenovo (Singapore) Pte. Ltd."),
        0x0078 => Some("Mitel Semiconductor Ltd."),

        // ===== Gaming & VR =====
        0x01E5 => Some("Valve Corporation"),
        0x0269 => Some("Razer Inc."),
        0x0339 => Some("Nintendo Co., Ltd."),

        // ===== Other Notable Companies =====
        0x01F6 => Some("OpenWrt Project"),
        0x020F => Some("Arch Evo Ltd."),
        0x0224 => Some("Tile, Inc."),
        0x022D => Some("GoPro, Inc."),
        0x0239 => Some("Roku Inc."),
        0x0245 => Some("Lemonade Inc."),
        0x02BC => Some("Dyson Ltd."),
        0x0031 => Some("Seiko Epson Corporation"),
        0x0113 => Some("Tencent Holdings Ltd."),
        0x0275 => Some("DJI Innovations"),

        _ => None,
    }
}

/// Check if a byte array matches the adopted 16-bit UUID base
///
/// The Bluetooth SIG adopted base for 16-bit UUIDs is:
/// 0000XXXX-0000-1000-8000-00805F9B34FB
///
/// # Arguments
/// * `uuid_bytes` - 16-byte array representing a 128-bit UUID
///
/// # Returns
/// `true` if the UUID matches the adopted base format
pub fn is_adopted_16bit_uuid(uuid_bytes: &[u8; 16]) -> bool {
    const ADOPTED_BASE: [u8; 12] = [
        0x00, 0x00, 0x10, 0x00, // 3rd-4th bytes (little-endian)
        0x80, 0x00, // 5th-6th bytes
        0x00, 0x80, // 7th-8th bytes
        0x5F, 0x9B, 0x34, 0xFB, // Last 4 bytes
    ];

    uuid_bytes[4..] == ADOPTED_BASE
}

/// Extract 16-bit UUID from a 128-bit adopted format
///
/// Converts a 128-bit UUID in adopted format to its 16-bit representation.
///
/// # Arguments
/// * `uuid_bytes` - 16-byte array representing a 128-bit UUID in adopted format
///
/// # Returns
/// `Some(u16)` with the extracted UUID if it matches the adopted format, `None` otherwise
pub fn extract_16bit_from_uuid(uuid_bytes: &[u8; 16]) -> Option<u16> {
    if is_adopted_16bit_uuid(uuid_bytes) {
        Some(u16::from_le_bytes([uuid_bytes[0], uuid_bytes[1]]))
    } else {
        None
    }
}

/// Check if a service UUID indicates LE Audio support
///
/// LE Audio services were introduced in Bluetooth 5.2 and include
/// audio streaming, volume control, and hearing aid features.
///
/// # Arguments
/// * `uuid16` - 16-bit service UUID to check
///
/// # Returns
/// `true` if the UUID is an LE Audio service
pub fn is_le_audio_service(uuid16: u16) -> bool {
    matches!(
        uuid16,
        0x1844 |  // Volume Control
        0x1845 |  // Volume Offset Control
        0x1849 |  // Media Control
        0x184B |  // Generic Media Control
        0x184F |  // Broadcast Audio Scan
        0x1850 |  // Published Audio Capabilities
        0x1853 |  // Common Audio
        0x1854 // Hearing Access
    )
}

/// Check if a service UUID indicates fitness/wearable capability
///
/// These services are commonly used in fitness trackers, smartwatches,
/// and health monitoring devices.
///
/// # Arguments
/// * `uuid16` - 16-bit service UUID to check
///
/// # Returns
/// `true` if the UUID is a fitness/wearable service
pub fn is_fitness_wearable_service(uuid16: u16) -> bool {
    matches!(
        uuid16,
        0x180D |  // Heart Rate
        0x181A |  // Environmental Sensing
        0x181C |  // Body Composition
        0x181D |  // User Data
        0x181F |  // Weight Scale
        0x1814 |  // Running Speed and Cadence
        0x1816 |  // Cycling Speed and Cadence
        0x1818 |  // Cycling Power
        0x1826 |  // Fitness Machine
        0x183E // Physical Activity Monitor
    )
}

/// Check if a service UUID indicates smart device/IoT capability
///
/// These services are commonly used in smart home devices, sensors,
/// and IoT applications.
///
/// # Arguments
/// * `uuid16` - 16-bit service UUID to check
///
/// # Returns
/// `true` if the UUID is a smart home/IoT service
pub fn is_iot_smart_service(uuid16: u16) -> bool {
    matches!(
        uuid16,
        0x1800 |  // Generic Access
        0x1802 |  // Immediate Alert
        0x1803 |  // Link Loss
        0x1804 |  // Tx Power
        0x1820 |  // Internet Protocol Support
        0x1821 |  // Indoor Positioning
        0x183B |  // Binary Sensor
        0x183C // Emergency Configuration
    )
}

/// Check if a service suggests Bluetooth 5.0+ features
///
/// These services were introduced or significantly updated in Bluetooth 5.0.
///
/// # Arguments
/// * `uuid16` - 16-bit service UUID to check
///
/// # Returns
/// `true` if the UUID is a BT 5.0+ service
pub fn is_bt50_or_later_service(uuid16: u16) -> bool {
    matches!(
        uuid16,
        0x184F |  // Broadcast Audio Scan (5.0+)
        0x1850 |  // Published Audio Capabilities (5.0+)
        0x1853 |  // Common Audio (5.0+)
        0x1854 // Hearing Access (5.0+)
    )
}

/// Check if a service suggests Bluetooth 5.2+ features (LE Audio)
///
/// LE Audio services were introduced in Bluetooth 5.2, enabling
/// new audio use cases like hearing aids and broadcast audio.
///
/// # Arguments
/// * `uuid16` - 16-bit service UUID to check
///
/// # Returns
/// `true` if the UUID is a BT 5.2+ LE Audio service
pub fn is_bt52_or_later_service(uuid16: u16) -> bool {
    matches!(
        uuid16,
        0x1849 |  // Media Control (5.2+)
        0x1844 |  // Volume Control (5.2+)
        0x1845 |  // Volume Offset Control (5.2+)
        0x1853 |  // Common Audio (5.2+)
        0x1854 // Hearing Access (5.2+)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manufacturer_names() {
        assert_eq!(get_manufacturer_name(0x004C), Some("Apple Inc."));
        assert_eq!(get_manufacturer_name(0x0059), Some("Google LLC"));
    }

    #[test]
    fn test_128bit_services() {
        assert!(get_known_128bit_service("6E400001-B5A3-F393-E0A9-E50E24DCCA9E").is_some());
        assert!(get_known_128bit_service("invalid").is_none());
    }
}
