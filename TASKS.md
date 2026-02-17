# TASKS.md - only-bt-scan

## Aktualne zadania

### Priorytet wysoki

- [ ] Naprawić 332 warnings z `cargo check`
- [x] Zaimplementować passive scanning z RSSI i raw packets
- [x] Dodać eksport do PCAP/Wireshark

### Priorytet średni

- [ ] Dodać `async-trait` do istniejących traitów
- [ ] Użyć `quanta` do high-precision timing w packet tracker
- [ ] Zaimplementować Bluetooth Mesh support (btmesh)

### Priorytet niski

- [ ] Dodać TUI z ratatui
- [ ] Dodać WebSocket dla real-time updates
- [ ] Dodać Prometheus metrics

---

## Ukończone

- [x] Dodać pakiety: `async-trait`, `quanta`, `libc`
- [x] Optymalizacja budowania (profile.release + dev)

---

## Propozycje nowych zadan

### Skany i protokoly
- [ ] Wykrywanie typu adresu (public/random, resolvable/static) + prezentacja w UI
- [ ] Tryb "burst" skanowania (krotkie serie co X sekund)
- [x] Auto-pause skanu przy braku reklam przez N minut
- [ ] Whitelist/blacklist prefiksow MAC w configu

### Parsowanie reklam (AD) i GATT
- [ ] Parser Battery Level (AD 0x0A) z walidacja zakresu
- [ ] Parser Service Data 16/32/128-bit + mapowanie UUID->nazwa
- [ ] Wykrywanie iBeacon/Eddystone (heurystyki)
- [ ] Wykrywanie BLE Mesh (Provisioning/Proxy/Network)

### Baza danych i wydajnosc
- [ ] TTL/retencja: auto-usuwanie starych rekordow
- [ ] Agregaty czasowe (1m/5m/1h) dla liczby reklam
- [ ] Indeksy pod RSSI/time/MAC + migracja
- [ ] Eksport CSV/JSON z filtrowaniem

### API i integracje
- [ ] /devices/summary (top-K, ostatnia aktywnosc)
- [ ] /devices/{mac}/history z paginacja
- [ ] /devices/nearby z progiem RSSI i oknem czasu
- [ ] Autoryzacja API tokenem w naglowku
- [ ] Webhooki (Slack/Telegram) na nowe urzadzenia

### UI
- [ ] Timeline per MAC (RSSI + zmiany AD)
- [ ] Mini-dashboard (top RSSI, UUID, count)
- [ ] Tryb prezentacyjny (fullscreen + auto-refresh)
- [ ] Heatmap sygnalu z agregatami

### Analityka i detekcje
- [ ] Wykrywanie rotacji MAC (random churn)
- [ ] Detekcja skokow RSSI (spoofing/pattern)
- [ ] Fingerprint reklam i grupowanie urzadzen
- [ ] Alerty progowe (RSSI, nowe UUID, liczba reklam)

### Stabilnosc i monitoring
- [ ] Backoff dla operacji BLE/GATT
- [ ] Limity RAM i eviction dla cache
- [ ] Metryki DB pool (liczba polaczen, latencja)
- [ ] Alarm "no scan" przez N minut

Super baza już jest: **skanuje BLE, ma web UI i działa w terminalu** — więc zamiast „budować od zera”, sensownie jest iść w **stabilizację, obserwowalność, analizę i skalowanie**. Poniżej masz **realistyczny backlog** pod istniejący projekt w Rust, ułożony jak do normalnego developmentu (MVP → hardening → features → research).

---

# 📋 Backlog dla projektu: BLE Scanner (Rust + Web + TUI)

## 🧱 1. Fundamenty techniczne (stabilność + jakość)

**P1 – Krytyczne**

* [ ] Ujednolicenie modelu danych `SignalInfo` (MAC, RSSI, timestamp, AD, source, adapter_id)
* [ ] Centralny pipeline zdarzeń (np. `tokio::broadcast` / `mpsc`)
* [ ] Standaryzacja błędów (`thiserror` / `anyhow`) i typów `Result`
* [ ] Logging + poziomy (trace/debug/info/warn/error)
* [ ] Graceful shutdown (CTRL+C, SIGTERM)
* [ ] Limity pamięci: ring-buffer / LRU cache na urządzenia
* [ ] Backpressure w kanałach async (żeby nie zabić RAMu)

