/// Diagnostic test for advertisement parser
/// Shows exactly what happens with hex data

#[cfg(test)]
mod diagnostic {
    use only_bt_scan::db::parse_advertisement_data;

    /// Test most basic hex: just flags, no manufacturer
    #[test]
    fn test_flags_only() {
        // This is just: 02 01 06
        // Length=2, Type=Flags (0x01), Data=0x06 (LE General Discoverable + No BR/EDR)
        let hex = "020106";
        println!("\n=== Parsing: {} ===", hex);
        let result = parse_advertisement_data(hex);
        println!("Result: {:?}", result);
        println!("Flags: {:?}", result.flags);
        assert!(result.flags.is_some(), "Flags should be Some");
    }

    /// Test TX Power - simple two frame advertisement
    #[test]
    fn test_flags_and_tx_power() {
        // 02 01 06      - Flags (length=2)
        // 02 0a c5      - TX Power (length=2, type=0x0a, data=0xc5)
        let hex = "020106020ac5";
        println!("\n=== Parsing: {} ===", hex);
        let result = parse_advertisement_data(hex);
        println!("Result: {:?}", result);
        println!("Flags: {:?}", result.flags);
        println!("TX Power: {:?}", result.tx_power);
        assert!(result.flags.is_some());
        assert_eq!(result.tx_power, Some(-59));
    }

    /// Test with actual manufacturer data from test_packets.txt
    /// Microsoft proximity beacon (REAL from actual device)
    ///  Device: 14:0e:90:a4:b3:90
    ///  Mfg Data Hex (from test_packets): 0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7
    #[test]
    fn test_microsoft_real_data() {
        // Let me construct proper BLE advertisement frame format:
        // 02 01 00       - Flags: length=2, type=01, data=00 (non-discoverable)
        // XX ff 06 00    - Manufacturer: type=ff, company_id_low=06, company_id_high=00 (0x0006 = Microsoft)
        //                  The data we got was: 0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7
        //                  That's 27 bytes
        //                  So length = 1 (type) + 2 (company_id) + 27 (data) = 30 = 0x1E
        
        let mfg_data = "0109202231AF58B9F00F7253DF20A1848DA6DEACC747C910C10EE7";
        let length = (1 + 2 + mfg_data.len() / 2) as u8; // Type + company_id + data
        let frame = format!("020100{:02x}ff0600{}", length, mfg_data);
        
        println!("\n=== Parsing Microsoft: {} ===", frame);
        let result = parse_advertisement_data(&frame);
        println!("Result: {:?}", result);
        println!("Manufacturer: {:?}", result.manufacturer_name);
        println!("Mfg Data: {:?}", result.manufacturer_data);
    }

    /// Test Google beacon from test_packets.txt
    /// Device: 11:22:33:44:55:66 "GoogleDevice"
    /// Mfg data from packet: 020106030334A2A4
    #[test]
    fn test_google_real_data() {
        // Mfg data: 020106030334A2A4 = 8 bytes
        // Length = 1 (type) + 2 (company_id) + 8 (data) = 11 = 0x0B
        let mfg_data = "020106030334A2A4";
        let length = (1 + 2 + mfg_data.len() / 2) as u8;
        let frame = format!("020106{:02x}ff5900{}", length, mfg_data);
        
        println!("\n=== Parsing Google: {} ===", frame);
        let result = parse_advertisement_data(&frame);
        println!("Result: {:?}", result);
        println!("Manufacturer: {:?}", result.manufacturer_name);
    }

    /// Test with complete hex from bluetooth spec example
    #[test]
    fn test_spec_example() {
        // From Bluetooth specification examples:
        // Flags (type 0x01), length 2: 02 01 06
        // TX Power (type 0x0a), length 2: 02 0a c5
        // Local name (type 0x09), length N: 0x(N+1) 09 "data..."
        
        let name_data = "54657374";  // "Test" in hex
        let name_frame = format!("{:02x}09{}", 1 + name_data.len() / 2, name_data);
        let full_hex = format!("020106020ac5{}", name_frame);
        
        println!("\n=== Parsing spec example: {} ===", full_hex);
        let result = parse_advertisement_data(&full_hex);
        println!("Flags: {:?}", result.flags);
        println!("TX Power: {:?}", result.tx_power);
        println!("Local Name: {:?}", result.local_name);
    }
}
