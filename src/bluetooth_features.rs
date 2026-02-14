#![allow(dead_code)]

/// Bluetooth Feature Detection and Version Mapping
/// Supports Bluetooth 1.0 through 6.0 with comprehensive feature tracking

use std::collections::HashSet;

/// Bluetooth Core Specification Versions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BluetoothVersion {
    V1_0,  // Bluetooth 1.0 (Baseband)
    V1_0b, // Bluetooth 1.0b (Baseband + LMP)
    V1_1,  // Bluetooth 1.1
    V1_2,  // Bluetooth 1.2 - AFH (Adaptive Frequency Hopping)
    V2_0,  // Bluetooth 2.0 - EDR (Enhanced Data Rate) - 2x, 3x speed
    V2_1,  // Bluetooth 2.1 - SSP/LE (Simple Secure Pairing)
    V3_0,  // Bluetooth 3.0 + HS (High Speed) - WiFi
    V4_0,  // Bluetooth 4.0 - LE (Low Energy) introduced
    V4_1,  // Bluetooth 4.1 - L2CAP enhancements
    V4_2,  // Bluetooth 4.2 - LE improvements, GATT enhancements
    V5_0,  // Bluetooth 5.0 - 2x speed, 4x range, 8x broadcast
    V5_1,  // Bluetooth 5.1 - Direction Finding (AoA/AoD)
    V5_2,  // Bluetooth 5.2 - LE Audio, LC3 codec, EATT
    V5_3,  // Bluetooth 5.3 - Power optimization, filtering
    V5_4,  // Bluetooth 5.4 - Reserved/TBD
    V6_0,  // Bluetooth 6.0 - Large scale IoT, broadcast improvements
}

impl BluetoothVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V1_0 => "1.0",
            Self::V1_0b => "1.0b",
            Self::V1_1 => "1.1",
            Self::V1_2 => "1.2",
            Self::V2_0 => "2.0",
            Self::V2_1 => "2.1",
            Self::V3_0 => "3.0",
            Self::V4_0 => "4.0",
            Self::V4_1 => "4.1",
            Self::V4_2 => "4.2",
            Self::V5_0 => "5.0",
            Self::V5_1 => "5.1",
            Self::V5_2 => "5.2",
            Self::V5_3 => "5.3",
            Self::V5_4 => "5.4",
            Self::V6_0 => "6.0",
        }
    }

    pub fn full_name(&self) -> &'static str {
        match self {
            Self::V1_0 => "Bluetooth 1.0 (Baseband)",
            Self::V1_0b => "Bluetooth 1.0b (Baseband + LMP)",
            Self::V1_1 => "Bluetooth 1.1",
            Self::V1_2 => "Bluetooth 1.2 (AFH)",
            Self::V2_0 => "Bluetooth 2.0 + EDR",
            Self::V2_1 => "Bluetooth 2.1 + EDR + SSP",
            Self::V3_0 => "Bluetooth 3.0 + HS",
            Self::V4_0 => "Bluetooth 4.0 (BLE introduced)",
            Self::V4_1 => "Bluetooth 4.1",
            Self::V4_2 => "Bluetooth 4.2",
            Self::V5_0 => "Bluetooth 5.0",
            Self::V5_1 => "Bluetooth 5.1 (Direction Finding)",
            Self::V5_2 => "Bluetooth 5.2 (LE Audio)",
            Self::V5_3 => "Bluetooth 5.3",
            Self::V5_4 => "Bluetooth 5.4",
            Self::V6_0 => "Bluetooth 6.0",
        }
    }
}

