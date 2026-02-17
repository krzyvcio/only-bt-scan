use std::collections::HashMap;
use std::env;

static COMPILED_ENV: std::sync::LazyLock<HashMap<String, String>> =
    std::sync::LazyLock::new(|| {
        let content = include_str!("../.env");
        let mut map = HashMap::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                map.insert(key, value);
            }
        }

        map
    });

pub fn init() {
    // Always set all compiled env vars (override system env)
    for (key, value) in COMPILED_ENV.iter() {
        env::set_var(key, value);
    }

    // Set defaults if not set
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    eprintln!(
        "[ENV] Loaded {} variables from compiled .env",
        COMPILED_ENV.len()
    );
    eprintln!(
        "[ENV] TELEGRAM_BOT_TOKEN={}",
        if env::var("TELEGRAM_BOT_TOKEN")
            .map(|s| s.is_empty())
            .unwrap_or(true)
        {
            "EMPTY"
        } else {
            "SET"
        }
    );
}

pub fn get(key: &str) -> Option<&str> {
    COMPILED_ENV.get(key).map(|s| s.as_str())
}
