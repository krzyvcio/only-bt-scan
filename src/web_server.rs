use crate::hci_scanner::HciScanner;
use crate::mac_address_handler::MacAddress;
use crate::pcap_exporter::{HciPcapPacket, PcapExporter};
use crate::class_of_device;
use actix::Actor;
use actix::StreamHandler;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

const MAX_RAW_PACKETS: usize = 500;
const DEFAULT_PAGE_SIZE: usize = 50;

/// Validates MAC address format (AA:BB:CC:DD:EE:FF or AA-BB-CC-DD-EE-FF)
/// Returns normalized MAC address or error if invalid
pub fn validate_mac_address(mac: &str) -> Result<String, &'static str> {
    let trimmed = mac.trim();

    // Check length
    if trimmed.len() != 17 && trimmed.len() != 12 {
        return Err("Invalid MAC address length (expected 17 with separators or 12 without)");
    }

    // Check valid hex characters
    let cleaned: String = if trimmed.contains(':') || trimmed.contains('-') {
        trimmed.replace(':', "").replace('-', "").to_uppercase()
    } else {
        trimmed.to_uppercase()
    };

    if cleaned.len() != 12 {
        return Err("Invalid MAC address format");
    }

    // Validate each byte is valid hex
    for chunk in cleaned.as_bytes().chunks(2) {
        let byte_str = std::str::from_utf8(chunk).map_err(|_| "Invalid hex in MAC")?;
        u8::from_str_radix(byte_str, 16).map_err(|_| "Invalid hex in MAC")?;
    }

    // Normalize to colon-separated format
    let normalized: String = cleaned
        .as_bytes()
        .chunks(2)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect::<Vec<&str>>()
        .join(":");

    Ok(normalized)
}

