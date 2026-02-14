# Device Telemetry & Response Time Tracking

## Nowe Pola Urządzenia

Struktura `BluetoothDevice` została rozszerzona o trzy nowe pola do śledzenia czasów odpowiedzi:

```rust
pub struct BluetoothDevice {
    pub mac_address: String,           // Adres MAC
    pub name: Option<String>,          // Nazwa urządzenia
    pub rssi: i8,                      // Siła sygnału (dBm)
    pub device_type: DeviceType,       // BLE / BR/EDR / DUAL
    pub manufacturer_id: Option<u16>,  // ID producenta
    pub manufacturer_name: Option<String>, // Nazwa producenta
    pub is_connectable: bool,          // Czy można się połączyć
    pub services: Vec<ServiceInfo>,    // Usługi BLE
    
    // ===== NOWE - ŚLEDZENIE CZASU ODPOWIEDZI =====
    pub first_detected_ns: i64,        // Czas pierwszego wykrycia (nanosekund od epoch)
    pub last_detected_ns: i64,         // Czas ostatniego wykrycia (nanosekund od epoch)
    pub response_time_ms: u64,         // Całkowity czas odpowiedzi (milisekundy)
}
```

## Jak Działa Śledzenie

### 1. **Podczas Skanowania**
- Każde urządzenie otrzymuje sygnaturę czasową `first_detected_ns`
- Podczas każdego cyklu skanowania zapisywana jest `last_detected_ns`

### 2. **Scalanie Urządzeń**
Skaner automatycznie łączy wyniki z wielu cykli:
- Zachowuje **najwcześniejszy** czas pierwszego wykrycia
- Aktualizuje **najnowszy** czas ostatniego wykrycia
- Oblicza **czas odpowiedzi** = `(last_detected_ns - first_detected_ns) / 1,000,000`

### 3. **Wynik**
```
Cykl 1 (0s):   Urządzenie A wykryte
                first_detected_ns = 0 ns
                response_time_ms = 0 ms

Cykl 2 (5.2s): Urządzenie A znowu wykryte
                last_detected_ns = 5,200,000,000 ns
                response_time_ms = 5200 ms

Cykl 3 (10.5s): Urządzenie A znowu wykryte
                last_detected_ns = 10,500,000,000 ns
                response_time_ms = 10500 ms
```

## Format Wyświetlania

Nowa metoda `BluetoothScanner::format_device_info()` wyświetla wszystkie dane urządzenia:

```
AA:BB:CC:DD:EE:FF | iPhone 15 | -45 dBm | 5200 ms | DUAL Apple
```

Znaczenie:
- `AA:BB:CC:DD:EE:FF` - Adres MAC
- `iPhone 15` - Nazwa urządzenia
- `-45 dBm` - Siła sygnału (im bliżej 0, tym silniejszy)
- `5200 ms` - Czas odpowiedzi w milisekundach
- `DUAL` - Typ (BLE / BR/EDR / DUAL)
- `Apple` - Producent urządzenia

## Przykładowe Wyjście

```
=== Bluetooth Scanner v0.1.0 ===
✓ Database initialized
✓ Raw frame storage initialized
Starting scan cycle...
Starting Bluetooth scan with 3 cycles
Scan cycle 1/3
...
Scan cycle 3/3
Scan completed. Found 5 devices
╔════════════════════════════════════════════════════════════════╗
║ MAC Address        │ Device Name       │ RSSI │ Response │ Type │
╠════════════════════════════════════════════════════════════════╣
║ AA:BB:CC:DD:EE:FF | iPhone 15 | -45 dBm | 5200 ms | DUAL Apple ║
  ├─ Services: 12 detected
║ 11:22:33:44:55:66 | Samsung Watch | -67 dBm | 8500 ms | BLE Samsung ║
  ├─ Services: 8 detected
║ FF:EE:DD:CC:BB:AA | AirPods | -52 dBm | 3100 ms | BLE Apple ║
  ├─ Services: 5 detected
║ 12:34:56:78:90:AB | <Unknown> | -78 dBm | 10500 ms | BLE ? ║
║ AA:11:BB:22:CC:33 | Laptop | -55 dBm | 4200 ms | DUAL Intel ║
  ├─ Services: 15 detected
╚════════════════════════════════════════════════════════════════╝
Total devices in database: 5
Next scan in 5 minutes. Press Ctrl+C to stop.
```

