//! Entry point aplikacji only-bt-scan
//!
//! Punkt wejścia - inicjalizuje środowisko i uruchamia główną pętlę aplikacji

use dotenv::dotenv;
use only_bt_scan::run;
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Znajdź katalog główny projektu i załaduj .env
    let exe_path = std::env::current_exe().unwrap_or_default();
    let project_root = exe_path
        .parent()
        .and_then(|p| p.parent()) // target/debug lub target/release
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let env_path = project_root.join(".env");
    if env_path.exists() {
        dotenv::from_path(&env_path).ok();
    }

    // Spróbuj też domyślnej lokalizacji
    dotenv().ok();

    // Ustaw domyślny poziom logowania jeśli nie jest ustawiony
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    run().await
}
