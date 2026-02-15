use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let bot_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let chat_id = env::var("TELEGRAM_CHAT_ID").expect("TELEGRAM_CHAT_ID not set");

    println!("🧪 Testowanie konfiguracji Telegrama...");
    println!(
        "Bot Token: {}",
        if bot_token.is_empty() {
            "❌ BRAK"
        } else {
            "✅ USTAWIONY"
        }
    );
    println!(
        "Chat ID: {}",
        if chat_id.is_empty() {
            "❌ BRAK"
        } else {
            "✅ USTAWIONY"
        }
    );

    let message = "🧪 <b>TEST WIADOMOŚCI</b>\n\nSkanowanie Bluetooth jest gotowe do wysyłania notyfikacji! ✅";

    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

    let client = reqwest::Client::new();
    let params = serde_json::json!({
        "chat_id": chat_id,
        "text": message,
        "parse_mode": "HTML"
    });

    println!("\n📤 Wysyłanie testowej wiadomości...");

    match client.post(&url).json(&params).send().await {
        Ok(response) => {
            if response.status().is_success() {
                println!("✅ Wiadomość wysłana pomyślnie!");
            } else {
                println!("❌ Błąd API: {}", response.status());
                if let Ok(body) = response.text().await {
                    println!("Szczegóły: {}", body);
                }
            }
        }
        Err(e) => {
            println!("❌ Błąd połączenia: {}", e);
        }
    }
}
