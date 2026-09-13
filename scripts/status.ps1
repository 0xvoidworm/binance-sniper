$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$pidFile = Join-Path $root 'state\binance-sniper.pid'
try {
    $state = Invoke-RestMethod 'http://127.0.0.1:8765/api/state'
    $account = Invoke-RestMethod 'http://127.0.0.1:8765/api/account'
    $port = @(Get-NetTCPConnection -State Listen -LocalPort 8765)
    $ownerPid = $port[0].OwningProcess
    $process = Get-CimInstance Win32_Process -Filter "ProcessId = $ownerPid"
    $pidMatches = (Test-Path $pidFile) -and ([int](Get-Content $pidFile) -eq $ownerPid)
    $owned = $pidMatches -and $process.CommandLine -match '-m\s+uvicorn\s+binance_sniper\.app:app' -and $process.CommandLine.Contains($root) -and $process.CommandLine -match '--host\s+127\.0\.0\.1' -and $process.CommandLine -match '--port\s+8765'
    $feeds = @($state.feed_quality.PSObject.Properties.Value)
    [pscustomobject]@{
        PID = $ownerPid
        Ownership = $owned
        Transport = $state.transport_state
        WsState = $state.ws_state
        LastWsPayload = $state.last_ws_payload
        RestFallbackFeeds = @($feeds | Where-Object transport -eq 'LIVE_REST_FALLBACK').Count
        LiveWsFeeds = @($feeds | Where-Object transport -eq 'LIVE_WS').Count
        StaleFeeds = @($feeds | Where-Object transport -eq 'STALE').Count
        MaxSuccessAgeMs = ($feeds | Measure-Object success_age_ms -Maximum).Maximum
        MaxPreviewAgeMs = ($feeds | Measure-Object preview_age_ms -Maximum).Maximum
        MaxLagBars = ($feeds | Measure-Object lag_bars -Maximum).Maximum
        HealthyTasks = @($state.task_health.PSObject.Properties.Value | Where-Object status -eq 'RUNNING').Count
        TotalTasks = @($state.task_health.PSObject.Properties.Value).Count
        UsedWeight1m = $state.rest_telemetry.used_weight_1m
        WeightLimit1m = $state.rest_telemetry.weight_limit_1m
        Account = $account.auth
        SnapshotAgeMs = $account.snapshot_age_ms
        RegularOrders = $account.regular_orders
        ConditionalOrders = $account.conditional_orders
        ExecutionAlgoOrders = $account.execution_algo_orders
        TotalOrders = $account.total_orders
    } | ConvertTo-Json -Depth 4
} catch {
    Write-Output "Binance Sniper status failed: $($_.Exception.Message)"
    exit 1
}