/// Checks if MAC address is valid (without throwing error)
pub fn is_valid_mac(mac: &str) -> bool {
    validate_mac_address(mac).is_ok()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDevice {
    pub id: Option<i32>,
    pub mac_address: String,
    pub device_name: Option<String>,
    pub rssi: i8,
    pub first_seen: String,
    pub last_seen: String,
    pub manufacturer_id: Option<u16>,
    pub manufacturer_name: Option<String>,
    pub device_type: Option<String>,
    pub number_of_scan: i32,
    pub mac_type: Option<String>,
    pub is_rpa: bool,
    pub security_level: Option<String>,
    pub pairing_method: Option<String>,
    pub services: Vec<ApiService>,
    pub detection_count: Option<i64>,
    pub avg_rssi: Option<f64>,
    pub detection_percentage: f64,
    pub is_authenticated: bool,
    pub device_class: Option<String>,
    pub service_classes: Option<String>, // Parsed from Device Class
    pub bt_device_type: Option<String>,  // Parsed from Device Class (renamed to avoid conflict)

    // Advertisement Data fields (wszystkie możliwe pola)
    pub ad_local_name: Option<String>,
    pub ad_tx_power: Option<i8>,
    pub ad_flags: Option<String>,
    pub ad_appearance: Option<String>,
    pub ad_service_uuids: Vec<String>,
    pub ad_manufacturer_name: Option<String>,
    pub ad_manufacturer_data: Option<String>,

    // Temporal metrics (1ms resolution)
    pub frame_interval_ms: Option<i32>, // Time since last frame in milliseconds
    pub frames_per_second: Option<f32>, // Transmission rate Hz
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiService {
    pub uuid16: Option<u16>,
    pub uuid128: Option<String>,
    pub service_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPacket {
    pub id: i64,
    pub mac_address: String,
    pub rssi: i8,
    pub advertising_data: String,
    pub phy: String,
    pub channel: i32,
    pub frame_type: String,
    pub timestamp: String,
    pub scan_number: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanHistoryEntry {
    pub id: i64,
    pub rssi: i8,
    pub scan_number: i32,
    pub scan_timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceHistory {
    pub device: ApiDevice,
    pub scan_history: Vec<ScanHistoryEntry>,
    pub packet_history: Vec<RawPacket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiStats {
    pub total_devices: i32,
    pub total_packets: i64,
    pub recent_devices: i32,
    pub active_devices: i32,
    pub total_scans: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsocketMessage {
    pub msg_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub page: usize,
    pub page_size: usize,
    pub total: usize,
    pub total_pages: usize,
}

pub struct AppState {
    pub devices: Mutex<Vec<ApiDevice>>,
    pub raw_packets: Mutex<VecDeque<RawPacket>>,
    pub last_scan_time: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            devices: Mutex::new(Vec::new()),
            raw_packets: Mutex::new(VecDeque::with_capacity(MAX_RAW_PACKETS)),
            last_scan_time: Mutex::new(None),
        }
    }
}

static WS_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct WsSession {
    pub id: usize,
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("WebSocket client {} connected", self.id);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        log::info!("WebSocket client {} disconnected", self.id);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                ctx.pong(&msg);
            }
            Ok(ws::Message::Text(text)) => {
                log::debug!("WebSocket {} received: {}", self.id, text);
                let _ = ctx.text(text);
            }
            Ok(ws::Message::Close(reason)) => {
                log::info!("WebSocket {} close: {:?}", self.id, reason);
                ctx.close(reason);
            }
            _ => {}
        }
    }
}

pub async fn ws_endpoint(
    req: actix_web::HttpRequest,
    stream: web::Payload,
) -> Result<actix_web::HttpResponse, actix_web::Error> {
    let session_id = WS_COUNTER.fetch_add(1, Ordering::SeqCst);
    let ws_session = WsSession { id: session_id };
    ws::start(ws_session, &req, stream)
}

pub fn update_devices(devices: Vec<ApiDevice>) {
    if let Some(state) = get_state() {
        if let Ok(mut d) = state.devices.lock() {
            *d = devices;
        }
    }
}

pub fn add_raw_packet(packet: RawPacket) {
    if let Some(state) = get_state() {
        if let Ok(mut packets) = state.raw_packets.lock() {
            if packets.len() >= MAX_RAW_PACKETS {
                packets.pop_front();
            }
            packets.push_back(packet);
        }
    }
}

pub fn update_last_scan(time: String) {
    if let Some(state) = get_state() {
        if let Ok(mut t) = state.last_scan_time.lock() {
            *t = Some(time);
        }
    }
}

fn get_state() -> Option<web::Data<AppState>> {
    None
}

pub fn init_state() -> web::Data<AppState> {
    web::Data::new(AppState::default())
}

/// Get last advertisement data for a device and parse it
fn get_parsed_ad_data(
    conn: &rusqlite::Connection,
    mac_address: &str,
) -> crate::db::ParsedAdvertisementData {
    if let Ok(mut stmt) = conn.prepare(
        "SELECT advertising_data FROM ble_advertisement_frames
         WHERE mac_address = ?
         ORDER BY timestamp DESC
         LIMIT 1",
    ) {
        if let Ok(ad_hex) = stmt.query_row([mac_address], |row| row.get::<_, String>(0)) {
            return crate::db::parse_advertisement_data(&ad_hex);
        }
    }
    crate::db::ParsedAdvertisementData::default()
}

/// Batch get advertisement data for multiple devices (fixes N+1 problem)
/// Returns a HashMap of MAC -> ParsedAdvertisementData
fn get_parsed_ad_data_batch(
    conn: &rusqlite::Connection,
    mac_addresses: &[String],
) -> std::collections::HashMap<String, crate::db::ParsedAdvertisementData> {
    use std::collections::HashMap;

    let mut result = HashMap::new();

    if mac_addresses.is_empty() {
        return result;
    }

    // Build query with IN clause
    let placeholders: Vec<&str> = mac_addresses.iter().map(|_| "?").collect();
    let query = format!(
        r#"SELECT mac_address, advertising_data 
           FROM ble_advertisement_frames f1
           WHERE id = (
               SELECT MAX(id) FROM ble_advertisement_frames 
               WHERE mac_address = f1.mac_address
           )
           AND mac_address IN ({})"#,
        placeholders.join(",")
    );

    if let Ok(mut stmt) = conn.prepare(&query) {
        let mut rows = match stmt.query(rusqlite::params_from_iter(mac_addresses.iter())) {
            Ok(r) => r,
            Err(_) => return result,
        };

        while let Ok(Some(row)) = rows.next() {
            if let (Ok(mac), Ok(ad_hex)) = (row.get::<_, String>(0), row.get::<_, String>(1)) {
                result.insert(mac, crate::db::parse_advertisement_data(&ad_hex));
            }
        }
    }

    // Fill in missing MACs with defaults
    for mac in mac_addresses {
        result
            .entry(mac.clone())
            .or_insert_with(|| crate::db::ParsedAdvertisementData::default());
    }

    result
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    page: Option<usize>,
    page_size: Option<usize>,
}

pub async fn get_devices(web::Query(params): web::Query<PaginationParams>) -> impl Responder {
    let conn = match rusqlite::Connection::open("bluetooth_scan.db") {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    };

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .min(100)
        .max(1);
    let offset = (page - 1) * page_size;

    let total: usize = conn
        .query_row("SELECT COUNT(*) FROM devices", [], |row| row.get(0))
        .unwrap_or(0);

    let mut stmt = match conn.prepare(
        "SELECT d.id, d.mac_address, d.device_name, d.rssi, d.first_seen, d.last_seen,
                d.manufacturer_id, d.manufacturer_name, d.device_type, d.number_of_scan,
                d.mac_type, d.is_rpa, d.security_level, d.pairing_method, d.is_authenticated, d.device_class
         FROM devices d
         ORDER BY d.last_seen DESC
         LIMIT ? OFFSET ?",
    ) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Query error: {}", e)
            }))
        }
    };

    let devices: Result<Vec<ApiDevice>, _> = stmt
        .query_map([page_size as i64, offset as i64], |row| {
            let device_class: Option<String> = row.get(15).ok();
            let (service_classes, bt_device_type) = if let Some(ref dc) = device_class {
                crate::db::parse_device_class(Some(dc.as_str()))
            } else {
                (None, None)
            };

            Ok(ApiDevice {
                id: row.get(0)?,
                mac_address: row.get(1)?,
                device_name: row.get(2)?,
                rssi: row.get(3)?,
                first_seen: row.get(4)?,
                last_seen: row.get(5)?,
                manufacturer_id: row.get(6)?,
                manufacturer_name: row.get(7)?,
                device_type: row.get(8)?,
                number_of_scan: row.get(9).unwrap_or(1),
                mac_type: row.get(10).ok(),
                is_rpa: row.get::<_, i32>(11).unwrap_or(0) != 0,
                security_level: row.get(12).ok(),
                pairing_method: row.get(13).ok(),
                services: Vec::new(),
                detection_count: None,
                avg_rssi: None,
                detection_percentage: 0.0,
                is_authenticated: row.get::<_, i32>(14).unwrap_or(0) != 0,
                device_class,
                service_classes,
                bt_device_type,
                // Advertisement Data fields (default, będzie updates w Rust)
                ad_local_name: None,
                ad_tx_power: None,
                ad_flags: None,
                ad_appearance: None,
                ad_service_uuids: Vec::new(),
                ad_manufacturer_name: None,
                ad_manufacturer_data: None,
                frame_interval_ms: None,
                frames_per_second: None,
            })
        })
        .map_err(|e| e.to_string())
        .and_then(|iter| {
            iter.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        });

    match devices {
        Ok(mut device_list) => {
            // Load services for all devices in one query (avoid N+1)
            let device_ids: Vec<i32> = device_list.iter().filter_map(|d| d.id).collect();
            if !device_ids.is_empty() {
                if let Ok(services_map) = get_all_device_services(&conn, &device_ids) {
                    for device in &mut device_list {
                        if let Some(id) = device.id {
                            if let Some(services) = services_map.get(&id) {
                                device.services = services.clone();
                            }
                        }
                    }
                }
            }

            // Load Advertisement Data for all devices (BATCH - fixes N+1)
            let macs: Vec<String> = device_list.iter().map(|d| d.mac_address.clone()).collect();
            let ad_data_map = get_parsed_ad_data_batch(&conn, &macs);

            for device in &mut device_list {
                if let Some(ad_data) = ad_data_map.get(&device.mac_address) {
                    device.ad_local_name = ad_data.local_name.clone();
                    device.ad_tx_power = ad_data.tx_power;
                    device.ad_flags = ad_data.flags.clone();
                    device.ad_appearance = ad_data.appearance.clone();
                    device.ad_service_uuids = ad_data.service_uuids.clone();
                    device.ad_manufacturer_name = ad_data.manufacturer_name.clone();
                    device.ad_manufacturer_data = ad_data.manufacturer_data.clone();
                }
            }

            let total_pages = (total as f64 / page_size as f64).ceil() as usize;
            HttpResponse::Ok().json(PaginatedResponse {
                data: device_list,
                page,
                page_size,
                total,
                total_pages,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e
        })),
    }
}

