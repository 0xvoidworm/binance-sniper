$ErrorActionPreference='Stop'
$root = Split-Path $PSScriptRoot -Parent
$python = Join-Path $root '.venv\Scripts\python.exe'
if (-not (Test-Path $python)) { throw 'Virtual environment missing. Run setup.ps1 first.' }
$port = Get-NetTCPConnection -State Listen -LocalPort 8765 -ErrorAction SilentlyContinue
if ($port) {
	$owner = Get-CimInstance Win32_Process -Filter "ProcessId = $($port.OwningProcess)"
	if ($owner.CommandLine -notmatch 'binance-sniper.*uvicorn binance_sniper\.app:app') { throw "Port 8765 is owned by another process; refusing to stop or reuse it." }
	throw "Binance Sniper already owns port 8765 (PID $($port.OwningProcess))."
}
Push-Location $root
try { & $python -m uvicorn binance_sniper.app:app --host 127.0.0.1 --port 8765 } finally { Pop-Location }
