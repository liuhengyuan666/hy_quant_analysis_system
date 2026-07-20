# Phase 2C Shadow Validation - Daily Run Script
# Run this script daily to generate Shadow Deployment reports

param(
    [string]$Scope = "cn",
    [string]$FromDate = "",
    [string]$ToDate = ""
)

# Default dates: last 30 days
if (-not $FromDate) {
    $FromDate = (Get-Date).AddDays(-30).ToString("yyyy-MM-dd")
}
if (-not $ToDate) {
    $ToDate = (Get-Date).ToString("yyyy-MM-dd")
}

$ReportDir = "reports/shadow-validation"
if (-not (Test-Path $ReportDir)) {
    New-Item -ItemType Directory -Path $ReportDir -Force | Out-Null
}

$DateStamp = (Get-Date).ToString("yyyy-MM-dd")
$MarkdownReport = "$ReportDir/shadow_deployment_${Scope}_${DateStamp}.md"
$JsonReport = "$ReportDir/shadow_deployment_${Scope}_${DateStamp}.json"
$IntegrityReport = "$ReportDir/context_integrity_${Scope}_${DateStamp}.md"

Write-Host "Phase 2C Shadow Validation - Daily Run"
Write-Host "Scope: $Scope"
Write-Host "Date Range: $FromDate to $ToDate"
Write-Host ""

# TASK-170: Live Context Integrity Gate
Write-Host "Step 1: Context Integrity Gate (live)"
cargo run -p quant-cli --quiet -- execution-context-integrity-gate `
    --live `
    --scope $Scope `
    --output markdown | Out-File -FilePath $IntegrityReport -Encoding UTF8

if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Context Integrity Gate FAILED. Shadow Validation BLOCKED."
    Write-Host "Review: $IntegrityReport"
    exit 1
}

Write-Host "Context Integrity Gate PASS: $IntegrityReport"
Write-Host ""

# Run shadow-deployment
Write-Host "Step 2: Shadow Deployment"
cargo run -p quant-cli --quiet -- shadow-deployment `
    --scope $Scope `
    --from $FromDate `
    --to $ToDate `
    --output markdown | Out-File -FilePath $MarkdownReport -Encoding UTF8

if ($LASTEXITCODE -eq 0) {
    Write-Host "Markdown report saved: $MarkdownReport"
} else {
    Write-Host "ERROR: shadow-deployment failed"
    exit 1
}

cargo run -p quant-cli --quiet -- shadow-deployment `
    --scope $Scope `
    --from $FromDate `
    --to $ToDate `
    --output json | Out-File -FilePath $JsonReport -Encoding UTF8

if ($LASTEXITCODE -eq 0) {
    Write-Host "JSON report saved: $JsonReport"
} else {
    Write-Host "ERROR: shadow-deployment JSON failed"
    exit 1
}

# TASK-172: Shadow Validation Monitor
Write-Host ""
Write-Host "Step 3: Shadow Validation Monitor"
$jsonContent = Get-Content -Path $JsonReport -Raw | ConvertFrom-Json
$validationStatus = $jsonContent.summary.validation_status
$totalDays = $jsonContent.summary.total_days
$transitionDays = $jsonContent.summary.transition_detected_days

Write-Host "Validation Status: $validationStatus"
Write-Host "Total Days: $totalDays"
Write-Host "Transition Detected Days: $transitionDays"

if ($validationStatus -eq "INSUFFICIENT_EVENTS") {
    Write-Host ""
    Write-Host "WARNING: INSUFFICIENT_EVENTS detected."
    Write-Host "No Transition Detection events for 20+ consecutive trading days."
    Write-Host "This is NOT a failure, but Shadow Validation should pause and review."
    Write-Host "Consider: (1) extending observation window, (2) reviewing regime distribution."
}

Write-Host ""
Write-Host "Shadow Validation complete."
Write-Host "Review the reports in $ReportDir"
