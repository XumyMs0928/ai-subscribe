[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$evidenceDirectory = Join-Path $projectRoot "target\story-4-5"
$lockPath = Join-Path $evidenceDirectory "performance-gate.lock"
$expectedDatasetSize = 50000
$expectedSampleCount = 30
$expectedThresholdMs = 200
$expectedThresholdUs = $expectedThresholdMs * 1000
New-Item -ItemType Directory -Force -Path $evidenceDirectory | Out-Null

function Test-Story45EvidenceContract {
    param([Parameter(Mandatory)]$Evidence)

    $hashPattern = "^[0-9a-f]{64}$"
    return (
        $Evidence.story -eq "4.5" -and
        $Evidence.dataset_size -eq $expectedDatasetSize -and
        $Evidence.sample_count_per_scenario -eq $expectedSampleCount -and
        $Evidence.threshold_ms -eq $expectedThresholdMs -and
        @($Evidence.default_samples_us).Count -eq $expectedSampleCount -and
        @($Evidence.combined_samples_us).Count -eq $expectedSampleCount -and
        $Evidence.default_p95_us -lt $expectedThresholdUs -and
        $Evidence.combined_p95_us -lt $expectedThresholdUs -and
        $Evidence.dataset_sha256 -match $hashPattern -and
        $Evidence.source_sha256 -match $hashPattern -and
        $Evidence.candidate_sha256 -match $hashPattern -and
        @($Evidence.query_plan.default).Count -gt 0 -and
        @($Evidence.query_plan.combined).Count -gt 0
    )
}

$lockStream = $null
try {
    try {
        $lockStream = [System.IO.File]::Open(
            $lockPath,
            [System.IO.FileMode]::CreateNew,
            [System.IO.FileAccess]::Write,
            [System.IO.FileShare]::None
        )
    }
    catch [System.IO.IOException] {
        throw "Story 4.5 performance gate is already running (lock: $lockPath)."
    }

    $knownEvidence = @{}
    Get-ChildItem -LiteralPath $evidenceDirectory -Filter "intel-feed-performance-*.json" |
        ForEach-Object { $knownEvidence[$_.FullName] = $true }

    Push-Location $projectRoot
    try {
        & (Join-Path $projectRoot "scripts\rust-msvc-env.cmd") `
            test -p radar-core `
            "application::intel_feed::tests::fixed_50000_item_feed_queries_have_30_sample_p95_below_200ms" `
            --lib -- --ignored --exact --nocapture
        if ($LASTEXITCODE -ne 0) {
            throw "Story 4.5 isolated performance test failed with exit code $LASTEXITCODE."
        }
    }
    finally {
        Pop-Location
    }

    $freshEvidence = @(
        Get-ChildItem -LiteralPath $evidenceDirectory -Filter "intel-feed-performance-*.json" |
            Where-Object { -not $knownEvidence.ContainsKey($_.FullName) }
    )
    if ($freshEvidence.Count -ne 1) {
        throw "Expected exactly one fresh Story 4.5 evidence file, found $($freshEvidence.Count)."
    }

    $evidenceFile = $freshEvidence[0]
    $evidence = Get-Content -Raw -LiteralPath $evidenceFile.FullName | ConvertFrom-Json
    $valid = Test-Story45EvidenceContract -Evidence $evidence
    if (-not $valid) {
        throw "Fresh Story 4.5 evidence failed contract validation: $($evidenceFile.FullName)"
    }

    Write-Output "Story 4.5 performance gate PASS"
    Write-Output "Evidence: $($evidenceFile.FullName)"
    Write-Output "Default P95: $([math]::Round($evidence.default_p95_us / 1000, 3)) ms"
    Write-Output "Combined P95: $([math]::Round($evidence.combined_p95_us / 1000, 3)) ms"
}
finally {
    if ($null -ne $lockStream) {
        $lockStream.Dispose()
        Remove-Item -LiteralPath $lockPath -Force -ErrorAction SilentlyContinue
    }
}