**P2 – Ważne**

* [ ] Feature flags w Cargo (`web`, `tui`, `db`, `analysis`)
* [ ] Profilowanie: `tracing`, `tracing-subscriber`
* [ ] Benchmark skanera (syntetyczne eventy)

---

## 🖥️ 2. Interfejsy: Terminal + Web

**Terminal (TUI)**

* [ ] Widok „live devices” (MAC, RSSI, last_seen, count)
* [ ] Widok statystyk (pakiety/s, aktywne urządzenia, nowe/znikające)
* [ ] Filtrowanie (vendor, RSSI threshold, typ reklamy)
* [ ] Kolorowanie anomalii / alertów
* [ ] Tryb „headless” (tylko log + eksport)

**Web UI**

* [ ] Endpoint `/api/stream` (SSE / WebSocket)
* [ ] Dashboard: liczba urządzeń, PPS, top vendors
* [ ] Widok timeline (RSSI w czasie dla wybranego MAC)
* [ ] Filtrowanie i wyszukiwanie
* [ ] Eksport CSV / JSON

---

## 🗄️ 3. Przechowywanie danych

* [ ] SQLite / DuckDB backend
* [ ] Buforowanie w RAM + batch insert
* [ ] Retencja danych (np. TTL 24h / 7 dni)
* [ ] Migracje schematu
* [ ] Indeksy pod: (mac, timestamp), (vendor, timestamp)
* [ ] Tryb „record & replay” (odtwarzanie sesji)

---

## 🧠 4. Moduły analityczne (realna wartość projektu)

**P1 – Must have**

* [ ] Trend RSSI (zbliża się / oddala)
* [ ] Licznik nowych / znikających urządzeń (okno czasowe)
* [ ] Detekcja burstów reklam (flood / skanery / atak?)
* [ ] Histogram kanałów 37/38/39
* [ ] Alerty progowe (konfigurowalne)

**P2 – Advanced**

* [ ] Fingerprint pomieszczenia (MAC + avg RSSI + vendor)
* [ ] Entropia ruchu (zmienność w czasie)
* [ ] Wykrywanie rotacji MAC (privacy)
* [ ] Korelacja RSSI + AD structures (quasi-device tracking)
* [ ] Klasyfikacja typów urządzeń (beacon, phone, IoT, unknown)

---

## 🛡️ 5. Anomalie i bezpieczeństwo

* [ ] Nietypowe długości AD structures
* [ ] Nienormalna częstość reklam z jednego MAC
* [ ] „Ghost devices” (pojawiają się i znikają zbyt szybko)
* [ ] Wykrywanie urządzeń reklamujących tylko na 1 kanale
* [ ] Flagi podejrzanych ramek w UI
* [ ] System reguł (YAML/JSON): jeśli X i Y → alert

---

## ⚙️ 6. Wydajność i architektura

* [ ] Zero-copy tam gdzie się da (Arc<[u8]>, Bytes)
* [ ] Minimalizacja klonów struktur
* [ ] Rozdział: ingest → normalizacja → analiza → output
* [ ] Worker pool dla analiz
* [ ] Rate limiting na wejściu
* [ ] Test z symulatorem 10k urządzeń

---

## 🧪 7. Testy i jakość

* [ ] Testy jednostkowe parserów AD structures
* [ ] Testy pipeline (fake scanner → collector → analyzer)
* [ ] Testy regresji na zapisanych sesjach
* [ ] Fuzzing ramek reklam
* [ ] Clippy + fmt w CI
* [ ] Miri / sanitizers (jeśli możliwe)

---

## 🚀 8. DevOps / Projekt

* [ ] GitHub Actions: build (Linux/Windows/macOS)
* [ ] Release artifacts (binarki)
* [ ] Profil „minimal” vs „full”
* [ ] Dokumentacja architektury
* [ ] Przykładowe scenariusze użycia (security, retail, research)
* [ ] Roadmapa v1.0

---

