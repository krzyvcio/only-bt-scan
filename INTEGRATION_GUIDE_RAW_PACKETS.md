# 🔗 Raw Packet System - Integration Guide

## Overview

This guide explains how to integrate the raw packet handling system into your main Bluetooth scanner application.

## Step 1: Verify Files Are in Place

Ensure these files exist:
- `src/raw_packet_parser.rs` - Parser implementation
- `MANUAL_RAW_PACKET_PROCESSING.md` - User guide
- `RAW_PACKET_SYSTEM.md` - System architecture
- `examples/parse_raw_packets.rs` - Working example

## Step 2: Module Declaration

The module is already declared in `src/main.rs`:
```rust
mod raw_packet_parser;
```

No action needed - it's ready to use.

## Step 3: Import in Your Code

```rust
use crate::raw_packet_parser::{
    RawPacketParser,
    RawPacketBatchProcessor,
    RawPacketData,
    RawPacketStatistics,
};
use crate::db_frames;
```

## Step 4: Create Helper Function

Add this to your main scan loop or a dedicated module:

```rust
/// Process raw Bluetooth packet text and store in database
pub async fn process_raw_packet_input(
    raw_text: &str,
    conn: &rusqlite::Connection,
) -> Result<RawPacketStatistics, Box<dyn std::error::Error>> {
    log::info!("Processing raw packet input ({} bytes)", raw_text.len());
    
    // Parse packets
    let parser = RawPacketParser::new();
    let packets = parser.parse_packets(raw_text);
    
    if packets.is_empty() {
        log::warn!("No packets parsed from input");
        return Err("No valid packets found".into());
    }
    
    log::info!("Parsed {} packets", packets.len());
    
    // Process batch
    let mut processor = RawPacketBatchProcessor::new();
    for packet in packets {
        processor.packets.push(packet);
    }
    
    let models = processor.process_all();
    let stats = processor.get_statistics();
    
    // Store in database
    db_frames::insert_raw_packets_from_scan(conn, &models)?;
    
    // Store statistics with session ID
    let session_id = chrono::Utc::now().to_rfc3339();
    db_frames::store_packet_statistics(conn, &stats, &session_id)?;
    
    log::info!(
        "✅ Stored {} packets ({} unique devices)",
        stats.total_packets,
        stats.unique_macs
    );
    
    Ok(stats)
}
```

## Step 5: Add Web API Endpoint

In `src/web_server.rs`, add this handler:

```rust
use serde::{Deserialize, Serialize};
use crate::raw_packet_parser::RawPacketParser;

#[derive(Deserialize)]
pub struct RawPacketUpload {
    pub raw_text: String,
}

#[derive(Serialize)]
pub struct RawPacketUploadResponse {
    pub success: bool,
    pub packets_uploaded: usize,
    pub unique_devices: usize,
    pub min_rssi: i8,
    pub max_rssi: i8,
    pub avg_rssi: f64,
}

pub async fn upload_raw_packets(
    payload: web::Json<RawPacketUpload>,
) -> impl Responder {
    log::info!("Received raw packet upload request");
    
    let parser = RawPacketParser::new();
    let packets = parser.parse_packets(&payload.raw_text);
    
    if packets.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "No packets parsed from input"
        }));
    }
    
    match rusqlite::Connection::open("bluetooth_scan.db") {
        Ok(conn) => {
            let mut processor = crate::raw_packet_parser::RawPacketBatchProcessor::new();
            for packet in packets {
                processor.packets.push(packet);
            }
            
            processor.process_all();
            let stats = processor.get_statistics();
            
            match crate::db_frames::insert_parsed_raw_packets(&conn, &processor.packets) {
                Ok(_) => {
                    let session_id = chrono::Utc::now().to_rfc3339();
                    let _ = crate::db_frames::store_packet_statistics(&conn, &stats, &session_id);
                    
                    log::info!(
                        "✅ Uploaded {} packets from {} devices",
                        stats.total_packets,
                        stats.unique_macs
                    );
                    
                    HttpResponse::Ok().json(RawPacketUploadResponse {
                        success: true,
                        packets_uploaded: stats.total_packets,
                        unique_devices: stats.unique_macs,
                        min_rssi: stats.min_rssi,
                        max_rssi: stats.max_rssi,
                        avg_rssi: stats.avg_rssi,
                    })
                },
                Err(e) => {
                    log::error!("Failed to insert packets: {}", e);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Database error: {}", e)
                    }))
                }
            }
        },
        Err(e) => {
            log::error!("Database connection failed: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database connection error: {}", e)
            }))
        }
    }
}
```

