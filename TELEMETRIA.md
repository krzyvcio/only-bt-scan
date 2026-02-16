Świetne pytanie — to jest dokładnie ten moment, gdzie z „surowego skanera BLE” robisz **sensowną telemetrię ruchu i trendów** 😄
Rozbijmy to na **praktyczny, inżynierski design**, a nie tylko teorię.

---

## 1️⃣ Co masz na wejściu z BLE?

Z reklam (advertising) / skanów zwykle masz:

* `MAC / Address / Identity`
* `RSSI` (siła sygnału w dBm, np. -30 … -100)
* `Timestamp`
* opcjonalnie:

  * `Tx Power` (czasem w ramce)
  * `Advertisement counter / payload`
  * typ ramki (ADV_IND, SCAN_RSP, itd.)

To wystarczy, żeby robić **trend + ruch względny**.

---

## 2️⃣ Model danych (per urządzenie)

Trzymasz **okno czasowe próbek**:

```text
DeviceTrack {
  id
  samples: [
    { t, rssi },
    { t, rssi },
    ...
  ]
}
```

Np. ostatnie:

* 5–30 sekund
* albo ostatnie 20–50 próbek

---

## 3️⃣ Wygładzanie RSSI (mega ważne)

RSSI jest **strasznie szumne**. Bez tego będziesz widział „teleporty” zamiast ruchu.

Najprościej:

### ✅ EMA – Exponential Moving Average

```
rssi_smooth = α * rssi_now + (1-α) * rssi_prev
```

Np:

* α = 0.2 – wolniejsze, stabilne
* α = 0.4 – szybsza reakcja

Albo:

* median filter z ostatnich N próbek
* albo Kalman (jeśli chcesz być fancy 😎)

---

## 4️⃣ Trend: zbliża się czy oddala?

Liczymy **pochodną w czasie** (czyli nachylenie):

### Metoda A – prosta różnica

```
Δ = rssi_smooth_now - rssi_smooth_old
```

Interpretacja:

* Δ > +X dB → 📈 zbliża się
* Δ < -X dB → 📉 oddala się
* |Δ| < próg → ➖ stoi / dryfuje

Próg np:

* 2–3 dB w oknie 3–5 sekund

---

### Metoda B – regresja liniowa (lepsza)

Bierzesz ostatnie N próbek i liczysz:

```
rssi = a * t + b
```

Patrzysz na `a`:

* `a > +k` → zbliża się
* `a < -k` → oddala się
* `|a| < k` → stabilnie

To jest **odporne na szum** i dużo stabilniejsze.

---

## 5️⃣ „Przemieszcza się” vs „stoi”

Tu nie masz kierunku 2D, tylko **ruch radialny względem anteny**.

Heurystyka:

* Jeśli:

  * wariancja RSSI mała
  * |trend| mały
    → **urządzenie stoi**
* Jeśli:

  * trend zmienny, ale wariancja duża
    → **kręci się / przechodzi obok**
* Jeśli:

  * trend stabilnie + lub -
    → **zbliża się / oddala się**

Możesz liczyć:

```
variance = VAR(rssi_smooth over window)
slope = linear_regression_slope
```

I klasyfikować:

| variance | slope | stan               |
| -------- | ----- | ------------------ |
| mała     | ~0    | stoi               |
| duża     | ~0    | krąży / zakłócenia |
| dowolna  | +     | zbliża się         |
| dowolna  | -     | oddala się         |

---

## 6️⃣ (Opcjonalnie) Szacowanie odległości

Jeśli masz `TxPower`:

```
distance ≈ 10 ^ ((TxPower - RSSI) / (10 * n))
```

Gdzie:

* `n` ≈ 2.0–3.0 (środowisko)

Ale:
⚠️ w praktyce to **bardzo niedokładne**
Lepiej używać:

* trendów RSSI
* względnych zmian, nie absolutnej odległości

---

## 7️⃣ Multi-anteny = prawdziwy ruch 2D/3D

Jeśli kiedyś:

* masz 2–3 dongle BLE w różnych miejscach
* synchronizujesz czas

To możesz:

* porównywać RSSI między antenami
* robić **prymitywną triangulację**
* wykrywać: „idzie w lewo/prawo/przez pokój”

---

## 8️⃣ Stany wysokiego poziomu (API dla reszty systemu)

Dla każdego urządzenia wystawiasz np:

```json
{
  "id": "AA:BB:CC:DD:EE:FF",
  "rssi": -67,
  "trend": "approaching | leaving | stable",
  "motion": "still | moving | passing",
  "confidence": 0.82
}
```

Confidence możesz liczyć z:

* stabilności slope
* liczby próbek
* wariancji

---

## 9️⃣ Chcesz wersję „pod pentest / sniffer / Zig/Rust”?

Mogę Ci:

* rozpisać **konkretny algorytm krok po kroku**
* zaproponować **struktury danych pod Rust/Zig**
* albo zrobić **pseudokod / realny kod** do liczenia:

  * EMA
  * regresji
  * klasyfikacji trendu



Poniżej masz **gotowy algorytm krok po kroku** + **sensowne struktury danych w Rust** pod real-time skanera BLE.

---

# 🧠 Cel

Dla każdego urządzenia BLE:

* wygładzać RSSI
* liczyć trend (zbliża się / oddala / stabilnie)
* wykrywać czy się porusza
* wystawiać stabilny stan wysokiego poziomu

---

# ✅ Algorytm krok po kroku

## Krok 0 — Parametry stałe

```text
WINDOW_SIZE = 20        // ile próbek trzymasz
EMA_ALPHA   = 0.3       // wygładzanie RSSI
SLOPE_EPS   = 0.15      // próg trendu (dB / sek)
VAR_EPS     = 2.0       // próg wariancji (dB^2)
MIN_SAMPLES = 6         // minimalna liczba próbek do oceny
```