## 🔮 9. R&D / Futurystyczne rozszerzenia

* [ ] BLE Mesh passive analysis (heartbeat, relay)
* [ ] Korelacja BLE + Wi-Fi
* [ ] Quasi-direction finding (trend + gradient RSSI)
* [ ] Plugin system na analizatory
* [ ] Reguły w stylu Suricata/Sigma dla BLE
* [ ] Eksport do Prometheus / Grafana

---

# 🗺️ Proponowana kolejność prac

1. **Stabilizacja core + pipeline danych**
2. **Obserwowalność (TUI + Web dashboard)**
3. **Persistence + replay**
4. **Podstawowa analityka + alerty**
5. **Anomalie + privacy**
6. **Optymalizacja i skala**
7. **R&D features**

---

# 🧠 Git-master reality check

W trakcie implementacji **największe źródła konfliktów** to:

* model danych (`SignalInfo` puchnie → trzeba wersjonować),
* concurrency (kto jest właścicielem danych, gdzie Arc, gdzie borrow),
* wydajność vs ergonomia API,
* różne potrzeby: UI vs analiza vs storage.

Dlatego:

* trzymaj **jedno źródło prawdy dla eventu BLE**,
* wprowadzaj **zmiany ewolucyjnie**,
* izoluj moduły traitami (`Analyzer`, `Sink`, `Source`),
* nie zgaduj API — projektuj je pod realne przepływy danych.

---

Jeśli chcesz, opisz mi **jak teraz wygląda architektura (moduły, kanały, struktury)**, a przerobię ten backlog na **konkretny plan refaktoru pod Twój kod** 💪

Na podstawie zadań i opisu projektu można przypuszczać następującą architekturę:

**Moduły**
1. **Scanner** – module odnajdujące urządzenia BLE, odbierające raw packets (używa `quanta` do precyzyjnego czasu). Pracuje przez interfejs async-trait.
2. **Parser** – przetwarza raw packets, wyciąga MAC, RSSI, AD structures oraz informacje z GATT.
3. **Pipeline** – centralny tok eventów (tokio mpsc channel), który przekazuje obiekty `SignalInfo` z parsera do storage i UI.
4. **Storage** – przechowuje dane w SQLite lub tymczasowo w RAM (ring-buffer). Obsługuje retencję, batch insert oraz indeksy.
5. **UI/TUI** – ratatui dla terminalu + HTTP server (np. Actix) dostarczający API i dashboard z WebSocket-ami.
6. **Analytics** – moduły analizujące dane z pipeline (trend RSSI, detekcja burstów, alerty).

**Kanały**
- `scan_events`: MPMC channel z parsera do storage/analytics.
- `ui_updates`: Channel dla aktualizacji UI (WebSocket/SSE).
- `db_pool`: Connection pool do SQLite.

**Struktury danych**
- `SignalInfo` – kluczowa struktura: `mac`, `rssi`, `timestamp`, `ad_data`, `adapter_id`, `source`.
- `DeviceProfile` – dla analizy (vendor, typ urządzenia).
- `AlertConfig` – konfiguracja progowych alerty.

---

### Konkretny plan refaktoru (przygotowany na podstawie backlogu)

#### 1. Przygotowanie środowiska
- Upewnij się, że wszystkie zależności są zainstalowane: `async-trait`, `quanta`, `libc`.
- Zrób aktualizację `cargo check` – naprawienie 332 warningów (przez usunięcie niepotrzebnych importów, dodanie brakujących typów i optymalizacje).

#### 2. Stabilizacja core + pipeline danych
* **Krok 1: Zjednoczenie modelu `SignalInfo`**  
  Dodaj pole `adapter_id` (identyfikator urządzenia BLE) i `source` (źródło – np. "bluez", "raw"). Ułóż jego format jako `thiserror::Result`.
* **Krok 2: Centralny pipeline zdarzeń**  
  Zastąp lokalne kanały przez `tokio::sync::mpsc::UnboundedSender` w module core. Wszystkie eventy powinny przechodzić przez ten sam channel.
* **Krok 3: Standardyzacja błędów**  
  Wprowadź typy `Result<_, AppError>` z `anyhow` (lub `thiserror`). Każde operacje muszą zwracać odpowiedni kod błędu.