/// Bluetooth Feature Categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BluetoothFeature {
    // Baseband Features (1.0-2.1)
    Baseband,
    AFH,               // Adaptive Frequency Hopping (1.2+)
    EDR,               // Enhanced Data Rate (2.0+)
    SSP,               // Simple Secure Pairing (2.1+)

    // Radio Frequency (2.0+)
    BrEdrBasicRate,    // BR/EDR Basic Rate
    BrEdrEdr2Mbps,     // BR/EDR Enhanced Data Rate 2 Mbps
    BrEdrEdr3Mbps,     // BR/EDR Enhanced Data Rate 3 Mbps

    // High Speed (3.0+)
    HighSpeed,         // Bluetooth 3.0+ HS via WiFi

    // Low Energy (4.0+)
    BLE,
    LEAdvertising,
    LEConnections,
    LEScan,

    // LE Features (4.1+)
    LEPhy2M,           // LE 2M PHY (4.2+, secondary)
    LEPhyCoded,        // LE Coded PHY (5.0+)
    LEExtendedAdvertising, // Extended Advertising (5.0+)
    LEPeriodicAdvertising, // Periodic Advertising (5.0+)

    // LE Audio Features (5.2+)
    LEAudio,           // LE Audio framework
    LC3Codec,          // LC3 codec support
    LEAudioUnicast,    // Unicast Audio
    LEAudioBroadcast,  // Broadcast Audio (isochronous)
    LEAudioMultiStream,// Multiple audio streams

    // LE Connection Features (4.1+)
    EATT,              // Enhanced ATT (5.2+)
    LEConnLengthExtension, // Connection Length Extension (4.2+)
    LEChannelSelection,    // Channel Selection Algorithm #2 (5.0+)
    LEDataPacketLengthExtension, // Data Packet Length Extension (4.2+)

    // Range & Speed (5.0+)
    LE2xSpeed,         // 2x BLE speed (5.0+)
    LE4xRange,         // 4x range improvement (5.0+)
    LE8xBroadcast,     // 8x broadcast capacity (5.0+)

    // Direction Finding (5.1+)
    LEDirectionFinding,
    LEAngleOfArrival,  // AoA
    LEAngleOfDeparture,// AoD

    // Power Management (5.3+)
    LEPowerControl,    // LE Power Control
    LEPathLoss,        // LE Path Loss Monitoring
    LEConnSubrating,   // Connection Subrating

    // Advanced (6.0+)
    LargeScaleNetworking, // Large scale IoT support
    LEBroadcastImprovements,

    // Mesh (5.0+ optional)
    BLEMesh,

    // Other
    MultipleLE,        // Multiple LE Controllers
    DualMode,          // Simultaneous LE & BR/EDR
}

impl BluetoothFeature {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Baseband => "Baseband",
            Self::AFH => "Adaptive Frequency Hopping",
            Self::EDR => "Enhanced Data Rate",
            Self::SSP => "Simple Secure Pairing",
            Self::BrEdrBasicRate => "BR/EDR Basic Rate",
            Self::BrEdrEdr2Mbps => "BR/EDR 2 Mbps",
            Self::BrEdrEdr3Mbps => "BR/EDR 3 Mbps",
            Self::HighSpeed => "High Speed (3.0+ HS)",
            Self::BLE => "Bluetooth Low Energy",
            Self::LEAdvertising => "LE Advertising",
            Self::LEConnections => "LE Connections",
            Self::LEScan => "LE Scanning",
            Self::LEPhy2M => "LE 2M PHY",
            Self::LEPhyCoded => "LE Coded PHY",
            Self::LEExtendedAdvertising => "Extended Advertising",
            Self::LEPeriodicAdvertising => "Periodic Advertising",
            Self::LEAudio => "LE Audio",
            Self::LC3Codec => "LC3 Codec",
            Self::LEAudioUnicast => "LE Audio Unicast",
            Self::LEAudioBroadcast => "LE Audio Broadcast",
            Self::LEAudioMultiStream => "LE Audio Multi-Stream",
            Self::EATT => "Enhanced ATT (EATT)",
            Self::LEConnLengthExtension => "LE Connection Length Extension",
            Self::LEChannelSelection => "LE Channel Selection Alg #2",
            Self::LEDataPacketLengthExtension => "LE Data Packet Length Ext",
            Self::LE2xSpeed => "2x Speed (5.0+)",
            Self::LE4xRange => "4x Range (5.0+)",
            Self::LE8xBroadcast => "8x Broadcast (5.0+)",
            Self::LEDirectionFinding => "LE Direction Finding",
            Self::LEAngleOfArrival => "LE Angle of Arrival (AoA)",
            Self::LEAngleOfDeparture => "LE Angle of Departure (AoD)",
            Self::LEPowerControl => "LE Power Control",
            Self::LEPathLoss => "LE Path Loss Monitoring",
            Self::LEConnSubrating => "LE Connection Subrating",
            Self::LargeScaleNetworking => "Large Scale Networking",
            Self::LEBroadcastImprovements => "LE Broadcast Improvements",
            Self::BLEMesh => "BLE Mesh",
            Self::MultipleLE => "Multiple LE Controllers",
            Self::DualMode => "Dual Mode (LE + BR/EDR)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Baseband => "Core Bluetooth baseband layer",
            Self::AFH => "Hop across 79 channels to avoid interference",
            Self::EDR => "2-3x faster data transfer rate",
            Self::SSP => "Secure pairing without entering PINs",
            Self::BrEdrBasicRate => "1 Mbps Basic Rate",
            Self::BrEdrEdr2Mbps => "2 Mbps Enhanced Data Rate",
            Self::BrEdrEdr3Mbps => "3 Mbps Enhanced Data Rate",
            Self::HighSpeed => "802.11 WiFi bridge for high bandwidth",
            Self::BLE => "Low power wireless personal area network",
            Self::LEAdvertising => "Send broadcast advertisements",
            Self::LEConnections => "Establish point-to-point connections",
            Self::LEScan => "Scan for nearby devices",
            Self::LEPhy2M => "2 Mbps PHY on 2.4 GHz",
            Self::LEPhyCoded => "125/500 kbps range-extended PHY",
            Self::LEExtendedAdvertising => "Extended advertisement packets up to 251 bytes",
            Self::LEPeriodicAdvertising => "Periodic repeating advertisements",
            Self::LEAudio => "LE Audio transport layer",
            Self::LC3Codec => "Low Complexity Communications Codec",
            Self::LEAudioUnicast => "Point-to-point audio streaming",
            Self::LEAudioBroadcast => "Broadcast audio to multiple listeners",
            Self::LEAudioMultiStream => "Multiple concurrent audio streams",
            Self::EATT => "Pipelined ATT operations",
            Self::LEConnLengthExtension => "Support up to 251 bytes of data per connection event",
            Self::LEChannelSelection => "Better channel selection to avoid collisions",
            Self::LEDataPacketLengthExtension => "Support up to 251 bytes per packet",
            Self::LE2xSpeed => "2x faster BLE speed (up to 2 Mbps)",
            Self::LE4xRange => "4x greater range",
            Self::LE8xBroadcast => "8x higher broadcast capacity",
            Self::LEDirectionFinding => "Determine direction to transmitter",
            Self::LEAngleOfArrival => "Calculate angle of arrival of signal",
            Self::LEAngleOfDeparture => "Calculate angle of departure",
            Self::LEPowerControl => "Dynamic power adjustment during connection",
            Self::LEPathLoss => "Monitor and react to path loss changes",
            Self::LEConnSubrating => "Reduce power consumption via subrating",
            Self::LargeScaleNetworking => "Support for extensive mesh networks",
            Self::LEBroadcastImprovements => "Enhanced broadcast-related features",
            Self::BLEMesh => "Mesh networking for multi-hop coverage",
            Self::MultipleLE => "Multiple independent LE controllers",
            Self::DualMode => "Run both LE and BR/EDR simultaneously",
        }
    }
}

