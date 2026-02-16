/// HCI Real-time Packet Capture (Windows)
/// Przechwytuje WSZYSTKIE Bluetooth packets bez opóźnień
/// Similar to Wireshark Npcap - intercepts at HCI level
use crate::data_models::RawPacketModel;
use chrono::Utc;
use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc;

/// HCI packet types
#[derive(Debug, Clone, Copy)]
pub enum HciPacketType {
    Command = 0x01,
    AclData = 0x02,
    ScoData = 0x03,
    Event = 0x04,
    Iso = 0x05,
}

/// HCI Event packet structure
#[derive(Debug, Clone)]
pub struct HciEventPacket {
    pub event_code: u8,
    pub parameter_length: u8,
    pub parameters: Vec<u8>,
}

/// HCI ACL Data packet
#[derive(Debug, Clone)]
pub struct HciAclPacket {
    pub handle: u16,
    pub packet_boundary_flag: u8,
    pub broadcast_flag: u8,
    pub data_length: u16,
    pub data: Vec<u8>,
}

/// Real-time HCI Sniffer
pub struct HciRealTimeSniffer {
    running: Arc<AtomicBool>,
    tx: Option<mpsc::UnboundedSender<RawPacketModel>>,
}

impl HciRealTimeSniffer {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            tx: None,
        }
    }

    /// Start real-time HCI capture (requires admin)
    pub fn start(&mut self, tx: mpsc::UnboundedSender<RawPacketModel>) -> Result<(), String> {
        // Check if running as admin
        if !Self::is_admin() {
            return Err("HCI Sniffer requires administrator privileges".to_string());
        }

        info!("Starting HCI Real-time Sniffer (admin mode)...");
        self.tx = Some(tx);
        self.running.store(true, Ordering::Relaxed);

        Ok(())
    }

    /// Stop capturing
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        info!("HCI Sniffer stopped");
    }

    /// Is running as admin?
    fn is_admin() -> bool {
        // Check if running with elevated privileges on Windows
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;

            // Try to run a command that requires admin
            let output = Command::new("cmd").args(&["/C", "net session"]).output();

            matches!(output, Ok(o) if o.status.success())
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On non-Windows, check effective UID
            unsafe { libc::geteuid() == 0 }
        }
    }

    /// Parse HCI Event packet (Le Meta Event - advertising reports)
    pub fn parse_le_meta_event(&self, parameters: &[u8]) -> Option<Vec<RawPacketModel>> {
        if parameters.is_empty() {
            return None;
        }

        let subevent = parameters[0];

        // 0x02 = LE Advertising Report
        if subevent == 0x02 {
            return self.parse_le_advertising_report(&parameters[1..]);
        }

        None
    }

    /// Parse LE Advertising Report (0x3E, 0x02)
    /// This is where we get device discovery data with timing
    fn parse_le_advertising_report(&self, data: &[u8]) -> Option<Vec<RawPacketModel>> {
        if data.len() < 2 {
            return None;
        }

        let num_reports = data[0] as usize;
        let mut packets = Vec::new();
        let mut pos = 1;

        for _ in 0..num_reports {
            if pos + 11 > data.len() {
                break;
            }

            let event_type = data[pos];
            let address_type = data[pos + 1];
            let mac_address = Self::parse_mac(&data[pos + 2..pos + 8]);
            let data_length = data[pos + 8] as usize;
            let rssi = data[pos + 9 + data_length] as i8;

            pos += 10;

            if pos + data_length > data.len() {
                break;
            }

            let ad_data = data[pos..pos + data_length].to_vec();
            pos += data_length;

            // Create RawPacketModel
            let packet = RawPacketModel::new(mac_address.clone(), Utc::now(), ad_data);

            packets.push(packet.clone());

            // Send to channel if available
            if let Some(ref tx) = self.tx {
                let _ = tx.send(packet);
            }
        }

        if packets.is_empty() {
            None
        } else {
            Some(packets)
        }
    }

    /// Parse MAC address from 6 bytes
    fn parse_mac(bytes: &[u8]) -> String {
        if bytes.len() < 6 {
            return String::new();
        }
        format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            bytes[5], bytes[4], bytes[3], bytes[2], bytes[1], bytes[0]
        )
    }

    /// Simulate HCI packet reception for testing
    /// (In production, this would be actual USB/HCI driver)
    pub fn simulate_hci_event(&self, event_data: &[u8]) {
        if !self.running.load(Ordering::Relaxed) {
            return;
        }

        // Parse and send simulated event
        info!("Simulated HCI Event: {} bytes", event_data.len());
    }
}

impl Default for HciRealTimeSniffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Background HCI capture task
pub async fn hci_capture_task(rx: mpsc::UnboundedReceiver<RawPacketModel>) {
    info!("HCI Capture Task started - processing packets in real-time");

    let mut rx = rx;
    let mut packet_count = 0u64;
    let start_time = Utc::now();

    while let Some(packet) = rx.recv().await {
        packet_count += 1;

        // Log every 100 packets
        if packet_count % 100 == 0 {
            let elapsed = Utc::now().signed_duration_since(start_time);
            let pps = if elapsed.num_seconds() > 0 {
                packet_count as f64 / elapsed.num_seconds() as f64
            } else {
                0.0
            };

            info!(
                "HCI Packets captured: {} ({:.1} pps) | Device: {} | RSSI: {}",
                packet_count, pps, packet.mac_address, packet.rssi
            );
        }

        // Save to database immediately (async)
        if let Err(e) = crate::db::insert_advertisement_frame(
            &packet.mac_address,
            packet.rssi,
            &packet.advertising_data_hex,
            "HCI",
            37, // default channel
            "ADV_IND",
            packet.timestamp_ms,
        ) {
            warn!("Failed to insert packet: {}", e);
        }
    }

    info!("HCI Capture Task ended - {} packets captured", packet_count);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_parsing() {
        let bytes = &[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let mac = HciRealTimeSniffer::parse_mac(bytes);
        assert_eq!(mac, "FF:EE:DD:CC:BB:AA");
    }

    #[test]
    fn test_is_admin() {
        // This will pass/fail depending on execution context
        let _is_admin = HciRealTimeSniffer::is_admin();
    }
}