fn get_device_services_internal(
    conn: &rusqlite::Connection,
    device_id: i32,
) -> Result<Vec<ApiService>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT uuid16, uuid128, service_name FROM ble_services WHERE device_id = ?")?;

    let services = stmt.query_map([device_id], |row| {
        Ok(ApiService {
            uuid16: row.get(0)?,
            uuid128: row.get(1)?,
            service_name: row.get(2)?,
        })
    })?;

    services.collect()
}

fn get_all_device_services(
    conn: &rusqlite::Connection,
    device_ids: &[i32],
) -> Result<std::collections::HashMap<i32, Vec<ApiService>>, rusqlite::Error> {
    use std::collections::HashMap;

    let mut result = HashMap::new();

    let placeholders = device_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!(
        "SELECT device_id, uuid16, uuid128, service_name FROM ble_services WHERE device_id IN ({})",
        placeholders
    );

    let mut stmt = conn.prepare(&query)?;

    let services = stmt.query_map(rusqlite::params_from_iter(device_ids.iter()), |row| {
        let device_id: i32 = row.get(0)?;
        Ok((
            device_id,
            ApiService {
                uuid16: row.get(1)?,
                uuid128: row.get(2)?,
                service_name: row.get(3)?,
            },
        ))
    })?;

    for service_result in services {
        if let Ok((device_id, service)) = service_result {
            result
                .entry(device_id)
                .or_insert_with(Vec::new)
                .push(service);
        }
    }

    Ok(result)
}

pub async fn get_device_detail(path: web::Path<String>) -> impl Responder {
    let raw_mac = path.into_inner();

    // Validate MAC address
    let mac = match validate_mac_address(&raw_mac) {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": e
            }));
        }
    };

    let conn = match rusqlite::Connection::open("bluetooth_scan.db") {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    };

    let device: Option<ApiDevice> = conn
        .query_row(
            "SELECT d.id, d.mac_address, d.device_name, d.rssi, d.first_seen, d.last_seen,
                d.manufacturer_id, d.manufacturer_name, d.device_type, d.number_of_scan,
                d.mac_type, d.is_rpa, d.security_level, d.pairing_method, d.is_authenticated, d.device_class
         FROM devices d
         WHERE d.mac_address = ?",
            [&mac],
            |row| {
                let device_class: Option<String> = row.get(15).ok();
                let (service_classes, bt_device_type) = if let Some(ref dc) = device_class {
                    crate::db::parse_device_class(Some(dc.as_str()))
                } else {
                    (None, None)
                };
                
                Ok(ApiDevice {
                    id: row.get(0)?,
                    mac_address: row.get(1)?,
                    device_name: row.get(2)?,
                    rssi: row.get(3)?,
                    first_seen: row.get(4)?,
                    last_seen: row.get(5)?,
                    manufacturer_id: row.get(6)?,
                    manufacturer_name: row.get(7)?,
                    device_type: row.get(8)?,
                    number_of_scan: row.get(9).unwrap_or(1),
                    mac_type: row.get(10).ok(),
                    is_rpa: row.get::<_, i32>(11).unwrap_or(0) != 0,
                    security_level: row.get(12).ok(),
                    pairing_method: row.get(13).ok(),
                    services: Vec::new(),
                    detection_count: None,
                    avg_rssi: None,
                    detection_percentage: 0.0,
                    is_authenticated: row.get::<_, i32>(14).unwrap_or(0) != 0,
                    device_class,
                    service_classes,
                    bt_device_type,
                    ad_local_name: None,
                    ad_tx_power: None,
                    ad_flags: None,
                    ad_appearance: None,
                    ad_service_uuids: Vec::new(),
                    ad_manufacturer_name: None,
                    ad_manufacturer_data: None,
                    frame_interval_ms: None,
                    frames_per_second: None,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())
        .ok()
        .flatten();

    match device {
        Some(mut d) => {
            if let Ok(id) = d.id.ok_or(()) {
                if let Ok(services) = get_device_services_internal(&conn, id) {
                    d.services = services;
                }
            }

            // Load Advertisement Data
            let ad_data = get_parsed_ad_data(&conn, &d.mac_address);
            d.ad_local_name = ad_data.local_name;
            d.ad_tx_power = ad_data.tx_power;
            d.ad_flags = ad_data.flags;
            d.ad_appearance = ad_data.appearance;
            d.ad_service_uuids = ad_data.service_uuids;
            d.ad_manufacturer_name = ad_data.manufacturer_name;
            d.ad_manufacturer_data = ad_data.manufacturer_data;

            HttpResponse::Ok().json(d)
        }
        None => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Device not found"
        })),
    }
}

pub async fn get_raw_packets(web::Query(params): web::Query<PaginationParams>) -> impl Responder {
    let conn = match rusqlite::Connection::open("bluetooth_scan.db") {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    };

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .min(100)
        .max(1);
    let offset = (page - 1) * page_size;

    let total: usize = conn
        .query_row("SELECT COUNT(*) FROM ble_advertisement_frames", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    let mut stmt = match conn.prepare(
        "SELECT id, mac_address, rssi, advertising_data, phy, channel, frame_type, timestamp
         FROM ble_advertisement_frames
         ORDER BY timestamp DESC
         LIMIT ? OFFSET ?",
    ) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Query error: {}", e)
            }))
        }
    };

    let packets: Result<Vec<RawPacket>, _> = stmt
        .query_map([page_size as i64, offset as i64], |row| {
            Ok(RawPacket {
                id: row.get(0)?,
                mac_address: row.get(1)?,
                rssi: row.get(2)?,
                advertising_data: row.get(3)?,
                phy: row.get(4)?,
                channel: row.get(5)?,
                frame_type: row.get(6)?,
                timestamp: row.get(7)?,
                scan_number: None,
            })
        })
        .map_err(|e| e.to_string())
        .and_then(|iter| {
            iter.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        });

    match packets {
        Ok(p) => {
            let total_pages = (total as f64 / page_size as f64).ceil() as usize;
            HttpResponse::Ok().json(PaginatedResponse {
                data: p,
                page,
                page_size,
                total,
                total_pages,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e
        })),
    }
}

pub async fn decode_cod(web::Query(params): web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let cod_str = params.get("cod").and_then(|s| s.parse::<u32>().ok());
    
    match cod_str {
        Some(cod) => {
            let device_class = class_of_device::format_cod(cod);
            let services = class_of_device::get_cod_services(cod);
            
            HttpResponse::Ok().json(serde_json::json!({
                "cod": cod,
                "device_class": device_class,
                "services": services
            }))
        }
        None => HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Missing or invalid 'cod' parameter (expected u32)"
        })),
    }
}

