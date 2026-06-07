# Shadow Production Daily Log Script
# Usage: Run daily after market close to record states
# Phase A (Days 1-30): State Layer only
# Phase B (Days 31-60): + Economic Layer
# Phase C (Days 61-90): + Allocation suggestions

param(
    [Parameter()]
    [ValidateSet("A","B","C")]
    [string]$Phase = "A",

    [Parameter()]
    [string]$LogDir = "$PSScriptRoot"
)

$date = Get-Date -Format "yyyy-MM-dd"
$logFile = Join-Path $LogDir "shadow-log-$date.json"

Write-Host "=== Shadow Production Daily Log ($Phase) ===" -ForegroundColor Cyan
Write-Host "Date: $date" -ForegroundColor Cyan
Write-Host ""

# Function to get dashboard snapshot
function Get-DashboardSnapshot($scope) {
    try {
        $result = cargo run --release -p quant-cli -- dashboard-snapshot --scope $scope --quiet 2>$null | ConvertFrom-Json
        return $result
    } catch {
        Write-Warning "Failed to get $scope dashboard snapshot: $_"
        return $null
    }
}

# Get CN State
Write-Host "Fetching CN State..." -NoNewline
$cn = Get-DashboardSnapshot "cn"
if ($cn) {
    $cnState = $cn.market_regime.state
    Write-Host " $cnState" -ForegroundColor Green
} else {
    $cnState = "ERROR"
    Write-Host " ERROR" -ForegroundColor Red
}

# Get HK State
Write-Host "Fetching HK State..." -NoNewline
$hk = Get-DashboardSnapshot "hk"
if ($hk) {
    $hkState = $hk.market_regime.state
    Write-Host " $hkState" -ForegroundColor Green
} else {
    $hkState = "ERROR"
    Write-Host " ERROR" -ForegroundColor Red
}

# Build log entry
$logEntry = @{
    date = $date
    phase = $Phase
    timestamp = (Get-Date -Format "yyyy-MM-ddTHH:mm:ss")
    state_layer = @{
        cn_state = $cnState
        hk_state = $hkState
    }
}

# Phase B/C: Add Economic Layer
if ($Phase -in @("B", "C")) {
    Write-Host "Fetching Economic State..." -NoNewline
    $global = Get-DashboardSnapshot "global"
    if ($global -and $global.environment) {
        $econScore = $global.environment.economic_score
        # Map score to state (using ADR-063 boundaries)
        if ($econScore -ge 61.2) { $econState = "Favorable" }
        elseif ($econScore -ge 37.5) { $econState = "Neutral" }
        else { $econState = "Unfavorable" }

        $logEntry.economic_layer = @{
            state = $econState
            score = $econScore
            factors = @{}  # TODO: Add factor contributions when available
        }
        Write-Host " $econState (score=$econScore)" -ForegroundColor Green
    } else {
        $logEntry.economic_layer = @{ state = "ERROR"; score = $null }
        Write-Host " ERROR" -ForegroundColor Red
    }
}

# Phase C: Add Allocation suggestion
if ($Phase -eq "C") {
    # Simple heuristic for demonstration
    # TODO: Replace with actual Allocation Layer when implemented
    $allocation = "Neutral"
    if ($cnState -eq "RiskOn" -and $econState -eq "Favorable") {
        $allocation = "Aggressive"
    } elseif ($cnState -eq "RiskOff" -or $econState -eq "Unfavorable") {
        $allocation = "Conservative"
    }

    $logEntry.allocation = @{
        suggestion = $allocation
        confidence = 50  # Placeholder
        note = "Phase C prototype - human judgment required"
    }
    Write-Host "Suggested Allocation: $allocation" -ForegroundColor Yellow
}

# Save log
$logEntry | ConvertTo-Json -Depth 4 | Out-File $logFile -Encoding UTF8
Write-Host ""
Write-Host "Log saved to: $logFile" -ForegroundColor Cyan

# Also append to master CSV
$csvFile = Join-Path $LogDir "shadow-master.csv"
$csvExists = Test-Path $csvFile

$csvEntry = [PSCustomObject]@{
    Date = $date
    Phase = $Phase
    CN_State = $cnState
    HK_State = $hkState
    Economic_State = if ($logEntry.economic_layer) { $logEntry.economic_layer.state } else { "" }
    Economic_Score = if ($logEntry.economic_layer) { $logEntry.economic_layer.score } else { "" }
    Allocation = if ($logEntry.allocation) { $logEntry.allocation.suggestion } else { "" }
    T20_Return = ""   # To be filled after 20 days
    T60_Return = ""   # To be filled after 60 days
    T120_Return = ""  # To be filled after 120 days
}

if (-not $csvExists) {
    $csvEntry | Export-Csv $csvFile -NoTypeInformation -Encoding UTF8
} else {
    $csvEntry | Export-Csv $csvFile -NoTypeInformation -Encoding UTF8 -Append
}

Write-Host "Master CSV updated: $csvFile" -ForegroundColor Cyan
Write-Host ""
Write-Host "=== Next Steps ===" -ForegroundColor Yellow
Write-Host "1. Verify states are correct" -ForegroundColor Gray
Write-Host "2. Check trust_summary for data health" -ForegroundColor Gray
Write-Host "3. Weekly: Review state transitions and persistence" -ForegroundColor Gray
Write-Host "4. Monthly: Export report and analyze" -ForegroundColor Gray
