use crate::db;
use chrono::Local;
/// HTML Report Generator for Bluetooth Scanner
/// Creates comprehensive HTML reports with all detected devices and RAW packets
use std::fs::{read_to_string, File};
use std::io::Write;

/// Format milliseconds to HH:MM:SS format
fn format_duration_human(ms: u64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    let millis = ms % 1000;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}.{:03}h", hours, minutes, seconds, millis)
    } else if minutes > 0 {
        format!("{:02}:{:02}.{:03}m", minutes, seconds, millis)
    } else {
        format!("{:02}.{:03}s", seconds, millis)
    }
}

/// Generate HTML report with all scanned devices and RAW packet logs
pub fn generate_html_report(
    raw_packet_filename: &str,
    scan_duration_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let report_filename = format!("{}_raport.html", timestamp);

    println!("📊 Generating HTML report: {}", report_filename);

    // Load all devices from database
    let devices = db::get_all_devices()?;
    let device_count = devices.len();

    // Load RAW packet logs
    let raw_packets_content = read_to_string(raw_packet_filename)
        .unwrap_or_else(|_| String::from("No RAW packet data available"));

    // Generate HTML
    let html = generate_html_content(
        &devices,
        &raw_packets_content,
        device_count,
        scan_duration_ms,
    );

    // Write to file
    let mut file = File::create(&report_filename)?;
    file.write_all(html.as_bytes())?;

    println!(
        "✅ HTML report created: {} (Scan duration: {})",
        report_filename,
        format_duration_human(scan_duration_ms)
    );
    Ok(())
}

