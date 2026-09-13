$ErrorActionPreference='Stop'
$root = Split-Path $PSScriptRoot -Parent
$python = Join-Path $root '.venv\Scripts\python.exe'
if (-not (Test-Path $python)) { throw 'Virtual environment missing. Run setup.ps1 first.' }
Push-Location $root
try { & $python -m uvicorn binance_sniper.app:app --host 127.0.0.1 --port 8765 } finally { Pop-Location }
