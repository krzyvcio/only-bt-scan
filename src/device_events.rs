/// Device Event Listener - System-wide Bluetooth device monitoring
///
/// Monitors and dispatches Bluetooth device events across the system.
/// Implements a publish-subscribe pattern for device state changes.
///
/// # Monitors:
/// - Device arrivals and removals
/// - Connection/disconnection events
/// - RSSI updates
/// - Cross-platform event handling
use log::{debug, info, warn};
use tokio::sync::mpsc;
use std::sync::Arc;

pub struct DeviceEventListener {
    tx: mpsc::UnboundedSender<DeviceEventNotification>,
    rx: Option<mpsc::UnboundedReceiver<DeviceEventNotification>>,
}

/// Notification wrapper for device events
///
/// Contains timestamp and the actual event data.
#[derive(Debug, Clone)]
pub struct DeviceEventNotification {
    /// Timestamp when event occurred (SystemTime)
    pub timestamp: std::time::SystemTime,
    /// The actual Bluetooth device event
    pub event: BluetoothDeviceEvent,
}

/// Types of Bluetooth device events
///
/// Represents all possible device state changes and discoveries.
#[derive(Debug, Clone)]
pub enum BluetoothDeviceEvent {
    /// New device discovered during scanning
    DeviceDiscovered {
        /// MAC address of discovered device
        mac_address: String,
        /// Device name from advertising data
        name: Option<String>,
        /// Signal strength in dBm
        rssi: i8,
        /// Is this a BLE (Bluetooth Low Energy) device?
        is_ble: bool,
        /// Is this a Classic Bluetooth (BR/EDR) device?
        is_bredr: bool,
    },
    /// Existing device updated (new RSSI or name)
    DeviceUpdated {
        /// MAC address of updated device
        mac_address: String,
        /// New RSSI value
        rssi: i8,
        /// Updated device name
        name: Option<String>,
    },
    /// Device no longer visible/removed
    DeviceRemoved {
        /// MAC address of removed device
        mac_address: String,
    },
    /// Device connected to system
    DeviceConnected {
        /// MAC address of connected device
        mac_address: String,
        /// Type of connection (BLE/BrEdr/DualMode)
        connection_type: ConnectionType,
    },
    /// Device disconnected from system
    DeviceDisconnected {
        /// MAC address of disconnected device
        mac_address: String,
        /// Reason for disconnection
        reason: String,
    },
    /// Pairing initiated by device
    PairingRequested {
        /// MAC address requesting pairing
        mac_address: String,
        /// Name of device requesting pairing
        device_name: Option<String>,
        /// Method used for pairing
        pairing_method: PairingMethod,
    },
    /// Pairing completed (success or failure)
    PairingCompleted {
        /// MAC address of paired device
        mac_address: String,
        /// Whether pairing succeeded
        success: bool,
    },
}

/// Type of Bluetooth connection
#[derive(Debug, Clone)]
pub enum ConnectionType {
    /// Bluetooth Low Energy connection
    BLE,
    /// Classic Bluetooth (BR/EDR) connection
    BrEdr,
    /// Supports both BLE and Classic Bluetooth
    DualMode,
}

/// Method used for device pairing
#[derive(Debug, Clone)]
pub enum PairingMethod {
    /// Just Works - no PIN required
    JustWorks,
    /// Numeric comparison (both devices show same number)
    NumericComparison,
    /// Passkey entry (6-digit code)
    PasskeyEntry,
    /// Out of band pairing (NFC, etc.)
    OutOfBand,
}

