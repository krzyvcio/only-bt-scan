/// Packet analyzer for terminal display
/// Provides formatting for Bluetooth LE advertisement packets

use crate::company_ids;
use crate::data_models::RawPacketModel;

/// Format a raw packet model for terminal display
pub fn format_packet_for_terminal(packet: &RawPacketModel) -> String {
    let adv_data = &packet.advertising_data;
    
    if adv_data.is_empty() {
        return format!("Packet {} from {}: Empty advertising data", packet.packet_id, packet.mac_address);
    }

    let timestamp_str = packet
        .timestamp
        .format("%Y-%m-%d %H:%M:%S%.3f UTC")
        .to_string();
    let mut result = format!(
        "Packet {} [{}] ({}b): ",
        packet.packet_id,
        timestamp_str,
        adv_data.len()
    );
    
    // Format first 32 bytes as hex
    let display_len = std::cmp::min(32, adv_data.len());
    let hex_parts: Vec<String> = adv_data[0..display_len].iter()
        .map(|b| format!("{:02X}", b))
        .collect();
    
    result.push_str(&hex_parts.join(" "));
    
    if adv_data.len() > 32 {
        result.push_str(&format!("... ({} bytes total)", adv_data.len()));
    }
    
    // Try to parse manufacturer data if present
    if let Some(manufacturer_data) = packet.manufacturer_data.iter().next() {
        let mfg_id = *manufacturer_data.0;
        let name = company_ids::get_company_name(mfg_id);
        if !name.starts_with("Unknown") {
            result.push_str(&format!(" [Mfg: 0x{:04X} = {}]", mfg_id, name));
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_empty_packet() {
        assert_eq!(format_packet_for_terminal(&[]), "Empty packet");
    }

    #[test]
    fn test_format_basic_packet() {
        let packet = vec![0x01, 0x02, 0x03];
        let formatted = format_packet_for_terminal(&packet);
        assert!(formatted.contains("01 02 03"));
    }
}