pub async fn get_stats() -> impl Responder {
    let conn = match rusqlite::Connection::open("bluetooth_scan.db") {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    };

    let total_devices: i32 = conn
        .query_row("SELECT COUNT(*) FROM devices", [], |row| row.get(0))
        .unwrap_or(0);

    let total_packets: i64 = conn
        .query_row("SELECT COUNT(*) FROM ble_advertisement_frames", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    let recent_devices: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM devices WHERE last_seen > datetime('now', '-5 minutes')",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let active_devices: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM devices WHERE last_seen > datetime('now', '-1 minute')",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    HttpResponse::Ok().json(ApiStats {
        total_devices,
        total_packets,
        recent_devices,
        active_devices,
        total_scans: conn
            .query_row(
                "SELECT COALESCE(MAX(counter), 0) FROM scan_counter",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0),
    })
}

pub async fn get_all_raw_packets(
    web::Query(params): web::Query<PaginationParams>,
) -> impl Responder {
    let conn = match rusqlite::Connection::open("bluetooth_scan.db") {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    };

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .min(100)
        .max(1);
    let offset = (page - 1) * page_size;

    let total: usize = conn
        .query_row("SELECT COUNT(*) FROM ble_advertisement_frames", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    let mut stmt = match conn.prepare(
        "SELECT id, mac_address, rssi, advertising_data, phy, channel, frame_type, timestamp
         FROM ble_advertisement_frames
         ORDER BY timestamp DESC
         LIMIT ? OFFSET ?",
    ) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Query error: {}", e)
            }))
        }
    };

    let packets: Result<Vec<RawPacket>, _> = stmt
        .query_map([page_size as i64, offset as i64], |row| {
            Ok(RawPacket {
                id: row.get(0)?,
                mac_address: row.get(1)?,
                rssi: row.get(2)?,
                advertising_data: row.get(3)?,
                phy: row.get(4)?,
                channel: row.get(5)?,
                frame_type: row.get(6)?,
                timestamp: row.get(7)?,
                scan_number: None,
            })
        })
        .map_err(|e| e.to_string())
        .and_then(|iter| {
            iter.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        });

    match packets {
        Ok(p) => {
            let total_pages = (total as f64 / page_size as f64).ceil() as usize;
            HttpResponse::Ok().json(PaginatedResponse {
                data: p,
                page,
                page_size,
                total,
                total_pages,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e
        })),
    }
}

pub async fn get_scan_history(web::Query(params): web::Query<PaginationParams>) -> impl Responder {
    let conn = match rusqlite::Connection::open("bluetooth_scan.db") {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    };

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .min(100)
        .max(1);
    let offset = (page - 1) * page_size;

    let total: usize = conn
        .query_row("SELECT COUNT(*) FROM scan_history", [], |row| row.get(0))
        .unwrap_or(0);

    let mut stmt = match conn.prepare(
        "SELECT sh.id, sh.rssi, sh.scan_number, sh.scan_timestamp, d.mac_address
         FROM scan_history sh
         JOIN devices d ON sh.device_id = d.id
         ORDER BY sh.scan_timestamp DESC
         LIMIT ? OFFSET ?",
    ) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Query error: {}", e)
            }))
        }
    };

    let history: Result<Vec<serde_json::Value>, _> = stmt
        .query_map([page_size as i64, offset as i64], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "rssi": row.get::<_, i8>(1)?,
                "scan_number": row.get::<_, i32>(2)?,
                "timestamp": row.get::<_, String>(3)?,
                "mac_address": row.get::<_, String>(4)?,
            }))
        })
        .map_err(|e| e.to_string())
        .and_then(|iter| {
            iter.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        });

    match history {
        Ok(h) => {
            let total_pages = (total as f64 / page_size as f64).ceil() as usize;
            HttpResponse::Ok().json(PaginatedResponse {
                data: h,
                page,
                page_size,
                total,
                total_pages,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e
        })),
    }
}

pub async fn get_device_history(path: web::Path<String>) -> impl Responder {
    let raw_mac = path.into_inner();

    // Validate MAC address
    let mac = match validate_mac_address(&raw_mac) {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": e
            }));
        }
    };

    let conn = match rusqlite::Connection::open("bluetooth_scan.db") {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    };

    let device: Option<ApiDevice> = conn
        .query_row(
            "SELECT d.id, d.mac_address, d.device_name, d.rssi, d.first_seen, d.last_seen,
                d.manufacturer_id, d.manufacturer_name, d.device_type, d.number_of_scan,
                d.mac_type, d.is_rpa, d.security_level, d.pairing_method, d.is_authenticated, d.device_class
         FROM devices d
         WHERE d.mac_address = ?",
            [&mac],
            |row| {
                let device_class: Option<String> = row.get(15).ok();
                let (service_classes, bt_device_type) = if let Some(ref dc) = device_class {
                    crate::db::parse_device_class(Some(dc.as_str()))
                } else {
                    (None, None)
                };
                
                Ok(ApiDevice {
                    id: row.get(0)?,
                    mac_address: row.get(1)?,
                    device_name: row.get(2)?,
                    rssi: row.get(3)?,
                    first_seen: row.get(4)?,
                    last_seen: row.get(5)?,
                    manufacturer_id: row.get(6)?,
                    manufacturer_name: row.get(7)?,
                    device_type: row.get(8)?,
                    number_of_scan: row.get(9).unwrap_or(1),
                    mac_type: row.get(10).ok(),
                    is_rpa: row.get::<_, i32>(11).unwrap_or(0) != 0,
                    security_level: row.get(12).ok(),
                    pairing_method: row.get(13).ok(),
                    services: Vec::new(),
                    detection_count: None,
                    avg_rssi: None,
                    detection_percentage: 0.0,
                    is_authenticated: row.get::<_, i32>(14).unwrap_or(0) != 0,
                    device_class,
                    service_classes,
                    bt_device_type,
                    ad_local_name: None,
                    ad_tx_power: None,
                    ad_flags: None,
                    ad_appearance: None,
                    ad_service_uuids: Vec::new(),
                    ad_manufacturer_name: None,
                    ad_manufacturer_data: None,
                    frame_interval_ms: None,
                    frames_per_second: None,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())
        .ok()
        .flatten();

    match device {
        Some(mut d) => {
            // Load Advertisement Data
            let ad_data = get_parsed_ad_data(&conn, &d.mac_address);
            d.ad_local_name = ad_data.local_name;
            d.ad_tx_power = ad_data.tx_power;
            d.ad_flags = ad_data.flags;
            d.ad_appearance = ad_data.appearance;
            d.ad_service_uuids = ad_data.service_uuids;
            d.ad_manufacturer_name = ad_data.manufacturer_name;
            d.ad_manufacturer_data = ad_data.manufacturer_data;

            let mut scan_history = Vec::new();
            let mut packet_history = Vec::new();

            if let Some(device_id) = d.id {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, rssi, scan_number, scan_timestamp FROM scan_history
                     WHERE device_id = ? ORDER BY scan_timestamp DESC LIMIT 100",
                    )
                    .ok();

                if let Some(ref mut s) = stmt {
                    let history: Vec<ScanHistoryEntry> = s
                        .query_map([device_id], |row| {
                            Ok(ScanHistoryEntry {
                                id: row.get(0)?,
                                rssi: row.get(1)?,
                                scan_number: row.get(2)?,
                                scan_timestamp: row.get(3)?,
                            })
                        })
                        .ok()
                        .map(|r| r.filter_map(|x| x.ok()).collect())
                        .unwrap_or_default();
                    scan_history = history;
                }

                let mut stmt = conn.prepare(
                    "SELECT id, mac_address, rssi, advertising_data, phy, channel, frame_type, timestamp
                     FROM ble_advertisement_frames WHERE device_id = ? ORDER BY timestamp DESC LIMIT 100"
                ).ok();

                if let Some(ref mut s) = stmt {
                    let packets: Vec<RawPacket> = s
                        .query_map([device_id], |row| {
                            Ok(RawPacket {
                                id: row.get(0)?,
                                mac_address: row.get(1)?,
                                rssi: row.get(2)?,
                                advertising_data: row.get(3)?,
                                phy: row.get(4)?,
                                channel: row.get(5)?,
                                frame_type: row.get(6)?,
                                timestamp: row.get(7)?,
                                scan_number: None,
                            })
                        })
                        .ok()
                        .map(|r| r.filter_map(|x| x.ok()).collect())
                        .unwrap_or_default();
                    packet_history = packets;
                }
            }

            HttpResponse::Ok().json(DeviceHistory {
                device: d,
                scan_history,
                packet_history,
            })
        }
        None => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Device not found"
        })),
    }
}

