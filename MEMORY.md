# only-bt-scan - Plan Napraw i Memory

## 📋 PRIORYTETY POPRAWEK

### 🔴 CRITICAL (Napraw dziś/jutro)

| # | Problem | Lokalizacja | Akcja |
|---|---------|-------------|-------|
| 1 | Wielokrotne otwieranie połączeń DB (każda funkcja `Connection::open()`) | `db.rs`, `db_frames.rs` | Utwórz `DbPool` z mutexem lub użyj `r2d2` |
| 2 | Brak walidacji MAC address w API | `web_server.rs:372, 739, 896` | Dodaj `validate_mac()` przed query |
| 3 | Memory leak w `DeviceTrackerManager` | `device_tracker.rs:227-248` | Dodaj limit na liczbę urządzeń |
| 4 | Race condition w `record_detection` | `device_tracker.rs:251-295` | Napraw podwójne lockowanie |

### 🟠 HIGH (Ten tydzień)

| # | Problem | Lokalizacja | Akcja |
|---|---------|-------------|-------|
| 5 | `.unwrap()` i `.expect()` na ścieżkach błędów | Wiele plików | Zmień na `match` lub `ok()` + log |
| 6 | Brak timeout na BLE connect | `bluetooth_scanner.rs:874` | Dodaj timeout z retry |
| 7 | N+1 query problem w API | `web_server.rs:288-297` | Batch query dla AD data |
| 8 | Brak limitów page_size | `web_server.rs` | Hard limit `min(100)` |
| 9 | Duplikacja kodu parsowania AD | `db.rs` vs `advertising_parser.rs` | Ujednolicić |

### 🟡 MEDIUM (Ten sprint)

| # | Problem | Lokalizacja | Akcja |
|---|---------|-------------|-------|
| 10 | Zbyteczne `.clone()` przy HashMap | `bluetooth_scanner.rs` | Użyj `Entry` API |
| 11 | Brak retry w async tasks | `lib.rs:157` | Dodaj exponential backoff |
| 12 | Duży `lib.rs` (859 linii) | `lib.rs` | Podziel na moduły |
| 13 | Brak trait dla scannerów | `bluetooth_scanner.rs` | Dodaj trait `Scanner` |

### 🟢 LOW (Kiedyś)

| # | Problem | Akcja |
|---|---------|-------|
| 14 | Brak rustdoc | Dodaj `///` comments |
| 15 | Magic numbers | Przenieś do `config.rs` |
| 16 | Niespójne nazwy | `ScannedDevice` vs `BluetoothDevice` vs `ApiDevice` |

---

## 🚀 QUICK START PO CLONE

```bash
# 1. Build
cargo build --release

# 2. Uruchom
cargo run

# 3. Sprawdź czy działa
curl http://localhost:8080/api/stats

# 4. Zobacz urządzenia
curl http://localhost:8080/api/devices
```

---

## 🔧 STRUKTURA PROJEKTU

```
only-bt-scan/
├── src/
│   ├── lib.rs              # ❌ ZBYT DUŻY - podzielić
│   ├── main.rs             # OK
│   ├── db.rs               # ✅ Dodano *_pooled funkcje
│   ├── db_pool.rs          # ✅ NOWY: Connection pool
│   ├── db_writer.rs        # ✅ NOWY: Batch DB writer z backpressure
│   ├── db_frames.rs        # OK
│   ├── adapter_manager.rs   # ✅ NOWY: Adapter detection i selection
│   ├── async_scanner.rs    # ✅ NOWY: Async scanner z kanałami
│   ├── bluetooth_scanner.rs # ⚠️  unwrap(), brak timeout
│   ├── web_server.rs       # ✅ Walidacja MAC, batch queries
│   ├── device_tracker.rs   # ✅ Limit + race fix
│   ├── advertising_parser.rs # ✅ Dobry wzorzec
│   ├── packet_tracker.rs   # ✅ Dobry wzorzec
│   └── ...
├── SPEC.md                  # ✅ NOWY: Specyfikacja techniczna
├── AGENTS.md               # ✅ NOWY: Instrukcje dla agentów
├── Cargo.toml
└── tests/
```

---

## 📝 NOTATKI DLA KOLEJNEJ SESJI

### Co już naprawione (oznaczone ✅)
- [x] #1: Utworzono DbPool w src/db_pool.rs + dodano funkcje *_pooled w db.rs
- [x] #2: Dodano validate_mac_address() w web_server.rs + walidacja w endpointach
- [x] #3: Dodano limit 10000 urządzeń w DeviceTrackerManager
- [x] #4: Naprawiono race condition - lock na całą operację
- [x] #7: Dodano get_parsed_ad_data_batch() - N+1 naprawione
- [x] #8: page_size limit już istniał (min 100)

### Co sprawdzić jako pierwsze
1. Czy baza danych nie jest zablokowana? (`lsof bluetooth_scan.db`)
2. Czy Bluetooth adapter działa? (`bluetoothctl list`)
3. Czy port 8080 jest wolny?

### Typowe problemy
- **SQLite locked**: Zamykanie połączenia które jest otwarte
- **No devices found**: Bluetooth disabled w BIOS
- **HCI capture failed**: Wymaga admin/root

---

## 📦 ZALEŻNOŚCI KLUCZOWE

```toml
btuleplug = "0.11"      # BLE scanning
rusqlite = "0.30"       # DB
tokio = "1"             # Async
actix-web = "4.4"       # Web server
windows = "0.54"        # Windows API
```

---

## 🎯 PLAN IMPLEMENTACJI

### Krok 1: DbPool (1-2h)
```
1. Utwórz src/db/pool.rs
2. Zamień Connection::open() na pool.get()
3. Przetestuj że działa
```

### Krok 2: Walidacja MAC (30 min)
```
1. Dodaj fn validate_mac() do web_server.rs
2. Użyj we wszystkich endpointach
```

### Krok 3: Fix tracker (1h)
```
1. Dodaj limit w DeviceTrackerManager
2. Napraw race condition
```

### Krok 4: Batch query (1h)
```
1. Utwórz get_advertisement_data_batch()
2. Zastąp pętlę w get_devices()
```

---

## 📌 KONTA (DO PRZYPOMNIENIA)

- [ ] Pamiętaj: projekt ma 50 modułów - nie próbuj wszystkiego na raz
- [ ] Testuj po każdej zmianie: `cargo test`
- [ ] Lint: `cargo clippy`
- [ ] Format: `cargo fmt`

---

*Last updated: 2026-02-16*
*Created for: Code review session*