fn generate_html_content(
    devices: &[crate::db::ScannedDevice],
    raw_packets: &str,
    total_devices: usize,
    scan_duration_ms: u64,
) -> String {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let scan_time_formatted = format_duration_human(scan_duration_ms);

    // Generate device cards
    let device_cards: String = devices
        .iter()
        .map(|device| {
            let rssi_class = if device.rssi >= -50 {
                "rssi-excellent"
            } else if device.rssi >= -70 {
                "rssi-good"
            } else if device.rssi >= -85 {
                "rssi-fair"
            } else {
                "rssi-poor"
            };

            let manufacturer = device.manufacturer_name.as_deref().unwrap_or("Unknown");
            let device_name = device.name.as_deref().unwrap_or("Unnamed Device");

            format!(
                r#"
        <div class="device-card">
            <div class="device-header">
                <h3>📱 {}</h3>
                <span class="rssi-badge {}">{} dBm</span>
            </div>
            <div class="device-details">
                <div class="detail-row">
                    <span class="detail-label">MAC Address:</span>
                    <span class="detail-value">{}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Manufacturer:</span>
                    <span class="detail-value">{}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">First Seen:</span>
                    <span class="detail-value">{}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Last Seen:</span>
                    <span class="detail-value">{}</span>
                </div>
            </div>
        </div>
        "#,
                device_name,
                rssi_class,
                device.rssi,
                device.mac_address,
                manufacturer,
                device.first_seen.format("%Y-%m-%d %H:%M:%S"),
                device.last_seen.format("%Y-%m-%d %H:%M:%S")
            )
        })
        .collect();

    // Escape HTML in RAW packets for safe display
    let raw_packets_escaped = html_escape(raw_packets);

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Bluetooth Scanner Report - {}</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: #333;
            padding: 20px;
            min-height: 100vh;
        }}
        
        .container {{
            max-width: 1400px;
            margin: 0 auto;
            background: white;
            border-radius: 16px;
            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
            overflow: hidden;
        }}
        
        .header {{
            background: linear-gradient(135deg, #1e3c72 0%, #2a5298 100%);
            color: white;
            padding: 40px;
            text-align: center;
        }}
        
        .header h1 {{
            font-size: 2.5em;
            margin-bottom: 10px;
            text-shadow: 2px 2px 4px rgba(0, 0, 0, 0.3);
        }}
        
        .header p {{
            font-size: 1.1em;
            opacity: 0.9;
        }}
        
        .stats {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            padding: 30px 40px;
            background: #f8f9fa;
            border-bottom: 3px solid #e9ecef;
        }}
        
        .stat-card {{
            background: white;
            padding: 20px;
            border-radius: 12px;
            text-align: center;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
            transition: transform 0.3s ease;
        }}
        
        .stat-card:hover {{
            transform: translateY(-5px);
        }}
        
        .stat-number {{
            font-size: 2.5em;
            font-weight: bold;
            color: #667eea;
            display: block;
            margin-bottom: 5px;
        }}
        
        .stat-label {{
            font-size: 0.9em;
            color: #6c757d;
            text-transform: uppercase;
            letter-spacing: 1px;
        }}
        
        .section {{
            padding: 40px;
        }}
        
        .section-title {{
            font-size: 1.8em;
            margin-bottom: 25px;
            color: #2c3e50;
            border-left: 5px solid #667eea;
            padding-left: 15px;
        }}
        
        .devices-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
            gap: 20px;
            margin-bottom: 40px;
        }}
        
        .device-card {{
            background: white;
            border: 2px solid #e9ecef;
            border-radius: 12px;
            padding: 20px;
            transition: all 0.3s ease;
        }}
        
        .device-card:hover {{
            border-color: #667eea;
            box-shadow: 0 8px 16px rgba(102, 126, 234, 0.2);
            transform: translateY(-3px);
        }}
        
        .device-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 15px;
            padding-bottom: 15px;
            border-bottom: 2px solid #f8f9fa;
        }}
        
        .device-header h3 {{
            color: #2c3e50;
            font-size: 1.2em;
        }}
        
        .rssi-badge {{
            padding: 5px 12px;
            border-radius: 20px;
            font-size: 0.85em;
            font-weight: bold;
        }}
        
        .rssi-excellent {{
            background: #28a745;
            color: white;
        }}
        
        .rssi-good {{
            background: #5cb85c;
            color: white;
        }}
        
        .rssi-fair {{
            background: #ffc107;
            color: #333;
        }}
        
        .rssi-poor {{
            background: #dc3545;
            color: white;
        }}
        
        .device-details {{
            display: flex;
            flex-direction: column;
            gap: 10px;
        }}
        
        .detail-row {{
            display: flex;
            justify-content: space-between;
            padding: 8px;
            background: #f8f9fa;
            border-radius: 6px;
        }}
        
        .detail-label {{
            font-weight: 600;
            color: #6c757d;
        }}
        
        .detail-value {{
            color: #2c3e50;
            font-family: 'Courier New', monospace;
        }}
        
        .raw-packets {{
            background: #1e1e1e;
            color: #d4d4d4;
            padding: 30px;
            border-radius: 12px;
            font-family: 'Courier New', monospace;
            font-size: 0.9em;
            line-height: 1.6;
            overflow-x: auto;
            white-space: pre-wrap;
            word-wrap: break-word;
            max-height: 800px;
            overflow-y: auto;
        }}
        
        .raw-packets::-webkit-scrollbar {{
            width: 12px;
        }}
        
        .raw-packets::-webkit-scrollbar-track {{
            background: #2a2a2a;
            border-radius: 6px;
        }}
        
        .raw-packets::-webkit-scrollbar-thumb {{
            background: #667eea;
            border-radius: 6px;
        }}
        
        .footer {{
            background: #f8f9fa;
            padding: 20px;
            text-align: center;
            color: #6c757d;
            font-size: 0.9em;
            border-top: 3px solid #e9ecef;
        }}
        
        @media (max-width: 768px) {{
            .container {{
                border-radius: 0;
            }}
            
            .devices-grid {{
                grid-template-columns: 1fr;
            }}
            
            .header h1 {{
                font-size: 1.8em;
            }}
            
            .section {{
                padding: 20px;
            }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🔵 Bluetooth Scanner Report</h1>
            <p>Generated: {}</p>
        </div>
        
        <div class="stats">
            <div class="stat-card">
                <span class="stat-number">{}</span>
                <span class="stat-label">Total Devices</span>
            </div>
            <div class="stat-card">
                <span class="stat-number">{}</span>
                <span class="stat-label">Scan Duration</span>
            </div>
            <div class="stat-card">
                <span class="stat-number">✅</span>
                <span class="stat-label">Scan Complete</span>
            </div>
            <div class="stat-card">
                <span class="stat-number">📡</span>
                <span class="stat-label">BLE + Classic</span>
            </div>
        </div>
        
        <div class="section">
            <h2 class="section-title">📱 Detected Devices</h2>
            <div class="devices-grid">
                {}
            </div>
        </div>
        
        <div class="section">
            <h2 class="section-title">📄 RAW Packet Logs</h2>
            <div class="raw-packets">{}</div>
        </div>
        
        <div class="footer">
            <p>Bluetooth Scanner v0.1.0 | Built with Rust & btleplug</p>
            <p>Report generated at: {}</p>
        </div>
    </div>
</body>
</html>"#,
        timestamp,
        timestamp,
        total_devices,
        scan_time_formatted,
        device_cards,
        raw_packets_escaped,
        timestamp
    )
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
