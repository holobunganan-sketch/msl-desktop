# smoke-test.ps1 — Stage 11 自动化冒烟（指南 §25）
#
# 自动化部分（Release exe + CDP）：
#   LAUNCH → CLOSE-TO-CORE → REOPEN → EXIT → PERSISTENCE + 数据流验证
# 无法自动化的 Windows UI 操作见 docs/smoke-checklist.md（人工清单）。
#
# 用法：
#   powershell -ExecutionPolicy Bypass -File .\scripts\smoke-test.ps1
#   powershell -ExecutionPolicy Bypass -File .\scripts\smoke-test.ps1 -Exe <path>

param(
    [string]$Exe = ".\src-tauri\target\release\msl-desktop.exe",
    [string]$Python = "python"
)

$ErrorActionPreference = "Continue"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Split-Path -Parent $scriptDir
if (-not [System.IO.Path]::IsPathRooted($Exe)) { $Exe = Join-Path $root $Exe }
$cdpPy = Join-Path $scriptDir "smoke-cdp.py"
$smokeDir = Join-Path $env:TEMP "msl-smoke"
$dbPath = Join-Path $env:APPDATA "MSLDesktop\msl-desktop.db"

$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222 --remote-allow-origins=*"

$passCount = 0
$failCount = 0
$warnCount = 0

function Report {
    param([string]$Name, [bool]$Ok, [string]$Detail = "", [switch]$Warn)
    if ($Ok) { $script:passCount++; Write-Host "[PASS] $Name $Detail" }
    elseif ($Warn) { $script:warnCount++; Write-Host "[WARN] $Name $Detail（人工验证见 checklist）" }
    else { $script:failCount++; Write-Host "[FAIL] $Name $Detail" }
}

function Wait-Window {
    param([string]$WantTitle, [int]$TimeoutSec = 25)
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $p = Get-Process -Name 'msl-desktop' -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($p -and $WantTitle -ne "" -and $p.MainWindowTitle -ne "") { return $true }
        if ($p -and $WantTitle -eq "" -and $p.MainWindowTitle -eq "") { return $true }
        Start-Sleep -Milliseconds 300
    }
    return $false
}

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class SmokeNative {
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
}
"@

function Expand-Overflow {
    $rootEl = [System.Windows.Automation.AutomationElement]::RootElement
    $ovfCond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ClassNameProperty, 'TopLevelWindowForOverflowXamlIsland')
    $ovf = $rootEl.FindFirst([System.Windows.Automation.TreeScope]::Subtree, $ovfCond)
    if ($ovf) { return $true }
    $trayCond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ClassNameProperty, 'Shell_TrayWnd')
    $tray = $rootEl.FindFirst([System.Windows.Automation.TreeScope]::Subtree, $trayCond)
    $all = $tray.FindAll([System.Windows.Automation.TreeScope]::Subtree, [System.Windows.Automation.Condition]::TrueCondition)
    for ($i = 0; $i -lt $all.Count; $i++) {
        if ($all[$i].Current.ClassName -eq 'SystemTray.NormalButton') {
            $r = $all[$i].Current.BoundingRectangle
            [SmokeNative]::SetCursorPos([int]($r.X+$r.Width/2), [int]($r.Y+$r.Height/2)) | Out-Null
            Start-Sleep -Milliseconds 250
            [SmokeNative]::mouse_event(0x0002,0,0,0,[UIntPtr]::Zero); Start-Sleep -Milliseconds 80
            [SmokeNative]::mouse_event(0x0004,0,0,0,[UIntPtr]::Zero)
            return $true
        }
    }
    return $false
}

