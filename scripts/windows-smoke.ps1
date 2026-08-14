[CmdletBinding()]
param(
    [ValidateRange(1, 60)]
    [int]$TimeoutSeconds = 60
)

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$executable = (Resolve-Path (Join-Path $projectRoot 'target\x86_64-pc-windows-msvc\release\ai-subscribe-desktop.exe')).Path
$expectedRoot = Join-Path $projectRoot 'target\x86_64-pc-windows-msvc\release'
if (-not $executable.StartsWith($expectedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'Smoke executable resolved outside the project target directory.'
}

if (@(Get-Process -Name 'ai-subscribe-desktop' -ErrorAction SilentlyContinue).Count -ne 0) {
    throw 'Smoke requires zero existing ai-subscribe-desktop processes.'
}

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName UIAutomationClientsideProviders
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public static class StoryWindowProbe
{
    private delegate bool EnumWindowsProc(IntPtr window, IntPtr state);
    private static int targetProcessId;
    private static int visibleWindowCount;
    private static int allowlistedAuxiliaryWindowCount;
    private static IntPtr visibleMainWindow;

    [DllImport("user32.dll")]
    private static extern bool EnumWindows(EnumWindowsProc callback, IntPtr state);

    [DllImport("user32.dll")]
    private static extern bool EnumChildWindows(IntPtr parent, EnumWindowsProc callback, IntPtr state);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

    [DllImport("user32.dll")]
    private static extern bool IsWindowVisible(IntPtr window);

    [DllImport("user32.dll")]
    private static extern IntPtr GetWindow(IntPtr window, uint command);

    [DllImport("user32.dll")]
    private static extern int GetWindowTextLength(IntPtr window);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(IntPtr window, StringBuilder className, int maximumCount);

    [DllImport("user32.dll")]
    private static extern bool PostMessage(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);

    private static bool IsAllowlistedTaoEventTarget(IntPtr window)
    {
        var className = new StringBuilder(256);
        GetClassName(window, className, className.Capacity);
        return className.ToString() == "Tao Thread Event Target"
            && GetWindowTextLength(window) == 0;
    }

    private static bool CountWindow(IntPtr window, IntPtr state)
    {
        uint processId;
        GetWindowThreadProcessId(window, out processId);
        if (processId == targetProcessId && IsWindowVisible(window))
        {
            if (IsAllowlistedTaoEventTarget(window))
            {
                allowlistedAuxiliaryWindowCount += 1;
                return true;
            }
            visibleWindowCount += 1;
            if (visibleMainWindow == IntPtr.Zero)
            {
                visibleMainWindow = window;
            }
        }
        return true;
    }

    public static int CountVisibleTopLevelWindows(int expectedProcessId)
    {
        targetProcessId = expectedProcessId;
        visibleWindowCount = 0;
        allowlistedAuxiliaryWindowCount = 0;
        visibleMainWindow = IntPtr.Zero;
        EnumWindows(CountWindow, IntPtr.Zero);
        return visibleWindowCount;
    }

    public static IntPtr GetLastVisibleMainWindow()
    {
        return visibleMainWindow;
    }

    public static int GetLastAllowlistedAuxiliaryWindowCount()
    {
        return allowlistedAuxiliaryWindowCount;
    }

    public static bool RequestNormalClose(IntPtr window)
    {
        return PostMessage(window, 0x0010, IntPtr.Zero, IntPtr.Zero);
    }

    public static IntPtr[] GetChildWindows(IntPtr parent)
    {
        var windows = new List<IntPtr>();
        EnumChildWindows(parent, delegate(IntPtr window, IntPtr state) {
            windows.Add(window);
            return true;
        }, IntPtr.Zero);
        return windows.ToArray();
    }
}
'@
$projectLocalAppData = (New-Item -ItemType Directory -Force (Join-Path $projectRoot 'target\story-1-2-local-app-data')).FullName
$previousLocalAppData = $env:LOCALAPPDATA
$previousWebViewArguments = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$process = $null
try {
    $env:LOCALAPPDATA = $projectLocalAppData
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--force-renderer-accessibility'
    $process = Start-Process -FilePath $executable -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    $names = @()
    $healthyVisible = $false
    $contractVersionVisible = $false
    $visibleWindowCount = 0
    $allowlistedAuxiliaryWindowCount = 0
    $mainWindowHandle = [IntPtr]::Zero
    $rootOffscreen = $null
    $rootName = ''
    $elementCount = 0
    do {
        Start-Sleep -Milliseconds 500
        $process.Refresh()
        if (-not $process.HasExited -and $process.MainWindowHandle -ne 0) {
            $visibleWindowCount = [StoryWindowProbe]::CountVisibleTopLevelWindows($process.Id)
            $allowlistedAuxiliaryWindowCount = [StoryWindowProbe]::GetLastAllowlistedAuxiliaryWindowCount()
            $mainWindowHandle = [StoryWindowProbe]::GetLastVisibleMainWindow()
            if ($mainWindowHandle -eq [IntPtr]::Zero) {
                continue
            }
            $root = [System.Windows.Automation.AutomationElement]::FromHandle($mainWindowHandle)
            $rootOffscreen = $root.Current.IsOffscreen
            $rootName = $root.Current.Name
            if ($visibleWindowCount -ne 1 -or $rootOffscreen) {
                continue
            }
            $healthyCount = 0
            $contractCount = 0
            $elementCount = 0
            $seenRuntimeIds = @{}
            $automationHandles = @($mainWindowHandle) + @([StoryWindowProbe]::GetChildWindows($mainWindowHandle))
            foreach ($handle in $automationHandles) {
                try {
                    $automationRoot = [System.Windows.Automation.AutomationElement]::FromHandle($handle)
                    foreach ($expectedName in @('共享核心 healthy', 'contract_version: 1')) {
                        $condition = if ($expectedName -eq '共享核心 healthy') {
                            [System.Windows.Automation.Condition]::TrueCondition
                        }
                        else {
                            New-Object System.Windows.Automation.PropertyCondition(
                                [System.Windows.Automation.AutomationElement]::NameProperty,
                                $expectedName
                            )
                        }
                        $matches = $automationRoot.FindAll(
                            [System.Windows.Automation.TreeScope]::Subtree,
                            $condition
                        )
                        for ($index = 0; $index -lt $matches.Count; $index += 1) {
                            $candidate = $matches.Item($index)
                            $candidateName = $candidate.Current.Name
                            $nameMatches = if ($expectedName -eq '共享核心 healthy') {
                                $candidateName.Contains($expectedName)
                            }
                            else {
                                $candidateName -eq $expectedName
                            }
                            if (-not $nameMatches) { continue }
                            $elementCount += 1
                            $runtimeId = ($candidate.GetRuntimeId() -join '.')
                            $runtimeKey = "$expectedName|$runtimeId"
                            if (-not $seenRuntimeIds.ContainsKey($runtimeKey) -and
                                -not $candidate.Current.IsOffscreen) {
                                $seenRuntimeIds[$runtimeKey] = $true
                                if ($expectedName -eq '共享核心 healthy') { $healthyCount += 1 }
                                if ($expectedName -eq 'contract_version: 1') { $contractCount += 1 }
                            }
                        }
                    }
                }
                catch [System.Windows.Automation.ElementNotAvailableException] { continue }
            }
            $names = @("healthy=$healthyCount", "contract=$contractCount")
            $healthyVisible = $healthyCount -ge 1
            $contractVersionVisible = $contractCount -ge 1
        }
    } while (
        -not $process.HasExited -and
        (-not $healthyVisible -or -not $contractVersionVisible) -and
        [DateTime]::UtcNow -lt $deadline
    )

    $running = @(Get-Process -Name 'ai-subscribe-desktop' -ErrorAction SilentlyContinue)
    if ($process.HasExited -or $mainWindowHandle -eq [IntPtr]::Zero -or $running.Count -ne 1 -or $visibleWindowCount -ne 1) {
        throw "Release process/window assertion failed: exited=$($process.HasExited), handle=$mainWindowHandle, processes=$($running.Count), visible_windows=$visibleWindowCount, allowlisted_auxiliary_windows=$allowlistedAuxiliaryWindowCount."
    }
    if (-not $healthyVisible -or -not $contractVersionVisible) {
        $observedNames = ($names | ForEach-Object { "[$_]" }) -join ', '
        throw "Accessible healthy/contract_version state was not observed within $TimeoutSeconds seconds. visible_windows=$visibleWindowCount, offscreen=$rootOffscreen, root=$rootName, elements=$elementCount, observed=$observedNames"
    }
    $windowTitle = $process.MainWindowTitle
    if (-not [StoryWindowProbe]::RequestNormalClose($mainWindowHandle)) {
        throw 'The application rejected the normal close request.'
    }
    if (-not $process.WaitForExit(10000)) {
        throw 'The application did not exit normally within 10 seconds.'
    }
    $remaining = @(Get-Process -Name 'ai-subscribe-desktop' -ErrorAction SilentlyContinue)
    if ($process.ExitCode -ne 0 -or $remaining.Count -ne 0) {
        throw 'The application left a non-zero exit code or residual process.'
    }

    [pscustomobject]@{
        title = $windowTitle
        healthy_accessible = $true
        healthy_accessible_count = $healthyCount
        contract_version = 1
        contract_version_accessible_count = $contractCount
        exit_code = $process.ExitCode
        remaining_processes = $remaining.Count
        allowlisted_auxiliary_windows = $allowlistedAuxiliaryWindowCount
        local_app_data = $projectLocalAppData
    }
}
finally {
    if ($null -ne $process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
    }
    $env:LOCALAPPDATA = $previousLocalAppData
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $previousWebViewArguments
}
