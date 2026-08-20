[CmdletBinding()]
param(
    [ValidateRange(5, 60)][int]$TimeoutSeconds = 20,
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$pnpmEnvironment = Join-Path $projectRoot 'scripts\pnpm-env.cmd'
$rustEnvironment = Join-Path $projectRoot 'scripts\rust-msvc-env.cmd'
if (-not $SkipBuild) {
    & $pnpmEnvironment build
    if ($LASTEXITCODE -ne 0) { throw "Windows frontend build failed with exit code $LASTEXITCODE." }
    & $rustEnvironment build -p ai-subscribe-desktop --release --features benchmark-instrumentation
    if ($LASTEXITCODE -ne 0) { throw "Windows release build failed with exit code $LASTEXITCODE." }
}
$releaseRoot = (Resolve-Path (Join-Path $projectRoot 'target\x86_64-pc-windows-msvc\release')).Path
$executable = (Resolve-Path (Join-Path $releaseRoot 'ai-subscribe-desktop.exe')).Path
if (-not $executable.StartsWith($releaseRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Candidate resolved outside the project release directory.'
}
if (@(Get-Process -Name 'ai-subscribe-desktop' -ErrorAction SilentlyContinue).Count -ne 0) {
    throw 'Configuration smoke requires zero existing candidate processes.'
}

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class ConfigurationKeyboardProbe {
    [DllImport("user32.dll")] private static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    public static void PressEnter() {
        keybd_event(0x0D, 0, 0, UIntPtr.Zero);
        keybd_event(0x0D, 0, 0x0002, UIntPtr.Zero);
    }
}
'@
$runId = [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss-fff')
$runRoot = New-Item -ItemType Directory -Force (Join-Path $projectRoot "target\story-1-6-benchmark\configuration-smoke-$runId")
$previousLocalAppData = $env:LOCALAPPDATA
$previousWebViewArguments = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$previousBenchmarkDataDir = $env:AI_SUBSCRIBE_BENCHMARK_DATA_DIR
$process = $null

function Get-ById([IntPtr]$Handle, [string]$AutomationId) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($Handle)
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::AutomationIdProperty,
        $AutomationId
    )
    $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
}

function Wait-ById([System.Diagnostics.Process]$Process, [string]$AutomationId) {
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    do {
        Start-Sleep -Milliseconds 100
        $Process.Refresh()
        if (-not $Process.HasExited -and $Process.MainWindowHandle -ne 0) {
            $element = Get-ById -Handle $Process.MainWindowHandle -AutomationId $AutomationId
            if ($null -ne $element) {
                if ($element.Current.IsOffscreen) {
                    $scrollItem = $null
                    if ($element.TryGetCurrentPattern(
                        [System.Windows.Automation.ScrollItemPattern]::Pattern,
                        [ref]$scrollItem
                    )) {
                        $scrollItem.ScrollIntoView()
                    }
                }
                if (-not $element.Current.IsOffscreen) { return $element }
            }
        }
    } while (-not $Process.HasExited -and [DateTime]::UtcNow -lt $deadline)
    $snapshot = @()
    if (-not $Process.HasExited -and $Process.MainWindowHandle -ne 0) {
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($Process.MainWindowHandle)
        $all = $root.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        $snapshot = @($all | Select-Object -First 80 | ForEach-Object {
            "$($_.Current.AutomationId)|$($_.Current.Name)|offscreen=$($_.Current.IsOffscreen)"
        })
    }
    throw "Visible automation element '$AutomationId' was not available within $TimeoutSeconds seconds. Tree: $($snapshot -join '; ')"
}

function Wait-FocusedId([string]$AutomationId) {
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    do {
        Start-Sleep -Milliseconds 100
        $focused = [System.Windows.Automation.AutomationElement]::FocusedElement
        if ($null -ne $focused -and $focused.Current.AutomationId -eq $AutomationId) { return }
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Focus did not move to '$AutomationId'."
}

function Wait-NameContains(
    [System.Diagnostics.Process]$Process,
    [string]$AutomationId,
    [string]$ExpectedText
) {
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    $observed = '<missing>'
    do {
        Start-Sleep -Milliseconds 100
        $Process.Refresh()
        if (-not $Process.HasExited -and $Process.MainWindowHandle -ne 0) {
            $element = Get-ById -Handle $Process.MainWindowHandle -AutomationId $AutomationId
            if ($null -ne $element) {
                $descendants = $element.FindAll(
                    [System.Windows.Automation.TreeScope]::Descendants,
                    [System.Windows.Automation.Condition]::TrueCondition
                )
                $observed = (@($element.Current.Name) + @(
                    $descendants | ForEach-Object { $_.Current.Name }
                ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ' '
                if ($observed.Contains($ExpectedText)) { return }
            }
        }
    } while (-not $Process.HasExited -and [DateTime]::UtcNow -lt $deadline)
    throw "Automation element '$AutomationId' did not contain '$ExpectedText'; observed '$observed'."
}

function Invoke-Element([System.Windows.Automation.AutomationElement]$Element) {
    $pattern = $null
    if (-not $Element.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) {
        throw "Element '$($Element.Current.AutomationId)' does not support InvokePattern."
    }
    $pattern.Invoke()
}

function Set-ElementValue([System.Windows.Automation.AutomationElement]$Element, [string]$Value) {
    $pattern = $null
    if (-not $Element.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$pattern)) {
        throw "Element '$($Element.Current.AutomationId)' does not support ValuePattern."
    }
    $pattern.SetValue($Value)
}

function Get-ElementValue([System.Windows.Automation.AutomationElement]$Element) {
    $pattern = $null
    if (-not $Element.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$pattern)) {
        throw "Element '$($Element.Current.AutomationId)' does not support ValuePattern."
    }
    $pattern.Current.Value
}

function Invoke-WithKeyboard([System.Windows.Automation.AutomationElement]$Element) {
    $Element.SetFocus()
    [ConfigurationKeyboardProbe]::PressEnter()
}

function Set-ToggleOff([System.Windows.Automation.AutomationElement]$Element) {
    $pattern = $null
    if (-not $Element.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$pattern)) {
        throw "Element '$($Element.Current.AutomationId)' does not support TogglePattern."
    }
    if ($pattern.Current.ToggleState -ne [System.Windows.Automation.ToggleState]::Off) { $pattern.Toggle() }
}

function Get-OwnedProcessIds([int]$RootProcessId) {
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

function Stop-Candidate([System.Diagnostics.Process]$Process) {
    $rootProcessId = $Process.Id
    $ownedSet = [System.Collections.Generic.HashSet[int]]::new()
    foreach ($processId in @(Get-OwnedProcessIds -RootProcessId $rootProcessId)) {
        [void]$ownedSet.Add($processId)
    }
    if (-not $Process.HasExited -and -not $Process.CloseMainWindow()) {
        Stop-Process -Id $rootProcessId -Force -ErrorAction SilentlyContinue
    }
    $closeDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while (-not $Process.HasExited -and [DateTime]::UtcNow -lt $closeDeadline) {
        foreach ($processId in @(Get-OwnedProcessIds -RootProcessId $rootProcessId)) {
            [void]$ownedSet.Add($processId)
        }
        [void]$Process.WaitForExit(100)
        $Process.Refresh()
    }
    if (-not $Process.HasExited) {
        foreach ($processId in @($ownedSet | Sort-Object -Descending)) {
            Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
        }
        Stop-Process -Id $rootProcessId -Force -ErrorAction SilentlyContinue
    }
    foreach ($processId in @($ownedSet | Sort-Object -Descending)) {
        Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
    }
    if ($null -ne (Get-Process -Id $rootProcessId -ErrorAction SilentlyContinue)) {
        Stop-Process -Id $rootProcessId -Force -ErrorAction SilentlyContinue
    }
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        foreach ($processId in @(Get-OwnedProcessIds -RootProcessId $rootProcessId)) {
            if ($ownedSet.Add($processId)) {
                Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
            }
        }
        $remaining = @($rootProcessId) + @($ownedSet) | Where-Object {
            $null -ne (Get-Process -Id $_ -ErrorAction SilentlyContinue)
        }
        if ($remaining.Count -eq 0) { return }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Candidate process tree cleanup failed: $($remaining -join ', ')."
}

function Start-Candidate {
    $candidate = Start-Process -FilePath $executable -PassThru
    Wait-ById -Process $candidate -AutomationId 'app-nav-rules' | Out-Null
    $candidate
}

function Open-Rules([System.Diagnostics.Process]$Process) {
    Invoke-Element (Wait-ById -Process $Process -AutomationId 'app-nav-rules')
    Wait-ById -Process $Process -AutomationId 'configuration-save' | Out-Null
}

$sourcePaths = @(
    'scripts\windows-configuration-smoke.ps1',
    'crates\radar-core\src\application\configuration.rs',
    'crates\radar-core\src\application\demo.rs',
    'crates\radar-core\src\application\setup.rs',
    'crates\radar-core\src\contracts\dto\configuration_validation.rs',
    'crates\radar-core\src\domain\rules\configuration_validation.rs',
    'apps\windows\src-tauri\src\commands\mod.rs',
    'apps\windows\src-tauri\src\lib.rs',
    'apps\windows\src\features\configuration-validation\configuration-editor.tsx',
    'apps\windows\src\lib\desktop-api\tauri-desktop-api.ts',
    'apps\windows\src\app\router\app-router.tsx',
    'contracts\fixtures\configuration-validation\blocking\cases-v1.json',
    'contracts\fixtures\configuration-validation\narrowing\cases-v1.json',
    'contracts\fixtures\configuration-validation\valid\basic-v1.json',
    'apps\windows\package.json',
    'Cargo.lock',
    'pnpm-lock.yaml'
)
function Get-SourceFingerprints {
    $fingerprints = [ordered]@{}
    foreach ($relativeSource in $sourcePaths) {
        $fingerprints[$relativeSource] = (
            Get-FileHash -LiteralPath (Join-Path $projectRoot $relativeSource) -Algorithm SHA256
        ).Hash.ToLowerInvariant()
    }
    $fingerprints
}
$candidateShaBefore = (Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash.ToLowerInvariant()
$sourceFingerprintsBefore = Get-SourceFingerprints

try {
    $env:LOCALAPPDATA = $runRoot.FullName
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--force-renderer-accessibility'
    $env:AI_SUBSCRIBE_BENCHMARK_DATA_DIR = $runRoot.FullName
    $process = Start-Candidate
    Open-Rules -Process $process

    $track = Wait-ById -Process $process -AutomationId 'track-name-0'
    $initialTrackName = Get-ElementValue -Element $track
    Set-ElementValue -Element $track -Value ''
    Invoke-WithKeyboard (Wait-ById -Process $process -AutomationId 'configuration-save')
    Wait-ById -Process $process -AutomationId 'configuration-blocking-errors' | Out-Null
    Wait-FocusedId -AutomationId 'track-name-0'
    Stop-Candidate -Process $process
    $process = $null

    $process = Start-Candidate
    Open-Rules -Process $process
    $track = Wait-ById -Process $process -AutomationId 'track-name-0'
    if ((Get-ElementValue -Element $track) -ne $initialTrackName) {
        throw 'Blocking validation changed the persisted configuration.'
    }

    Set-ElementValue -Element $track -Value 'AI Agents'
    Set-ToggleOff (Wait-ById -Process $process -AutomationId 'source-enabled-0')
    Invoke-WithKeyboard (Wait-ById -Process $process -AutomationId 'configuration-save')
    $riskReturnButton = Wait-ById -Process $process -AutomationId 'configuration-risk-return'
    Invoke-WithKeyboard $riskReturnButton
    Wait-FocusedId -AutomationId 'configuration-save'
    Stop-Candidate -Process $process
    $process = $null

    $process = Start-Candidate
    Open-Rules -Process $process
    $restoredBeforeConfirmation = Wait-ById -Process $process -AutomationId 'source-enabled-0'
    $beforeConfirmationToggle = $null
    if (-not $restoredBeforeConfirmation.TryGetCurrentPattern(
        [System.Windows.Automation.TogglePattern]::Pattern,
        [ref]$beforeConfirmationToggle
    )) { throw 'Pre-confirmation source does not expose TogglePattern.' }
    if ($beforeConfirmationToggle.Current.ToggleState -ne [System.Windows.Automation.ToggleState]::On) {
        throw 'Narrowing configuration was persisted before explicit confirmation.'
    }
    Set-ToggleOff $restoredBeforeConfirmation
    Invoke-WithKeyboard (Wait-ById -Process $process -AutomationId 'configuration-save')
    Wait-ById -Process $process -AutomationId 'configuration-risk-return' | Out-Null
    Invoke-WithKeyboard (Wait-ById -Process $process -AutomationId 'configuration-risk-confirm')
    Wait-NameContains -Process $process -AutomationId 'configuration-state' -ExpectedText 'saved'
    Stop-Candidate -Process $process
    $process = $null

    $process = Start-Candidate
    Open-Rules -Process $process
    $restoredSource = Wait-ById -Process $process -AutomationId 'source-enabled-0'
    $toggle = $null
    if (-not $restoredSource.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$toggle)) {
        throw 'Restored source does not expose TogglePattern.'
    }
    if ($toggle.Current.ToggleState -ne [System.Windows.Automation.ToggleState]::Off) {
        throw 'Confirmed narrowing configuration did not survive restart.'
    }
    Stop-Candidate -Process $process
    $process = $null
    $candidateShaAfter = (Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash.ToLowerInvariant()
    $sourceFingerprints = Get-SourceFingerprints
    if ($candidateShaAfter -ne $candidateShaBefore) {
        throw 'Release candidate changed while native evidence was being collected.'
    }
    foreach ($relativeSource in $sourcePaths) {
        if ($sourceFingerprints[$relativeSource] -ne $sourceFingerprintsBefore[$relativeSource]) {
            throw "Source changed while native evidence was being collected: $relativeSource"
        }
    }
    $evidence = [ordered]@{
        story = '2.1'
        passed = $true
        candidate = $executable
        candidate_sha256 = $candidateShaAfter
        source_sha256 = $sourceFingerprints
        data_root = $runRoot.FullName
        blocking_focus_automation_id = 'track-name-0'
        risk_confirmation = $true
        restart_persistence = $true
        process_tree_cleanup = $true
        system_theme = 'not_measured'
        system_scaling = 'not_measured'
        external_permissions = 'not_measured'
        completed_at_utc = [DateTime]::UtcNow.ToString('o')
    }
    $evidencePath = Join-Path $runRoot.FullName 'native-configuration-smoke.json'
    $evidenceJson = $evidence | ConvertTo-Json -Depth 5
    $evidenceJson | Set-Content -LiteralPath $evidencePath -Encoding UTF8
    $durableEvidence = Join-Path $projectRoot '_agentic-out\tests\evidence\story-2-1-native-configuration.json'
    New-Item -ItemType Directory -Force (Split-Path $durableEvidence) | Out-Null
    $evidenceJson | Set-Content -LiteralPath $durableEvidence -Encoding UTF8
    Write-Output "Story 2.1 native configuration smoke PASS: $evidencePath"
}
finally {
    try {
        if ($null -ne $process) { Stop-Candidate -Process $process }
    }
    finally {
        $env:LOCALAPPDATA = $previousLocalAppData
        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $previousWebViewArguments
        $env:AI_SUBSCRIBE_BENCHMARK_DATA_DIR = $previousBenchmarkDataDir
    }
}
