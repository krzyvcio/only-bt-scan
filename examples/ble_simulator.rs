use rand::Rng;
use rusqlite::{Connection, Result};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

struct ScanStats {
    total_writes: AtomicU64,
    errors: AtomicU64,
    total_time_ms: AtomicU64,
}

fn init_db() -> Result<Connection> {
    let conn = Connection::open("test_scanner.db")?;
    
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;
         PRAGMA mmap_size = 30000000000;
         PRAGMA page_size = 4096;"
    )?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS devices (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            mac TEXT NOT NULL,
            name TEXT,
            rssi INTEGER NOT NULL,
            timestamp INTEGER NOT NULL
        )",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_timestamp ON devices(timestamp)",
        [],
    )?;
    
    Ok(conn)
}

fn generate_mac(rng: &mut rand::rngs::ThreadRng) -> String {
    let hex = rng.gen_range(0..256);
    format!("AA:BB:CC:DD:EE:{:02X}", hex)
}

fn generate_name(rng: &mut rand::rngs::ThreadRng) -> String {
    let names = ["Device", "Beacon", "Sensor", "Tracker", "Tag"];
    let idx = rng.gen_range(0..names.len());
    format!("{}_{}", names[idx], rng.gen_range(1000..9999))
}

async fn run_scanner(stats: Arc<ScanStats>) {
    let conn = match init_db() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ERROR] Nie można otworzyć bazy: {}", e);
            return;
        }
    };
    
    let mut rng = rand::thread_rng();
    let start_time = Instant::now();
    let mut last_log_count = 0u64;
    let mut last_log_time = Instant::now();
    
    println!("[SCAN] Rozpoczynam symulację skanowania BLE...");
    println!("[SCAN] Baza: test_scanner.db (WAL mode)");
    println!("[SCAN] Czas pracy: 30 sekund");
    println!();
    
    loop {
        if start_time.elapsed().as_secs() >= 30 {
            break;
        }
        
        let mac = generate_mac(&mut rng);
        let name = generate_name(&mut rng);
        let rssi = rng.gen_range(-90..=-50);
        let timestamp = chrono::Utc::now().timestamp_millis();
        
        let write_start = Instant::now();
        
        let result = conn.execute(
            "INSERT INTO devices (mac, name, rssi, timestamp) VALUES (?1, ?2, ?3, ?4)",
            [&mac, &name, &rssi.to_string(), &timestamp.to_string()],
        );
        
        let write_time = write_start.elapsed().as_micros() as u64;
        
        match result {
            Ok(_) => {
                let count = stats.total_writes.fetch_add(1, Ordering::Relaxed) + 1;
                stats.total_time_ms.fetch_add(write_time, Ordering::Relaxed);
                
                if count % 100 == 0 {
                    let elapsed = last_log_time.elapsed().as_millis() as u64;
                    let batch_size = count - last_log_count;
                    println!(
                        "[SCAN] Zapisano {} urządzeń, czas: {}ms ({} zapisów/s)",
                        count, elapsed, (batch_size as f64 / elapsed as f64 * 1000.0) as u64
                    );
                    last_log_count = count;
                    last_log_time = Instant::now();
                }
                
                if write_time > 10_000 {
                    eprintln!("[WARN] Wolny zapis: {}µs", write_time);
                }
            }
            Err(e) => {
                stats.errors.fetch_add(1, Ordering::Relaxed);
                eprintln!("[ERROR] Błąd zapisu: {}", e);
            }
        }
        
        let delay = rng.gen_range(1..=100);
        sleep(Duration::from_millis(delay)).await;
    }
}

#[tokio::main]
async fn main() {
    let stats = Arc::new(ScanStats {
        total_writes: AtomicU64::new(0),
        errors: AtomicU64::new(0),
        total_time_ms: AtomicU64::new(0),
    });
    
    let start = Instant::now();
    
    run_scanner(stats.clone()).await;
    
    let total_time = start.elapsed();
    let total_writes = stats.total_writes.load(Ordering::Relaxed);
    let errors = stats.errors.load(Ordering::Relaxed);
    let total_write_time = stats.total_time_ms.load(Ordering::Relaxed);
    
    println!();
    println!("╔════════════════════════════════════════════════════╗");
    println!("║         STATYSTYKI SYMULACJI BLE SCANNER          ║");
    println!("╠════════════════════════════════════════════════════╣");
    println!("║ Całkowity czas pracy:        {:>8} s          ║", total_time.as_secs());
    println!("║ Liczba zapisów:              {:>8}              ║", total_writes);
    println!("║ Liczba błędów:               {:>8}              ║", errors);
    println!("║ Średni czas zapisu:          {:>8.2} µs         ║", 
        if total_writes > 0 { total_write_time as f64 / total_writes as f64 } else { 0.0 });
    println!("║ Zapisów na sekundę:          {:>8.2}              ║", 
        total_writes as f64 / total_time.as_secs_f64());
    println!("╚════════════════════════════════════════════════════╝");
    
    match Connection::open("test_scanner.db") {
        Ok(conn) => {
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM devices", [], |row| row.get(0))
                .unwrap_or(0);
            println!("[INFO] Wszystkich rekordów w bazie: {}", count);
        }
        Err(e) => eprintln!("[ERROR] Nie można odczytać statystyk bazy: {}", e),
    }
}
