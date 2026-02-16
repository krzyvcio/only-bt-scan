# Bluetooth Scanner - only-bt-scan

A Rust-based BLE/Bluetooth Low Energy scanner application for Windows with Telegram notifications.

## Installation

1. Clone the repository
2. Install Rust from https://rustup.rs/
3. Configure your `.env` file with Telegram credentials (optional)

## Running the Application

### Quick Start

```powershell
cd C:/projekty/only-bt-scan
.\target\debug\only-bt-scan.exe
```

### Important: Clear Environment Variables Before Running

PowerShell may cache environment variables from previous sessions. To ensure the application reads fresh configuration from `.env`, clear cached environment variables first:

```powershell
Remove-Item env:TELEGRAM_BOT_TOKEN -ErrorAction SilentlyContinue
Remove-Item env:TELEGRAM_CHAT_ID -ErrorAction SilentlyContinue  
Remove-Item env:RUST_LOG -ErrorAction SilentlyContinue
cd C:/projekty/only-bt-scan
.\target\debug\only-bt-scan.exe
```

This ensures that:
- Telegram bot token is loaded from `.env` (not cached from previous sessions)
- Chat ID configuration is fresh
- Logging level is properly configured

## Configuration

Edit `.env` file to configure:

```ini
# Logging level: TRACE, DEBUG, INFO, WARN, ERROR
RUST_LOG=debug

# Database path (relative to project root)
DB_PATH=bluetooth_scan.db

# Scan duration in seconds
SCAN_DURATION=1

# Number of scan cycles (default: 3)
SCAN_CYCLES=3

# Interval between scans in minutes (default: 5)
SCAN_INTERVAL_MINUTES=5

# Telegram Bot Token (optional)
TELEGRAM_BOT_TOKEN=your_bot_token_here

# Telegram Chat ID (optional)
TELEGRAM_CHAT_ID=your_chat_id_here

# Web server port (optional)
WEB_SERVER_PORT=8080

# Clean up old records older than (days)
CLEANUP_DAYS=30
```

## Building

### Debug Build (faster compilation)
```bash
cargo build
```

### Release Build (optimized)
```bash
cargo build --release
```

Then run:
```powershell
.\target\release\only-bt-scan.exe
```

## Features

- ✅ Real-time BLE device scanning
- ✅ Telegram notifications (startup, periodic reports)
- ✅ Web dashboard at `http://localhost:8080`
- ✅ SQLite database storage
- ✅ System tray support
- ✅ Device tracking and history

## Troubleshooting

### Telegram not working?
1. Verify `TELEGRAM_BOT_TOKEN` and `TELEGRAM_CHAT_ID` in `.env`
2. Clear PowerShell environment variables as shown above
3. Ensure you have internet connection
4. Check bot token validity with BotFather on Telegram

### HCI Sniffer requires admin privileges
Run as Administrator if you want advanced packet capture features.

## Development

### Run tests
```bash
cargo test
```

### Format code
```bash
cargo fmt
```

### Run clippy linter
```bash
cargo clippy
```
