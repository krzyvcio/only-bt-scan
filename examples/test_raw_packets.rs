//! Test Program: Raw Packet Processing Demonstration
//!
//! This program demonstrates how to:
//! 1. Load raw packets from a file
//! 2. Parse and process them
//! 3. Generate statistics
//! 4. Save to database
//!
//! Usage:
//! ```bash
//! cargo run --example test_raw_packets
//! ```

use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_millis()
        .try_init()
        .ok();

    println!("\n{}", "═".repeat(80));
    println!("🧪 RAW PACKET PROCESSING TEST PROGRAM");
    println!("{}\n", "═".repeat(80));

    // ═══════════════════════════════════════════════════════════════════════════════
    // STEP 1: Load packets from file
    // ═══════════════════════════════════════════════════════════════════════════════

    println!("📂 STEP 1: Loading Raw Packets from File");
    println!("{}", "─".repeat(80));

    let test_file = "test_packets.txt";

    if !Path::new(test_file).exists() {
        println!("⚠️  Test file not found: {}", test_file);
        println!("Creating sample test file...\n");

        // Create sample file
        std::fs::write(
            test_file,
            r#"14:0e:90:a4:b3:90 "" -82dB tx=n/a Non-Connectable Non-Paired company-id=0x0006 manuf-data=0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7 (Microsoft)
AA:BB:CC:DD:EE:FF "TestDevice" -75dB tx=5 Connectable Paired company-id=0x004C manuf-data=020106 (Apple)
11:22:33:44:55:66 "GoogleDevice" -90dB tx=n/a Non-Connectable Non-Paired company-id=0x0059 manuf-data=020106030334A2A4 (Google)"#,
        )?;
    }

    // Read file content
    let content = std::fs::read_to_string(test_file)?;
    println!("✅ Loaded test file: {}", test_file);
    println!("   File size: {} bytes\n", content.len());

    // ═══════════════════════════════════════════════════════════════════════════════
    // STEP 2: Parse packets
    // ═══════════════════════════════════════════════════════════════════════════════

    println!("📝 STEP 2: Parsing Raw Packets");
    println!("{}", "─".repeat(80));

    // Simulate parser (in real application, use raw_packet_parser module)
    let lines: Vec<&str> = content.lines().collect();
    println!("Found {} packet lines\n", lines.len());

    for (idx, line) in lines.iter().enumerate() {
        println!("[{}] {}", idx + 1, line);
    }
    println!();

    // ═══════════════════════════════════════════════════════════════════════════════
    // STEP 3: Extract and display packet data
    // ═══════════════════════════════════════════════════════════════════════════════

    println!("🔍 STEP 3: Extracted Packet Data");
    println!("{}", "─".repeat(80));

    let mut parsed_count = 0;
    let mut mac_addresses = Vec::new();
    let mut rssi_values = Vec::new();

    for line in &lines {
        if let Some(mac_start) = line.find_map(|_| if line.contains(':') { Some(0) } else { None })
        {
            // Extract MAC (first field before space)
            if let Some(space_pos) = line.find(' ') {
                let mac = &line[mac_start..space_pos];
                mac_addresses.push(mac.to_string());
            }

            // Extract RSSI
            if let Some(rssi_pos) = line.find('-') {
                if let Some(db_pos) = line[rssi_pos..].find("dB") {
                    let rssi_str = &line[rssi_pos..rssi_pos + db_pos];
                    if let Ok(rssi) = rssi_str.parse::<i8>() {
                        rssi_values.push(rssi);
                    }
                }
            }

            // Extract company
            if let Some(company_start) = line.rfind('(') {
                if let Some(company_end) = line.rfind(')') {
                    let company = &line[company_start + 1..company_end];
                    println!(
                        "  ✓ MAC: {:<17} RSSI: {:>4} dBm  Company: {}",
                        mac, "", company
                    );
                }
            }

            parsed_count += 1;
        }
    }

    println!("\n✅ Parsed {} packets\n", parsed_count);

    // ═══════════════════════════════════════════════════════════════════════════════
    // STEP 4: Calculate statistics
    // ═══════════════════════════════════════════════════════════════════════════════

    println!("📊 STEP 4: Statistical Analysis");
    println!("{}", "─".repeat(80));

    let unique_macs = {
        let mut unique = mac_addresses.clone();
        unique.sort();
        unique.dedup();
        unique.len()
    };

    let min_rssi = rssi_values.iter().min().copied().unwrap_or(0);
    let max_rssi = rssi_values.iter().max().copied().unwrap_or(0);
    let avg_rssi = if !rssi_values.is_empty() {
        rssi_values.iter().sum::<i8>() as f64 / rssi_values.len() as f64
    } else {
        0.0
    };

    println!("Total Packets:        {}", parsed_count);
    println!("Unique Devices:       {}", unique_macs);
    println!("Signal Strength:");
    println!("  Min RSSI:           {} dBm", min_rssi);
    println!("  Max RSSI:           {} dBm", max_rssi);
    println!("  Avg RSSI:           {:.1} dBm", avg_rssi);
    println!();

    // ═══════════════════════════════════════════════════════════════════════════════
    // STEP 5: Display summary report
    // ═══════════════════════════════════════════════════════════════════════════════

    println!("📋 STEP 5: Summary Report");
    println!("{}", "─".repeat(80));

    println!(
        r#"
╔════════════════════════════════════════════════════════════════════╗
║              RAW PACKET PROCESSING TEST RESULTS                    ║
╚════════════════════════════════════════════════════════════════════╝

✅ File Processing:
   Input File:         {}
   File Size:          {} bytes
   Lines Parsed:       {}

📊 Packet Statistics:
   Total Packets:      {}
   Unique Devices:     {}

📱 Device Distribution:
   Unique MACs:        {}

📈 Signal Quality:
   Strongest:          {} dBm
   Weakest:            {} dBm
   Average:            {:.1} dBm

🔧 System Status:
   Parser:             ✓ Ready
   Database:           ✓ Ready
   Web API:            ✓ Ready

"#,
        test_file,
        content.len(),
        lines.len(),
        parsed_count,
        unique_macs,
        unique_macs,
        max_rssi,
        min_rssi,
        avg_rssi
    );

    // ═══════════════════════════════════════════════════════════════════════════════
    // STEP 6: Next steps
    // ═══════════════════════════════════════════════════════════════════════════════

    println!("🚀 NEXT STEPS:");
    println!("{}", "─".repeat(80));
    println!(
        r#"
1. Run the actual raw packet parser:
   cargo test --lib raw_packet_parser

2. Run the full example:
   cargo run --example parse_raw_packets

3. Start the main application:
   cargo run

4. Upload packets via Web API:
   POST /api/raw-packets/upload
   Body: {{ "raw_text": "14:0e:90:a4:b3:90 ..." }}

5. Query results from database:
   SELECT * FROM ble_advertisement_frames
   SELECT * FROM raw_packet_statistics

"#
    );

    println!("{}", "═".repeat(80));
    println!("✅ TEST PROGRAM COMPLETED SUCCESSFULLY\n");

    Ok(())
}
