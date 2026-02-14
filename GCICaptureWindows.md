# GCI Capture Windows (HCI RAW) - Plan

Goal: Capture LE Advertising Reports via Windows HCI, parse full payload, and store/serve complete packet data (including lengths).

## Steps
1) Add a real HCI event reader for Windows
   - Implement read of HCI Event packets (packet type 0x04) from a Windows HCI source
   - Source options: WinUSB/COM (USB dongle) or Windows.Devices.Bluetooth HCI passthrough

2) Parse LE Meta Event (0x3E)
   - Parse subevent 0x02 (LE Advertising Report)
   - Extract per-report fields: event_type, address_type, address, data_length, data, rssi

3) Map report to RawPacketModel
   - advertising_data = report data (raw bytes)
   - packet_type from event_type (ADV_IND, ADV_DIRECT_IND, ADV_SCAN_IND, ADV_NONCONN_IND, SCAN_RSP)
   - rssi from report
   - phy/channel if available (fallback: LE 1M, channel 37)
   - set total_length = advertising_data.len()

4) Store in SQLite
   - Insert into ble_advertisement_frames with device_id resolved by mac_address
   - Add lengths (frame_len, payload_len, manuf_len) if schema updated

5) Web API
   - Extend RawPacket response with lengths and raw data hex
   - Verify /api/raw-packets and /api/raw-packets/all show full payloads

6) UI
   - Add columns for frame_len, payload_len, manuf_len
   - Show manufacturer data hex for quick validation

7) Validation
   - Compare output with external tool examples (Microsoft Swift Pair, etc.)
   - Confirm non-connectable frames show ADV_NONCONN_IND

Notes:
- btleplug does not provide full raw advertising payload; HCI capture is required.
- Keep btleplug as fallback, but mark packet_source if full HCI is unavailable.
