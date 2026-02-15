use crate::hci_scanner::HciScanner;
use crate::mac_address_handler::MacAddress;
use crate::pcap_exporter::{HciPcapPacket, PcapExporter};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;

const MAX_RAW_PACKETS: usize = 500;
const DEFAULT_PAGE_SIZE: usize = 50;

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
    pub service_classes: Option<String>,  // Parsed from Device Class
    pub bt_device_type: Option<String>,   // Parsed from Device Class (renamed to avoid conflict)
    
    // Advertisement Data fields (wszystkie możliwe pola)
    pub ad_local_name: Option<String>,
    pub ad_tx_power: Option<i8>,
    pub ad_flags: Option<String>,
    pub ad_appearance: Option<String>,
    pub ad_service_uuids: Vec<String>,
    pub ad_manufacturer_name: Option<String>,
    pub ad_manufacturer_data: Option<String>,
    
    // Temporal metrics (1ms resolution)
    pub frame_interval_ms: Option<i32>,        // Time since last frame in milliseconds
    pub frames_per_second: Option<f32>,        // Transmission rate Hz
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
fn get_parsed_ad_data(conn: &rusqlite::Connection, mac_address: &str) -> crate::db::ParsedAdvertisementData {
    if let Ok(mut stmt) = conn.prepare(
        "SELECT advertising_data FROM ble_advertisement_frames
         WHERE mac_address = ?
         ORDER BY timestamp DESC
         LIMIT 1"
    ) {
        if let Ok(ad_hex) = stmt.query_row([mac_address], |row| row.get::<_, String>(0)) {
            return crate::db::parse_advertisement_data(&ad_hex);
        }
    }
    crate::db::ParsedAdvertisementData::default()
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
            
            // Load Advertisement Data for all devices
            for device in &mut device_list {
                let ad_data = get_parsed_ad_data(&conn, &device.mac_address);
                device.ad_local_name = ad_data.local_name;
                device.ad_tx_power = ad_data.tx_power;
                device.ad_flags = ad_data.flags;
                device.ad_appearance = ad_data.appearance;
                device.ad_service_uuids = ad_data.service_uuids;
                device.ad_manufacturer_name = ad_data.manufacturer_name;
                device.ad_manufacturer_data = ad_data.manufacturer_data;
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
    let conn = match rusqlite::Connection::open("bluetooth_scan.db") {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    };

    let mac = path.into_inner();

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
    let conn = match rusqlite::Connection::open("bluetooth_scan.db") {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    };

    let mac = path.into_inner();

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
    let mac_address = path.into_inner();

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
    log::info!("📡 Manual company IDs update requested via API");
    
    match crate::company_ids::update_from_bluetooth_sig().await {
        Ok(count) => {
            log::info!("✅ Updated {} company IDs", count);
            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": format!("Successfully updated {} company IDs from Bluetooth SIG", count),
                "count": count
            }))
        },
        Err(e) => {
            log::error!("❌ Failed to update company IDs: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": format!("Failed to update: {}", e)
            }))
        }
    }
}

pub fn configure_services(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/devices", web::get().to(get_devices))
            .route("/devices/{mac}", web::get().to(get_device_detail))
            .route("/devices/{mac}/history", web::get().to(get_device_history))
            .route("/devices/{mac}/l2cap", web::get().to(get_l2cap_info))
            .route("/mac/{mac}", web::get().to(get_mac_info))
            .route("/hci-scan", web::get().to(get_hci_scan))
            .route("/export-pcap", web::get().to(export_pcap))
            .route("/raw-packets", web::get().to(get_raw_packets))
            .route("/raw-packets/latest", web::get().to(get_latest_raw_packets))
            .route("/raw-packets/all", web::get().to(get_all_raw_packets))
            .route("/scan-history", web::get().to(get_scan_history))
            .route("/telemetry", web::get().to(get_telemetry))
            .route("/stats", web::get().to(get_stats))
            .route("/company-ids/stats", web::get().to(get_company_ids_stats))
            .route("/company-ids/update", web::post().to(update_company_ids)),
    )
    .route("/", web::get().to(index))
    .route("/styles.css", web::get().to(static_css))
    .route("/app.js", web::get().to(static_js));
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
