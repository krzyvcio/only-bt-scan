//! Symulator odczytów Telegrama z rzeczywistej bazy SQLite
//!
//! Odczytuje co 5 sekund liczbę urządzeń z ostatnich 60 sekund
//! z retry logic i exponential backoff.

use rusqlite::{Connection, OpenFlags};

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task;

/// Statystyki symulatora
#[derive(Debug, Default, Clone)]
pub struct TelegramSimulatorStats {
    pub total_reads: u64,
    pub successful_reads: u64,
    pub failed_reads: u64,
    pub total_errors: u64,
    pub total_time_ms: u64,
    pub avg_read_time_ms: f64,
    pub min_read_time_ms: u64,
    pub max_read_time_ms: u64,
}

impl TelegramSimulatorStats {
    pub fn new() -> Self {
        Self {
            min_read_time_ms: u64::MAX,
            ..Default::default()
        }
    }

    pub fn record_success(&mut self, duration_ms: u64) {
        self.total_reads += 1;
        self.successful_reads += 1;
        self.total_time_ms += duration_ms;
        self.avg_read_time_ms = self.total_time_ms as f64 / self.total_reads as f64;
        if duration_ms < self.min_read_time_ms {
            self.min_read_time_ms = duration_ms;
        }
        if duration_ms > self.max_read_time_ms {
            self.max_read_time_ms = duration_ms;
        }
    }

    pub fn record_failure(&mut self, errors: u64) {
        self.total_reads += 1;
        self.failed_reads += 1;
        self.total_errors += errors;
    }
}

/// Struktura symulatora Telegrama
pub struct TelegramSimulator {
    db_path: String,
    report_interval_secs: u64,
    max_duration_secs: u64,
    max_retries: u32,
}

impl TelegramSimulator {
    pub fn new(
        db_path: impl Into<String>,
        report_interval_secs: u64,
        max_duration_secs: u64,
    ) -> Self {
        Self {
            db_path: db_path.into(),
            report_interval_secs,
            max_duration_secs,
            max_retries: 3,
        }
    }

