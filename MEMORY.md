# MEMORY.md - Postępy projektu only-bt-scan

## 2026-02-17: Sesja główna (ostatnia)

### Zmiany w tej sesji:

#### 1. RSSI Trend 24h API
- Dodano endpoint `/api/devices/{mac}/rssi-24h` - zwraca pomiary z ostatnich 24h
- Dodano funkcję w db.rs: `get_raw_rssi_measurements_24h()`
- Frontend ładuje teraz dane z 24h zamiast 100 ostatnich pomiarów

#### 2. Telegram - naprawa raportów okresowych
- Zmieniono interwał z 5 min na **1 minutę**
- Naprawiono błędy SQL w zapytaniach datetime
- Przeniesiono task Telegrama do osobnego wątku z Tokio runtime (std::thread::spawn)
- Raport teraz zawiera: urządzenia, RSSI trends, surowe pakiety HTML

#### 3. Frontend - rozdzielenie app.js
Podzielono na mniejsze moduły:
- `api.js` - funkcje API
- `devices.js` - obsługa urządzeń
- `packets.js` - obsługa pakietów
- `rssi.js` - wykresy RSSI
- `modals.js` - okna modalowe
- `app.js` - główna logika

#### 4. Dodatki w UI
- Dodano przycisk "📶 RSSI" w modal szczegółów pakietu
- Kliknięcie przechodzi do zakładki RSSI i ładuje wykres dla tego urządzenia

#### 5. Kompilacja .env
- Dodano `env_config.rs` - ładuje .env przy kompilacji (include_str!)
- Nie trzeba już usuwać zmiennych systemowych przed uruchomieniem

#### 6. Class of Device
- Dodano `class_of_device.rs` - dekodowanie COD z pliku YAML
- Endpoint: `/api/decode-cod?cod=0x040100`

#### 7. Terminal - uptime
- Dodano wyświetlanie uptime przy skanowaniu: `Uptime: 1h 23m 45s`

#### 8. Terminal - wszystkie pakiety
- Zmieniono z wyświetlania tylko nowych urządzeń na **wszystkie wykryte pakiety**

#### 9. Czyszczenie warningów (subagent)
- Usunięto nieużywane funkcje z ble_uuids.rs (12 funkcji)
- Usunięto nieużywane funkcje z config_params.rs (4 funkcje)
- Naprawiono prefixowanie zmiennych (_) w telegram_notifier.rs
- Warningi: 336 -> 295 (-41)

---

## 2026-02-16: Integracja analyzerów

### event_analyzer.rs
- Globalny stan (LazyLock<Mutex>)
- Funkcje: add_timeline_events, analyze_device_behavior, detect_anomalies, find_correlations

### data_flow_estimator.rs
- Globalny stan (LazyLock<Mutex>)
- Wykrywanie protokołów: Meshtastic, Eddystone, iBeacon, AltBeacon, Cybertrack

### API endpoints dodane:
- GET /api/devices/{mac}/behavior
- GET /api/devices/{mac}/anomalies  
- GET /api/temporal-correlations
- GET /api/event-analyzer-stats
- GET /api/devices/{mac}/data-flow
- GET /api/data-flows
- GET /api/data-flow-stats
- POST /api/event-analyzer-clear

---

## Warningi - stan (336 -> 295)
Nadal nieużywane ale zostawione:
- Platformowe: android_ble_bridge, core_bluetooth_integration, bluey_integration (#[cfg])
- advertising_parser (używany przez vendor_protocols)

---

## Dane do Telegram raportu (co 1 min)
- Lista urządzeń z ostatniej minuty
- RSSI trends (approaching/moving away/stable)
- Surowe pakiety (do 50)
- Jako HTML załącznik: ble_scan_report.html

---

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

---

## Proponowane kolejne funkcjonalności (subagent 2)

### Priorytet HIGH:
1. **GATT Service Discovery** - połącz z urządzeniami BLE i odczytaj GATT services
2. **Live WebSocket Updates** - zamiast pollingu, push updates na web UI

### Priorytet MEDIUM:
3. **Interactive Telegram Commands** - /stats, /device MAC, /export
4. **Device Filtering & Search** - filtrowanie po producencie, RSSI
5. **Device Watchlist & Alerts** - śledź konkretne urządzenia
6. **Historical Trend Charts** - wykresy historyczne
7. **DB Query Optimization** - indeksy

### Priorytet LOW:
8. **Multi-Adapter Support** - wiele adapterów BT jednocześnie
9. **Data Export** - JSON/CSV/PCAP
10. **Extended Advertising Parsing** - BLE 5.0

---

## 2026-02-17: Subagenci - równoległe zadania

### Zadanie 1: DB Index (Data & Protocol)
- Dodano indeks: `idx_devices_rssi ON devices(rssi)`
- Większość indeksów już istniała

### Zadanie 2: Device Filtering UI (Frontend Dev)
- Dodano dropdowny filtrowania:
  - RSSI Range (Excellent/Good/Fair/Poor)
  - Manufacturer (dynamicznie z listy urządzeń)
  - MAC Type (Public/Random)
- Filtrowanie działa w czasie rzeczywistym

### Zadanie 3: WebSocket (Web API Dev)
- Dodano endpoint `/ws` dla połączeń WebSocket
- Dodano zależności: actix-web-actors, actix
- Struct WsSession do obsługi połączeń
- Broadcasting gotowy do integracji ze scannerem

### Warningi: 336 -> 295 (-41)
