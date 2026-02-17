use rusqlite::Connection;

fn main() {
    let conn = Connection::open("test_scanner.db").unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM devices", [], |row| row.get(0))
        .unwrap();
    println!("\n📊 WYNIKI SYMULACJI:");
    println!("═══════════════════════════════════════");
    println!("Liczba rekordów w bazie: {}", count);

    let avg_rssi: f64 = conn
        .query_row("SELECT AVG(rssi) FROM devices", [], |row| row.get(0))
        .unwrap_or(0.0);
    println!("Średni RSSI: {:.1} dBm", avg_rssi);

    let min_rssi: i64 = conn
        .query_row("SELECT MIN(rssi) FROM devices", [], |row| row.get(0))
        .unwrap_or(0);
    let max_rssi: i64 = conn
        .query_row("SELECT MAX(rssi) FROM devices", [], |row| row.get(0))
        .unwrap_or(0);
    println!("RSSI zakres: {} do {} dBm", min_rssi, max_rssi);

    let distinct_macs: i64 = conn
        .query_row("SELECT COUNT(DISTINCT mac) FROM devices", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);
    println!("Unikalne MAC: {}", distinct_macs);

    println!("\n✅ Symulacja zakończona pomyślnie!");
    println!("   Baza: test_scanner.db (WAL mode)");
}