## Interpretacja Czasu Odpowiedzi

### Czas Odpowiedzi Wysoki (>5000 ms)
- ✅ Urządzenie obecne przez wiele cykli skanowania
- ✅ Stabilne połączenie
- ℹ️ Przydatne do śledzenia długotrwałe

### Czas Odpowiedzi Niski (0-1000 ms)
- ⚠️ Urządzenie wykryte w niesąsiadujących cyklach LUB
- ℹ️ Właśnie pojawione, jeszcze nie wykryte w następnym cyklu

### Czas Odpowiedzi Średni (1000-5000 ms)
- ✅ Normalny zakres dla urządzeń aktywnie transmitujących
- ℹ️ Urządzenie stale w zasięgu

## Przypadki Użycia

### 1. **Monitorowanie Dostępności**
```sql
-- Urządzenia, które odpowiadają szybko (stabilne)
SELECT mac_address, response_time_ms, rssi
FROM devices_with_telemetry
WHERE response_time_ms > 5000  -- obecne przez wiele cykli
ORDER BY response_time_ms DESC;
```

### 2. **Wykrywanie Nowych Urządzeń**
```rust
// Urządzenia z czasem odpowiedzi bliskim 0
for device in devices {
    if device.response_time_ms < 1000 {
        println!("Nowe urządzenie: {}", device.mac_address);
    }
}
```

### 3. **Analiza Ruchu Sieciowego**
```
Urządzenie pojawia się:
- Raz na 5 cykli (25 minut) → response_time_ms ≈ 25000
- Ciągle dostępne (każdy cykl) → response_time_ms rosnący liniowo
- Czasu odpowiedzi 0 → właśnie się pojawiło
```

## Przechowywanie w Bazie Danych

Chwilowo `response_time_ms` jest obliczany na żywo. Aby zapisać do bazy:

```rust
// Dodaj do tabeli devices:
response_time_ms INTEGER,
first_detected_ts DATETIME,
last_detected_ts DATETIME,

// Lub do scan_history:
response_time_ms INTEGER,
detection_sequence INTEGER,  -- numer cyklu
```

## Aktualizacja Bazy Danych (Opcjonalnie)

Jeśli chcesz przechowywać historię czasów odpowiedzi:

```sql
-- Tabela do śledzenia ewolucji czasu odpowiedzi
CREATE TABLE device_response_times (
    id INTEGER PRIMARY KEY,
    device_id INTEGER NOT NULL,
    response_time_ms INTEGER,
    detected_in_cycles INTEGER,  -- ile cykli został wykryty
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(device_id) REFERENCES devices(id)
);
```

## Wydajność & Dokładność

- **Dokładność**: Nanosekundy (1 ns = 0.000001 ms)
- **Typ**: Całkowity czas trwania od pierwszego do ostatniego wykrycia
- **Reset**: Czas resetuje się przy każdym uruchomieniu skanera
- **Wzór**: `response_time_ms = (last_ns - first_ns) / 1,000,000`

## Integracja z Telegram

Możesz wysyłać powiadomienia o urządzeniach:

```rust
if device.response_time_ms == 0 {
    // Nowe urządzenie
    send_telegram(format!(
        "🆕 Nowe urządzenie: {} ({}) [RSSI: {} dBm]",
        device.name.as_ref().unwrap_or(&"<Unknown>".to_string()),
        device.mac_address,
        device.rssi
    ));
} else if device.response_time_ms > 30000 {
    // Długo obecne
    send_telegram(format!(
        "📡 Stabilne: {} | {} ms | {} dBm",
        device.mac_address,
        device.response_time_ms,
        device.rssi
    ));
}
```

## Przyszłe Rozszerzenia

- [ ] Przechowywanie historii w bazie
- [ ] Wykresy response_time w interfejsie webowym
- [ ] Alertowanie o anomaliach czasowych
- [ ] Prognozowanie dostępności na podstawie historii
- [ ] Eksport do pliku CSV z czasami

---

**Status**: ✅ Wdrożone i testowane
