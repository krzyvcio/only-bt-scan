# TASKS.md - only-bt-scan

## Aktualne zadania

### Priorytet wysoki

- [ ] Naprawić 332 warnings z `cargo check`
- [x] Zaimplementować passive scanning z RSSI i raw packets
- [ ] Dodać eksport do PCAP/Wireshark

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
- [ ] Auto-pause skanu przy braku reklam przez N minut
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
