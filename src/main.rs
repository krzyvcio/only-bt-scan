//! Entry point for the only-bt-scan application.
//!
//! Initializes the runtime environment and launches the main application.

use dotenv::dotenv;
use only_bt_scan::run;
use std::env;
use std::path::PathBuf;

#[tokio::main]
/// Application entry point.
/// 
/// Loads environment variables from .env files and starts the async runtime.
/// Searches for .env in the following order:
/// 1. Project root (next to executable)
/// 2. Current working directory
/// 
/// Sets default RUST_LOG to "info" if not already configured.
/// 
/// # Returns
/// Result<(), anyhow::Error> - Ok on successful exit, Error on failure
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