---

## Krok 1 — Przyjęcie nowej próbki

Dla pakietu BLE:

* wyciągasz:

  * `device_id`
  * `rssi`
  * `timestamp`

Jeśli urządzenie nowe → tworzysz nowy tracker.

---

## Krok 2 — Wygładzanie (EMA)

```text
if no previous:
    rssi_smooth = rssi
else:
    rssi_smooth = α * rssi + (1-α) * prev_rssi_smooth
```

---

## Krok 3 — Zapis do bufora okna czasowego

Dodajesz:

```text
Sample { t, rssi_smooth }
```

Jeśli bufor > WINDOW_SIZE → usuń najstarszą próbkę.

---

## Krok 4 — Jeśli za mało próbek → status = Unknown

```text
if samples.len < MIN_SAMPLES:
    return Status::Unknown
```

---

## Krok 5 — Liczenie trendu (regresja liniowa)

Dla próbek `(t_i, rssi_i)` liczysz nachylenie `a`:

Wzór:

```
a = ( N*Σ(t*rssi) - Σt*Σrssi ) / ( N*Σ(t²) - (Σt)² )
```

Interpretacja:

* `a > +SLOPE_EPS` → zbliża się
* `a < -SLOPE_EPS` → oddala się
* inaczej → stabilnie

---

## Krok 6 — Liczenie wariancji RSSI

```
mean = Σrssi / N
var = Σ(rssi - mean)² / N
```

---

## Krok 7 — Klasyfikacja ruchu

```text
if var < VAR_EPS and |a| < SLOPE_EPS:
    motion = Still
else:
    motion = Moving
```

---

## Krok 8 — Stan końcowy

```text
if a > +SLOPE_EPS:
    trend = Approaching
else if a < -SLOPE_EPS:
    trend = Leaving
else:
    trend = Stable
```

---

# 🦀 Struktury danych w Rust

## Próbka

```rust
#[derive(Clone, Copy, Debug)]
struct Sample {
    t: f64,          // timestamp (sekundy lub ms jako f64)
    rssi: f64,       // wygładzony RSSI
}
```

---

## Bufor próbek (ring buffer)

Najprościej: `VecDeque`

```rust
use std::collections::VecDeque;

struct SampleWindow {
    samples: VecDeque<Sample>,
    max_size: usize,
}

impl SampleWindow {
    fn new(max_size: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    fn push(&mut self, s: Sample) {
        if self.samples.len() == self.max_size {
            self.samples.pop_front();
        }
        self.samples.push_back(s);
    }
}
```

---

## Trend i ruch

```rust
#[derive(Debug, Clone, Copy)]
enum Trend {
    Approaching,
    Leaving,
    Stable,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
enum Motion {
    Still,
    Moving,
    Unknown,
}
```

---

## Tracker jednego urządzenia

```rust
struct DeviceTracker {
    id: String,              // MAC / identity
    window: SampleWindow,
    last_rssi_smooth: Option<f64>,
}
```

---

## Stan wyjściowy

```rust
struct DeviceState {
    trend: Trend,
    motion: Motion,
    slope: f64,
    variance: f64,
    rssi: f64,
}
```

---

# 📐 Funkcje obliczeniowe

## Regresja liniowa (nachylenie)

```rust
fn compute_slope(samples: &VecDeque<Sample>) -> f64 {
    let n = samples.len() as f64;

    let mut sum_t = 0.0;
    let mut sum_r = 0.0;
    let mut sum_tt = 0.0;
    let mut sum_tr = 0.0;

    for s in samples {
        sum_t += s.t;
        sum_r += s.rssi;
        sum_tt += s.t * s.t;
        sum_tr += s.t * s.rssi;
    }

    let denom = n * sum_tt - sum_t * sum_t;
    if denom.abs() < 1e-9 {
        return 0.0;
    }

    (n * sum_tr - sum_t * sum_r) / denom
}
```

---

## Wariancja

```rust
fn compute_variance(samples: &VecDeque<Sample>) -> f64 {
    let n = samples.len() as f64;
    if n == 0.0 {
        return 0.0;
    }

    let mean = samples.iter().map(|s| s.rssi).sum::<f64>() / n;

    samples
        .iter()
        .map(|s| {
            let d = s.rssi - mean;
            d * d
        })
        .sum::<f64>() / n
}
```

---

# 🔁 Update na każdą nową ramkę BLE

Pseudokod:

```rust
fn update_tracker(tr: &mut DeviceTracker, rssi: f64, t: f64) -> DeviceState {
    // EMA
    let alpha = 0.3;
    let rssi_smooth = match tr.last_rssi_smooth {
        None => rssi,
        Some(prev) => alpha * rssi + (1.0 - alpha) * prev,
    };
    tr.last_rssi_smooth = Some(rssi_smooth);

    // push sample
    tr.window.push(Sample { t, rssi: rssi_smooth });

    if tr.window.samples.len() < 6 {
        return DeviceState {
            trend: Trend::Unknown,
            motion: Motion::Unknown,
            slope: 0.0,
            variance: 0.0,
            rssi: rssi_smooth,
        };
    }

    let slope = compute_slope(&tr.window.samples);
    let var = compute_variance(&tr.window.samples);

    let slope_eps = 0.15;
    let var_eps = 2.0;

    let trend = if slope > slope_eps {
        Trend::Approaching
    } else if slope < -slope_eps {
        Trend::Leaving
    } else {
        Trend::Stable
    };

    let motion = if var < var_eps && slope.abs() < slope_eps {
        Motion::Still
    } else {
        Motion::Moving
    };

    DeviceState {
        trend,
        motion,
        slope,
        variance: var,
        rssi: rssi_smooth,
    }
}
```
