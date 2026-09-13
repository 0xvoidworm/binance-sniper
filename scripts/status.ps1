$url='http://127.0.0.1:8765/api/state'; try { Invoke-RestMethod $url | ConvertTo-Json -Depth 6 } catch { Write-Output 'Binance Sniper is not reachable on port 8765.'; exit 1 }