/// Feature Set for a specific Bluetooth Version
pub struct VersionFeatureSet {
    pub version: BluetoothVersion,
    pub features: HashSet<BluetoothFeature>,
}

impl VersionFeatureSet {
    pub fn new(version: BluetoothVersion) -> Self {
        let features = get_features_for_version(version);
        Self { version, features }
    }

    pub fn has_feature(&self, feature: BluetoothFeature) -> bool {
        self.features.contains(&feature)
    }

    pub fn list_features(&self) -> Vec<&'static str> {
        let mut features: Vec<_> = self
            .features
            .iter()
            .map(|f| f.name())
            .collect();
        features.sort();
        features
    }
}

/// Get all features introduced in or before a specific version
pub fn get_features_for_version(version: BluetoothVersion) -> HashSet<BluetoothFeature> {
    use BluetoothFeature as F;

    let mut features = HashSet::new();

    // Bluetooth 1.0
    if version >= BluetoothVersion::V1_0 {
        features.insert(F::Baseband);
        features.insert(F::BrEdrBasicRate);
    }

    // Bluetooth 1.2 - AFH introduced
    if version >= BluetoothVersion::V1_2 {
        features.insert(F::AFH);
    }

    // Bluetooth 2.0 - EDR introduced
    if version >= BluetoothVersion::V2_0 {
        features.insert(F::EDR);
        features.insert(F::BrEdrEdr2Mbps);
        features.insert(F::BrEdrEdr3Mbps);
    }

    // Bluetooth 2.1 - SSP introduced
    if version >= BluetoothVersion::V2_1 {
        features.insert(F::SSP);
    }

    // Bluetooth 3.0 - High Speed
    if version >= BluetoothVersion::V3_0 {
        features.insert(F::HighSpeed);
    }

    // Bluetooth 4.0 - LE introduced
    if version >= BluetoothVersion::V4_0 {
        features.insert(F::BLE);
        features.insert(F::LEAdvertising);
        features.insert(F::LEConnections);
        features.insert(F::LEScan);
        features.insert(F::DualMode);
    }

    // Bluetooth 4.1 - EATT, improved L2CAP
    if version >= BluetoothVersion::V4_1 {
        features.insert(F::LEConnLengthExtension);
    }

    // Bluetooth 4.2 - Connection length extension, data length extension
    if version >= BluetoothVersion::V4_2 {
        features.insert(F::LEPhy2M);
        features.insert(F::LEDataPacketLengthExtension);
    }

    // Bluetooth 5.0 - Major LE improvements
    if version >= BluetoothVersion::V5_0 {
        features.insert(F::LE2xSpeed);
        features.insert(F::LE4xRange);
        features.insert(F::LE8xBroadcast);
        features.insert(F::LEPhyCoded);
        features.insert(F::LEExtendedAdvertising);
        features.insert(F::LEPeriodicAdvertising);
        features.insert(F::LEChannelSelection);
        features.insert(F::BLEMesh);
    }

    // Bluetooth 5.1 - Direction Finding
    if version >= BluetoothVersion::V5_1 {
        features.insert(F::LEDirectionFinding);
        features.insert(F::LEAngleOfArrival);
        features.insert(F::LEAngleOfDeparture);
    }

    // Bluetooth 5.2 - LE Audio, EATT
    if version >= BluetoothVersion::V5_2 {
        features.insert(F::LEAudio);
        features.insert(F::LC3Codec);
        features.insert(F::LEAudioUnicast);
        features.insert(F::LEAudioBroadcast);
        features.insert(F::LEAudioMultiStream);
        features.insert(F::EATT);
    }

    // Bluetooth 5.3 - Power optimization
    if version >= BluetoothVersion::V5_3 {
        features.insert(F::LEPowerControl);
        features.insert(F::LEPathLoss);
        features.insert(F::LEConnSubrating);
    }

    // Bluetooth 5.4 - Reserved/TBD
    if version >= BluetoothVersion::V5_4 {
        // Features yet to be formalized
    }

    // Bluetooth 6.0 - Large scale IoT
    if version >= BluetoothVersion::V6_0 {
        features.insert(F::LargeScaleNetworking);
        features.insert(F::LEBroadcastImprovements);
    }

    features
}

