[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$RunRoot,
    [ValidateSet('story-1-6', 'story-2-1', 'story-2-2')]
    [string]$EvidenceStory = 'story-1-6',
    [switch]$Publish
)

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$resolvedRunRoot = (Resolve-Path -LiteralPath $RunRoot).Path
$benchmarkRoot = (Resolve-Path (Join-Path $projectRoot 'target\story-1-6-benchmark')).Path
if (-not $resolvedRunRoot.StartsWith($benchmarkRoot + '\', [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'RunRoot must resolve beneath the project benchmark directory.'
}

function Get-ProjectRelativePath([string]$BasePath, [string]$TargetPath) {
    $baseUri = [Uri]::new($BasePath.TrimEnd('\') + '\')
    $targetUri = [Uri]::new($TargetPath)
    [Uri]::UnescapeDataString($baseUri.MakeRelativeUri($targetUri).ToString()).Replace('/', '\')
}

function Write-JsonUtf8([string]$Path, [object]$Value, [int]$Depth = 6) {
    $json = $Value | ConvertTo-Json -Depth $Depth
    [System.IO.File]::WriteAllText($Path, $json, [System.Text.UTF8Encoding]::new($false))
}

$manifestPath = Join-Path $resolvedRunRoot 'run-manifest.json'
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw 'The run has no run-manifest.json; exact provenance cannot be reconstructed.'
}
$manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json
if ($manifest.schema_version -ne 1 -or $manifest.requested_samples -lt 1) {
    throw 'The run manifest is unsupported or invalid.'
}
if ($manifest.evidence_story -ne $EvidenceStory) {
    throw "EvidenceStory '$EvidenceStory' does not match manifest '$($manifest.evidence_story)'."
}
$candidatePath = Join-Path $projectRoot 'target\x86_64-pc-windows-msvc\release\ai-subscribe-desktop.exe'
if (-not (Test-Path -LiteralPath $candidatePath -PathType Leaf) -or
    (Get-FileHash -Algorithm SHA256 -LiteralPath $candidatePath).Hash -ne $manifest.candidate_sha256) {
    throw 'The release candidate no longer matches the run manifest.'
}
foreach ($sourceProperty in $manifest.source_sha256.PSObject.Properties) {
    $sourcePath = Join-Path $projectRoot $sourceProperty.Name
    if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf) -or
        (Get-FileHash -Algorithm SHA256 -LiteralPath $sourcePath).Hash -ne $sourceProperty.Value) {
        throw "Source no longer matches the run manifest: $($sourceProperty.Name)"
    }
}

$sampleFiles = @(Get-ChildItem -LiteralPath $resolvedRunRoot -Directory |
    Where-Object { $_.Name -match '^sample-\d{3}$' } |
    Sort-Object Name |
    ForEach-Object { Join-Path $_.FullName 'sample-result.json' })
if ($sampleFiles.Count -ne [int]$manifest.requested_samples) {
    throw "Expected $($manifest.requested_samples) sample result files, found $($sampleFiles.Count)."
}

$samples = @($sampleFiles | ForEach-Object {
    if (-not (Test-Path -LiteralPath $_ -PathType Leaf)) {
        throw "Missing sample result: $_"
    }
    Get-Content -Raw -Encoding UTF8 -LiteralPath $_ | ConvertFrom-Json
})
$expectedIds = @(1..([int]$manifest.requested_samples))
$actualIds = @($samples | ForEach-Object { [int]$_.sample } | Sort-Object)
if ((Compare-Object $expectedIds $actualIds).Count -ne 0) {
    throw 'Sample IDs are duplicated or incomplete.'
}
if (@($samples | Where-Object { $_.schema_version -ne 1 -or -not $_.ready -or -not $_.process_tree_zero }).Count -ne 0) {
    throw 'At least one sample is invalid, not ready, or left an owned process.'
}

$ordered = @($samples | ForEach-Object { [double]$_.duration_ms } | Sort-Object)
$medianIndex = [Math]::Max(0, [Math]::Ceiling($ordered.Count * 0.50) - 1)
$percentileIndex = [Math]::Max(0, [Math]::Ceiling($ordered.Count * 0.95) - 1)
$result = [pscustomobject]@{
    schema_version = 1
    evidence_story = $EvidenceStory
    samples = $ordered.Count
    successful_samples = $ordered.Count
    success_rate = 1
    p50_ms = [Math]::Round($ordered[$medianIndex], 2)
    p95_ms = [Math]::Round($ordered[$percentileIndex], 2)
    minimum_ms = [Math]::Round($ordered[0], 2)
    maximum_ms = [Math]::Round($ordered[-1], 2)
    threshold_ms = 5000
    passed = $ordered[$percentileIndex] -le 5000
    evidence_root = Get-ProjectRelativePath -BasePath $projectRoot -TargetPath $resolvedRunRoot
    aggregated_at_utc = [DateTime]::UtcNow.ToString('o')
    candidate_sha256 = $manifest.candidate_sha256
    source_sha256 = $manifest.source_sha256
    raw_samples_ms = @($samples | Sort-Object sample | ForEach-Object { [double]$_.duration_ms })
}

$runEvidencePath = Join-Path $resolvedRunRoot 'windows-demo-cold-start.json'
Write-JsonUtf8 -Path $runEvidencePath -Value $result
if ($Publish) {
    $durableEvidence = Join-Path $projectRoot "_agentic-out\tests\evidence\$EvidenceStory-cold-start.json"
    New-Item -ItemType Directory -Force (Split-Path $durableEvidence) | Out-Null
    Write-JsonUtf8 -Path $durableEvidence -Value $result
}
$result
if (-not $result.passed) {
    throw "Cold-start P95 exceeded 5000 ms: $($result.p95_ms) ms."
}
