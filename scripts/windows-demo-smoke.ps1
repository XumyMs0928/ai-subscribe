[CmdletBinding()]
param(
    [ValidateRange(1, 100)]
    [int]$Samples = 1,
    [ValidateRange(1, 30)]
    [int]$TimeoutSeconds = 15,
    [switch]$Evidence
)

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$releaseRoot = (Resolve-Path (Join-Path $projectRoot 'target\x86_64-pc-windows-msvc\release')).Path
$executable = (Resolve-Path (Join-Path $releaseRoot 'ai-subscribe-desktop.exe')).Path
if (-not $executable.StartsWith($releaseRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'The candidate executable resolved outside the project release directory.'
}
if (@(Get-Process -Name 'ai-subscribe-desktop' -ErrorAction SilentlyContinue).Count -ne 0) {
    throw 'The demo smoke requires zero existing ai-subscribe-desktop processes.'
}
if ($Evidence -and $Samples -ne 30) {
    throw 'Formal Story 1.6 Windows milestone evidence requires exactly 30 samples.'
}

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName UIAutomationClientsideProviders
Add-Type -AssemblyName System.Windows.Forms
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class DemoMouseProbe {
    [DllImport("user32.dll")] private static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] private static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] private static extern bool ShowWindow(IntPtr window, int command);
    [DllImport("user32.dll")] private static extern bool SetForegroundWindow(IntPtr window);
    public static void Activate(IntPtr window) {
        ShowWindow(window, 5);
        SetForegroundWindow(window);
    }
    public static void Click(double x, double y) {
        SetCursorPos((int)x, (int)y);
        mouse_event(0x0002, 0, 0, 0, UIntPtr.Zero);
        mouse_event(0x0004, 0, 0, 0, UIntPtr.Zero);
    }
}
'@

