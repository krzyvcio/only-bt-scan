# Troubleshooting Guide: only-bt-scan

This guide provides solutions to common problems encountered while using the `only-bt-scan` application.

## Table of Contents

1.  [General Issues](#general-issues)
    *   [Compilation Errors](#compilation-errors)
    *   [Application Fails to Start](#application-fails-to-start)
2.  [Bluetooth Scanning Issues](#bluetooth-scanning-issues)
    *   [No Devices Detected](#no-devices-detected)
    *   [Incorrect Device Information](#incorrect-device-information)
    *   [Permission Denied Errors](#permission-denied-errors)
3.  [Raw Packet Processing](#raw-packet-processing)
    *   [Packet Parsing Errors](#packet-parsing-errors)
    *   [Incomplete Packet Data](#incomplete-packet-data)
4.  [Platform-Specific Problems](#platform-specific-problems)
    *   [Windows](#windows)
    *   [macOS](#macos)
    *   [Linux](#linux)
5.  [Frontend & UI](#frontend--ui)
    *   [Web Interface Not Loading](#web-interface-not-loading)
    *   [Data Not Updating in Real-time](#data-not-updating-in-real-time)

---

## General Issues

### Compilation Errors

*   **Problem**: The project fails to compile with `cargo build`.
*   **Solution**:
    1.  Ensure you have the latest stable version of Rust installed: `rustup update stable`.
    2.  Check if all dependencies in `Cargo.toml` are correctly specified.
    3.  Delete the `target` directory and `Cargo.lock` file, then try building again: `cargo clean && cargo build`.
    4.  Verify that any platform-specific dependencies (like `winrt-rust` for Windows) are correctly configured.

### Application Fails to Start

*   **Problem**: The application compiles but exits immediately or throws an error on launch.
*   **Solution**:
    1.  Check the application logs for any error messages. If logging is not enabled, consider adding it.
    2.  Ensure that no other process is using the same ports or resources (e.g., another Bluetooth scanner).
    3.  Verify that the required `.env` file is present and correctly configured if the application expects it.

---

## Bluetooth Scanning Issues

### No Devices Detected

*   **Problem**: The scanner runs but does not find any Bluetooth devices.
*   **Solution**:
    1.  **Check Bluetooth Adapter**: Make sure your computer's Bluetooth is turned on and functional.
    2.  **Driver Issues**: Ensure you have the correct Bluetooth drivers installed for your operating system. For Windows, this might mean installing drivers from the manufacturer's website.
    3.  **Permissions**: The application may not have the necessary permissions to access the Bluetooth adapter. See [Permission Denied Errors](#permission-denied-errors).
    4.  **Hardware Support**: Verify that your Bluetooth adapter supports the required features (e.g., LE scanning, raw packet access). Refer to `HCI_SUPPORT.md` for more details.

### Incorrect Device Information

*   **Problem**: Devices are detected, but their names, services, or other information are incorrect or missing.
*   **Solution**:
    1.  This could be an issue with the advertising packet parser. Refer to `advertising_parser.rs` to see how packets are decoded.
    2.  Some devices may use proprietary protocols. See `vendor_protocols.rs` for handling of specific vendor data.
    3.  Report the issue with the raw packet data of the problematic device if possible.

### Permission Denied Errors

*   **Problem**: The application throws an error related to permissions or access rights.
*   **Solution**:
    *   **Linux**:
        *   Run the application with `sudo`.
        *   Alternatively, grant the executable the required capabilities: `sudo setcap cap_net_raw,cap_net_admin+eip /path/to/your/executable`.
    *   **Windows**:
        *   Ensure the application is run as an Administrator.
        *   Check if any security software or group policy is blocking access to Bluetooth hardware.
    *   **macOS**:
        *   Go to `System Settings > Privacy & Security > Bluetooth` and ensure your application or terminal is allowed to use Bluetooth.

---

## Raw Packet Processing

### Packet Parsing Errors

*   **Problem**: The application logs errors related to parsing raw HCI or L2CAP packets.
*   **Solution**:
    1.  This is likely a bug in the packet parsing logic (`hci_packet_parser.rs`, `raw_packet_parser.rs`).
    2.  Capture the raw packet data that causes the error. You can use the `pcap_exporter.rs` module for this.
    3.  Open an issue and provide the problematic packet data for analysis.

### Incomplete Packet Data

*   **Problem**: Captured packets seem to be truncated or incomplete.
*   **Solution**:
    1.  This could be a buffer size issue in the raw sniffer (`raw_sniffer.rs`). Try increasing the buffer size.
    2.  It could also be a driver-level issue. Make sure your Bluetooth drivers are up-to-date.

---

## Platform-Specific Problems

### Windows

*   **Issue**: Problems with WinRT or Windows HCI API.
*   **Guide**: Refer to `windows_bluetooth.rs` and `windows_hci.rs` for implementation details. Ensure you have the necessary Windows SDKs installed.

### macOS

*   **Issue**: Problems with Core Bluetooth integration.
*   **Guide**: The integration is handled in `core_bluetooth_integration.rs`. Ensure your macOS version is compatible.

### Linux

*   **Issue**: Problems with BlueZ or kernel-level access.
*   **Guide**: Ensure the `bluez-dev` package (or equivalent for your distribution) is installed.

---

## Frontend & UI

### Web Interface Not Loading

*   **Problem**: The web server is running, but the frontend at `index.html` does not load or is blank.
*   **Solution**:
    1.  Check the browser's developer console (F12) for JavaScript errors in `app.js`.
    2.  Verify that the web server (`web_server.rs`) is running and accessible on the correct port.
    3.  Make sure all frontend files (`index.html`, `app.js`, `styles.css`) are in the correct location.

### Data Not Updating in Real-time

*   **Problem**: The web interface loads, but no new devices appear.
*   **Solution**:
    1.  Check for WebSocket connection errors in the browser's developer console.
    2.  Verify that the `web_server.rs` is correctly broadcasting device events to connected clients.
    3.  Ensure the backend is successfully scanning and processing devices.