pub async fn get_latest_raw_packets(state: web::Data<AppState>) -> impl Responder {
    if let Ok(packets) = state.raw_packets.lock() {
        let packets_vec: Vec<RawPacket> = packets.iter().cloned().collect();
        HttpResponse::Ok().json(packets_vec)
    } else {
        HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to get packets"
        }))
    }
}

pub async fn get_l2cap_info(path: web::Path<String>) -> impl Responder {
    let raw_mac = path.into_inner();

    // Validate MAC address
    let mac_address = match validate_mac_address(&raw_mac) {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": e
            }));
        }
    };

    match rusqlite::Connection::open("bluetooth_scan.db") {
        Ok(conn) => {
            // Get device_id first
            let device_id: Option<i32> = conn
                .query_row(
                    "SELECT id FROM devices WHERE mac_address = ?",
                    [&mac_address],
                    |row| row.get(0),
                )
                .optional()
                .unwrap_or(None);

            match device_id {
                Some(_id) => {
                    // Return a placeholder L2CAP profile with the correct structure
                    let profile = crate::l2cap_analyzer::L2CapDeviceProfile {
                        mac_address: mac_address.clone(),
                        device_name: None,
                        channels: vec![],
                        psm_usage: std::collections::HashMap::new(),
                        total_tx_bytes: 0,
                        total_rx_bytes: 0,
                        supports_ble: true,
                        supports_bredr: false,
                        supports_eatt: false,
                    };
                    HttpResponse::Ok().json(profile)
                }
                None => HttpResponse::NotFound().json(serde_json::json!({
                    "error": "Device not found"
                })),
            }
        }
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Database error"
        })),
    }
}

pub async fn export_pcap() -> impl Responder {
    match PcapExporter::new("bluetooth_capture.pcap") {
        Ok(mut exporter) => {
            if let Err(e) = exporter.write_header() {
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to write PCAP header: {}", e)
                }));
            }

            // Simulate adding some HCI events to PCAP
            let event1 = HciPcapPacket::event(0x05, &[0x00, 0x01, 0x02, 0x13]);
            let event2 = HciPcapPacket::event(0x3E, &[0x02, 0x01, 0x7F, 0x01, 0x01]);
            let acl1 = HciPcapPacket::acl_in(0x0001, &[0x01, 0x02, 0x03, 0x04]);

            let packets = vec![event1, event2, acl1];
            for packet in packets {
                if let Err(e) = exporter.write_packet(&packet) {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Failed to write packet: {}", e)
                    }));
                }
            }

            if let Err(e) = exporter.flush() {
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to flush file: {}", e)
                }));
            }

            let stats = exporter.get_stats();
            HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "file": stats.file_path,
                "packets": stats.packet_count,
                "bytes": stats.total_bytes,
                "message": "PCAP file created successfully - open with Wireshark"
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to create PCAP file: {}", e)
        })),
    }
}

pub async fn get_mac_info(path: web::Path<String>) -> impl Responder {
    let mac_str = path.into_inner();

    match MacAddress::from_string(&mac_str) {
        Ok(mac) => {
            HttpResponse::Ok().json(serde_json::json!({
                "mac_address": mac.as_str().to_string(),
                "is_unicast": mac.is_unicast(),
                "is_multicast": mac.is_multicast(),
                "is_locally_administered": mac.is_locally_administered(),
                "is_universally_administered": mac.is_universally_administered(),
                "is_rpa": mac.is_rpa(),
                "is_static_random": mac.is_static_random(),
                "is_nrpa": mac.is_nrpa(),
                "manufacturer_id": format!("{:02X}:{:02X}:{:02X}", mac.as_bytes()[0], mac.as_bytes()[1], mac.as_bytes()[2]),
                "device_id": format!("{:02X}:{:02X}:{:02X}", mac.as_bytes()[3], mac.as_bytes()[4], mac.as_bytes()[5]),
                "address_type": if mac.is_rpa() { "RPA (Resolvable Private)" } else if mac.is_static_random() { "Static Random" } else if mac.is_locally_administered() { "Locally Administered" } else { "Public" }
            }))
        },
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Invalid MAC address: {}", e)
        }))
    }
}

