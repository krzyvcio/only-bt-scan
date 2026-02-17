use rusqlite::Connection;

fn main() {
    let conn = Connection::open("test_scanner.db").unwrap();
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM devices", [], |row| row.get(0)).unwrap();
    println!("Liczba rekordów: {}", count);
    
    let avg_time: f64 = conn.query_row(
        "SELECT AVG(timestamp) FROM devices", [], |row| row.get::<_, f64>(0)
    ).unwrap_or(0.0);
    println!("Średni timestamp: {}", avg_time);
}
