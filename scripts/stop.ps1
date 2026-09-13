$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$pidFile = Join-Path $root 'state\binance-sniper.pid'
if (-not (Test-Path $pidFile)) { Write-Output 'No project PID file.'; exit 0 }
$targetPid = [int](Get-Content $pidFile)
$process = Get-CimInstance Win32_Process -Filter "ProcessId = $targetPid" -ErrorAction SilentlyContinue
$port = @(Get-NetTCPConnection -State Listen -LocalPort 8765 -ErrorAction SilentlyContinue | Where-Object OwningProcess -eq $targetPid)
$owned = $process -and $port.Count -gt 0 -and $process.CommandLine -match '-m\s+uvicorn\s+binance_sniper\.app:app' -and $process.CommandLine.Contains($root) -and $process.CommandLine -match '--host\s+127\.0\.0\.1' -and $process.CommandLine -match '--port\s+8765'
if (-not $owned) { throw 'PID ownership verification failed; refusing to stop process.' }
Stop-Process -Id $targetPid -ErrorAction Stop
try { Wait-Process -Id $targetPid -Timeout 8 -ErrorAction Stop } catch { if (Get-Process -Id $targetPid -ErrorAction SilentlyContinue) { Stop-Process -Id $targetPid -Force -ErrorAction Stop } }
foreach ($path in @($pidFile, (Join-Path $root 'state\binance-sniper.lock'))) {
    if ((Test-Path $path) -and ([int](Get-Content $path) -eq $targetPid)) { Remove-Item -LiteralPath $path -Force }
}
Write-Output "Stopped verified Binance Sniper PID $targetPid."
