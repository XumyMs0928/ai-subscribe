[CmdletBinding()]
param([ValidateRange(5, 60)][int]$TimeoutSeconds = 20)

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$releaseRoot = (Resolve-Path (Join-Path $projectRoot 'target\x86_64-pc-windows-msvc\release')).Path
$executable = (Resolve-Path (Join-Path $releaseRoot 'ai-subscribe-desktop.exe')).Path
if (-not $executable.StartsWith($releaseRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Candidate resolved outside the project release directory.'
}
if (@(Get-Process -Name 'ai-subscribe-desktop' -ErrorAction SilentlyContinue).Count -ne 0) {
    throw 'Setup smoke requires zero existing ai-subscribe-desktop processes.'
}

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class SetupSmokeInput {
    [DllImport("user32.dll")] private static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] private static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] private static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] private static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    public static void Click(IntPtr window, double x, double y) {
        SetForegroundWindow(window);
        SetCursorPos((int)x, (int)y);
        mouse_event(0x0002, 0, 0, 0, UIntPtr.Zero);
        mouse_event(0x0004, 0, 0, 0, UIntPtr.Zero);
    }
    public static void Enter(IntPtr window) {
        SetForegroundWindow(window);
        keybd_event(0x0D, 0, 0, UIntPtr.Zero);
        keybd_event(0x0D, 0, 0x0002, UIntPtr.Zero);
    }
}
'@
$runId = [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss-fff')
$runRoot = New-Item -ItemType Directory -Force (Join-Path $projectRoot "target\story-1-6-benchmark\story-1-8-$runId")
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
            if ($null -ne $element -and -not $element.Current.IsOffscreen) { return $element }
        }
    } while (-not $Process.HasExited -and [DateTime]::UtcNow -lt $deadline)
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($Process.MainWindowHandle)
    $all = $root.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.Condition]::TrueCondition
    )
    $observed = for ($index = 0; $index -lt [Math]::Min($all.Count, 100); $index += 1) {
        $candidate = $all.Item($index)
        if ($candidate.Current.AutomationId -or $candidate.Current.Name) {
            "$($candidate.Current.AutomationId)=$($candidate.Current.Name)"
        }
    }
    throw "Visible automation element '$AutomationId' was not available within $TimeoutSeconds seconds. observed=$($observed -join ' | ')"
}

function Wait-FocusedId([string]$AutomationId) {
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    do {
        Start-Sleep -Milliseconds 100
        $focused = [System.Windows.Automation.AutomationElement]::FocusedElement
        if ($null -ne $focused -and $focused.Current.AutomationId -eq $AutomationId) {
            return $focused
        }
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Focus did not move to '$AutomationId' within $TimeoutSeconds seconds."
}

function Get-OwnedProcessIds([int]$RootProcessId) {
    $all = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue)
    $owned = [System.Collections.Generic.HashSet[int]]::new()
    $queue = [System.Collections.Generic.Queue[int]]::new()
    $queue.Enqueue($RootProcessId)
    while ($queue.Count -gt 0) {
        $parent = $queue.Dequeue()
        foreach ($child in $all | Where-Object { $_.ParentProcessId -eq $parent }) {
            if ($owned.Add([int]$child.ProcessId)) {
                $queue.Enqueue([int]$child.ProcessId)
            }
        }
    }
    @($owned)
}

function Stop-OwnedProcessTree([int]$RootProcessId) {
    $owned = @(Get-OwnedProcessIds -RootProcessId $RootProcessId)
    foreach ($processId in @($owned | Sort-Object -Descending)) {
        Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
    }
    Stop-Process -Id $RootProcessId -Force -ErrorAction SilentlyContinue
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        $remaining = @($owned + $RootProcessId | Where-Object {
            $null -ne (Get-Process -Id $_ -ErrorAction SilentlyContinue)
        })
        if ($remaining.Count -eq 0) { return }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Candidate process tree cleanup timed out: $($remaining -join ',')."
}

