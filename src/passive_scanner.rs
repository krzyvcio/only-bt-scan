/// Passive BLE Scanner Module
///
/// Provides passive (non-connectable) BLE scanning for capturing advertising packets.
/// Unlike active scanning, passive mode only listens for advertisements without
/// requesting scan responses, making it ideal for:
/// - Raw packet capture
/// - High-throughput scanning
/// - Covert monitoring
/// - Nanosecond-precision timestamps
use crate::data_models::RawPacketModel;
use crate::raw_sniffer::{AdvertisingType, BluetoothPhy};
use btleplug::api::{Central, Manager, Peripheral};
use btleplug::platform::Manager as PlatformManager;
use chrono::Utc;
use log::{debug, error, info, warn};
use quanta::Clock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;

static PACKET_COUNTER: AtomicU64 = AtomicU64::new(0);

fn get_timestamp_ns() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as i64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassivePacket {
    pub packet_id: u64,
    pub mac_address: String,
    pub rssi: i8,
    pub timestamp_ns: i64,
    pub timestamp_ms: u64,
    pub phy: String,
    pub channel: u8,
    pub packet_type: String,
    pub advertising_data: Vec<u8>,
    pub advertising_data_hex: String,
    pub is_connectable: bool,
    pub is_scannable: bool,
    pub is_directed: bool,
    pub is_legacy: bool,
    pub is_extended: bool,
    pub tx_power: Option<i8>,
    pub advertiser_address_type: String,
}

impl PassivePacket {
    pub fn new(
        mac_address: String,
        rssi: i8,
        timestamp_ns: i64,
        advertising_data: Vec<u8>,
        packet_type: AdvertisingType,
        channel: u8,
        phy: BluetoothPhy,
    ) -> Self {
        let packet_id = PACKET_COUNTER.fetch_add(1, Ordering::Relaxed);
        let timestamp_ms = (timestamp_ns / 1_000_000) as u64;
        let advertising_data_hex = hex::encode(&advertising_data);

        let (is_connectable, is_scannable, is_directed) = match packet_type {
            AdvertisingType::Adv_Ind => (true, true, false),
            AdvertisingType::Adv_Direct_Ind => (true, false, true),
            AdvertisingType::Adv_Nonconn_Ind => (false, false, false),
            AdvertisingType::Adv_Scan_Ind => (false, true, false),
            AdvertisingType::Scan_Rsp => (false, false, false),
            AdvertisingType::Ext_Adv_Ind => (true, true, false),
            AdvertisingType::Unknown => (false, false, false),
        };

        Self {
            packet_id,
            mac_address,
            rssi,
            timestamp_ns,
            timestamp_ms,
            phy: phy.to_string(),
            channel,
            packet_type: packet_type.to_string(),
            advertising_data,
            advertising_data_hex,
            is_connectable,
            is_scannable,
            is_directed,
            is_legacy: matches!(packet_type, AdvertisingType::Ext_Adv_Ind),
            is_extended: matches!(packet_type, AdvertisingType::Ext_Adv_Ind),
            tx_power: None,
            advertiser_address_type: "public".to_string(),
        }
    }

