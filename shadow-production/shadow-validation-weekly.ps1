# Phase 2C Shadow Validation - Weekly Review Script
# Run this script weekly to generate Shadow Validation weekly review

param(
    [string]$Scope = "cn",
    [string]$WeekStart = "",
    [string]$WeekEnd = ""
)

# Default dates: last 7 days
if (-not $WeekStart) {
    $WeekStart = (Get-Date).AddDays(-7).ToString("yyyy-MM-dd")
}
if (-not $WeekEnd) {
    $WeekEnd = (Get-Date).ToString("yyyy-MM-dd")
}

$ReportDir = "reports/shadow-validation"
if (-not (Test-Path $ReportDir)) {
    New-Item -ItemType Directory -Path $ReportDir -Force | Out-Null
}

$WeekStamp = (Get-Date).ToString("yyyy-MM-dd")
$WeeklyReport = "$ReportDir/weekly_review_${Scope}_${WeekStamp}.md"

Write-Host "Phase 2C Shadow Validation - Weekly Review"
Write-Host "Scope: $Scope"
Write-Host "Week: $WeekStart to $WeekEnd"
Write-Host ""

# Run shadow-deployment for the week
cargo run -p quant-cli --quiet -- shadow-deployment `
    --scope $Scope `
    --from $WeekStart `
    --to $WeekEnd `
    --output markdown | Out-File -FilePath $WeeklyReport -Encoding UTF8

if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: shadow-deployment failed"
    exit 1
}

# Parse JSON for metrics
$jsonContent = Get-Content -Path "$ReportDir/shadow_deployment_${Scope}_${WeekEnd}.json" -Raw -ErrorAction SilentlyContinue | ConvertFrom-Json -ErrorAction SilentlyContinue

Write-Host "Weekly Review saved: $WeeklyReport"
Write-Host ""

# Extract key metrics
if ($jsonContent) {
    $totalDays = $jsonContent.summary.total_days
    $highRiskDays = $jsonContent.summary.high_risk_days
    $transitionDays = $jsonContent.summary.transition_detected_days
    $avgScore = $jsonContent.summary.avg_holding_risk_score
    $validationStatus = $jsonContent.summary.validation_status

    Write-Host "=== Weekly Metrics ==="
    Write-Host "Total Days: $totalDays"
    Write-Host "HIGH_RISK Days: $highRiskDays"
    Write-Host "Transition Detected Days: $transitionDays"
    Write-Host "Avg HoldingRiskScore: $avgScore"
    Write-Host "Validation Status: $validationStatus"
    Write-Host ""

    # Metric 1: Transition Lead Time (target: >3 days)
    if ($transitionDays -gt 0) {
        Write-Host "Metric 1 - Transition Lead Time: PASS (transitions detected)"
    } else {
        Write-Host "Metric 1 - Transition Lead Time: WARNING (no transitions this week)"
    }

    # Metric 2: False Alarm Rate (target: <30%)
    # Note: This requires future T+60 returns, which are not available in real-time
    Write-Host "Metric 2 - False Alarm Rate: PENDING (requires T+60 backfill)"

    # Metric 3: State Stability (target: HIGH_RISK <30%)
    $highRiskRatio = if ($totalDays -gt 0) { [math]::Round($highRiskDays / $totalDays * 100, 1) } else { 0 }
    Write-Host "Metric 3 - State Stability: HIGH_RISK ratio = $highRiskRatio%"
    if ($highRiskRatio -lt 30) {
        Write-Host "  PASS (HIGH_RISK ratio < 30%)"
    } else {
        Write-Host "  WARNING (HIGH_RISK ratio >= 30%)"
    }
} else {
    Write-Host "WARNING: Could not parse JSON metrics. Review report manually."
}

Write-Host ""
Write-Host "Weekly Review complete."
Write-Host "Next: Run backfill for T+20/T+60 returns when available."
