[CmdletBinding()]
param(
    [ValidateRange(1, 100)]
    [int]$Samples = 1,
    [ValidateRange(1, 30)]
    [int]$TimeoutSeconds = 15,
    [switch]$Evidence,
    [switch]$AccessibilityEvidence,
    [ValidateSet('story-1-6', 'story-2-1', 'story-2-2')]
    [string]$EvidenceStory = 'story-1-6'
)

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$pnpmEnvironment = Join-Path $projectRoot 'scripts\pnpm-env.cmd'
$rustEnvironment = Join-Path $projectRoot 'scripts\rust-msvc-env.cmd'
& $pnpmEnvironment build
if ($LASTEXITCODE -ne 0) { throw "Windows frontend build failed with exit code $LASTEXITCODE." }
& $rustEnvironment build -p ai-subscribe-desktop --release --features benchmark-instrumentation
if ($LASTEXITCODE -ne 0) { throw "Windows release build failed with exit code $LASTEXITCODE." }
$releaseRoot = (Resolve-Path (Join-Path $projectRoot 'target\x86_64-pc-windows-msvc\release')).Path
$executable = (Resolve-Path (Join-Path $releaseRoot 'ai-subscribe-desktop.exe')).Path
if (-not $executable.StartsWith($releaseRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'The candidate executable resolved outside the project release directory.'
}
if (@(Get-Process -Name 'ai-subscribe-desktop' -ErrorAction SilentlyContinue).Count -ne 0) {
    throw 'The demo smoke requires zero existing ai-subscribe-desktop processes.'
}
if ($Evidence -and $Samples -ne 30) {
    throw 'Formal Windows milestone evidence requires exactly 30 samples.'
}

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName UIAutomationClientsideProviders
Add-Type -AssemblyName System.Windows.Forms

function Get-ContentSnapshot(
    [System.Windows.Automation.AutomationElement]$Root,
    [int]$MaximumElements = 500
) {
    $walker = [System.Windows.Automation.TreeWalker]::ContentViewWalker
    $stack = [System.Collections.Generic.Stack[System.Windows.Automation.AutomationElement]]::new()
    $elementsById = @{}
    $names = [System.Collections.Generic.List[string]]::new()
    $stack.Push($Root)
    $visited = 0

    while ($stack.Count -gt 0 -and $visited -lt $MaximumElements) {
        $element = $stack.Pop()
        $visited += 1
        $automationId = $element.Current.AutomationId
        if ($automationId -and -not $elementsById.ContainsKey($automationId)) {
            $elementsById[$automationId] = $element
        }
        if (-not $element.Current.IsOffscreen -and $element.Current.Name) {
            $names.Add($element.Current.Name)
        }

        $children = [System.Collections.Generic.List[System.Windows.Automation.AutomationElement]]::new()
        $child = $walker.GetFirstChild($element)
        while ($null -ne $child) {
            $children.Add($child)
            $child = $walker.GetNextSibling($child)
        }
        for ($index = $children.Count - 1; $index -ge 0; $index -= 1) {
            $stack.Push($children[$index])
        }
    }

    [pscustomobject]@{
        ElementsById = $elementsById
        Names = $names
        Visited = $visited
        Truncated = $stack.Count -gt 0
    }
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

function Write-JsonUtf8([string]$Path, [object]$Value, [int]$Depth = 6) {
    $json = $Value | ConvertTo-Json -Depth $Depth
    [System.IO.File]::WriteAllText(
        $Path,
        $json,
        [System.Text.UTF8Encoding]::new($false)
    )
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
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class DemoMouseProbe {
    [DllImport("user32.dll")] private static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] private static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] private static extern bool ShowWindow(IntPtr window, int command);
    [DllImport("user32.dll")] private static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] private static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    public static void Activate(IntPtr window) {
        ShowWindow(window, 5);
        SetForegroundWindow(window);
    }
    public static void Click(double x, double y) {
        SetCursorPos((int)x, (int)y);
        mouse_event(0x0002, 0, 0, 0, UIntPtr.Zero);
        mouse_event(0x0004, 0, 0, 0, UIntPtr.Zero);
    }
    public static void KeyPress(byte key) {
        keybd_event(key, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, 0x0002, UIntPtr.Zero);
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
$activeOwnedProcessIds = $null
$sourcePaths = @(
    'scripts\windows-demo-smoke.ps1',
    'scripts\windows-rss-smoke.ps1',
    'crates\radar-core\src\application\demo.rs',
    'crates\radar-core\src\application\setup.rs',
    'crates\radar-core\src\application\configuration.rs',
    'crates\radar-core\src\application\sources.rs',
    'crates\radar-core\src\contracts\dto\configuration_validation.rs',
    'crates\radar-core\src\contracts\dto\source.rs',
    'crates\radar-core\src\domain\rules\configuration_validation.rs',
    'crates\radar-core\src\domain\sources\mod.rs',
    'crates\radar-core\src\infrastructure\http\source_http_policy.rs',
    'crates\radar-core\src\infrastructure\sources\rss_atom\mod.rs',
    'apps\windows\src\app\shell\app-shell.tsx',
    'apps\windows\src\app\router\app-router.tsx',
    'apps\windows\src\features\demo-intelligence\demo-intelligence.tsx',
    'apps\windows\src\features\configuration-validation\configuration-editor.tsx',
    'apps\windows\src\features\sources\sources-page.tsx',
    'apps\windows\src\lib\desktop-api\desktop-api.ts',
    'apps\windows\src\lib\desktop-api\tauri-desktop-api.ts',
    'apps\windows\src\lib\query-client.ts',
    'apps\windows\src\features\setup-guide\progressive-setup-guide.tsx',
    'apps\windows\src-tauri\src\commands\mod.rs',
    'apps\windows\src-tauri\src\lib.rs',
    'apps\windows\src-tauri\Cargo.toml',
    'apps\windows\src-tauri\tauri.conf.json',
    'apps\windows\src-tauri\capabilities\main.json',
    'crates\radar-core\Cargo.toml',
    'contracts\schemas\contract-manifest-v1.json',
    'contracts\snapshots\error-codes-v1.json',
    'contracts\fixtures\golden\source_view_v1.json',
    'contracts\fixtures\demo\manifest-v1.json',
    'contracts\fixtures\setup\defaults-v1.json',
    'contracts\fixtures\configuration-validation\blocking\cases-v1.json',
    'contracts\fixtures\configuration-validation\narrowing\cases-v1.json',
    'contracts\fixtures\configuration-validation\valid\basic-v1.json',
    'contracts\fixtures\rss-atom\rss2-v1.xml',
    'contracts\fixtures\rss-atom\atom-v1.xml',
    'apps\windows\package.json',
    'Cargo.lock',
    'pnpm-lock.yaml'
)
function Get-SourceFingerprints {
    $fingerprints = [ordered]@{}
    foreach ($relativeSource in $sourcePaths) {
        $fingerprints[$relativeSource] = (
            Get-FileHash -Algorithm SHA256 (Join-Path $projectRoot $relativeSource)
        ).Hash
    }
    $fingerprints
}
$candidateShaBefore = (Get-FileHash -Algorithm SHA256 $executable).Hash
Assert-ReleaseCandidateSurface -ExecutablePath $executable
$sourceFingerprintsBefore = Get-SourceFingerprints
$runManifest = [pscustomobject]@{
    schema_version = 1
    run_id = $runId
    requested_samples = $Samples
    evidence_story = $EvidenceStory
    candidate_sha256 = $candidateShaBefore
    source_sha256 = $sourceFingerprintsBefore
    started_at_utc = [DateTime]::UtcNow.ToString('o')
}
Write-JsonUtf8 -Path (Join-Path $runRoot.FullName 'run-manifest.json') -Value $runManifest

try {
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--force-renderer-accessibility'
    for ($sample = 1; $sample -le $Samples; $sample += 1) {
        $sampleRoot = New-Item -ItemType Directory -Force (Join-Path $runRoot.FullName ("sample-{0:D3}" -f $sample))
        $env:LOCALAPPDATA = $sampleRoot.FullName
        $env:AI_SUBSCRIBE_BENCHMARK_DATA_DIR = $sampleRoot.FullName
        $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
        $process = Start-Process -FilePath $executable -PassThru
        $rootProcessId = $process.Id
        $activeOwnedProcessIds = [System.Collections.Generic.HashSet[int]]::new()
        $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
        $ready = $false
        $observed = @()
        $readiness = 'not-probed'
        $keyboardActivated = $false
        $detailFocusObserved = $false
        $returnRequested = $false
        $returnFocusObserved = $false
        $listContextStable = $false
        $initialListRectangle = $null

        do {
            Start-Sleep -Milliseconds 100
            $process.Refresh()
            if ($process.HasExited -or $process.MainWindowHandle -eq 0) { continue }
            try {
                $root = [System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
                if ($root.Current.IsOffscreen) { continue }
                $snapshot = Get-ContentSnapshot -Root $root
                $observed = $snapshot.Names
                $healthy = @($snapshot.Names | Where-Object { $_.Contains('healthy') }).Count -ge 1
                $contract = @(
                    $snapshot.Names | Where-Object { $_.Contains('contract_version: 1') }
                ).Count -ge 1
                $list = $snapshot.ElementsById['demo-intelligence-list']
                $detail = $snapshot.ElementsById['demo-intelligence-detail']
                $listReady = $false
                $detailReady = $false
                $scrollSurfaceReady = $false
                if ($null -ne $list) {
                    $selectedListItem = $snapshot.ElementsById['demo-item-demo:rust-197-001']
                    $listReady = $null -ne $selectedListItem
                    $scrollSurfaceReady = $list.Current.BoundingRectangle.Height -gt 0
                    $scrollPattern = $null
                    if ($list.TryGetCurrentPattern(
                        [System.Windows.Automation.ScrollPattern]::Pattern,
                        [ref]$scrollPattern
                    ) -and $null -ne $scrollPattern) {
                        $scrollSurfaceReady = $true
                    }
                    if ($listReady -and -not $keyboardActivated) {
                        $initialListRectangle = $list.Current.BoundingRectangle
                        $scrollItemPattern = $null
                        if ($selectedListItem.TryGetCurrentPattern(
                            [System.Windows.Automation.ScrollItemPattern]::Pattern,
                            [ref]$scrollItemPattern
                        ) -and $null -ne $scrollItemPattern) {
                            $scrollItemPattern.ScrollIntoView()
                        }
                        if ($AccessibilityEvidence) {
                            [DemoMouseProbe]::Activate($process.MainWindowHandle)
                            $selectedListItem.SetFocus()
                            Start-Sleep -Milliseconds 100
                            $focusedBeforeEnter = [System.Windows.Automation.AutomationElement]::FocusedElement
                            if ($null -ne $focusedBeforeEnter -and
                                $focusedBeforeEnter.Current.AutomationId -eq 'demo-item-demo:rust-197-001') {
                                [DemoMouseProbe]::KeyPress(0x0D)
                                $keyboardActivated = $true
                            }
                        } else {
                            $invokePattern = $null
                            if ($selectedListItem.TryGetCurrentPattern(
                                [System.Windows.Automation.InvokePattern]::Pattern,
                                [ref]$invokePattern
                            ) -and $null -ne $invokePattern) {
                                $invokePattern.Invoke()
                                $keyboardActivated = $true
                            }
                        }
                        if (-not $keyboardActivated -and -not $AccessibilityEvidence) {
                            [DemoMouseProbe]::Activate($process.MainWindowHandle)
                            $point = $selectedListItem.GetClickablePoint()
                            [DemoMouseProbe]::Click($point.X, $point.Y)
                            $keyboardActivated = $true
                        }
                    }
                }
                $semanticSectionsReady = @(
                    'what-happened-heading',
                    'why-it-matters-heading',
                    'possible-impact-heading',
                    'facts-heading',
                    'rules-heading',
                    'ai-heading',
                    'demo-provenance-heading'
                ) | Where-Object { -not $snapshot.ElementsById.ContainsKey($_) }
                $semanticSectionsReady = $semanticSectionsReady.Count -eq 0
                if ($null -ne $detail) {
                    $detailTitle = $snapshot.ElementsById['demo-detail-title']
                    if ($null -ne $detailTitle) {
                        $detailReady = $detailTitle.Current.Name.Contains('Rust 1.97')
                    }
                }
                $focusedElement = [System.Windows.Automation.AutomationElement]::FocusedElement
                $focusedId = if ($null -ne $focusedElement) {
                    $focusedElement.Current.AutomationId
                } else { '' }
                if ($AccessibilityEvidence -and $keyboardActivated -and $detailReady -and
                    $focusedId -eq 'demo-detail-title' -and -not $returnRequested) {
                    $detailFocusObserved = $true
                    [DemoMouseProbe]::KeyPress(0x1B)
                    $returnRequested = $true
                }
                if ($returnRequested -and $focusedId -eq 'demo-item-demo:rust-197-001') {
                    $returnFocusObserved = $true
                    $currentListRectangle = $list.Current.BoundingRectangle
                    $listContextStable = $null -ne $initialListRectangle -and
                        [Math]::Abs($currentListRectangle.X - $initialListRectangle.X) -le 1 -and
                        [Math]::Abs($currentListRectangle.Y - $initialListRectangle.Y) -le 1 -and
                        [Math]::Abs($currentListRectangle.Width - $initialListRectangle.Width) -le 1 -and
                        [Math]::Abs($currentListRectangle.Height - $initialListRectangle.Height) -le 1
                }
                # The fixed three-item fixture can fit without exposing ScrollPattern;
                # native readiness therefore requires a visible, positive-height list
                # surface. Overflow behavior is covered separately at responsive sizes.
                $accessibilityReady = -not $AccessibilityEvidence -or
                    ($detailFocusObserved -and $returnFocusObserved -and $listContextStable)
                $ready = $healthy -and $contract -and $listReady -and $keyboardActivated -and
                    $detailReady -and $semanticSectionsReady -and $scrollSurfaceReady -and
                    $accessibilityReady
                $readiness = "healthy=$healthy contract=$contract list=$listReady interaction=$keyboardActivated detail=$detailReady semantic_sections=$semanticSectionsReady accessibility_mode=$AccessibilityEvidence focused_id=$focusedId detail_focus=$detailFocusObserved return_focus=$returnFocusObserved list_context=$listContextStable scroll_surface=$scrollSurfaceReady visited=$($snapshot.Visited) truncated=$($snapshot.Truncated)"
                if ($null -eq $webViewVersion) {
                    $webViewProcess = @($activeOwnedProcessIds | ForEach-Object {
                        Get-Process -Id $_ -ErrorAction SilentlyContinue
                    } | Where-Object { $_.ProcessName -eq 'msedgewebview2' } | Select-Object -First 1)
                    $webViewProcess = $webViewProcess | Select-Object -First 1
                    if ($null -ne $webViewProcess) { $webViewVersion = $webViewProcess.MainModule.FileVersionInfo.FileVersion }
                }
            }
            catch [System.Windows.Automation.ElementNotAvailableException] { continue }
            catch {
                # WebView2 may invalidate an AutomationElement between snapshot and a
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
        foreach ($ownedId in @(Get-DescendantProcessIds -RootProcessId $process.Id)) {
            [void]$activeOwnedProcessIds.Add($ownedId)
        }
        if ($null -eq $webViewVersion) {
            $ownedWebView = @($activeOwnedProcessIds | ForEach-Object {
                Get-Process -Id $_ -ErrorAction SilentlyContinue
            } | Where-Object { $_.ProcessName -eq 'msedgewebview2' } | Select-Object -First 1)
            $ownedWebView = $ownedWebView | Select-Object -First 1
            if ($null -ne $ownedWebView) {
                $webViewVersion = $ownedWebView.MainModule.FileVersionInfo.FileVersion
            }
        }

        if (-not $process.CloseMainWindow()) {
            throw "Sample $sample did not accept a normal close request."
        }
        $closeDeadline = [DateTime]::UtcNow.AddSeconds(10)
        while (-not $process.HasExited -and [DateTime]::UtcNow -lt $closeDeadline) {
            foreach ($ownedId in @(Get-DescendantProcessIds -RootProcessId $process.Id)) {
                [void]$activeOwnedProcessIds.Add($ownedId)
            }
            [void]$process.WaitForExit(100)
            $process.Refresh()
        }
        if (-not $process.HasExited) {
            throw "Sample $sample did not close normally within 10 seconds."
        }
        if ($process.ExitCode -ne 0) {
            throw "Sample $sample exited with code $($process.ExitCode)."
        }
        foreach ($ownedId in @(Get-DescendantProcessIds -RootProcessId $process.Id)) {
            [void]$activeOwnedProcessIds.Add($ownedId)
        }
        $process = $null
        if (@(Get-Process -Name 'ai-subscribe-desktop' -ErrorAction SilentlyContinue).Count -ne 0) {
            throw "Sample $sample left a residual application process."
        }
        $childDeadline = [DateTime]::UtcNow.AddSeconds(5)
        do {
            foreach ($ownedId in @(Get-DescendantProcessIds -RootProcessId $rootProcessId)) {
                [void]$activeOwnedProcessIds.Add($ownedId)
            }
            $remainingOwned = @($activeOwnedProcessIds | Where-Object {
                $null -ne (Get-Process -Id $_ -ErrorAction SilentlyContinue)
            })
            if ($remainingOwned.Count -eq 0) { break }
            Start-Sleep -Milliseconds 100
        } while ([DateTime]::UtcNow -lt $childDeadline)
        if ($remainingOwned.Count -ne 0) {
            throw "Sample $sample left residual owned processes: $($remainingOwned -join ', ')."
        }
        $sampleResult = [pscustomobject]@{
            schema_version = 1
            sample = $sample
            duration_ms = [Math]::Round($stopwatch.Elapsed.TotalMilliseconds, 2)
            ready = $true
            process_tree_zero = $true
            completed_at_utc = [DateTime]::UtcNow.ToString('o')
        }
        Write-JsonUtf8 -Path (Join-Path $sampleRoot.FullName 'sample-result.json') -Value $sampleResult
        $activeOwnedProcessIds = $null
    }
}
finally {
    try {
        if ($null -ne $process -and -not $process.HasExited) {
            $process.Kill()
            if (-not $process.WaitForExit(10000)) {
                throw 'Timed out while terminating the candidate process.'
            }
        }
        if ($null -ne $activeOwnedProcessIds) {
            foreach ($ownedId in @($activeOwnedProcessIds)) {
                Stop-Process -Id $ownedId -Force -ErrorAction SilentlyContinue
            }
        }
    }
    finally {
        $env:LOCALAPPDATA = $previousLocalAppData
        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $previousWebViewArguments
        $env:AI_SUBSCRIBE_BENCHMARK_DATA_DIR = $previousBenchmarkDataDir
    }
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
$sourceFingerprints = Get-SourceFingerprints
$candidateShaAfter = (Get-FileHash -Algorithm SHA256 $executable).Hash
if ($candidateShaAfter -ne $candidateShaBefore) {
    throw 'Release candidate changed while cold-start evidence was being collected.'
}
foreach ($relativeSource in $sourcePaths) {
    if ($sourceFingerprints[$relativeSource] -ne $sourceFingerprintsBefore[$relativeSource]) {
        throw "Source changed while cold-start evidence was being collected: $relativeSource"
    }
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
    passed = if ($AccessibilityEvidence) {
        $durations.Count -eq $Samples
    } else {
        $p95 -le 5000
    }
    evidence_root = Get-ProjectRelativePath -BasePath $projectRoot -TargetPath $runRoot.FullName
    measured_at_utc = [DateTime]::UtcNow.ToString('o')
    os = [Environment]::OSVersion.VersionString
    architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    device_profile_sha256 = [BitConverter]::ToString(
        [Security.Cryptography.SHA256]::Create().ComputeHash(
            [Text.Encoding]::UTF8.GetBytes([string]$env:COMPUTERNAME)
        )
    ).Replace('-', '')
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
    accessibility_evidence = [bool]$AccessibilityEvidence
    candidate_sha256 = $candidateShaAfter
    source_sha256 = $sourceFingerprints
    raw_samples_ms = @($durations | ForEach-Object { [Math]::Round($_, 2) })
}
$evidencePath = Join-Path $runRoot.FullName 'windows-demo-cold-start.json'
$evidenceJson = $result | ConvertTo-Json -Depth 4
Write-JsonUtf8 -Path $evidencePath -Value $result
if ($Evidence) {
    $durableEvidence = Join-Path $projectRoot "_agentic-out\tests\evidence\$EvidenceStory-cold-start.json"
    New-Item -ItemType Directory -Force (Split-Path $durableEvidence) | Out-Null
    Write-JsonUtf8 -Path $durableEvidence -Value $result
}
$result
if (-not $AccessibilityEvidence -and -not $result.passed) {
    throw "Story 1.6 cold-start P95 exceeded 5000 ms: $($result.p95_ms) ms."
}