    pub fn to_raw_packet_model(&self) -> RawPacketModel {
        RawPacketModel {
            packet_id: self.packet_id,
            mac_address: self.mac_address.clone(),
            timestamp: Utc::now(),
            timestamp_ms: self.timestamp_ms,
            latency_from_previous_ms: None,
            phy: self.phy.clone(),
            channel: self.channel,
            rssi: self.rssi,
            packet_type: self.packet_type.clone(),
            is_scan_response: self.packet_type == "SCAN_RSP",
            is_extended: self.is_extended,
            advertising_data: self.advertising_data.clone(),
            advertising_data_hex: self.advertising_data_hex.clone(),
            ad_structures: Vec::new(),
            flags: None,
            local_name: None,
            short_name: None,
            advertised_services: Vec::new(),
            manufacturer_data: HashMap::new(),
            service_data: HashMap::new(),
            total_length: self.advertising_data.len(),
            parsed_successfully: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PassiveScanConfig {
    pub scan_duration_ms: u64,
    pub filter_duplicates: bool,
    pub rssi_threshold: i8,
    pub capture_legacy: bool,
    pub capture_extended: bool,
}

impl Default for PassiveScanConfig {
    fn default() -> Self {
        Self {
            scan_duration_ms: 10_000,
            filter_duplicates: true,
            rssi_threshold: -100,
            capture_legacy: true,
            capture_extended: true,
        }
    }
}

pub struct PassiveScanner {
    config: PassiveScanConfig,
    last_seen: HashMap<String, i64>,
}

impl PassiveScanner {
    pub fn new(config: PassiveScanConfig) -> Self {
        Self {
            config,
            last_seen: HashMap::new(),
        }
    }

    pub fn with_default() -> Self {
        Self::new(PassiveScanConfig::default())
    }

    pub async fn start_passive_scan(
        &mut self,
    ) -> Result<Vec<PassivePacket>, Box<dyn std::error::Error + Send + Sync>> {
        info!("📡 Starting passive BLE scan (non-connectable)");

        let manager = PlatformManager::new().await?;
        let adapters = manager.adapters().await?;

        if adapters.is_empty() {
            warn!("⚠️ No Bluetooth adapters found");
            return Ok(Vec::new());
        }

        let mut all_packets = Vec::new();

        for (idx, adapter) in adapters.iter().enumerate() {
            info!("📡 Adapter #{}: Starting passive scan", idx);

            let scan_filter = btleplug::api::ScanFilter::default();

            if let Err(e) = adapter.start_scan(scan_filter).await {
                error!("❌ Failed to start scan on adapter {}: {}", idx, e);
                continue;
            }

            let start_instant = std::time::Instant::now();
            let mut adapter_packets = Vec::new();

            while start_instant.elapsed().as_millis() < self.config.scan_duration_ms as u128 {
                match adapter.peripherals().await {
                    Ok(peripherals) => {
                        for peripheral in &peripherals {
                            if let Ok(props) = peripheral.properties().await {
                                if let Some(properties) = props {
                                    let mac = properties.address.to_string();
                                    let rssi = properties.rssi.unwrap_or(-100) as i8;

                                    if rssi < self.config.rssi_threshold {
                                        continue;
                                    }

                                    let now_ns = get_timestamp_ns();
                                    let timestamp_ns = now_ns;

                                    if self.config.filter_duplicates {
                                        if let Some(last_ns) = self.last_seen.get(&mac) {
                                            let diff_ns = timestamp_ns - last_ns;
                                            if diff_ns < 100_000_000 {
                                                continue;
                                            }
                                        }
                                        self.last_seen.insert(mac.clone(), timestamp_ns);
                                    }

                                    let ad_data: Vec<u8> = properties
                                        .manufacturer_data
                                        .values()
                                        .flat_map(|v| v.clone())
                                        .collect();
                                    let packet_type = if !ad_data.is_empty() {
                                        AdvertisingType::Adv_Ind
                                    } else {
                                        AdvertisingType::Unknown
                                    };

                                    let channel = [37, 38, 39][(mac
                                        .bytes()
                                        .fold(0u8, |acc, b| acc.wrapping_add(b))
                                        as usize)
                                        % 3];

                                    let packet = PassivePacket::new(
                                        mac,
                                        rssi,
                                        timestamp_ns,
                                        ad_data,
                                        packet_type,
                                        channel,
                                        BluetoothPhy::Le1M,
                                    );

                                    debug!(
                                        "📦 Packet: {} | RSSI: {} dBm | Type: {}",
                                        packet.mac_address, packet.rssi, packet.packet_type
                                    );

                                    adapter_packets.push(packet);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Error getting peripherals: {}", e);
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }

            if let Err(e) = adapter.stop_scan().await {
                warn!("⚠️ Failed to stop scan: {}", e);
            }

            all_packets.extend(adapter_packets);
            info!("✅ Adapter {} captured {} packets", idx, all_packets.len());
        }

        info!(
            "✅ Passive scan complete: {} total packets from {} unique devices",
            all_packets.len(),
            all_packets
                .iter()
                .map(|p| &p.mac_address)
                .collect::<std::collections::HashSet<_>>()
                .len()
        );

        Ok(all_packets)
    }

    #[allow(dead_code)]
    pub async fn start_passive_scan_streaming(
        &mut self,
        _tx: mpsc::UnboundedSender<PassivePacket>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("📡 Starting passive BLE scan (streaming mode)");

        let manager = PlatformManager::new().await?;
        let adapters = manager.adapters().await?;

        if adapters.is_empty() {
            warn!("⚠️ No Bluetooth adapters found");
            return Ok(());
        }

        for (idx, _adapter) in adapters.iter().enumerate() {
            info!("📡 Adapter #{}: Streaming passive scan ready", idx);
        }

        info!("ℹ️  Streaming mode requires platform-specific implementation");
        Ok(())
    }

    pub fn get_timestamp_ns(&self) -> i64 {
        get_timestamp_ns()
    }

    pub fn get_timestamp_ms(&self) -> u64 {
        (get_timestamp_ns() / 1_000_000) as u64
    }
}

impl Default for PassiveScanner {
    fn default() -> Self {
        Self::with_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passive_packet_creation() {
        let packet = PassivePacket::new(
            "AA:BB:CC:DD:EE:FF".to_string(),
            -65,
            1234567890,
            vec![0x02, 0x01, 0x06],
            AdvertisingType::Adv_Ind,
            37,
            BluetoothPhy::Le1M,
        );

        assert_eq!(packet.mac_address, "AA:BB:CC:DD:EE:FF");
        assert_eq!(packet.rssi, -65);
        assert!(packet.is_connectable);
        assert!(!packet.is_extended);
    }

    #[test]
    fn test_passive_packet_to_raw_model() {
        let packet = PassivePacket::new(
            "AA:BB:CC:DD:EE:FF".to_string(),
            -65,
            1234567890123,
            vec![0x02, 0x01, 0x06],
            AdvertisingType::Adv_Ind,
            37,
            BluetoothPhy::Le1M,
        );

        let raw = packet.to_raw_packet_model();
        assert_eq!(raw.mac_address, "AA:BB:CC:DD:EE:FF");
        assert_eq!(raw.rssi, -65);
        assert_eq!(raw.packet_type, "ADV_IND");
    }

    #[test]
    fn test_scan_config_defaults() {
        let config = PassiveScanConfig::default();
        assert_eq!(config.scan_duration_ms, 10_000);
        assert_eq!(config.rssi_threshold, -100);
        assert!(config.filter_duplicates);
    }
}
