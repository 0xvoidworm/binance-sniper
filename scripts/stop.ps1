$root = Split-Path $PSScriptRoot -Parent
$pidFile = Join-Path $root 'state\binance-sniper.pid'
if (-not (Test-Path $pidFile)) { Write-Output 'No project PID file.'; exit 0 }
$pid = [int](Get-Content $pidFile)
$process = Get-CimInstance Win32_Process -Filter "ProcessId = $pid" -ErrorAction SilentlyContinue
$port = Get-NetTCPConnection -State Listen -LocalPort 8765 -ErrorAction SilentlyContinue
if (-not $process -or -not $port -or $port.OwningProcess -ne $pid -or $process.CommandLine -notmatch 'binance-sniper.*uvicorn binance_sniper\.app:app') { throw 'PID ownership verification failed; refusing to stop process.' }
Stop-Process -Id $pid -Force