$runId = [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss-fff')
$runRoot = New-Item -ItemType Directory -Force (Join-Path $projectRoot "target\story-1-6-benchmark\$runId")
$previousLocalAppData = $env:LOCALAPPDATA
$previousWebViewArguments = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$previousBenchmarkDataDir = $env:AI_SUBSCRIBE_BENCHMARK_DATA_DIR
$durations = [System.Collections.Generic.List[double]]::new()
$process = $null
$webViewVersion = $null

try {
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--force-renderer-accessibility'
    for ($sample = 1; $sample -le $Samples; $sample += 1) {
        $sampleRoot = New-Item -ItemType Directory -Force (Join-Path $runRoot.FullName ("sample-{0:D3}" -f $sample))
        $env:LOCALAPPDATA = $sampleRoot.FullName
        $env:AI_SUBSCRIBE_BENCHMARK_DATA_DIR = $sampleRoot.FullName
        $webViewProcessesBefore = @(Get-Process -Name 'msedgewebview2' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
        $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
        $process = Start-Process -FilePath $executable -PassThru
        $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
        $ready = $false
        $observed = @()
        $readiness = 'not-probed'
        $selectionInvoked = $false
        $lastSelectionAttempt = [DateTime]::MinValue

        do {
            Start-Sleep -Milliseconds 100
            $process.Refresh()
            if ($process.HasExited -or $process.MainWindowHandle -eq 0) { continue }
            try {
                $root = [System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
                if ($root.Current.IsOffscreen) { continue }
                $elements = $root.FindAll(
                    [System.Windows.Automation.TreeScope]::Subtree,
                    [System.Windows.Automation.Condition]::TrueCondition
                )
                $names = [System.Collections.Generic.List[string]]::new()
                $list = $null
                $detail = $null
                for ($index = 0; $index -lt $elements.Count; $index += 1) {
                    $element = $elements.Item($index)
                    if ($element.Current.AutomationId -eq 'demo-intelligence-list') { $list = $element }
                    if ($element.Current.AutomationId -eq 'demo-intelligence-detail') { $detail = $element }
                    if (-not $element.Current.IsOffscreen -and $element.Current.Name) {
                        $names.Add($element.Current.Name)
                    }
                }
                $observed = $names
                $healthy = @($names | Where-Object { $_.Contains('healthy') }).Count -ge 1
                $contract = @($names | Where-Object { $_.Contains('contract_version: 1') }).Count -ge 1
                $listReady = $false
                $detailReady = $false
                $scrollable = $false
                if ($null -ne $list) {
                    $listElements = $list.FindAll(
                        [System.Windows.Automation.TreeScope]::Subtree,
                        [System.Windows.Automation.Condition]::TrueCondition
                    )
                    $selectedListItem = $null
                    $offscreenListItemCount = 0
                    for ($listIndex = 0; $listIndex -lt $listElements.Count; $listIndex += 1) {
                        $listElement = $listElements.Item($listIndex)
                        if ($listElement.Current.AutomationId -eq 'demo-item-demo:rust-197-001') {
                            $selectedListItem = $listElement
                        }
                        if ($listElement.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button -and
                            $listElement.Current.IsOffscreen) {
                            $offscreenListItemCount += 1
                        }
                    }
                    $listReady = $null -ne $selectedListItem
                    $scrollPattern = $null
                    if ($list.TryGetCurrentPattern(
                        [System.Windows.Automation.ScrollPattern]::Pattern,
                        [ref]$scrollPattern
                    ) -and $null -ne $scrollPattern) {
                        $scrollable = $scrollPattern.Current.VerticallyScrollable
                    }
                    $scrollable = $scrollable -or $offscreenListItemCount -ge 1
                    if ($listReady -and
                        ([DateTime]::UtcNow - $lastSelectionAttempt).TotalMilliseconds -ge 500) {
                        $selectionInvoked = $false
                        $scrollItemPattern = $null
                        if ($selectedListItem.TryGetCurrentPattern(
                            [System.Windows.Automation.ScrollItemPattern]::Pattern,
                            [ref]$scrollItemPattern
                        ) -and $null -ne $scrollItemPattern) {
                            $scrollItemPattern.ScrollIntoView()
                        }
                        try {
                            [DemoMouseProbe]::Activate($process.MainWindowHandle)
                            Start-Sleep -Milliseconds 50
                            $point = $selectedListItem.GetClickablePoint()
                            [DemoMouseProbe]::Click($point.X, $point.Y)
                            $selectionInvoked = $true
                        }
                        catch [System.Windows.Automation.NoClickablePointException] {
                            $selectionInvoked = $false
                        }
                        if (-not $selectionInvoked) {
                            $invokePattern = $null
                            if ($selectedListItem.TryGetCurrentPattern(
                                [System.Windows.Automation.InvokePattern]::Pattern,
                                [ref]$invokePattern
                            ) -and $null -ne $invokePattern) {
                                $invokePattern.Invoke()
                                $selectionInvoked = $true
                            }
                        }
                        if (-not $selectionInvoked) {
                            $selectionPattern = $null
                            if ($selectedListItem.TryGetCurrentPattern(
                                [System.Windows.Automation.SelectionItemPattern]::Pattern,
                                [ref]$selectionPattern
                            ) -and $null -ne $selectionPattern) {
                                $selectionPattern.Select()
                                $selectionInvoked = $true
                            }
                        }
                        $lastSelectionAttempt = [DateTime]::UtcNow
                    }
                }
                if ($null -ne $detail) {
                    $detailElements = $detail.FindAll(
                        [System.Windows.Automation.TreeScope]::Subtree,
                        [System.Windows.Automation.Condition]::TrueCondition
                    )
                    for ($detailIndex = 0; $detailIndex -lt $detailElements.Count; $detailIndex += 1) {
                        if ($detailElements.Item($detailIndex).Current.AutomationId -eq 'demo-detail-title' -and
                            $detailElements.Item($detailIndex).Current.Name.Contains('Rust 1.97')) {
                            $detailReady = $true
                        }
                    }
                }
                $ready = $healthy -and $contract -and $listReady -and $detailReady -and $scrollable
                $readiness = "healthy=$healthy contract=$contract list=$listReady selected=$selectionInvoked detail=$detailReady scrollable=$scrollable"
                if ($null -eq $webViewVersion) {
                    $webViewProcess = Get-Process -Name 'msedgewebview2' -ErrorAction SilentlyContinue | Select-Object -First 1
                    if ($null -ne $webViewProcess) { $webViewVersion = $webViewProcess.MainModule.FileVersionInfo.FileVersion }
                }
            }
            catch [System.Windows.Automation.ElementNotAvailableException] { continue }
            catch {
                # WebView2 may invalidate an AutomationElement between FindAll and a
                # pattern/property read. Treat only that known transient null race as
                # a retry; every other probe failure remains fatal.
                if ($_.Exception.Message -like '*null-valued expression*') { continue }
                throw
            }
        } while (-not $process.HasExited -and -not $ready -and [DateTime]::UtcNow -lt $deadline)

        $stopwatch.Stop()
        if (-not $ready) {
            $preview = (@($observed | Select-Object -First 50) -join ' | ')
            throw "Sample $sample did not expose visible health and demo catalog evidence within $TimeoutSeconds seconds. $readiness observed=$preview"
        }
        $durations.Add($stopwatch.Elapsed.TotalMilliseconds)

        if (-not $process.CloseMainWindow() -or -not $process.WaitForExit(10000)) {
            throw "Sample $sample did not close normally."
        }
        if ($process.ExitCode -ne 0) {
            throw "Sample $sample exited with code $($process.ExitCode)."
        }
        $process = $null
        if (@(Get-Process -Name 'ai-subscribe-desktop' -ErrorAction SilentlyContinue).Count -ne 0) {
            throw "Sample $sample left a residual application process."
        }
        $childDeadline = [DateTime]::UtcNow.AddSeconds(5)
        do {
            $newWebViewProcesses = @(Get-Process -Name 'msedgewebview2' -ErrorAction SilentlyContinue |
                Where-Object { $_.Id -notin $webViewProcessesBefore })
            if ($newWebViewProcesses.Count -eq 0) { break }
            Start-Sleep -Milliseconds 100
        } while ([DateTime]::UtcNow -lt $childDeadline)
        if ($newWebViewProcesses.Count -ne 0) {
            throw "Sample $sample left residual WebView2 processes."
        }
    }
}
finally {
    if ($null -ne $process -and -not $process.HasExited) {
        $process.Kill()
        $process.WaitForExit()
    }
    $env:LOCALAPPDATA = $previousLocalAppData
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $previousWebViewArguments
    $env:AI_SUBSCRIBE_BENCHMARK_DATA_DIR = $previousBenchmarkDataDir
}

$ordered = @($durations | Sort-Object)
$medianIndex = [Math]::Max(0, [Math]::Ceiling($ordered.Count * 0.50) - 1)
$percentileIndex = [Math]::Max(0, [Math]::Ceiling($ordered.Count * 0.95) - 1)
$p95 = $ordered[$percentileIndex]
$nodeVersion = (& (Join-Path $projectRoot '.toolchains\node\node.exe') --version).TrimStart('v')
$cargoVersion = 'cargo 1.97.1 (project-isolated rust-toolchain.toml)'
$clangVersion = (& (Join-Path $projectRoot '.toolchains\llvm-mingw\bin\clang-cl.exe') --version | Select-Object -First 1)
$package = Get-Content -Raw -Encoding UTF8 (Join-Path $projectRoot 'apps\windows\package.json') | ConvertFrom-Json
$tauriCargo = Get-Content -Raw -Encoding UTF8 (Join-Path $projectRoot 'apps\windows\src-tauri\Cargo.toml')
$tauriCoreVersion = [regex]::Match($tauriCargo, 'tauri\s*=\s*\{\s*version\s*=\s*"=([^"]+)"').Groups[1].Value
$sourceFingerprints = [ordered]@{}
foreach ($relativeSource in @(
    'scripts\windows-demo-smoke.ps1',
    'crates\radar-core\src\application\demo.rs',
    'apps\windows\src\app\shell\app-shell.tsx',
    'apps\windows\src\features\demo-intelligence\demo-intelligence.tsx',
    'apps\windows\src-tauri\src\lib.rs',
    'apps\windows\src-tauri\Cargo.toml',
    'contracts\fixtures\demo\manifest-v1.json',
    'Cargo.lock',
    'pnpm-lock.yaml'
)) {
    $sourceFingerprints[$relativeSource] = (Get-FileHash -Algorithm SHA256 (Join-Path $projectRoot $relativeSource)).Hash
}
$result = [pscustomobject]@{
    samples = $Samples
    successful_samples = $durations.Count
    success_rate = $durations.Count / $Samples
    p50_ms = [Math]::Round($ordered[$medianIndex], 2)
    p95_ms = [Math]::Round($p95, 2)
    minimum_ms = [Math]::Round($ordered[0], 2)
    maximum_ms = [Math]::Round($ordered[-1], 2)
    threshold_ms = 5000
    passed = $p95 -le 5000
    evidence_root = $runRoot.FullName
    measured_at_utc = [DateTime]::UtcNow.ToString('o')
    os = [Environment]::OSVersion.VersionString
    architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    device = $env:COMPUTERNAME
    webview2 = $webViewVersion
    rust_cargo = $cargoVersion
    tauri = $package.devDependencies.'@tauri-apps/cli'
    tauri_core = $tauriCoreVersion
    node = $nodeVersion
    pnpm = ((Get-Content -Raw -Encoding UTF8 (Join-Path $projectRoot 'package.json') | ConvertFrom-Json).packageManager -split '@')[-1]
    compiler = $clangVersion
    msvc_target = 'x86_64-pc-windows-msvc'
    windows_sdk = 'project-local xwin sysroot'
    fixture = 'demo-v1'
    instrumentation = 'benchmark-instrumentation'
    candidate_sha256 = (Get-FileHash -Algorithm SHA256 $executable).Hash
    source_sha256 = $sourceFingerprints
    raw_samples_ms = @($durations | ForEach-Object { [Math]::Round($_, 2) })
}
$evidencePath = Join-Path $runRoot.FullName 'windows-demo-cold-start.json'
[System.IO.File]::WriteAllText(
    $evidencePath,
    ($result | ConvertTo-Json -Depth 4),
    [System.Text.UTF8Encoding]::new($false)
)
$result
if (-not $result.passed) {
    throw "Story 1.6 cold-start P95 exceeded 5000 ms: $($result.p95_ms) ms."
}
