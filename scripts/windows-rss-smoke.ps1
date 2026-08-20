[CmdletBinding()]
param(
    [ValidateRange(5, 60)][int]$TimeoutSeconds = 20,
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if ($SkipBuild) {
    throw 'SkipBuild is not allowed when producing durable Story 2.2 evidence.'
}
if (-not $SkipBuild) {
    & (Join-Path $projectRoot 'scripts\pnpm-env.cmd') build
    if ($LASTEXITCODE -ne 0) { throw "Windows frontend build failed with exit code $LASTEXITCODE." }
    & (Join-Path $projectRoot 'scripts\rust-msvc-env.cmd') build -p ai-subscribe-desktop --release --features benchmark-instrumentation
    if ($LASTEXITCODE -ne 0) { throw "Windows release build failed with exit code $LASTEXITCODE." }
}

$releaseRoot = (Resolve-Path (Join-Path $projectRoot 'target\x86_64-pc-windows-msvc\release')).Path
$executable = (Resolve-Path (Join-Path $releaseRoot 'ai-subscribe-desktop.exe')).Path
if (-not $executable.StartsWith($releaseRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Candidate resolved outside the project release directory.'
}
if (@(Get-Process -Name 'ai-subscribe-desktop' -ErrorAction SilentlyContinue).Count -ne 0) {
    throw 'RSS smoke requires zero existing candidate processes.'
}

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

function Get-ById([IntPtr]$Handle, [string]$AutomationId) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($Handle)
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::AutomationIdProperty,
        $AutomationId
    )
    $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
}

function Wait-Element(
    [System.Diagnostics.Process]$Process,
    [string]$AutomationId
) {
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    do {
        Start-Sleep -Milliseconds 100
        $Process.Refresh()
        if (-not $Process.HasExited -and $Process.MainWindowHandle -ne 0) {
            $element = Get-ById -Handle $Process.MainWindowHandle -AutomationId $AutomationId
            if ($null -ne $element -and -not $element.Current.IsOffscreen) { return $element }
        }
    } while (-not $Process.HasExited -and [DateTime]::UtcNow -lt $deadline)
    $snapshot = @()
    if (-not $Process.HasExited -and $Process.MainWindowHandle -ne 0) {
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($Process.MainWindowHandle)
        $all = $root.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        $snapshot = @($all | Select-Object -First 100 | ForEach-Object {
            "$($_.Current.AutomationId)|$($_.Current.Name)|$($_.Current.ControlType.ProgrammaticName)"
        })
    }
    throw "Visible element id='$AutomationId' was not available within $TimeoutSeconds seconds. Tree: $($snapshot -join '; ')"
}

function Invoke-Element([System.Windows.Automation.AutomationElement]$Element) {
    $pattern = $null
    if (-not $Element.TryGetCurrentPattern(
        [System.Windows.Automation.InvokePattern]::Pattern,
        [ref]$pattern
    )) { throw "Element '$($Element.Current.Name)' does not expose InvokePattern." }
    $pattern.Invoke()
}

function Get-DescendantProcessIds([int]$RootProcessId) {
    $all = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue)
    $owned = [System.Collections.Generic.HashSet[int]]::new()
    $queue = [System.Collections.Generic.Queue[int]]::new()
    $queue.Enqueue($RootProcessId)
    while ($queue.Count -gt 0) {
        $parent = $queue.Dequeue()
        foreach ($child in $all | Where-Object { $_.ParentProcessId -eq $parent }) {
            if ($owned.Add([int]$child.ProcessId)) { $queue.Enqueue([int]$child.ProcessId) }
        }
    }
    @($owned)
}

