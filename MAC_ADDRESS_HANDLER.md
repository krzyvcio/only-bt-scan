# MAC Address Handler - Bluetooth MAC Address Parsing and Filtering

## Overview

Moduł `mac_address_handler` zapewnia zaawansowaną obsługę MAC addressów Bluetooth z filtrowaniem, walidacją i dekodowaniem typów adresów.

## Moduł

### mac_address_handler.rs (300+ linii)

**Główne struktury:**
- `MacAddress` - Reprezentacja MAC addressu
- `MacAddressFilter` - Filtrowanie adresów
- `MacAddressStats` - Statystyki adresów

## API Endpoint

### GET /api/mac/{mac_address}

Zwraca szczegółową informację o MAC addressie.

**Request:**
```
GET /api/mac/AA:BB:CC:DD:EE:FF
```

**Response:**
```json
{
  "mac_address": "AA:BB:CC:DD:EE:FF",
  "is_unicast": true,
  "is_multicast": false,
  "is_locally_administered": false,
  "is_universally_administered": true,
  "is_rpa": false,
  "is_static_random": false,
  "is_nrpa": false,
  "manufacturer_id": "AA:BB:CC",
  "device_id": "DD:EE:FF",
  "address_type": "Public"
}
```

## Obsługiwane Formaty

### Input Formats
- `AA:BB:CC:DD:EE:FF` (with colons)
- `AABBCCDDEEFF` (without separators)
- `AA-BB-CC-DD-EE-FF` (with dashes)

### Output Format
- `AA:BB:CC:DD:EE:FF` (standardowy)

## Typy Adresów BLE

### Public Address (GAP)
- Assigned by IEEE
- First octet LSB = 0 (Universally Administered)
- Globally unique
- Used for classic Bluetooth

### Random Address
- Locally generated
- First octet LSB = 1 (Locally Administered)
- Two subtypes: Static Random, Private

#### Static Random Address
- First octet: 0xC0-0xFF (top 2 bits = 11)
- Fixed for device lifetime
- Set during boot
- Must pass address check

#### Private Address
Two variants:

**Resolvable Private Address (RPA)**
- First octet: 0x40-0x7F (top 2 bits = 01)
- Changes periodically (15-60 minutes)
- Can be resolved with IRK
- Preferred for privacy

**Non-Resolvable Private Address (NRPA)**
- First octet: 0x00-0x3F (top 2 bits = 00)
- Cannot be resolved
- Used when no IRK available
- Rare in modern BLE

## Funkcjonalności

### MacAddress Methods

```rust
// Parsing
MacAddress::from_string("AA:BB:CC:DD:EE:FF")?
MacAddress::from_bytes(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF])

// Type Detection
is_unicast()            // Single recipient
is_multicast()          // Multiple recipients
is_locally_administered()
is_universally_administered()
is_rpa()                // Resolvable Private Address
is_static_random()      // Static Random Address
is_nrpa()               // Non-Resolvable Private Address
is_randomly_generated()

// Components
manufacturer_id()       // First 3 octets (OUI)
device_id()             // Last 3 octets

// Pattern Matching
matches_pattern("AA:BB:CC:*:*:*")
```

### MacAddressFilter Methods

```rust
// Whitelist/Blacklist
add_whitelist("AA:BB:CC:DD:EE:FF")?
add_blacklist("AA:BB:CC:DD:EE:FF")?

// Pattern Filters
add_pattern("AA:BB:*:*:*:*")?

// Type Filtering
set_allow_rpa(true)
set_allow_static_random(true)
set_allow_public(true)

// Matching
matches(&mac)  // Returns true if address passes all filters
```

## Przykłady

### Detect Address Type

```rust
use crate::mac_address_handler::MacAddress;

let mac = MacAddress::from_string("4A:BB:CC:DD:EE:FF")?;

if mac.is_rpa() {
    println!("RPA (Resolvable Private Address)");
} else if mac.is_static_random() {
    println!("Static Random Address");
} else {
    println!("Public Address");
}
```

### Filter Devices

