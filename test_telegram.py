#!/usr/bin/env python3
import os
import requests
from dotenv import load_dotenv

load_dotenv()

bot_token = os.getenv("TELEGRAM_BOT_TOKEN", "").strip()
chat_id = os.getenv("TELEGRAM_CHAT_ID", "").strip()

print("🧪 Testowanie konfiguracji Telegrama...")
print(f"Bot Token: {'✅ USTAWIONY' if bot_token else '❌ BRAK'}")
print(f"Chat ID: {'✅ USTAWIONY' if chat_id else '❌ BRAK'}")

if not bot_token or not chat_id:
    print("\n❌ Brakuje konfiguracji!")
    exit(1)

message = """🧪 <b>TEST WIADOMOŚCI</b>

Skanowanie Bluetooth jest gotowe do wysyłania notyfikacji! ✅"""

url = f"https://api.telegram.org/bot{bot_token}/sendMessage"

params = {
    "chat_id": chat_id,
    "text": message,
    "parse_mode": "HTML"
}

print("\n📤 Wysyłanie testowej wiadomości...")

try:
    response = requests.post(url, json=params, timeout=10)

    if response.status_code == 200:
        print("✅ Wiadomość wysłana pomyślnie!")
        print(f"Response: {response.json()}")
    else:
        print(f"❌ Błąd API: {response.status_code}")
        print(f"Szczegóły: {response.text}")
except Exception as e:
    print(f"❌ Błąd połączenia: {e}")