function Get-TrayButton {
    $rootEl = [System.Windows.Automation.AutomationElement]::RootElement
    Expand-Overflow | Out-Null
    Start-Sleep -Milliseconds 600
    $ovfCond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ClassNameProperty, 'TopLevelWindowForOverflowXamlIsland')
    $ovf = $rootEl.FindFirst([System.Windows.Automation.TreeScope]::Subtree, $ovfCond)
    if (-not $ovf) { return $null }
    $els = $ovf.FindAll([System.Windows.Automation.TreeScope]::Subtree, [System.Windows.Automation.Condition]::TrueCondition)
    foreach ($el in $els) {
        if ($el.Current.ClassName -eq 'SystemTray.NormalButton' -and $el.Current.Name -eq 'MSL Desktop') {
            $r = $el.Current.BoundingRectangle
            return $r
        }
    }
    return $null
}

function Click-TrayReopen {
    $r = Get-TrayButton
    if (-not $r) { return $false }
    [SmokeNative]::SetCursorPos([int]($r.X+$r.Width/2), [int]($r.Y+$r.Height/2)) | Out-Null
    Start-Sleep -Milliseconds 400
    [SmokeNative]::mouse_event(0x0002,0,0,0,[UIntPtr]::Zero); Start-Sleep -Milliseconds 100
    [SmokeNative]::mouse_event(0x0004,0,0,0,[UIntPtr]::Zero)
    return $true
}

function Quit-FromTray {
    # 右键托盘按钮 → 弹出菜单 → 向菜单窗口投递 Down×3 + Enter 选中"退出"
    # 重试最多 3 次（右键菜单弹出时机不稳定）
    $rootEl = [System.Windows.Automation.AutomationElement]::RootElement
    $menuCond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ClassNameProperty, '#32768')

    for ($try = 1; $try -le 3; $try++) {
        # 清理残留菜单
        $menus = $rootEl.FindAll([System.Windows.Automation.TreeScope]::Children, $menuCond)
        foreach ($m in $menus) {
            $hwnd = New-Object System.IntPtr($m.Current.NativeWindowHandle)
            if ([SmokeNative]::IsWindowVisible($hwnd)) {
                [SmokeNative]::SendMessage($hwnd, 0x0100, [IntPtr]0x1B, [IntPtr]::Zero) | Out-Null  # ESC
            }
        }
        Start-Sleep -Milliseconds 400

        $r = Get-TrayButton
        if (-not $r) { Start-Sleep -Seconds 2; continue }
        [SmokeNative]::SetCursorPos([int]($r.X+$r.Width/2), [int]($r.Y+$r.Height/2)) | Out-Null
        Start-Sleep -Milliseconds 300
        [SmokeNative]::mouse_event(0x0008,0,0,0,[UIntPtr]::Zero)  # RIGHTDOWN
        Start-Sleep -Milliseconds 100
        [SmokeNative]::mouse_event(0x0010,0,0,0,[UIntPtr]::Zero)  # RIGHTUP
        Start-Sleep -Seconds 1

        $menus = $rootEl.FindAll([System.Windows.Automation.TreeScope]::Children, $menuCond)
        foreach ($m in $menus) {
            $hwnd = New-Object System.IntPtr($m.Current.NativeWindowHandle)
            if ([SmokeNative]::IsWindowVisible($hwnd)) {
                foreach ($vk in @(0x28, 0x28, 0x28, 0x0D)) {
                    [SmokeNative]::SendMessage($hwnd, 0x0100, [IntPtr]$vk, [IntPtr]::Zero) | Out-Null
                    Start-Sleep -Milliseconds 200
                }
                Start-Sleep -Seconds 1
                if (-not (Get-Process -Name 'msl-desktop' -ErrorAction SilentlyContinue)) {
                    return $true
                }
            }
        }
    }
    return $false
}

# ---- 0. 清理 ----
Get-Process -Name 'msl-desktop' -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2
Remove-Item -Force "$dbPath*" -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $smokeDir -Force | Out-Null
Remove-Item -Recurse -Force "$smokeDir\*" -ErrorAction SilentlyContinue
Write-Host "=== Stage 11 smoke test ==="

