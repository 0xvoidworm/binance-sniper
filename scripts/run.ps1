$ErrorActionPreference='Stop'
$root = Split-Path $PSScriptRoot -Parent
$python = Join-Path $root '.venv\Scripts\python.exe'
if (-not (Test-Path $python)) { throw 'Virtual environment missing. Run setup.ps1 first.' }
$port = Get-NetTCPConnection -State Listen -LocalPort 8765 -ErrorAction SilentlyContinue
if ($port) { throw "Binance Sniper already owns port 8765 (PID $($port.OwningProcess)). Use scripts\status.ps1 or scripts\stop.ps1." }
Push-Location $root
try { & $python -m uvicorn binance_sniper.app:app --host 127.0.0.1 --port 8765 } finally { Pop-Location }