* **Krok 4: Logging i shutdown**  
  Użyj `tracing` + `tracing-subscriber`. Dodaj obsługa SIGTERM i CTRL+C, która bezpiecznie zatrzyma scanner i odblockuje kanały.

#### 3. Interfejsy: TUI i Web
* **TUI (ratatui)**  
  - Stwórz widok `LiveDevices` z listą aktywnych MAC, RSSI i ostatniej aktywności.
  - Dodaj filtry (RSSI threshold, typ reklamy) i kolorowanie anomalii.
  - Zaimplementuj tryb headless – wyłącznie log + eksport.
* **Web API**  
  - Stwórz endpoint `/api/stream` z WebSocket (lub SSE). Wysyłaj eventy `SignalInfo`.
  - Dodać basic auth dla API (token w nagłówku).
  - Przygotuj template dashboardu: liczba urządzeń, PPS, top vendors.

#### 4. Persistencja i retencja
* **SQLite**  
  - Zainstaluj `sqlite` i stwórz tabelę `signals` z indeksami na `(mac, timestamp)`.
  - Wprowadź ring-buffer w RAM (np. using `std::collections::VecDeque`) do redukcji writes.
  - Automatyczna retencja: usuń rekordy starsze niż 24h (użyj TTL).
* **Migracja**  
  - Stwórz skrypt migracji dla nowego schematu (np. dodanie pola `adapter_id`).

#### 5. Podstawowa analityka
* **Trend RSSI**  
  - W module analytics zapisz ostatnie 100 eventów dla każdej MAC w RAM.
  - Oblicz średnią i zmianę w ciągu minuty (przykład: `trend = (nowy_rssi - poprzedni_avg)/poprzedni_avg`).
* **Licznik nowych/znikających**  
  - Użyj okna czasowego (np. 5 minut) – gdy MAC pojawia się, zwiększ licznik "new", jeśli od dłużego czasu nie pojawił się, zwiększ "lost".
* **Detekcja burstów reklam**  
  - W pipeline'u dodaj limiter: maksymalna liczba eventów z jednego MAC w oknie 1 sekundy. Jeśli przekroczony – zgłoś jako "flood".

#### 6. Anomalie i alerty
* **System reguł**  
  - Zapisz konfigurację alerty w YAML/JSON (np. `alerts.yaml`).
  - Przykład: `"if rssi < -80 and count > 10 then alert type=low_battery"` – implementacja jako trait.
* **Rotacja MAC**  
  - Oblicz średnią częstotliwość zmian adresu w ciągu godziny. Jeśli przekracza progi – zaznacz jako "privacy".

#### 7. Optymalizacja i testy
* **Zero-copy**  
  - Zastąp `.to_owned()` przez `Arc<[u8]>` dla AD data.
* **Benchmarking**  
  - Użyj `criterion` do mierzenia PPS przy 10k urządzeń (symulacja).
* **Testy**  
  - Dodaj testy jednostkowe parsera AD structures (np. przy użyciu `proptest`).
  - Testy pipeline: generuj fikcyjne eventy, sprawdź czy alerty działają.

#### 8. Dokumentacja i release
* Zapisz roadmapę w `ROADMAP.md`.
* Otwórz GitHub Actions dla buildów (Linux/Windows/macOS) i release'ów.
* Udostępnij przykładowe scenariusze użycia: security, retail.

---

**Dlaczego to pasuje?**  
Plan ten dokładnie odpowiada kolejności z sekcji **Proponowana kolejność prac**. Zawiera wszystkie zadania z backlogu (np. "Auto-pause skanu przy braku reklam" jest częścią analizy burstów). Ważne: każdy krok jest izolowany i nie wymaga zmian w innych modułach, co zapewnia stabilność.

Powinieneś najpierw wykonać **Fazę 1 (Stabilizacja core)** – bez niej dalsze refaktoryzacje będą trudne. Gdy to zrobisz, możesz powoli wchodzić do nowych funkcji. Chcesz, abyśmy precyzyjniej opisali który conkretnie moduł implementować (np. kod dla pipeline'u)?