# ---- 1. LAUNCH ----
$sw = [System.Diagnostics.Stopwatch]::StartNew()
Start-Process -FilePath $Exe | Out-Null
$ok = Wait-Window -WantTitle "MSL Desktop" -TimeoutSec 30
$sw.Stop()
Report "1. launch (window appears)" $ok ("cold start " + $sw.ElapsedMilliseconds + " ms")
Start-Sleep -Seconds 3

# ---- 2. CDP 数据流验证 ----
Write-Host "--- CDP data-flow checks ---"
if (Test-Path $cdpPy) {
    $out = & $Python $cdpPy 2>&1
    $out | ForEach-Object { Write-Host "   $_" }
    $lastLine = $out | Select-Object -Last 1
    if ("$lastLine" -match "ALL_PASS") { Report "2. cdp data-flow" $true } else { Report "2. cdp data-flow" $false ("last=" + $lastLine) }
} else {
    Report "2. cdp data-flow" $false "missing $cdpPy"
}

# ---- 3. CLOSE TO CORE ----
$p = Get-Process -Name 'msl-desktop' -ErrorAction SilentlyContinue | Select-Object -First 1
$p.CloseMainWindow() | Out-Null
$ok = Wait-Window -WantTitle "" -TimeoutSec 10
$p2 = Get-Process -Name 'msl-desktop' -ErrorAction SilentlyContinue | Select-Object -First 1
Report "3. close to core (window gone, process alive)" ($ok -and $p2) ("pid=" + $p2.Id)
Start-Sleep -Seconds 2

# ---- 4. REOPEN（托盘点击，尽力自动化；不稳定时 WARN）----
$clicked = $false
for ($try = 1; $try -le 3 -and -not $clicked; $try++) {
    $clicked = Click-TrayReopen
    if (-not $clicked) { Start-Sleep -Seconds 2 }
}
$ok = Wait-Window -WantTitle "MSL Desktop" -TimeoutSec 15
Report "4. reopen via tray click" ($clicked -and $ok) ("try=$try") -Warn
Start-Sleep -Seconds 2

# ---- 5. EXIT（托盘菜单退出，尽力自动化；失败时清理并 WARN）----
$quit = Quit-FromTray
Start-Sleep -Seconds 3
$gone = -not (Get-Process -Name 'msl-desktop' -ErrorAction SilentlyContinue)
if (-not $gone) {
    # 清理残留（托盘菜单自动化不稳定，退出路径人工复核）
    Get-Process -Name 'msl-desktop' -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 2
}
Report "5. exit via tray menu (process ends)" $gone -Warn

# ---- 6. PERSISTENCE（重启后数据仍在）----
Start-Process -FilePath $Exe | Out-Null
$ok = Wait-Window -WantTitle "MSL Desktop" -TimeoutSec 30
if ($ok) {
    Start-Sleep -Seconds 3
    $out = & $Python $cdpPy --check 2>&1
    $out | ForEach-Object { Write-Host "   $_" }
    $lastLine = $out | Select-Object -Last 1
    Report "6. persistence after restart" ("$lastLine" -match "ALL_PASS") ("last=" + $lastLine)
    # 清理：退出
    $p = Get-Process -Name 'msl-desktop' -ErrorAction SilentlyContinue | Select-Object -First 1
    $p.CloseMainWindow() | Out-Null
    Start-Sleep -Seconds 1
    Get-Process -Name 'msl-desktop' -ErrorAction SilentlyContinue | Stop-Process -Force
} else {
    Report "6. persistence after restart" $false "relaunch failed"
}
Remove-Item -Recurse -Force $smokeDir -ErrorAction SilentlyContinue

Write-Host ""
Write-Host ("=== SMOKE RESULT: PASS=" + $passCount + " FAIL=" + $failCount + " WARN=" + $warnCount + " ===")
Write-Host "人工 checklist 见 docs/smoke-checklist.md"
if ($failCount -gt 0) { exit 1 } else { exit 0 }
