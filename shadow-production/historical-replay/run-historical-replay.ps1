# Historical Replay Automation Pipeline
# Phase 1 — V7.4 / ADR-078 Evidence Accumulation
#
# Usage:
#   .\run-historical-replay.ps1
#   .\run-historical-replay.ps1 -Scopes @("global","cn","hk") -FromDate "2026-04-01" -ToDate "2026-07-09"
#
# Pipeline: Replay -> Analytics -> Report -> Evidence -> Candidate Library
# Outputs:  manifest.json, summary.json, evidence-index.json, candidate-index.json

param(
    [Parameter()]
    [string[]]$Scopes = @("global", "cn", "hk"),

    [Parameter()]
    [string]$FromDate = (Get-Date).AddDays(-90).ToString("yyyy-MM-dd"),

    [Parameter()]
    [string]$ToDate = (Get-Date -Format "yyyy-MM-dd"),

    [Parameter()]
    [string]$OutputDir = "$PSScriptRoot"
)

$ErrorActionPreference = "Continue"  # Allow manual $LASTEXITCODE checks after native commands
$schemaVersion = "v1"
$analyticsVersion = "v1"
$generatedAt = (Get-Date -Format "yyyy-MM-ddTHH:mm:ss")

function Ensure-Dir($path) {
    if (-not (Test-Path $path)) {
        New-Item -ItemType Directory -Path $path -Force | Out-Null
    }
}

function Invoke-Cli($arguments, $label) {
    $output = cargo run -p quant-cli -- --quiet @arguments 2>$null
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "$label failed (exit code $LASTEXITCODE). Continuing pipeline."
    }
    return $output
}

Ensure-Dir $OutputDir

Write-Host "=== Historical Replay Pipeline ===" -ForegroundColor Cyan
Write-Host "From: $FromDate  To: $ToDate" -ForegroundColor Cyan
Write-Host "Scopes: $($Scopes -join ', ')" -ForegroundColor Cyan
Write-Host "Output: $OutputDir" -ForegroundColor Cyan
Write-Host ""

$conditions = @("srd-strong", "stretch-extreme-crowding-momentum")
$horizons = @(20, 60)

$allReports = [System.Collections.Generic.List[string]]::new()
$allEvidence = [System.Collections.Generic.List[hashtable]]::new()
$allCandidates = [System.Collections.Generic.List[hashtable]]::new()
$summaryByScope = [System.Collections.Generic.List[hashtable]]::new()

foreach ($scope in $Scopes) {
    Write-Host "[Scope: $scope] Starting replay..." -ForegroundColor Yellow

    $scopeSafe = $scope.ToLower()
    $runStamp = "$scopeSafe-$ToDate"

    # 1. Research Review (quarterly synthesis)
    $reviewOut = Join-Path $OutputDir "review-$runStamp.txt"
    $reviewMd = Join-Path $OutputDir "review-$runStamp.md"
    Write-Host "  Running research review -> $reviewOut" -ForegroundColor Gray
    $reviewText = Invoke-Cli @("research", "review", "--scope", $scope, "--from", $FromDate, "--to", $ToDate, "--output", $reviewMd) "research review ($scope)"
    $reviewText | Out-File $reviewOut -Encoding UTF8
    $allReports.Add("review-$runStamp.txt")
    $allReports.Add("review-$runStamp.md")

    # 2. Conditional Forward-Return Analytics for each condition x horizon
    foreach ($condition in $conditions) {
        foreach ($horizon in $horizons) {
            $analyticsOut = Join-Path $OutputDir "analytics-$condition-$scopeSafe-h$horizon.txt"
            Write-Host "  Running analytics: $condition / H$horizon -> $analyticsOut" -ForegroundColor Gray
            $analyticsText = Invoke-Cli @("research", "analytics", "--condition", $condition, "--scope", $scope, "--horizon", $horizon.ToString(), "--save-evidence") "research analytics ($condition / $scope / H$horizon)"
            $analyticsText | Out-File $analyticsOut -Encoding UTF8
            $allReports.Add("analytics-$condition-$scopeSafe-h$horizon.txt")

            # Extract occurrences, positive ratio, and median return from the text output for the index.
            # Format: "Occurrences:              212" and "Positive ratio:           51.9%"
            $occurrences = 0
            $positiveRatio = 0.0
            $medianReturn = 0.0
            $workspaceEvidenceId = $null
            foreach ($line in $analyticsText) {
                if ($line -match 'Occurrences:\s+(\d+)') {
                    $occurrences = [int]$matches[1]
                }
                if ($line -match 'Positive ratio:\s+([0-9.]+)%') {
                    $positiveRatio = [double]$matches[1] / 100.0
                }
                if ($line -match 'Forward return median:\s+([+-]?[0-9.]+)%') {
                    $medianReturn = [double]$matches[1] / 100.0
                }
                if ($line -match 'Evidence saved:\s+(RA-\d+)') {
                    $workspaceEvidenceId = $matches[1]
                }
            }

            $evidenceId = if ($workspaceEvidenceId) { $workspaceEvidenceId } else { "RA-$ToDate-$($scope.ToUpper())-$($condition.ToUpper())-H$horizon" }
            $allEvidence.Add(@{
                id = $evidenceId
                condition = $condition
                scope = $scope
                horizon = $horizon
                occurrences = $occurrences
                positive_ratio = $positiveRatio
                median_forward_return = $medianReturn
                generated_at = $generatedAt
                status = "candidate"
                artifact = "analytics-$condition-$scopeSafe-h$horizon.txt"
                workspace_path = if ($workspaceEvidenceId) { "workspace/evidence/replay/$workspaceEvidenceId/body.json" } else { $null }
            })

            # Mark as candidate evidence if positive ratio is notably high or low.
            # This is a deterministic, extensible placeholder rule. Future ADR-078
            # attribution will replace this heuristic with regime-aware classification.
            if ($occurrences -ge 5 -and ($positiveRatio -ge 0.75 -or $positiveRatio -le 0.25)) {
                $allCandidates.Add(@{
                    id = "CD-$ToDate-$($scope.ToUpper())-$($condition.ToUpper())-H$horizon"
                    condition = $condition
                    scope = $scope
                    horizon = $horizon
                    window = "$FromDate ~ $ToDate"
                    positive_ratio = $positiveRatio
                    occurrences = $occurrences
                    status = "candidate_evidence"
                    attribution_status = "pending"
                    evidence_id = $evidenceId
                })
            }
        }
    }

    # 3. Symbol Scoreboard (snapshot of divergence cases)
    $scoreboardOut = Join-Path $OutputDir "scoreboard-$runStamp.txt"
    Write-Host "  Running symbol scoreboard -> $scoreboardOut" -ForegroundColor Gray
    $scoreboardText = Invoke-Cli @("symbol-scoreboard", "--scope", $scope, "--date", $ToDate) "symbol-scoreboard ($scope)"
    $scoreboardText | Out-File $scoreboardOut -Encoding UTF8
    $allReports.Add("scoreboard-$runStamp.txt")

    $summaryByScope.Add(@{
        scope = $scope
        from = $FromDate
        to = $ToDate
        conditions = $conditions
        horizons = $horizons
        evidence_count = $allEvidence.Count
    })

    Write-Host "  [Scope: $scope] Done." -ForegroundColor Green
    Write-Host ""
}

