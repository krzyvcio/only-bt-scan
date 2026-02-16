# AGENTS.md - Instructions for AI Agents

This file provides guidelines for AI coding agents working on the **only-bt-scan** project - a Rust BLE/Bluetooth scanner application.

---

## 🏗️ BUILD, LINT & TEST COMMANDS

### Build
```bash
# Debug build (fast)
cargo build

# Release build (optimized)
cargo build --release
```

### Linting & Code Quality
```bash
# Check without building (fast)
cargo check

# Run clippy lints
cargo clippy

# Format code
cargo fmt

# Fix automatically
cargo fix --lib -p only-bt-scan
```

### Testing
```bash
# Run all tests
cargo test

# Run single test by name
cargo test test_name

# Run tests in specific module
cargo test module_name

# Run with output
cargo test -- --nocapture
```

### Database
```bash
# DB is in: bluetooth_scan.db (SQLite)
# WAL mode enabled for concurrency
```

---

## 🎨 CODE STYLE GUIDELINES

### General Principles
- **Be concise** - Short responses, answer directly without preamble
- **Don't over-explain** - No unnecessary comments or summaries
- **Follow existing patterns** - Match the codebase style, don't invent new conventions

### Naming Conventions
```rust
// Modules: snake_case
mod bluetooth_scanner;
mod device_tracker;

// Structs/Enums: PascalCase
pub struct BluetoothDevice;
pub enum ScanStatus;

// Functions: snake_case
pub fn get_device_detail()
fn parse_advertising_packet()

// Constants: SCREAMING_SNAKE_CASE
const MAX_RAW_PACKETS: usize = 500;
const DEFAULT_PAGE_SIZE: usize = 50;
```

### Imports
```rust
// Order: std → external → crate
use std::collections::HashMap;
use rusqlite::{Connection, params};
use crate::bluetooth_scanner::BluetoothDevice;

// Group by blank line
use actix_web::{web, App, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
```

### Error Handling
```rust
// ✅ PREFERRED: Use Result<T, E> with meaningful errors
pub fn get_device(mac: &str) -> Result<Option<Device>, rusqlite::Error> {
    // ...
}

// ✅ ACCEPTABLE: Use ? operator with map/filter
let device = conn.query_row(...)?.optional()?;

// ❌ AVOID: unwrap() and expect() on critical paths
// Only use unwrap() in tests or when failure is truly impossible
let value = parsed_data.unwrap_or_default();

// ✅ GOOD: Log errors and provide fallbacks
match result {
    Ok(data) => data,
    Err(e) => {
        log::warn!("Failed to parse: {}", e);
        default_value()
    }
}
```

### Ownership & Borrowing
```rust
// ✅ Use references when not taking ownership
fn process_device(device: &BluetoothDevice) { }

// ✅ Use Arc<Mutex<T>> for shared state
pub struct DeviceTrackerManager {
    devices: Arc<Mutex<HashMap<String, DeviceTracker>>>,
}

// ✅ Clone only when necessary
// Bad: devices.get(mac).unwrap().clone() in hot path
// Good: Return reference or use Entry API
```

### Async/Await
```rust
// ✅ Always use tokio for async
use tokio::time::{sleep, timeout};

// ✅ Add timeouts for external operations
match timeout(Duration::from_secs(5), async_operation()).await {
    Ok(result) => result,
    Err(_) => {
        log::error!("Operation timed out");
        None
    }
}
```

---

## 📋 PROJECT STRUCTURE

```
only-bt-scan/
├── src/
│   ├── lib.rs           # Main library entry, app initialization
│   ├── main.rs          # Binary entry point
│   ├── db.rs            # Database operations (use *_pooled functions)
│   ├── db_pool.rs       # Connection pool (NEW - use this!)
│   ├── db_frames.rs     # Frame storage operations
│   ├── web_server.rs    # Actix-web API endpoints
│   ├── bluetooth_scanner.rs  # BLE scanning via btleplug
│   ├── device_tracker.rs      # Device tracking with limits
│   ├── advertising_parser.rs  # AD structure parsing
│   ├── packet_tracker.rs     # Packet ordering/dedup
│   └── ... (50+ modules)
├── Cargo.toml
└── MEMORY.md            # Project-specific notes & TODOs
```

---

## ⚠️ CRITICAL PATTERNS (LEARN FROM PAST)

### Database Pool (MANDATORY for new code)
```rust
// ✅ Use pool functions when available
use crate::db_pool::get_pool;

if let Some(pool) = get_pool() {
    pool.execute(|conn| {
        // operations on shared connection
    })
} else {
    // fallback to old function
    db::get_device(mac)
}
```

### MAC Address Validation (REQUIRED for API)
```rust
// ✅ Always validate MAC in API endpoints
pub async fn get_device_detail(path: web::Path<String>) -> impl Responder {
    let raw_mac = path.into_inner();
    let mac = match validate_mac_address(&raw_mac) {
        Ok(m) => m,
        Err(e) => return HttpResponse::BadRequest().json({"error": e}),
    };
    // ...
}
```

### Device Tracker Limits
```rust
// ✅ Always use with_limit() for long-running scans
let manager = DeviceTrackerManager::with_limit(10000);

// Default limit is 10000, oldest devices evicted when full
```

---

## 🔧 COMMON TASKS

### Adding a new database function
1. Create inner function with `conn: &Connection` parameter
2. Create public wrapper that uses pool if available
3. Add fallback to old function if pool not initialized

### Adding API endpoint
1. Validate all inputs (especially MAC addresses)
2. Use existing DB query patterns
3. Add proper error responses (not just unwrap)
4. Use pagination for lists

### Running BLE scanner
```bash
# Requires Bluetooth adapter
# On Windows: uses btleplug or native Windows API
# On Linux: uses BlueZ via btleplug
# On macOS: uses CoreBluetooth via btleplug
```

---

## 📝 NOTES FROM PREVIOUS AGENT

The previous session fixed these critical issues:
1. Created `db_pool.rs` - Use this for new DB operations
2. Added MAC validation in `web_server.rs`
3. Fixed memory leak in `device_tracker.rs` (10k device limit)
4. Fixed race condition in `record_detection()`
5. Fixed N+1 query with batch `get_parsed_ad_data_batch()`

**Check MEMORY.md for detailed todo list and progress.**

---

## 🆘 TROUBLESHOOTING

| Problem | Solution |
|---------|----------|
| `SQLite locked` | Use pool connection instead of `Connection::open()` |
| `No devices found` | Check Bluetooth adapter: `bluetoothctl list` |
| `Port 8080 in use` | Kill existing process or change port |
| `unwrap() panic` | Replace with proper error handling |
| Build warnings | Run `cargo fix --lib -p only-bt-scan` |

---

*Last updated: 2026-02-16*
*For AI agent use - based on Rust best practices and project-specific patterns*