pub async fn get_hci_scan() -> impl Responder {
    let mut scanner = HciScanner::default();

    // Simulate HCI events for demo
    scanner.simulate_hci_event(0x05, &[0x00, 0x01, 0x02, 0x13]);
    scanner.simulate_hci_event(0x3E, &[0x02, 0x01, 0x7F, 0x01, 0x01]);

    // Simulate L2CAP packets
    let att_packet = vec![0x04, 0x00, 0x1F, 0x00, 0x01, 0x02, 0x03, 0x04];
    let _ = scanner.simulate_l2cap_packet(&att_packet, Some("AA:BB:CC:DD:EE:FF".to_string()));

    let smp_packet = vec![0x06, 0x00, 0x23, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
    let _ = scanner.simulate_l2cap_packet(&smp_packet, Some("AA:BB:CC:DD:EE:FF".to_string()));

    let result = scanner.get_results();
    HttpResponse::Ok().json(result)
}

pub async fn index() -> impl Responder {
    let html = include_str!("../frontend/index.html");
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

pub async fn static_css() -> impl Responder {
    let css = include_str!("../frontend/styles.css");
    HttpResponse::Ok()
        .content_type("text/css; charset=utf-8")
        .body(css)
}

pub async fn static_js() -> impl Responder {
    let js = include_str!("../frontend/app.js");
    HttpResponse::Ok()
        .content_type("application/javascript; charset=utf-8")
        .body(js)
}

pub async fn get_telemetry() -> impl Responder {
    match crate::telemetry::get_global_telemetry() {
        Some(snapshot) => HttpResponse::Ok().json(snapshot),
        None => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Telemetry not available yet"
        })),
    }
}

#[derive(Serialize)]
pub struct RssiTelemetryResponse {
    pub timestamp: String,
    pub devices: Vec<RssiDeviceTelemetry>,
}

#[derive(Serialize)]
pub struct RssiDeviceTelemetry {
    pub mac: String,
    pub rssi: f64,
    pub trend: String,
    pub motion: String,
    pub slope: f64,
    pub variance: f64,
    pub sample_count: usize,
    pub confidence: f64,
}

fn calculate_confidence(state: &crate::rssi_analyzer::DeviceState) -> f64 {
    let sample_factor = (state.sample_count as f64 / 20.0).min(1.0);
    let variance_factor = 1.0 - (state.variance / 20.0).min(1.0) * 0.5 ;
    (sample_factor * variance_factor).max(0.0)
}

pub async fn get_rssi_telemetry() -> impl Responder {
    let manager = crate::get_rssi_manager();
    let states = manager.get_all_states();
    
    let devices: Vec<RssiDeviceTelemetry> = states
        .into_iter()
        .map(|(mac, state)| {
            let confidence = calculate_confidence(&state);
            RssiDeviceTelemetry {
                mac,
                rssi: state.rssi,
                trend: state.trend.to_string(),
                motion: state.motion.to_string(),
                slope: state.slope,
                variance: state.variance,
                sample_count: state.sample_count,
                confidence,
            }
        })
        .collect();
    
    HttpResponse::Ok().json(RssiTelemetryResponse {
        timestamp: chrono::Utc::now().to_rfc3339(),
        devices,
    })
}

pub async fn get_device_rssi_telemetry(path: web::Path<String>) -> impl Responder {
    let raw_mac = path.into_inner();
    let mac = match validate_mac_address(&raw_mac) {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": e
            }));
        }
    };
    
    let manager = crate::get_rssi_manager();
    match manager.get_device_state(&mac) {
        Some(state) => {
            let confidence = calculate_confidence(&state);
            HttpResponse::Ok().json(RssiDeviceTelemetry {
                mac: mac.clone(),
                rssi: state.rssi,
                trend: state.trend.to_string(),
                motion: state.motion.to_string(),
                slope: state.slope,
                variance: state.variance,
                sample_count: state.sample_count,
                confidence,
            })
        }
        None => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Device not found in telemetry"
        })),
    }
}

#[derive(Deserialize)]
pub struct TrendParams {
    pub hours: Option<u32>,
}

#[derive(Deserialize)]
pub struct RawRssiParams {
    pub limit: Option<u32>,
}

/// Get RSSI trend for a specific device
/// Query params: ?hours=5 (default 24 hours)
pub async fn get_device_rssi_trend(
    path: web::Path<String>,
    query: web::Query<TrendParams>,
) -> impl Responder {
    let raw_mac = path.into_inner();
    let mac = match validate_mac_address(&raw_mac) {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": e
            }))
        }
    };

    let hours = query.hours.unwrap_or(24);

    match crate::db::get_device_rssi_trend_pooled(&mac, hours) {
        Ok(trend_points) => {
            if trend_points.is_empty() {
                return HttpResponse::NotFound().json(serde_json::json!({
                    "error": "No telemetry data found for this device",
                    "device_mac": mac,
                    "hours": hours
                }));
            }

            let first_seen = trend_points.first().map(|p| p.timestamp);
            let last_seen = trend_points.last().map(|p| p.timestamp);
            let min_rssi = trend_points
                .iter()
                .map(|p| p.avg_rssi as i32)
                .min()
                .unwrap_or(0);
            let max_rssi = trend_points
                .iter()
                .map(|p| p.avg_rssi as i32)
                .max()
                .unwrap_or(0);
            let avg_rssi = trend_points.iter().map(|p| p.avg_rssi).sum::<f64>()
                / trend_points.len() as f64;

            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "device_mac": mac,
                "period_hours": hours,
                "data_points": trend_points.len(),
                "first_detected": first_seen,
                "last_detected": last_seen,
                "rssi_stats": {
                    "min": min_rssi,
                    "max": max_rssi,
                    "avg": format!("{:.2}", avg_rssi)
                },
                "trend": trend_points
            }))
        }
        Err(e) => {
            log::error!("Failed to get RSSI trend: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to retrieve trend data",
                "details": e.to_string()
            }))
        }
    }
}

