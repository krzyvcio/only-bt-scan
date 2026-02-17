//! PCAP Exporter - Export Bluetooth packets to Wireshark-compatible PCAP format

use log::info;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone)]
struct PcapGlobalHeader {
    magic_number: u32,
    version_major: u16,
    version_minor: u16,
    thiszone: i32,
    sigfigs: u32,
    snaplen: u32,
    network: u32,
}

impl PcapGlobalHeader {
    fn new() -> Self {
        PcapGlobalHeader {
            magic_number: 0xa1b2c3d4,
            version_major: 2,
            version_minor: 4,
            thiszone: 0,
            sigfigs: 0,
            snaplen: 65535,
            network: 201,
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.magic_number.to_le_bytes());
        bytes.extend_from_slice(&self.version_major.to_le_bytes());
        bytes.extend_from_slice(&self.version_minor.to_le_bytes());
        bytes.extend_from_slice(&(self.thiszone as u32).to_le_bytes());
        bytes.extend_from_slice(&self.sigfigs.to_le_bytes());
        bytes.extend_from_slice(&self.snaplen.to_le_bytes());
        bytes.extend_from_slice(&self.network.to_le_bytes());
        bytes
    }
}

#[derive(Debug, Clone)]
struct PcapPacketHeader {
    ts_sec: u32,
    ts_usec: u32,
    incl_len: u32,
    orig_len: u32,
}

impl PcapPacketHeader {
    fn new(packet_len: u32) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();

        PcapPacketHeader {
            ts_sec: now.as_secs() as u32,
            ts_usec: now.subsec_micros(),
            incl_len: packet_len,
            orig_len: packet_len,
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.ts_sec.to_le_bytes());
        bytes.extend_from_slice(&self.ts_usec.to_le_bytes());
        bytes.extend_from_slice(&self.incl_len.to_le_bytes());
        bytes.extend_from_slice(&self.orig_len.to_le_bytes());
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BluetoothPacketType {
    Command = 0x01,
    AclOut = 0x02,
    ScoOut = 0x03,
    Event = 0x04,
    AclIn = 0x05,
    ScoIn = 0x06,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HciPcapPacket {
    pub packet_type: u8,
    pub data: Vec<u8>,
    pub source_mac: Option<String>,
    pub dest_mac: Option<String>,
}

impl HciPcapPacket {
    pub fn new(packet_type: u8, data: Vec<u8>) -> Self {
        HciPcapPacket {
            packet_type,
            data,
            source_mac: None,
            dest_mac: None,
        }
    }

    pub fn event(event_code: u8, parameters: &[u8]) -> Self {
        let mut data = vec![event_code];
        let len = parameters.len() as u8;
        data.push(len);
        data.extend_from_slice(parameters);

        HciPcapPacket {
            packet_type: BluetoothPacketType::Event as u8,
            data,
            source_mac: None,
            dest_mac: None,
        }
    }

    pub fn acl_in(handle: u16, data: &[u8]) -> Self {
        let mut packet_data = Vec::new();
        let handle_flags = (handle & 0x0FFF) | ((2 & 0x03) << 12);
        packet_data.extend_from_slice(&handle_flags.to_le_bytes());
        packet_data.extend_from_slice(&(data.len() as u16).to_le_bytes());
        packet_data.extend_from_slice(data);

        HciPcapPacket {
            packet_type: BluetoothPacketType::AclIn as u8,
            data: packet_data,
            source_mac: None,
            dest_mac: None,
        }
    }

    pub fn get_size(&self) -> u32 {
        (1 + self.data.len()) as u32
    }
}

#[derive(Debug)]
pub struct PcapExporter {
    file_path: String,
    file: Option<File>,
    packet_count: u64,
    total_bytes: u64,
}

impl PcapExporter {
    pub fn new(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let path = Path::new(file_path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let file = File::create(path)?;
        info!("Created PCAP file: {}", file_path);

        Ok(PcapExporter {
            file_path: file_path.to_string(),
            file: Some(file),
            packet_count: 0,
            total_bytes: 0,
        })
    }

    pub fn new_to_memory() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(PcapExporter {
            file_path: ":memory:".to_string(),
            file: None,
            packet_count: 0,
            total_bytes: 0,
        })
    }

    pub fn write_header(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut file) = self.file {
            let header = PcapGlobalHeader::new();
            let header_bytes = header.to_bytes();
            file.write_all(&header_bytes)?;
            self.total_bytes += header_bytes.len() as u64;
            info!("Wrote PCAP global header (24 bytes)");
        }
        Ok(())
    }

    pub fn write_packet(
        &mut self,
        packet: &HciPcapPacket,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut file) = self.file {
            let packet_size = packet.get_size();
            let pkt_header = PcapPacketHeader::new(packet_size);
            let pkt_header_bytes = pkt_header.to_bytes();

            file.write_all(&pkt_header_bytes)?;
            file.write_all(&[packet.packet_type])?;
            file.write_all(&packet.data)?;

            self.packet_count += 1;
            self.total_bytes += (pkt_header_bytes.len() + 1 + packet.data.len()) as u64;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut file) = self.file {
            file.flush()?;
        }
        Ok(())
    }