```rust
let mut filter = MacAddressFilter::new();

// Only Apple devices (first 3 octets: 4C:A0:25)
filter.add_pattern("4C:A0:25:*:*:*")?;

// Exclude specific device
filter.add_blacklist("4C:A0:25:11:22:33")?;

// Only static random addresses
filter.set_allow_public(false);
filter.set_allow_rpa(true);
filter.set_allow_static_random(true);

if filter.matches(&mac) {
    println!("Device matches filter criteria");
}
```

### Extract IDs

```rust
let mac = MacAddress::from_string("AA:BB:CC:DD:EE:FF")?;

let manufacturer_id = mac.manufacturer_id();  // [0xAA, 0xBB, 0xCC]
let device_id = mac.device_id();             // [0xDD, 0xEE, 0xFF]
```

## Filter Operations

### Whitelist Only
```rust
let mut filter = MacAddressFilter::new();
filter.add_whitelist("AA:BB:CC:DD:EE:FF")?;
filter.add_whitelist("11:22:33:44:55:66")?;
// Only these two devices pass filter
```

### Pattern Matching
```rust
// Apple devices
filter.add_pattern("4C:A0:25:*:*:*")?;

// Google Pixel devices
filter.add_pattern("00:1A:7D:DA:71:13")?;

// Any first 3 octets
filter.add_pattern("*:*:*:DD:EE:FF")?;
```

### Type-Based Filtering
```rust
// Only RPAs (privacy-preserving)
filter.set_allow_public(false);
filter.set_allow_static_random(false);
filter.set_allow_rpa(true);

// Only public addresses
filter.set_allow_public(true);
filter.set_allow_static_random(false);
filter.set_allow_rpa(false);
```

## MAC Address Components

### First Octet (Byte 0)

| Bit | Name | Meaning |
|-----|------|---------|
| 0 | U/L | Universal (0) or Local (1) |
| 1 | I/G | Individual (0) or Group (1) |
| 7-2 | OUI | Organization ID |

### Address Type Decision Tree

```
First Octet & 0xC0:
  0x00 -> Possibly NRPA (if random)
  0x40 -> RPA (Resolvable Private)
  0x80 -> Possibly NRPA (if random)
  0xC0 -> Static Random Address

First Octet & 0x01:
  0x00 -> Unicast
  0x01 -> Multicast/Broadcast
```

## Statistics

`MacAddressStats` zawiera:
- `total_addresses` - Liczba wszystkich adresów
- `unique_addresses` - Liczba unikalnych adresów
- `rpa_count` - RPAs
- `static_random_count` - Static random
- `public_count` - Public addresses
- `multicast_count` - Broadcast/multicast

## Testing

```bash
cargo test mac_address_handler
```

Includes tests for:
- Parsing (with/without colons)
- RPA detection
- Static random detection
- Filtering
- Pattern matching
- Unicast/multicast
- Locally/universally administered

## Dependency

```toml
mac_conditions = "1.0"  # Platform-specific MAC handling
```

## Integration with Scanning

```rust
// Filter scan results
let mut filter = MacAddressFilter::new();
filter.add_blacklist("AA:BB:CC:DD:EE:FF")?;

for device in scan_results {
    if let Ok(mac) = MacAddress::from_string(&device.mac) {
        if filter.matches(&mac) {
            println!("Processing: {}", device.name);
        }
    }
}
```

## Privacy Implications

### RPA (Resolvable Private Address)
- Privacy: GOOD - Changes periodically
- Tracking: Possible only with IRK
- Recommended for: User devices

### Static Random
- Privacy: MEDIUM - Fixed but random
- Tracking: Possible
- Recommended for: Peripheral devices

### Public Address
- Privacy: POOR - Globally unique
- Tracking: Easy
- Note: Used for official services

## Performance

- Parsing: < 1 microsecond
- Type detection: < 1 microsecond
- Pattern matching: < 10 microseconds
- Filter matching: < 50 microseconds

## References

- Bluetooth Core Spec Vol 6, Part B (Link Layer)
- BLE Random Address Format
- IEEE 802 MAC Address Standards
