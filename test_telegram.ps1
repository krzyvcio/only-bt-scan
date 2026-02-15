# Load .env file
$envContent = Get-Content .env -Raw
$envContent | ForEach-Object {
    if ($_ -match '^\s*([^#=]\w+)=(.*)$') {
        $key = $matches[1].Trim()
        $value = $matches[2].Trim()
        if ($key -match 'TELEGRAM') {
            [Environment]::SetEnvironmentVariable($key, $value, "Process")
        }
    }
}

$botToken = [Environment]::GetEnvironmentVariable('TELEGRAM_BOT_TOKEN', 'Process')
$chatId = [Environment]::GetEnvironmentVariable('TELEGRAM_CHAT_ID', 'Process')

Write-Host "Testowanie konfiguracji Telegrama..." -ForegroundColor Cyan
Write-Host "Bot Token: $(if ($botToken) { 'OK' } else { 'BRAK' })" -ForegroundColor Yellow
Write-Host "Chat ID: $(if ($chatId) { 'OK' } else { 'BRAK' })" -ForegroundColor Yellow

if (-not $botToken -or -not $chatId) {
    Write-Host "Brakuje konfiguracji!" -ForegroundColor Red
    exit 1
}

$message = "TEST WIADOMOSCI - Skanowanie Bluetooth gotowe do wysylania notyfikacji!"

$url = "https://api.telegram.org/bot$botToken/sendMessage"

$body = @{
    chat_id = $chatId
    text = $message
    parse_mode = "HTML"
} | ConvertTo-Json

Write-Host "Wysylanie testowej wiadomosci..." -ForegroundColor Cyan

try {
    $response = Invoke-WebRequest -Uri $url -Method Post -ContentType "application/json" -Body $body -TimeoutSec 10 -ErrorAction Stop

    if ($response.StatusCode -eq 200) {
        Write-Host "OK - Wiadomosc wysłana!" -ForegroundColor Green
    }
} catch {
    Write-Host "Blad: $_" -ForegroundColor Red
    exit 1
}
