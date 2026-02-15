#[cfg(target_os = "windows")]
pub use self::{
    adapter::{WindowsHciAdapter, WindowsHciScanner},
    models::{HciAdapterInfo, LeAdvertisingReport},
};

#[cfg(target_os = "windows")]
mod adapter {
    use super::models::LeAdvertisingReport;
    use log::{debug, info, warn};
    use std::collections::HashMap;
    use std::time::Duration;

    pub struct WindowsHciAdapter {
        adapter_id: String,
        port: Option<Box<dyn serialport::SerialPort>>,
        is_open: bool,
        device_rssi_cache: HashMap<String, i8>,
    }

    impl WindowsHciAdapter {
        pub fn new(adapter_id: String) -> Self {
            Self {
                adapter_id,
                port: None,
                is_open: false,
                device_rssi_cache: HashMap::new(),
            }
        }

        pub fn open(&mut self) -> Result<(), String> {
            info!("📡 Opening Windows HCI adapter: {}", self.adapter_id);

            let available_ports = serialport::available_ports()
                .map_err(|e| format!("Failed to list ports: {}", e))?;
            if available_ports.is_empty() {
                return Err("No serial ports found.".to_string());
            }

            for port_info in available_ports {
                info!("Attempting to open port: {}", port_info.port_name);
                match serialport::new(&port_info.port_name, 115200)
                    .timeout(std::time::Duration::from_millis(10))
                    .open()
                {
                    Ok(port) => {
                        self.port = Some(port);
                        self.is_open = true;
                        info!("✅ HCI adapter opened on port {}", port_info.port_name);
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("Failed to open port {}: {}", port_info.port_name, e);
                        continue;
                    }
                }
            }

            Err("Could not open any serial port for HCI.".to_string())
        }

        pub fn close(&mut self) -> Result<(), String> {
            info!("🔌 Closing HCI adapter");
            self.is_open = false;
            Ok(())
        }

        pub fn send_hci_command(
            &mut self,
            opcode: u16,
            parameters: &[u8],
        ) -> Result<Vec<u8>, String> {
            if !self.is_open || self.port.is_none() {
                return Err("HCI adapter not open".to_string());
            }

            debug!(
                "📤 Sending HCI command 0x{:04X} ({} bytes)",
                opcode,
                parameters.len()
            );

            let packet_type: u8 = 0x01;
            let opcode_le = opcode.to_le_bytes();
            let param_len = parameters.len() as u8;

            let mut command = vec![packet_type];
            command.extend_from_slice(&opcode_le);
            command.push(param_len);
            command.extend_from_slice(parameters);

            debug!("HCI command packet: {:02X?}", command);

            if let Some(port) = self.port.as_mut() {
                port.write_all(&command)
                    .map_err(|e| format!("Failed to write command: {}", e))?;
            }

            Ok(vec![])
        }

        pub fn receive_hci_event(&mut self) -> Result<Vec<u8>, String> {
            if !self.is_open || self.port.is_none() {
                return Err("HCI adapter not open".to_string());
            }

            let port = self.port.as_mut().unwrap();

            let mut header = [0u8; 2];
            port.read_exact(&mut header)
                .map_err(|e| format!("Failed to read header: {}", e))?;

            let param_len = header[1] as usize;

            let mut event_packet = Vec::with_capacity(2 + param_len);
            event_packet.extend_from_slice(&header);

            if param_len > 0 {
                let mut params = vec![0u8; param_len];
                port.read_exact(&mut params)
                    .map_err(|e| format!("Failed to read params: {}", e))?;
                event_packet.extend_from_slice(&params);
            }

            debug!("Received HCI Event: {:02X?}", event_packet);

            Ok(event_packet)
        }

