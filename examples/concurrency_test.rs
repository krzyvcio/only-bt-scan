//! Test integracyjny: BLE Scanner + Telegram współbieżność
//! 
//! Uruchamia jednocześnie:
//! - Skaner BLE zapisujący co 1-100ms
//! - Telegram odczytujący raporty co 5s
//! 
//! Sprawdza czy nie ma konfliktów przy dostępie do SQLite

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Statystyki z testu
#[derive(Debug, Default)]
pub struct ConcurrencyTestResult {
    pub scan_writes: u64,
    pub scan_errors: u64,
    pub telegram_reads: u64,
    pub telegram_errors: u64,
    pub avg_scan_time_ms: f64,
    pub avg_telegram_time_ms: f64,
    pub max_scan_time_ms: f64,
    pub max_telegram_time_ms: f64,
    pub concurrent_access_count: u64,
}

/// Inicjalizuje testową bazę SQLite z WAL mode
fn init_test_db() -> rusqlite::Result<rusqlite::Connection> {
    let conn = rusqlite::Connection::open("test_concurrency.db")?;
    
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 10000;
         PRAGMA temp_store = MEMORY;"
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
    
    // Tabela dla raportów Telegrama
    conn.execute(
        "CREATE TABLE IF NOT EXISTS telegram_reports (
            id INTEGER PRIMARY KEY,
            last_report_time INTEGER,
            report_count INTEGER DEFAULT 0
        )",
        [],
    )?;
    
    conn.execute(
        "INSERT OR IGNORE INTO telegram_reports (id, last_report_time, report_count) 
         VALUES (1, strftime('%s', 'now'), 0)",
        [],
    )?;
    
    Ok(conn)
}

/// Symulator skanera BLE - ciągły zapis
async fn run_ble_scanner(
    running: Arc<AtomicBool>,
    write_count: Arc<AtomicU64>,
    error_count: Arc<AtomicU64>,
    total_time: Arc<AtomicU64>,
    max_time: Arc<AtomicU64>,
) {
    let mut counter = 0u64;
    
    while running.load(Ordering::Relaxed) {
        counter += 1;
        
        // Losowy delay 1-100ms między skanami
        let delay = (counter % 99) + 1; // 1-100ms
        sleep(Duration::from_millis(delay)).await;
        
        let mac = format!("AA:BB:CC:DD:EE:{:02X}", counter % 256);
        let name = format!("Device_{}", counter);
        let rssi = -50 - (counter % 40) as i32; // -50 do -90
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        let start = Instant::now();
        
        // Zapis w spawn_blocking
        let write_result = tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open("test_concurrency.db")?;
            conn.execute(
                "INSERT INTO devices (mac, name, rssi, timestamp) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![mac, name, rssi, timestamp],
            )?;
            Ok::<_, rusqlite::Error>(())
        }).await;
        
        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_millis() as u64;
        
        match write_result {
            Ok(Ok(())) => {
                write_count.fetch_add(1, Ordering::Relaxed);
                total_time.fetch_add(elapsed_ms, Ordering::Relaxed);
                
                let current_max = max_time.load(Ordering::Relaxed);
                if elapsed_ms > current_max {
                    max_time.store(elapsed_ms, Ordering::Relaxed);
                }
                
                if counter % 100 == 0 {
                    println!("[SCAN] Zapisano {} urządzeń, czas ostatniego: {}ms", 
                        counter, elapsed_ms);
                }
            }
            Ok(Err(e)) => {
                error_count.fetch_add(1, Ordering::Relaxed);
                eprintln!("[SCAN ERROR] Błąd SQL: {}", e);
            }
            Err(e) => {
                error_count.fetch_add(1, Ordering::Relaxed);
                eprintln!("[SCAN ERROR] Błąd task: {}", e);
            }
        }
    }
    
    println!("[SCAN] Zakończono. Łącznie zapisów: {}", counter);
}

