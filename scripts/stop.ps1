Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -match 'binance_sniper.app:app' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