        pub fn enable_le_advertising_scan(&mut self) -> Result<(), String> {
            info!("🔍 Enabling LE advertising data scan");

            let scan_type = 0x01;
            let address_type = 0x01;
            let filter_duplicates = 0x01;

            let mut params = vec![scan_type, address_type];
            params.extend_from_slice(&100u16.to_le_bytes());
            params.extend_from_slice(&100u16.to_le_bytes());
            params.push(0x00);
            params.push(0x00);

            self.send_hci_command(0x200B, &params)?;

            let enable_params = vec![0x01, filter_duplicates];
            self.send_hci_command(0x200C, &enable_params)?;

            Ok(())
        }
    }

    pub struct WindowsHciScanner {
        adapter: WindowsHciAdapter,
    }

    impl WindowsHciScanner {
        pub fn new(adapter_id: String) -> Self {
            Self {
                adapter: WindowsHciAdapter::new(adapter_id),
            }
        }

        pub async fn start_scan(&mut self) -> Result<(), String> {
            self.adapter.open()?;
            self.adapter.enable_le_advertising_scan()?;
            info!("✅ Windows HCI scanning started");

            Ok(())
        }

        pub async fn receive_advertisement(
            &mut self,
        ) -> Result<Option<LeAdvertisingReport>, String> {
            loop {
                let event_packet = self.adapter.receive_hci_event()?;

                if event_packet.len() < 2 {
                    continue;
                }

                let event_code = event_packet[0];

                if event_code == 0x3E {
                    let subevent_code = event_packet[2];

                    if subevent_code == 0x02 {
                        let num_reports = event_packet[3] as usize;
                        let offset = 4;

                        for _ in 0..num_reports {
                            if offset + 9 > event_packet.len() {
                                break;
                            }
                            let report_event_type = event_packet[offset];
                            let address_type = event_packet[offset + 1];
                            let address_bytes = &event_packet[offset + 2..offset + 8];
                            let address = format!(
                                "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                                address_bytes[5],
                                address_bytes[4],
                                address_bytes[3],
                                address_bytes[2],
                                address_bytes[1],
                                address_bytes[0]
                            );
                            let data_length = event_packet[offset + 8] as usize;
                            if offset + 9 + data_length + 1 > event_packet.len() {
                                break;
                            }
                            let advertising_data =
                                event_packet[offset + 9..offset + 9 + data_length].to_vec();
                            let rssi = event_packet[offset + 9 + data_length] as i8;

                            let report = LeAdvertisingReport {
                                event_type: report_event_type,
                                address_type,
                                address,
                                data_length: data_length as u8,
                                advertising_data,
                                rssi,
                            };

                            return Ok(Some(report));
                        }
                    }
                }
            }
        }

        pub async fn stop_scan(&mut self) -> Result<(), String> {
            self.adapter.close()?;
            info!("✅ Windows HCI scanning stopped");

            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
mod models {
    use crate::data_models::RawPacketModel;
    use chrono::Utc;

    #[derive(Debug, Clone)]
    pub struct HciAdapterInfo {
        pub adapter_id: String,
        pub friendly_name: String,
        pub address: String,
        pub is_primary: bool,
        pub supports_ble: bool,
        pub supports_bredr: bool,
        pub device_path: String,
    }

    #[derive(Debug, Clone)]
    pub struct LeAdvertisingReport {
        pub event_type: u8,
        pub address_type: u8,
        pub address: String,
        pub data_length: u8,
        pub advertising_data: Vec<u8>,
        pub rssi: i8,
    }

    impl LeAdvertisingReport {
        pub fn to_raw_packet(&self, packet_id: u64) -> RawPacketModel {
            let mut packet = RawPacketModel::new(
                self.address.clone(),
                Utc::now(),
                self.advertising_data.clone(),
            );

            packet.packet_id = packet_id;
            packet.rssi = self.rssi;
            packet.phy = "LE 1M".to_string();
            packet.channel = 37;
            packet.packet_type = match self.event_type {
                0x00 => "ADV_IND".to_string(),
                0x01 => "ADV_DIRECT_IND".to_string(),
                0x02 => "ADV_SCAN_IND".to_string(),
                0x03 => "ADV_NONCONN_IND".to_string(),
                0x04 => "SCAN_RSP".to_string(),
                _ => "UNKNOWN".to_string(),
            };

            packet
        }
    }
}