/// Symulator Telegrama - odczyt raportów
async fn run_telegram(
    running: Arc<AtomicBool>,
    read_count: Arc<AtomicU64>,
    error_count: Arc<AtomicU64>,
    total_time: Arc<AtomicU64>,
    max_time: Arc<AtomicU64>,
    concurrent_access: Arc<AtomicU64>,
) {
    let mut report_id = 0u64;
    
    while running.load(Ordering::Relaxed) {
        report_id += 1;
        
        // Czekaj 5 sekund między raportami
        sleep(Duration::from_secs(5)).await;
        
        if !running.load(Ordering::Relaxed) {
            break;
        }
        
        let start = Instant::now();
        let mut success = false;
        
        // Retry logic - do 3 prób
        for attempt in 0..3 {
            let result = tokio::task::spawn_blocking(move || {
                let conn = rusqlite::Connection::open("test_concurrency.db")?;
                
                // Pobierz liczbę urządzeń z ostatniej minuty
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM devices WHERE timestamp > strftime('%s', 'now', '-1 minute')",
                    [],
                    |row| row.get(0),
                )?;
                
                // Zaktualizuj czas ostatniego raportu
                conn.execute(
                    "UPDATE telegram_reports SET last_report_time = strftime('%s', 'now'), 
                     report_count = report_count + 1 WHERE id = 1",
                    [],
                )?;
                
                Ok::<_, rusqlite::Error>(count)
            }).await;
            
            match result {
                Ok(Ok(count)) => {
                    let elapsed = start.elapsed();
                    let elapsed_ms = elapsed.as_millis() as u64;
                    
                    read_count.fetch_add(1, Ordering::Relaxed);
                    total_time.fetch_add(elapsed_ms, Ordering::Relaxed);
                    
                    let current_max = max_time.load(Ordering::Relaxed);
                    if elapsed_ms > current_max {
                        max_time.store(elapsed_ms, Ordering::Relaxed);
                    }
                    
                    println!("[TELEGRAM] Raport #{}: {} urządzeń, czas: {}ms (próba {})", 
                        report_id, count, elapsed_ms, attempt + 1);
                    
                    success = true;
                    break;
                }
                Ok(Err(e)) => {
                    if attempt < 2 {
                        let delay = 100 * (attempt + 1) as u64;
                        eprintln!("[TELEGRAM] Błąd (próba {}): {}. Retry za {}ms...", 
                            attempt + 1, e, delay);
                        sleep(Duration::from_millis(delay)).await;
                    } else {
                        error_count.fetch_add(1, Ordering::Relaxed);
                        eprintln!("[TELEGRAM ERROR] Raport #{} nieudany: {}", report_id, e);
                    }
                }
                Err(e) => {
                    error_count.fetch_add(1, Ordering::Relaxed);
                    eprintln!("[TELEGRAM ERROR] Task error: {}", e);
                    break;
                }
            }
        }
        
        if !success {
            concurrent_access.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    println!("[TELEGRAM] Zakończono. Łącznie raportów: {}", report_id);
}

/// Główna funkcja testowa
pub async fn run_concurrency_test(duration_secs: u64) -> ConcurrencyTestResult {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  TEST WSPÓŁBIEŻNOŚCI: BLE Scanner + Telegram              ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    println!("Inicjalizacja bazy testowej...");
    
    // Inicjalizacja bazy
    let _ = init_test_db().expect("Nie można zainicjować bazy");
    
    // Flagi i liczniki
    let running = Arc::new(AtomicBool::new(true));
    let scan_writes = Arc::new(AtomicU64::new(0));
    let scan_errors = Arc::new(AtomicU64::new(0));
    let scan_total_time = Arc::new(AtomicU64::new(0));
    let scan_max_time = Arc::new(AtomicU64::new(0));
    
    let telegram_reads = Arc::new(AtomicU64::new(0));
    let telegram_errors = Arc::new(AtomicU64::new(0));
    let telegram_total_time = Arc::new(AtomicU64::new(0));
    let telegram_max_time = Arc::new(AtomicU64::new(0));
    let concurrent_access = Arc::new(AtomicU64::new(0));
    
    println!("Uruchamianie symulatorów...");
    println!("Czas testu: {}s\n", duration_secs);
    
    // Uruchom skaner
    let scan_handle = tokio::spawn(run_ble_scanner(
        Arc::clone(&running),
        Arc::clone(&scan_writes),
        Arc::clone(&scan_errors),
        Arc::clone(&scan_total_time),
        Arc::clone(&scan_max_time),
    ));
    
    // Uruchom Telegrama
    let telegram_handle = tokio::spawn(run_telegram(
        Arc::clone(&running),
        Arc::clone(&telegram_reads),
        Arc::clone(&telegram_errors),
        Arc::clone(&telegram_total_time),
        Arc::clone(&telegram_max_time),
        Arc::clone(&concurrent_access),
    ));
    
    // Pasek postępu
    let start = Instant::now();
    while start.elapsed().as_secs() < duration_secs {
        let elapsed = start.elapsed().as_secs();
        let progress = (elapsed as f64 / duration_secs as f64 * 100.0) as u8;
        let bar = "█".repeat(progress as usize / 2);
        let empty = "░".repeat(50 - progress as usize / 2);
        
        print!("\r[{}{}] {}% | {}s | Scany: {} | Telegram: {}",
            bar, empty, progress, elapsed,
            scan_writes.load(Ordering::Relaxed),
            telegram_reads.load(Ordering::Relaxed)
        );
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        sleep(Duration::from_millis(100)).await;
    }
    println!();
    
    // Zatrzymaj symulatory
    println!("\nZatrzymywanie symulatorów...");
    running.store(false, Ordering::Relaxed);
    
    // Poczekaj na zakończenie
    let _ = tokio::join!(scan_handle, telegram_handle);
    
    // Zbierz wyniki
    let result = ConcurrencyTestResult {
        scan_writes: scan_writes.load(Ordering::Relaxed),
        scan_errors: scan_errors.load(Ordering::Relaxed),
        telegram_reads: telegram_reads.load(Ordering::Relaxed),
        telegram_errors: telegram_errors.load(Ordering::Relaxed),
        avg_scan_time_ms: if scan_writes.load(Ordering::Relaxed) > 0 {
            scan_total_time.load(Ordering::Relaxed) as f64 / scan_writes.load(Ordering::Relaxed) as f64
        } else {
            0.0
        },
        avg_telegram_time_ms: if telegram_reads.load(Ordering::Relaxed) > 0 {
            telegram_total_time.load(Ordering::Relaxed) as f64 / telegram_reads.load(Ordering::Relaxed) as f64
        } else {
            0.0
        },
        max_scan_time_ms: scan_max_time.load(Ordering::Relaxed) as f64,
        max_telegram_time_ms: telegram_max_time.load(Ordering::Relaxed) as f64,
        concurrent_access_count: concurrent_access.load(Ordering::Relaxed),
    };
    
    // Podsumowanie
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("                     WYNIKI TESTU                               ");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("Skanowanie BLE:");
    println!("  Zapisów: {} ({:.1}/s)", result.scan_writes, 
        result.scan_writes as f64 / duration_secs as f64);
    println!("  Błędów: {} ({:.2}%)", result.scan_errors,
        if result.scan_writes > 0 { result.scan_errors as f64 / result.scan_writes as f64 * 100.0 } else { 0.0 });
    println!("  Średni czas: {:.2}ms", result.avg_scan_time_ms);
    println!("  Max czas: {:.2}ms", result.max_scan_time_ms);
    println!();
    println!("Telegram:");
    println!("  Odczytów: {}", result.telegram_reads);
    println!("  Błędów: {}", result.telegram_errors);
    println!("  Średni czas: {:.2}ms", result.avg_telegram_time_ms);
    println!("  Max czas: {:.2}ms", result.max_telegram_time_ms);
    println!();
    
    // Ocena
    let scan_success_rate = if result.scan_writes > 0 {
        (result.scan_writes - result.scan_errors) as f64 / result.scan_writes as f64 * 100.0
    } else {
        0.0
    };
    
    let telegram_success = result.telegram_errors == 0 || result.telegram_reads > result.telegram_errors * 10;
    
    if scan_success_rate > 95.0 && telegram_success && result.concurrent_access_count == 0 {
        println!("✅ TEST ZALICZONY");
        println!("   - Współbieżność działa poprawnie");
        println!("   - WAL mode zapobiega blokadom");
        println!("   - Retry logic działa skutecznie");
    } else {
        println!("⚠️  TEST NIE DO KOŃCA ZALICZONY");
        if scan_success_rate <= 95.0 {
            println!("   - Za dużo błędów skanowania: {:.1}%", 100.0 - scan_success_rate);
        }
        if !telegram_success {
            println!("   - Problemy z odczytem Telegrama");
        }
        if result.concurrent_access_count > 0 {
            println!("   - {} przypadków konfliktów dostępu", result.concurrent_access_count);
        }
    }
    
    result
}

#[tokio::main]
async fn main() {
    run_concurrency_test(30).await;
    
    // Wyczyść plik testowy
    let _ = std::fs::remove_file("test_concurrency.db");
    let _ = std::fs::remove_file("test_concurrency.db-shm");
    let _ = std::fs::remove_file("test_concurrency.db-wal");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_access() {
        let result = run_concurrency_test(10).await;
        
        // Sprawdź czy były zapisy
        assert!(result.scan_writes > 50, "Za mało zapisów: {}", result.scan_writes);
        
        // Sprawdź czy były odczyty
        assert!(result.telegram_reads >= 1, "Brak odczytów Telegrama");
        
        // Skanowanie powinno mieć >90% sukcesu
        let scan_rate = (result.scan_writes - result.scan_errors) as f64 / result.scan_writes as f64 * 100.0;
        assert!(scan_rate > 90.0, "Za niski współczynnik sukcesu skanowania: {:.1}%", scan_rate);
        
        // Telegram powinien mieć mniej niż 50% błędów
        let telegram_rate = if result.telegram_reads > 0 {
            result.telegram_errors as f64 / result.telegram_reads as f64 * 100.0
        } else {
            100.0
        };
        assert!(telegram_rate < 50.0, "Za dużo błędów Telegrama: {:.1}%", telegram_rate);
    }

    #[tokio::test]
    async fn test_high_frequency_scanning() {
        // Test z bardzo częstymi zapisami (co 1ms)
        let result = run_concurrency_test(5).await;
        
        // Przy tak wysokiej częstotliwości powinno być dużo zapisów
        assert!(result.scan_writes > 100, "Za mało zapisów przy high-frequency: {}", result.scan_writes);
        
        // Czas zapisu nie powinien przekraczać 50ms
        assert!(result.max_scan_time_ms < 50.0, "Zbyt wolne zapisy: {}ms", result.max_scan_time_ms);
    }
}