/// Get RSSI measurements from last 24 hours for trend chart
pub async fn get_rssi_24h(path: web::Path<String>) -> impl Responder {
    let raw_mac = path.into_inner();
    let mac = match validate_mac_address(&raw_mac) {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({ "error": e }));
        }
    };

    match crate::db::get_raw_rssi_measurements_24h(&mac) {
        Ok(measurements) => {
            if measurements.is_empty() {
                return HttpResponse::NotFound().json(serde_json::json!({
                    "error": "No RSSI measurements found in last 24 hours",
                    "device_mac": mac
                }));
            }

            let first_rssi = measurements.first().map(|m| m.rssi).unwrap_or(0);
            let last_rssi = measurements.last().map(|m| m.rssi).unwrap_or(0);
            let rssi_delta = last_rssi - first_rssi;
            let trend = if rssi_delta > 5 {
                "getting_closer"
            } else if rssi_delta < -5 {
                "getting_farther"
            } else {
                "stable"
            };

            let rssi_values: Vec<i32> = measurements.iter().map(|m| m.rssi).collect();
            let avg_rssi = rssi_values.iter().sum::<i32>() as f64 / rssi_values.len() as f64;

            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "device_mac": mac,
                "time_range": "24 hours",
                "measurements_count": measurements.len(),
                "rssi_stats": {
                    "min": rssi_values.iter().min().copied().unwrap_or(0),
                    "max": rssi_values.iter().max().copied().unwrap_or(0),
                    "avg": avg_rssi
                },
                "trend": {
                    "direction": trend,
                    "delta": rssi_delta,
                    "description": match trend {
                        "getting_closer" => "Device is approaching",
                        "getting_farther" => "Device is moving away",
                        _ => "Signal is stable"
                    }
                },
                "measurements": measurements
            }))
        }
        Err(e) => {
            log::error!("Failed to get 24h RSSI: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

/// Get raw RSSI measurements from advertisement frames (last 100 by default)
/// Query params: ?limit=100 (default)
/// Returns actual signal strength readings with trend analysis
pub async fn get_raw_rssi(
    path: web::Path<String>,
    query: web::Query<RawRssiParams>,
) -> impl Responder {
    let raw_mac = path.into_inner();
    let mac = match validate_mac_address(&raw_mac) {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": e
            }))
        }
    };

    let limit = query.limit.unwrap_or(100).min(500); // Cap at 500 measurements

    match crate::db::get_raw_rssi_measurements_pooled(&mac, limit) {
        Ok(measurements) => {
            if measurements.is_empty() {
                return HttpResponse::NotFound().json(serde_json::json!({
                    "error": "No raw RSSI measurements found for this device",
                    "device_mac": mac,
                    "limit": limit
                }));
            }

            // Calculate trend: first vs last measurement
            let first_rssi = measurements.first().map(|m| m.rssi).unwrap_or(0);
            let last_rssi = measurements.last().map(|m| m.rssi).unwrap_or(0);
            let rssi_delta = last_rssi - first_rssi;
            let trend = if rssi_delta > 5 {
                "getting_closer" // RSSI increasing (less negative = closer)
            } else if rssi_delta < -5 {
                "getting_farther" // RSSI decreasing (more negative = farther)
            } else {
                "stable"
            };

            // Calculate statistics
            let rssi_values: Vec<i32> = measurements.iter().map(|m| m.rssi).collect();
            let min_rssi = rssi_values.iter().min().copied().unwrap_or(0);
            let max_rssi = rssi_values.iter().max().copied().unwrap_or(0);
            let avg_rssi = rssi_values.iter().sum::<i32>() as f64 / rssi_values.len() as f64;

            // Calculate signal quality distribution
            let excellent = measurements.iter().filter(|m| m.rssi >= -50).count();
            let good = measurements.iter().filter(|m| m.rssi >= -60 && m.rssi < -50).count();
            let fair = measurements.iter().filter(|m| m.rssi >= -70 && m.rssi < -60).count();
            let poor = measurements.iter().filter(|m| m.rssi >= -85 && m.rssi < -70).count();
            let very_poor = measurements.iter().filter(|m| m.rssi < -85).count();

            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "device_mac": mac,
                "measurements_count": measurements.len(),
                "time_range": {
                    "first": measurements.first().map(|m| m.timestamp),
                    "last": measurements.last().map(|m| m.timestamp)
                },
                "rssi_stats": {
                    "min": min_rssi,
                    "max": max_rssi,
                    "avg": format!("{:.2}", avg_rssi)
                },
                "trend": {
                    "direction": trend,
                    "delta_dbm": rssi_delta,
                    "description": match trend {
                        "getting_closer" => "Device is getting closer (signal strengthening)",
                        "getting_farther" => "Device is moving away (signal weakening)",
                        _ => "Device distance is stable"
                    }
                },
                "signal_quality_distribution": {
                    "excellent": excellent,
                    "good": good,
                    "fair": fair,
                    "poor": poor,
                    "very_poor": very_poor
                },
                "measurements": measurements
            }))
        }
        Err(e) => {
            log::error!("Failed to get raw RSSI measurements: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to retrieve raw RSSI data",
                "details": e.to_string()
            }))
        }
    }
}

/// Get real-time RSSI trend for a specific device (trend, motion, slope, variance)
pub async fn get_device_trend_state(
    path: web::Path<String>,
) -> impl Responder {
    let raw_mac = path.into_inner();
    let mac = match validate_mac_address(&raw_mac) {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": e
            }))
        }
    };

    match crate::rssi_trend_manager::GLOBAL_RSSI_MANAGER.get_device_state(&mac) {
        Some(state) => {
            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "device_mac": mac,
                "trend": state.trend.to_string(),
                "motion": state.motion.to_string(),
                "rssi": state.rssi,
                "slope_dbm_per_sec": format!("{:.4}", state.slope),
                "variance": format!("{:.2}", state.variance),
                "sample_count": state.sample_count,
                "metrics": {
                    "approaching": state.trend.to_string() == "approaching",
                    "leaving": state.trend.to_string() == "leaving",
                    "stable": state.trend.to_string() == "stable",
                    "moving": state.motion.to_string() == "moving",
                    "still": state.motion.to_string() == "still"
                }
            }))
        }
        None => {
            HttpResponse::NotFound().json(serde_json::json!({
                "error": "No trend data available for this device yet",
                "device_mac": mac
            }))
        }
    }
}

/// Get RSSI trend state for ALL devices
pub async fn get_all_device_trends() -> impl Responder {
    let snapshot = crate::rssi_trend_manager::GLOBAL_RSSI_MANAGER.get_snapshot();

    // Group by trend
    let approaching: Vec<_> = snapshot.devices.iter()
        .filter(|d| d.trend == "approaching")
        .collect();
    let leaving: Vec<_> = snapshot.devices.iter()
        .filter(|d| d.trend == "leaving")
        .collect();
    let stable: Vec<_> = snapshot.devices.iter()
        .filter(|d| d.trend == "stable")
        .collect();

    // Group by motion
    let moving: Vec<_> = snapshot.devices.iter()
        .filter(|d| d.motion == "moving")
        .collect();
    let still: Vec<_> = snapshot.devices.iter()
        .filter(|d| d.motion == "still")
        .collect();

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "timestamp": snapshot.timestamp,
        "total_devices": snapshot.devices.len(),
        "summary": {
            "approaching": approaching.len(),
            "leaving": leaving.len(),
            "stable": stable.len(),
            "moving": moving.len(),
            "still": still.len()
        },
        "by_trend": {
            "approaching": approaching,
            "leaving": leaving,
            "stable": stable
        },
        "by_motion": {
            "moving": moving,
            "still": still
        },
        "all_devices": snapshot.devices
    }))
}