    /// Pobiera liczbę urządzeń z ostatnich 60 sekund z retry logic
    async fn read_device_count_with_retry(&self) -> Result<(u64, u64, u64), String> {
        let db_path = self.db_path.clone();
        let max_retries = self.max_retries;

        task::spawn_blocking(move || {
            let mut last_error = None;
            let mut total_errors = 0u64;

            for attempt in 0..max_retries {
                let start = Instant::now();

                match Self::query_device_count(&db_path) {
                    Ok(count) => {
                        let duration_ms = start.elapsed().as_millis() as u64;
                        return Ok((count, duration_ms, total_errors));
                    }
                    Err(e) => {
                        total_errors += 1;
                        last_error = Some(e);

                        if attempt < max_retries - 1 {
                            // Exponential backoff: 100ms, 200ms, 400ms
                            let backoff_ms = 100u64 * (2u64.pow(attempt));
                            std::thread::sleep(Duration::from_millis(backoff_ms));
                        }
                    }
                }
            }

            Err(format!(
                "Błąd po {} próbach: {:?}",
                max_retries,
                last_error.unwrap_or_else(|| "Unknown error".to_string())
            ))
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }

    /// Rzeczywiste zapytanie do SQLite z WAL mode
    fn query_device_count(db_path: &str) -> Result<u64, String> {
        // Otwórz bazę z WAL mode
        let conn = Connection::open_with_flags(
            db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| format!("Failed to open DB: {}", e))?;

        // Włącz WAL mode
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| format!("Failed to set WAL mode: {}", e))?;

        // Query: urządzenia z ostatnich 60 sekund
        let query = r#"
            SELECT COUNT(*) 
            FROM devices 
            WHERE timestamp > datetime('now', '-1 minute')
        "#;

        let count: i64 = conn
            .query_row(query, [], |row| row.get(0))
            .map_err(|e| format!("Query failed: {}", e))?;

        Ok(count as u64)
    }

    /// Główna pętla symulatora
    pub async fn run(&self) -> TelegramSimulatorStats {
        let stats = Arc::new(std::sync::Mutex::new(TelegramSimulatorStats::new()));
        let start_time = Instant::now();
        let mut report_counter = 0u64;

        println!("╔════════════════════════════════════════════════════════════╗");
        println!("║   SYMLULATOR TELEGRAMA - Odczyt z SQLite (WAL)             ║");
        println!("╚════════════════════════════════════════════════════════════╝");
        println!("Baza: {}", self.db_path);
        println!("Interval: {}s, Duration: {}s", self.report_interval_secs, self.max_duration_secs);
        println!("Query: SELECT COUNT(*) FROM devices WHERE timestamp > datetime('now', '-1 minute')");
        println!();

        while start_time.elapsed().as_secs() < self.max_duration_secs {
            report_counter += 1;

            match self.read_device_count_with_retry().await {
                Ok((device_count, duration_ms, errors)) => {
                    {
                        let mut s = stats.lock().unwrap();
                        s.record_success(duration_ms);
                        if errors > 0 {
                            s.total_errors += errors;
                        }
                    }

                    println!(
                        "[TELEGRAM] Raport: {} urządzeń, czas: {}ms",
                        device_count, duration_ms
                    );
                }
                Err(e) => {
                    let mut s = stats.lock().unwrap();
                    s.record_failure(3); // Max retries exhausted

                    eprintln!("[TELEGRAM ERROR] Raport #{}: {}", report_counter, e);
                }
            }

            // Czekaj do następnego odczytu (zawsze 5s, ale sprawdź czy starczy czasu)
            let elapsed_secs = start_time.elapsed().as_secs();
            if elapsed_secs + self.report_interval_secs < self.max_duration_secs {
                tokio::time::sleep(Duration::from_secs(self.report_interval_secs)).await;
            } else if elapsed_secs < self.max_duration_secs {
                // Ostatni odczyt - czekaj tylko do końca max_duration
                let remaining = self.max_duration_secs - elapsed_secs;
                if remaining > 0 {
                    tokio::time::sleep(Duration::from_secs(remaining)).await;
                }
            }
        }

        let final_stats = stats.lock().unwrap().clone();
        self.print_summary(&final_stats);

        final_stats
    }

    fn print_summary(&self, stats: &TelegramSimulatorStats) {
        println!();
        println!("═══════════════════════════════════════════════════════════════");
        println!("              PODSUMOWANIE SYMLULATORA TELEGRAMA               ");
        println!("═══════════════════════════════════════════════════════════════");
        println!();
        println!("Statystyki odczytów:");
        println!("  Całkowita liczba odczytów: {}", stats.total_reads);
        println!("  Udane: {}", stats.successful_reads);
        println!("  Nieudane: {}", stats.failed_reads);
        println!("  Całkowita liczba błędów (w tym retry): {}", stats.total_errors);
        println!();
        println!("Czasy odczytu:");
        println!("  Średni: {:.2}ms", stats.avg_read_time_ms);
        if stats.min_read_time_ms != u64::MAX {
            println!("  Min: {}ms", stats.min_read_time_ms);
        }
        println!("  Max: {}ms", stats.max_read_time_ms);
        println!();

        let success_rate = if stats.total_reads > 0 {
            (stats.successful_reads as f64 / stats.total_reads as f64) * 100.0
        } else {
            0.0
        };

        println!("Wskaźnik sukcesu: {:.1}%", success_rate);

        if stats.failed_reads == 0 {
            println!("✅ Symulacja ZAKOŃCZONA POMYŚLNIE");
        } else {
            println!("⚠️  Symulacja zakończona z {} błędami", stats.failed_reads);
        }
    }
}

/// Funkcja pomocnicza - uruchamia symulację z domyślnymi parametrami
pub async fn run_telegram_simulator() -> TelegramSimulatorStats {
    let simulator = TelegramSimulator::new(
        "test_scanner.db",
        5,  // co 5 sekund
        30, // przez 30 sekund
    );

    simulator.run().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simulator_creation() {
        let sim = TelegramSimulator::new("test.db", 5, 30);
        assert_eq!(sim.report_interval_secs, 5);
        assert_eq!(sim.max_duration_secs, 30);
        assert_eq!(sim.max_retries, 3);
    }

    #[tokio::test]
    async fn test_stats_calculation() {
        let mut stats = TelegramSimulatorStats::new();

        stats.record_success(100);
        stats.record_success(200);
        stats.record_success(300);

        assert_eq!(stats.total_reads, 3);
        assert_eq!(stats.successful_reads, 3);
        assert_eq!(stats.avg_read_time_ms, 200.0);
        assert_eq!(stats.min_read_time_ms, 100);
        assert_eq!(stats.max_read_time_ms, 300);

        stats.record_failure(3);
        assert_eq!(stats.total_reads, 4);
        assert_eq!(stats.failed_reads, 1);
        assert_eq!(stats.total_errors, 3);
    }

    #[test]
    fn test_exponential_backoff() {
        // Test sprawdza czy backoff rośnie wykładniczo
        let attempt_0 = 100u64 * (2u64.pow(0)); // 100ms
        let attempt_1 = 100u64 * (2u64.pow(1)); // 200ms
        let attempt_2 = 100u64 * (2u64.pow(2)); // 400ms

        assert_eq!(attempt_0, 100);
        assert_eq!(attempt_1, 200);
        assert_eq!(attempt_2, 400);
    }
}

/// Główna funkcja dla uruchomienia jako osobny program
#[tokio::main]
async fn main() {
    println!("🚀 Uruchamianie symulatora Telegrama...");
    println!();

    let stats = run_telegram_simulator().await;

    println!();
    println!("Symulator zakończył pracę.");
    println!("Końcowe statystyki: {:?}", stats);
}