    pub fn get_stats(&self) -> PcapExportStats {
        PcapExportStats {
            file_path: self.file_path.clone(),
            packet_count: self.packet_count,
            total_bytes: self.total_bytes,
        }
    }
}

/// Helper function to export a list of Bluetooth frames to a PCAP buffer
pub fn export_frames_to_buffer(
    frames: &[crate::raw_sniffer::BluetoothFrame],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut buffer = Vec::new();

    // 1. Write Global Header
    let global_header = PcapGlobalHeader::new();
    buffer.extend_from_slice(&global_header.to_bytes());

    for (_idx, frame) in frames.iter().enumerate() {
        // 2. Create LE Advertising Report HCI Event
        // Structure:
        // [0] Subevent: 0x02 (LE Advertising Report)
        // [1] Num Reports: 0x01
        // [2] Event Type: 0x00 (ADV_IND) mapping
        // [3] Addr Type: 0x00 (Public)
        // [4..9] Addr: 6 bytes (Little Endian)
        // [10] Length: N
        // [11..N+11] Data
        // [N+11] RSSI

        let mut hci_params = Vec::new();
        hci_params.push(0x02); // Subevent: LE Advertising Report
        hci_params.push(0x01); // Num Reports: 1

        let event_type = match format!("{}", frame.frame_type).as_str() {
            "ADV_IND" => 0x00,
            "ADV_DIRECT_IND" => 0x01,
            "ADV_SCAN_IND" => 0x02,
            "ADV_NONCONN_IND" => 0x03,
            "SCAN_RSP" => 0x04,
            _ => 0x00,
        };
        hci_params.push(event_type);
        hci_params.push(0x00); // Addr Type: Public (default)

        // Parse MAC bytes (reverse order for HCI)
        let mac_clean = frame.mac_address.replace(":", "");
        if let Ok(mac_bytes) = hex::decode(&mac_clean) {
            let mut mac_reversed = mac_bytes.clone();
            mac_reversed.reverse();
            hci_params.extend_from_slice(&mac_reversed);
        } else {
            hci_params.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
        }

        hci_params.push(frame.advertising_data.len() as u8);
        hci_params.extend_from_slice(&frame.advertising_data);
        hci_params.push(frame.rssi as u8);

        // LE Meta Event (0x3E)
        let mut event_data = vec![0x3E, hci_params.len() as u8];
        event_data.extend_from_slice(&hci_params);

        // HCI Packet Header (Packet Type = Event 0x04)
        let mut final_pkt_data = vec![0x04];
        final_pkt_data.extend_from_slice(&event_data);

        // 3. Write PCAP Packet Record
        let pkt_len = final_pkt_data.len() as u32;
        let pkt_header = PcapPacketHeader::new(pkt_len);

        // Set custom timestamp from frame
        let mut custom_header = pkt_header.clone();
        custom_header.ts_sec = frame.timestamp.timestamp() as u32;
        custom_header.ts_usec = frame.timestamp.timestamp_subsec_micros();

        buffer.extend_from_slice(&custom_header.to_bytes());
        buffer.extend_from_slice(&final_pkt_data);
    }

    info!(
        "Exported {} frames to PCAP buffer ({:.1} KB)",
        frames.len(),
        buffer.len() as f64 / 1024.0
    );
    Ok(buffer)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcapExportStats {
    pub file_path: String,
    pub packet_count: u64,
    pub total_bytes: u64,
}

impl PcapExportStats {
    pub fn avg_packet_size(&self) -> f64 {
        if self.packet_count == 0 {
            0.0
        } else {
            self.total_bytes as f64 / self.packet_count as f64
        }
    }

    pub fn to_string(&self) -> String {
        format!(
            "PCAP Export: {} packets, {} bytes ({:.2} KB)",
            self.packet_count,
            self.total_bytes,
            self.total_bytes as f64 / 1024.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcap_global_header() {
        let header = PcapGlobalHeader::new();
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 24);
    }

    #[test]
    fn test_hci_event_packet() {
        let packet = HciPcapPacket::event(0x05, &[0x00, 0x01, 0x02, 0x13]);
        assert_eq!(packet.packet_type, BluetoothPacketType::Event as u8);
    }

    #[test]
    fn test_hci_acl_packet() {
        let packet = HciPcapPacket::acl_in(0x0001, &[0x01, 0x02, 0x03, 0x04]);
        assert_eq!(packet.packet_type, BluetoothPacketType::AclIn as u8);
    }
}