/// Get company IDs cache statistics
pub async fn get_company_ids_stats() -> impl Responder {
    if let Some((count, last_updated)) = crate::company_ids::get_cache_stats() {
        HttpResponse::Ok().json(serde_json::json!({
            "count": count,
            "last_updated": last_updated,
            "cache_file": "company_ids_cache.json"
        }))
    } else {
        HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Company IDs cache not available"
        }))
    }
}

/// Trigger manual update of company IDs from Bluetooth SIG
pub async fn update_company_ids() -> impl Responder {
    match crate::company_ids::update_from_bluetooth_sig().await {
        Ok(count) => {
            log::info!("✅ Updated {} company IDs", count);
            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": format!("Successfully updated {} company IDs from Bluetooth SIG", count),
                "count": count
            }))
        }
        Err(e) => {
            log::error!("❌ Failed to update company IDs: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": format!("Failed to update: {}", e)
            }))
        }
    }
}

pub async fn get_device_behavior(path: web::Path<String>) -> impl Responder {
    let raw_mac = path.into_inner();
    let mac = match validate_mac_address(&raw_mac) {
        Ok(m) => m,
        Err(e) => return HttpResponse::BadRequest().json(serde_json::json!({"error": e})),
    };

    match crate::event_analyzer::analyze_device_behavior(&mac) {
        Some(behavior) => HttpResponse::Ok().json(behavior),
        None => HttpResponse::NotFound().json(serde_json::json!({
            "error": "No behavior data found for device",
            "mac": mac
        })),
    }
}

pub async fn get_device_anomalies(path: web::Path<String>) -> impl Responder {
    let raw_mac = path.into_inner();
    let mac = match validate_mac_address(&raw_mac) {
        Ok(m) => m,
        Err(e) => return HttpResponse::BadRequest().json(serde_json::json!({"error": e})),
    };

    let anomalies = crate::event_analyzer::detect_anomalies(&mac);
    HttpResponse::Ok().json(serde_json::json!({
        "mac": mac,
        "count": anomalies.len(),
        "anomalies": anomalies
    }))
}

pub async fn get_temporal_correlations() -> impl Responder {
    let correlations = crate::event_analyzer::find_correlations();
    HttpResponse::Ok().json(serde_json::json!({
        "count": correlations.len(),
        "correlations": correlations
    }))
}

pub async fn get_event_analyzer_stats() -> impl Responder {
    let event_count = crate::event_analyzer::get_event_count();
    HttpResponse::Ok().json(serde_json::json!({
        "event_count": event_count,
    }))
}

pub async fn clear_event_analyzer() -> impl Responder {
    crate::event_analyzer::clear_events();
    crate::data_flow_estimator::clear_estimates();
    HttpResponse::Ok().json(serde_json::json!({
        "message": "Event analyzer and data flow estimator cleared"
    }))
}

pub async fn get_device_data_flow(path: web::Path<String>) -> impl Responder {
    let raw_mac = path.into_inner();
    let mac = match validate_mac_address(&raw_mac) {
        Ok(m) => m,
        Err(e) => return HttpResponse::BadRequest().json(serde_json::json!({"error": e})),
    };

    match crate::data_flow_estimator::analyze_device(&mac) {
        Some(flow) => HttpResponse::Ok().json(flow),
        None => HttpResponse::NotFound().json(serde_json::json!({
            "error": "No flow data found for device",
            "mac": mac
        })),
    }
}

pub async fn get_all_data_flows() -> impl Responder {
    let flows = crate::data_flow_estimator::analyze_all_devices();
    HttpResponse::Ok().json(serde_json::json!({
        "count": flows.len(),
        "devices": flows
    }))
}

pub async fn get_data_flow_stats() -> impl Responder {
    let device_count = crate::data_flow_estimator::get_device_count();
    HttpResponse::Ok().json(serde_json::json!({
        "tracked_devices": device_count,
    }))
}

pub fn configure_services(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/devices", web::get().to(get_devices))
            .route("/devices/{mac}", web::get().to(get_device_detail))
            .route("/devices/{mac}/history", web::get().to(get_device_history))
            .route("/devices/{mac}/trend", web::get().to(get_device_rssi_trend))
            .route("/devices/{mac}/trend-state", web::get().to(get_device_trend_state))
            .route("/devices/{mac}/rssi-raw", web::get().to(get_raw_rssi))
            .route("/devices/{mac}/rssi-24h", web::get().to(get_rssi_24h))
            .route("/devices/{mac}/l2cap", web::get().to(get_l2cap_info))
            .route("/trends/all", web::get().to(get_all_device_trends))
            .route("/mac/{mac}", web::get().to(get_mac_info))
            .route("/hci-scan", web::get().to(get_hci_scan))
            .route("/export-pcap", web::get().to(export_pcap))
            .route("/raw-packets", web::get().to(get_raw_packets))
            .route("/raw-packets/latest", web::get().to(get_latest_raw_packets))
            .route("/raw-packets/all", web::get().to(get_all_raw_packets))
            .route("/scan-history", web::get().to(get_scan_history))
            .route("/telemetry", web::get().to(get_telemetry))
            .route("/rssi-telemetry", web::get().to(get_rssi_telemetry))
            .route("/devices/{mac}/rssi-telemetry", web::get().to(get_device_rssi_telemetry))
            .route("/decode-cod", web::get().to(decode_cod))
            .route("/stats", web::get().to(get_stats))
            .route("/company-ids/stats", web::get().to(get_company_ids_stats))
            .route("/company-ids/update", web::post().to(update_company_ids))
            .route("/devices/{mac}/behavior", web::get().to(get_device_behavior))
            .route("/devices/{mac}/anomalies", web::get().to(get_device_anomalies))
            .route("/temporal-correlations", web::get().to(get_temporal_correlations))
            .route("/event-analyzer-stats", web::get().to(get_event_analyzer_stats))
            .route("/event-analyzer-clear", web::post().to(clear_event_analyzer))
            .route("/devices/{mac}/data-flow", web::get().to(get_device_data_flow))
            .route("/data-flows", web::get().to(get_all_data_flows))
            .route("/data-flow-stats", web::get().to(get_data_flow_stats)),
    )
    .route("/", web::get().to(index))
    .route("/styles.css", web::get().to(static_css))
    .route("/app.js", web::get().to(static_js))
    .route("/ws", web::get().to(ws_endpoint));
}

pub async fn start_server(port: u16, app_state: web::Data<AppState>) -> std::io::Result<()> {
    log::info!("Starting web server on http://localhost:{}", port);

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .configure(configure_services)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
