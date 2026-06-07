# Weekly Review Dashboard Script
# Usage: Run every Sunday after market close
# Generates weekly summary report for Shadow Production monitoring

param(
    [Parameter()]
    [string]$LogDir = "$PSScriptRoot",

    [Parameter()]
    [string]$ReportDir = "$PSScriptRoot\reports"
)

# Ensure report directory exists
if (-not (Test-Path $ReportDir)) {
    New-Item -ItemType Directory -Path $ReportDir -Force | Out-Null
}

$weekEnd = Get-Date -Format "yyyy-MM-dd"
$weekStart = (Get-Date).AddDays(-6).ToString("yyyy-MM-dd")
$reportFile = Join-Path $ReportDir "weekly-report-$weekEnd.md"

Write-Host "=== Weekly Review Dashboard ===" -ForegroundColor Cyan
Write-Host "Week: $weekStart to $weekEnd" -ForegroundColor Cyan
Write-Host ""

# Read master CSV
$csvFile = Join-Path $LogDir "shadow-master.csv"
if (-not (Test-Path $csvFile)) {
    Write-Error "No master log found. Run daily-log.ps1 first."
    exit 1
}

$logs = Import-Csv $csvFile | Where-Object {
    $_.Date -ge $weekStart -and $_.Date -le $weekEnd
}

if ($logs.Count -eq 0) {
    Write-Error "No logs found for week $weekStart to $weekEnd"
    exit 1
}

Write-Host "Processing $($logs.Count) days of data..." -ForegroundColor Green

# ========== STATE LAYER METRICS ==========

$cnStates = $logs | Group-Object CN_State
$hkStates = $logs | Group-Object HK_State

$cnRiskOffPct = ($cnStates | Where-Object Name -eq "RiskOff" | Select-Object -Expand Count) / $logs.Count * 100
$hkRiskOffPct = ($hkStates | Where-Object Name -eq "RiskOff" | Select-Object -Expand Count) / $logs.Count * 100

# State transitions
$cnTransitions = 0
$hkTransitions = 0
for ($i = 1; $i -lt $logs.Count; $i++) {
    if ($logs[$i].CN_State -ne $logs[$i-1].CN_State) { $cnTransitions++ }
    if ($logs[$i].HK_State -ne $logs[$i-1].HK_State) { $hkTransitions++ }
}

$cnTransitionRate = $cnTransitions / ($logs.Count - 1) * 100
$hkTransitionRate = $hkTransitions / ($logs.Count - 1) * 100

# State persistence (consecutive days in same state)
function Get-AvgPersistence($states) {
    if ($states.Count -eq 0) { return 0 }
    $runs = @()
    $currentRun = 1
    for ($i = 1; $i -lt $states.Count; $i++) {
        if ($states[$i] -eq $states[$i-1]) {
            $currentRun++
        } else {
            $runs += $currentRun
            $currentRun = 1
        }
    }
    $runs += $currentRun
    if ($runs.Count -eq 0) { return 0 }
    return ($runs | Measure-Object -Average).Average
}

$cnPersistence = Get-AvgPersistence $logs.CN_State
$hkPersistence = Get-AvgPersistence $logs.HK_State

# ========== ECONOMIC LAYER METRICS ==========

$econLogs = $logs | Where-Object { $_.Economic_State -ne "" }
$econStates = $econLogs | Group-Object Economic_State

$favorablePct = 0
$neutralPct = 0
$unfavorablePct = 0
if ($econLogs.Count -gt 0) {
    $favorablePct = ($econStates | Where-Object Name -eq "Favorable" | Select-Object -Expand Count) / $econLogs.Count * 100
    $neutralPct = ($econStates | Where-Object Name -eq "Neutral" | Select-Object -Expand Count) / $econLogs.Count * 100
    $unfavorablePct = ($econStates | Where-Object Name -eq "Unfavorable" | Select-Object -Expand Count) / $econLogs.Count * 100
}

# PSI calculation (vs baseline from ADR-063)
$baseline = @{ Favorable = 37.4; Neutral = 40.3; Unfavorable = 22.4 }
$psi = 0
foreach ($state in @("Favorable", "Neutral", "Unfavorable")) {
    $actual = if ($state -eq "Favorable") { $favorablePct } elseif ($state -eq "Neutral") { $neutralPct } else { $unfavorablePct }
    $base = $baseline[$state]
    if ($base -gt 0) {
        $psi += [Math]::Abs($actual - $base) / 100 * [Math]::Log($actual / $base)
    }
}
$psi = [Math]::Abs($psi)

# ========== ALLOCATION LAYER METRICS ==========

$allocLogs = $logs | Where-Object { $_.Allocation -ne "" }
$allocations = $allocLogs | Group-Object Allocation

# ========== GENERATE REPORT ==========

$report = @"
# Shadow Production Weekly Report

**Week:** $weekStart to $weekEnd  
**Generated:** $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")  
**Days Logged:** $($logs.Count)

---

## State Layer

### CN Market
| Metric | Value | Baseline | Status |
|--------|-------|----------|--------|
| RiskOff Coverage | $($cnRiskOffPct.ToString("F1"))% | ~45% | $(if ($cnRiskOffPct -gt 80) { "🔴 KILL S1" } elseif ($cnRiskOffPct -gt 60) { "🟡 WARN" } else { "🟢 OK" }) |
| Transition Rate | $($cnTransitionRate.ToString("F1"))% | ~30% | $(if ($cnTransitionRate -gt 50) { "🔴 KILL S3" } elseif ($cnTransitionRate -gt 40) { "🟡 WARN" } else { "🟢 OK" }) |
| Avg Persistence | $($cnPersistence.ToString("F1"))d | ~2.0d | $(if ($cnPersistence -lt 1.5) { "🔴 KILL S2" } elseif ($cnPersistence -lt 1.8) { "🟡 WARN" } else { "🟢 OK" }) |