# 4. Manifest
$manifest = @{
    generated_at = $generatedAt
    schema_version = $schemaVersion
    analytics_version = $analyticsVersion
    from = $FromDate
    to = $ToDate
    scopes = $Scopes
    conditions = $conditions
    horizons = $horizons
    reports = $allReports.ToArray()
    indices = @("manifest.json", "summary.json", "evidence-index.json", "candidate-index.json")
}
$manifest | ConvertTo-Json -Depth 4 | Out-File (Join-Path $OutputDir "manifest.json") -Encoding UTF8

# 5. Summary
$summary = @{
    generated_at = $generatedAt
    schema_version = $schemaVersion
    from = $FromDate
    to = $ToDate
    scopes = $Scopes
    conditions_analyzed = $conditions
    total_evidence = $allEvidence.Count
    total_candidates = $allCandidates.Count
    scope_summaries = $summaryByScope.ToArray()
    top_candidates = ($allCandidates | Sort-Object positive_ratio -Descending | Select-Object -First 5 | ForEach-Object { $_ })
}
$summary | ConvertTo-Json -Depth 4 | Out-File (Join-Path $OutputDir "summary.json") -Encoding UTF8

# 6. Evidence Index
$evidenceIndex = @{
    generated_at = $generatedAt
    schema_version = $schemaVersion
    evidence = $allEvidence.ToArray()
}
$evidenceIndex | ConvertTo-Json -Depth 4 | Out-File (Join-Path $OutputDir "evidence-index.json") -Encoding UTF8

# 7. Candidate Index
$candidateIndex = @{
    generated_at = $generatedAt
    schema_version = $schemaVersion
    candidates = $allCandidates.ToArray()
}
$candidateIndex | ConvertTo-Json -Depth 4 | Out-File (Join-Path $OutputDir "candidate-index.json") -Encoding UTF8

Write-Host "=== Historical Replay Pipeline Complete ===" -ForegroundColor Green
Write-Host "Reports: $($allReports.Count)" -ForegroundColor Green
Write-Host "Evidence entries: $($allEvidence.Count)" -ForegroundColor Green
Write-Host "Candidate entries: $($allCandidates.Count)" -ForegroundColor Green
Write-Host ""
Write-Host "Index files:" -ForegroundColor Cyan
Write-Host "  manifest.json" -ForegroundColor Gray
Write-Host "  summary.json" -ForegroundColor Gray
Write-Host "  evidence-index.json" -ForegroundColor Gray
Write-Host "  candidate-index.json" -ForegroundColor Gray