Register the endpoint in `configure_services()`:

```rust
pub fn configure_services(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/raw-packets/upload", web::post().to(upload_raw_packets))
            .route("/devices", web::get().to(get_devices))
            // ... other routes
    )
}
```

## Step 6: Add Frontend Form

In `frontend/index.html`, add this section:

```html
<div class="raw-packet-upload-section">
    <h3>📦 Upload Raw Packets</h3>
    <form id="raw-packet-form">
        <div class="form-group">
            <label for="raw-packet-input">Raw Packet Data:</label>
            <textarea 
                id="raw-packet-input" 
                placeholder="Paste raw packet data here...
Example:
14:0e:90:a4:b3:90 &quot;&quot; -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)"
                rows="8"
            ></textarea>
        </div>
        <button type="submit" class="btn btn-primary">Upload Packets</button>
    </form>
    <div id="upload-result" class="upload-result"></div>
</div>
```

## Step 7: Add Frontend JavaScript

In `frontend/app.js`, add:

```javascript
document.getElementById('raw-packet-form')?.addEventListener('submit', async (e) => {
    e.preventDefault();
    
    const rawText = document.getElementById('raw-packet-input').value;
    const resultDiv = document.getElementById('upload-result');
    
    if (!rawText.trim()) {
        resultDiv.innerHTML = '<p class="error">Please enter raw packet data</p>';
        return;
    }
    
    resultDiv.innerHTML = '<p class="loading">Processing...</p>';
    
    try {
        const response = await fetch('/api/raw-packets/upload', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ raw_text: rawText })
        });
        
        const data = await response.json();
        
        if (response.ok) {
            resultDiv.innerHTML = `
                <div class="success">
                    <h4>✅ Upload Successful</h4>
                    <p>Packets Uploaded: <strong>${data.packets_uploaded}</strong></p>
                    <p>Unique Devices: <strong>${data.unique_devices}</strong></p>
                    <p>Signal Strength:</p>
                    <ul>
                        <li>Min: ${data.min_rssi} dBm</li>
                        <li>Max: ${data.max_rssi} dBm</li>
                        <li>Avg: ${data.avg_rssi.toFixed(1)} dBm</li>
                    </ul>
                </div>
            `;
            document.getElementById('raw-packet-input').value = '';
        } else {
            resultDiv.innerHTML = `<p class="error">Error: ${data.error}</p>`;
        }
    } catch (error) {
        resultDiv.innerHTML = `<p class="error">Upload failed: ${error.message}</p>`;
    }
});
```

## Step 8: Add Frontend CSS

In `frontend/styles.css`, add:

```css
.raw-packet-upload-section {
    background: #f9f9f9;
    padding: 20px;
    border-radius: 8px;
    margin: 20px 0;
}

.raw-packet-upload-section h3 {
    margin-top: 0;
    color: #333;
}

#raw-packet-input {
    width: 100%;
    padding: 10px;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-family: 'Courier New', monospace;
    font-size: 12px;
    resize: vertical;
}

#raw-packet-input::placeholder {
    color: #999;
    font-size: 11px;
}

.upload-result {
    margin-top: 15px;
}

.upload-result .success {
    background: #d4edda;
    border: 1px solid #c3e6cb;
    color: #155724;
    padding: 15px;
    border-radius: 4px;
}

.upload-result .success h4 {
    margin-top: 0;
}

.upload-result .success ul {
    margin: 10px 0 0 20px;
}

.upload-result .error {
    background: #f8d7da;
    border: 1px solid #f5c6cb;
    color: #721c24;
    padding: 15px;
    border-radius: 4px;
}

.upload-result .loading {
    color: #0066cc;
    font-style: italic;
}
```

## Step 9: Test the Integration

### Compile
```bash
cargo check
```

### Run Tests
```bash
cargo test --lib raw_packet_parser
```

### Run Example
```bash
cargo run --example parse_raw_packets
```

