# Build and Run only-bt-scan
# Kills existing process, runs cargo clean if needed, then builds and runs

Remove-Item Env:TELEGRAM_BOT_TOKEN -ErrorAction SilentlyContinue
Remove-Item Env:TELEGRAM_CHAT_ID -ErrorAction SilentlyContinue
Remove-Item Env:RUST_LOG -ErrorAction SilentlyContinue

$projectDir = "C:\projekty\only-bt-scan"
$exePath = "$projectDir\target\debug\only-bt-scan.exe"
$exeName = "only-bt-scan"

# Kill any running process
$runningProcess = Get-Process -Name $exeName -ErrorAction SilentlyContinue
if ($runningProcess) {
    Write-Host "Killing existing process (PID: $($runningProcess.Id))..." -ForegroundColor Yellow
    Stop-Process -Id $runningProcess.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 3
}

Push-Location $projectDir

# Try to build, if fails due to locked file, run cargo clean and retry
Write-Host "Building project..." -ForegroundColor Cyan

$buildSuccess = $false
for ($i = 0; $i -lt 3; $i++) {
    cargo build 2>&1
    if ($LASTEXITCODE -eq 0) {
        $buildSuccess = $true
        break
    }
    
    # Check if error is due to locked file
    $buildOutput = cargo build 2>&1 | Out-String
    if ($buildOutput -match "failed to remove") {
        Write-Host "File locked, running cargo clean and retrying..." -ForegroundColor Yellow
        cargo clean
        Start-Sleep -Seconds 2
    }
    else {
        break
    }
}

if (-not $buildSuccess) {
    Write-Host "Build failed!" -ForegroundColor Red
    Pop-Location
    exit 1
}

Write-Host "Build successful! Running..." -ForegroundColor Green
.\target\debug\$exeName.exe
Pop-Location