function Get-ProjectRelativePath([string]$BasePath, [string]$TargetPath) {
    $baseUri = [Uri]::new($BasePath.TrimEnd('\') + '\')
    $targetUri = [Uri]::new($TargetPath)
    [Uri]::UnescapeDataString($baseUri.MakeRelativeUri($targetUri).ToString()).Replace('/', '\')
}

function Assert-ReleaseCandidateSurface([string]$ExecutablePath) {
    $candidateText = [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($ExecutablePath))
    foreach ($forbiddenMarker in @(
        'FakeResolver',
        'FakeConnector',
        'private.example.test',
        'fixture-probe',
        'test-transport',
        'allow-localhost'
    )) {
        if ($candidateText.Contains($forbiddenMarker)) {
            throw "Release candidate contains forbidden test transport marker '$forbiddenMarker'."
        }
    }
}

Assert-ReleaseCandidateSurface -ExecutablePath $executable

$runId = [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss-fff')
$runRoot = New-Item -ItemType Directory -Force (Join-Path $projectRoot "target\story-1-6-benchmark\rss-smoke-$runId")
$previousLocalAppData = $env:LOCALAPPDATA
$previousDataDir = $env:AI_SUBSCRIBE_BENCHMARK_DATA_DIR
$previousWebViewArguments = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$process = $null
$ownedIds = [System.Collections.Generic.HashSet[int]]::new()
$evidence = $null
$evidencePath = Join-Path $projectRoot '_agentic-out\tests\evidence\story-2-2-native-rss-smoke.json'

try {
    $env:LOCALAPPDATA = $runRoot.FullName
    $env:AI_SUBSCRIBE_BENCHMARK_DATA_DIR = $runRoot.FullName
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--force-renderer-accessibility'
    $process = Start-Process -FilePath $executable -PassThru
    $rootProcessId = $process.Id

    $sourcesNavigation = Wait-Element -Process $process -AutomationId 'app-nav-sources'
    Invoke-Element $sourcesNavigation
    $sourceInput = Wait-Element -Process $process -AutomationId 'source-url'
    [void](Wait-Element -Process $process -AutomationId 'source-empty-state')

    $valuePattern = $null
    if (-not $sourceInput.TryGetCurrentPattern(
        [System.Windows.Automation.ValuePattern]::Pattern,
        [ref]$valuePattern
    )) { throw 'Source input does not expose ValuePattern.' }
    $valuePattern.SetValue('http://127.0.0.1/feed.xml')
    Invoke-Element (Wait-Element -Process $process -AutomationId 'source-save-button')
    $errorElement = Wait-Element -Process $process -AutomationId 'source-save-error'
    $errorText = @($errorElement.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.Condition]::TrueCondition
    ) | ForEach-Object { $_.Current.Name }) -join ' '
    if (-not $errorText.Contains('validation.source')) {
        throw "Blocked source returned an unexpected error: $errorText"
    }

    Invoke-Element (Wait-Element -Process $process -AutomationId 'source-refresh-button')
    [void](Wait-Element -Process $process -AutomationId 'source-empty-state')
    $database = Join-Path $runRoot.FullName 'ai-subscribe.sqlite3'
    if (-not (Test-Path -LiteralPath $database -PathType Leaf)) {
        throw 'The native source query did not create/open the isolated project database.'
    }

    $evidence = [ordered]@{
        passed = $true
        measured_at_utc = [DateTime]::UtcNow.ToString('o')
        build_kind = 'release-candidate-no-test-transport'
        candidate_sha256 = (Get-FileHash -Algorithm SHA256 $executable).Hash
        data_root = Get-ProjectRelativePath -BasePath $projectRoot -TargetPath $runRoot.FullName
        query_ipc_observed = $true
        rejected_save_ipc_observed = $true
        rejected_code = 'validation.source'
        post_rejection_source_count = 0
        database_created = $true
        system_settings_changed = $false
    }
}
finally {
    if ($null -ne $process) {
        foreach ($ownedId in @(Get-DescendantProcessIds -RootProcessId $process.Id)) {
            [void]$ownedIds.Add($ownedId)
        }
        if (-not $process.HasExited) {
            [void]$process.CloseMainWindow()
            if (-not $process.WaitForExit(5000)) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
        }
    }
    foreach ($ownedId in @($ownedIds)) {
        Stop-Process -Id $ownedId -Force -ErrorAction SilentlyContinue
    }
    $cleanupDeadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        $remainingOwnedIds = @($ownedIds | Where-Object {
            $null -ne (Get-Process -Id $_ -ErrorAction SilentlyContinue)
        })
        if ($remainingOwnedIds.Count -eq 0) { break }
        foreach ($ownedId in $remainingOwnedIds) {
            Stop-Process -Id $ownedId -Force -ErrorAction SilentlyContinue
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $cleanupDeadline)
    $env:LOCALAPPDATA = $previousLocalAppData
    $env:AI_SUBSCRIBE_BENCHMARK_DATA_DIR = $previousDataDir
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $previousWebViewArguments
}

if (@(Get-Process -Name 'ai-subscribe-desktop' -ErrorAction SilentlyContinue).Count -ne 0) {
    throw 'RSS smoke left a residual candidate process.'
}
foreach ($ownedId in @($ownedIds)) {
    if ($null -ne (Get-Process -Id $ownedId -ErrorAction SilentlyContinue)) {
        throw "RSS smoke left candidate-owned process $ownedId running."
    }
}
if ($null -eq $evidence) { throw 'RSS smoke completed without evidence.' }
$evidence.process_tree_zero = $true
New-Item -ItemType Directory -Force (Split-Path $evidencePath) | Out-Null
$temporaryEvidencePath = "$evidencePath.tmp"
[System.IO.File]::WriteAllText(
    $temporaryEvidencePath,
    ($evidence | ConvertTo-Json -Depth 4),
    [System.Text.UTF8Encoding]::new($false)
)
Move-Item -LiteralPath $temporaryEvidencePath -Destination $evidencePath -Force
Write-Output "Story 2.2 native RSS smoke PASS: $evidencePath"