function Invoke-Element([System.Windows.Automation.AutomationElement]$Element) {
    $pattern = $null
    if (-not $Element.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) {
        throw "Element '$($Element.Current.AutomationId)' does not support InvokePattern."
    }
    $pattern.Invoke()
}

function Click-Element(
    [System.Diagnostics.Process]$Process,
    [System.Windows.Automation.AutomationElement]$Element
) {
    $point = $Element.GetClickablePoint()
    [SetupSmokeInput]::Click($Process.MainWindowHandle, $point.X, $point.Y)
}

function Keyboard-Activate(
    [System.Diagnostics.Process]$Process,
    [System.Windows.Automation.AutomationElement]$Element
) {
    $Element.SetFocus()
    [SetupSmokeInput]::Enter($Process.MainWindowHandle)
}

function Start-Candidate {
    $candidate = Start-Process -FilePath $executable -PassThru
    Wait-ById -Process $candidate -AutomationId 'app-nav-settings' | Out-Null
    $candidate
}

function Open-Setup([System.Diagnostics.Process]$Process) {
    Invoke-Element (Wait-ById -Process $Process -AutomationId 'app-nav-settings')
    Invoke-Element (Wait-ById -Process $Process -AutomationId 'setup-guide-entry')
}

function Stop-Candidate([System.Diagnostics.Process]$Process) {
    $rootProcessId = $Process.Id
    if (-not $Process.HasExited) {
        if (-not $Process.CloseMainWindow() -or -not $Process.WaitForExit(10000)) {
            Stop-OwnedProcessTree -RootProcessId $rootProcessId
            throw 'Candidate did not close normally.'
        }
    }
    Stop-OwnedProcessTree -RootProcessId $rootProcessId
}

try {
    $env:LOCALAPPDATA = $runRoot.FullName
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--force-renderer-accessibility'
    $env:AI_SUBSCRIBE_BENCHMARK_DATA_DIR = $runRoot.FullName
    $process = Start-Candidate
    Open-Setup -Process $process
    $track = Wait-ById -Process $process -AutomationId 'setup-option-ai_agents'
    $toggle = $null
    if (-not $track.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$toggle)) {
        throw 'Track option does not support TogglePattern.'
    }
    $toggle.Toggle()
    Keyboard-Activate -Process $process -Element (Wait-ById -Process $process -AutomationId 'setup-save')
    Wait-ById -Process $process -AutomationId 'setup-option-github_releases' | Out-Null
    Stop-Candidate -Process $process
    $process = $null

    $process = Start-Candidate
    Open-Setup -Process $process
    $restored = Wait-ById -Process $process -AutomationId 'setup-option-github_releases'
    Keyboard-Activate -Process $process -Element (Wait-ById -Process $process -AutomationId 'setup-return-settings')
    Wait-ById -Process $process -AutomationId 'setup-guide-entry' | Out-Null
    $returnFocus = Wait-FocusedId -AutomationId 'setup-guide-entry'
    $evidence = [ordered]@{
        story = '1.8'
        passed = $true
        candidate = $executable
        candidate_sha256 = (Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash.ToLowerInvariant()
        data_root = $runRoot.FullName
        restored_automation_id = $restored.Current.AutomationId
        return_focus_automation_id = $returnFocus.Current.AutomationId
        keyboard_save = $true
        keyboard_return = $true
        external_permissions_changed = 'not_measured'
        completed_at_utc = [DateTime]::UtcNow.ToString('o')
    }
    $evidencePath = Join-Path $runRoot.FullName 'native-setup-smoke.json'
    $evidence | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $evidencePath -Encoding UTF8
    Write-Output "Story 1.8 native setup smoke PASS: $evidencePath"
}
finally {
    if ($null -ne $process) {
        Stop-OwnedProcessTree -RootProcessId $process.Id
    }
    $env:LOCALAPPDATA = $previousLocalAppData
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $previousWebViewArguments
    $env:AI_SUBSCRIBE_BENCHMARK_DATA_DIR = $previousBenchmarkDataDir
}