/// Detect likely Bluetooth version based on discovered features/services
pub fn detect_version_from_features(discovered_features: &[BluetoothFeature]) -> Option<BluetoothVersion> {
    let feature_set: HashSet<_> = discovered_features.iter().copied().collect();

    // Work from newest to oldest
    for version in &[
        BluetoothVersion::V6_0,
        BluetoothVersion::V5_4,
        BluetoothVersion::V5_3,
        BluetoothVersion::V5_2,
        BluetoothVersion::V5_1,
        BluetoothVersion::V5_0,
        BluetoothVersion::V4_2,
        BluetoothVersion::V4_1,
        BluetoothVersion::V4_0,
        BluetoothVersion::V3_0,
        BluetoothVersion::V2_1,
        BluetoothVersion::V2_0,
        BluetoothVersion::V1_2,
        BluetoothVersion::V1_0,
    ] {
        let version_features = get_features_for_version(*version);
        // Check if all discovered features are in this version
        if feature_set.iter().all(|f| version_features.contains(f)) {
            return Some(*version);
        }
    }

    None
}

/// Estimate version from common service UUIDs
pub fn detect_version_from_services(service_uuids: &[u16]) -> Option<BluetoothVersion> {
    // LE Audio services (5.2+)
    if service_uuids.iter().any(|uuid| matches!(uuid, 0x1849 | 0x1844 | 0x184F | 0x1853)) {
        return Some(BluetoothVersion::V5_2);
    }

    // Environmental sensing (4.0+)
    if service_uuids.iter().any(|uuid| matches!(uuid, 0x181A)) {
        return Some(BluetoothVersion::V4_0);
    }

    // Heart rate, battery (4.0+) are very common
    if service_uuids.iter().any(|uuid| matches!(uuid, 0x180D | 0x180F)) {
        return Some(BluetoothVersion::V4_0);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bluetooth_5_2_features() {
        let v52 = VersionFeatureSet::new(BluetoothVersion::V5_2);
        assert!(v52.has_feature(BluetoothFeature::LEAudio));
        assert!(v52.has_feature(BluetoothFeature::LC3Codec));
        assert!(v52.has_feature(BluetoothFeature::LE2xSpeed));
    }

    #[test]
    fn test_bluetooth_4_0_has_ble_only() {
        let v40 = VersionFeatureSet::new(BluetoothVersion::V4_0);
        assert!(v40.has_feature(BluetoothFeature::BLE));
        assert!(!v40.has_feature(BluetoothFeature::LEAudio));
    }

    #[test]
    fn test_feature_count_increases_with_version() {
        let v40 = VersionFeatureSet::new(BluetoothVersion::V4_0);
        let v50 = VersionFeatureSet::new(BluetoothVersion::V5_0);
        let v52 = VersionFeatureSet::new(BluetoothVersion::V5_2);

        assert!(v40.features.len() < v50.features.len());
        assert!(v50.features.len() < v52.features.len());
    }
}