### Start Server
```bash
cargo run --release
```

Then navigate to `http://localhost:8080` and use the "Upload Raw Packets" form.

## Step 10: Use in Scan Loop (Optional)

To automatically process raw packets during normal scanning:

```rust
// In your scan loop
let raw_data = "14:0e:90:a4:b3:90 \"\" -82dB tx=n/a Non-Connectable...";

if let Ok(stats) = process_raw_packet_input(raw_data, &conn).await {
    log::info!("Processed {} packets", stats.total_packets);
}
```

## Integration Points Summary

| Component | File | Function |
|-----------|------|----------|
| Parser | `src/raw_packet_parser.rs` | `RawPacketParser::parse_packets()` |
| Database | `src/db_frames.rs` | `insert_parsed_raw_packets()` |
| Web API | `src/web_server.rs` | `upload_raw_packets()` |
| Frontend | `frontend/app.js` | Raw packet form handler |
| Styles | `frontend/styles.css` | `.raw-packet-upload-section` |

## Testing the Integration

### Test 1: Via Web API
```bash
curl -X POST http://localhost:8080/api/raw-packets/upload \
  -H "Content-Type: application/json" \
  -d '{"raw_text":"14:0e:90:a4:b3:90 \"\" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)"}'
```

### Test 2: Via Rust Code
```rust
let raw_text = "14:0e:90:a4:b3:90 \"\" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)";
let parser = RawPacketParser::new();
let packets = parser.parse_packets(raw_text);
assert_eq!(packets.len(), 1);
assert_eq!(packets[0].mac_address, "14:0E:90:A4:B3:90");
```

### Test 3: Via Frontend
1. Navigate to `http://localhost:8080`
2. Scroll to "Upload Raw Packets" section
3. Paste raw packet data
4. Click "Upload Packets"
5. View results

## Troubleshooting

### Issue: Compilation Error "mod raw_packet_parser not found"
**Solution**: Ensure `mod raw_packet_parser;` is in `src/main.rs`

### Issue: Database insertion fails
**Solution**: Ensure `db_frames::init_frame_storage()` was called during startup

### Issue: Web endpoint returns 404
**Solution**: Verify route is registered in `configure_services()`

### Issue: Parser returns empty results
**Solution**: Check input format matches specification exactly

## Next Steps

1. ✅ Run `cargo check` to verify compilation
2. ✅ Run tests: `cargo test --lib raw_packet_parser`
3. ✅ Run example: `cargo run --example parse_raw_packets`
4. ✅ Start application: `cargo run`
5. ✅ Test web endpoint with curl
6. ✅ Test frontend form in browser
7. ✅ Monitor logs with `RUST_LOG=debug`

## Performance Considerations

- **Batch Processing**: Process 100+ packets per request for best performance
- **Database**: Insert in transactions for ~1000 packets/second
- **Memory**: Each packet ~50-200 bytes, 1000 packets = ~100 KB
- **Deduplication**: Use `deduplicate_by_mac()` to reduce storage

## Security Notes

- Validate input length (reject > 1 MB raw text)
- Sanitize MAC addresses before storage
- Validate hex data format
- Use parameterized queries (already done)
- Add rate limiting to API endpoint

## Monitoring

Enable debug logging to monitor raw packet processing:

```bash
RUST_LOG=raw_packet_parser=debug cargo run
```

You'll see:
```
[DEBUG] raw_packet_parser: Parsing packet: 14:0e:90:a4:b3:90 ...
[INFO] raw_packet_parser: Successfully inserted 5 parsed raw packets
[DEBUG] db_frames: Stored statistics for scan session: 2024-01-15T...
```

## References

- User Guide: `MANUAL_RAW_PACKET_PROCESSING.md`
- Architecture: `RAW_PACKET_SYSTEM.md`
- Quick Reference: `RAW_PACKET_QUICK_REFERENCE.md`
- Implementation: `RAW_PACKET_IMPLEMENTATION_SUMMARY.md`
- Examples: `examples/parse_raw_packets.rs`

## Support

For issues or questions:
1. Check the troubleshooting sections in documentation
2. Review the example program
3. Enable debug logging
4. Check database schema in `db_frames.rs`

---

**Status**: ✅ Ready for Integration
**Last Updated**: 2024
**Version**: 1.0