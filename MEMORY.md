# MEMORY.md - Postępy analizy nieużywanego kodu

## 2026-02-16: Analiza rozpoczęta

### Podsumowanie
- **292 warningi** o nieużywanym kodzie z `cargo check`

### Top pliki z nieużywanym kodem (ilość warningów):
1. `l2cap_analyzer.rs` - ~16 (L2CAP protokół)
2. `ble_uuids.rs` - 16 (UUID services/characteristics)
3. `event_analyzer.rs` - 15 (analiza zdarzeń)
4. `core_bluetooth_integration.rs` - 14 (macOS CoreBluetooth)
5. `data_flow_estimator.rs` - 14 (estymacja przepływu)
6. `advertising_parser.rs` - 13+ (parsowanie AD struktur)
7. `link_layer.rs` - 14 (link layer)
8. `bluey_integration.rs` - 10 (Bluey scanner)
9. `android_ble_bridge.rs` - 8 (Android BLE)
10. `config_params.rs` - 10 (stałe konfiguracyjne)
11. `device_events.rs` - 12 (zdarzenia urządzeń)
12. `gatt_client.rs` - 11 (GATT klient)
13. `background.rs` - 6 (tryb background)

### Całe nieużywane moduły (można usunąć):
- `android_ble_bridge.rs` - Android bridge
- `bluey_integration.rs` - Bluey scanner
- `core_bluetooth_integration.rs` - macOS CoreBluetooth
- `data_flow_estimator.rs` - estymacja przepływu
- `event_analyzer.rs` - analizator zdarzeń
- `html_report.rs` - generowanie HTML
- `interactive_ui.rs` - interaktywny UI
- `gatt_client.rs` - GATT klient

### Kolejność czyszczenia:
1. **ETAP 1**: config_params.rs - proste stałe i funkcje
2. **ETAP 2**: company_ids.rs + company_id_reference.rs - stałe
3. **ETAP 3**: Całe moduły (android_ble, bluey, core_bluetooth)
4. **ETAP 4**: advertising_parser.rs (używa innego parsera?)
5. **ETAP 5**: Pozostałe

### Status: ANALIZA TRWA - raport z cargo check przetworzony

## 2026-02-16: ETAP 1 - config_params.rs
- **6 nieużywanych stałych/funkcji** (używane tylko w testach):
  - RSSI_SMOOTHING_FACTOR, RSSI_VARIANCE_LIMIT, SIGNAL_LOSS_TIMEOUT_MS
  - MIN_PACKET_INTERVAL_MS, TIMESTAMP_PRECISION_MS
  - rssi_to_signal_quality, is_signal_stable, is_duplicate_packet, should_process_packet
- Gotowe do usunięcia (tylko testy)

## 2026-02-16: ETAP 2 - company_ids.rs + company_id_reference.rs
- **Nieużywane w company_ids.rs**:
  - get_company_name_u32(), search_company(), is_registered()
- **Nieużywane w company_id_reference.rs**:
  - all_company_ids(), all_companies(), lookup_company_id_u32()
  - search_company_by_name() (używane przez search_company, które nie jest używane)
  - is_registered_company_id() (używane przez is_registered, które nie jest używane)

## 2026-02-16: ETAP 3 - Platform-specific moduły
Nie są martwe - są conditional #[cfg]:
- `android_ble_bridge.rs` - #[cfg(target_os = "android")]
- `core_bluetooth_integration.rs` - #[cfg(target_os = "macos")]
- `bluey_integration.rs` - ?

## Status: ANALIZA KONTYNUOWANA

## 2026-02-16: ETAP 4 - advertising_parser.rs
- Funkcje są używane WEWNĄTRZ modułu (parse_advertising_packet wywołuje wewnętrzne)
- Ale zewnętrzne wywołania są warunkowe #[cfg] lub nie używają wszystkich funkcji
- parse_advertising_packet() jest używany przez multi_method_scanner
- WAŻNE: vendor_protocols używa ParsedAdvertisingPacket z advertising_parser!

## 2026-02-16: ETAP 5 - Częściowo używane moduły
- `interactive_ui` - używany (display_countdown_interruptible), reszta martwa
- `html_report` - brak użycia w kodzie głównym
- `event_analyzer` - brak użycia
- `data_flow_estimator` - brak użycia
- `gatt_client` - brak użycia
- `link_layer` - brak użycia
- `l2cap_analyzer` - brak użycia

## Podsumowanie: Możliwe do usunięcia:
1. config_params.rs (6+ elementów) - TEST ONLY
2. company_ids/company_id_reference (7 funkcji) - nieużywane API
3. html_report.rs - cały moduł
4. event_analyzer.rs - cały moduł  
5. data_flow_estimator.rs - cały moduł
6. gatt_client.rs - cały moduł
7. link_layer.rs - cały moduł
8. l2cap_analyzer.rs - cały moduł