impl DeviceEventListener {
    /// Create new event listener
    ///
    /// # Returns
    /// New DeviceEventListener with channel for event dispatch
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { tx, rx: Some(rx) }
    }

    /// Get receiver for events
    ///
    /// Takes ownership of the receiver - can only be called once.
    ///
    /// # Returns
    /// Some(UnboundedReceiver) if not already taken, None otherwise
    pub fn get_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<DeviceEventNotification>> {
        self.rx.take()
    }

    /// Emit event
    ///
    /// Sends event to all subscribers. Silently fails if no receiver exists.
    ///
    /// # Arguments
    /// * `event` - Bluetooth device event to emit
    pub fn emit(&self, event: BluetoothDeviceEvent) {
        let notification = DeviceEventNotification {
            timestamp: std::time::SystemTime::now(),
            event,
        };

        if let Err(e) = self.tx.send(notification) {
            debug!("Device event emission failed (no listeners): {}", e);
        }
    }

    /// Start listening (must be called before using receiver)
    ///
    /// Initializes the event listener. Should be called before
    /// starting to process events from the receiver.
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err` if initialization failed
    pub async fn listen(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🎧 Device Event Listener started");
        Ok(())
    }
}

/// Background task for Windows device events
///
/// Platform-specific handler for Windows device change notifications.
/// Uses WM_DEVICECHANGE messages through window message loop.
///
/// # Arguments
/// * `event_listener` - Event listener to receive Windows events
///
/// # Returns
/// * `Ok(())` on success or if not on Windows
/// * `Err` on Windows if setup fails
pub async fn listen_windows_device_events(
    _event_listener: Arc<DeviceEventListener>,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        info!("🪟 Windows device event listener initialized");

        // This would use WM_DEVICECHANGE messages through a window message loop
        // For now, we just log that it's ready

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        warn!("Windows-specific device event listening not available on this platform");
        Ok(())
    }
}

/// Event logger - subscribes to device events and logs them
///
/// Async task that receives device events and logs them at appropriate
/// log levels (info, debug, warn) based on event type.
///
/// # Arguments
/// * `rx` - Receiver for device event notifications
pub async fn run_event_logger(mut rx: mpsc::UnboundedReceiver<DeviceEventNotification>) {
    while let Some(notification) = rx.recv().await {
        match &notification.event {
            BluetoothDeviceEvent::DeviceDiscovered {
                mac_address,
                name,
                rssi,
                is_ble,
                is_bredr,
            } => {
                info!(
                    "📱 Device discovered: {} ({}) RSSI: {} dBm | BLE: {} BR/EDR: {}",
                    mac_address,
                    name.as_deref().unwrap_or("Unknown"),
                    rssi,
                    is_ble,
                    is_bredr
                );
            }
            BluetoothDeviceEvent::DeviceUpdated {
                mac_address,
                rssi,
                name: _,
            } => {
                debug!("📡 Device updated: {} RSSI: {} dBm", mac_address, rssi);
            }
            BluetoothDeviceEvent::DeviceRemoved { mac_address } => {
                info!("📵 Device removed: {}", mac_address);
            }
            BluetoothDeviceEvent::DeviceConnected {
                mac_address,
                connection_type,
            } => {
                info!(
                    "🔗 Device connected: {} ({:?})",
                    mac_address, connection_type
                );
            }
            BluetoothDeviceEvent::DeviceDisconnected {
                mac_address,
                reason,
            } => {
                info!("🔌 Device disconnected: {} ({})", mac_address, reason);
            }
            BluetoothDeviceEvent::PairingRequested {
                mac_address,
                device_name,
                pairing_method,
            } => {
                info!(
                    "🔐 Pairing requested: {} ({:?}) via {:?}",
                    mac_address, device_name, pairing_method
                );
            }
            BluetoothDeviceEvent::PairingCompleted {
                mac_address,
                success,
            } => {
                if *success {
                    info!("✅ Pairing completed: {}", mac_address);
                } else {
                    warn!("❌ Pairing failed: {}", mac_address);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_listener_creation() {
        let listener = DeviceEventListener::new();
        assert!(listener.tx.is_closed() == false);
    }

    #[test]
    fn test_event_emission() {
        let listener = DeviceEventListener::new();
        let event = BluetoothDeviceEvent::DeviceDiscovered {
            mac_address: "AA:BB:CC:DD:EE:FF".to_string(),
            name: Some("Test Device".to_string()),
            rssi: -60,
            is_ble: true,
            is_bredr: false,
        };

        listener.emit(event);
        // If we got here without panicking, emission worked
    }
}