### HK Market
| Metric | Value | Baseline | Status |
|--------|-------|----------|--------|
| RiskOff Coverage | $($hkRiskOffPct.ToString("F1"))% | ~28% | $(if ($hkRiskOffPct -gt 80) { "🔴 KILL S1" } elseif ($hkRiskOffPct -gt 50) { "🟡 WARN" } else { "🟢 OK" }) |
| Transition Rate | $($hkTransitionRate.ToString("F1"))% | ~25% | $(if ($hkTransitionRate -gt 50) { "🔴 KILL S3" } elseif ($hkTransitionRate -gt 40) { "🟡 WARN" } else { "🟢 OK" }) |
| Avg Persistence | $($hkPersistence.ToString("F1"))d | ~3.0d | $(if ($hkPersistence -lt 1.5) { "🔴 KILL S2" } elseif ($hkPersistence -lt 2.0) { "🟡 WARN" } else { "🟢 OK" }) |

### State Distribution
| State | CN Days | HK Days |
|-------|---------|---------|
$(foreach ($s in $cnStates) { "| $($s.Name) | $($s.Count) | $(($hkStates | Where-Object Name -eq $s.Name | Select-Object -Expand Count)) |`n" })

---

## Economic Layer

$(if ($econLogs.Count -gt 0) {
@"
### State Distribution
| State | This Week | Baseline | Drift |
|-------|-----------|----------|-------|
| Favorable | $($favorablePct.ToString("F1"))% | 37.4% | $($($favorablePct - 37.4).ToString("F1"))pp |
| Neutral | $($neutralPct.ToString("F1"))% | 40.3% | $($($neutralPct - 40.3).ToString("F1"))pp |
| Unfavorable | $($unfavorablePct.ToString("F1"))% | 22.4% | $($($unfavorablePct - 22.4).ToString("F1"))pp |

### PSI (Population Stability Index)
**PSI:** $($psi.ToString("F3"))  
**Status:** $(if ($psi -gt 0.25) { "🔴 KILL E2 (PSI > 0.25)" } elseif ($psi -gt 0.1) { "🟡 WARN (PSI 0.1-0.25)" } else { "🟢 OK (PSI < 0.1)" })

"@
} else {
"### Economic Layer not yet active (Phase A)"
})

---

## Allocation Layer

$(if ($allocLogs.Count -gt 0) {
@"
### Allocation Distribution
| Allocation | Days | Percentage |
|------------|------|------------|
$(foreach ($a in $allocations) { "| $($a.Name) | $($a.Count) | $(($a.Count / $allocLogs.Count * 100).ToString("F1"))% |`n" })

### Paper Portfolio (if tracked)
_Requires T+20/T+60/T+120 returns to be filled in shadow-master.csv_
"@
} else {
"### Allocation Layer not yet active (Phase A/B)"
})

---

## Kill Criteria Check

| Criterion | Triggered? | Notes |
|-----------|-----------|-------|
| S1: RiskOff > 80% (30d) | $(if ($cnRiskOffPct -gt 80 -or $hkRiskOffPct -gt 80) { "🔴 YES" } else { "🟢 No" }) | Check data freshness |
| S2: Persistence < 1.5d (14d) | $(if ($cnPersistence -lt 1.5 -or $hkPersistence -lt 1.5) { "🔴 YES" } else { "🟢 No" }) | Check volatility |
| S3: Transitions > 50% (14d) | $(if ($cnTransitionRate -gt 50 -or $hkTransitionRate -gt 50) { "🔴 YES" } else { "🟢 No" }) | Check synchronization |
| E2: PSI > 0.25 | $(if ($psi -gt 0.25) { "🔴 YES" } else { "🟢 No" }) | Check structural shift |

---

## Observations & Notes

_Weekly human observations:_

- 
- 
- 

---

## Next Week Action Items

- [ ] 
- [ ] 
- [ ] 

---

*Report generated by weekly-review.ps1*
*For Kill Criteria details, see kill-criteria.md*
"@

$report | Out-File $reportFile -Encoding UTF8

Write-Host "Weekly report generated: $reportFile" -ForegroundColor Green
Write-Host ""
Write-Host "=== Kill Criteria Status ===" -ForegroundColor Yellow

if ($cnRiskOffPct -gt 80 -or $hkRiskOffPct -gt 80) {
    Write-Host "🔴 KILL S1 TRIGGERED: RiskOff coverage excessive" -ForegroundColor Red
}
if ($cnPersistence -lt 1.5 -or $hkPersistence -lt 1.5) {
    Write-Host "🔴 KILL S2 TRIGGERED: State persistence collapsed" -ForegroundColor Red
}
if ($cnTransitionRate -gt 50 -or $hkTransitionRate -gt 50) {
    Write-Host "🔴 KILL S3 TRIGGERED: Transition rate too high" -ForegroundColor Red
}
if ($psi -gt 0.25) {
    Write-Host "🔴 KILL E2 TRIGGERED: PSI exceeds 0.25" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Next Steps ===" -ForegroundColor Cyan
Write-Host "1. Review report: $reportFile" -ForegroundColor Gray
Write-Host "2. Fill in Observations & Notes" -ForegroundColor Gray
Write-Host "3. Check any triggered Kill Criteria" -ForegroundColor Gray
Write-Host "4. Update Action Items for next week" -ForegroundColor Gray