## Status: ANALIZA ZAKOŃCZONA - gotowe do czyszczenia

## 2026-02-16: INTEGRACJA - Event Analyzer + Data Flow Estimator

### Dodane funkcjonalności:

#### 1. event_analyzer.rs - zintegrowany z globalnym stanem
- Dodano globalny `EVENT_ANALYZER` (LazyLock<Mutex>)
- Funkcje API:
  - `add_timeline_events()` - dodawanie zdarzeń
  - `analyze_device_behavior(mac)` - analiza wzorców urządzenia
  - `detect_anomalies(mac)` - wykrywanie anomalii
  - `find_correlations()` - korelacje czasowe między urządzeniami
  - `get_event_count()` - licznik zdarzeń

#### 2. data_flow_estimator.rs - zintegrowany z globalnym stanem
- Dodano globalny `DATA_FLOW_ESTIMATOR` (LazyLock<Mutex>)
- Funkcje API:
  - `add_packet(mac, timestamp, payload, rssi)` - dodawanie pakietów
  - `analyze_device(mac)` - analiza przepływu dla urządzenia
  - `analyze_all_devices()` - analiza wszystkich urządzeń
  - `get_device_count()` - licznik śledzonych urządzeń
  - `clear_estimates()` - czyszczenie danych

#### 3. API endpoints (web_server.rs):
- `GET /api/devices/{mac}/behavior` - wzorce urządzenia
- `GET /api/devices/{mac}/anomalies` - anomalia urządzenia
- `GET /api/temporal-correlations` - korelacje czasowe
- `GET /api/event-analyzer-stats` - statystyki analyzera
- `GET /api/devices/{mac}/data-flow` - przepływ danych urządzenia
- `GET /api/data-flows` - wszystkie przepływy
- `GET /api/data-flow-stats` - statystyki przepływu

### Następne kroki:
1. ✅ Połączyć z ScannerWithTracking (dodawanie zdarzeń)
2. ✅ Uruchomić cargo check - 292 -> 277 warnings (-15)

### PODSUMOWANIE INTEGRACJI:
- **Warningi zmniejszone: 292 -> 277** (15 mniej)
- Nowe funkcjonalności:
  - Analiza wzorców urządzeń (event_analyzer)
  - Wykrywanie anomalii sygnału
  - Korelacje czasowe między urządzeniami
  - Estymacja przepływu danych (protokoły: Meshtastic, Eddystone, iBeacon, etc.)
  - 9 nowych API endpoints

### Pozostałe warningi (277) - do dalszej analizy:
- advertising_parser.rs - 23 warningi (używany przez vendor_protocols)
- ble_uuids.rs - 16+ (częściowo używane)
- config_params.rs - 9 (tylko testy używają)
- Inne moduły platformowe (android, bluey, core_bluetooth)

---

## 2026-02-17: Passive Scanner Module

### Implemented: passive_scanner.rs

Nowy moduł do pasywnego skanowania BLE z precyzyjnymi znacznikami czasu:

#### Features:
- **PassivePacket** - struktura pakietu z:
  - `packet_id` - unikalny ID pakietu
  - `mac_address` - adres MAC urządzenia
  - `rssi` - siła sygnału w dBm
  - `timestamp_ns` - znacznik czasu w nanosekundach
  - `timestamp_ms` - znacznik czasu w milisekundach
  - `phy` - PHY (LE 1M, LE 2M, LE Coded)
  - `channel` - kanał BLE (37-39)
  - `packet_type` - typ pakietu (ADV_IND, etc.)
  - `advertising_data` - surowe dane reklamowe
  - Flagi: is_connectable, is_scannable, is_directed, is_legacy, is_extended
  - tx_power - moc nadawania

- **PassiveScanConfig** - konfiguracja skanowania:
  - `scan_duration_ms` - czas skanowania
  - `filter_duplicates` - filtrowanie duplikatów
  - `rssi_threshold` - próg RSSI
  - `capture_legacy/capture_extended` - typy pakietów

- **PassiveScanner** - główny skaner:
  - `start_passive_scan()` - synchroniczne skanowanie
  - `start_passive_scan_streaming()` - strumieniowanie (placeholder)
  - `get_timestamp_ns/ms()` - precyzyjne znaczniki czasu

#### Integration:
- Wykorzystuje `quanta` do precyzyjnego pomiaru czasu (nanosekundy)
- Integracja z `data_models::RawPacketModel`
- Wykorzystuje btleplug do cross-platform skanowania
- Deduplikacja pakietów w oknie czasowym

#### Files modified/created:
- Created: `src/passive_scanner.rs` (nowy moduł)
- Modified: `src/lib.rs` (dodany moduł)
